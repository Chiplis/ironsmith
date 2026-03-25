//! Modular static ability system for MTG.
//!
//! This module provides a trait-based architecture for static abilities.
//! Each ability type implements the `StaticAbilityKind` trait, allowing for:
//! - Co-located tests with each ability implementation
//! - Self-contained ability logic
//! - Easy addition of new abilities without modifying central code
//! - Scalable to thousands of unique card abilities
//!
//! # Module Structure
//!
//! ```text
//! static_abilities/
//!   mod.rs              - This file, trait definition and StaticAbility wrapper
//!   id.rs               - StaticAbilityId enum for identity checking
//!   keywords.rs         - Simple keyword abilities (Flying, Trample, etc.)
//!   combat.rs           - Combat modifiers (MustAttack, CantBlock, etc.)
//!   protection.rs       - Protection, Hexproof, Ward, Shroud
//!   continuous.rs       - Effect-generating abilities (Anthem, GrantAbility, etc.)
//!   cost_modifiers.rs   - Cost modification (Affinity, Delve, Convoke, etc.)
//!   restrictions.rs     - Game rule restrictions (PlayersCantGainLife, etc.)
//!   characteristics.rs  - Characteristic-defining abilities
//!   misc.rs             - Other abilities
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
//!
//! // Create abilities using convenience constructors
//! let flying = StaticAbility::flying();
//! let anthem = StaticAbility::anthem(filter, 1, 1);
//!
//! // Check ability identity
//! if ability.id() == StaticAbilityId::Flying {
//!     // Handle flying
//! }
//!
//! // Generate continuous effects
//! let effects = ability.generate_effects(source, controller);
//! ```

mod characteristics;
mod combat;
mod continuous;
mod cost_modifiers;
mod id;
mod keywords;
mod misc;
mod protection;
mod restrictions;
mod text_utils;

// Re-export the ID enum
pub use id::StaticAbilityId;

// Re-export ability structs for direct construction
pub use characteristics::*;
pub use combat::*;
pub use continuous::*;
pub use cost_modifiers::*;
pub use keywords::*;
pub use misc::*;
pub use protection::*;
pub use restrictions::*;

pub(crate) use continuous::resolve_anthem_count_expression;
use std::sync::Arc;

use crate::continuous::ContinuousEffect;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Keyword granted to a spell conditionally from its own rules text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalSpellKeywordKind {
    Flash,
    Cascade,
}

/// Graveyard aggregate used by conditional spell-keyword checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraveyardCountMetric {
    CardTypes,
    ManaValues,
}

/// Conditional spell-keyword configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalSpellKeywordSpec {
    pub keyword: ConditionalSpellKeywordKind,
    pub metric: GraveyardCountMetric,
    pub threshold: u32,
}

/// Timing window for "Cast this spell only ..." restrictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThisSpellCastTiming {
    DuringDeclareAttackersStep,
    DuringCombat,
    DuringCombatBeforeBlockersAreDeclared,
    DuringCombatAfterBlockersAreDeclared,
    DuringCombatOnYourTurnBeforeBlockersAreDeclared,
    DuringCombatOnOpponentsTurn,
    BeforeAttackersAreDeclared,
    BeforeCombatDamageStep,
    DuringOpponentsUpkeep,
    DuringOpponentsTurnAfterUpkeep,
    DuringYourEndStep,
    AfterCombat,
}

/// Extra condition for "Cast this spell only ..." restrictions.
#[derive(Debug, Clone, PartialEq)]
pub enum ThisSpellCastCondition {
    /// "only if you've been attacked this step"
    YouWereAttackedThisStep,
    /// "only if [player] cast N or more [matching] spells this turn"
    PlayerCastSpellThisTurnOrMore {
        player: crate::target::PlayerFilter,
        spell_filter: crate::target::ObjectFilter,
        count: u32,
    },
    /// "only if a creature is attacking you"
    CreatureIsAttackingYou,
    /// "only if no permanents named <name> are on the battlefield"
    NoPermanentsNamedOnBattlefield(&'static str),
    /// "only if you control N or more matching permanents"
    YouControlAtLeast {
        filter: crate::target::ObjectFilter,
        count: u32,
    },
    /// "only if you control fewer creatures than each opponent"
    YouControlFewerCreaturesThanEachOpponent,
    /// "only if you control N or more permanents whose names contain <word>"
    YouControlNameWordOrMore { word: &'static str, count: u32 },
}

/// Cast-time restriction for "Cast this spell only ..." lines.
#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCastRestrictionKind {
    pub timing: Option<ThisSpellCastTiming>,
    pub condition: Option<ThisSpellCastCondition>,
}

impl ThisSpellCastRestrictionKind {
    pub fn timing(timing: ThisSpellCastTiming) -> Self {
        Self {
            timing: Some(timing),
            condition: None,
        }
    }

    pub fn timing_and_condition(
        timing: ThisSpellCastTiming,
        condition: ThisSpellCastCondition,
    ) -> Self {
        Self {
            timing: Some(timing),
            condition: Some(condition),
        }
    }

    pub fn condition(condition: ThisSpellCastCondition) -> Self {
        Self {
            timing: None,
            condition: Some(condition),
        }
    }

    pub fn during_declare_attackers_step() -> Self {
        Self::timing(ThisSpellCastTiming::DuringDeclareAttackersStep)
    }

    pub fn during_declare_attackers_step_if_you_were_attacked_this_step() -> Self {
        Self::timing_and_condition(
            ThisSpellCastTiming::DuringDeclareAttackersStep,
            ThisSpellCastCondition::YouWereAttackedThisStep,
        )
    }

    pub fn during_combat() -> Self {
        Self::timing(ThisSpellCastTiming::DuringCombat)
    }

    pub fn during_combat_before_blockers_are_declared() -> Self {
        Self::timing(ThisSpellCastTiming::DuringCombatBeforeBlockersAreDeclared)
    }

    pub fn during_combat_after_blockers_are_declared() -> Self {
        Self::timing(ThisSpellCastTiming::DuringCombatAfterBlockersAreDeclared)
    }

    pub fn during_combat_on_your_turn_before_blockers_are_declared() -> Self {
        Self::timing(ThisSpellCastTiming::DuringCombatOnYourTurnBeforeBlockersAreDeclared)
    }

    pub fn during_combat_on_opponents_turn() -> Self {
        Self::timing(ThisSpellCastTiming::DuringCombatOnOpponentsTurn)
    }

    pub fn before_attackers_are_declared() -> Self {
        Self::timing(ThisSpellCastTiming::BeforeAttackersAreDeclared)
    }

    pub fn before_combat_damage_step() -> Self {
        Self::timing(ThisSpellCastTiming::BeforeCombatDamageStep)
    }

    pub fn during_opponents_upkeep() -> Self {
        Self::timing(ThisSpellCastTiming::DuringOpponentsUpkeep)
    }

    pub fn during_opponents_turn_after_upkeep() -> Self {
        Self::timing(ThisSpellCastTiming::DuringOpponentsTurnAfterUpkeep)
    }

    pub fn during_your_end_step() -> Self {
        Self::timing(ThisSpellCastTiming::DuringYourEndStep)
    }

    pub fn if_you_cast_another_spell_this_turn() -> Self {
        Self::condition(ThisSpellCastCondition::PlayerCastSpellThisTurnOrMore {
            player: crate::target::PlayerFilter::You,
            spell_filter: crate::target::ObjectFilter::default(),
            count: 1,
        })
    }

    pub fn if_you_cast_another_green_spell_this_turn() -> Self {
        Self::condition(ThisSpellCastCondition::PlayerCastSpellThisTurnOrMore {
            player: crate::target::PlayerFilter::You,
            spell_filter: crate::target::ObjectFilter::default().with_colors(
                crate::color::ColorSet::from_color(crate::color::Color::Green),
            ),
            count: 1,
        })
    }

    pub fn if_opponent_cast_creature_spell_this_turn() -> Self {
        Self::condition(ThisSpellCastCondition::PlayerCastSpellThisTurnOrMore {
            player: crate::target::PlayerFilter::Opponent,
            spell_filter: crate::target::ObjectFilter::default()
                .with_type(crate::types::CardType::Creature),
            count: 1,
        })
    }

    pub fn if_creature_is_attacking_you() -> Self {
        Self::condition(ThisSpellCastCondition::CreatureIsAttackingYou)
    }

    pub fn after_combat() -> Self {
        Self::timing(ThisSpellCastTiming::AfterCombat)
    }

    pub fn if_no_permanents_named_on_battlefield(name: &'static str) -> Self {
        Self::condition(ThisSpellCastCondition::NoPermanentsNamedOnBattlefield(name))
    }

    pub fn if_you_control_snow_land() -> Self {
        Self::condition(ThisSpellCastCondition::YouControlAtLeast {
            filter: crate::target::ObjectFilter::default()
                .with_type(crate::types::CardType::Land)
                .with_supertype(crate::types::Supertype::Snow),
            count: 1,
        })
    }

    pub fn if_you_control_fewer_creatures_than_each_opponent() -> Self {
        Self::condition(ThisSpellCastCondition::YouControlFewerCreaturesThanEachOpponent)
    }

    pub fn if_you_control_subtype_or_more(subtype: crate::types::Subtype, count: u32) -> Self {
        Self::condition(ThisSpellCastCondition::YouControlAtLeast {
            filter: crate::target::ObjectFilter::default().with_subtype(subtype),
            count,
        })
    }

    pub fn if_you_control_name_word_or_more(word: &'static str, count: u32) -> Self {
        Self::condition(ThisSpellCastCondition::YouControlNameWordOrMore { word, count })
    }
}

/// Pregame action available from a card in opening hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PregameActionKind {
    BeginOnBattlefield(PregameBeginOnBattlefieldSpec),
    ChooseColor,
}

/// "You may begin the game with this on the battlefield" pregame action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PregameBeginOnBattlefieldSpec {
    pub require_not_starting_player: bool,
    pub counters: Vec<(crate::object::CounterType, u32)>,
    pub exile_cards_from_hand: usize,
}

/// Trait for static ability behavior.
///
/// All static abilities implement this trait. Each ability is responsible for:
/// - Providing its identity (for equality/matching checks)
/// - Generating continuous effects (if applicable)
/// - Applying game restrictions (if applicable)
/// - Providing display text
///
/// Most abilities only override a few methods - the defaults handle the common case
/// of simple keyword abilities that don't generate effects.
pub trait StaticAbilityKindClone {
    /// Clone this ability into a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn StaticAbilityKind>;
}

impl<T> StaticAbilityKindClone for T
where
    T: StaticAbilityKind + Clone + 'static,
{
    fn clone_boxed(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }
}

pub trait StaticAbilityKind: std::fmt::Debug + Send + Sync + StaticAbilityKindClone {
    /// Get the unique identifier for this ability type.
    ///
    /// Used for identity checks like `ability.id() == StaticAbilityId::Flying`.
    fn id(&self) -> StaticAbilityId;

    /// Human-readable display name for this ability.
    ///
    /// Examples: "Flying", "Protection from red", "Creatures you control get +1/+1"
    fn display(&self) -> String;

    /// Clone this ability while attaching a static condition, when the concrete
    /// ability kind supports native conditional evaluation.
    fn with_static_condition(&self, _condition: crate::ConditionExpr) -> Option<StaticAbility> {
        None
    }

    /// Clone this ability into a boxed trait object.
    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        StaticAbilityKindClone::clone_boxed(self)
    }

    /// Generate continuous effects for this ability.
    ///
    /// Called by the static ability processor to create effects that go through
    /// the layer system. Most abilities return empty (the default).
    ///
    /// Override for: Anthem, GrantAbility, BloodMoon, Humility, etc.
    fn generate_effects(
        &self,
        _source: ObjectId,
        _controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![]
    }

    /// Apply game restrictions for this ability.
    ///
    /// Called when a permanent with this ability is on the battlefield.
    /// Modifies the game's restriction trackers.
    ///
    /// Override for: PlayersCantGainLife, CantAttack, Hexproof, etc.
    fn apply_restrictions(&self, _game: &mut GameState, _source: ObjectId, _controller: PlayerId) {
        // Default: no restrictions
    }

    /// Generate a replacement effect for this ability.
    ///
    /// Returns None if this ability doesn't create a replacement effect.
    /// Override for: EntersTapped, ShuffleIntoLibraryFromGraveyard, etc.
    fn generate_replacement_effect(
        &self,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Option<crate::replacement::ReplacementEffect> {
        None
    }

    /// Check if this ability is currently active.
    ///
    /// Most abilities are always active. Override for conditional abilities
    /// like Metalcraft or Devotion-based effects.
    fn is_active(&self, _game: &GameState, _source: ObjectId) -> bool {
        true
    }

    // ========================================================================
    // Query methods for specific ability checks
    // These allow checking ability properties without pattern matching.
    // ========================================================================

    /// Returns true if this is a keyword ability (Flying, Trample, etc.)
    fn is_keyword(&self) -> bool {
        false
    }

    /// Returns true if this ability grants evasion (Flying, Shadow, etc.)
    fn grants_evasion(&self) -> bool {
        false
    }

    /// Returns the threshold for "can't be blocked by creatures with power N or less".
    fn cant_be_blocked_by_power_or_less(&self) -> Option<i32> {
        None
    }

    /// Returns the threshold for "can't be blocked by creatures with power N or greater".
    fn cant_be_blocked_by_power_or_greater(&self) -> Option<i32> {
        None
    }

    /// Returns true if this ability prevents blocking (Unblockable, etc.)
    fn is_unblockable(&self) -> bool {
        false
    }

    /// Defender-specific attack legality hook for "can't attack unless ...".
    ///
    /// Return:
    /// - `Some(true)` if this ability allows attacking this defending player
    /// - `Some(false)` if this ability forbids attacking this defending player
    /// - `None` if this ability does not impose defender-specific attack legality
    fn can_attack_specific_defender(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
        _defending_player: PlayerId,
    ) -> Option<bool> {
        None
    }

    /// Attacking-group legality hook for "can't attack unless ... also attacks" style clauses.
    ///
    /// Return:
    /// - `Some(true)` if this source can attack with the provided attacker set
    /// - `Some(false)` if this source cannot attack with the provided attacker set
    /// - `None` if this ability does not depend on the full attacker set
    fn can_attack_with_attacking_group(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
        _attacking_creatures: &[ObjectId],
    ) -> Option<bool> {
        None
    }

    /// Attack-cost payability hook for "can't attack unless you pay/sacrifice/return ..." clauses.
    ///
    /// Return:
    /// - `Some(true)` if this source can currently pay this attack cost
    /// - `Some(false)` if this source cannot currently pay this attack cost
    /// - `None` if this ability does not impose an explicit attack cost
    fn can_pay_attack_cost(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Option<bool> {
        None
    }

    /// Generic mana contribution required to attack from this ability.
    ///
    /// Return `Some(n)` for explicit attack costs like "pay {1} for each ...";
    /// return `None` when this ability does not add an attacker-paid mana component.
    fn generic_attack_mana_cost_for_source(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Option<u32> {
        None
    }

    /// Pays any non-mana attack cost imposed by this ability for the source.
    ///
    /// This is called only during attack declaration after legality checks pass.
    /// Return:
    /// - `Some(Ok(()))` when this ability imposes and successfully pays a non-mana attack cost
    /// - `Some(Err(msg))` when paying that cost fails
    /// - `None` when this ability has no non-mana attack payment
    fn pay_non_mana_attack_cost(
        &self,
        _game: &mut GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Option<Result<(), String>> {
        None
    }

    /// Returns the generic mana tax per attacking creature required to attack this ability's
    /// controller directly.
    fn generic_attack_tax_per_attacker_against_you(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Option<u32> {
        None
    }

    /// Returns landwalk behavior for unblockable checks.
    fn landwalk_kind(&self) -> Option<crate::static_abilities::LandwalkKind> {
        None
    }

    /// Returns required card type for "can't be blocked as long as defending player controls ...".
    fn required_defending_player_card_type_for_unblockable(
        &self,
    ) -> Option<crate::types::CardType> {
        None
    }

    /// Returns required card-type conjunction for
    /// "can't be blocked as long as defending player controls ...".
    fn required_defending_player_card_types_for_unblockable(
        &self,
    ) -> Option<Vec<crate::types::CardType>> {
        None
    }

    /// Returns the maximum number of blockers this creature can be blocked by.
    fn maximum_blockers(&self) -> Option<usize> {
        None
    }

    /// Returns how many additional attackers this creature can block.
    ///
    /// Used for abilities like "This creature can block an additional creature each combat."
    fn additional_blockable_attackers(&self) -> Option<usize> {
        None
    }

    /// Returns the maximum number of creatures that can attack in a combat.
    fn max_creatures_can_attack_each_combat(&self) -> Option<usize> {
        None
    }

    /// Returns the maximum number of creatures that can block in a combat.
    fn max_creatures_can_block_each_combat(&self) -> Option<usize> {
        None
    }

    /// Returns true if this is a first/double strike ability.
    fn has_first_strike(&self) -> bool {
        false
    }

    /// Returns true if this is a double strike ability.
    fn has_double_strike(&self) -> bool {
        false
    }

    /// Returns true if this grants deathtouch.
    fn has_deathtouch(&self) -> bool {
        false
    }

    /// Returns true if this grants lifelink.
    fn has_lifelink(&self) -> bool {
        false
    }

    /// Returns true if this grants trample.
    fn has_trample(&self) -> bool {
        false
    }

    /// Returns true if this grants vigilance.
    fn has_vigilance(&self) -> bool {
        false
    }

    /// Returns true if this grants haste.
    fn has_haste(&self) -> bool {
        false
    }

    /// Returns true if this grants flash.
    fn has_flash(&self) -> bool {
        false
    }

    /// Returns true if this grants reach.
    fn has_reach(&self) -> bool {
        false
    }

    /// Returns true if this grants defender.
    fn has_defender(&self) -> bool {
        false
    }

    /// Returns info for "as this enters, choose a color" abilities.
    fn color_choice_as_enters(&self) -> Option<ChooseColorAsEntersSpec> {
        None
    }

    /// Returns info for "as this enters, choose a player" abilities.
    fn player_choice_as_enters(&self) -> Option<ChoosePlayerAsEntersSpec> {
        None
    }

    /// Returns info for "as this enters, choose a basic land type" abilities.
    fn basic_land_type_choice_as_enters(&self) -> Option<ChooseBasicLandTypeAsEntersSpec> {
        None
    }

    /// Returns info for "as this enters, choose a land type" abilities.
    fn land_type_choice_as_enters(&self) -> Option<ChooseLandTypeAsEntersSpec> {
        None
    }

    /// Returns info for "as this enters, choose a creature type" abilities.
    fn creature_type_choice_as_enters(&self) -> Option<ChooseCreatureTypeAsEntersSpec> {
        None
    }

    /// Returns info for "as this enters, choose <A> or <B>" abilities.
    fn named_option_choice_as_enters(&self) -> Option<ChooseNamedOptionAsEntersSpec> {
        None
    }

    /// Returns info for "you may have this enter as a copy ..." abilities.
    fn enter_as_copy_as_enters(&self) -> Option<&EnterAsCopyAsEntersSpec> {
        None
    }

    /// Returns an inline granted ability when this static ability wraps one.
    fn granted_inline_ability(&self) -> Option<&crate::ability::Ability> {
        None
    }

    /// Returns true if this grants indestructible.
    fn has_indestructible(&self) -> bool {
        false
    }

    /// Returns true if this grants hexproof.
    fn has_hexproof(&self) -> bool {
        false
    }

    /// Get hexproof-from filter if this is a hexproof-from ability.
    fn hexproof_from_filter(&self) -> Option<&crate::target::ObjectFilter> {
        None
    }

    /// Returns true if this grants shroud.
    fn has_shroud(&self) -> bool {
        false
    }

    /// Returns true if this is menace.
    fn has_menace(&self) -> bool {
        false
    }

    /// Returns true if this is flying.
    fn has_flying(&self) -> bool {
        false
    }

    /// Returns true if this grants protection from something.
    fn has_protection(&self) -> bool {
        false
    }

    /// Get protection details if this is a protection ability.
    fn protection_from(&self) -> Option<&crate::ability::ProtectionFrom> {
        None
    }

    /// Get ward cost if this is a ward ability.
    fn ward_cost(&self) -> Option<&crate::cost::TotalCost> {
        None
    }

    /// Get turn-face-up cost if this is a morph/megamorph ability.
    fn turn_face_up_cost(&self) -> Option<&crate::mana::ManaCost> {
        None
    }

    /// Returns true if this is a megamorph ability.
    fn is_megamorph(&self) -> bool {
        false
    }

    /// Returns true if this is an anthem effect.
    fn is_anthem(&self) -> bool {
        false
    }

    /// Returns true if this grants abilities to other permanents.
    fn grants_abilities(&self) -> bool {
        false
    }

    /// Returns true if this modifies casting costs.
    fn modifies_costs(&self) -> bool {
        false
    }

    /// Returns true if this ability lets its controller pay {B} with 2 life.
    fn black_mana_may_be_paid_with_life(&self) -> bool {
        false
    }

    /// Returns the minimum total mana value a spell must cost to cast.
    fn minimum_total_spell_mana(&self) -> Option<u32> {
        None
    }

    /// Returns true if this ability stops a player from paying life to cast spells
    /// or activate abilities.
    fn forbids_paying_life_for_cast_or_activate(&self) -> bool {
        false
    }

    /// Returns true if this ability stops a player from sacrificing nonland permanents
    /// to cast spells or activate abilities.
    fn forbids_sacrificing_nonland_for_cast_or_activate(&self) -> bool {
        false
    }

    /// Returns true if this is affinity for artifacts.
    fn has_affinity(&self) -> bool {
        false
    }

    /// Returns true if this is delve.
    fn has_delve(&self) -> bool {
        false
    }

    /// Returns true if this is convoke.
    fn has_convoke(&self) -> bool {
        false
    }

    /// Returns true if this is improvise.
    fn has_improvise(&self) -> bool {
        false
    }

    /// Get cost reduction details if this modifies the cost of *this spell* while casting.
    fn this_spell_cost_reduction(&self) -> Option<&ThisSpellCostReduction> {
        None
    }

    /// Get mana-symbol cost reduction details if this modifies this spell's cost while casting.
    fn this_spell_cost_reduction_mana_cost(&self) -> Option<&ThisSpellCostReductionManaCost> {
        None
    }

    /// Get cost reduction details if this is a cost reduction ability.
    fn cost_reduction(&self) -> Option<&CostReduction> {
        None
    }

    /// Get activated-ability cost reduction details.
    fn activated_ability_cost_reduction(&self) -> Option<&ActivatedAbilityCostReduction> {
        None
    }

    /// Get cost increase details if this is a cost increase ability.
    fn cost_increase(&self) -> Option<&CostIncrease> {
        None
    }

    /// Get cost reduction details if this reduces specific mana symbols (e.g., "{B} less").
    fn cost_reduction_mana_cost(&self) -> Option<&CostReductionManaCost> {
        None
    }

    /// Get cost increase details if this adds specific mana symbols (e.g., "{B} more").
    fn cost_increase_mana_cost(&self) -> Option<&CostIncreaseManaCost> {
        None
    }

    /// Get additional cost per target beyond the first, if any.
    fn cost_increase_per_additional_target(&self) -> Option<u32> {
        None
    }

    /// Returns true if this affects the untap step.
    fn affects_untap(&self) -> bool {
        false
    }

    /// Returns true if this causes entering tapped.
    fn enters_tapped(&self) -> bool {
        false
    }

    /// Returns true if this is changeling (all creature types).
    fn is_changeling(&self) -> bool {
        false
    }

    /// Returns true if this is Devoid.
    fn is_devoid(&self) -> bool {
        false
    }

    /// Returns true if this ability can't be countered.
    fn cant_be_countered(&self) -> bool {
        false
    }

    /// Get level abilities if this is a level-up ability.
    fn level_abilities(&self) -> Option<&[crate::ability::LevelAbility]> {
        None
    }

    /// Get equipment grant abilities if this is an equipment grant.
    fn equipment_grant(&self) -> Option<&[Box<dyn StaticAbilityKind>]> {
        None
    }

    /// Get equipment grant abilities as StaticAbility slice (for convenience).
    fn equipment_grant_abilities(&self) -> Option<&[StaticAbility]> {
        None
    }

    /// Get the grant specification if this ability grants something to cards.
    ///
    /// This is the unified way to check if a static ability grants abilities
    /// or alternative casting methods to cards in non-battlefield zones.
    fn grant_spec(&self) -> Option<crate::grant::GrantSpec> {
        None
    }

    /// Return a conditional spell-keyword descriptor, if this ability provides one.
    fn conditional_spell_keyword_spec(&self) -> Option<ConditionalSpellKeywordSpec> {
        None
    }

    /// Return a trigger-duplication descriptor, if this ability causes matching triggers
    /// to trigger additional times.
    fn trigger_duplication_spec(&self) -> Option<TriggerDuplicationSpec> {
        None
    }

    /// Return a trigger-suppression descriptor, if this ability prevents matching
    /// triggers from triggering.
    fn trigger_suppression_spec(&self) -> Option<TriggerSuppressionSpec> {
        None
    }

    /// Return a "Cast this spell only ..." restriction descriptor, if any.
    fn this_spell_cast_restriction_kind(&self) -> Option<ThisSpellCastRestrictionKind> {
        None
    }

    /// Return a pregame-action descriptor, if this ability creates one.
    fn pregame_action_kind(&self) -> Option<PregameActionKind> {
        None
    }
}

/// Spec for "as this enters, choose a color" abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChooseColorAsEntersSpec {
    pub excluded: Option<crate::color::Color>,
}

/// Spec for "as this enters, choose a player" abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChoosePlayerAsEntersSpec;

/// Spec for "as this enters, choose a basic land type" abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChooseBasicLandTypeAsEntersSpec;

/// Spec for "as this enters, choose a land type" abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChooseLandTypeAsEntersSpec;

/// Spec for "as this enters, choose a creature type" abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChooseCreatureTypeAsEntersSpec;

/// Spec for "as this enters, choose <A> or <B>" abilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooseNamedOptionAsEntersSpec {
    pub options: Vec<String>,
}

/// Spec for "you may have this enter as a copy ..." abilities.
#[derive(Debug, Clone, PartialEq)]
pub struct EnterAsCopyAsEntersSpec {
    pub filter: crate::target::ObjectFilter,
    pub may: bool,
    pub enters_tapped_if_chosen: bool,
    pub added_card_types: Vec<crate::types::CardType>,
    pub added_subtypes: Vec<crate::types::Subtype>,
}

/// Spec for static abilities that duplicate matching triggered abilities.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerDuplicationSpec {
    pub source_filter: Option<crate::target::ObjectFilter>,
    pub event_matcher: Option<crate::triggers::Trigger>,
    pub copies: usize,
}

/// Spec for static abilities that suppress matching triggered abilities.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerSuppressionSpec {
    pub source_filter: Option<crate::target::ObjectFilter>,
    pub event_matcher: Option<crate::triggers::Trigger>,
}

// Implement Clone for Box<dyn StaticAbilityKind>
impl Clone for Box<dyn StaticAbilityKind> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A wrapper around a boxed StaticAbilityKind trait object.
///
/// This provides a convenient way to work with static abilities as values
/// while maintaining the flexibility of trait objects.
#[derive(Debug, Clone)]
pub struct StaticAbility(pub Arc<dyn StaticAbilityKind>);

impl PartialEq for StaticAbility {
    fn eq(&self, other: &Self) -> bool {
        // Compare by ID and display (for abilities with parameters)
        self.0.id() == other.0.id() && self.0.display() == other.0.display()
    }
}

impl StaticAbility {
    /// Create a new StaticAbility from any StaticAbilityKind implementation.
    pub fn new<K: StaticAbilityKind + 'static>(kind: K) -> Self {
        StaticAbility(Arc::new(kind))
    }

    /// Get the ability's unique identifier.
    pub fn id(&self) -> StaticAbilityId {
        self.0.id()
    }

    pub fn color_choice_as_enters(&self) -> Option<ChooseColorAsEntersSpec> {
        self.0.color_choice_as_enters()
    }

    pub fn player_choice_as_enters(&self) -> Option<ChoosePlayerAsEntersSpec> {
        self.0.player_choice_as_enters()
    }

    pub fn basic_land_type_choice_as_enters(&self) -> Option<ChooseBasicLandTypeAsEntersSpec> {
        self.0.basic_land_type_choice_as_enters()
    }

    pub fn land_type_choice_as_enters(&self) -> Option<ChooseLandTypeAsEntersSpec> {
        self.0.land_type_choice_as_enters()
    }

    pub fn creature_type_choice_as_enters(&self) -> Option<ChooseCreatureTypeAsEntersSpec> {
        self.0.creature_type_choice_as_enters()
    }

    pub fn named_option_choice_as_enters(&self) -> Option<ChooseNamedOptionAsEntersSpec> {
        self.0.named_option_choice_as_enters()
    }

    pub fn enter_as_copy_as_enters(&self) -> Option<&EnterAsCopyAsEntersSpec> {
        self.0.enter_as_copy_as_enters()
    }

    pub fn granted_inline_ability(&self) -> Option<&crate::ability::Ability> {
        self.0.granted_inline_ability()
    }

    pub fn conditional_spell_keyword_spec(&self) -> Option<ConditionalSpellKeywordSpec> {
        self.0.conditional_spell_keyword_spec()
    }

    pub fn trigger_duplication_spec(&self) -> Option<TriggerDuplicationSpec> {
        self.0.trigger_duplication_spec()
    }

    pub fn trigger_suppression_spec(&self) -> Option<TriggerSuppressionSpec> {
        self.0.trigger_suppression_spec()
    }

    pub fn this_spell_cast_restriction_kind(&self) -> Option<ThisSpellCastRestrictionKind> {
        self.0.this_spell_cast_restriction_kind()
    }

    pub fn pregame_action_kind(&self) -> Option<PregameActionKind> {
        self.0.pregame_action_kind()
    }

    /// Get the display text for this ability.
    pub fn display(&self) -> String {
        self.0.display()
    }

    pub fn with_condition(&self, condition: crate::ConditionExpr) -> Option<Self> {
        self.0.with_static_condition(condition)
    }

    /// Generate continuous effects for this ability.
    pub fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        self.0.generate_effects(source, controller, game)
    }

    /// Apply game restrictions for this ability.
    pub fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        self.0.apply_restrictions(game, source, controller)
    }

    /// Check if this ability is currently active.
    pub fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        self.0.is_active(game, source)
    }

    /// Generate a replacement effect for this ability.
    pub fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<crate::replacement::ReplacementEffect> {
        self.0.generate_replacement_effect(source, controller)
    }

    // ========================================================================
    // Delegate query methods
    // ========================================================================

    pub fn is_keyword(&self) -> bool {
        self.0.is_keyword()
    }

    pub fn grants_evasion(&self) -> bool {
        self.0.grants_evasion()
    }

    pub fn blocked_by_power_or_less_threshold(&self) -> Option<i32> {
        self.0.cant_be_blocked_by_power_or_less()
    }

    pub fn blocked_by_power_or_greater_threshold(&self) -> Option<i32> {
        self.0.cant_be_blocked_by_power_or_greater()
    }

    pub fn is_unblockable(&self) -> bool {
        self.0.is_unblockable()
    }

    pub fn can_attack_specific_defender(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        defending_player: PlayerId,
    ) -> Option<bool> {
        self.0
            .can_attack_specific_defender(game, source, controller, defending_player)
    }

    pub fn can_attack_with_attacking_group(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        attacking_creatures: &[ObjectId],
    ) -> Option<bool> {
        self.0
            .can_attack_with_attacking_group(game, source, controller, attacking_creatures)
    }

    pub fn generic_attack_tax_per_attacker_against_you(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<u32> {
        self.0
            .generic_attack_tax_per_attacker_against_you(game, source, controller)
    }

    pub fn can_pay_attack_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<bool> {
        self.0.can_pay_attack_cost(game, source, controller)
    }

    pub fn generic_attack_mana_cost_for_source(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<u32> {
        self.0
            .generic_attack_mana_cost_for_source(game, source, controller)
    }

    pub fn pay_non_mana_attack_cost(
        &self,
        game: &mut GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<Result<(), String>> {
        self.0.pay_non_mana_attack_cost(game, source, controller)
    }

    pub fn landwalk_kind(&self) -> Option<crate::static_abilities::LandwalkKind> {
        self.0.landwalk_kind()
    }

    pub fn required_defending_player_card_type_for_unblockable(
        &self,
    ) -> Option<crate::types::CardType> {
        self.0.required_defending_player_card_type_for_unblockable()
    }

    pub fn required_defending_player_card_types_for_unblockable(
        &self,
    ) -> Option<Vec<crate::types::CardType>> {
        self.0
            .required_defending_player_card_types_for_unblockable()
    }

    pub fn maximum_blockers(&self) -> Option<usize> {
        self.0.maximum_blockers()
    }

    pub fn additional_blockable_attackers(&self) -> Option<usize> {
        self.0.additional_blockable_attackers()
    }

    pub fn max_creatures_can_attack_each_combat(&self) -> Option<usize> {
        self.0.max_creatures_can_attack_each_combat()
    }

    pub fn max_creatures_can_block_each_combat(&self) -> Option<usize> {
        self.0.max_creatures_can_block_each_combat()
    }

    pub fn has_first_strike(&self) -> bool {
        self.0.has_first_strike()
    }

    pub fn has_double_strike(&self) -> bool {
        self.0.has_double_strike()
    }

    pub fn has_deathtouch(&self) -> bool {
        self.0.has_deathtouch()
    }

    pub fn has_lifelink(&self) -> bool {
        self.0.has_lifelink()
    }

    pub fn has_trample(&self) -> bool {
        self.0.has_trample()
    }

    pub fn has_vigilance(&self) -> bool {
        self.0.has_vigilance()
    }

    pub fn has_haste(&self) -> bool {
        self.0.has_haste()
    }

    pub fn has_flash(&self) -> bool {
        self.0.has_flash()
    }

    pub fn has_reach(&self) -> bool {
        self.0.has_reach()
    }

    pub fn has_defender(&self) -> bool {
        self.0.has_defender()
    }

    pub fn has_indestructible(&self) -> bool {
        self.0.has_indestructible()
    }

    pub fn has_hexproof(&self) -> bool {
        self.0.has_hexproof()
    }

    pub fn hexproof_from_filter(&self) -> Option<&crate::target::ObjectFilter> {
        self.0.hexproof_from_filter()
    }

    pub fn has_shroud(&self) -> bool {
        self.0.has_shroud()
    }

    pub fn has_menace(&self) -> bool {
        self.0.has_menace()
    }

    pub fn has_flying(&self) -> bool {
        self.0.has_flying()
    }

    pub fn has_protection(&self) -> bool {
        self.0.has_protection()
    }

    pub fn protection_from(&self) -> Option<&crate::ability::ProtectionFrom> {
        self.0.protection_from()
    }

    pub fn ward_cost(&self) -> Option<&crate::cost::TotalCost> {
        self.0.ward_cost()
    }

    pub fn turn_face_up_cost(&self) -> Option<&crate::mana::ManaCost> {
        self.0.turn_face_up_cost()
    }

    pub fn is_megamorph(&self) -> bool {
        self.0.is_megamorph()
    }

    pub fn cost_reduction(&self) -> Option<&CostReduction> {
        self.0.cost_reduction()
    }

    pub fn activated_ability_cost_reduction(&self) -> Option<&ActivatedAbilityCostReduction> {
        self.0.activated_ability_cost_reduction()
    }

    pub fn this_spell_cost_reduction(&self) -> Option<&ThisSpellCostReduction> {
        self.0.this_spell_cost_reduction()
    }

    pub fn this_spell_cost_reduction_mana_cost(&self) -> Option<&ThisSpellCostReductionManaCost> {
        self.0.this_spell_cost_reduction_mana_cost()
    }

    pub fn cost_increase(&self) -> Option<&CostIncrease> {
        self.0.cost_increase()
    }

    pub fn cost_reduction_mana_cost(&self) -> Option<&CostReductionManaCost> {
        self.0.cost_reduction_mana_cost()
    }

    pub fn cost_increase_mana_cost(&self) -> Option<&CostIncreaseManaCost> {
        self.0.cost_increase_mana_cost()
    }

    pub fn cost_increase_per_additional_target(&self) -> Option<u32> {
        self.0.cost_increase_per_additional_target()
    }

    pub fn is_anthem(&self) -> bool {
        self.0.is_anthem()
    }

    pub fn grants_abilities(&self) -> bool {
        self.0.grants_abilities()
    }

    pub fn modifies_costs(&self) -> bool {
        self.0.modifies_costs()
    }

    pub fn black_mana_may_be_paid_with_life(&self) -> bool {
        self.0.black_mana_may_be_paid_with_life()
    }

    pub fn minimum_total_spell_mana(&self) -> Option<u32> {
        self.0.minimum_total_spell_mana()
    }

    pub fn forbids_paying_life_for_cast_or_activate(&self) -> bool {
        self.0.forbids_paying_life_for_cast_or_activate()
    }

    pub fn forbids_sacrificing_nonland_for_cast_or_activate(&self) -> bool {
        self.0.forbids_sacrificing_nonland_for_cast_or_activate()
    }

    pub fn has_affinity(&self) -> bool {
        self.0.has_affinity()
    }

    pub fn has_delve(&self) -> bool {
        self.0.has_delve()
    }

    pub fn has_convoke(&self) -> bool {
        self.0.has_convoke()
    }

    pub fn has_improvise(&self) -> bool {
        self.0.has_improvise()
    }

    pub fn affects_untap(&self) -> bool {
        self.0.affects_untap()
    }

    pub fn enters_tapped(&self) -> bool {
        self.0.enters_tapped()
    }

    pub fn is_changeling(&self) -> bool {
        self.0.is_changeling()
    }

    pub fn is_devoid(&self) -> bool {
        self.0.is_devoid()
    }

    pub fn cant_be_countered(&self) -> bool {
        self.0.cant_be_countered()
    }

    pub fn level_abilities(&self) -> Option<&[crate::ability::LevelAbility]> {
        self.0.level_abilities()
    }

    pub fn equipment_grant_abilities(&self) -> Option<&[StaticAbility]> {
        self.0.equipment_grant_abilities()
    }

    /// Get the grant specification if this ability grants something to cards.
    pub fn grant_spec(&self) -> Option<crate::grant::GrantSpec> {
        self.0.grant_spec()
    }

    // ========================================================================
    // Convenience constructors for common abilities
    // ========================================================================

    pub fn flying() -> Self {
        Self::new(Flying)
    }

    pub fn first_strike() -> Self {
        Self::new(FirstStrike)
    }

    pub fn double_strike() -> Self {
        Self::new(DoubleStrike)
    }

    pub fn deathtouch() -> Self {
        Self::new(Deathtouch)
    }

    pub fn defender() -> Self {
        Self::new(Defender)
    }

    pub fn flash() -> Self {
        Self::new(Flash)
    }

    pub fn haste() -> Self {
        Self::new(Haste)
    }

    pub fn hexproof() -> Self {
        Self::new(Hexproof)
    }

    pub fn indestructible() -> Self {
        Self::new(Indestructible)
    }

    pub fn lifelink() -> Self {
        Self::new(Lifelink)
    }

    pub fn menace() -> Self {
        Self::new(Menace)
    }

    pub fn reach() -> Self {
        Self::new(Reach)
    }

    pub fn shroud() -> Self {
        Self::new(Shroud)
    }

    pub fn trample() -> Self {
        Self::new(Trample)
    }

    pub fn vigilance() -> Self {
        Self::new(Vigilance)
    }

    pub fn fear() -> Self {
        Self::new(Fear)
    }

    pub fn skulk() -> Self {
        Self::new(Skulk)
    }

    pub fn intimidate() -> Self {
        Self::new(Intimidate)
    }

    pub fn shadow() -> Self {
        Self::new(Shadow)
    }

    pub fn horsemanship() -> Self {
        Self::new(Horsemanship)
    }

    pub fn flanking() -> Self {
        Self::new(Flanking)
    }

    pub fn umbra_armor() -> Self {
        Self::new(UmbraArmor)
    }

    pub fn phasing() -> Self {
        Self::new(Phasing)
    }

    pub fn wither() -> Self {
        Self::new(Wither)
    }

    pub fn infect() -> Self {
        Self::new(Infect)
    }

    pub fn changeling() -> Self {
        Self::new(Changeling)
    }

    pub fn partner() -> Self {
        Self::new(Partner)
    }

    pub fn doctors_companion() -> Self {
        Self::new(DoctorsCompanion)
    }

    pub fn assist() -> Self {
        Self::new(Assist)
    }

    pub fn split_second() -> Self {
        Self::new(SplitSecond)
    }

    pub fn rebound() -> Self {
        Self::new(Rebound)
    }

    pub fn cascade() -> Self {
        Self::new(Cascade)
    }

    pub fn unleash() -> Self {
        Self::new(Unleash)
    }

    pub fn protection(from: crate::ability::ProtectionFrom) -> Self {
        Self::new(Protection::new(from))
    }

    pub fn ward(cost: crate::cost::TotalCost) -> Self {
        Self::new(Ward::new(cost))
    }

    pub fn hexproof_from(filter: crate::target::ObjectFilter) -> Self {
        Self::new(HexproofFrom::new(filter))
    }

    pub fn unblockable() -> Self {
        Self::new(Unblockable)
    }

    pub fn cant_attack() -> Self {
        Self::new(CantAttack)
    }

    pub fn cant_attack_unless_controller_cast_creature_spell_this_turn() -> Self {
        Self::new(CantAttackUnlessControllerCastCreatureSpellThisTurn)
    }

    pub fn cant_attack_unless_controller_cast_noncreature_spell_this_turn() -> Self {
        Self::new(CantAttackUnlessControllerCastNonCreatureSpellThisTurn)
    }

    pub fn cant_block() -> Self {
        Self::new(CantBlock)
    }

    pub fn must_attack() -> Self {
        Self::new(MustAttack)
    }

    pub fn cant_attack_unless_defending_player_controls_land_subtype(
        subtype: crate::types::Subtype,
    ) -> Self {
        Self::cant_attack_unless_condition(
            CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                DefendingPlayerAttackCondition::Controls(
                    crate::filter::ObjectFilter::default()
                        .with_type(crate::types::CardType::Land)
                        .with_subtype(subtype),
                ),
            ),
            "",
        )
    }

    pub fn cant_attack_unless_condition(
        condition: CantAttackUnlessConditionSpec,
        display: impl Into<String>,
    ) -> Self {
        Self::new(CantAttackUnlessCondition::new(condition, display))
    }

    pub fn cant_attack_you_unless_controller_pays_per_attacker(amount: u32) -> Self {
        Self::new(CantAttackYouUnlessControllerPaysPerAttacker::new(amount))
    }

    pub fn cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control()
    -> Self {
        Self::new(CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl)
    }

    pub fn must_block() -> Self {
        Self::new(MustBlock)
    }

    pub fn flying_restriction() -> Self {
        Self::new(FlyingRestriction)
    }

    pub fn flying_only_restriction() -> Self {
        Self::new(FlyingOnlyRestriction)
    }

    pub fn can_block_flying() -> Self {
        Self::new(CanBlockFlying)
    }

    pub fn can_block_only_flying() -> Self {
        Self::new(CanBlockOnlyFlying)
    }

    pub fn can_block_additional_creature_each_combat(additional: usize) -> Self {
        Self::new(CanBlockAdditionalCreatureEachCombat::new(additional))
    }

    pub fn max_attackers_each_combat(maximum: usize) -> Self {
        Self::new(MaxCreaturesCanAttackEachCombat::new(maximum))
    }

    pub fn max_blockers_each_combat(maximum: usize) -> Self {
        Self::new(MaxCreaturesCanBlockEachCombat::new(maximum))
    }

    pub fn landwalk(land_subtype: crate::types::Subtype) -> Self {
        Self::new(Landwalk::new(LandwalkKind::Subtype {
            subtype: land_subtype,
            snow: false,
        }))
    }

    pub fn snow_landwalk(land_subtype: crate::types::Subtype) -> Self {
        Self::new(Landwalk::new(LandwalkKind::Subtype {
            subtype: land_subtype,
            snow: true,
        }))
    }

    pub fn any_landwalk() -> Self {
        Self::new(Landwalk::new(LandwalkKind::AnyLand))
    }

    pub fn nonbasic_landwalk() -> Self {
        Self::new(Landwalk::new(LandwalkKind::NonbasicLand))
    }

    pub fn artifact_landwalk() -> Self {
        Self::new(Landwalk::new(LandwalkKind::ArtifactLand))
    }

    pub fn attached_chosen_landwalk_grant(display: String, snow: bool) -> Self {
        Self::new(AttachedChosenLandwalkGrant::new(display, snow))
    }

    pub fn cant_be_blocked_as_long_as_defending_player_controls_card_type(
        card_type: crate::types::CardType,
    ) -> Self {
        Self::new(CantBeBlockedAsLongAsDefendingPlayerControlsCardType::new(
            card_type,
        ))
    }

    pub fn cant_be_blocked_as_long_as_defending_player_controls_card_types(
        card_types: Vec<crate::types::CardType>,
    ) -> Self {
        Self::new(CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes::new(
            card_types,
        ))
    }

    pub fn bloodthirst(amount: u32) -> Self {
        Self::new(Bloodthirst::new(amount))
    }

    pub fn morph(cost: crate::mana::ManaCost) -> Self {
        Self::new(Morph::new(cost))
    }

    pub fn megamorph(cost: crate::mana::ManaCost) -> Self {
        Self::new(Megamorph::new(cost))
    }

    pub fn cant_be_blocked_by_power_or_less(threshold: i32) -> Self {
        Self::new(CantBeBlockedByPowerOrLess::new(threshold))
    }

    pub fn cant_be_blocked_by_power_or_greater(threshold: i32) -> Self {
        Self::new(CantBeBlockedByPowerOrGreater::new(threshold))
    }

    pub fn cant_be_blocked_by_lower_power_than_source() -> Self {
        Self::new(CantBeBlockedByLowerPowerThanSource)
    }

    pub fn cant_be_blocked_by_more_than(max_blockers: usize) -> Self {
        Self::new(CantBeBlockedByMoreThan::new(max_blockers))
    }

    pub fn can_attack_as_though_no_defender() -> Self {
        Self::new(CanAttackAsThoughNoDefender)
    }

    pub fn doesnt_untap() -> Self {
        Self::new(DoesntUntap)
    }

    pub fn boast_twice_each_turn() -> Self {
        Self::new(BoastTwiceEachTurn)
    }

    pub fn may_choose_not_to_untap_during_untap_step(subject: impl Into<String>) -> Self {
        Self::new(MayChooseNotToUntapDuringUntapStep::new(subject))
    }

    pub fn enters_tapped_ability() -> Self {
        Self::new(EntersTapped)
    }

    pub fn enters_tapped_unless_control_two_or_more_other_lands() -> Self {
        Self::new(EntersTappedUnlessControlTwoOrMoreOtherLands)
    }

    pub fn enters_tapped_unless_control_two_or_fewer_other_lands() -> Self {
        Self::new(EntersTappedUnlessControlTwoOrFewerOtherLands)
    }

    pub fn enters_tapped_unless_control_two_or_more_basic_lands() -> Self {
        Self::new(EntersTappedUnlessControlTwoOrMoreBasicLands)
    }

    pub fn enters_tapped_unless_a_player_has_13_or_less_life() -> Self {
        Self::new(EntersTappedUnlessAPlayerHas13OrLessLife)
    }

    pub fn enters_tapped_unless_two_or_more_opponents() -> Self {
        Self::new(EntersTappedUnlessTwoOrMoreOpponents)
    }

    pub fn enters_tapped_unless_condition(
        condition: crate::effect::Condition,
        display: String,
    ) -> Self {
        Self::new(
            crate::static_abilities::misc::EntersTappedUnlessCondition::new(condition, display),
        )
    }

    pub fn enters_with_counters(counter_type: crate::object::CounterType, count: u32) -> Self {
        Self::new(EntersWithCounters::new(
            counter_type,
            crate::effect::Value::Fixed(count as i32),
        ))
    }

    pub fn enters_with_counters_value(
        counter_type: crate::object::CounterType,
        count: crate::effect::Value,
    ) -> Self {
        Self::new(EntersWithCounters::new(counter_type, count))
    }

    pub fn enters_with_counters_if_condition(
        counter_type: crate::object::CounterType,
        count: crate::effect::Value,
        condition: crate::effect::Condition,
        condition_display: String,
    ) -> Self {
        Self::new(EntersWithCountersIfCondition::new(
            counter_type,
            count,
            condition,
            condition_display,
        ))
    }

    pub fn permanents_enter_tapped() -> Self {
        Self::new(AllPermanentsEnterTapped)
    }

    pub fn enters_tapped_for_filter(filter: crate::target::ObjectFilter) -> Self {
        Self::new(EnterTappedForFilter::new(filter))
    }

    pub fn enters_untapped_for_filter(filter: crate::target::ObjectFilter) -> Self {
        Self::new(EnterUntappedForFilter::new(filter))
    }

    pub fn enters_with_counters_for_filter(
        filter: crate::target::ObjectFilter,
        counter_type: crate::object::CounterType,
        count: u32,
    ) -> Self {
        Self::new(EnterWithCountersForFilter::new(
            filter,
            counter_type,
            crate::effect::Value::Fixed(count as i32),
        ))
    }

    pub fn enters_with_counters_value_for_filter(
        filter: crate::target::ObjectFilter,
        counter_type: crate::object::CounterType,
        count: crate::effect::Value,
    ) -> Self {
        Self::new(EnterWithCountersForFilter::new(filter, counter_type, count))
    }

    pub fn enters_with_counters_and_subtypes_for_filter(
        filter: crate::target::ObjectFilter,
        counter_type: crate::object::CounterType,
        count: crate::effect::Value,
        added_subtypes: Vec<crate::types::Subtype>,
    ) -> Self {
        Self::new(
            EnterWithCountersForFilter::new(filter, counter_type, count)
                .with_added_subtypes(added_subtypes),
        )
    }

    pub fn anthem(filter: crate::target::ObjectFilter, power: i32, toughness: i32) -> Self {
        Self::new(Anthem::new(filter, power, toughness))
    }

    pub fn grant_ability(filter: crate::target::ObjectFilter, ability: StaticAbility) -> Self {
        Self::new(GrantAbility::new(filter, ability))
    }

    pub fn soulbond_shared_power_toughness(power: i32, toughness: i32) -> Self {
        Self::new(SoulbondSharedBonus::power_toughness(power, toughness))
    }

    pub fn soulbond_shared_ability(ability: StaticAbility) -> Self {
        Self::new(SoulbondSharedBonus::ability(ability))
    }

    pub fn soulbond_shared_object_ability(ability: crate::ability::Ability) -> Self {
        Self::new(SoulbondSharedBonus::object_ability(ability))
    }

    pub fn remove_ability(filter: crate::target::ObjectFilter, ability: StaticAbility) -> Self {
        Self::new(RemoveAbilityForFilter::new(filter, ability))
    }

    pub fn remove_all_abilities(filter: crate::target::ObjectFilter) -> Self {
        Self::new(RemoveAllAbilitiesForFilter::new(filter))
    }

    pub fn remove_all_abilities_except_mana(filter: crate::target::ObjectFilter) -> Self {
        Self::new(RemoveAllAbilitiesExceptManaForFilter::new(filter))
    }

    pub fn set_base_power_toughness(
        filter: crate::target::ObjectFilter,
        power: i32,
        toughness: i32,
    ) -> Self {
        Self::new(SetBasePowerToughnessForFilter::new(
            filter, power, toughness,
        ))
    }

    pub fn set_colors(filter: crate::target::ObjectFilter, colors: crate::color::ColorSet) -> Self {
        Self::new(SetColorsForFilter::new(filter, colors))
    }

    pub fn set_name(filter: crate::target::ObjectFilter, name: impl Into<String>) -> Self {
        Self::new(SetNameForFilter::new(filter, name.into()))
    }

    pub fn add_colors(filter: crate::target::ObjectFilter, colors: crate::color::ColorSet) -> Self {
        Self::new(AddColorsForFilter::new(filter, colors))
    }

    pub fn add_card_types(
        filter: crate::target::ObjectFilter,
        card_types: Vec<crate::types::CardType>,
    ) -> Self {
        Self::new(AddCardTypesForFilter::new(filter, card_types))
    }

    pub fn remove_card_types(
        filter: crate::target::ObjectFilter,
        card_types: Vec<crate::types::CardType>,
    ) -> Self {
        Self::new(RemoveCardTypesForFilter::new(filter, card_types))
    }

    pub fn set_card_types(
        filter: crate::target::ObjectFilter,
        card_types: Vec<crate::types::CardType>,
    ) -> Self {
        Self::new(SetCardTypesForFilter::new(filter, card_types))
    }

    pub fn add_subtypes(
        filter: crate::target::ObjectFilter,
        subtypes: Vec<crate::types::Subtype>,
    ) -> Self {
        Self::new(AddSubtypesForFilter::new(filter, subtypes))
    }

    pub fn set_creature_subtypes(
        filter: crate::target::ObjectFilter,
        subtypes: Vec<crate::types::Subtype>,
    ) -> Self {
        Self::new(SetCreatureSubtypesForFilter::new(filter, subtypes))
    }

    pub fn make_colorless(filter: crate::target::ObjectFilter) -> Self {
        Self::new(MakeColorlessForFilter::new(filter))
    }

    pub fn add_supertypes(
        filter: crate::target::ObjectFilter,
        supertypes: Vec<crate::types::Supertype>,
    ) -> Self {
        Self::new(AddSupertypesForFilter::new(filter, supertypes))
    }

    pub fn remove_supertypes(
        filter: crate::target::ObjectFilter,
        supertypes: Vec<crate::types::Supertype>,
    ) -> Self {
        Self::new(RemoveSupertypesForFilter::new(filter, supertypes))
    }

    pub fn equipment_grant(abilities: Vec<StaticAbility>) -> Self {
        Self::new(EquipmentGrant::new(abilities))
    }

    pub fn copy_activated_abilities(ability: CopyActivatedAbilities) -> Self {
        Self::new(ability)
    }

    pub fn attached_ability_grant(ability: crate::ability::Ability, display: String) -> Self {
        Self::new(AttachedAbilityGrant::new(ability, display))
    }

    pub fn control_attached_permanent(display: String) -> Self {
        Self::new(ControlAttachedPermanent::new(display))
    }

    pub fn grant_object_ability_for_filter(
        filter: crate::target::ObjectFilter,
        ability: crate::ability::Ability,
        display: String,
    ) -> Self {
        Self::new(GrantObjectAbilityForFilter::new(filter, ability, display))
    }

    pub fn mana_spend_permission(
        permission: crate::effect::ManaSpendPermission,
        display: impl Into<String>,
    ) -> Self {
        Self::new(ManaSpendPermissionAbility::new(permission, display.into()))
    }

    pub fn spend_mana_as_any_color_players() -> Self {
        Self::mana_spend_permission(
            crate::effect::ManaSpendPermission::any_color(crate::target::PlayerFilter::Any),
            "Players may spend mana as though it were mana of any color",
        )
    }

    pub fn spend_mana_as_any_color_activation_costs() -> Self {
        Self::mana_spend_permission(
            crate::effect::ManaSpendPermission::any_color_for_activation(
                crate::target::PlayerFilter::You,
                crate::target::ObjectFilter::source(),
            ),
            "You may spend mana as though it were mana of any color to pay activation costs of this",
        )
    }

    pub fn krrik_black_mana_may_be_paid_with_life() -> Self {
        Self::new(BlackManaMayBePaidWithLife)
    }

    pub fn minimum_spell_total_mana(amount: u32) -> Self {
        Self::new(MinimumSpellTotalMana::new(amount))
    }

    pub fn cant_pay_life_or_sacrifice_nonland_for_cast_or_activate() -> Self {
        Self::new(CantPayLifeOrSacrificeNonlandForCastOrActivate)
    }

    pub fn with_level_abilities(levels: Vec<crate::ability::LevelAbility>) -> Self {
        Self::new(LevelAbilities::new(levels))
    }

    pub fn may_assign_damage_as_unblocked() -> Self {
        Self::new(MayAssignDamageAsUnblocked)
    }

    pub fn creatures_assign_combat_damage_using_toughness() -> Self {
        Self::new(CreaturesAssignCombatDamageUsingToughness)
    }

    pub fn creatures_you_control_assign_combat_damage_using_toughness() -> Self {
        Self::new(CreaturesYouControlAssignCombatDamageUsingToughness)
    }

    pub fn prevent_all_damage_dealt_to_creatures() -> Self {
        Self::new(PreventAllDamageDealtToCreatures)
    }

    pub fn prevent_all_combat_damage_to_self() -> Self {
        Self::new(PreventAllCombatDamageToSelf)
    }

    pub fn prevent_all_damage_to_self_by_creatures() -> Self {
        Self::new(PreventAllDamageToSelfByCreatures)
    }

    pub fn prevent_damage_to_self_remove_counter(
        counter_type: crate::object::CounterType,
        amount: u32,
    ) -> Self {
        Self::new(PreventDamageToSelfRemoveCounter::new(counter_type, amount))
    }

    pub fn prevent_damage_to_self_put_counters_instead(
        counter_type: crate::object::CounterType,
        display: impl Into<String>,
    ) -> Self {
        Self::new(PreventDamageToSelfPutCountersInstead::new(
            counter_type,
            display,
        ))
    }

    pub fn prevent_constrained_damage_to_self_put_counters_instead(
        counter_type: crate::object::CounterType,
        display: impl Into<String>,
        source_filter: Option<crate::target::ObjectFilter>,
        combat_only: Option<bool>,
    ) -> Self {
        Self::new(PreventConstrainedDamageToSelfPutCountersInstead::new(
            counter_type,
            display,
            source_filter,
            combat_only,
        ))
    }

    pub fn prevent_damage_to_other_creature_you_control_put_counters_instead(
        counter_type: crate::object::CounterType,
        display: impl Into<String>,
    ) -> Self {
        Self::new(
            PreventDamageToOtherCreatureYouControlPutCountersInstead::new(counter_type, display),
        )
    }

    pub fn shuffle_into_library_from_graveyard() -> Self {
        Self::new(ShuffleIntoLibraryFromGraveyard)
    }

    pub fn affinity_for_artifacts() -> Self {
        Self::new(AffinityForArtifacts)
    }

    pub fn cost_increase_per_target_beyond_first(amount: u32) -> Self {
        Self::new(CostIncreasePerAdditionalTarget::new(amount))
    }

    pub fn reduce_activated_ability_costs(
        filter: crate::target::ObjectFilter,
        reduction: u32,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        let mut ability = ActivatedAbilityCostReduction::new(filter, reduction);
        if let Some(minimum) = minimum_total_mana {
            ability = ability.with_minimum_total_mana(minimum);
        }
        Self::new(ability)
    }

    pub fn reduce_activated_ability_costs_if_targets(
        filter: crate::target::ObjectFilter,
        reduction: u32,
        condition: crate::static_abilities::cost_modifiers::ActivatedAbilityCostCondition,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        let mut ability =
            ActivatedAbilityCostReduction::new(filter, reduction).with_condition(condition);
        if let Some(minimum) = minimum_total_mana {
            ability = ability.with_minimum_total_mana(minimum);
        }
        Self::new(ability)
    }

    pub fn reduce_activated_ability_costs_for_each(
        filter: crate::target::ObjectFilter,
        reduction: u32,
        per_matching_objects: crate::target::ObjectFilter,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        let mut ability = ActivatedAbilityCostReduction::new(filter, reduction)
            .with_per_matching_objects(per_matching_objects);
        if let Some(minimum) = minimum_total_mana {
            ability = ability.with_minimum_total_mana(minimum);
        }
        Self::new(ability)
    }

    pub fn delve() -> Self {
        Self::new(Delve)
    }

    pub fn convoke() -> Self {
        Self::new(Convoke)
    }

    pub fn improvise() -> Self {
        Self::new(Improvise)
    }

    pub fn blood_moon() -> Self {
        Self::new(BloodMoon)
    }

    pub fn no_maximum_hand_size() -> Self {
        Self::new(NoMaximumHandSize)
    }

    pub fn reduce_maximum_hand_size(player: crate::target::PlayerFilter, amount: u32) -> Self {
        Self::new(ReduceMaximumHandSize::new(player, amount))
    }

    pub fn max_hand_size_seven_minus_your_graveyard_card_types(
        player: crate::target::PlayerFilter,
        minimum_types: u32,
    ) -> Self {
        Self::new(MaximumHandSizeSevenMinusYourGraveyardCardTypes::new(
            player,
            minimum_types,
        ))
    }

    pub fn conditional_spell_keyword(spec: ConditionalSpellKeywordSpec) -> Self {
        Self::new(ConditionalSpellKeyword::new(spec))
    }

    pub fn this_spell_cast_restriction(
        kind: ThisSpellCastRestrictionKind,
        display: impl Into<String>,
    ) -> Self {
        Self::new(ThisSpellCastRestriction::new(kind, display))
    }

    pub fn damage_not_removed_during_cleanup() -> Self {
        Self::new(DamageNotRemovedDuringCleanup)
    }

    pub fn choose_color_as_enters(excluded: Option<crate::color::Color>, display: String) -> Self {
        Self::new(ChooseColorAsEnters::new(excluded, display))
    }

    pub fn choose_player_as_enters(display: String) -> Self {
        Self::new(ChoosePlayerAsEnters::new(display))
    }

    pub fn choose_basic_land_type_as_enters(display: String) -> Self {
        Self::new(ChooseBasicLandTypeAsEnters::new(display))
    }

    pub fn choose_land_type_as_enters(display: String) -> Self {
        Self::new(ChooseLandTypeAsEnters::new(display))
    }

    pub fn choose_creature_type_as_enters(display: String) -> Self {
        Self::new(ChooseCreatureTypeAsEnters::new(display))
    }

    pub fn choose_named_option_as_enters(options: Vec<String>, display: String) -> Self {
        Self::new(ChooseNamedOptionAsEnters::new(options, display))
    }

    pub fn with_enter_as_copy_as_enters(spec: EnterAsCopyAsEntersSpec, display: String) -> Self {
        Self::new(EnterAsCopyAsEnters::new(spec, display))
    }

    pub fn enchanted_land_is_chosen_type(display: String) -> Self {
        Self::new(EnchantedLandIsChosenType::new(display))
    }

    pub fn add_chosen_creature_type(filter: crate::target::ObjectFilter, display: String) -> Self {
        Self::new(AddChosenCreatureTypeForFilter::new(filter, display))
    }

    pub fn set_chosen_color(filter: crate::target::ObjectFilter, display: String) -> Self {
        Self::new(SetChosenColorForFilter::new(filter, display))
    }

    pub fn redirect_damage_from_you_and_other_permanents_to_source() -> Self {
        Self::new(RedirectDamageToSource::new(
            crate::target::PlayerFilter::You,
            crate::target::ObjectFilter::permanent()
                .you_control()
                .other(),
            "All damage that would be dealt to you and other permanents you control is dealt to this creature instead.".to_string(),
        ))
    }

    pub fn players_cant_cycle() -> Self {
        Self::new(PlayersCantCycle)
    }

    pub fn players_skip_upkeep() -> Self {
        Self::new(PlayersSkipUpkeep)
    }

    pub fn starting_life_bonus(amount: i32) -> Self {
        Self::new(StartingLifeBonus::new(amount))
    }

    pub fn buyback_cost_reduction(amount: u32) -> Self {
        Self::new(BuybackCostReduction::new(amount))
    }

    pub fn legend_rule_doesnt_apply() -> Self {
        Self::new(LegendRuleDoesntApply)
    }

    pub fn additional_land_plays(count: u32) -> Self {
        let display = match count {
            1 => "You may play an additional land on each of your turns.".to_string(),
            2 => "You may play two additional lands on each of your turns.".to_string(),
            _ => format!("You may play {count} additional lands on each of your turns."),
        };
        Self::restriction(
            crate::effect::Restriction::additional_land_plays(
                crate::target::PlayerFilter::You,
                count,
            ),
            display,
        )
    }

    pub fn creatures_entering_dont_cause_abilities_to_trigger() -> Self {
        Self::new(CreaturesEnteringDontCauseAbilitiesToTrigger)
    }

    pub fn suppress_matching_triggered_abilities(
        source_filter: Option<crate::target::ObjectFilter>,
        event_matcher: Option<crate::triggers::Trigger>,
        display: String,
    ) -> Self {
        Self::new(SuppressMatchingTriggeredAbilities::new(
            source_filter,
            event_matcher,
            display,
        ))
    }

    pub fn other_chosen_type_creature_triggered_abilities_trigger_additional_time(
        display: String,
    ) -> Self {
        Self::duplicate_matching_triggered_abilities(
            Some(
                crate::target::ObjectFilter::creature()
                    .you_control()
                    .other()
                    .of_chosen_creature_type(),
            ),
            None,
            1,
            display,
        )
    }

    pub fn double_damage_from_sources_you_control_of_chosen_type(display: String) -> Self {
        Self::new(DoubleDamageFromSourcesYouControlOfChosenType::new(display))
    }

    pub fn library_of_leng_discard_replacement() -> Self {
        Self::new(LibraryOfLengDiscardReplacement)
    }

    pub fn duplicate_matching_triggered_abilities(
        source_filter: Option<crate::target::ObjectFilter>,
        event_matcher: Option<crate::triggers::Trigger>,
        copies: usize,
        display: String,
    ) -> Self {
        Self::new(DuplicateMatchingTriggeredAbilities::new(
            source_filter,
            event_matcher,
            copies,
            display,
        ))
    }

    pub fn draw_replacement_exile_top_face_down() -> Self {
        Self::new(DrawReplacementExileTopFaceDown)
    }

    pub fn exile_to_countered_exile_instead_of_graveyard(
        player: crate::target::PlayerFilter,
        counter_type: crate::object::CounterType,
    ) -> Self {
        Self::new(ExileToCounteredExileInsteadOfGraveyard::new(
            player,
            counter_type,
        ))
    }

    pub fn players_cant_gain_life() -> Self {
        Self::new(PlayersCantGainLife)
    }

    pub fn players_cant_search() -> Self {
        Self::new(PlayersCantSearch)
    }

    pub fn damage_cant_be_prevented() -> Self {
        Self::new(DamageCantBePrevented)
    }

    pub fn you_cant_lose_game() -> Self {
        Self::new(YouCantLoseGame)
    }

    pub fn opponents_cant_win_game() -> Self {
        Self::new(OpponentsCantWinGame)
    }

    pub fn your_life_total_cant_change() -> Self {
        Self::new(YourLifeTotalCantChange)
    }

    pub fn opponents_cant_cast_spells() -> Self {
        Self::new(OpponentsCantCastSpells)
    }

    pub fn opponents_cant_draw_extra_cards() -> Self {
        Self::new(OpponentsCantDrawExtraCards)
    }

    pub fn cant_have_counters_placed() -> Self {
        Self::new(CantHaveCountersPlaced)
    }

    pub fn permanents_you_control_cant_be_sacrificed() -> Self {
        Self::new(PermanentsCantBeSacrificed)
    }

    pub fn restriction(restriction: crate::effect::Restriction, display: String) -> Self {
        Self::new(RuleRestriction::new(restriction, display))
    }

    pub fn can_be_commander() -> Self {
        Self::new(CanBeCommander)
    }

    pub fn uncounterable() -> Self {
        Self::new(CantBeCountered)
    }

    pub fn characteristic_defining_pt(
        power: crate::effect::Value,
        toughness: crate::effect::Value,
    ) -> Self {
        Self::new(CharacteristicDefiningPT::new(power, toughness))
    }

    /// Create a discard-or-redirect ETB replacement ability.
    ///
    /// Used by Mox Diamond: "If Mox Diamond would enter the battlefield, you may discard
    /// a land card instead. If you do, put Mox Diamond onto the battlefield. If you don't,
    /// put it into its owner's graveyard."
    pub fn discard_or_redirect_replacement(
        filter: crate::target::ObjectFilter,
        redirect_zone: crate::zone::Zone,
    ) -> Self {
        Self::new(DiscardOrRedirectReplacement::new(filter, redirect_zone))
    }

    /// Create a pay-life-or-enter-tapped ETB replacement ability.
    ///
    /// Used by shock lands (Godless Shrine, etc.): "As ~ enters the battlefield,
    /// you may pay 2 life. If you don't, it enters the battlefield tapped."
    pub fn pay_life_or_enter_tapped(life_cost: u32) -> Self {
        Self::new(PayLifeOrEnterTappedReplacement::new(life_cost))
    }

    pub fn keyword_fallback_text(text: impl Into<String>) -> Self {
        Self::new(KeywordFallbackText::new(text))
    }

    pub fn keyword_text(text: impl Into<String>) -> Self {
        Self::new(KeywordText::new(text))
    }

    pub fn rule_fallback_text(text: impl Into<String>) -> Self {
        Self::new(RuleFallbackText::new(text))
    }

    pub fn keyword_marker(marker: impl Into<String>) -> Self {
        Self::keyword_fallback_text(marker)
    }

    pub fn rule_text_placeholder(text: impl Into<String>) -> Self {
        Self::rule_fallback_text(text)
    }

    pub fn unsupported_parser_line(raw_line: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(UnsupportedParserLine::new(raw_line, reason))
    }

    pub fn pregame_action(kind: PregameActionKind, text: impl Into<String>) -> Self {
        Self::new(PregameAction::new(kind, text))
    }

    pub fn cant_be_countered_ability() -> Self {
        Self::new(CantBeCountered)
    }

    /// Create a unified grant ability from a grant specification.
    ///
    /// This is the preferred way to create abilities that grant things to cards
    /// in non-battlefield zones (like granting flash to cards in hand, or
    /// granting escape to cards in graveyard).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Grant flash to noncreature spells in hand
    /// StaticAbility::grants(GrantSpec::flash_to_noncreature_spells())
    ///
    /// // Grant escape to nonland cards in graveyard
    /// StaticAbility::grants(GrantSpec::escape_to_nonland(3))
    /// ```
    pub fn grants(spec: crate::grant::GrantSpec) -> Self {
        Self::new(Grants::new(spec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_ability_equality() {
        let flying1 = StaticAbility::flying();
        let flying2 = StaticAbility::flying();
        let trample = StaticAbility::trample();

        assert_eq!(flying1, flying2);
        assert_ne!(flying1, trample);
    }

    #[test]
    fn test_static_ability_id() {
        let flying = StaticAbility::flying();
        assert_eq!(flying.id(), StaticAbilityId::Flying);

        let trample = StaticAbility::trample();
        assert_eq!(trample.id(), StaticAbilityId::Trample);
    }

    #[test]
    fn test_keyword_query_methods() {
        let flying = StaticAbility::flying();
        assert!(flying.is_keyword());
        assert!(flying.has_flying());
        assert!(flying.grants_evasion());
        assert!(!flying.has_trample());

        let trample = StaticAbility::trample();
        assert!(trample.is_keyword());
        assert!(trample.has_trample());
        assert!(!trample.has_flying());
    }

    #[test]
    fn test_static_ability_clone() {
        let flying = StaticAbility::flying();
        let cloned = flying.clone();
        assert_eq!(flying, cloned);
    }

    #[test]
    fn test_display() {
        assert_eq!(StaticAbility::flying().display(), "Flying");
        assert_eq!(StaticAbility::trample().display(), "Trample");
        assert_eq!(StaticAbility::deathtouch().display(), "Deathtouch");
    }
}
