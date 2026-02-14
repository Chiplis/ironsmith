//! Filter system for selecting objects in the game.
//!
//! This module provides filters for selecting objects (permanents, spells, cards)
//! based on various criteria like card types, colors, power/toughness, etc.
//!
//! Filters are used by:
//! - Target specifications (for spells and abilities that target)
//! - Effect conditions (for effects that affect "all creatures" etc.)
//! - Cost requirements (for sacrifice costs, etc.)
//! - Triggered ability conditions (for triggers that watch for specific events)

use crate::color::ColorSet;
use crate::ids::{ObjectId, PlayerId};
use crate::object::{CounterType, Object, ObjectKind};
use crate::static_abilities::StaticAbilityId;
use crate::tag::TagKey;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

// ============================================================================
// Object Reference (for cross-effect tagging)
// ============================================================================

/// A reference to an object for use in filters and effects.
///
/// This allows effects to reference objects from prior effects in the same
/// spell/ability resolution. For example, "Destroy target permanent. Its
/// controller creates a token" - the token creation needs to reference
/// the controller of the destroyed permanent.
///
/// # Variants
///
/// - `Target`: The first object target in the targets list (default)
/// - `Specific(ObjectId)`: A specific object by ID
/// - `Tagged(TagKey)`: An object referenced by tag from a prior effect
///
/// # Example
///
/// ```ignore
/// // Tag the destroyed permanent
/// Effect::destroy(ChooseSpec::permanent()).tag("destroyed"),
/// // Reference the tagged object's controller
/// Effect::create_tokens_player(token, 1, PlayerFilter::ControllerOf(ObjectRef::tagged("destroyed")))
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ObjectRef {
    /// The first object target in the targets list (default).
    #[default]
    Target,
    /// A specific object by ID.
    Specific(ObjectId),
    /// An object referenced by tag from a prior effect.
    Tagged(TagKey),
}

impl ObjectRef {
    /// Create a reference to a tagged object.
    pub fn tagged(tag: impl Into<TagKey>) -> Self {
        Self::Tagged(tag.into())
    }

    /// Create a reference to a specific object.
    pub fn specific(id: ObjectId) -> Self {
        Self::Specific(id)
    }
}

/// Context needed for evaluating filters.
///
/// Provides information about "you" (the controller), the source object,
/// active player, and other contextual details.
#[derive(Debug, Clone, Default)]
pub struct FilterContext {
    /// The controller of the source ability ("you")
    pub you: Option<PlayerId>,

    /// The source object of the ability
    pub source: Option<ObjectId>,

    /// The active player (whose turn it is)
    pub active_player: Option<PlayerId>,

    /// Players who are opponents of "you"
    pub opponents: Vec<PlayerId>,

    /// Players who are teammates of "you" (for team games)
    pub teammates: Vec<PlayerId>,

    /// The defending player (in combat)
    pub defending_player: Option<PlayerId>,

    /// The attacking player (in combat)
    pub attacking_player: Option<PlayerId>,

    /// Commander IDs controlled by "you" (for Commander format)
    pub your_commanders: Vec<ObjectId>,

    /// The current iterated player (for ForEachOpponent/ForEachPlayer effects)
    pub iterated_player: Option<PlayerId>,

    /// Resolved player targets from the current execution context.
    pub target_players: Vec<PlayerId>,

    /// Tagged objects from prior effects in the same spell/ability.
    /// Used by tag-aware object filter constraints.
    pub tagged_objects: std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
}

impl FilterContext {
    /// Create a new context with the controller specified.
    pub fn new(you: PlayerId) -> Self {
        Self {
            you: Some(you),
            ..Default::default()
        }
    }

    /// Set the source object.
    pub fn with_source(mut self, source: ObjectId) -> Self {
        self.source = Some(source);
        self
    }

    /// Set the active player.
    pub fn with_active_player(mut self, active: PlayerId) -> Self {
        self.active_player = Some(active);
        self
    }

    /// Set the opponents.
    pub fn with_opponents(mut self, opponents: Vec<PlayerId>) -> Self {
        self.opponents = opponents;
        self
    }

    /// Set your commanders (for Commander format filtering).
    pub fn with_your_commanders(mut self, commanders: Vec<ObjectId>) -> Self {
        self.your_commanders = commanders;
        self
    }

    /// Set the iterated player (for ForEachOpponent/ForEachPlayer effects).
    pub fn with_iterated_player(mut self, player: Option<PlayerId>) -> Self {
        self.iterated_player = player;
        self
    }

    /// Set resolved player targets from the execution context.
    pub fn with_target_players(mut self, players: Vec<PlayerId>) -> Self {
        self.target_players = players;
        self
    }

    /// Set tagged objects from the execution context.
    pub fn with_tagged_objects(
        mut self,
        tagged: &std::collections::HashMap<TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    ) -> Self {
        self.tagged_objects = tagged.clone();
        self
    }
}

/// A numeric comparison for filtering.
#[derive(Debug, Clone, PartialEq)]
pub enum Comparison {
    Equal(i32),
    NotEqual(i32),
    LessThan(i32),
    LessThanOrEqual(i32),
    GreaterThan(i32),
    GreaterThanOrEqual(i32),
}

impl Comparison {
    /// Check if a value satisfies this comparison.
    pub fn satisfies(&self, value: i32) -> bool {
        match self {
            Comparison::Equal(n) => value == *n,
            Comparison::NotEqual(n) => value != *n,
            Comparison::LessThan(n) => value < *n,
            Comparison::LessThanOrEqual(n) => value <= *n,
            Comparison::GreaterThan(n) => value > *n,
            Comparison::GreaterThanOrEqual(n) => value >= *n,
        }
    }
}

/// Which power/toughness reference a filter should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PtReference {
    /// "Power"/"toughness" (effective values, with continuous effects applied).
    #[default]
    Effective,
    /// "Base power"/"base toughness" (without counters/modifiers).
    Base,
}

/// Filter for selecting players.
///
/// This enum handles both filtering/matching players and specifying
/// which player(s) an effect applies to.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PlayerFilter {
    /// Any player in the game
    #[default]
    Any,

    /// The controller of the source ability
    You,

    /// An opponent of the controller
    Opponent,

    /// A teammate (for team games)
    Teammate,

    /// The active player (whose turn it is)
    Active,

    /// The defending player (in combat)
    Defending,

    /// The attacking player (in combat)
    Attacking,

    /// The player who was dealt damage by the triggering damage event.
    DamagedPlayer,

    /// A specific player
    Specific(PlayerId),

    /// The current player in a ForEachPlayer iteration
    IteratedPlayer,

    /// Target player (uses targeting with a filter)
    Target(Box<PlayerFilter>),

    /// The controller of an object.
    ///
    /// - `ObjectRef::Target` = the first object target (default)
    /// - `ObjectRef::Specific(id)` = a specific object by ID
    /// - `ObjectRef::tagged(tag)` = an object referenced by tag from a prior effect
    ControllerOf(ObjectRef),

    /// The owner of an object.
    ///
    /// - `ObjectRef::Target` = the first object target (default)
    /// - `ObjectRef::Specific(id)` = a specific object by ID
    /// - `ObjectRef::tagged(tag)` = an object referenced by tag from a prior effect
    OwnerOf(ObjectRef),
}

impl PlayerFilter {
    /// Create a filter for targeting any player.
    pub fn target_player() -> Self {
        Self::Target(Box::new(PlayerFilter::Any))
    }

    /// Create a filter for targeting an opponent.
    pub fn target_opponent() -> Self {
        Self::Target(Box::new(PlayerFilter::Opponent))
    }

    /// Check if a player matches this filter.
    ///
    /// Note: Some variants (EachOpponent, EachPlayer, Target, ControllerOf, OwnerOf, IteratedPlayer)
    /// are resolved at runtime during effect execution, not through this method.
    pub fn matches_player(&self, player: PlayerId, ctx: &FilterContext) -> bool {
        match self {
            PlayerFilter::Any => true,

            PlayerFilter::You => ctx.you.is_some_and(|you| player == you),

            PlayerFilter::Opponent => ctx.opponents.contains(&player),

            PlayerFilter::Teammate => ctx.teammates.contains(&player),

            PlayerFilter::Active => ctx.active_player.is_some_and(|ap| player == ap),

            PlayerFilter::Defending => ctx.defending_player.is_some_and(|dp| player == dp),

            PlayerFilter::Attacking => ctx.attacking_player.is_some_and(|ap| player == ap),

            // Resolved from the triggering event during effect execution.
            PlayerFilter::DamagedPlayer => false,

            PlayerFilter::Specific(id) => player == *id,

            // These are resolved at runtime during effect execution
            PlayerFilter::IteratedPlayer => ctx.iterated_player.is_some_and(|p| p == player),
            PlayerFilter::Target(inner) => {
                if !ctx.target_players.is_empty() {
                    return ctx.target_players.contains(&player)
                        && inner.matches_player(player, ctx);
                }
                ctx.iterated_player.is_some_and(|p| p == player)
                    && inner.matches_player(player, ctx)
            }
            PlayerFilter::ControllerOf(_) => false, // Resolved via object lookup
            PlayerFilter::OwnerOf(_) => false,      // Resolved via object lookup
        }
    }
}

/// Relationship an object may have with a tagged object set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedOpbjectRelation {
    /// The object must be one of the tagged objects.
    IsTaggedObject,
    /// The object must share at least one card type with any tagged object.
    SharesCardType,
    /// The object must share at least one subtype with any tagged object.
    SharesSubtypeWithTagged,
    /// The object must share at least one color with any tagged object.
    SharesColorWithTagged,
    /// The object must share the same stable_id with a tagged object.
    SameStableId,
    /// The object must have the same name as a tagged object.
    SameNameAsTagged,
    /// The object must have the same controller as a tagged object.
    SameControllerAsTagged,
    /// The object must NOT be one of the tagged objects.
    IsNotTaggedObject,
}

/// Alternative casting capability qualifier for card filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlternativeCastKind {
    Flashback,
    JumpStart,
    Escape,
    Madness,
    Miracle,
}

/// Counter-state qualifier for object filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterConstraint {
    /// At least one counter of any type.
    Any,
    /// At least one counter of the given type.
    Typed(CounterType),
}

/// A tagged-object constraint used by `ObjectFilter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedObjectConstraint {
    pub tag: TagKey,
    pub relation: TaggedOpbjectRelation,
}

/// Filter for selecting objects (permanents, spells, cards).
///
/// This is the primary filter type used throughout the game engine for
/// selecting objects based on various criteria. It can be used for:
/// - Targeting (spells and abilities that target)
/// - Effects (affecting "all creatures", "each artifact", etc.)
/// - Costs (sacrifice costs that require specific permanents)
/// - Triggers (watching for specific types of events)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectFilter {
    /// Zone the object must be in (None = any zone, but typically battlefield for permanents)
    pub zone: Option<Zone>,

    /// Controller filter (None = any controller)
    pub controller: Option<PlayerFilter>,

    /// Owner filter (None = any owner)
    pub owner: Option<PlayerFilter>,

    /// If set, only match spells/abilities that target a player matching this filter.
    pub targets_player: Option<PlayerFilter>,

    /// If set, only match spells/abilities that target an object matching this filter.
    pub targets_object: Option<Box<ObjectFilter>>,

    /// Required card types (object must have at least one if non-empty)
    pub card_types: Vec<CardType>,

    /// Required card types (object must have all of these if non-empty)
    pub all_card_types: Vec<CardType>,

    /// Excluded card types (object must have none of these)
    pub excluded_card_types: Vec<CardType>,

    /// Required subtypes (object must have at least one if non-empty)
    pub subtypes: Vec<Subtype>,

    /// If true, `card_types` and `subtypes` are matched as an OR-union
    /// instead of the default AND behavior.
    pub type_or_subtype_union: bool,

    /// Excluded subtypes (object must have none of these)
    pub excluded_subtypes: Vec<Subtype>,

    /// Required supertypes (object must have at least one if non-empty)
    pub supertypes: Vec<Supertype>,

    /// Excluded supertypes (object must have none of these)
    pub excluded_supertypes: Vec<Supertype>,

    /// Color filter (object must have at least one of these colors, if set)
    pub colors: Option<ColorSet>,

    /// Excluded colors (object must have none of these colors)
    pub excluded_colors: ColorSet,

    /// If true, must be colorless
    pub colorless: bool,

    /// If true, must be multicolored (2+ colors)
    pub multicolored: bool,

    /// If true, must be monocolored (exactly 1 color)
    pub monocolored: bool,

    /// If true, must be historic (artifact, legendary, or Saga)
    pub historic: bool,

    /// If true, must be nonhistoric
    pub nonhistoric: bool,

    /// If true, must be a token
    pub token: bool,

    /// If true, must be a nontoken
    pub nontoken: bool,

    /// If true, must be "another" (not the source)
    pub other: bool,

    /// If true, must be tapped
    pub tapped: bool,

    /// If true, must be untapped
    pub untapped: bool,

    /// If true, must be attacking
    pub attacking: bool,

    /// If true, must be blocking
    pub blocking: bool,

    /// If true, must have entered since your last turn ended.
    /// This is currently approximated via summoning-sick state.
    pub entered_since_your_last_turn_ended: bool,

    /// Power comparison (creature must satisfy)
    pub power: Option<Comparison>,
    /// Whether `power` is checked against effective or base power.
    pub power_reference: PtReference,

    /// Toughness comparison (creature must satisfy)
    pub toughness: Option<Comparison>,
    /// Whether `toughness` is checked against effective or base toughness.
    pub toughness_reference: PtReference,

    /// Mana value comparison
    pub mana_value: Option<Comparison>,

    /// If true, the card must have a mana cost (not empty/None)
    /// Cards like suspend-only cards or back faces may not have a mana cost
    pub has_mana_cost: bool,

    /// If true, object must have an activated ability with {T} in its cost.
    pub has_tap_activated_ability: bool,

    /// If true, the mana cost must not contain X
    pub no_x_in_cost: bool,

    /// Counter-state requirements such as "with a +1/+1 counter on it".
    pub with_counter: Option<CounterConstraint>,

    /// Counter-state exclusions such as "without a +1/+1 counter on it".
    pub without_counter: Option<CounterConstraint>,

    /// Name must match (for cards like "Rat Colony")
    pub name: Option<String>,

    /// Require a card to have a specific alternative casting capability.
    pub alternative_cast: Option<AlternativeCastKind>,

    /// Required static ability IDs (object must have all of these).
    pub static_abilities: Vec<StaticAbilityId>,

    /// Excluded static ability IDs (object must have none of these).
    pub excluded_static_abilities: Vec<StaticAbilityId>,

    /// Required custom static-ability marker text (case-insensitive match on ability display text).
    pub custom_static_markers: Vec<String>,

    /// Excluded custom static-ability marker text.
    pub excluded_custom_static_markers: Vec<String>,

    /// If true, must be a commander creature (for Commander format)
    pub is_commander: bool,

    /// Tagged-object constraints evaluated against `FilterContext::tagged_objects`.
    pub tagged_constraints: Vec<TaggedObjectConstraint>,

    /// If set, only match this specific object ID.
    pub specific: Option<ObjectId>,

    /// If true, only match the source object from the current filter context.
    pub source: bool,
}

impl ObjectFilter {
    /// Create a filter for any permanent (on the battlefield).
    pub fn permanent() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            ..Default::default()
        }
    }

    /// Create a filter that matches a specific object ID.
    pub fn specific(id: ObjectId) -> Self {
        Self {
            specific: Some(id),
            ..Default::default()
        }
    }

    /// Create a filter for creatures.
    pub fn creature() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Creature],
            ..Default::default()
        }
    }

    /// Create a filter for artifacts.
    pub fn artifact() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Artifact],
            ..Default::default()
        }
    }

    /// Create a filter for enchantments.
    pub fn enchantment() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Enchantment],
            ..Default::default()
        }
    }

    /// Create a filter for lands.
    pub fn land() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    /// Create a filter for planeswalkers.
    pub fn planeswalker() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Planeswalker],
            ..Default::default()
        }
    }

    /// Create a filter for spells (on the stack).
    pub fn spell() -> Self {
        Self {
            zone: Some(Zone::Stack),
            ..Default::default()
        }
    }

    /// Create a filter for instant or sorcery spells.
    pub fn instant_or_sorcery() -> Self {
        Self {
            zone: Some(Zone::Stack),
            card_types: vec![CardType::Instant, CardType::Sorcery],
            ..Default::default()
        }
    }

    /// Create a filter for noncreature spells (any card type except creature).
    /// Used for "You may cast noncreature spells as though they had flash."
    /// Note: This filter doesn't specify a zone, so it can be used for cards in hand.
    pub fn noncreature_spell() -> Self {
        Self {
            excluded_card_types: vec![CardType::Creature, CardType::Land],
            ..Default::default()
        }
    }

    /// Create a filter for nonland permanents.
    pub fn nonland_permanent() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            excluded_card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    /// Create a filter for noncreature permanents.
    pub fn noncreature_permanent() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            excluded_card_types: vec![CardType::Creature],
            ..Default::default()
        }
    }

    /// Create a filter for nonland cards (any zone).
    pub fn nonland() -> Self {
        Self {
            excluded_card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    /// Set the zone filter.
    pub fn in_zone(mut self, zone: Zone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Require the object to be a spell/ability on the stack targeting a player
    /// and/or object matching the provided filters.
    ///
    /// This implicitly filters to `Zone::Stack`.
    pub fn targeting(mut self, player: Option<PlayerFilter>, object: Option<ObjectFilter>) -> Self {
        self.zone = Some(Zone::Stack);
        self.targets_player = player;
        self.targets_object = object.map(Box::new);
        self
    }

    /// Require the object to be a spell/ability on the stack targeting a player.
    pub fn targeting_player(self, player: PlayerFilter) -> Self {
        self.targeting(Some(player), None)
    }

    /// Require the object to be a spell/ability on the stack targeting an object.
    pub fn targeting_object(self, object: ObjectFilter) -> Self {
        self.targeting(None, Some(object))
    }

    /// Set the controller filter.
    pub fn controlled_by(mut self, controller: PlayerFilter) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Require the object to be controlled by "you" (the source's controller).
    pub fn you_control(self) -> Self {
        self.controlled_by(PlayerFilter::You)
    }

    /// Require the object to be controlled by an opponent.
    pub fn opponent_controls(self) -> Self {
        self.controlled_by(PlayerFilter::Opponent)
    }

    /// Set the owner filter.
    pub fn owned_by(mut self, owner: PlayerFilter) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Require the object to be "another" (not the source).
    pub fn other(mut self) -> Self {
        self.other = true;
        self
    }

    /// Add a required card type.
    pub fn with_type(mut self, card_type: CardType) -> Self {
        self.card_types.push(card_type);
        self
    }

    /// Require the object to have all of the specified card types.
    pub fn with_all_type(mut self, card_type: CardType) -> Self {
        self.all_card_types.push(card_type);
        self
    }

    /// Add an excluded card type.
    pub fn without_type(mut self, card_type: CardType) -> Self {
        self.excluded_card_types.push(card_type);
        self
    }

    /// Add a required subtype.
    pub fn with_subtype(mut self, subtype: Subtype) -> Self {
        self.subtypes.push(subtype);
        self
    }

    /// Add an excluded subtype.
    pub fn without_subtype(mut self, subtype: Subtype) -> Self {
        self.excluded_subtypes.push(subtype);
        self
    }

    /// Add a required supertype.
    pub fn with_supertype(mut self, supertype: Supertype) -> Self {
        self.supertypes.push(supertype);
        self
    }

    /// Add an excluded supertype.
    pub fn without_supertype(mut self, supertype: Supertype) -> Self {
        self.excluded_supertypes.push(supertype);
        self
    }

    /// Shorthand for excluding Basic supertype (for "nonbasic" filters).
    pub fn nonbasic(self) -> Self {
        self.without_supertype(Supertype::Basic)
    }

    /// Require the object to be a token.
    pub fn token(mut self) -> Self {
        self.token = true;
        self
    }

    /// Require the object to be a nontoken.
    pub fn nontoken(mut self) -> Self {
        self.nontoken = true;
        self
    }

    /// Require the object to be tapped.
    pub fn tapped(mut self) -> Self {
        self.tapped = true;
        self
    }

    /// Require the object to be untapped.
    pub fn untapped(mut self) -> Self {
        self.untapped = true;
        self
    }

    /// Require power to satisfy a comparison.
    pub fn with_power(mut self, cmp: Comparison) -> Self {
        self.power = Some(cmp);
        self.power_reference = PtReference::Effective;
        self
    }

    /// Require base power to satisfy a comparison.
    pub fn with_base_power(mut self, cmp: Comparison) -> Self {
        self.power = Some(cmp);
        self.power_reference = PtReference::Base;
        self
    }

    /// Require toughness to satisfy a comparison.
    pub fn with_toughness(mut self, cmp: Comparison) -> Self {
        self.toughness = Some(cmp);
        self.toughness_reference = PtReference::Effective;
        self
    }

    /// Require base toughness to satisfy a comparison.
    pub fn with_base_toughness(mut self, cmp: Comparison) -> Self {
        self.toughness = Some(cmp);
        self.toughness_reference = PtReference::Base;
        self
    }

    /// Require mana value to satisfy a comparison.
    pub fn with_mana_value(mut self, cmp: Comparison) -> Self {
        self.mana_value = Some(cmp);
        self
    }

    /// Require the object to have certain colors.
    pub fn with_colors(mut self, colors: ColorSet) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Exclude objects that have any of the specified colors.
    pub fn without_colors(mut self, colors: ColorSet) -> Self {
        self.excluded_colors = self.excluded_colors.union(colors);
        self
    }

    /// Require the object to be colorless.
    pub fn colorless(mut self) -> Self {
        self.colorless = true;
        self
    }

    /// Require the object to be multicolored.
    pub fn multicolored(mut self) -> Self {
        self.multicolored = true;
        self
    }

    /// Require the object to be monocolored.
    pub fn monocolored(mut self) -> Self {
        self.monocolored = true;
        self
    }

    /// Require the object to be historic (artifact, legendary, or Saga).
    pub fn historic(mut self) -> Self {
        self.historic = true;
        self
    }

    /// Require the object to be nonhistoric.
    pub fn nonhistoric(mut self) -> Self {
        self.nonhistoric = true;
        self
    }

    /// Require a specific name.
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Require the object to be a commander (for Commander format).
    pub fn commander(mut self) -> Self {
        self.is_commander = true;
        self
    }

    /// Require a specific alternative casting capability.
    pub fn with_alternative_cast(mut self, kind: AlternativeCastKind) -> Self {
        self.alternative_cast = Some(kind);
        self
    }

    /// Require the object to have at least one counter.
    pub fn with_any_counter(mut self) -> Self {
        self.with_counter = Some(CounterConstraint::Any);
        self
    }

    /// Require the object to have at least one counter of the given type.
    pub fn with_counter_type(mut self, counter_type: CounterType) -> Self {
        self.with_counter = Some(CounterConstraint::Typed(counter_type));
        self
    }

    /// Require the object to have no counters.
    pub fn without_any_counter(mut self) -> Self {
        self.without_counter = Some(CounterConstraint::Any);
        self
    }

    /// Require the object to have no counters of the given type.
    pub fn without_counter_type(mut self, counter_type: CounterType) -> Self {
        self.without_counter = Some(CounterConstraint::Typed(counter_type));
        self
    }

    /// Require a specific static ability ID.
    pub fn with_static_ability(mut self, ability_id: StaticAbilityId) -> Self {
        if !self.static_abilities.contains(&ability_id) {
            self.static_abilities.push(ability_id);
        }
        self
    }

    /// Exclude objects with a specific static ability ID.
    pub fn without_static_ability(mut self, ability_id: StaticAbilityId) -> Self {
        if !self.excluded_static_abilities.contains(&ability_id) {
            self.excluded_static_abilities.push(ability_id);
        }
        self
    }

    /// Require a custom static marker (for marker-style keyword abilities such as landwalk).
    pub fn with_custom_static_marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !self
            .custom_static_markers
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&marker))
        {
            self.custom_static_markers.push(marker);
        }
        self
    }

    /// Exclude objects with a custom static marker.
    pub fn without_custom_static_marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !self
            .excluded_custom_static_markers
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&marker))
        {
            self.excluded_custom_static_markers.push(marker);
        }
        self
    }

    /// Require an activated ability with {T} in its cost.
    pub fn with_tap_activated_ability(mut self) -> Self {
        self.has_tap_activated_ability = true;
        self
    }

    /// Add a tagged-object matching rule.
    pub fn match_tagged(mut self, tag: impl Into<TagKey>, relation: TaggedOpbjectRelation) -> Self {
        self.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.into(),
            relation,
        });
        self
    }

    /// Require the object to share at least one card type with tagged objects.
    ///
    /// Convenience wrapper around
    /// `match_tagged(tag, TaggedObjectMatch::SharesCardType)`.
    pub fn shares_card_type_with_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesCardType)
    }

    /// Require the object to share at least one color with tagged objects.
    pub fn shares_color_with_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesColorWithTagged)
    }

    /// Require the object to share at least one subtype with tagged objects.
    pub fn shares_subtype_with_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesSubtypeWithTagged)
    }

    /// Filter to only match objects that share the same stable_id with a tagged object.
    pub fn same_stable_id_as_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SameStableId)
    }

    /// Filter to only match objects stored under a specific tag.
    ///
    /// Convenience constructor for
    /// `match_tagged(tag, TaggedObjectMatch::IsTaggedObject)`.
    pub fn tagged(tag: impl Into<TagKey>) -> Self {
        Self::default().match_tagged(tag, TaggedOpbjectRelation::IsTaggedObject)
    }

    /// Filter to exclude objects stored under a specific tag.
    pub fn not_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::IsNotTaggedObject)
    }

    /// Create a filter that matches any of the given card types.
    ///
    /// This is useful for Braids-style effects that need to match "artifact, creature,
    /// enchantment, land, or planeswalker" (i.e., any permanent type).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let filter = ObjectFilter::any_of_types(&[
    ///     CardType::Artifact,
    ///     CardType::Creature,
    ///     CardType::Enchantment,
    ///     CardType::Land,
    ///     CardType::Planeswalker,
    /// ]).you_control();
    /// ```
    pub fn any_of_types(types: &[CardType]) -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: types.to_vec(),
            ..Default::default()
        }
    }

    /// Create a filter matching only the source object.
    pub fn source() -> Self {
        Self {
            source: true,
            ..Default::default()
        }
    }

    /// Check if an object matches this filter, with access to game state.
    ///
    /// # Arguments
    /// * `object` - The object to check
    /// * `ctx` - Context providing information about "you", the source, etc.
    /// * `game` - Game state for checking tapped/untapped status
    pub fn matches(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool {
        self.matches_internal(object, ctx, game, true)
    }

    /// Check if an object matches this filter without consulting calculated characteristics.
    ///
    /// This is used by layer-calculation paths that must avoid recursively
    /// re-entering characteristic computation.
    pub fn matches_non_recursive(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool {
        self.matches_internal(object, ctx, game, false)
    }

    fn matches_internal(
        &self,
        object: &Object,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
        allow_calculated_pt: bool,
    ) -> bool {
        // Specific object check
        if let Some(id) = self.specific
            && object.id != id
        {
            return false;
        }

        if self.source && ctx.source.is_none_or(|source_id| object.id != source_id) {
            return false;
        }

        if self.entered_since_your_last_turn_ended && !game.is_summoning_sick(object.id) {
            return false;
        }

        // Zone check
        if let Some(zone) = &self.zone
            && object.zone != *zone
        {
            return false;
        }

        // Controller check
        if let Some(controller_filter) = &self.controller
            && !controller_filter.matches_player(object.controller, ctx)
        {
            return false;
        }

        // Owner check
        if let Some(owner_filter) = &self.owner
            && !owner_filter.matches_player(object.owner, ctx)
        {
            return false;
        }

        if self.type_or_subtype_union {
            let type_match = !self.card_types.is_empty()
                && self
                    .card_types
                    .iter()
                    .any(|t| object.card_types.contains(t));
            let subtype_match = !self.subtypes.is_empty()
                && self.subtypes.iter().any(|t| object.subtypes.contains(t));
            if (!self.card_types.is_empty() || !self.subtypes.is_empty())
                && !(type_match || subtype_match)
            {
                return false;
            }
        } else if !self.card_types.is_empty()
            && !self
                .card_types
                .iter()
                .any(|t| object.card_types.contains(t))
        {
            return false;
        }

        // Card types (must have all if specified)
        if !self.all_card_types.is_empty()
            && !self
                .all_card_types
                .iter()
                .all(|t| object.card_types.contains(t))
        {
            return false;
        }

        // Excluded card types (must have none of these)
        if self
            .excluded_card_types
            .iter()
            .any(|t| object.card_types.contains(t))
        {
            return false;
        }

        // Subtypes (must have at least one if specified)
        if !self.type_or_subtype_union
            && !self.subtypes.is_empty()
            && !self.subtypes.iter().any(|t| object.subtypes.contains(t))
        {
            return false;
        }

        // Excluded subtypes (must have none of these)
        if self
            .excluded_subtypes
            .iter()
            .any(|t| object.subtypes.contains(t))
        {
            return false;
        }

        // Supertypes (must have at least one if specified)
        if !self.supertypes.is_empty()
            && !self
                .supertypes
                .iter()
                .any(|t| object.supertypes.contains(t))
        {
            return false;
        }

        // Excluded supertypes (must have none of these)
        if self
            .excluded_supertypes
            .iter()
            .any(|t| object.supertypes.contains(t))
        {
            return false;
        }

        // Color check
        if let Some(required_colors) = &self.colors {
            let obj_colors = object.colors();
            if required_colors.intersection(obj_colors).is_empty() {
                return false;
            }
        }

        // Excluded colors check
        if !self.excluded_colors.is_empty()
            && !self
                .excluded_colors
                .intersection(object.colors())
                .is_empty()
        {
            return false;
        }

        // Colorless check
        if self.colorless && !object.colors().is_empty() {
            return false;
        }

        // Multicolored check
        if self.multicolored && object.colors().count() < 2 {
            return false;
        }

        // Monocolored check
        if self.monocolored && object.colors().count() != 1 {
            return false;
        }

        let is_historic = object.card_types.contains(&CardType::Artifact)
            || object.supertypes.contains(&Supertype::Legendary)
            || object.subtypes.contains(&Subtype::Saga);
        if self.historic && !is_historic {
            return false;
        }
        if self.nonhistoric && is_historic {
            return false;
        }

        // Token/nontoken check
        if self.token && object.kind != ObjectKind::Token {
            return false;
        }
        if self.nontoken && object.kind == ObjectKind::Token {
            return false;
        }

        // "Other" check (not the source)
        if self.other
            && let Some(source_id) = ctx.source
            && object.id == source_id
        {
            return false;
        }

        let is_tapped = game.is_tapped(object.id);
        if self.tapped && !is_tapped {
            return false;
        }
        if self.untapped && is_tapped {
            return false;
        }
        if self.attacking
            && !game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_attacking(combat, object.id))
        {
            return false;
        }
        if self.blocking
            && !game
                .combat
                .as_ref()
                .is_some_and(|combat| crate::combat_state::is_blocking(combat, object.id))
        {
            return false;
        }

        // Power check
        if let Some(power_cmp) = &self.power {
            if let Some(power) = resolve_object_power_for_filter(
                object,
                game,
                self.power_reference,
                allow_calculated_pt,
            ) {
                if !power_cmp.satisfies(power) {
                    return false;
                }
            } else {
                return false; // No power means not a creature
            }
        }

        // Toughness check
        if let Some(toughness_cmp) = &self.toughness {
            if let Some(toughness) = resolve_object_toughness_for_filter(
                object,
                game,
                self.toughness_reference,
                allow_calculated_pt,
            ) {
                if !toughness_cmp.satisfies(toughness) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Mana value check
        if let Some(mv_cmp) = &self.mana_value {
            let mv = object
                .mana_cost
                .as_ref()
                .map(|mc| mc.mana_value() as i32)
                .unwrap_or(0);
            if !mv_cmp.satisfies(mv) {
                return false;
            }
        }

        // Has mana cost check (must have a non-empty mana cost)
        if self.has_mana_cost {
            match &object.mana_cost {
                Some(mc) if !mc.is_empty() => {} // Has a mana cost, OK
                _ => return false,               // No mana cost or empty
            }
        }

        // No X in cost check
        if self.no_x_in_cost
            && let Some(mc) = &object.mana_cost
            && mc.has_x()
        {
            return false;
        }

        // Name check
        if let Some(required_name) = &self.name
            && !object.name.eq_ignore_ascii_case(required_name)
        {
            return false;
        }

        if let Some(counter_requirement) = self.with_counter {
            let has_counter = match counter_requirement {
                CounterConstraint::Any => object.counters.values().any(|count| *count > 0),
                CounterConstraint::Typed(counter_type) => {
                    object.counters.get(&counter_type).copied().unwrap_or(0) > 0
                }
            };
            if !has_counter {
                return false;
            }
        }
        if let Some(counter_exclusion) = self.without_counter {
            let has_excluded_counter = match counter_exclusion {
                CounterConstraint::Any => object.counters.values().any(|count| *count > 0),
                CounterConstraint::Typed(counter_type) => {
                    object.counters.get(&counter_type).copied().unwrap_or(0) > 0
                }
            };
            if has_excluded_counter {
                return false;
            }
        }

        if let Some(kind) = self.alternative_cast
            && !object_has_alternative_cast_kind(object, kind, game, ctx)
        {
            return false;
        }

        // Required static ability IDs
        if self
            .static_abilities
            .iter()
            .any(|ability_id| !object_has_static_ability_id(object, *ability_id))
        {
            return false;
        }

        // Excluded static ability IDs
        if self
            .excluded_static_abilities
            .iter()
            .any(|ability_id| object_has_static_ability_id(object, *ability_id))
        {
            return false;
        }

        // Required/excluded custom marker abilities
        if self
            .custom_static_markers
            .iter()
            .any(|marker| !object_has_custom_static_marker(object, marker))
        {
            return false;
        }
        if self
            .excluded_custom_static_markers
            .iter()
            .any(|marker| object_has_custom_static_marker(object, marker))
        {
            return false;
        }

        if self.has_tap_activated_ability && !object_has_tap_activated_ability(object) {
            return false;
        }

        // Commander check
        if self.is_commander && !game.is_commander(object.id) {
            return false;
        }

        for constraint in &self.tagged_constraints {
            let Some(tagged_snapshots) = ctx.tagged_objects.get(constraint.tag.as_str()) else {
                // Tag not found - no match possible for positive constraints.
                if matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject) {
                    return false;
                }
                // For negative constraints, missing tag means nothing is excluded.
                continue;
            };

            match constraint.relation {
                TaggedOpbjectRelation::IsTaggedObject => {
                    // Object must be one of the tagged objects.
                    if !tagged_snapshots.iter().any(|s| s.object_id == object.id) {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SharesCardType => {
                    // Object must share at least one card type with any tagged object.
                    let tagged_types: std::collections::HashSet<CardType> = tagged_snapshots
                        .iter()
                        .flat_map(|s| s.card_types.iter().cloned())
                        .collect();
                    if !object.card_types.iter().any(|t| tagged_types.contains(t)) {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SharesSubtypeWithTagged => {
                    let tagged_subtypes: std::collections::HashSet<Subtype> = tagged_snapshots
                        .iter()
                        .flat_map(|s| s.subtypes.iter().cloned())
                        .collect();
                    if !object.subtypes.iter().any(|t| tagged_subtypes.contains(t)) {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SharesColorWithTagged => {
                    if !tagged_snapshots
                        .iter()
                        .any(|s| !object.colors().intersection(s.colors).is_empty())
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SameStableId => {
                    if !tagged_snapshots
                        .iter()
                        .any(|s| s.stable_id == object.stable_id)
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SameNameAsTagged => {
                    if !tagged_snapshots
                        .iter()
                        .any(|s| s.name.eq_ignore_ascii_case(&object.name))
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SameControllerAsTagged => {
                    if !tagged_snapshots
                        .iter()
                        .any(|s| s.controller == object.controller)
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::IsNotTaggedObject => {
                    // Object must NOT be one of the tagged objects.
                    if tagged_snapshots.iter().any(|s| s.object_id == object.id) {
                        return false;
                    }
                }
            }
        }

        // Targeting checks (spell/ability targets on the stack)
        if self.targets_player.is_some() || self.targets_object.is_some() {
            if object.zone != Zone::Stack {
                return false;
            }

            let entry = game.stack.iter().find(|e| e.object_id == object.id);
            let Some(entry) = entry else {
                return false;
            };

            if let Some(player_filter) = &self.targets_player {
                let matches_player = entry.targets.iter().any(|target| match target {
                    crate::game_state::Target::Player(pid) => {
                        player_filter.matches_player(*pid, ctx)
                    }
                    _ => false,
                });
                if !matches_player {
                    return false;
                }
            }

            if let Some(object_filter) = &self.targets_object {
                let matches_object = entry.targets.iter().any(|target| match target {
                    crate::game_state::Target::Object(obj_id) => game
                        .object(*obj_id)
                        .is_some_and(|obj| object_filter.matches(obj, ctx, game)),
                    _ => false,
                });
                if !matches_object {
                    return false;
                }
            }
        }

        true
    }

    /// Check if a snapshot matches this filter.
    ///
    /// This is used for LKI/tagged-object comparisons where the object
    /// may no longer be available in the game state.
    pub fn matches_snapshot(
        &self,
        snapshot: &crate::snapshot::ObjectSnapshot,
        ctx: &FilterContext,
        game: &crate::game_state::GameState,
    ) -> bool {
        if let Some(id) = self.specific
            && snapshot.object_id != id
        {
            return false;
        }

        if self.source
            && ctx
                .source
                .is_none_or(|source_id| snapshot.object_id != source_id)
        {
            return false;
        }

        if self.entered_since_your_last_turn_ended
            && !game.is_summoning_sick(snapshot.object_id)
        {
            return false;
        }

        // Zone check
        if let Some(zone) = &self.zone
            && snapshot.zone != *zone
        {
            return false;
        }

        // Controller check
        if let Some(controller_filter) = &self.controller
            && !controller_filter.matches_player(snapshot.controller, ctx)
        {
            return false;
        }

        // Owner check
        if let Some(owner_filter) = &self.owner
            && !owner_filter.matches_player(snapshot.owner, ctx)
        {
            return false;
        }

        // Card types (must have at least one if specified)
        if !self.card_types.is_empty()
            && !self
                .card_types
                .iter()
                .any(|t| snapshot.card_types.contains(t))
        {
            return false;
        }

        // Card types (must have all if specified)
        if !self.all_card_types.is_empty()
            && !self
                .all_card_types
                .iter()
                .all(|t| snapshot.card_types.contains(t))
        {
            return false;
        }

        // Excluded card types (must have none of these)
        if self
            .excluded_card_types
            .iter()
            .any(|t| snapshot.card_types.contains(t))
        {
            return false;
        }

        // Subtypes (must have at least one if specified)
        if !self.subtypes.is_empty() && !self.subtypes.iter().any(|t| snapshot.subtypes.contains(t))
        {
            return false;
        }

        // Excluded subtypes (must have none of these)
        if self
            .excluded_subtypes
            .iter()
            .any(|t| snapshot.subtypes.contains(t))
        {
            return false;
        }

        // Supertypes (must have at least one if specified)
        if !self.supertypes.is_empty()
            && !self
                .supertypes
                .iter()
                .any(|t| snapshot.supertypes.contains(t))
        {
            return false;
        }

        // Excluded supertypes (must have none of these)
        if self
            .excluded_supertypes
            .iter()
            .any(|t| snapshot.supertypes.contains(t))
        {
            return false;
        }

        // Color check
        if let Some(required_colors) = &self.colors
            && required_colors.intersection(snapshot.colors).is_empty()
        {
            return false;
        }

        // Excluded colors check
        if !self.excluded_colors.is_empty()
            && !self
                .excluded_colors
                .intersection(snapshot.colors)
                .is_empty()
        {
            return false;
        }

        // Colorless check
        if self.colorless && !snapshot.colors.is_empty() {
            return false;
        }

        // Multicolored check
        if self.multicolored && snapshot.colors.count() < 2 {
            return false;
        }

        // Monocolored check
        if self.monocolored && snapshot.colors.count() != 1 {
            return false;
        }

        let is_historic = snapshot.card_types.contains(&CardType::Artifact)
            || snapshot.supertypes.contains(&Supertype::Legendary)
            || snapshot.subtypes.contains(&Subtype::Saga);
        if self.historic && !is_historic {
            return false;
        }
        if self.nonhistoric && is_historic {
            return false;
        }

        // Token/nontoken check
        if self.token && !snapshot.is_token {
            return false;
        }
        if self.nontoken && snapshot.is_token {
            return false;
        }

        // "Other" check (not the source)
        if self.other
            && let Some(source_id) = ctx.source
            && snapshot.object_id == source_id
        {
            return false;
        }

        if self.tapped && !snapshot.tapped {
            return false;
        }
        if self.untapped && snapshot.tapped {
            return false;
        }

        // Power check
        if let Some(power_cmp) = &self.power {
            if let Some(power) = resolve_snapshot_power_for_filter(snapshot, self.power_reference) {
                if !power_cmp.satisfies(power) {
                    return false;
                }
            } else {
                return false; // No power means not a creature
            }
        }

        // Toughness check
        if let Some(toughness_cmp) = &self.toughness {
            if let Some(toughness) =
                resolve_snapshot_toughness_for_filter(snapshot, self.toughness_reference)
            {
                if !toughness_cmp.satisfies(toughness) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Mana value check
        if let Some(mv_cmp) = &self.mana_value {
            let mv = snapshot
                .mana_cost
                .as_ref()
                .map(|mc| mc.mana_value() as i32)
                .unwrap_or(0);
            if !mv_cmp.satisfies(mv) {
                return false;
            }
        }

        // Has mana cost check (must have a non-empty mana cost)
        if self.has_mana_cost {
            match &snapshot.mana_cost {
                Some(mc) if !mc.is_empty() => {}
                _ => return false,
            }
        }

        // No X in cost check
        if self.no_x_in_cost
            && let Some(mc) = &snapshot.mana_cost
            && mc.has_x()
        {
            return false;
        }

        // Name check
        if let Some(required_name) = &self.name
            && !snapshot.name.eq_ignore_ascii_case(required_name)
        {
            return false;
        }

        if let Some(counter_requirement) = self.with_counter {
            let has_counter = match counter_requirement {
                CounterConstraint::Any => snapshot.counters.values().any(|count| *count > 0),
                CounterConstraint::Typed(counter_type) => {
                    snapshot.counters.get(&counter_type).copied().unwrap_or(0) > 0
                }
            };
            if !has_counter {
                return false;
            }
        }
        if let Some(counter_exclusion) = self.without_counter {
            let has_excluded_counter = match counter_exclusion {
                CounterConstraint::Any => snapshot.counters.values().any(|count| *count > 0),
                CounterConstraint::Typed(counter_type) => {
                    snapshot.counters.get(&counter_type).copied().unwrap_or(0) > 0
                }
            };
            if has_excluded_counter {
                return false;
            }
        }

        if let Some(kind) = self.alternative_cast {
            let has_kind = game
                .object(snapshot.object_id)
                .is_some_and(|obj| object_has_alternative_cast_kind(obj, kind, game, ctx));
            if !has_kind {
                return false;
            }
        }

        // Required static ability IDs
        if self
            .static_abilities
            .iter()
            .any(|ability_id| !snapshot_has_static_ability_id(snapshot, *ability_id))
        {
            return false;
        }

        // Excluded static ability IDs
        if self
            .excluded_static_abilities
            .iter()
            .any(|ability_id| snapshot_has_static_ability_id(snapshot, *ability_id))
        {
            return false;
        }

        // Required/excluded custom marker abilities
        if self
            .custom_static_markers
            .iter()
            .any(|marker| !snapshot_has_custom_static_marker(snapshot, marker))
        {
            return false;
        }
        if self
            .excluded_custom_static_markers
            .iter()
            .any(|marker| snapshot_has_custom_static_marker(snapshot, marker))
        {
            return false;
        }

        if self.has_tap_activated_ability && !snapshot_has_tap_activated_ability(snapshot) {
            return false;
        }

        // Commander check
        if self.is_commander && !snapshot.is_commander {
            return false;
        }

        for constraint in &self.tagged_constraints {
            let Some(tagged_snapshots) = ctx.tagged_objects.get(constraint.tag.as_str()) else {
                if matches!(
                    constraint.relation,
                    TaggedOpbjectRelation::IsTaggedObject
                        | TaggedOpbjectRelation::SharesSubtypeWithTagged
                        | TaggedOpbjectRelation::SharesColorWithTagged
                        | TaggedOpbjectRelation::SameStableId
                        | TaggedOpbjectRelation::SameNameAsTagged
                        | TaggedOpbjectRelation::SameControllerAsTagged
                ) {
                    return false;
                }
                continue;
            };

            match constraint.relation {
                TaggedOpbjectRelation::IsTaggedObject => {
                    if !tagged_snapshots
                        .iter()
                        .any(|s| s.object_id == snapshot.object_id)
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SharesCardType => {
                    let tagged_types: std::collections::HashSet<CardType> = tagged_snapshots
                        .iter()
                        .flat_map(|s| s.card_types.iter().cloned())
                        .collect();
                    if !snapshot.card_types.iter().any(|t| tagged_types.contains(t)) {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SharesSubtypeWithTagged => {
                    let tagged_subtypes: std::collections::HashSet<Subtype> = tagged_snapshots
                        .iter()
                        .flat_map(|s| s.subtypes.iter().cloned())
                        .collect();
                    if !snapshot
                        .subtypes
                        .iter()
                        .any(|t| tagged_subtypes.contains(t))
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SharesColorWithTagged => {
                    if !tagged_snapshots
                        .iter()
                        .any(|s| !snapshot.colors.intersection(s.colors).is_empty())
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::IsNotTaggedObject => {
                    if tagged_snapshots
                        .iter()
                        .any(|s| s.object_id == snapshot.object_id)
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SameStableId => {
                    if !tagged_snapshots
                        .iter()
                        .any(|s| s.stable_id == snapshot.stable_id)
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SameNameAsTagged => {
                    if !tagged_snapshots
                        .iter()
                        .any(|s| s.name.eq_ignore_ascii_case(&snapshot.name))
                    {
                        return false;
                    }
                }
                TaggedOpbjectRelation::SameControllerAsTagged => {
                    if !tagged_snapshots
                        .iter()
                        .any(|s| s.controller == snapshot.controller)
                    {
                        return false;
                    }
                }
            }
        }

        if self.targets_player.is_some() || self.targets_object.is_some() {
            if snapshot.zone != Zone::Stack {
                return false;
            }

            let entry = game
                .stack
                .iter()
                .find(|e| e.object_id == snapshot.object_id);
            let Some(entry) = entry else {
                return false;
            };

            if let Some(player_filter) = &self.targets_player {
                let matches_player = entry.targets.iter().any(|target| match target {
                    crate::game_state::Target::Player(pid) => {
                        player_filter.matches_player(*pid, ctx)
                    }
                    _ => false,
                });
                if !matches_player {
                    return false;
                }
            }

            if let Some(object_filter) = &self.targets_object {
                let matches_object = entry.targets.iter().any(|target| match target {
                    crate::game_state::Target::Object(obj_id) => game
                        .object(*obj_id)
                        .is_some_and(|obj| object_filter.matches(obj, ctx, game)),
                    _ => false,
                });
                if !matches_object {
                    return false;
                }
            }
        }

        true
    }

    /// Generate a human-readable description of this filter.
    ///
    /// Used primarily for trigger display text.
    pub fn description(&self) -> String {
        let mut parts = Vec::new();
        let mut post_noun_qualifiers: Vec<String> = Vec::new();
        let append_token_after_type = self.token;
        let mut controller_suffix: Option<String> = None;
        let mut owner_suffix: Option<String> = None;

        // Handle "other" modifier
        if self.other {
            parts.push("another".to_string());
        }
        let has_target_tag = self.tagged_constraints.iter().any(|constraint| {
            matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                && constraint.tag.as_str().starts_with("targeted")
        });
        if has_target_tag {
            parts.push("target".to_string());
        }
        if self.source {
            parts.push("this".to_string());
        }

        // Handle controller
        if let Some(ref ctrl) = self.controller {
            match ctrl {
                PlayerFilter::You => {
                    if !self.other {
                        parts.push("a".to_string());
                    }
                    controller_suffix = Some("you control".to_string());
                }
                PlayerFilter::Opponent => parts.push("an opponent's".to_string()),
                PlayerFilter::Any => {}
                PlayerFilter::Active => parts.push("the active player's".to_string()),
                PlayerFilter::Specific(_) => parts.push("a specific player's".to_string()),
                PlayerFilter::Teammate => parts.push("a teammate's".to_string()),
                PlayerFilter::Defending => parts.push("the defending player's".to_string()),
                PlayerFilter::Attacking => parts.push("an attacking player's".to_string()),
                PlayerFilter::DamagedPlayer => parts.push("the damaged player's".to_string()),
                PlayerFilter::IteratedPlayer => {
                    if !self.other {
                        parts.push("a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string())
                }
                PlayerFilter::Target(inner) => {
                    let inner_desc = describe_player_filter(inner.as_ref());
                    if inner_desc == "player" {
                        parts.push("target player's".to_string());
                    } else {
                        parts.push(format!("target {inner_desc}'s"));
                    }
                }
                PlayerFilter::ControllerOf(_) => parts.push("a controller's".to_string()),
                PlayerFilter::OwnerOf(_) => parts.push("an owner's".to_string()),
            }
        }

        // Handle owner on object-level filters (battlefield/stack/any-zone object references).
        // Zone-restricted card references (e.g. "in your graveyard") already encode ownership.
        let owner_conveyed_by_zone = matches!(
            self.zone,
            Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
        );
        if !owner_conveyed_by_zone && let Some(ref owner) = self.owner {
            owner_suffix = Some(match owner {
                PlayerFilter::You => "you own".to_string(),
                PlayerFilter::Opponent => "an opponent owns".to_string(),
                PlayerFilter::Any => "a player owns".to_string(),
                PlayerFilter::Active => "the active player owns".to_string(),
                PlayerFilter::Specific(_) => "that player owns".to_string(),
                PlayerFilter::Teammate => "a teammate owns".to_string(),
                PlayerFilter::Defending => "the defending player owns".to_string(),
                PlayerFilter::Attacking => "an attacking player owns".to_string(),
                PlayerFilter::DamagedPlayer => "the damaged player owns".to_string(),
                PlayerFilter::IteratedPlayer => "that player owns".to_string(),
                PlayerFilter::Target(inner) => {
                    format!("target {} owns", describe_player_filter(inner.as_ref()))
                }
                PlayerFilter::ControllerOf(_) => "that object's controller owns".to_string(),
                PlayerFilter::OwnerOf(_) => "that object's owner owns".to_string(),
            });
        }

        // Handle token/nontoken
        if self.nontoken {
            parts.push("nontoken".to_string());
        }
        if let Some(colors) = self.colors {
            let mut color_words = Vec::new();
            if colors.contains(crate::color::Color::White) {
                color_words.push("white");
            }
            if colors.contains(crate::color::Color::Blue) {
                color_words.push("blue");
            }
            if colors.contains(crate::color::Color::Black) {
                color_words.push("black");
            }
            if colors.contains(crate::color::Color::Red) {
                color_words.push("red");
            }
            if colors.contains(crate::color::Color::Green) {
                color_words.push("green");
            }
            if !color_words.is_empty() {
                parts.push(color_words.join(" or "));
            }
        }
        for constraint in &self.tagged_constraints {
            match constraint.relation {
                TaggedOpbjectRelation::IsTaggedObject => match constraint.tag.as_str() {
                    "it" => parts.push("that".to_string()),
                    "enchanted" => parts.push("enchanted".to_string()),
                    "equipped" => parts.push("equipped".to_string()),
                    _ => {}
                },
                TaggedOpbjectRelation::IsNotTaggedObject => {
                    parts.push("other".to_string());
                }
                TaggedOpbjectRelation::SameNameAsTagged => {
                    post_noun_qualifiers.push("with the same name as that object".to_string());
                }
                TaggedOpbjectRelation::SameControllerAsTagged => {
                    post_noun_qualifiers.push("controlled by that object's controller".to_string());
                }
                TaggedOpbjectRelation::SharesColorWithTagged => {
                    post_noun_qualifiers.push("that shares a color with that object".to_string());
                }
                TaggedOpbjectRelation::SharesSubtypeWithTagged => {
                    post_noun_qualifiers
                        .push("that shares a creature type with that object".to_string());
                }
                TaggedOpbjectRelation::SharesCardType => {
                    post_noun_qualifiers.push("that shares a card type with that object".to_string());
                }
                TaggedOpbjectRelation::SameStableId => {}
            }
        }
        if !self.supertypes.is_empty() {
            for supertype in &self.supertypes {
                parts.push(format!("{supertype:?}").to_ascii_lowercase());
            }
        }
        if !self.excluded_card_types.is_empty() {
            for card_type in &self.excluded_card_types {
                parts.push(format!("non{}", describe_card_type_word(*card_type)));
            }
        }
        if !self.excluded_supertypes.is_empty() {
            for supertype in &self.excluded_supertypes {
                parts.push(format!(
                    "non{}",
                    format!("{supertype:?}").to_ascii_lowercase()
                ));
            }
        }
        if !self.excluded_subtypes.is_empty() {
            let mut remaining = self.excluded_subtypes.clone();
            let outlaw_pack = [
                Subtype::Assassin,
                Subtype::Mercenary,
                Subtype::Pirate,
                Subtype::Rogue,
                Subtype::Warlock,
            ];
            if outlaw_pack
                .iter()
                .all(|subtype| remaining.contains(subtype))
            {
                parts.push("non-outlaw".to_string());
                remaining.retain(|subtype| !outlaw_pack.contains(subtype));
            }
            for subtype in &remaining {
                parts.push(format!("non-{subtype:?}"));
            }
        }
        if !self.excluded_colors.is_empty() {
            if self.excluded_colors.contains(crate::color::Color::White) {
                parts.push("nonwhite".to_string());
            }
            if self.excluded_colors.contains(crate::color::Color::Blue) {
                parts.push("nonblue".to_string());
            }
            if self.excluded_colors.contains(crate::color::Color::Black) {
                parts.push("nonblack".to_string());
            }
            if self.excluded_colors.contains(crate::color::Color::Red) {
                parts.push("nonred".to_string());
            }
            if self.excluded_colors.contains(crate::color::Color::Green) {
                parts.push("nongreen".to_string());
            }
        }
        if self.colorless {
            parts.push("colorless".to_string());
        }
        if self.multicolored {
            parts.push("multicolored".to_string());
        }
        if self.monocolored {
            parts.push("monocolored".to_string());
        }
        if self.historic {
            parts.push("historic".to_string());
        }
        if self.nonhistoric {
            post_noun_qualifiers.push("that's not historic".to_string());
        }
        if self.is_commander {
            parts.push("commander".to_string());
        }
        if self.attacking && self.blocking {
            parts.push("attacking/blocking".to_string());
        } else if self.attacking {
            parts.push("attacking".to_string());
        } else if self.blocking {
            parts.push("blocking".to_string());
        }
        if self.tapped && self.untapped {
            parts.push("tapped/untapped".to_string());
        } else if self.tapped {
            parts.push("tapped".to_string());
        } else if self.untapped {
            parts.push("untapped".to_string());
        }
        if self.entered_since_your_last_turn_ended {
            post_noun_qualifiers.push("that entered since your last turn ended".to_string());
        }

        let subtype_implies_type = !self.subtypes.is_empty()
            && matches!(self.zone, None | Some(Zone::Battlefield))
            && self.all_card_types.is_empty()
            && self.card_types.is_empty();

        let has_all_permanent_types = {
            let required = [
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ];
            self.card_types.len() == required.len()
                && required
                    .iter()
                    .all(|card_type| self.card_types.contains(card_type))
        };

        let mut type_phrase = if !self.all_card_types.is_empty() {
            Some((
                true,
                self.all_card_types
                    .iter()
                    .map(|t| format!("{:?}", t).to_lowercase())
                    .collect::<Vec<_>>()
                    .join(" "),
            ))
        } else if !self.card_types.is_empty() {
            if has_all_permanent_types {
                Some((true, "permanent".to_string()))
            } else {
                Some((
                    true,
                    self.card_types
                        .iter()
                        .map(|t| format!("{:?}", t).to_lowercase())
                        .collect::<Vec<_>>()
                        .join(" or "),
                ))
            }
        } else if !self.token && !subtype_implies_type {
            // Default noun depends on zone context.
            let default_noun = if self.source {
                "source"
            } else {
                match self.zone {
                    Some(Zone::Battlefield) | None => "permanent",
                    Some(Zone::Stack) => "spell",
                    Some(Zone::Graveyard)
                    | Some(Zone::Hand)
                    | Some(Zone::Library)
                    | Some(Zone::Exile)
                    | Some(Zone::Command) => "card",
                }
            };
            Some((false, default_noun.to_string()))
        } else {
            None
        };

        let subtype_phrase = if !self.subtypes.is_empty() {
            Some(
                self.subtypes
                    .iter()
                    .map(|t| format!("{:?}", t))
                    .collect::<Vec<_>>()
                    .join(" or "),
            )
        } else {
            None
        };

        if let Some((type_is_card_type, phrase)) = type_phrase.as_mut()
            && *type_is_card_type
            && matches!(
                self.zone,
                Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
            )
            && !phrase.ends_with(" card")
        {
            phrase.push_str(" card");
        }

        let creature_only = self.all_card_types.is_empty()
            && self.card_types.len() == 1
            && self.card_types[0] == CardType::Creature;
        if self.type_or_subtype_union {
            match (type_phrase, subtype_phrase) {
                (Some((_, type_phrase)), Some(subtype_phrase)) => {
                    parts.push(format!("{type_phrase} or {subtype_phrase}"));
                }
                (Some((_, type_phrase)), None) => parts.push(type_phrase),
                (None, Some(subtype_phrase)) => parts.push(subtype_phrase),
                (None, None) => {}
            }
        } else {
            match (type_phrase, subtype_phrase) {
                (Some((_, type_phrase)), Some(subtype_phrase)) if creature_only => {
                    parts.push(subtype_phrase);
                    parts.push(type_phrase);
                }
                (Some((_, type_phrase)), Some(subtype_phrase)) => {
                    parts.push(type_phrase);
                    parts.push(subtype_phrase);
                }
                (Some((_, type_phrase)), None) => parts.push(type_phrase),
                (None, Some(subtype_phrase)) => parts.push(subtype_phrase),
                (None, None) => {}
            }
        }
        if append_token_after_type {
            parts.push("token".to_string());
        }
        if !post_noun_qualifiers.is_empty() {
            parts.extend(post_noun_qualifiers);
        }

        // Handle name
        if let Some(ref name) = self.name {
            return format!("a {} named {}", parts.join(" "), name);
        }

        if let Some(ref power) = self.power {
            let label = match self.power_reference {
                PtReference::Effective => "power",
                PtReference::Base => "base power",
            };
            parts.push(format!("with {label} {}", describe_comparison(power)));
        }
        if let Some(ref toughness) = self.toughness {
            let label = match self.toughness_reference {
                PtReference::Effective => "toughness",
                PtReference::Base => "base toughness",
            };
            parts.push(format!("with {label} {}", describe_comparison(toughness)));
        }
        if let Some(ref mana_value) = self.mana_value {
            parts.push(format!(
                "with mana value {}",
                describe_comparison(mana_value)
            ));
        }
        for ability in &self.static_abilities {
            if let Some(label) = describe_filter_static_ability(*ability) {
                parts.push(format!("with {}", label));
            }
        }
        for marker in &self.custom_static_markers {
            parts.push(format!("with {}", marker.to_ascii_lowercase()));
        }
        for ability in &self.excluded_static_abilities {
            if let Some(label) = describe_filter_static_ability(*ability) {
                parts.push(format!("without {}", label));
            }
        }
        for marker in &self.excluded_custom_static_markers {
            parts.push(format!("without {}", marker.to_ascii_lowercase()));
        }
        if let Some(counter_requirement) = self.with_counter {
            parts.push(format!(
                "with {} on it",
                describe_counter_constraint(counter_requirement)
            ));
        }
        if let Some(counter_exclusion) = self.without_counter {
            parts.push(format!(
                "without {} on it",
                describe_counter_constraint(counter_exclusion)
            ));
        }
        if let Some(kind) = self.alternative_cast {
            parts.push(format!("with {}", describe_alternative_cast_kind(kind)));
        }
        if self.has_tap_activated_ability {
            parts.push("that has an activated ability with {T} in its cost".to_string());
        }

        if let Some(zone) = self.zone {
            let zone_name = match zone {
                Zone::Battlefield => None,
                Zone::Graveyard => Some("graveyard"),
                Zone::Hand => Some("hand"),
                Zone::Library => Some("library"),
                Zone::Exile => Some("exile"),
                Zone::Stack => None,
                Zone::Command => Some("command zone"),
            };
            if let Some(zone_name) = zone_name {
                if let Some(owner) = &self.owner {
                    parts.push(format!(
                        "in {} {}",
                        describe_possessive_player_filter(owner),
                        zone_name
                    ));
                } else {
                    parts.push(format!("in {}", zone_name));
                }
            } else if zone == Zone::Stack {
                // "on stack" is usually implicit in Oracle text (e.g., "target spell").
                // Avoid adding it to reduce render-only mismatches.
            }
        }
        if let Some(suffix) = controller_suffix {
            parts.push(suffix);
        }
        if let Some(suffix) = owner_suffix {
            parts.push(suffix);
        }

        let mut target_fragments = Vec::new();
        if let Some(player_filter) = &self.targets_player {
            target_fragments.push(describe_player_filter(player_filter));
        }
        if let Some(object_filter) = &self.targets_object {
            target_fragments.push(object_filter.description());
        }
        if !target_fragments.is_empty() {
            let target_text = if target_fragments.len() == 2 {
                format!("{} and {}", target_fragments[0], target_fragments[1])
            } else {
                target_fragments[0].clone()
            };
            parts.push(format!("that targets {target_text}"));
        }

        parts.join(" ")
    }
}

fn plus_minus_counter_delta(counters: &std::collections::HashMap<CounterType, u32>) -> i32 {
    let plus = counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0) as i32;
    let minus = counters
        .get(&CounterType::MinusOneMinusOne)
        .copied()
        .unwrap_or(0) as i32;
    plus - minus
}

fn object_base_power_for_filter(object: &Object) -> Option<i32> {
    if let Some(power) = object.power() {
        return Some(power - plus_minus_counter_delta(&object.counters));
    }
    object.base_power.as_ref().map(|pt| pt.base_value())
}

fn object_base_toughness_for_filter(object: &Object) -> Option<i32> {
    if let Some(toughness) = object.toughness() {
        return Some(toughness - plus_minus_counter_delta(&object.counters));
    }
    object.base_toughness.as_ref().map(|pt| pt.base_value())
}

fn resolve_object_power_for_filter(
    object: &Object,
    game: &crate::game_state::GameState,
    reference: PtReference,
    allow_calculated_pt: bool,
) -> Option<i32> {
    match reference {
        PtReference::Base => object_base_power_for_filter(object),
        PtReference::Effective => {
            if allow_calculated_pt {
                game.calculated_power(object.id).or_else(|| object.power())
            } else {
                object.power()
            }
        }
    }
}

fn resolve_object_toughness_for_filter(
    object: &Object,
    game: &crate::game_state::GameState,
    reference: PtReference,
    allow_calculated_pt: bool,
) -> Option<i32> {
    match reference {
        PtReference::Base => object_base_toughness_for_filter(object),
        PtReference::Effective => {
            if allow_calculated_pt {
                game.calculated_toughness(object.id)
                    .or_else(|| object.toughness())
            } else {
                object.toughness()
            }
        }
    }
}

fn snapshot_base_power_for_filter(snapshot: &crate::snapshot::ObjectSnapshot) -> Option<i32> {
    if let Some(power) = snapshot.power {
        return Some(power - plus_minus_counter_delta(&snapshot.counters));
    }
    snapshot.base_power
}

fn snapshot_base_toughness_for_filter(snapshot: &crate::snapshot::ObjectSnapshot) -> Option<i32> {
    if let Some(toughness) = snapshot.toughness {
        return Some(toughness - plus_minus_counter_delta(&snapshot.counters));
    }
    snapshot.base_toughness
}

fn resolve_snapshot_power_for_filter(
    snapshot: &crate::snapshot::ObjectSnapshot,
    reference: PtReference,
) -> Option<i32> {
    match reference {
        PtReference::Effective => snapshot.power,
        PtReference::Base => snapshot_base_power_for_filter(snapshot),
    }
}

fn resolve_snapshot_toughness_for_filter(
    snapshot: &crate::snapshot::ObjectSnapshot,
    reference: PtReference,
) -> Option<i32> {
    match reference {
        PtReference::Effective => snapshot.toughness,
        PtReference::Base => snapshot_base_toughness_for_filter(snapshot),
    }
}

fn describe_possessive_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "a player's".to_string(),
        PlayerFilter::You => "your".to_string(),
        PlayerFilter::Opponent => "an opponent's".to_string(),
        PlayerFilter::Teammate => "a teammate's".to_string(),
        PlayerFilter::Active => "the active player's".to_string(),
        PlayerFilter::Defending => "the defending player's".to_string(),
        PlayerFilter::Attacking => "an attacking player's".to_string(),
        PlayerFilter::DamagedPlayer => "the damaged player's".to_string(),
        PlayerFilter::Specific(_) => "that player's".to_string(),
        PlayerFilter::IteratedPlayer => "that player's".to_string(),
        PlayerFilter::Target(inner) => {
            let base = match inner.as_ref() {
                PlayerFilter::Any => "target player".to_string(),
                other => format!("target {}", describe_player_filter(other)),
            };
            format!("{base}'s")
        }
        PlayerFilter::ControllerOf(_) => "that object's controller's".to_string(),
        PlayerFilter::OwnerOf(_) => "that object's owner's".to_string(),
    }
}

fn describe_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "player".to_string(),
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::Opponent => "opponent".to_string(),
        PlayerFilter::Teammate => "teammate".to_string(),
        PlayerFilter::Active => "active player".to_string(),
        PlayerFilter::Defending => "defending player".to_string(),
        PlayerFilter::Attacking => "attacking player".to_string(),
        PlayerFilter::DamagedPlayer => "damaged player".to_string(),
        PlayerFilter::Specific(_) => "player".to_string(),
        PlayerFilter::IteratedPlayer => "that player".to_string(),
        PlayerFilter::Target(inner) => format!("target {}", describe_player_filter(inner)),
        PlayerFilter::ControllerOf(_) => "controller".to_string(),
        PlayerFilter::OwnerOf(_) => "owner".to_string(),
    }
}

fn describe_card_type_word(card_type: CardType) -> &'static str {
    match card_type {
        CardType::Artifact => "artifact",
        CardType::Battle => "battle",
        CardType::Creature => "creature",
        CardType::Enchantment => "enchantment",
        CardType::Instant => "instant",
        CardType::Kindred => "kindred",
        CardType::Land => "land",
        CardType::Planeswalker => "planeswalker",
        CardType::Sorcery => "sorcery",
    }
}

fn alternative_cast_matches_kind(
    method: &crate::alternative_cast::AlternativeCastingMethod,
    kind: AlternativeCastKind,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    match (kind, method) {
        (AlternativeCastKind::Flashback, AlternativeCastingMethod::Flashback { .. }) => true,
        (AlternativeCastKind::JumpStart, AlternativeCastingMethod::JumpStart) => true,
        (AlternativeCastKind::Escape, AlternativeCastingMethod::Escape { .. }) => true,
        (AlternativeCastKind::Madness, AlternativeCastingMethod::Madness { .. }) => true,
        (AlternativeCastKind::Miracle, AlternativeCastingMethod::Miracle { .. }) => true,
        _ => false,
    }
}

fn object_has_alternative_cast_kind(
    object: &Object,
    kind: AlternativeCastKind,
    game: &crate::game_state::GameState,
    ctx: &FilterContext,
) -> bool {
    if object
        .alternative_casts
        .iter()
        .any(|method| alternative_cast_matches_kind(method, kind))
    {
        return true;
    }

    // Include temporary grants (e.g., Snapcaster Mage granting flashback).
    let Some(player) = ctx.you else {
        return false;
    };
    game.grant_registry
        .granted_alternative_casts_for_card(game, object.id, object.zone, player)
        .iter()
        .any(|grant| alternative_cast_matches_kind(&grant.method, kind))
}

fn object_has_static_ability_id(object: &Object, ability_id: StaticAbilityId) -> bool {
    use crate::ability::AbilityKind;

    let has_regular = object.abilities.iter().any(|ability| {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            static_ability.id() == ability_id
        } else {
            false
        }
    });
    if has_regular {
        return true;
    }

    object
        .level_granted_abilities()
        .iter()
        .any(|ability| ability.id() == ability_id)
}

fn object_has_custom_static_marker(object: &Object, marker: &str) -> bool {
    use crate::ability::AbilityKind;

    let has_regular = object.abilities.iter().any(|ability| {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            static_ability.id() == StaticAbilityId::Custom
                && static_ability.display().eq_ignore_ascii_case(marker)
        } else {
            false
        }
    });
    if has_regular {
        return true;
    }

    object.level_granted_abilities().iter().any(|ability| {
        ability.id() == StaticAbilityId::Custom && ability.display().eq_ignore_ascii_case(marker)
    })
}

fn object_has_tap_activated_ability(object: &Object) -> bool {
    use crate::ability::AbilityKind;
    object.abilities.iter().any(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => activated
            .mana_cost
            .costs()
            .iter()
            .any(|cost| cost.requires_tap()),
        AbilityKind::Mana(mana) => mana
            .mana_cost
            .costs()
            .iter()
            .any(|cost| cost.requires_tap()),
        _ => false,
    })
}

fn snapshot_has_static_ability_id(
    snapshot: &crate::snapshot::ObjectSnapshot,
    ability_id: StaticAbilityId,
) -> bool {
    snapshot.has_static_ability_id(ability_id)
}

fn snapshot_has_custom_static_marker(
    snapshot: &crate::snapshot::ObjectSnapshot,
    marker: &str,
) -> bool {
    use crate::ability::AbilityKind;

    snapshot.abilities.iter().any(|ability| {
        if let AbilityKind::Static(static_ability) = &ability.kind {
            static_ability.id() == StaticAbilityId::Custom
                && static_ability.display().eq_ignore_ascii_case(marker)
        } else {
            false
        }
    })
}

fn snapshot_has_tap_activated_ability(snapshot: &crate::snapshot::ObjectSnapshot) -> bool {
    use crate::ability::AbilityKind;
    snapshot
        .abilities
        .iter()
        .any(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .mana_cost
                .costs()
                .iter()
                .any(|cost| cost.requires_tap()),
            AbilityKind::Mana(mana) => mana
                .mana_cost
                .costs()
                .iter()
                .any(|cost| cost.requires_tap()),
            _ => false,
        })
}

fn describe_counter_constraint(constraint: CounterConstraint) -> String {
    match constraint {
        CounterConstraint::Any => "a counter".to_string(),
        CounterConstraint::Typed(counter_type) => {
            format!("a {} counter", describe_counter_type(counter_type))
        }
    }
}

fn describe_counter_type(counter_type: CounterType) -> String {
    match counter_type {
        CounterType::PlusOnePlusOne => "+1/+1".to_string(),
        CounterType::MinusOneMinusOne => "-1/-1".to_string(),
        other => format!("{other:?}").to_ascii_lowercase(),
    }
}

fn describe_alternative_cast_kind(kind: AlternativeCastKind) -> &'static str {
    match kind {
        AlternativeCastKind::Flashback => "flashback",
        AlternativeCastKind::JumpStart => "jump-start",
        AlternativeCastKind::Escape => "escape",
        AlternativeCastKind::Madness => "madness",
        AlternativeCastKind::Miracle => "miracle",
    }
}

fn describe_filter_static_ability(ability_id: StaticAbilityId) -> Option<&'static str> {
    use StaticAbilityId::*;
    match ability_id {
        Flying => Some("flying"),
        FirstStrike => Some("first strike"),
        DoubleStrike => Some("double strike"),
        Deathtouch => Some("deathtouch"),
        Defender => Some("defender"),
        Flash => Some("flash"),
        Haste => Some("haste"),
        Hexproof => Some("hexproof"),
        Indestructible => Some("indestructible"),
        Intimidate => Some("intimidate"),
        Lifelink => Some("lifelink"),
        Menace => Some("menace"),
        Reach => Some("reach"),
        Shroud => Some("shroud"),
        Trample => Some("trample"),
        Vigilance => Some("vigilance"),
        Fear => Some("fear"),
        Flanking => Some("flanking"),
        Landwalk => Some("landwalk"),
        Bloodthirst => Some("bloodthirst"),
        Morph => Some("morph"),
        Megamorph => Some("megamorph"),
        Shadow => Some("shadow"),
        Horsemanship => Some("horsemanship"),
        Wither => Some("wither"),
        Infect => Some("infect"),
        Changeling => Some("changeling"),
        _ => None,
    }
}

fn describe_comparison(cmp: &Comparison) -> String {
    match cmp {
        Comparison::Equal(v) => format!("{v}"),
        Comparison::NotEqual(v) => format!("not equal to {v}"),
        Comparison::LessThan(v) => format!("less than {v}"),
        Comparison::LessThanOrEqual(v) => format!("{v} or less"),
        Comparison::GreaterThan(v) => format!("greater than {v}"),
        Comparison::GreaterThanOrEqual(v) => format!("{v} or greater"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison() {
        assert!(Comparison::Equal(5).satisfies(5));
        assert!(!Comparison::Equal(5).satisfies(4));

        assert!(Comparison::LessThanOrEqual(2).satisfies(2));
        assert!(Comparison::LessThanOrEqual(2).satisfies(1));
        assert!(!Comparison::LessThanOrEqual(2).satisfies(3));

        assert!(Comparison::GreaterThan(3).satisfies(4));
        assert!(!Comparison::GreaterThan(3).satisfies(3));
    }

    #[test]
    fn test_creature_filter() {
        let filter = ObjectFilter::creature();
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
    }

    #[test]
    fn test_filter_chaining() {
        let filter = ObjectFilter::creature()
            .you_control()
            .other()
            .with_power(Comparison::GreaterThanOrEqual(3));

        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(filter.other);
        assert!(filter.power.is_some());
    }

    #[test]
    fn test_nonland_filter() {
        let filter = ObjectFilter::nonland();
        assert!(filter.excluded_card_types.contains(&CardType::Land));
    }

    #[test]
    fn test_filter_with_subtypes() {
        let filter = ObjectFilter::creature()
            .with_subtype(crate::types::Subtype::Elf)
            .with_subtype(crate::types::Subtype::Warrior);

        assert_eq!(filter.subtypes.len(), 2);
    }

    #[test]
    fn test_filter_description_includes_positive_colors() {
        let filter =
            ObjectFilter::creature().with_colors(ColorSet::from_color(crate::color::Color::Blue));
        assert_eq!(filter.description(), "blue creature");
    }

    #[test]
    fn test_filter_description_includes_tapped_state() {
        let filter = ObjectFilter::creature().tapped();
        assert_eq!(filter.description(), "tapped creature");
    }

    #[test]
    fn test_filter_description_includes_all_card_types() {
        let filter = ObjectFilter::default()
            .with_all_type(CardType::Artifact)
            .with_all_type(CardType::Creature);
        assert_eq!(filter.description(), "artifact creature");
    }

    #[test]
    fn test_filter_description_includes_excluded_subtypes() {
        let filter = ObjectFilter::creature()
            .without_subtype(crate::types::Subtype::Vampire)
            .without_subtype(crate::types::Subtype::Werewolf)
            .without_subtype(crate::types::Subtype::Zombie);
        assert_eq!(
            filter.description(),
            "non-Vampire non-Werewolf non-Zombie creature"
        );
    }

    #[test]
    fn test_filter_description_includes_excluded_colors() {
        let filter = ObjectFilter::creature().without_colors(
            ColorSet::from_color(crate::color::Color::Black)
                .union(ColorSet::from_color(crate::color::Color::Red)),
        );
        assert_eq!(filter.description(), "nonblack nonred creature");
    }

    #[test]
    fn test_filter_description_includes_entered_since_last_turn_ended_clause() {
        let filter = ObjectFilter {
            card_types: vec![CardType::Creature],
            entered_since_your_last_turn_ended: true,
            ..Default::default()
        };
        assert_eq!(
            filter.description(),
            "creature that entered since your last turn ended"
        );
    }

    #[test]
    fn test_filter_description_includes_commander_owner_and_controller_distinction() {
        let filter = ObjectFilter::creature()
            .commander()
            .owned_by(PlayerFilter::You)
            .controlled_by(PlayerFilter::Opponent);
        assert_eq!(
            filter.description(),
            "an opponent's commander creature you own"
        );
    }

    #[test]
    fn test_player_filter_matching() {
        let you = PlayerId::from_index(0);
        let opponent = PlayerId::from_index(1);

        let ctx = FilterContext::new(you).with_opponents(vec![opponent]);

        assert!(PlayerFilter::Any.matches_player(you, &ctx));
        assert!(PlayerFilter::Any.matches_player(opponent, &ctx));

        assert!(PlayerFilter::You.matches_player(you, &ctx));
        assert!(!PlayerFilter::You.matches_player(opponent, &ctx));

        assert!(!PlayerFilter::Opponent.matches_player(you, &ctx));
        assert!(PlayerFilter::Opponent.matches_player(opponent, &ctx));

        assert!(PlayerFilter::Specific(you).matches_player(you, &ctx));
        assert!(!PlayerFilter::Specific(you).matches_player(opponent, &ctx));
    }

    #[test]
    fn test_excluded_supertypes_builder() {
        use crate::types::Supertype;

        let filter = ObjectFilter::land().without_supertype(Supertype::Basic);
        assert_eq!(filter.excluded_supertypes, vec![Supertype::Basic]);
    }

    #[test]
    fn test_nonbasic_shorthand() {
        use crate::types::Supertype;

        let filter = ObjectFilter::land().nonbasic();
        assert_eq!(filter.excluded_supertypes, vec![Supertype::Basic]);
    }

    #[test]
    fn test_excluded_supertypes_matching() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::CardId;
        use crate::object::Object;
        use crate::types::Supertype;

        let p0 = PlayerId::from_index(0);

        // Create a basic land
        let basic_forest_card = CardBuilder::new(CardId::from_raw(1), "Forest")
            .card_types(vec![CardType::Land])
            .supertypes(vec![Supertype::Basic])
            .subtypes(vec![crate::types::Subtype::Forest])
            .build();
        let basic_forest = Object::from_card(
            crate::ids::ObjectId::from_raw(1),
            &basic_forest_card,
            p0,
            Zone::Battlefield,
        );

        // Create a nonbasic land
        let nonbasic_land_card = CardBuilder::new(CardId::from_raw(2), "Steam Vents")
            .card_types(vec![CardType::Land])
            .subtypes(vec![
                crate::types::Subtype::Island,
                crate::types::Subtype::Mountain,
            ])
            .build();
        let nonbasic_land = Object::from_card(
            crate::ids::ObjectId::from_raw(2),
            &nonbasic_land_card,
            p0,
            Zone::Battlefield,
        );

        // Filter for nonbasic lands (excludes Basic supertype)
        let nonbasic_filter = ObjectFilter::land().nonbasic();
        let ctx = FilterContext::new(p0);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        // Basic land should NOT match (has Basic supertype which is excluded)
        assert!(
            !nonbasic_filter.matches(&basic_forest, &ctx, &game),
            "Basic Forest should not match nonbasic filter"
        );

        // Nonbasic land SHOULD match (doesn't have Basic supertype)
        assert!(
            nonbasic_filter.matches(&nonbasic_land, &ctx, &game),
            "Steam Vents should match nonbasic filter"
        );
    }

    #[test]
    fn test_blood_moon_filter_for_nonbasic_lands() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::CardId;
        use crate::types::Supertype;

        let p0 = PlayerId::from_index(0);

        // Blood Moon filter: nonbasic lands on the battlefield
        let blood_moon_filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Land],
            excluded_supertypes: vec![Supertype::Basic],
            ..Default::default()
        };

        // Create basic Plains
        let plains_card = CardBuilder::new(CardId::from_raw(1), "Plains")
            .card_types(vec![CardType::Land])
            .supertypes(vec![Supertype::Basic])
            .subtypes(vec![crate::types::Subtype::Plains])
            .build();
        let plains = Object::from_card(
            crate::ids::ObjectId::from_raw(1),
            &plains_card,
            p0,
            Zone::Battlefield,
        );

        // Create Breeding Pool (nonbasic)
        let breeding_pool_card = CardBuilder::new(CardId::from_raw(2), "Breeding Pool")
            .card_types(vec![CardType::Land])
            .subtypes(vec![
                crate::types::Subtype::Forest,
                crate::types::Subtype::Island,
            ])
            .build();
        let breeding_pool = Object::from_card(
            crate::ids::ObjectId::from_raw(2),
            &breeding_pool_card,
            p0,
            Zone::Battlefield,
        );

        let ctx = FilterContext::new(p0);
        let game = GameState::new(vec!["Alice".to_string()], 20);

        // Blood Moon should NOT affect basic Plains
        assert!(
            !blood_moon_filter.matches(&plains, &ctx, &game),
            "Blood Moon filter should not match basic Plains"
        );

        // Blood Moon SHOULD affect Breeding Pool
        assert!(
            blood_moon_filter.matches(&breeding_pool, &ctx, &game),
            "Blood Moon filter should match Breeding Pool"
        );
    }

    #[test]
    fn test_commander_filter_matches_true_commander_regardless_of_ctx_owner_list() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::{CardId, ObjectId};
        use crate::object::Object;

        let you = PlayerId::from_index(0);
        let opponent = PlayerId::from_index(1);

        let commander_card = CardBuilder::new(CardId::from_raw(99), "Opponent Commander")
            .card_types(vec![CardType::Creature])
            .build();
        let commander_obj = Object::from_card(
            ObjectId::from_raw(99),
            &commander_card,
            opponent,
            Zone::Battlefield,
        );

        let mut game = GameState::new(vec!["You".to_string(), "Opponent".to_string()], 20);
        game.add_object(commander_obj.clone());
        game.set_as_commander(commander_obj.id, opponent);

        let filter = ObjectFilter::creature().commander();
        let ctx = FilterContext::new(you).with_your_commanders(Vec::new());
        assert!(
            filter.matches(&commander_obj, &ctx, &game),
            "commander filter should rely on game commander identity, not ctx.your_commanders"
        );
    }

    #[test]
    fn test_historic_and_nonhistoric_filters_match_correctly() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::{CardId, ObjectId};
        use crate::object::Object;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let artifact_card = CardBuilder::new(CardId::from_raw(1), "Mox")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_obj = Object::from_card(
            ObjectId::from_raw(1),
            &artifact_card,
            you,
            Zone::Battlefield,
        );
        game.add_object(artifact_obj.clone());

        let creature_card = CardBuilder::new(CardId::from_raw(2), "Bear")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_obj = Object::from_card(
            ObjectId::from_raw(2),
            &creature_card,
            you,
            Zone::Battlefield,
        );
        game.add_object(creature_obj.clone());

        let ctx = FilterContext::new(you);
        assert!(
            ObjectFilter::permanent()
                .historic()
                .matches(&artifact_obj, &ctx, &game)
        );
        assert!(
            !ObjectFilter::permanent()
                .historic()
                .matches(&creature_obj, &ctx, &game)
        );
        assert!(
            ObjectFilter::permanent()
                .nonhistoric()
                .matches(&creature_obj, &ctx, &game)
        );
        assert!(
            !ObjectFilter::permanent()
                .nonhistoric()
                .matches(&artifact_obj, &ctx, &game)
        );
    }

    #[test]
    fn test_shares_color_with_tagged_constraint() {
        use crate::card::CardBuilder;
        use crate::game_state::GameState;
        use crate::ids::{CardId, ObjectId};
        use crate::object::Object;
        use crate::snapshot::ObjectSnapshot;
        use crate::tag::TagKey;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let red_card = CardBuilder::new(CardId::from_raw(10), "Red Creature")
            .card_types(vec![CardType::Creature])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Red,
            ]]))
            .build();
        let red_obj = Object::from_card(ObjectId::from_raw(10), &red_card, you, Zone::Battlefield);
        game.add_object(red_obj.clone());

        let blue_card = CardBuilder::new(CardId::from_raw(11), "Blue Creature")
            .card_types(vec![CardType::Creature])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Blue,
            ]]))
            .build();
        let blue_obj =
            Object::from_card(ObjectId::from_raw(11), &blue_card, you, Zone::Battlefield);
        game.add_object(blue_obj.clone());

        let mut tagged = std::collections::HashMap::new();
        tagged.insert(
            TagKey::from("it"),
            vec![ObjectSnapshot::from_object(&red_obj, &game)],
        );
        let ctx = FilterContext::new(you).with_tagged_objects(&tagged);
        let filter = ObjectFilter::creature().shares_color_with_tagged("it");

        assert!(filter.matches(&red_obj, &ctx, &game));
        assert!(!filter.matches(&blue_obj, &ctx, &game));
    }

    #[test]
    fn test_base_power_builder_sets_reference() {
        let filter = ObjectFilter::creature().with_base_power(Comparison::LessThanOrEqual(2));
        assert_eq!(filter.power, Some(Comparison::LessThanOrEqual(2)));
        assert_eq!(filter.power_reference, PtReference::Base);
        assert_eq!(
            filter.description(),
            "creature with base power 2 or less"
        );
    }

    #[test]
    fn test_filter_can_match_base_vs_effective_power() {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::game_state::GameState;
        use crate::ids::CardId;
        use crate::object::CounterType;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let card = CardBuilder::new(CardId::from_raw(30), "Counter Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object_id = game.create_object_from_card(&card, you, Zone::Battlefield);
        if let Some(obj) = game.object_mut(object_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 1);
        }

        let obj = game.object(object_id).expect("object should exist");
        let ctx = FilterContext::new(you);

        let effective_filter = ObjectFilter::creature().with_power(Comparison::GreaterThanOrEqual(3));
        let base_filter = ObjectFilter::creature().with_base_power(Comparison::GreaterThanOrEqual(3));

        assert!(
            effective_filter.matches(obj, &ctx, &game),
            "effective power should include +1/+1 counters"
        );
        assert!(
            !base_filter.matches(obj, &ctx, &game),
            "base power should ignore +1/+1 counters"
        );
    }

    #[test]
    fn test_non_recursive_match_avoids_calculated_power() {
        use crate::ability::Ability;
        use crate::card::{CardBuilder, PowerToughness};
        use crate::game_state::GameState;
        use crate::ids::CardId;
        use crate::static_abilities::StaticAbility;

        let you = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["You".to_string()], 20);

        let card = CardBuilder::new(CardId::from_raw(31), "Anthem Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object_id = game.create_object_from_card(&card, you, Zone::Battlefield);
        if let Some(obj) = game.object_mut(object_id) {
            obj.abilities.push(Ability::static_ability(StaticAbility::anthem(
                ObjectFilter::source(),
                2,
                0,
            )));
        }

        let obj = game.object(object_id).expect("object should exist");
        let ctx = FilterContext::new(you);
        let filter = ObjectFilter::creature().with_power(Comparison::GreaterThanOrEqual(4));

        assert!(
            filter.matches(obj, &ctx, &game),
            "regular matching should use calculated power"
        );
        assert!(
            !filter.matches_non_recursive(obj, &ctx, &game),
            "non-recursive matching should avoid layer-calculated power"
        );
    }
}
