use crate::{
    CardType, ChoiceCount, Color, ColorSet, CounterType, ObjectId, PlayerId, StaticAbilityId,
    Subtype, Supertype, TagKey, Value, Zone,
};

fn small_number_word(n: u32) -> Option<&'static str> {
    match n {
        0 => Some("zero"),
        1 => Some("one"),
        2 => Some("two"),
        3 => Some("three"),
        4 => Some("four"),
        5 => Some("five"),
        6 => Some("six"),
        7 => Some("seven"),
        8 => Some("eight"),
        9 => Some("nine"),
        10 => Some("ten"),
        _ => None,
    }
}

/// A reference to an object for use in filters and effects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ObjectRef {
    #[default]
    Target,
    Specific(ObjectId),
    Tagged(TagKey),
}

impl ObjectRef {
    pub fn tagged(tag: impl Into<TagKey>) -> Self {
        Self::Tagged(tag.into())
    }

    pub fn specific(id: ObjectId) -> Self {
        Self::Specific(id)
    }
}

/// Constraint requiring the candidate object to be a legal target for a
/// referenced stack object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetabilityConstraint {
    pub stack_object: ObjectRef,
}

impl TargetabilityConstraint {
    pub fn by_stack_object(stack_object: ObjectRef) -> Self {
        Self { stack_object }
    }
}

/// Which power/toughness reference a filter should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PtReference {
    #[default]
    Effective,
    Base,
}

/// Relationship an object may have with a tagged object set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedOpbjectRelation {
    IsTaggedObject,
    SharesCardType,
    SharesSubtypeWithTagged,
    SharesColorWithTagged,
    SameStableId,
    SameNameAsTagged,
    SameControllerAsTagged,
    SameManaValueAsTagged,
    ManaValueLteTagged,
    ManaValueLtTagged,
    AttachedToTaggedObject,
    IsNotTaggedObject,
}

/// Alternative casting capability qualifier for card filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlternativeCastKind {
    Dash,
    Flashback,
    JumpStart,
    Escape,
    Madness,
    Miracle,
}

/// Counter-state qualifier for object filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterConstraint {
    Any,
    Typed(CounterType),
}

/// A parity requirement for numeric object properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParityRequirement {
    Odd,
    Even,
    Chosen,
}

impl ParityRequirement {
    pub fn explicit_label(self) -> Option<&'static str> {
        match self {
            Self::Odd => Some("odd"),
            Self::Even => Some("even"),
            Self::Chosen => None,
        }
    }

    pub fn describe_axis(self, axis: &str) -> String {
        match self {
            Self::Odd | Self::Even => {
                format!("with {} {axis}", self.explicit_label().unwrap_or(""))
            }
            Self::Chosen => format!("with {axis} of the chosen quality"),
        }
    }
}

/// Power relationship against the source object in filter context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePowerRelation {
    LessThanSource,
}

/// Stack object kind constraint for stack-targeting filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackObjectKind {
    Spell,
    Ability,
    ActivatedAbility,
    TriggeredAbility,
    SpellOrAbility,
}

/// A tagged-object constraint used by object filters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedObjectConstraint {
    pub tag: TagKey,
    pub relation: TaggedOpbjectRelation,
}

/// Filter for selecting players.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PlayerFilter {
    #[default]
    Any,
    You,
    NotYou,
    Opponent,
    Teammate,
    Active,
    Defending,
    Attacking,
    DamagedPlayer,
    EffectController,
    Specific(PlayerId),
    MostLifeTied,
    MostCardsInHand,
    CastCardTypeThisTurn(CardType),
    CardsInHandAtLeastMoreThanYou {
        base: Box<PlayerFilter>,
        count: u32,
    },
    ChosenPlayer,
    TaggedPlayer(TagKey),
    IteratedPlayer,
    TargetPlayerOrControllerOfTarget,
    Target(Box<PlayerFilter>),
    Excluding {
        base: Box<PlayerFilter>,
        excluded: Box<PlayerFilter>,
    },
    ControllerOf(ObjectRef),
    OwnerOf(ObjectRef),
    AliasedOwnerOf(ObjectRef),
    AliasedControllerOf(ObjectRef),
}

impl PlayerFilter {
    pub fn target_player() -> Self {
        Self::Target(Box::new(Self::Any))
    }

    pub fn target_opponent() -> Self {
        Self::Target(Box::new(Self::Opponent))
    }

    pub fn excluding(base: PlayerFilter, excluded: PlayerFilter) -> Self {
        Self::Excluding {
            base: Box::new(base),
            excluded: Box::new(excluded),
        }
    }

    pub fn mentions_iterated_player(&self) -> bool {
        match self {
            Self::IteratedPlayer => true,
            Self::Target(inner) => inner.mentions_iterated_player(),
            Self::CardsInHandAtLeastMoreThanYou { base, .. } => base.mentions_iterated_player(),
            Self::Excluding { base, excluded } => {
                base.mentions_iterated_player() || excluded.mentions_iterated_player()
            }
            Self::Any
            | Self::You
            | Self::NotYou
            | Self::Opponent
            | Self::Teammate
            | Self::Active
            | Self::Defending
            | Self::Attacking
            | Self::DamagedPlayer
            | Self::EffectController
            | Self::Specific(_)
            | Self::MostLifeTied
            | Self::MostCardsInHand
            | Self::CastCardTypeThisTurn(_)
            | Self::ChosenPlayer
            | Self::TaggedPlayer(_)
            | Self::TargetPlayerOrControllerOfTarget
            | Self::ControllerOf(_)
            | Self::OwnerOf(_)
            | Self::AliasedOwnerOf(_)
            | Self::AliasedControllerOf(_) => false,
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Any => "a player".to_string(),
            Self::You => "you".to_string(),
            Self::NotYou => "a player other than you".to_string(),
            Self::Opponent => "an opponent".to_string(),
            Self::Teammate => "a teammate".to_string(),
            Self::Active => "the active player".to_string(),
            Self::Defending => "the defending player".to_string(),
            Self::Attacking => "the attacking player".to_string(),
            Self::DamagedPlayer => "that player".to_string(),
            Self::EffectController => "the player who cast this spell".to_string(),
            Self::Specific(_) => "that player".to_string(),
            Self::MostLifeTied => "a player with the most life or tied for most life".to_string(),
            Self::MostCardsInHand => "the player who has the most cards in hand".to_string(),
            Self::CastCardTypeThisTurn(card_type) => format!(
                "a player who cast one or more {} spells this turn",
                card_type.to_string().to_ascii_lowercase()
            ),
            Self::CardsInHandAtLeastMoreThanYou { base, count } => {
                let count_text = small_number_word(*count)
                    .map(str::to_string)
                    .unwrap_or_else(|| count.to_string());
                format!(
                    "{} who has at least {} more cards in hand than you do",
                    base.description(),
                    count_text
                )
            }
            Self::ChosenPlayer => "the chosen player".to_string(),
            Self::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
                "enchanted player".to_string()
            }
            Self::TaggedPlayer(_) => "that player".to_string(),
            Self::IteratedPlayer => "that player".to_string(),
            Self::TargetPlayerOrControllerOfTarget => {
                "that player or that object's controller".to_string()
            }
            Self::Target(inner) => format!("target {}", inner.description()),
            Self::Excluding { base, excluded } => {
                format!(
                    "{} other than {}",
                    base.description(),
                    excluded.description()
                )
            }
            Self::ControllerOf(_) => "that object's controller".to_string(),
            Self::OwnerOf(_) => "that object's owner".to_string(),
            Self::AliasedOwnerOf(_) | Self::AliasedControllerOf(_) => "that player".to_string(),
        }
    }
}

/// A numeric comparison for filtering.
#[derive(Debug, Clone, PartialEq)]
pub enum Comparison {
    Equal(i32),
    OneOf(Vec<i32>),
    NotEqual(i32),
    LessThan(i32),
    LessThanOrEqual(i32),
    GreaterThan(i32),
    GreaterThanOrEqual(i32),
    EqualExpr(Box<Value>),
    NotEqualExpr(Box<Value>),
    LessThanExpr(Box<Value>),
    LessThanOrEqualExpr(Box<Value>),
    GreaterThanExpr(Box<Value>),
    GreaterThanOrEqualExpr(Box<Value>),
}

impl Comparison {
    pub fn satisfies(&self, value: i32) -> bool {
        match self {
            Self::Equal(n) => value == *n,
            Self::OneOf(values) => values.contains(&value),
            Self::NotEqual(n) => value != *n,
            Self::LessThan(n) => value < *n,
            Self::LessThanOrEqual(n) => value <= *n,
            Self::GreaterThan(n) => value > *n,
            Self::GreaterThanOrEqual(n) => value >= *n,
            Self::EqualExpr(_)
            | Self::NotEqualExpr(_)
            | Self::LessThanExpr(_)
            | Self::LessThanOrEqualExpr(_)
            | Self::GreaterThanExpr(_)
            | Self::GreaterThanOrEqualExpr(_) => false,
        }
    }
}

/// Filter for selecting objects (permanents, spells, cards).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectFilter {
    pub zone: Option<Zone>,
    pub controller: Option<PlayerFilter>,
    pub cast_by: Option<PlayerFilter>,
    pub owner: Option<PlayerFilter>,
    pub single_graveyard: bool,
    pub targets_player: Option<PlayerFilter>,
    pub targets_object: Option<Box<ObjectFilter>>,
    pub targets_any_of: bool,
    pub stack_kind: Option<StackObjectKind>,
    pub target_count: Option<ChoiceCount>,
    pub targets_only_player: Option<PlayerFilter>,
    pub targets_only_object: Option<Box<ObjectFilter>>,
    pub targets_only_any_of: bool,
    pub could_be_targeted_by: Option<TargetabilityConstraint>,
    pub card_types: Vec<CardType>,
    pub all_card_types: Vec<CardType>,
    pub excluded_card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub type_or_subtype_union: bool,
    pub excluded_subtypes: Vec<Subtype>,
    pub supertypes: Vec<Supertype>,
    pub excluded_supertypes: Vec<Supertype>,
    pub colors: Option<ColorSet>,
    pub chosen_color: bool,
    pub chosen_creature_type: bool,
    pub excluded_chosen_creature_type: bool,
    pub excluded_colors: ColorSet,
    pub colorless: bool,
    pub multicolored: bool,
    pub monocolored: bool,
    pub all_colors: Option<bool>,
    pub exactly_two_colors: Option<bool>,
    pub color_count: Option<Comparison>,
    pub historic: bool,
    pub nonhistoric: bool,
    pub modified: bool,
    pub token: bool,
    pub nontoken: bool,
    pub face_down: Option<bool>,
    pub other: bool,
    pub tapped: bool,
    pub untapped: bool,
    pub attacking: bool,
    pub attacking_player_or_planeswalker_controlled_by: Option<PlayerFilter>,
    pub nonattacking: bool,
    pub blocking: bool,
    pub nonblocking: bool,
    pub blocked: bool,
    pub unblocked: bool,
    pub in_combat_with_source: bool,
    pub entered_since_your_last_turn_ended: bool,
    pub entered_battlefield_this_turn: bool,
    pub entered_battlefield_controller: Option<PlayerFilter>,
    pub entered_graveyard_this_turn: bool,
    pub entered_graveyard_from_battlefield_this_turn: bool,
    pub was_dealt_damage_this_turn: bool,
    pub drawn_this_turn: bool,
    pub power: Option<Comparison>,
    pub power_parity: Option<ParityRequirement>,
    pub power_reference: PtReference,
    pub power_relative_to_source: Option<SourcePowerRelation>,
    pub power_greater_than_base_power: bool,
    pub toughness: Option<Comparison>,
    pub toughness_reference: PtReference,
    pub total_power_toughness: Option<Comparison>,
    pub mana_value: Option<Comparison>,
    pub mana_value_parity: Option<ParityRequirement>,
    pub mana_value_eq_counters_on_source: Option<CounterType>,
    pub has_mana_cost: bool,
    pub has_tap_activated_ability: bool,
    pub no_abilities: bool,
    pub no_x_in_cost: bool,
    pub with_counter: Option<CounterConstraint>,
    pub without_counter: Option<CounterConstraint>,
    pub total_counters_parity: Option<ParityRequirement>,
    pub name: Option<String>,
    pub excluded_name: Option<String>,
    pub distinct_names: bool,
    pub distinct_powers: bool,
    pub distinct_creature_types: bool,
    pub alternative_cast: Option<AlternativeCastKind>,
    pub static_abilities: Vec<StaticAbilityId>,
    pub excluded_static_abilities: Vec<StaticAbilityId>,
    pub ability_markers: Vec<String>,
    pub excluded_ability_markers: Vec<String>,
    pub is_commander: bool,
    pub noncommander: bool,
    pub tagged_constraints: Vec<TaggedObjectConstraint>,
    pub specific: Option<ObjectId>,
    pub any_of: Vec<ObjectFilter>,
    pub source: bool,
}

impl ObjectFilter {
    pub fn uses_power_or_toughness_characteristics(&self) -> bool {
        self.power.is_some()
            || self.power_parity.is_some()
            || self.power_relative_to_source.is_some()
            || self.power_greater_than_base_power
            || self.distinct_powers
            || self.distinct_creature_types
            || self.toughness.is_some()
            || self.total_power_toughness.is_some()
            || self
                .any_of
                .iter()
                .any(Self::uses_power_or_toughness_characteristics)
    }

    pub fn uses_non_pt_battlefield_characteristics(&self) -> bool {
        self.controller.is_some()
            || !self.card_types.is_empty()
            || !self.all_card_types.is_empty()
            || !self.excluded_card_types.is_empty()
            || !self.subtypes.is_empty()
            || self.type_or_subtype_union
            || !self.excluded_subtypes.is_empty()
            || !self.supertypes.is_empty()
            || !self.excluded_supertypes.is_empty()
            || self.colors.is_some()
            || self.chosen_color
            || self.chosen_creature_type
            || self.excluded_chosen_creature_type
            || !self.excluded_colors.is_empty()
            || self.colorless
            || self.multicolored
            || self.monocolored
            || self.all_colors.is_some()
            || self.exactly_two_colors.is_some()
            || self.color_count.is_some()
            || self.historic
            || self.nonhistoric
            || self.modified
            || self.drawn_this_turn
            || self.mana_value.is_some()
            || self.mana_value_parity.is_some()
            || self.mana_value_eq_counters_on_source.is_some()
            || self.has_mana_cost
            || self.has_tap_activated_ability
            || self.no_abilities
            || self.no_x_in_cost
            || self.name.is_some()
            || self.excluded_name.is_some()
            || self.alternative_cast.is_some()
            || !self.static_abilities.is_empty()
            || !self.excluded_static_abilities.is_empty()
            || !self.ability_markers.is_empty()
            || !self.excluded_ability_markers.is_empty()
            || !self.tagged_constraints.is_empty()
            || self
                .any_of
                .iter()
                .any(Self::uses_non_pt_battlefield_characteristics)
    }

    pub fn permanent() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            ..Default::default()
        }
    }

    pub fn permanent_card() -> Self {
        Self {
            card_types: vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ],
            ..Default::default()
        }
    }

    pub fn specific(id: ObjectId) -> Self {
        Self {
            specific: Some(id),
            ..Default::default()
        }
    }

    pub fn creature() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Creature],
            ..Default::default()
        }
    }

    pub fn artifact() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Artifact],
            ..Default::default()
        }
    }

    pub fn enchantment() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Enchantment],
            ..Default::default()
        }
    }

    pub fn land() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    pub fn planeswalker() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Planeswalker],
            ..Default::default()
        }
    }

    pub fn spell() -> Self {
        Self {
            zone: Some(Zone::Stack),
            has_mana_cost: true,
            stack_kind: Some(StackObjectKind::Spell),
            ..Default::default()
        }
    }

    pub fn spell_or_ability() -> Self {
        Self {
            zone: Some(Zone::Stack),
            stack_kind: Some(StackObjectKind::SpellOrAbility),
            ..Default::default()
        }
    }

    pub fn ability() -> Self {
        Self {
            zone: Some(Zone::Stack),
            stack_kind: Some(StackObjectKind::Ability),
            ..Default::default()
        }
    }

    pub fn activated_ability() -> Self {
        Self {
            zone: Some(Zone::Stack),
            stack_kind: Some(StackObjectKind::ActivatedAbility),
            ..Default::default()
        }
    }

    pub fn instant_or_sorcery() -> Self {
        Self {
            zone: Some(Zone::Stack),
            card_types: vec![CardType::Instant, CardType::Sorcery],
            stack_kind: Some(StackObjectKind::Spell),
            ..Default::default()
        }
    }

    pub fn noncreature_spell() -> Self {
        Self {
            excluded_card_types: vec![CardType::Creature, CardType::Land],
            ..Default::default()
        }
    }

    pub fn nonland_permanent() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            excluded_card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    pub fn noncreature_permanent() -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            excluded_card_types: vec![CardType::Creature],
            ..Default::default()
        }
    }

    pub fn nonland() -> Self {
        Self {
            excluded_card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    pub fn in_zone(mut self, zone: Zone) -> Self {
        self.zone = Some(zone);
        self
    }

    pub fn with_default_zone(mut self, zone: Zone) -> Self {
        self.zone.get_or_insert(zone);
        self
    }

    pub fn ensure_zone(&mut self, zone: Zone) -> Zone {
        *self.zone.get_or_insert(zone)
    }

    pub fn has_search_stated_quality(&self) -> bool {
        let mut generic = Self::default();
        generic.zone = self.zone;
        generic.owner = self.owner.clone();
        generic.controller = self.controller.clone();
        generic.cast_by = self.cast_by.clone();
        generic.single_graveyard = self.single_graveyard;
        self != &generic
    }

    pub fn targeting(mut self, player: Option<PlayerFilter>, object: Option<ObjectFilter>) -> Self {
        self.zone = Some(Zone::Stack);
        self.targets_player = player;
        self.targets_object = object.map(Box::new);
        self
    }

    pub fn targeting_only(
        mut self,
        player: Option<PlayerFilter>,
        object: Option<ObjectFilter>,
    ) -> Self {
        self.zone = Some(Zone::Stack);
        self.targets_only_player = player;
        self.targets_only_object = object.map(Box::new);
        if self.targets_only_player.is_some() && self.targets_only_object.is_some() {
            self.targets_only_any_of = true;
        }
        self
    }

    pub fn targeting_only_player(self, player: PlayerFilter) -> Self {
        self.targeting_only(Some(player), None)
    }

    pub fn targeting_only_object(self, object: ObjectFilter) -> Self {
        self.targeting_only(None, Some(object))
    }

    pub fn with_target_count(mut self, count: ChoiceCount) -> Self {
        self.target_count = Some(count);
        self
    }

    pub fn target_count_exact(self, count: usize) -> Self {
        self.with_target_count(ChoiceCount::exactly(count))
    }

    pub fn could_be_targeted_by(mut self, stack_object: ObjectRef) -> Self {
        self.could_be_targeted_by = Some(TargetabilityConstraint::by_stack_object(stack_object));
        self
    }

    pub fn targeting_player(self, player: PlayerFilter) -> Self {
        self.targeting(Some(player), None)
    }

    pub fn targeting_object(self, object: ObjectFilter) -> Self {
        self.targeting(None, Some(object))
    }

    pub fn controlled_by(mut self, controller: PlayerFilter) -> Self {
        self.controller = Some(controller);
        self
    }

    pub fn attacking_player_or_planeswalker_controlled_by(mut self, player: PlayerFilter) -> Self {
        self.attacking_player_or_planeswalker_controlled_by = Some(player);
        self
    }

    pub fn cast_by(mut self, caster: PlayerFilter) -> Self {
        self.cast_by = Some(caster);
        self
    }

    pub fn cast_by_you(self) -> Self {
        self.cast_by(PlayerFilter::You)
    }

    pub fn you_control(self) -> Self {
        self.controlled_by(PlayerFilter::You)
    }

    pub fn opponent_controls(self) -> Self {
        self.controlled_by(PlayerFilter::Opponent)
    }

    pub fn owned_by(mut self, owner: PlayerFilter) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn single_graveyard(mut self) -> Self {
        self.single_graveyard = true;
        self
    }

    pub fn other(mut self) -> Self {
        self.other = true;
        self
    }

    pub fn with_type(mut self, card_type: CardType) -> Self {
        self.card_types.push(card_type);
        self
    }

    pub fn with_all_type(mut self, card_type: CardType) -> Self {
        self.all_card_types.push(card_type);
        self
    }

    pub fn without_type(mut self, card_type: CardType) -> Self {
        self.excluded_card_types.push(card_type);
        self
    }

    pub fn with_subtype(mut self, subtype: Subtype) -> Self {
        self.subtypes.push(subtype);
        self
    }

    pub fn without_subtype(mut self, subtype: Subtype) -> Self {
        self.excluded_subtypes.push(subtype);
        self
    }

    pub fn exclude_subtype(self, subtype: Subtype) -> Self {
        self.without_subtype(subtype)
    }

    pub fn with_supertype(mut self, supertype: Supertype) -> Self {
        self.supertypes.push(supertype);
        self
    }

    pub fn without_supertype(mut self, supertype: Supertype) -> Self {
        self.excluded_supertypes.push(supertype);
        self
    }

    pub fn nonbasic(self) -> Self {
        self.without_supertype(Supertype::Basic)
    }

    pub fn token(mut self) -> Self {
        self.token = true;
        self
    }

    pub fn nontoken(mut self) -> Self {
        self.nontoken = true;
        self
    }

    pub fn face_down(mut self) -> Self {
        self.face_down = Some(true);
        self
    }

    pub fn face_up(mut self) -> Self {
        self.face_down = Some(false);
        self
    }

    pub fn tapped(mut self) -> Self {
        self.tapped = true;
        self
    }

    pub fn untapped(mut self) -> Self {
        self.untapped = true;
        self
    }

    pub fn with_power(mut self, cmp: Comparison) -> Self {
        self.power = Some(cmp);
        self.power_reference = PtReference::Effective;
        self
    }

    pub fn with_power_parity(mut self, parity: ParityRequirement) -> Self {
        self.power_parity = Some(parity);
        self
    }

    pub fn with_base_power(mut self, cmp: Comparison) -> Self {
        self.power = Some(cmp);
        self.power_reference = PtReference::Base;
        self
    }

    pub fn with_power_less_than_source(mut self) -> Self {
        self.power_relative_to_source = Some(SourcePowerRelation::LessThanSource);
        self
    }

    pub fn with_toughness(mut self, cmp: Comparison) -> Self {
        self.toughness = Some(cmp);
        self.toughness_reference = PtReference::Effective;
        self
    }

    pub fn with_total_power_toughness(mut self, cmp: Comparison) -> Self {
        self.total_power_toughness = Some(cmp);
        self
    }

    pub fn with_base_toughness(mut self, cmp: Comparison) -> Self {
        self.toughness = Some(cmp);
        self.toughness_reference = PtReference::Base;
        self
    }

    pub fn with_mana_value(mut self, cmp: Comparison) -> Self {
        self.mana_value = Some(cmp);
        self
    }

    pub fn with_color_count(mut self, cmp: Comparison) -> Self {
        self.color_count = Some(cmp);
        self
    }

    pub fn with_mana_value_parity(mut self, parity: ParityRequirement) -> Self {
        self.mana_value_parity = Some(parity);
        self
    }

    pub fn with_total_counters_parity(mut self, parity: ParityRequirement) -> Self {
        self.total_counters_parity = Some(parity);
        self
    }

    pub fn with_colors(mut self, colors: ColorSet) -> Self {
        self.colors = Some(colors);
        self
    }

    pub fn of_chosen_color(mut self) -> Self {
        self.chosen_color = true;
        self
    }

    pub fn of_chosen_creature_type(mut self) -> Self {
        self.chosen_creature_type = true;
        self
    }

    pub fn not_of_chosen_creature_type(mut self) -> Self {
        self.excluded_chosen_creature_type = true;
        self
    }

    pub fn without_colors(mut self, colors: ColorSet) -> Self {
        self.excluded_colors = self.excluded_colors.union(colors);
        self
    }

    pub fn colorless(mut self) -> Self {
        self.colorless = true;
        self
    }

    pub fn multicolored(mut self) -> Self {
        self.multicolored = true;
        self
    }

    pub fn monocolored(mut self) -> Self {
        self.monocolored = true;
        self
    }

    pub fn historic(mut self) -> Self {
        self.historic = true;
        self
    }

    pub fn nonhistoric(mut self) -> Self {
        self.nonhistoric = true;
        self
    }

    pub fn modified(mut self) -> Self {
        self.modified = true;
        self
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn not_named(mut self, name: impl Into<String>) -> Self {
        self.excluded_name = Some(name.into());
        self
    }

    pub fn commander(mut self) -> Self {
        self.is_commander = true;
        self
    }

    pub fn noncommander(mut self) -> Self {
        self.noncommander = true;
        self
    }

    pub fn with_alternative_cast(mut self, kind: AlternativeCastKind) -> Self {
        self.alternative_cast = Some(kind);
        self
    }

    pub fn with_any_counter(mut self) -> Self {
        self.with_counter = Some(CounterConstraint::Any);
        self
    }

    pub fn with_counter_type(mut self, counter_type: CounterType) -> Self {
        self.with_counter = Some(CounterConstraint::Typed(counter_type));
        self
    }

    pub fn without_any_counter(mut self) -> Self {
        self.without_counter = Some(CounterConstraint::Any);
        self
    }

    pub fn without_counter_type(mut self, counter_type: CounterType) -> Self {
        self.without_counter = Some(CounterConstraint::Typed(counter_type));
        self
    }

    pub fn with_static_ability(mut self, ability_id: StaticAbilityId) -> Self {
        if !self.static_abilities.contains(&ability_id) {
            self.static_abilities.push(ability_id);
        }
        self
    }

    pub fn without_static_ability(mut self, ability_id: StaticAbilityId) -> Self {
        if !self.excluded_static_abilities.contains(&ability_id) {
            self.excluded_static_abilities.push(ability_id);
        }
        self
    }

    pub fn with_ability_marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !self
            .ability_markers
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&marker))
        {
            self.ability_markers.push(marker);
        }
        self
    }

    pub fn without_ability_marker(mut self, marker: impl Into<String>) -> Self {
        let marker = marker.into();
        if !self
            .excluded_ability_markers
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&marker))
        {
            self.excluded_ability_markers.push(marker);
        }
        self
    }

    pub fn with_tap_activated_ability(mut self) -> Self {
        self.has_tap_activated_ability = true;
        self
    }

    pub fn match_tagged(mut self, tag: impl Into<TagKey>, relation: TaggedOpbjectRelation) -> Self {
        self.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.into(),
            relation,
        });
        self
    }

    pub fn shares_card_type_with_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesCardType)
    }

    pub fn shares_color_with_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesColorWithTagged)
    }

    pub fn shares_subtype_with_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SharesSubtypeWithTagged)
    }

    pub fn same_stable_id_as_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::SameStableId)
    }

    pub fn tagged(tag: impl Into<TagKey>) -> Self {
        Self::default().match_tagged(tag, TaggedOpbjectRelation::IsTaggedObject)
    }

    pub fn not_tagged(self, tag: impl Into<TagKey>) -> Self {
        self.match_tagged(tag, TaggedOpbjectRelation::IsNotTaggedObject)
    }

    pub fn any_of_types(types: &[CardType]) -> Self {
        Self {
            zone: Some(Zone::Battlefield),
            card_types: types.to_vec(),
            ..Default::default()
        }
    }

    pub fn source() -> Self {
        Self {
            source: true,
            ..Default::default()
        }
    }

    pub fn description(&self) -> String {
        let any_of_keyword_clause = describe_simple_any_of_keyword_clause(&self.any_of);
        if any_of_keyword_clause.is_none() && !self.any_of.is_empty() {
            return self
                .any_of
                .iter()
                .map(ObjectFilter::description)
                .collect::<Vec<_>>()
                .join(" or ");
        }

        let mut parts = Vec::new();
        let mut post_noun_qualifiers: Vec<String> = Vec::new();
        let append_token_after_type = self.token;
        let mut controller_suffix: Option<String> = None;
        let mut owner_suffix: Option<String> = None;

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
        if self.modified {
            parts.push("modified".to_string());
        }

        let has_leading_determiner = self.other || has_target_tag || self.source;

        if let Some(ref ctrl) = self.controller {
            match ctrl {
                PlayerFilter::You => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("you control".to_string());
                }
                PlayerFilter::NotYou => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("you don't control".to_string());
                }
                PlayerFilter::Opponent => parts.push("an opponent's".to_string()),
                PlayerFilter::Any => {}
                PlayerFilter::Active => parts.push("the active player's".to_string()),
                PlayerFilter::EffectController => {
                    parts.push("the player who cast this spell's".to_string())
                }
                PlayerFilter::Specific(_) => parts.push("a specific player's".to_string()),
                PlayerFilter::MostLifeTied => {
                    parts.push("the player with the most life's".to_string())
                }
                PlayerFilter::MostCardsInHand => {
                    parts.push("the player with the most cards in hand's".to_string())
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => parts.push(format!(
                    "a player who cast one or more {} spells this turn's",
                    card_type.to_string().to_ascii_lowercase()
                )),
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
                }
                PlayerFilter::ChosenPlayer => parts.push("the chosen player's".to_string()),
                PlayerFilter::TaggedPlayer(_) => parts.push("that player's".to_string()),
                PlayerFilter::Teammate => parts.push("a teammate's".to_string()),
                PlayerFilter::Defending => parts.push("the defending player's".to_string()),
                PlayerFilter::Attacking => parts.push("an attacking player's".to_string()),
                PlayerFilter::DamagedPlayer => parts.push("the damaged player's".to_string()),
                PlayerFilter::IteratedPlayer => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix = Some("that player controls".to_string())
                }
                PlayerFilter::TargetPlayerOrControllerOfTarget => {
                    if !has_leading_determiner {
                        parts.insert(0, "a".to_string());
                    }
                    controller_suffix =
                        Some("that player or that object's controller controls".to_string())
                }
                PlayerFilter::Excluding { .. } => {
                    parts.push(describe_possessive_player_filter(ctrl));
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
                PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
                    parts.push("that player's".to_string())
                }
            }
        }

        if let Some(cast_by) = &self.cast_by {
            post_noun_qualifiers.push(format!("cast by {}", describe_player_filter(cast_by)));
        }

        let owner_conveyed_by_zone = matches!(
            self.zone,
            Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
        );
        if !owner_conveyed_by_zone && let Some(ref owner) = self.owner {
            owner_suffix = Some(match owner {
                PlayerFilter::You => "you own".to_string(),
                PlayerFilter::NotYou => "you don't own".to_string(),
                PlayerFilter::Opponent => "an opponent owns".to_string(),
                PlayerFilter::Any => "a player owns".to_string(),
                PlayerFilter::Active => "the active player owns".to_string(),
                PlayerFilter::EffectController => "the player who cast this spell owns".to_string(),
                PlayerFilter::Specific(_) => "that player owns".to_string(),
                PlayerFilter::MostLifeTied => {
                    "the player with the most life or tied for most life owns".to_string()
                }
                PlayerFilter::MostCardsInHand => {
                    "the player who has the most cards in hand owns".to_string()
                }
                PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
                    "a player who cast one or more {} spells this turn owns",
                    card_type.to_string().to_ascii_lowercase()
                ),
                PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::ChosenPlayer => "the chosen player owns".to_string(),
                PlayerFilter::TaggedPlayer(_) => "that player owns".to_string(),
                PlayerFilter::Teammate => "a teammate owns".to_string(),
                PlayerFilter::Defending => "the defending player owns".to_string(),
                PlayerFilter::Attacking => "an attacking player owns".to_string(),
                PlayerFilter::DamagedPlayer => "the damaged player owns".to_string(),
                PlayerFilter::IteratedPlayer => "that player owns".to_string(),
                PlayerFilter::TargetPlayerOrControllerOfTarget => {
                    "that player or that object's controller owns".to_string()
                }
                PlayerFilter::Excluding { .. } => {
                    format!("{} owns", describe_player_filter(owner))
                }
                PlayerFilter::Target(inner) => {
                    format!("target {} owns", describe_player_filter(inner.as_ref()))
                }
                PlayerFilter::ControllerOf(_) => "that object's controller owns".to_string(),
                PlayerFilter::OwnerOf(_) => "that object's owner owns".to_string(),
                PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
                    "that player owns".to_string()
                }
            });
        }

        if self.nontoken {
            parts.push("nontoken".to_string());
        }
        if let Some(face_down) = self.face_down {
            parts.push(if face_down {
                "face-down".to_string()
            } else {
                "face-up".to_string()
            });
        }
        if let Some(colors) = self.colors {
            if colors.contains_all(Color::ALL.into_iter().collect::<ColorSet>()) {
                parts.push("colored".to_string());
            } else {
                let mut color_words = Vec::new();
                if colors.contains(Color::White) {
                    color_words.push("white");
                }
                if colors.contains(Color::Blue) {
                    color_words.push("blue");
                }
                if colors.contains(Color::Black) {
                    color_words.push("black");
                }
                if colors.contains(Color::Red) {
                    color_words.push("red");
                }
                if colors.contains(Color::Green) {
                    color_words.push("green");
                }
                if !color_words.is_empty() {
                    parts.push(color_words.join(" or "));
                }
            }
        }
        if self.chosen_color {
            post_noun_qualifiers.push("of the chosen color".to_string());
        }
        if self.chosen_creature_type {
            post_noun_qualifiers.push("of the chosen type".to_string());
        }
        if self.excluded_chosen_creature_type {
            post_noun_qualifiers.push("that aren't of the chosen type".to_string());
        }
        for constraint in &self.tagged_constraints {
            match constraint.relation {
                TaggedOpbjectRelation::IsTaggedObject => match constraint.tag.as_str() {
                    "it" => parts.push("that".to_string()),
                    "enchanted" => parts.push("enchanted".to_string()),
                    "equipped" => parts.push("equipped".to_string()),
                    "convoked_this_spell" => {
                        post_noun_qualifiers.push("that convoked this spell".to_string());
                    }
                    "improvised_this_spell" => {
                        post_noun_qualifiers.push("that improvised this spell".to_string());
                    }
                    "crewed_it_this_turn" => {
                        post_noun_qualifiers.push("that crewed it this turn".to_string());
                    }
                    "saddled_it_this_turn" => {
                        post_noun_qualifiers.push("that saddled it this turn".to_string());
                    }
                    tag if tag == crate::SOURCE_EXILED_TAG => {
                        post_noun_qualifiers.push("exiled with this permanent".to_string());
                    }
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
                TaggedOpbjectRelation::SameManaValueAsTagged => {
                    if constraint.tag.as_str().starts_with("sacrifice_cost_") {
                        post_noun_qualifiers.push(
                            "with the same mana value as the sacrificed creature".to_string(),
                        );
                    } else {
                        post_noun_qualifiers
                            .push("with the same mana value as that object".to_string());
                    }
                }
                TaggedOpbjectRelation::ManaValueLteTagged => {
                    post_noun_qualifiers.push(
                        "with mana value less than or equal to that object's mana value"
                            .to_string(),
                    );
                }
                TaggedOpbjectRelation::ManaValueLtTagged => {
                    post_noun_qualifiers
                        .push("with lesser mana value than that object".to_string());
                }
                TaggedOpbjectRelation::SharesColorWithTagged => {
                    post_noun_qualifiers.push("that shares a color with that object".to_string());
                }
                TaggedOpbjectRelation::SharesSubtypeWithTagged => {
                    post_noun_qualifiers
                        .push("that shares a creature type with that object".to_string());
                }
                TaggedOpbjectRelation::SharesCardType => {
                    let permanent_type_context = self.zone == Some(Zone::Battlefield)
                        || (!self.card_types.is_empty()
                            && self.card_types.iter().all(|card_type| {
                                matches!(
                                    card_type,
                                    CardType::Artifact
                                        | CardType::Creature
                                        | CardType::Enchantment
                                        | CardType::Land
                                        | CardType::Planeswalker
                                        | CardType::Battle
                                )
                            }));
                    if permanent_type_context {
                        post_noun_qualifiers
                            .push("that shares a permanent type with that object".to_string());
                    } else {
                        post_noun_qualifiers
                            .push("that shares a card type with that object".to_string());
                    }
                }
                TaggedOpbjectRelation::AttachedToTaggedObject => {
                    post_noun_qualifiers.push("attached to that object".to_string());
                }
                TaggedOpbjectRelation::SameStableId => {}
            }
        }
        if !self.supertypes.is_empty() {
            for supertype in &self.supertypes {
                parts.push(supertype.name().to_string());
            }
        }
        if !self.excluded_card_types.is_empty() {
            for card_type in &self.excluded_card_types {
                parts.push(format!("non{}", describe_card_type_word(*card_type)));
            }
        }
        if !self.excluded_supertypes.is_empty() {
            for supertype in &self.excluded_supertypes {
                parts.push(format!("non{}", supertype.name()));
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
                parts.push(format!("non-{}", subtype.to_string().to_ascii_lowercase()));
            }
        }
        if !self.excluded_colors.is_empty() {
            if self.excluded_colors.contains(Color::White) {
                parts.push("nonwhite".to_string());
            }
            if self.excluded_colors.contains(Color::Blue) {
                parts.push("nonblue".to_string());
            }
            if self.excluded_colors.contains(Color::Black) {
                parts.push("nonblack".to_string());
            }
            if self.excluded_colors.contains(Color::Red) {
                parts.push("nonred".to_string());
            }
            if self.excluded_colors.contains(Color::Green) {
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
        if let Some(all_colors) = self.all_colors {
            if all_colors {
                post_noun_qualifiers.push("that are all colors".to_string());
            } else {
                post_noun_qualifiers.push("that are not all colors".to_string());
            }
        }
        if let Some(exactly_two_colors) = self.exactly_two_colors {
            if exactly_two_colors {
                post_noun_qualifiers.push("that are exactly two colors".to_string());
            } else {
                post_noun_qualifiers.push("that are not exactly two colors".to_string());
            }
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
        if self.noncommander {
            parts.push("noncommander".to_string());
        }
        if self.blocked && self.unblocked {
            parts.push("blocked/unblocked".to_string());
        } else {
            if self.blocked {
                parts.push("blocked".to_string());
            }
            if self.unblocked {
                parts.push("unblocked".to_string());
            }
        }
        if self.attacking && self.blocking {
            parts.push("attacking/blocking".to_string());
        } else {
            if self.attacking {
                parts.push("attacking".to_string());
            }
            if self.blocking {
                parts.push("blocking".to_string());
            }
        }
        if let Some(player_filter) = &self.attacking_player_or_planeswalker_controlled_by {
            let player_text = player_filter.description();
            post_noun_qualifiers.push(format!(
                "attacking {player_text} or a planeswalker controlled by {player_text}"
            ));
        }
        if self.in_combat_with_source {
            post_noun_qualifiers.push("blocking or blocked by this creature".to_string());
        }
        if self.nonattacking && self.nonblocking {
            parts.push("nonattacking/nonblocking".to_string());
        } else {
            if self.nonattacking {
                parts.push("nonattacking".to_string());
            }
            if self.nonblocking {
                parts.push("nonblocking".to_string());
            }
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
        if self.no_abilities {
            post_noun_qualifiers.push("with no abilities".to_string());
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
        let stack_source_ability = matches!(self.zone, Some(Zone::Stack))
            && matches!(
                self.stack_kind,
                Some(
                    StackObjectKind::Ability
                        | StackObjectKind::ActivatedAbility
                        | StackObjectKind::TriggeredAbility
                )
            );

        let mut type_phrase = if !self.all_card_types.is_empty() {
            Some((
                true,
                self.all_card_types
                    .iter()
                    .map(|t| t.name().to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            ))
        } else if !self.card_types.is_empty() {
            if stack_source_ability {
                let kind = self.stack_kind.unwrap_or(StackObjectKind::Ability);
                post_noun_qualifiers.push(format!(
                    "from {} source",
                    describe_card_type_source_phrase(&self.card_types)
                ));
                Some((false, describe_stack_object_kind(kind).to_string()))
            } else if has_all_permanent_types {
                Some((true, "permanent".to_string()))
            } else {
                let joiner = if self.zone == Some(Zone::Stack)
                    && self.card_types.len() == 2
                    && self.card_types.contains(&CardType::Instant)
                    && self.card_types.contains(&CardType::Sorcery)
                {
                    " or "
                } else {
                    " or "
                };
                Some((
                    true,
                    self.card_types
                        .iter()
                        .map(|t| t.name().to_string())
                        .collect::<Vec<_>>()
                        .join(joiner),
                ))
            }
        } else if !self.token && !subtype_implies_type {
            let default_noun = if self.source {
                match self.zone {
                    Some(Zone::Graveyard)
                    | Some(Zone::Hand)
                    | Some(Zone::Library)
                    | Some(Zone::Exile)
                    | Some(Zone::Command) => "card",
                    _ => "source",
                }
            } else {
                match self.zone {
                    Some(Zone::Battlefield) | None => "permanent",
                    Some(Zone::Stack) => {
                        let kind = self.stack_kind.unwrap_or_else(|| {
                            if self.has_mana_cost {
                                StackObjectKind::Spell
                            } else {
                                StackObjectKind::SpellOrAbility
                            }
                        });
                        describe_stack_object_kind(kind)
                    }
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
            let mut parts = Vec::new();
            let mut remaining = self.subtypes.clone();
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
                parts.push("outlaw".to_string());
                remaining.retain(|subtype| !outlaw_pack.contains(subtype));
            }
            parts.extend(remaining.iter().map(std::string::ToString::to_string));
            Some(parts.join(" or "))
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
        if let Some((type_is_card_type, phrase)) = type_phrase.as_mut()
            && *type_is_card_type
            && matches!(self.zone, Some(Zone::Stack))
            && !phrase.ends_with(" spell")
        {
            phrase.push_str(" spell");
        }

        let creature_only = self.all_card_types.is_empty()
            && self.card_types.len() == 1
            && self.card_types[0] == CardType::Creature;
        let land_only = self.all_card_types.is_empty()
            && self.card_types.len() == 1
            && self.card_types[0] == CardType::Land
            && matches!(self.zone, None | Some(Zone::Battlefield));
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
                (Some((_, _type_phrase)), Some(subtype_phrase)) if land_only => {
                    parts.push(subtype_phrase);
                }
                (Some((type_is_card_type, type_phrase)), Some(subtype_phrase))
                    if !type_is_card_type && type_phrase == "card" =>
                {
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
        if self.distinct_names {
            parts.push("with different names".to_string());
        }
        if self.distinct_powers {
            parts.push("with different powers".to_string());
        }
        if self.distinct_creature_types {
            parts.push("that share no creature types".to_string());
        }

        if let Some(ref name) = self.name {
            return format!("a {} named {}", parts.join(" "), name);
        }
        if let Some(ref name) = self.excluded_name {
            return format!("{} not named {}", parts.join(" "), name);
        }

        if let (Some(power), Some(toughness)) = (&self.power, &self.toughness)
            && let (Comparison::Equal(power_value), Comparison::Equal(toughness_value)) =
                (power, toughness)
            && self.power_reference == self.toughness_reference
        {
            let label = match self.power_reference {
                PtReference::Effective => "power and toughness",
                PtReference::Base => "base power and toughness",
            };
            parts.push(format!("with {label} {power_value}/{toughness_value}"));
        } else {
            if let Some(ref power) = self.power {
                let label = match self.power_reference {
                    PtReference::Effective => "power",
                    PtReference::Base => "base power",
                };
                parts.push(format!("with {label} {}", describe_comparison(power)));
            }
            if let Some(power_parity) = self.power_parity {
                let axis = match self.power_reference {
                    PtReference::Effective => "power",
                    PtReference::Base => "base power",
                };
                parts.push(power_parity.describe_axis(axis));
            }
            if self.power_greater_than_base_power {
                parts.push("with power greater than its base power".to_string());
            }
            if let Some(relation) = self.power_relative_to_source {
                match relation {
                    SourcePowerRelation::LessThanSource => {
                        parts.push("with power less than this creature's power".to_string());
                    }
                }
            }
            if let Some(ref toughness) = self.toughness {
                let label = match self.toughness_reference {
                    PtReference::Effective => "toughness",
                    PtReference::Base => "base toughness",
                };
                parts.push(format!("with {label} {}", describe_comparison(toughness)));
            }
        }
        if let Some(ref total_power_toughness) = self.total_power_toughness {
            parts.push(format!(
                "with total power and toughness {}",
                describe_comparison(total_power_toughness)
            ));
        }
        if let Some(ref mana_value) = self.mana_value {
            parts.push(format!(
                "with mana value {}",
                describe_comparison(mana_value)
            ));
        }
        if let Some(ref color_count) = self.color_count {
            parts.push(format!(
                "with color count {}",
                describe_comparison(color_count)
            ));
        }
        if let Some(mana_value_parity) = self.mana_value_parity {
            parts.push(mana_value_parity.describe_axis("mana value"));
        }
        if let Some(counter_type) = self.mana_value_eq_counters_on_source {
            parts.push(format!(
                "with mana value equal to the number of {} counters on this artifact",
                counter_type.description()
            ));
        }
        if let Some(clause) = any_of_keyword_clause {
            parts.push(format!("with {clause}"));
        }
        for ability in &self.static_abilities {
            if let Some(label) = describe_filter_static_ability(*ability) {
                parts.push(format!("with {}", label));
            }
        }
        for marker in &self.ability_markers {
            parts.push(format!("with {}", marker.to_ascii_lowercase()));
        }
        for ability in &self.excluded_static_abilities {
            if let Some(label) = describe_filter_static_ability(*ability) {
                parts.push(format!("without {}", label));
            }
        }
        for marker in &self.excluded_ability_markers {
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
        if let Some(total_counters_parity) = self.total_counters_parity {
            match total_counters_parity {
                ParityRequirement::Odd | ParityRequirement::Even => parts.push(format!(
                    "with an {} number of counters on it",
                    total_counters_parity.explicit_label().unwrap_or("")
                )),
                ParityRequirement::Chosen => {
                    parts.push("with a number of counters on it of the chosen quality".to_string())
                }
            }
        }
        if let Some(kind) = self.alternative_cast {
            parts.push(format!("with {}", describe_alternative_cast_kind(kind)));
        }
        if self.has_tap_activated_ability {
            parts.push("that has an activated ability with {T} in its cost".to_string());
        }

        let has_source_exiled_constraint = self.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == crate::SOURCE_EXILED_TAG
        });
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
            if zone == Zone::Exile && has_source_exiled_constraint {
            } else if let Some(zone_name) = zone_name {
                if let Some(owner) = &self.owner {
                    parts.push(format!(
                        "in {} {}",
                        describe_possessive_player_filter(owner),
                        zone_name
                    ));
                } else if zone == Zone::Graveyard && self.single_graveyard {
                    parts.push("in single graveyard".to_string());
                } else if zone == Zone::Graveyard {
                    parts.push("in a graveyard".to_string());
                } else {
                    parts.push(format!("in {}", zone_name));
                }
            }
        }

        if (self.entered_battlefield_this_turn || self.entered_battlefield_controller.is_some())
            && self.zone == Some(Zone::Battlefield)
        {
            let clause = if let Some(controller) = &self.entered_battlefield_controller {
                match controller {
                    PlayerFilter::You => {
                        "that entered the battlefield under your control this turn".to_string()
                    }
                    PlayerFilter::Opponent => {
                        "that entered the battlefield under an opponent's control this turn"
                            .to_string()
                    }
                    PlayerFilter::Any => "that entered the battlefield this turn".to_string(),
                    other => format!(
                        "that entered the battlefield under {} control this turn",
                        describe_possessive_player_filter(other)
                    ),
                }
            } else {
                "that entered the battlefield this turn".to_string()
            };
            parts.push(clause);
        }

        if self.entered_graveyard_from_battlefield_this_turn && self.zone == Some(Zone::Graveyard) {
            parts.push("that was put there from the battlefield this turn".to_string());
        } else if self.entered_graveyard_this_turn && self.zone == Some(Zone::Graveyard) {
            parts.push("that was put there from anywhere this turn".to_string());
        }

        if self.was_dealt_damage_this_turn {
            parts.push("that was dealt damage this turn".to_string());
        }
        if self.drawn_this_turn {
            parts.push("drawn this turn".to_string());
        }

        match (controller_suffix, owner_suffix) {
            (Some(controller), Some(owner))
                if controller == "you control" && owner == "you own" =>
            {
                parts.push("you both own and control".to_string());
            }
            (Some(controller), Some(owner))
                if controller == "that player controls" && owner == "that player owns" =>
            {
                parts.push("that player both owns and controls".to_string());
            }
            (Some(controller), Some(owner)) => {
                parts.push(controller);
                parts.push(owner);
            }
            (Some(controller), None) => parts.push(controller),
            (None, Some(owner)) => parts.push(owner),
            (None, None) => {}
        }

        let ensure_indefinite_article = |text: String| -> String {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return "a permanent".to_string();
            }
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("a ")
                || lower.starts_with("an ")
                || lower.starts_with("the ")
                || lower.starts_with("another ")
                || lower.starts_with("each ")
                || lower.starts_with("all ")
                || lower.starts_with("this ")
                || lower.starts_with("that ")
                || lower.starts_with("those ")
                || lower.starts_with("target ")
                || lower.starts_with("any ")
                || lower.starts_with("up to ")
                || lower.starts_with("at least ")
                || lower.chars().next().is_some_and(|ch| ch.is_ascii_digit())
            {
                return trimmed.to_string();
            }
            let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
            let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
                "an"
            } else {
                "a"
            };
            format!("{article} {trimmed}")
        };

        let mut appended_targeting_only = false;
        if self.targets_only_player.is_some() || self.targets_only_object.is_some() {
            let mut target_fragments = Vec::new();
            if let Some(player_filter) = &self.targets_only_player {
                let mut text = describe_player_filter(player_filter);
                if text != "you" {
                    text = ensure_indefinite_article(text);
                }
                target_fragments.push(text);
            }
            if let Some(object_filter) = &self.targets_only_object {
                let mut text = ensure_indefinite_article(object_filter.description());
                if let Some(count) = self.target_count
                    && count.is_single()
                    && (text.starts_with("a ") || text.starts_with("an "))
                {
                    if let Some(rest) = text.strip_prefix("a ") {
                        text = format!("a single {rest}");
                    } else if let Some(rest) = text.strip_prefix("an ") {
                        text = format!("a single {rest}");
                    }
                }
                target_fragments.push(text);
            }
            if !target_fragments.is_empty() {
                let target_text = if target_fragments.len() == 2 {
                    let joiner = if self.targets_only_any_of {
                        "or"
                    } else {
                        "and"
                    };
                    format!("{} {} {}", target_fragments[0], joiner, target_fragments[1])
                } else {
                    target_fragments[0].clone()
                };
                parts.push(format!("that targets only {target_text}"));
                appended_targeting_only = true;
            }
        }

        if let Some(count) = self.target_count
            && !appended_targeting_only
        {
            let phrase = if count.is_single() {
                Some("with a single target".to_string())
            } else if let Some(max) = count.max {
                if count.min == max {
                    Some(format!("with {} targets", max))
                } else if count.min == 0 {
                    Some(format!("with up to {} targets", max))
                } else {
                    Some(format!("with between {} and {} targets", count.min, max))
                }
            } else if count.min == 0 {
                Some("with any number of targets".to_string())
            } else {
                Some(format!("with at least {} targets", count.min))
            };
            if let Some(phrase) = phrase {
                parts.push(phrase);
            }
        }

        if !appended_targeting_only {
            let mut target_fragments = Vec::new();
            if let Some(player_filter) = &self.targets_player {
                let mut text = describe_player_filter(player_filter);
                if text != "you" {
                    text = ensure_indefinite_article(text);
                }
                target_fragments.push(text);
            }
            if let Some(object_filter) = &self.targets_object {
                target_fragments.push(ensure_indefinite_article(object_filter.description()));
            }
            if !target_fragments.is_empty() {
                let target_text = if target_fragments.len() == 2 {
                    let joiner = if self.targets_any_of { "or" } else { "and" };
                    format!("{} {} {}", target_fragments[0], joiner, target_fragments[1])
                } else {
                    target_fragments[0].clone()
                };
                parts.push(format!("that targets {target_text}"));
            }
        }

        if let Some(targetability) = &self.could_be_targeted_by {
            let stack_text = match &targetability.stack_object {
                ObjectRef::Target => "that spell",
                ObjectRef::Tagged(tag)
                    if matches!(tag.as_str(), "triggering" | "__it__" | "it") =>
                {
                    "that spell"
                }
                ObjectRef::Tagged(tag) if tag.as_str().contains("copied") => "the copy",
                ObjectRef::Tagged(_) | ObjectRef::Specific(_) => "that object",
            };
            parts.push(format!("{stack_text} could target"));
        }

        parts.join(" ")
    }
}

fn describe_simple_any_of_keyword_clause(any_of: &[ObjectFilter]) -> Option<String> {
    if any_of.len() < 2 {
        return None;
    }

    let mut labels = Vec::new();
    for filter in any_of {
        if !filter.any_of.is_empty() {
            return None;
        }

        let mut stripped = filter.clone();
        stripped.static_abilities.clear();
        stripped.excluded_static_abilities.clear();
        stripped.ability_markers.clear();
        stripped.excluded_ability_markers.clear();
        if stripped != ObjectFilter::default() {
            return None;
        }

        if filter.static_abilities.len() == 1 && filter.ability_markers.is_empty() {
            let label = describe_filter_static_ability(filter.static_abilities[0])?;
            labels.push(label.to_string());
            continue;
        }
        if filter.ability_markers.len() == 1 && filter.static_abilities.is_empty() {
            labels.push(filter.ability_markers[0].to_ascii_lowercase());
            continue;
        }

        return None;
    }

    Some(labels.join(" or "))
}

fn describe_possessive_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "a player's".to_string(),
        PlayerFilter::You => "your".to_string(),
        PlayerFilter::NotYou => "a non-you player's".to_string(),
        PlayerFilter::Opponent => "an opponent's".to_string(),
        PlayerFilter::Teammate => "a teammate's".to_string(),
        PlayerFilter::Active => "the active player's".to_string(),
        PlayerFilter::Defending => "the defending player's".to_string(),
        PlayerFilter::Attacking => "an attacking player's".to_string(),
        PlayerFilter::DamagedPlayer => "the damaged player's".to_string(),
        PlayerFilter::EffectController => "the player who cast this spell's".to_string(),
        PlayerFilter::Specific(_) => "that player's".to_string(),
        PlayerFilter::MostLifeTied => "the chosen player's".to_string(),
        PlayerFilter::MostCardsInHand => "the player with the most cards in hand's".to_string(),
        PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
            "a player who cast one or more {} spells this turn's",
            card_type.to_string().to_ascii_lowercase()
        ),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            let count_text = small_number_word(*count)
                .map(str::to_string)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} who has at least {count_text} more cards in hand than you do's",
                describe_player_filter(base)
            )
        }
        PlayerFilter::ChosenPlayer => "the chosen player's".to_string(),
        PlayerFilter::TaggedPlayer(_) => "that player's".to_string(),
        PlayerFilter::IteratedPlayer => "that player's".to_string(),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller's".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            describe_possessive_player_filter(base),
            describe_possessive_player_filter(excluded)
        ),
        PlayerFilter::Target(inner) => {
            let base = match inner.as_ref() {
                PlayerFilter::Any => "target player".to_string(),
                other => format!("target {}", describe_player_filter(other)),
            };
            format!("{base}'s")
        }
        PlayerFilter::ControllerOf(_) => "that object's controller's".to_string(),
        PlayerFilter::OwnerOf(_) => "that object's owner's".to_string(),
        PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
            "that player's".to_string()
        }
    }
}

pub(crate) fn describe_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Any => "player".to_string(),
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::NotYou => "player other than you".to_string(),
        PlayerFilter::Opponent => "opponent".to_string(),
        PlayerFilter::Teammate => "teammate".to_string(),
        PlayerFilter::Active => "active player".to_string(),
        PlayerFilter::Defending => "defending player".to_string(),
        PlayerFilter::Attacking => "attacking player".to_string(),
        PlayerFilter::DamagedPlayer => "damaged player".to_string(),
        PlayerFilter::EffectController => "the player who cast this spell".to_string(),
        PlayerFilter::Specific(_) => "player".to_string(),
        PlayerFilter::MostLifeTied => "player with the most life or tied for most life".to_string(),
        PlayerFilter::MostCardsInHand => "the player who has the most cards in hand".to_string(),
        PlayerFilter::CastCardTypeThisTurn(card_type) => format!(
            "player who cast one or more {} spells this turn",
            card_type.to_string().to_ascii_lowercase()
        ),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            let count_text = small_number_word(*count)
                .map(str::to_string)
                .unwrap_or_else(|| count.to_string());
            format!(
                "{} who has at least {count_text} more cards in hand than you do",
                describe_player_filter(base)
            )
        }
        PlayerFilter::ChosenPlayer => "chosen player".to_string(),
        PlayerFilter::TaggedPlayer(tag) if tag.as_str() == "enchanted" => {
            "enchanted player".to_string()
        }
        PlayerFilter::TaggedPlayer(_) => "that player".to_string(),
        PlayerFilter::IteratedPlayer => "that player".to_string(),
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            "that player or that object's controller".to_string()
        }
        PlayerFilter::Excluding { base, excluded } => format!(
            "{} other than {}",
            describe_player_filter(base),
            describe_player_filter(excluded)
        ),
        PlayerFilter::Target(inner) => format!("target {}", describe_player_filter(inner)),
        PlayerFilter::ControllerOf(_) => "controller".to_string(),
        PlayerFilter::OwnerOf(_) => "owner".to_string(),
        PlayerFilter::AliasedOwnerOf(_) | PlayerFilter::AliasedControllerOf(_) => {
            "that player".to_string()
        }
    }
}

fn describe_card_type_word(card_type: CardType) -> &'static str {
    card_type.name()
}

fn describe_card_type_list(card_types: &[CardType]) -> String {
    match card_types {
        [] => String::new(),
        [single] => single.name().to_string(),
        [first, second] => format!("{} or {}", first.name(), second.name()),
        _ => {
            let mut names = card_types
                .iter()
                .map(|card_type| card_type.name())
                .collect::<Vec<_>>();
            let last = names.pop().expect("card type list is non-empty");
            format!("{}, or {}", names.join(", "), last)
        }
    }
}

fn describe_card_type_source_phrase(card_types: &[CardType]) -> String {
    let types = describe_card_type_list(card_types);
    if types.is_empty() {
        return "a source".to_string();
    }
    let article = if types
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {types}")
}

fn describe_stack_object_kind(kind: StackObjectKind) -> &'static str {
    match kind {
        StackObjectKind::Spell => "spell",
        StackObjectKind::Ability => "ability",
        StackObjectKind::ActivatedAbility => "activated ability",
        StackObjectKind::TriggeredAbility => "triggered ability",
        StackObjectKind::SpellOrAbility => "spell or ability",
    }
}

fn describe_counter_constraint(constraint: CounterConstraint) -> String {
    match constraint {
        CounterConstraint::Any => "a counter".to_string(),
        CounterConstraint::Typed(counter_type) => {
            format!("a {} counter", counter_type.description())
        }
    }
}

fn describe_alternative_cast_kind(kind: AlternativeCastKind) -> &'static str {
    match kind {
        AlternativeCastKind::Dash => "dash",
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
        Skulk => Some("skulk"),
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
    fn describe_value_expr(value: &Value) -> String {
        match value {
            Value::Fixed(v) => v.to_string(),
            Value::X => "X".to_string(),
            Value::Count(filter) => format!("the number of {}", filter.description()),
            Value::CountScaled(filter, factor) => {
                format!("{factor} times the number of {}", filter.description())
            }
            Value::LandsEnteredBattlefieldThisTurn(player) => {
                format!(
                    "the number of lands that entered the battlefield under {:?}'s control this turn",
                    player
                )
            }
            Value::ColorsAmong(filter) => {
                format!("the number of colors among {}", filter.description())
            }
            Value::CardTypesAmong(filter) => {
                format!("the number of card types among {}", filter.description())
            }
            Value::CreatureTypesAmong(filter) => {
                format!(
                    "the number of creature types among {}",
                    filter.description()
                )
            }
            Value::StartingLifeTotal(player) => format!("{player:?}'s starting life total"),
            Value::CountersOnSource(counter_type) => {
                format!(
                    "the number of {} counters on this",
                    counter_type.description()
                )
            }
            Value::CountersOn(_, Some(counter_type)) => {
                format!("the number of {} counters", counter_type.description())
            }
            Value::CountersOn(_, None) => "the number of counters".to_string(),
            Value::SourcePower => "this creature's power".to_string(),
            Value::SourceToughness => "this creature's toughness".to_string(),
            Value::Add(left, right) => {
                format!(
                    "{} plus {}",
                    describe_value_expr(left),
                    describe_value_expr(right)
                )
            }
            _ => "a dynamic value".to_string(),
        }
    }

    let describe_values = |values: &[i32]| -> String {
        match values.len() {
            0 => String::new(),
            1 => values[0].to_string(),
            2 => format!("{} or {}", values[0], values[1]),
            _ => {
                let head = values[..values.len() - 1]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{head}, or {}", values[values.len() - 1])
            }
        }
    };
    match cmp {
        Comparison::Equal(v) => format!("{v}"),
        Comparison::OneOf(values) => describe_values(values),
        Comparison::NotEqual(v) => format!("not equal to {v}"),
        Comparison::LessThan(v) => format!("less than {v}"),
        Comparison::LessThanOrEqual(v) => format!("{v} or less"),
        Comparison::GreaterThan(v) => format!("greater than {v}"),
        Comparison::GreaterThanOrEqual(v) => format!("{v} or greater"),
        Comparison::EqualExpr(value) => format!("equal to {}", describe_value_expr(value)),
        Comparison::NotEqualExpr(value) => {
            format!("not equal to {}", describe_value_expr(value))
        }
        Comparison::LessThanExpr(value) => format!("less than {}", describe_value_expr(value)),
        Comparison::LessThanOrEqualExpr(value) => {
            format!("{} or less", describe_value_expr(value))
        }
        Comparison::GreaterThanExpr(value) => {
            format!("greater than {}", describe_value_expr(value))
        }
        Comparison::GreaterThanOrEqualExpr(value) => {
            format!("{} or greater", describe_value_expr(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ObjectRef, ParityRequirement, PlayerFilter, PtReference, StackObjectKind,
        TaggedObjectConstraint, TaggedOpbjectRelation,
    };
    use crate::{CardType, ObjectId, TagKey};

    #[test]
    fn object_ref_helpers_preserve_payloads() {
        assert_eq!(
            ObjectRef::tagged("destroyed"),
            ObjectRef::Tagged(TagKey::from("destroyed"))
        );
        assert_eq!(
            ObjectRef::specific(ObjectId::from_raw(7)),
            ObjectRef::Specific(ObjectId::from_raw(7))
        );
    }

    #[test]
    fn player_filter_builders_and_descriptions_are_stable() {
        assert_eq!(
            PlayerFilter::target_player().description(),
            "target a player"
        );
        assert_eq!(
            PlayerFilter::target_opponent().description(),
            "target an opponent"
        );
        assert_eq!(
            PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::Defending).description(),
            "an opponent other than the defending player"
        );
        assert_eq!(
            PlayerFilter::CastCardTypeThisTurn(CardType::Artifact).description(),
            "a player who cast one or more artifact spells this turn"
        );
    }

    #[test]
    fn iterated_player_detection_only_flags_dynamic_variants() {
        assert!(PlayerFilter::IteratedPlayer.mentions_iterated_player());
        assert!(PlayerFilter::target_player().mentions_iterated_player() == false);
        assert!(
            PlayerFilter::excluding(PlayerFilter::IteratedPlayer, PlayerFilter::Opponent)
                .mentions_iterated_player()
        );
    }

    #[test]
    fn filter_support_types_keep_expected_defaults() {
        assert_eq!(PtReference::default(), PtReference::Effective);
        assert_eq!(ObjectRef::default(), ObjectRef::Target);
        assert_eq!(
            TaggedObjectConstraint {
                tag: TagKey::from("chosen"),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            }
            .relation,
            TaggedOpbjectRelation::IsTaggedObject
        );
        assert_eq!(
            StackObjectKind::SpellOrAbility,
            StackObjectKind::SpellOrAbility
        );
    }

    #[test]
    fn parity_requirement_formats_axes() {
        assert_eq!(ParityRequirement::Odd.explicit_label(), Some("odd"));
        assert_eq!(
            ParityRequirement::Even.describe_axis("mana value"),
            "with even mana value"
        );
        assert_eq!(
            ParityRequirement::Chosen.describe_axis("power"),
            "with power of the chosen quality"
        );
    }
}
