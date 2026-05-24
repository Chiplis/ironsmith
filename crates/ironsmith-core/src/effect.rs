//! Shared effect-domain metadata.
//!
//! These types describe effect identity and selection cardinality without
//! pulling in the runtime execution engine.

use crate::filter_model::{ObjectFilter, ObjectRef, PlayerFilter};
use crate::mana::{ManaCost, ManaSymbol};
use crate::tag::TagKey;
use crate::target_model::ChooseSpec;
use crate::types::{CardType, Subtype, Supertype};
use crate::value_model::{Restriction, Value, ValueSurfaceHint};
use crate::{Color, ColorSet};

/// Identifier for an effect within an effect sequence.
///
/// Used to reference effects for conditional logic ("if you do" patterns).
/// Effects are labeled with `Effect::WithId` and referenced by `Effect::If`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectId(pub u32);

impl EffectId {
    /// Special ID used by ForEachControllerOfTaggedEffect to store the count
    /// of tagged objects for the current controller during iteration.
    pub const TAGGED_COUNT: Self = Self(u32::MAX);
}

impl From<u32> for EffectId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

/// Specifies how many objects/players to choose.
///
/// Used for effects like "Exile any number of target spells" (Mindbreak Trap)
/// or "Choose up to two target creatures".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceCount {
    /// Minimum number to choose (0 for "any number" or "up to").
    pub min: usize,
    /// Maximum number to choose. None means unlimited ("any number").
    pub max: Option<usize>,
    /// Whether this count came from a dynamic `X target ...` clause.
    pub dynamic_x: bool,
    /// Whether a dynamic X count is optional ("up to X") instead of exact.
    pub up_to_x: bool,
    /// Whether the chosen object(s) should be selected at random.
    pub random: bool,
}

/// Distinguishes exact, optional, and "all matching" search instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSelectionMode {
    /// "a card", "three cards", or other exact-count search phrasing.
    Exact,
    /// "up to N", "any number", or otherwise optional search phrasing.
    Optional,
    /// "all cards ..." search phrasing.
    AllMatching,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Until {
    #[default]
    Forever,
    EndOfTurn,
    YourNextTurn,
    YourNextUpkeep,
    ControllersNextUntapStep,
    EndOfCombat,
    ThisLeavesTheBattlefield,
    YouStopControllingThis,
    TurnsPass(crate::value_model::Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectPredicate {
    Succeeded,
    Failed,
    Happened,
    DidNotHappen,
    HappenedNotReplaced,
    Value(crate::effect_model::Comparison),
    Chosen,
    WasDeclined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantPlayTaggedDuration {
    UntilEndOfTurn,
    UntilYourNextTurnEnd,
    ForAsLongAsExiled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementApplyMode {
    OneShot,
    UntilEndOfTurn,
    Resolution,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreventNextTimeDamageSource {
    Choice,
    Filter(crate::filter_model::ObjectFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreventNextTimeDamageTarget {
    AnyTarget,
    You,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectNextTimeDamageSource {
    Choice,
    Filter(crate::filter_model::ObjectFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RetargetMode {
    All,
    OneToFixed(crate::target_model::ChooseSpec),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DelayedTriggerSpec {
    BeginningOfUpkeep(PlayerFilter),
    BeginningOfDrawStep(PlayerFilter),
    BeginningOfEndStep(PlayerFilter),
    EndOfCombat,
    ThisDies,
    ThisAttacksAndIsntBlocked,
    Attacks(ObjectFilter),
    AttacksAndIsntBlocked(ObjectFilter),
    AttacksOneOrMore(ObjectFilter),
    Blocks(ObjectFilter),
    Dies(ObjectFilter),
    DealsCombatDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
    },
    IsDealtDamage(ChooseSpec),
    PutIntoGraveyard(ObjectFilter),
    PutIntoGraveyardFromZone {
        filter: ObjectFilter,
        from: crate::zone::Zone,
        one_or_more: bool,
    },
    SpellCast {
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleDelayedTriggerEffect<E> {
    pub trigger: DelayedTriggerSpec,
    pub effects: Vec<E>,
    pub one_shot: bool,
    pub start_next_turn: bool,
    pub until_end_of_turn: bool,
    pub target_choices: Vec<ChooseSpec>,
    pub target_tag: Option<crate::tag::TagKey>,
    pub target_filter: Option<ObjectFilter>,
    pub controller: PlayerFilter,
}

impl<E> ScheduleDelayedTriggerEffect<E> {
    pub fn new(
        trigger: DelayedTriggerSpec,
        effects: impl Into<Vec<E>>,
        one_shot: bool,
        target_choices: Vec<ChooseSpec>,
        controller: PlayerFilter,
    ) -> Self {
        Self {
            trigger,
            effects: effects.into(),
            one_shot,
            start_next_turn: false,
            until_end_of_turn: false,
            target_choices,
            target_tag: None,
            target_filter: None,
            controller,
        }
    }

    pub fn from_tag(
        tag: crate::tag::TagKey,
        trigger: DelayedTriggerSpec,
        effects: impl Into<Vec<E>>,
        one_shot: bool,
        target_choices: Vec<ChooseSpec>,
        controller: PlayerFilter,
    ) -> Self {
        Self {
            target_tag: Some(tag),
            ..Self::new(trigger, effects, one_shot, target_choices, controller)
        }
    }

    pub fn with_target_filter(mut self, filter: ObjectFilter) -> Self {
        self.target_filter = Some(filter);
        self
    }

    pub fn starting_next_turn(mut self) -> Self {
        self.start_next_turn = true;
        self
    }

    pub fn until_end_of_turn(mut self) -> Self {
        self.until_end_of_turn = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplyContinuousEffect<
    Target,
    Modification,
    RuntimeModification,
    Condition = (),
    SourceType = (),
> {
    pub target: Target,
    pub target_spec: Option<ChooseSpec>,
    pub modification: Option<Modification>,
    pub additional_modifications: Vec<Modification>,
    pub runtime_modifications: Vec<RuntimeModification>,
    pub until: Until,
    pub condition: Option<Condition>,
    pub source_type: Option<SourceType>,
    pub lock_filter_at_resolution: bool,
    pub resolve_set_pt_values_at_resolution: bool,
    pub require_creature_target: bool,
}

impl<Target, Modification, RuntimeModification, Condition, SourceType>
    ApplyContinuousEffect<Target, Modification, RuntimeModification, Condition, SourceType>
{
    pub fn new(target: Target, modification: Modification, until: Until) -> Self {
        Self {
            target,
            target_spec: None,
            modification: Some(modification),
            additional_modifications: Vec::new(),
            runtime_modifications: Vec::new(),
            until,
            condition: None,
            source_type: None,
            lock_filter_at_resolution: false,
            resolve_set_pt_values_at_resolution: false,
            require_creature_target: false,
        }
    }

    pub fn new_runtime(target: Target, modification: RuntimeModification, until: Until) -> Self {
        Self {
            target,
            target_spec: None,
            modification: None,
            additional_modifications: Vec::new(),
            runtime_modifications: vec![modification],
            until,
            condition: None,
            source_type: None,
            lock_filter_at_resolution: false,
            resolve_set_pt_values_at_resolution: false,
            require_creature_target: false,
        }
    }

    pub fn with_spec(target_spec: ChooseSpec, modification: Modification, until: Until) -> Self
    where
        Target: From<ChooseSpec>,
    {
        Self {
            target: target_spec.clone().into(),
            target_spec: Some(target_spec),
            modification: Some(modification),
            additional_modifications: Vec::new(),
            runtime_modifications: Vec::new(),
            until,
            condition: None,
            source_type: None,
            lock_filter_at_resolution: false,
            resolve_set_pt_values_at_resolution: false,
            require_creature_target: false,
        }
    }

    pub fn with_spec_runtime(
        target_spec: ChooseSpec,
        modification: RuntimeModification,
        until: Until,
    ) -> Self
    where
        Target: From<ChooseSpec>,
    {
        Self {
            target: target_spec.clone().into(),
            target_spec: Some(target_spec),
            modification: None,
            additional_modifications: Vec::new(),
            runtime_modifications: vec![modification],
            until,
            condition: None,
            source_type: None,
            lock_filter_at_resolution: false,
            resolve_set_pt_values_at_resolution: false,
            require_creature_target: false,
        }
    }

    pub fn with_additional_modification(mut self, modification: Modification) -> Self {
        self.additional_modifications.push(modification);
        self
    }

    pub fn with_additional_runtime_modification(
        mut self,
        modification: RuntimeModification,
    ) -> Self {
        self.runtime_modifications.push(modification);
        self
    }

    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_source_type(mut self, source_type: SourceType) -> Self {
        self.source_type = Some(source_type);
        self
    }

    pub fn lock_filter_at_resolution(mut self) -> Self {
        self.lock_filter_at_resolution = true;
        self
    }

    pub fn resolve_set_pt_values_at_resolution(mut self) -> Self {
        self.resolve_set_pt_values_at_resolution = true;
        self
    }

    pub fn require_creature_target(mut self) -> Self {
        self.require_creature_target = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NewTargetRestriction {
    Player(PlayerFilter),
    Object(ObjectFilter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTypeConstraint {
    CardType,
    PermanentType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExchangeValueOperand {
    LifeTotal(crate::filter_model::PlayerFilter),
    Power(crate::target_model::ChooseSpec),
    Toughness(crate::target_model::ChooseSpec),
}

impl Default for ChoiceCount {
    fn default() -> Self {
        Self::exactly(1)
    }
}

impl ChoiceCount {
    /// Exactly N (the default for most effects).
    pub const fn exactly(n: usize) -> Self {
        Self {
            min: n,
            max: Some(n),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    }

    /// Any number (0 or more, unlimited).
    pub const fn any_number() -> Self {
        Self {
            min: 0,
            max: None,
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    }

    /// At least N (N or more, unlimited).
    pub const fn at_least(n: usize) -> Self {
        Self {
            min: n,
            max: None,
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    }

    /// Up to N (0 to N).
    pub const fn up_to(n: usize) -> Self {
        Self {
            min: 0,
            max: Some(n),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    }

    /// Dynamic X-target count (rendered as `X target ...`).
    pub const fn dynamic_x() -> Self {
        Self {
            min: 0,
            max: None,
            dynamic_x: true,
            up_to_x: false,
            random: false,
        }
    }

    /// Dynamic "up to X" count.
    pub const fn up_to_dynamic_x() -> Self {
        Self {
            min: 0,
            max: None,
            dynamic_x: true,
            up_to_x: true,
            random: false,
        }
    }

    /// Returns true if this is "any number" (min 0, no max).
    pub fn is_any_number(&self) -> bool {
        self.min == 0 && self.max.is_none() && !self.dynamic_x
    }

    /// Returns true if this is exactly 1.
    pub fn is_single(&self) -> bool {
        self.min == 1 && self.max == Some(1)
    }

    pub const fn is_dynamic_x(&self) -> bool {
        self.dynamic_x
    }

    pub const fn is_up_to_dynamic_x(&self) -> bool {
        self.dynamic_x && self.up_to_x
    }

    pub const fn is_random(&self) -> bool {
        self.random
    }

    pub fn at_random(mut self) -> Self {
        self.random = true;
        self
    }
}

impl From<usize> for ChoiceCount {
    fn from(value: usize) -> Self {
        ChoiceCount::exactly(value)
    }
}

impl From<i32> for ChoiceCount {
    fn from(value: i32) -> Self {
        if value <= 0 {
            ChoiceCount::exactly(0)
        } else {
            ChoiceCount::exactly(value as usize)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DealDamageEffect {
    pub amount: Value,
    pub target: ChooseSpec,
    pub source_is_combat: bool,
}

impl DealDamageEffect {
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            target,
            source_is_combat: false,
        }
    }

    pub fn with_combat(mut self, is_combat: bool) -> Self {
        self.source_is_combat = is_combat;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawCardsEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl DrawCardsEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TargetOnlyEffect {
    pub target: ChooseSpec,
}

impl TargetOnlyEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TapEffect {
    pub target: ChooseSpec,
}

impl TapEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self { target }
    }

    pub fn target(target: ChooseSpec) -> Self {
        Self {
            target: ChooseSpec::target(target),
        }
    }

    pub fn targets(target: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            target: ChooseSpec::target(target).with_count(count),
        }
    }

    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            target: ChooseSpec::all(filter),
        }
    }

    pub fn source() -> Self {
        Self {
            target: ChooseSpec::Source,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UntapEffect {
    pub target: ChooseSpec,
}

impl UntapEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self { target }
    }

    pub fn target(target: ChooseSpec) -> Self {
        Self {
            target: ChooseSpec::target(target),
        }
    }

    pub fn targets(target: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            target: ChooseSpec::target(target).with_count(count),
        }
    }

    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            target: ChooseSpec::all(filter),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutCountersEffect {
    pub counter_type: crate::counter::CounterType,
    pub amount: Value,
    pub target: ChooseSpec,
    pub target_count: Option<ChoiceCount>,
    pub distributed: bool,
}

impl PutCountersEffect {
    pub fn new(
        counter_type: crate::counter::CounterType,
        amount: impl Into<Value>,
        target: ChooseSpec,
    ) -> Self {
        Self {
            counter_type,
            amount: amount.into(),
            target,
            target_count: None,
            distributed: false,
        }
    }

    pub fn with_target_count(mut self, count: ChoiceCount) -> Self {
        self.target_count = Some(count);
        self
    }

    pub fn with_distributed(mut self, distributed: bool) -> Self {
        self.distributed = distributed;
        self
    }

    pub fn plus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(crate::counter::CounterType::PlusOnePlusOne, count, target)
    }

    pub fn minus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(crate::counter::CounterType::MinusOneMinusOne, count, target)
    }

    pub fn on_source(counter_type: crate::counter::CounterType, count: impl Into<Value>) -> Self {
        Self::new(counter_type, count, ChooseSpec::Source)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveCountersEffect {
    pub counter_type: crate::counter::CounterType,
    pub count: Value,
    pub target: ChooseSpec,
}

impl RemoveCountersEffect {
    pub fn new(
        counter_type: crate::counter::CounterType,
        count: impl Into<Value>,
        target: ChooseSpec,
    ) -> Self {
        Self {
            counter_type,
            count: count.into(),
            target,
        }
    }

    pub fn plus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(crate::counter::CounterType::PlusOnePlusOne, count, target)
    }

    pub fn minus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(crate::counter::CounterType::MinusOneMinusOne, count, target)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CounterEffect {
    pub target: ChooseSpec,
}

impl CounterEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    pub fn any_spell() -> Self {
        Self::new(ChooseSpec::spell())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalEffect<E> {
    pub condition: crate::value_model::Condition,
    pub if_true: Vec<E>,
    pub if_false: Vec<E>,
}

impl<E> ConditionalEffect<E> {
    pub fn new(
        condition: crate::value_model::Condition,
        if_true: Vec<E>,
        if_false: Vec<E>,
    ) -> Self {
        Self {
            condition,
            if_true,
            if_false,
        }
    }

    pub fn if_only(condition: crate::value_model::Condition, if_true: Vec<E>) -> Self {
        Self::new(condition, if_true, vec![])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfEffect<E> {
    pub condition: EffectId,
    pub predicate: EffectPredicate,
    pub then: Vec<E>,
    pub else_: Vec<E>,
}

impl<E> IfEffect<E> {
    pub fn new(
        condition: EffectId,
        predicate: EffectPredicate,
        then: Vec<E>,
        else_: Vec<E>,
    ) -> Self {
        Self {
            condition,
            predicate,
            then,
            else_,
        }
    }

    pub fn if_then(condition: EffectId, predicate: EffectPredicate, then: Vec<E>) -> Self {
        Self::new(condition, predicate, then, vec![])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WithIdEffect<E> {
    pub id: EffectId,
    pub effect: Box<E>,
}

impl<E> WithIdEffect<E> {
    pub fn new(id: EffectId, effect: E) -> Self {
        Self {
            id,
            effect: Box::new(effect),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaggedEffect<E> {
    pub tag: crate::tag::TagKey,
    pub effect: Box<E>,
}

impl<E> TaggedEffect<E> {
    pub fn new(tag: impl Into<crate::tag::TagKey>, effect: E) -> Self {
        Self {
            tag: tag.into(),
            effect: Box::new(effect),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectMode<E> {
    pub description: String,
    pub effects: Vec<E>,
}

impl<E> EffectMode<E> {
    pub fn new(description: impl Into<String>, effects: Vec<E>) -> Self {
        Self {
            description: description.into(),
            effects,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseModeEffect<E> {
    pub modes: Vec<EffectMode<E>>,
    pub min: Value,
    pub max: Value,
    pub allow_repeat: bool,
    pub choose_count: Value,
    pub min_choose_count: Value,
    pub allow_repeated_modes: bool,
    pub disallow_previously_chosen_modes: bool,
    pub disallow_previously_chosen_modes_this_turn: bool,
}

impl<E> ChooseModeEffect<E> {
    pub fn new(modes: Vec<EffectMode<E>>, min: Value, max: Value, allow_repeat: bool) -> Self {
        let choose_count = max.clone();
        let min_choose_count = min.clone();
        Self {
            modes,
            min,
            max,
            allow_repeat,
            choose_count,
            min_choose_count,
            allow_repeated_modes: allow_repeat,
            disallow_previously_chosen_modes: false,
            disallow_previously_chosen_modes_this_turn: false,
        }
    }

    pub fn choose_one(modes: Vec<EffectMode<E>>) -> Self {
        Self::new(modes, Value::Fixed(1), Value::Fixed(1), false)
    }

    pub fn choose_exactly(count: impl Into<Value>, modes: Vec<EffectMode<E>>) -> Self {
        let count = count.into();
        Self::new(modes, count.clone(), count, false)
    }

    pub fn choose_up_to(
        max: impl Into<Value>,
        min: impl Into<Value>,
        modes: Vec<EffectMode<E>>,
    ) -> Self {
        Self::new(modes, min.into(), max.into(), false)
    }

    pub fn with_repeated_modes(mut self) -> Self {
        self.allow_repeat = true;
        self.allow_repeated_modes = true;
        self
    }

    pub fn with_previously_unchosen_modes_only(mut self) -> Self {
        self.disallow_previously_chosen_modes = true;
        self
    }

    pub fn with_previously_unchosen_modes_only_this_turn(mut self) -> Self {
        self.disallow_previously_chosen_modes = true;
        self.disallow_previously_chosen_modes_this_turn = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HauntExileEffect<E> {
    pub haunt_effects: Vec<E>,
    pub haunt_choices: Vec<ChooseSpec>,
}

impl<E> HauntExileEffect<E> {
    pub fn new(haunt_effects: Vec<E>, haunt_choices: Vec<ChooseSpec>) -> Self {
        Self {
            haunt_effects,
            haunt_choices,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchLibrarySlot {
    pub filter: ObjectFilter,
    pub optional: bool,
}

impl SearchLibrarySlot {
    pub fn required(filter: ObjectFilter) -> Self {
        Self {
            filter,
            optional: false,
        }
    }

    pub fn optional(filter: ObjectFilter) -> Self {
        Self {
            filter,
            optional: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchLibraryEffect {
    pub filter: ObjectFilter,
    pub destination: crate::zone::Zone,
    pub chooser: PlayerFilter,
    pub player: PlayerFilter,
    pub reveal: bool,
    pub search_mode: SearchSelectionMode,
    pub library_position_from_top: Option<Value>,
}

impl SearchLibraryEffect {
    pub fn new(
        filter: ObjectFilter,
        destination: crate::zone::Zone,
        chooser: PlayerFilter,
        player: PlayerFilter,
        reveal: bool,
    ) -> Self {
        Self {
            filter,
            destination,
            chooser,
            player,
            reveal,
            search_mode: SearchSelectionMode::Exact,
            library_position_from_top: None,
        }
    }

    pub fn with_search_mode(mut self, search_mode: SearchSelectionMode) -> Self {
        self.search_mode = search_mode;
        self
    }

    pub fn with_library_position_from_top(mut self, position: Value) -> Self {
        self.library_position_from_top = Some(position);
        self
    }

    pub fn to_hand(filter: ObjectFilter, player: PlayerFilter, reveal: bool) -> Self {
        Self::new(
            filter,
            crate::zone::Zone::Hand,
            player.clone(),
            player,
            reveal,
        )
    }

    pub fn to_battlefield(filter: ObjectFilter, player: PlayerFilter, reveal: bool) -> Self {
        Self::new(
            filter,
            crate::zone::Zone::Battlefield,
            player.clone(),
            player,
            reveal,
        )
    }

    pub fn to_library_top(filter: ObjectFilter, player: PlayerFilter, reveal: bool) -> Self {
        Self::new(
            filter,
            crate::zone::Zone::Library,
            player.clone(),
            player,
            reveal,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchLibrarySlotsEffect {
    pub slots: Vec<SearchLibrarySlot>,
    pub destination: crate::zone::Zone,
    pub chooser: PlayerFilter,
    pub player: PlayerFilter,
    pub reveal: bool,
    pub progress_tag: crate::tag::TagKey,
}

impl SearchLibrarySlotsEffect {
    pub fn new(
        slots: Vec<SearchLibrarySlot>,
        destination: crate::zone::Zone,
        chooser: PlayerFilter,
        player: PlayerFilter,
        reveal: bool,
        progress_tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            slots,
            destination,
            chooser,
            player,
            reveal,
            progress_tag: progress_tag.into(),
        }
    }

    pub fn to_hand(
        slots: Vec<SearchLibrarySlot>,
        player: PlayerFilter,
        reveal: bool,
        progress_tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self::new(
            slots,
            crate::zone::Zone::Hand,
            player.clone(),
            player,
            reveal,
            progress_tag,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LookAtHandEffect {
    pub target: ChooseSpec,
    pub reveal: bool,
}

impl LookAtHandEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            reveal: false,
        }
    }

    pub fn reveal(target: ChooseSpec) -> Self {
        Self {
            target,
            reveal: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealTaggedEffect {
    pub tag: crate::tag::TagKey,
}

impl RevealTaggedEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealSourceFromHandEffect;

impl RevealSourceFromHandEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseSpellCastHistoryEffect {
    pub chooser: PlayerFilter,
    pub cast_by: PlayerFilter,
    pub filter: ObjectFilter,
    pub tag: crate::tag::TagKey,
    pub description: String,
}

impl ChooseSpellCastHistoryEffect {
    pub fn new(
        chooser: PlayerFilter,
        cast_by: PlayerFilter,
        filter: ObjectFilter,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            chooser,
            cast_by,
            filter,
            tag: tag.into(),
            description: "Choose one of those spells".to_string(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScryEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl ScryEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurveilEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl SurveilEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FatesealEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl FatesealEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EachPlayerScryEffect {
    pub count: Value,
    pub player_filter: PlayerFilter,
}

impl EachPlayerScryEffect {
    pub fn new(count: impl Into<Value>, player_filter: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player_filter,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrewCostEffect {
    pub required_power: u32,
}

impl CrewCostEffect {
    pub fn new(required_power: u32) -> Self {
        Self { required_power }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MillEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl MillEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleGraveyardIntoLibraryEffect {
    pub player: PlayerFilter,
}

impl ShuffleGraveyardIntoLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleHandAndGraveyardIntoLibraryEffect {
    pub player: PlayerFilter,
}

impl ShuffleHandAndGraveyardIntoLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnToHandEffect {
    pub spec: ChooseSpec,
}

impl ReturnToHandEffect {
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    pub fn creatures() -> Self {
        Self::all(ObjectFilter::creature())
    }

    pub fn nonland_permanents() -> Self {
        Self::all(ObjectFilter::nonland_permanent())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveToLibraryNthFromTopEffect {
    pub target: ChooseSpec,
    pub position: Value,
}

impl MoveToLibraryNthFromTopEffect {
    pub fn new(target: ChooseSpec, position: Value) -> Self {
        Self { target, position }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveToLibraryTopOrBottomChoiceEffect {
    pub target: ChooseSpec,
}

impl MoveToLibraryTopOrBottomChoiceEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeControlEffect {
    pub permanent1: ChooseSpec,
    pub permanent2: ChooseSpec,
    pub shared_type: Option<SharedTypeConstraint>,
    pub permanent1_reference_tag: Option<crate::tag::TagKey>,
}

impl ExchangeControlEffect {
    pub fn new(permanent1: ChooseSpec, permanent2: ChooseSpec) -> Self {
        Self {
            permanent1,
            permanent2,
            shared_type: None,
            permanent1_reference_tag: None,
        }
    }

    pub fn with_shared_type(mut self, constraint: SharedTypeConstraint) -> Self {
        self.shared_type = Some(constraint);
        self
    }

    pub fn with_permanent1_reference_tag(mut self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.permanent1_reference_tag = Some(tag.into());
        self
    }

    pub fn creatures() -> Self {
        Self::new(ChooseSpec::creature(), ChooseSpec::creature())
    }

    pub fn permanents() -> Self {
        Self::new(ChooseSpec::permanent(), ChooseSpec::permanent())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlefieldController {
    Preserve,
    Owner,
    You,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveToZoneEffect {
    pub target: ChooseSpec,
    pub zone: crate::zone::Zone,
    pub to_top: bool,
    pub battlefield_controller: BattlefieldController,
    pub enters_tapped: bool,
    pub transfer_exiled_with_source_links: bool,
}

impl MoveToZoneEffect {
    pub fn new(target: ChooseSpec, zone: crate::zone::Zone, to_top: bool) -> Self {
        Self {
            target,
            zone,
            to_top,
            battlefield_controller: BattlefieldController::Preserve,
            enters_tapped: false,
            transfer_exiled_with_source_links: false,
        }
    }

    pub fn to_top_of_library(target: ChooseSpec) -> Self {
        Self::new(target, crate::zone::Zone::Library, true)
    }

    pub fn to_bottom_of_library(target: ChooseSpec) -> Self {
        Self::new(target, crate::zone::Zone::Library, false)
    }

    pub fn to_exile(target: ChooseSpec) -> Self {
        Self::new(target, crate::zone::Zone::Exile, false)
    }

    pub fn to_graveyard(target: ChooseSpec) -> Self {
        Self::new(target, crate::zone::Zone::Graveyard, false)
    }

    pub fn under_owner_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::Owner;
        self
    }

    pub fn transfer_exiled_with_source_links(mut self) -> Self {
        self.transfer_exiled_with_source_links = true;
        self
    }

    pub fn under_you_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::You;
        self
    }

    pub fn tapped(mut self) -> Self {
        self.enters_tapped = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnAllToBattlefieldEffect {
    pub filter: ObjectFilter,
    pub tapped: bool,
    pub battlefield_controller: BattlefieldController,
}

impl ReturnAllToBattlefieldEffect {
    pub fn new(filter: ObjectFilter, tapped: bool) -> Self {
        Self {
            filter,
            tapped,
            battlefield_controller: BattlefieldController::Owner,
        }
    }

    pub fn under_owner_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::Owner;
        self
    }

    pub fn under_you_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::You;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExecuteWithSourceEffect<E> {
    pub source: ChooseSpec,
    pub effect: Box<E>,
}

impl<E> ExecuteWithSourceEffect<E> {
    pub fn new(source: ChooseSpec, effect: E) -> Self {
        Self {
            source,
            effect: Box::new(effect),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetainManaUntilEndOfTurnEffect {
    pub player: PlayerFilter,
}

impl RetainManaUntilEndOfTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeldEffect {
    pub result_name: String,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
}

impl MeldEffect {
    pub fn new(result_name: impl Into<String>) -> Self {
        Self {
            result_name: result_name.into(),
            enters_tapped: false,
            enters_attacking: false,
        }
    }

    pub fn enters_tapped(mut self, enters_tapped: bool) -> Self {
        self.enters_tapped = enters_tapped;
        self
    }

    pub fn enters_attacking(mut self, enters_attacking: bool) -> Self {
        self.enters_attacking = enters_attacking;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReorderLibraryTopEffect {
    pub tag: crate::tag::TagKey,
}

impl ReorderLibraryTopEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExertCostEffect {
    pub display_text: String,
}

impl ExertCostEffect {
    pub fn new(display_text: impl Into<String>) -> Self {
        Self {
            display_text: display_text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachObject<E> {
    pub filter: ObjectFilter,
    pub effects: Vec<E>,
}

impl<E> ForEachObject<E> {
    pub fn new(filter: ObjectFilter, effects: Vec<E>) -> Self {
        Self { filter, effects }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseObjectsEffect {
    pub filter: ObjectFilter,
    pub count: ChoiceCount,
    pub count_value: Option<Value>,
    pub chooser: PlayerFilter,
    pub zone: Option<crate::zone::Zone>,
    pub additional_zones: Vec<crate::zone::Zone>,
    pub tag: crate::tag::TagKey,
    pub description: String,
    pub is_search: bool,
    pub reveal: bool,
    pub search_mode: SearchSelectionMode,
    pub top_only: bool,
    pub replace_tagged_objects: bool,
}

impl ChooseObjectsEffect {
    pub fn new(
        filter: ObjectFilter,
        count: impl Into<ChoiceCount>,
        chooser: PlayerFilter,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            filter,
            count: count.into(),
            count_value: None,
            chooser,
            zone: None,
            additional_zones: Vec::new(),
            tag: tag.into(),
            description: "Choose".to_string(),
            is_search: false,
            reveal: false,
            search_mode: SearchSelectionMode::Exact,
            top_only: false,
            replace_tagged_objects: false,
        }
    }

    pub fn in_zone(mut self, zone: crate::zone::Zone) -> Self {
        self.zone = Some(zone);
        self.additional_zones.clear();
        self
    }

    pub fn in_zones(mut self, zones: Vec<crate::zone::Zone>) -> Self {
        let mut iter = zones.into_iter();
        if let Some(first) = iter.next() {
            self.zone = Some(first);
            self.additional_zones = iter.collect();
        } else {
            self.zone = None;
            self.additional_zones.clear();
        }
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_count_value(mut self, count_value: Value) -> Self {
        self.count_value = Some(count_value);
        self
    }

    pub fn with_count_value_opt(mut self, count_value: Option<Value>) -> Self {
        self.count_value = count_value;
        self
    }

    pub fn as_search(mut self) -> Self {
        self.is_search = true;
        self.search_mode = SearchSelectionMode::Exact;
        self
    }

    pub fn as_optional_search(mut self) -> Self {
        self.is_search = true;
        self.search_mode = SearchSelectionMode::Optional;
        self
    }

    pub fn as_all_matching_search(mut self) -> Self {
        self.is_search = true;
        self.search_mode = SearchSelectionMode::AllMatching;
        self
    }

    pub fn reveal(mut self) -> Self {
        self.reveal = true;
        self
    }

    pub fn top_only(mut self) -> Self {
        self.top_only = true;
        self
    }

    pub fn replace_tagged_objects(mut self) -> Self {
        self.replace_tagged_objects = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulateEffect {
    pub count: Value,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
    pub has_haste: bool,
    pub sacrifice_at_next_end_step: bool,
    pub exile_at_next_end_step: bool,
    pub exile_at_end_of_combat: bool,
    pub sacrifice_at_end_of_combat: bool,
}

impl PopulateEffect {
    pub fn new(count: impl Into<Value>) -> Self {
        Self {
            count: count.into(),
            enters_tapped: false,
            enters_attacking: false,
            has_haste: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
        }
    }

    pub fn enters_tapped(mut self, value: bool) -> Self {
        self.enters_tapped = value;
        self
    }

    pub fn attacking(mut self, value: bool) -> Self {
        self.enters_attacking = value;
        self
    }

    pub fn haste(mut self, value: bool) -> Self {
        self.has_haste = value;
        self
    }

    pub fn sacrifice_at_next_end_step(mut self, value: bool) -> Self {
        self.sacrifice_at_next_end_step = value;
        self
    }

    pub fn exile_at_next_end_step(mut self, value: bool) -> Self {
        self.exile_at_next_end_step = value;
        self
    }

    pub fn exile_at_end_of_combat(mut self, value: bool) -> Self {
        self.exile_at_end_of_combat = value;
        self
    }

    pub fn sacrifice_at_end_of_combat(mut self, value: bool) -> Self {
        self.sacrifice_at_end_of_combat = value;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BecomeBasicLandTypeChoiceEffect {
    pub target: ChooseSpec,
    pub duration: Until,
    pub chooser: PlayerFilter,
    pub fixed_subtype: Option<crate::types::Subtype>,
}

impl BecomeBasicLandTypeChoiceEffect {
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self {
            target,
            duration,
            chooser: PlayerFilter::You,
            fixed_subtype: None,
        }
    }

    pub fn fixed(target: ChooseSpec, subtype: crate::types::Subtype, duration: Until) -> Self {
        Self {
            target,
            duration,
            chooser: PlayerFilter::You,
            fixed_subtype: Some(subtype),
        }
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BecomeCreatureTypeChoiceEffect {
    pub target: ChooseSpec,
    pub duration: Until,
    pub chooser: PlayerFilter,
    pub excluded_subtypes: Vec<crate::types::Subtype>,
}

impl BecomeCreatureTypeChoiceEffect {
    pub fn new(
        target: ChooseSpec,
        duration: Until,
        excluded_subtypes: Vec<crate::types::Subtype>,
    ) -> Self {
        Self {
            target,
            duration,
            chooser: PlayerFilter::You,
            excluded_subtypes,
        }
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }

    pub fn all_creature_types() -> Vec<crate::types::Subtype> {
        crate::types::Subtype::all_creature_types().to_vec()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BecomeColorChoiceEffect {
    pub target: ChooseSpec,
    pub duration: Until,
    pub chooser: PlayerFilter,
}

impl BecomeColorChoiceEffect {
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self {
            target,
            duration,
            chooser: PlayerFilter::You,
        }
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayManaEffect {
    pub cost: crate::mana::ManaCost,
    pub player: ChooseSpec,
}

impl PayManaEffect {
    pub fn new(cost: crate::mana::ManaCost, player: ChooseSpec) -> Self {
        Self { cost, player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AmassEffect {
    pub subtype: Option<crate::types::Subtype>,
    pub amount: Value,
}

impl AmassEffect {
    pub fn new(subtype: Option<crate::types::Subtype>, amount: impl Into<Value>) -> Self {
        Self {
            subtype,
            amount: amount.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantNextSpellCostReductionEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub reduction: crate::mana::ManaCost,
}

impl GrantNextSpellCostReductionEffect {
    pub fn new(
        player: PlayerFilter,
        filter: ObjectFilter,
        reduction: crate::mana::ManaCost,
    ) -> Self {
        Self {
            player,
            filter,
            reduction,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantAbilitiesTargetEffect<A> {
    pub target: ChooseSpec,
    pub abilities: Vec<A>,
    pub duration: Until,
}

impl<A> GrantAbilitiesTargetEffect<A> {
    pub fn new(
        target: ChooseSpec,
        abilities: impl IntoIterator<Item = A>,
        duration: Until,
    ) -> Self {
        Self {
            target,
            abilities: abilities.into_iter().collect(),
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModifyPowerToughnessEffect {
    pub target: ChooseSpec,
    pub power: Value,
    pub toughness: Value,
    pub duration: Until,
}

impl ModifyPowerToughnessEffect {
    pub fn new(
        target: ChooseSpec,
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        duration: Until,
    ) -> Self {
        Self {
            target,
            power: power.into(),
            toughness: toughness.into(),
            duration,
        }
    }

    pub fn pump(target: ChooseSpec, amount: impl Into<Value>, duration: Until) -> Self {
        let val = amount.into();
        Self::new(target, val.clone(), val, duration)
    }

    pub fn shrink(target: ChooseSpec, amount: i32, duration: Until) -> Self {
        Self::new(target, -amount, -amount, duration)
    }

    pub fn source(power: impl Into<Value>, toughness: impl Into<Value>, duration: Until) -> Self {
        Self::new(ChooseSpec::Source, power, toughness, duration)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FightEffect {
    pub creature1: ChooseSpec,
    pub creature2: ChooseSpec,
}

impl FightEffect {
    pub fn new(creature1: ChooseSpec, creature2: ChooseSpec) -> Self {
        Self {
            creature1,
            creature2,
        }
    }

    pub fn you_vs_opponent() -> Self {
        Self::new(ChooseSpec::creature(), ChooseSpec::creature())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExploreEffect {
    pub target: ChooseSpec,
}

impl ExploreEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestDreadEffect;

impl ManifestDreadEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestTopCardOfLibraryEffect {
    pub player: PlayerFilter,
}

impl ManifestTopCardOfLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SupportEffect {
    pub amount: u32,
}

impl SupportEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AmplifyEffect {
    pub amount: u32,
}

impl AmplifyEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DevourEffect {
    pub multiplier: u32,
}

impl DevourEffect {
    pub fn new(multiplier: u32) -> Self {
        Self { multiplier }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConniveEffect {
    pub target: ChooseSpec,
    pub count: Value,
}

impl ConniveEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self::new_with_count(target, Value::Fixed(1))
    }

    pub fn new_with_count(target: ChooseSpec, count: impl Into<Value>) -> Self {
        Self {
            target,
            count: count.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetainEffect {
    pub target: ChooseSpec,
}

impl DetainEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoadEffect {
    pub target: ChooseSpec,
}

impl GoadEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuspectEffect {
    pub target: ChooseSpec,
}

impl SuspectEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClearSuspectedEffect {
    pub target: Option<ChooseSpec>,
}

impl ClearSuspectedEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target: Some(target),
        }
    }

    pub const fn all() -> Self {
        Self { target: None }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenAttractionEffect;

impl OpenAttractionEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptEffect {
    pub amount: u32,
}

impl AdaptEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackupEffect<A> {
    pub amount: u32,
    pub granted_abilities: Vec<A>,
}

impl<A> BackupEffect<A> {
    pub fn new(amount: u32, granted_abilities: Vec<A>) -> Self {
        Self {
            amount,
            granted_abilities,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BeholdEffect {
    pub subtype: Subtype,
    pub count: u32,
    pub chooser: PlayerFilter,
}

impl BeholdEffect {
    pub fn new(subtype: Subtype, count: u32, chooser: PlayerFilter) -> Self {
        Self {
            subtype,
            count,
            chooser,
        }
    }

    pub fn you(subtype: Subtype, count: u32) -> Self {
        Self::new(subtype, count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClashOpponentMode {
    AnyOpponent,
    TargetOpponent,
    DefendingPlayer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClashEffect {
    pub opponent_mode: ClashOpponentMode,
}

impl ClashEffect {
    pub fn new(opponent_mode: ClashOpponentMode) -> Self {
        Self { opponent_mode }
    }

    pub fn against_any_opponent() -> Self {
        Self::new(ClashOpponentMode::AnyOpponent)
    }

    pub fn against_target_opponent() -> Self {
        Self::new(ClashOpponentMode::TargetOpponent)
    }

    pub fn against_defending_player() -> Self {
        Self::new(ClashOpponentMode::DefendingPlayer)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EarthbendEffect {
    pub target: ChooseSpec,
    pub counters: u32,
}

impl EarthbendEffect {
    pub fn new(target: ChooseSpec, counters: u32) -> Self {
        Self { target, counters }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalRewriteEffect<E> {
    pub effect: Box<E>,
    pub zone_replacements: Vec<RegisterZoneReplacementEffect>,
}

impl<E> LocalRewriteEffect<E> {
    pub fn new(effect: E, zone_replacements: Vec<RegisterZoneReplacementEffect>) -> Self {
        Self {
            effect: Box::new(effect),
            zone_replacements,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaggedLeavesAbilitySource {
    WatchedObject,
    CurrentSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleEffectsWhenTaggedLeavesEffect<E> {
    pub tag: crate::tag::TagKey,
    pub effects: Vec<E>,
    pub controller: PlayerFilter,
    pub ability_source: TaggedLeavesAbilitySource,
}

impl<E> ScheduleEffectsWhenTaggedLeavesEffect<E> {
    pub fn new(
        tag: impl Into<crate::tag::TagKey>,
        effects: Vec<E>,
        controller: PlayerFilter,
    ) -> Self {
        Self {
            tag: tag.into(),
            effects,
            controller,
            ability_source: TaggedLeavesAbilitySource::WatchedObject,
        }
    }

    pub fn with_current_source_as_ability_source(mut self) -> Self {
        self.ability_source = TaggedLeavesAbilitySource::CurrentSource;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateTokenEffect<D> {
    pub token: D,
    pub count: Value,
    pub controller: PlayerFilter,
    pub controller_target: Option<ChooseSpec>,
    pub suppress_aura_attachment_choice: bool,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
    pub exile_at_end_of_combat: bool,
    pub sacrifice_at_end_of_combat: bool,
    pub sacrifice_at_next_end_step: bool,
    pub exile_at_next_end_step: bool,
}

impl<D> CreateTokenEffect<D> {
    pub fn new(token: D, count: impl Into<Value>, controller: PlayerFilter) -> Self {
        let count = count.into().without_surface_hint(ValueSurfaceHint::ForEach);
        let controller_target = match &controller {
            PlayerFilter::Target(filter) => {
                Some(ChooseSpec::target(ChooseSpec::Player((**filter).clone())))
            }
            _ => None,
        };
        Self {
            token,
            count,
            controller,
            controller_target,
            suppress_aura_attachment_choice: false,
            enters_tapped: false,
            enters_attacking: false,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
        }
    }

    pub fn you(token: D, count: impl Into<Value>) -> Self {
        Self::new(token, count, PlayerFilter::You)
    }

    pub fn one(token: D) -> Self {
        Self::you(token, 1)
    }

    pub fn tapped(mut self) -> Self {
        self.enters_tapped = true;
        self
    }

    pub fn attacking(mut self) -> Self {
        self.enters_attacking = true;
        self
    }

    pub fn suppress_aura_attachment_choice(mut self) -> Self {
        self.suppress_aura_attachment_choice = true;
        self
    }

    pub fn exile_at_end_of_combat(mut self) -> Self {
        self.exile_at_end_of_combat = true;
        self
    }

    pub fn sacrifice_at_end_of_combat(mut self) -> Self {
        self.sacrifice_at_end_of_combat = true;
        self
    }

    pub fn sacrifice_at_next_end_step(mut self) -> Self {
        self.sacrifice_at_next_end_step = true;
        self
    }

    pub fn exile_at_next_end_step(mut self) -> Self {
        self.exile_at_next_end_step = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncubateEffect {
    pub amount: Value,
    pub count: Value,
    pub controller: PlayerFilter,
    pub controller_target: Option<ChooseSpec>,
}

impl IncubateEffect {
    pub fn new(
        amount: impl Into<Value>,
        count: impl Into<Value>,
        controller: PlayerFilter,
    ) -> Self {
        let controller_target = match &controller {
            PlayerFilter::Target(filter) => {
                Some(ChooseSpec::target(ChooseSpec::Player((**filter).clone())))
            }
            _ => None,
        };
        Self {
            amount: amount.into(),
            count: count.into().without_surface_hint(ValueSurfaceHint::ForEach),
            controller,
            controller_target,
        }
    }

    pub fn you(amount: impl Into<Value>, count: impl Into<Value>) -> Self {
        Self::new(amount, count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CopyPtAdjustment {
    HalfRoundUp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CopyAttackTargetMode {
    PlayerOrPlaneswalkerControlledBy(PlayerFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateTokenCopyEffect<A> {
    pub target: ChooseSpec,
    pub count: Value,
    pub controller: PlayerFilter,
    pub enters_tapped: bool,
    pub has_haste: bool,
    pub enters_attacking: bool,
    pub attack_target_mode: Option<CopyAttackTargetMode>,
    pub exile_at_end_of_combat: bool,
    pub sacrifice_at_next_end_step: bool,
    pub exile_at_next_end_step: bool,
    pub pt_adjustment: Option<CopyPtAdjustment>,
    pub clear_mana_cost: bool,
    pub added_card_types: Vec<CardType>,
    pub added_subtypes: Vec<Subtype>,
    pub removed_supertypes: Vec<Supertype>,
    pub set_base_power_toughness: Option<(i32, i32)>,
    pub set_colors: Option<ColorSet>,
    pub set_card_types: Option<Vec<CardType>>,
    pub set_subtypes: Option<Vec<Subtype>>,
    pub granted_static_abilities: Vec<A>,
}

impl<A> CreateTokenCopyEffect<A> {
    pub fn new(target: ChooseSpec, count: impl Into<Value>, controller: PlayerFilter) -> Self {
        Self {
            target,
            count: count.into(),
            controller,
            enters_tapped: false,
            has_haste: false,
            enters_attacking: false,
            attack_target_mode: None,
            exile_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            pt_adjustment: None,
            clear_mana_cost: false,
            added_card_types: Vec::new(),
            added_subtypes: Vec::new(),
            removed_supertypes: Vec::new(),
            set_base_power_toughness: None,
            set_colors: None,
            set_card_types: None,
            set_subtypes: None,
            granted_static_abilities: Vec::new(),
        }
    }

    pub fn one(target: ChooseSpec) -> Self {
        Self::new(target, 1, PlayerFilter::You)
    }

    pub fn with_haste(target: ChooseSpec) -> Self {
        let mut effect = Self::one(target);
        effect.has_haste = true;
        effect
    }

    pub fn tapped(target: ChooseSpec) -> Self {
        let mut effect = Self::one(target);
        effect.enters_tapped = true;
        effect
    }

    pub fn kiki_jiki_style(target: ChooseSpec) -> Self {
        let mut effect = Self::one(target);
        effect.has_haste = true;
        effect.exile_at_end_of_combat = true;
        effect
    }

    pub fn enters_tapped(mut self, value: bool) -> Self {
        self.enters_tapped = value;
        self
    }

    pub fn haste(mut self, value: bool) -> Self {
        self.has_haste = value;
        self
    }

    pub fn attacking(mut self, value: bool) -> Self {
        self.enters_attacking = value;
        if !value {
            self.attack_target_mode = None;
        }
        self
    }

    pub fn attack_target_mode(mut self, mode: CopyAttackTargetMode) -> Self {
        self.enters_attacking = true;
        self.attack_target_mode = Some(mode);
        self
    }

    pub fn attacking_player_or_planeswalker_controlled_by(mut self, player: PlayerFilter) -> Self {
        self.enters_attacking = true;
        self.attack_target_mode = Some(CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
            player,
        ));
        self
    }

    pub fn exile_at_eoc(mut self, value: bool) -> Self {
        self.exile_at_end_of_combat = value;
        self
    }

    pub fn sacrifice_at_next_end_step(mut self, value: bool) -> Self {
        self.sacrifice_at_next_end_step = value;
        self
    }

    pub fn exile_at_next_end_step(mut self, value: bool) -> Self {
        self.exile_at_next_end_step = value;
        self
    }

    pub fn half_power_toughness_round_up(mut self) -> Self {
        self.pt_adjustment = Some(CopyPtAdjustment::HalfRoundUp);
        self
    }

    pub fn without_mana_cost(mut self) -> Self {
        self.clear_mana_cost = true;
        self
    }

    pub fn added_card_type(mut self, card_type: CardType) -> Self {
        if !self.added_card_types.contains(&card_type) {
            self.added_card_types.push(card_type);
        }
        self
    }

    pub fn added_subtype(mut self, subtype: Subtype) -> Self {
        if !self.added_subtypes.contains(&subtype) {
            self.added_subtypes.push(subtype);
        }
        self
    }

    pub fn removed_supertype(mut self, supertype: Supertype) -> Self {
        if !self.removed_supertypes.contains(&supertype) {
            self.removed_supertypes.push(supertype);
        }
        self
    }

    pub fn set_base_power_toughness(mut self, power: i32, toughness: i32) -> Self {
        self.set_base_power_toughness = Some((power, toughness));
        self
    }

    pub fn set_colors(mut self, colors: ColorSet) -> Self {
        self.set_colors = Some(colors);
        self
    }

    pub fn set_card_types(mut self, card_types: Vec<CardType>) -> Self {
        self.set_card_types = Some(card_types);
        self
    }

    pub fn set_subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.set_subtypes = Some(subtypes);
        self
    }

    pub fn grant_static_ability(mut self, ability: A) -> Self {
        self.granted_static_abilities.push(ability);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantNextSpellAbilityEffect<A> {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub ability: A,
}

impl<A> GrantNextSpellAbilityEffect<A> {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, ability: A) -> Self {
        Self {
            player,
            filter,
            ability,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetargetStackObjectEffect {
    pub target: ChooseSpec,
    pub mode: RetargetMode,
    pub chooser: PlayerFilter,
    pub require_change: bool,
    pub new_target_restriction: Option<NewTargetRestriction>,
}

impl RetargetStackObjectEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            mode: RetargetMode::All,
            chooser: PlayerFilter::You,
            require_change: false,
            new_target_restriction: None,
        }
    }

    pub fn with_mode(mut self, mode: RetargetMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }

    pub fn require_change(mut self) -> Self {
        self.require_change = true;
        self
    }

    pub fn with_restriction(mut self, restriction: NewTargetRestriction) -> Self {
        self.new_target_restriction = Some(restriction);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileEffect {
    pub spec: ChooseSpec,
    pub face_down: bool,
}

impl ExileEffect {
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self {
            spec,
            face_down: false,
        }
    }

    pub fn with_face_down(mut self, face_down: bool) -> Self {
        self.face_down = face_down;
        self
    }

    pub fn target(spec: ChooseSpec) -> Self {
        Self::with_spec(ChooseSpec::target(spec))
    }

    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self::with_spec(ChooseSpec::target(spec).with_count(count))
    }

    pub fn all(filter: ObjectFilter) -> Self {
        Self::with_spec(ChooseSpec::all(filter))
    }

    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    pub fn any_number(target: ChooseSpec) -> Self {
        Self::targets(target, ChoiceCount::any_number())
    }

    pub fn specific(object_id: crate::ids::ObjectId) -> Self {
        Self::with_spec(ChooseSpec::SpecificObject(object_id))
    }

    pub fn creatures() -> Self {
        Self::all(ObjectFilter::creature())
    }

    pub fn nonland_permanents() -> Self {
        Self::all(ObjectFilter::nonland_permanent())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagMatchingObjectsEffect {
    pub filter: ObjectFilter,
    pub zone: Option<crate::zone::Zone>,
    pub additional_zones: Vec<crate::zone::Zone>,
    pub tag: crate::tag::TagKey,
}

impl TagMatchingObjectsEffect {
    pub fn new(filter: ObjectFilter, tag: impl Into<crate::tag::TagKey>) -> Self {
        Self {
            filter,
            zone: None,
            additional_zones: Vec::new(),
            tag: tag.into(),
        }
    }

    pub fn in_zone(mut self, zone: crate::zone::Zone) -> Self {
        self.zone = Some(zone);
        self.additional_zones.clear();
        self
    }

    pub fn in_zones(mut self, zones: Vec<crate::zone::Zone>) -> Self {
        let mut iter = zones.into_iter();
        if let Some(first) = iter.next() {
            self.zone = Some(first);
            self.additional_zones = iter.collect();
        } else {
            self.zone = None;
            self.additional_zones.clear();
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SacrificeTargetEffect {
    pub target: ChooseSpec,
}

impl SacrificeTargetEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileTaggedWhenSourceLeavesEffect {
    pub tag: crate::tag::TagKey,
    pub controller: PlayerFilter,
}

impl ExileTaggedWhenSourceLeavesEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>, controller: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            controller,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileUntilDuration {
    SourceLeavesBattlefield,
    NextEndStep,
    EndOfCombat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileUntilEffect {
    pub spec: ChooseSpec,
    pub duration: ExileUntilDuration,
    pub return_zone: crate::zone::Zone,
    pub face_down: bool,
}

impl ExileUntilEffect {
    pub fn new(spec: ChooseSpec, duration: ExileUntilDuration) -> Self {
        Self {
            spec,
            duration,
            return_zone: crate::zone::Zone::Battlefield,
            face_down: false,
        }
    }

    pub fn with_face_down(mut self, face_down: bool) -> Self {
        self.face_down = face_down;
        self
    }

    pub fn source_leaves(spec: ChooseSpec) -> Self {
        Self::new(spec, ExileUntilDuration::SourceLeavesBattlefield)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopySpellEffect {
    pub target: ChooseSpec,
    pub count: Value,
    pub copier: PlayerFilter,
    pub removed_supertypes: Vec<Supertype>,
}

impl CopySpellEffect {
    pub fn new(target: ChooseSpec, count: impl Into<Value>) -> Self {
        Self {
            target,
            count: count.into(),
            copier: PlayerFilter::You,
            removed_supertypes: Vec::new(),
        }
    }

    pub fn new_for_player(
        target: ChooseSpec,
        count: impl Into<Value>,
        copier: PlayerFilter,
    ) -> Self {
        Self {
            target,
            count: count.into(),
            copier,
            removed_supertypes: Vec::new(),
        }
    }

    pub fn single(target: ChooseSpec) -> Self {
        Self::new(target, 1)
    }

    pub fn removed_supertype(mut self, supertype: Supertype) -> Self {
        if !self.removed_supertypes.contains(&supertype) {
            self.removed_supertypes.push(supertype);
        }
        self
    }

    pub fn with_removed_supertypes(mut self, supertypes: Vec<Supertype>) -> Self {
        for supertype in supertypes {
            self = self.removed_supertype(supertype);
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopySpellForEachTargetEffect {
    pub target: ChooseSpec,
    pub object_filter: Option<ObjectFilter>,
    pub player_filter: Option<PlayerFilter>,
    pub copier: PlayerFilter,
    pub exclude_current_targets: bool,
    pub removed_supertypes: Vec<Supertype>,
}

impl CopySpellForEachTargetEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            object_filter: None,
            player_filter: None,
            copier: PlayerFilter::You,
            exclude_current_targets: false,
            removed_supertypes: Vec::new(),
        }
    }

    pub fn with_object_filter(mut self, filter: ObjectFilter) -> Self {
        self.object_filter = Some(filter);
        self
    }

    pub fn with_player_filter(mut self, filter: PlayerFilter) -> Self {
        self.player_filter = Some(filter);
        self
    }

    pub fn with_copier(mut self, copier: PlayerFilter) -> Self {
        self.copier = copier;
        self
    }

    pub fn exclude_current_targets(mut self, exclude: bool) -> Self {
        self.exclude_current_targets = exclude;
        self
    }

    pub fn removed_supertype(mut self, supertype: Supertype) -> Self {
        if !self.removed_supertypes.contains(&supertype) {
            self.removed_supertypes.push(supertype);
        }
        self
    }

    pub fn with_removed_supertypes(mut self, supertypes: Vec<Supertype>) -> Self {
        for supertype in supertypes {
            self = self.removed_supertype(supertype);
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableCasualtyPlaneswalkerCopyEffect;

impl VariableCasualtyPlaneswalkerCopyEffect {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VariableCasualtyPlaneswalkerCopyEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoteOption<E> {
    pub name: String,
    pub effects_per_vote: Vec<E>,
}

impl<E> VoteOption<E> {
    pub fn new(name: impl Into<String>, effects_per_vote: Vec<E>) -> Self {
        Self {
            name: name.into(),
            effects_per_vote,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VoteChoice<E> {
    NamedOptions(Vec<VoteOption<E>>),
    Objects {
        filter: ObjectFilter,
        count: ChoiceCount,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoteEffect<E> {
    pub choice: VoteChoice<E>,
    pub controller_extra_votes: u32,
    pub controller_optional_extra_votes: u32,
    pub secret: bool,
}

impl<E> VoteEffect<E> {
    pub fn new(options: Vec<VoteOption<E>>, controller_extra_votes: u32) -> Self {
        Self {
            choice: VoteChoice::NamedOptions(options),
            controller_extra_votes,
            controller_optional_extra_votes: 0,
            secret: false,
        }
    }

    pub fn with_optional_extra(
        options: Vec<VoteOption<E>>,
        controller_extra_votes: u32,
        controller_optional_extra_votes: u32,
    ) -> Self {
        Self {
            choice: VoteChoice::NamedOptions(options),
            controller_extra_votes,
            controller_optional_extra_votes,
            secret: false,
        }
    }

    pub fn named(
        options: Vec<VoteOption<E>>,
        controller_extra_votes: u32,
        controller_optional_extra_votes: u32,
    ) -> Self {
        Self::with_optional_extra(
            options,
            controller_extra_votes,
            controller_optional_extra_votes,
        )
    }

    pub fn vote_objects(
        filter: ObjectFilter,
        count: ChoiceCount,
        controller_extra_votes: u32,
    ) -> Self {
        Self {
            choice: VoteChoice::Objects { filter, count },
            controller_extra_votes,
            controller_optional_extra_votes: 0,
            secret: false,
        }
    }

    pub fn objects(
        filter: ObjectFilter,
        count: ChoiceCount,
        controller_extra_votes: u32,
        controller_optional_extra_votes: u32,
    ) -> Self {
        Self::vote_objects_with_optional_extra(
            filter,
            count,
            controller_extra_votes,
            controller_optional_extra_votes,
        )
    }

    pub fn vote_objects_with_optional_extra(
        filter: ObjectFilter,
        count: ChoiceCount,
        controller_extra_votes: u32,
        controller_optional_extra_votes: u32,
    ) -> Self {
        Self {
            choice: VoteChoice::Objects { filter, count },
            controller_extra_votes,
            controller_optional_extra_votes,
            secret: false,
        }
    }

    pub fn basic(options: Vec<VoteOption<E>>) -> Self {
        Self::new(options, 0)
    }

    pub fn councils_dilemma(options: Vec<VoteOption<E>>) -> Self {
        Self::with_optional_extra(options, 0, 1)
    }

    pub fn with_secret(mut self, secret: bool) -> Self {
        self.secret = secret;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantTaggedSpellFreeCastUntilEndOfTurnEffect {
    pub tag: crate::tag::TagKey,
    pub player: PlayerFilter,
}

impl GrantTaggedSpellFreeCastUntilEndOfTurnEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>, player: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantTaggedSpellLifeCostByManaValueEffect {
    pub tag: crate::tag::TagKey,
    pub player: PlayerFilter,
}

impl GrantTaggedSpellLifeCostByManaValueEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>, player: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MayCastMatchingSpellWithoutPayingManaCostEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub zone: crate::Zone,
}

impl MayCastMatchingSpellWithoutPayingManaCostEffect {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, zone: crate::Zone) -> Self {
        Self {
            player,
            filter,
            zone,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfChosenColorEffect {
    pub amount: Value,
    pub player: PlayerFilter,
    pub fixed_option: Option<crate::color::Color>,
}

impl AddManaOfChosenColorEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
            fixed_option: None,
        }
    }

    pub fn with_fixed_option(
        amount: impl Into<Value>,
        player: PlayerFilter,
        fixed: crate::color::Color,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            fixed_option: Some(fixed),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfImprintedColorsEffect;

impl AddManaOfImprintedColorsEffect {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AddManaOfImprintedColorsEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfColorsAmongEffect {
    pub filter: ObjectFilter,
    pub player: PlayerFilter,
}

impl AddManaOfColorsAmongEffect {
    pub fn new(filter: ObjectFilter, player: PlayerFilter) -> Self {
        Self { filter, player }
    }

    pub fn you(filter: ObjectFilter) -> Self {
        Self::new(filter, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddScaledManaEffect {
    pub mana: Vec<crate::mana::ManaSymbol>,
    pub amount: Value,
    pub player: PlayerFilter,
}

impl AddScaledManaEffect {
    pub fn new(mana: Vec<crate::mana::ManaSymbol>, amount: Value, player: PlayerFilter) -> Self {
        Self {
            mana,
            amount: amount.into_unhinted(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayEnergyEffect {
    pub amount: Value,
    pub player: ChooseSpec,
}

impl PayEnergyEffect {
    pub fn new(amount: impl Into<Value>, player: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayAnyEnergyEffect {
    pub player: ChooseSpec,
}

impl PayAnyEnergyEffect {
    pub fn new(player: ChooseSpec) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConsultTopOfLibraryStopRule {
    FirstMatch,
    MatchCount(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestroyEffect {
    pub spec: ChooseSpec,
    pub target: ChooseSpec,
    pub no_regen: bool,
}

impl DestroyEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self {
            spec: target.clone(),
            target,
            no_regen: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestroyNoRegenerationEffect {
    pub filter: Option<ObjectFilter>,
    pub target: Option<ChooseSpec>,
}

impl DestroyNoRegenerationEffect {
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            filter: Some(filter),
            target: None,
        }
    }

    pub fn with_spec(target: ChooseSpec) -> Self {
        Self {
            filter: None,
            target: Some(target),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SacrificeEffect {
    pub filter: ObjectFilter,
    pub count: i32,
    pub event_object_tags: Vec<TagKey>,
    pub event_source_tags: Vec<TagKey>,
}

impl SacrificeEffect {
    pub fn with_event_object_tag(mut self, tag: impl Into<TagKey>) -> Self {
        self.event_object_tags.push(tag.into());
        self
    }

    pub fn with_event_source_tag(mut self, tag: impl Into<TagKey>) -> Self {
        self.event_source_tags.push(tag.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscardEffect {
    pub count: Value,
    pub player: PlayerFilter,
    pub random: bool,
    pub any_number: bool,
    pub card_filter: Option<ObjectFilter>,
    pub tag: Option<crate::tag::TagKey>,
}

impl DiscardEffect {
    pub fn new_with_filter(
        count: impl Into<Value>,
        player: PlayerFilter,
        random: bool,
        card_filter: Option<ObjectFilter>,
    ) -> Self {
        Self {
            count: count.into(),
            player,
            random,
            any_number: false,
            card_filter,
            tag: None,
        }
    }

    pub fn with_any_number(mut self, any_number: bool) -> Self {
        self.any_number = any_number;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.tag = Some(tag.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAnyCountersFromSourceEffect {
    pub counter_type: Option<crate::counter::CounterType>,
    pub display_x: bool,
    pub remove_all: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DealDistributedDamageEffect {
    pub amount: Value,
    pub target: ChooseSpec,
}

impl DealDistributedDamageEffect {
    pub fn new(amount: Value, target: ChooseSpec) -> Self {
        Self { amount, target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachCounterKindPutOrRemoveEffect {
    pub target: ChooseSpec,
}

impl ForEachCounterKindPutOrRemoveEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseOutEffect {
    pub target: ChooseSpec,
}

impl PhaseOutEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseInEffect {
    pub target: ChooseSpec,
}

impl PhaseInEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveFromCombatEffect {
    pub target: ChooseSpec,
}

impl RemoveFromCombatEffect {
    pub fn with_spec(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawForEachTaggedMatchingEffect {
    pub tag: crate::tag::TagKey,
    pub filter: ObjectFilter,
    pub player: PlayerFilter,
}

impl DrawForEachTaggedMatchingEffect {
    pub fn new(player: PlayerFilter, tag: crate::tag::TagKey, filter: ObjectFilter) -> Self {
        Self {
            tag,
            filter,
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileTopOfLibraryEffect {
    pub count: Value,
    pub player: PlayerFilter,
    pub moved_tags: Vec<crate::tag::TagKey>,
    pub accumulated_tags: Vec<crate::tag::TagKey>,
}

impl ExileTopOfLibraryEffect {
    pub fn new(count: Value, player: PlayerFilter) -> Self {
        Self {
            count,
            player,
            moved_tags: Vec::new(),
            accumulated_tags: Vec::new(),
        }
    }

    pub fn tag_moved(mut self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.moved_tags.push(tag.into());
        self
    }

    pub fn append_tagged(mut self, tag: impl Into<crate::tag::TagKey>) -> Self {
        self.accumulated_tags.push(tag.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamageEffect<E> {
    pub amount: Value,
    pub target: ChooseSpec,
    pub until: Until,
    pub follow_up_effects: Vec<E>,
}

impl<E> PreventDamageEffect<E> {
    pub fn new(amount: Value, target: ChooseSpec, until: Until) -> Self {
        Self {
            amount,
            target,
            until,
            follow_up_effects: Vec::new(),
        }
    }

    pub fn with_follow_up_effects(mut self, effects: Vec<E>) -> Self {
        self.follow_up_effects = effects;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllDamageToTargetEffect<E> {
    pub target: ChooseSpec,
    pub until: Until,
    pub follow_up_effects: Vec<E>,
}

impl<E> PreventAllDamageToTargetEffect<E> {
    pub fn new(target: ChooseSpec, until: Until) -> Self {
        Self {
            target,
            until,
            follow_up_effects: Vec::new(),
        }
    }

    pub fn with_follow_up_effects(mut self, effects: Vec<E>) -> Self {
        self.follow_up_effects = effects;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventNextTimeDamageEffect {
    pub source: PreventNextTimeDamageSource,
    pub target: PreventNextTimeDamageTarget,
    pub reflect_damage_to_source_controller: bool,
}

impl PreventNextTimeDamageEffect {
    pub fn new(source: PreventNextTimeDamageSource, target: PreventNextTimeDamageTarget) -> Self {
        Self {
            source,
            target,
            reflect_damage_to_source_controller: false,
        }
    }

    pub fn reflecting_to_source_controller(mut self) -> Self {
        self.reflect_damage_to_source_controller = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedirectNextDamageToTargetEffect {
    pub target: ChooseSpec,
    pub amount: Option<Value>,
}

impl RedirectNextDamageToTargetEffect {
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            target,
            amount: Some(amount.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RedirectNextTimeDamageToSourceEffect {
    pub source: RedirectNextTimeDamageSource,
    pub target: Option<ChooseSpec>,
}

impl RedirectNextTimeDamageToSourceEffect {
    pub fn new(source: RedirectNextTimeDamageSource, target: ChooseSpec) -> Self {
        Self {
            source,
            target: Some(target),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantPlayTaggedEffect {
    pub tag: crate::tag::TagKey,
    pub player: PlayerFilter,
    pub duration: GrantPlayTaggedDuration,
    pub allow_land: bool,
    pub allow_any_color_for_cast: bool,
}

impl GrantPlayTaggedEffect {
    pub fn new(
        tag: crate::tag::TagKey,
        player: PlayerFilter,
        duration: GrantPlayTaggedDuration,
        allow_land: bool,
        allow_any_color_for_cast: bool,
    ) -> Self {
        Self {
            tag,
            player,
            duration,
            allow_land,
            allow_any_color_for_cast,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterZoneReplacementEffect {
    pub target: ChooseSpec,
    pub from_zone: Option<crate::zone::Zone>,
    pub to_zone: Option<crate::zone::Zone>,
    pub replacement_zone: crate::zone::Zone,
    pub mode: ReplacementApplyMode,
    pub optional: bool,
    pub choice_description: Option<String>,
}

impl RegisterZoneReplacementEffect {
    pub fn new(
        target: ChooseSpec,
        from_zone: Option<crate::zone::Zone>,
        to_zone: Option<crate::zone::Zone>,
        replacement_zone: crate::zone::Zone,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            target,
            from_zone,
            to_zone,
            replacement_zone,
            mode,
            optional: false,
            choice_description: None,
        }
    }

    pub fn optional(mut self, description: impl Into<String>) -> Self {
        self.optional = true;
        self.choice_description = Some(description.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterFutureZoneReplacementEffect {
    pub filter: crate::filter_model::ObjectFilter,
    pub from_zone: Option<crate::zone::Zone>,
    pub to_zone: Option<crate::zone::Zone>,
    pub replacement_zone: crate::zone::Zone,
    pub mode: ReplacementApplyMode,
    pub cause_filter: Option<crate::cause_model::CauseFilter>,
    pub require_cause_source_match: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterDamagedBySourceZoneReplacementEffect {
    pub filter: crate::filter_model::ObjectFilter,
    pub from_zone: Option<crate::zone::Zone>,
    pub to_zone: Option<crate::zone::Zone>,
    pub replacement_zone: crate::zone::Zone,
    pub mode: ReplacementApplyMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterEnterUnderControlReplacementEffect {
    pub filter: crate::filter_model::ObjectFilter,
    pub mode: ReplacementApplyMode,
}

impl RegisterEnterUnderControlReplacementEffect {
    pub fn new(filter: crate::filter_model::ObjectFilter, mode: ReplacementApplyMode) -> Self {
        Self { filter, mode }
    }
}

impl RegisterFutureZoneReplacementEffect {
    pub fn new(
        filter: crate::filter_model::ObjectFilter,
        from_zone: Option<crate::zone::Zone>,
        to_zone: Option<crate::zone::Zone>,
        replacement_zone: crate::zone::Zone,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            filter,
            from_zone,
            to_zone,
            replacement_zone,
            mode,
            cause_filter: None,
            require_cause_source_match: false,
        }
    }

    pub fn with_cause_filter(mut self, cause_filter: crate::cause_model::CauseFilter) -> Self {
        self.cause_filter = Some(cause_filter);
        self
    }

    pub fn requiring_cause_source_match(mut self) -> Self {
        self.require_cause_source_match = true;
        self
    }
}

impl RegisterDamagedBySourceZoneReplacementEffect {
    pub fn new(
        filter: crate::filter_model::ObjectFilter,
        from_zone: Option<crate::zone::Zone>,
        to_zone: Option<crate::zone::Zone>,
        replacement_zone: crate::zone::Zone,
        mode: ReplacementApplyMode,
    ) -> Self {
        Self {
            filter,
            from_zone,
            to_zone,
            replacement_zone,
            mode,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnEffect;

impl LearnEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvestigateEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl InvestigateEffect {
    pub fn new(count: Value, player: PlayerFilter) -> Self {
        Self { count, player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GainLifeEffect {
    pub amount: Value,
    pub player: ChooseSpec,
}

impl GainLifeEffect {
    pub fn new(amount: impl Into<Value>, player: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn with_filter(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self::new(amount, ChooseSpec::Player(player))
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::with_filter(amount, PlayerFilter::You)
    }

    pub fn target_player(amount: impl Into<Value>) -> Self {
        Self::new(amount, ChooseSpec::target_player())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncreaseSpeedEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl IncreaseSpeedEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReduceSpeedEffect {
    pub amount: Value,
    pub player: PlayerFilter,
    pub minimum: u8,
}

impl ReduceSpeedEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter, minimum: u8) -> Self {
        Self {
            amount: amount.into(),
            player,
            minimum,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CipherEffect;

impl CipherEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnearthEffect;

impl UnearthEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NinjutsuCostEffect;

impl NinjutsuCostEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConspireCostEffect;

impl ConspireCostEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NinjutsuEffect;

impl NinjutsuEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegenerateEffect {
    pub target: ChooseSpec,
    pub duration: Until,
}

impl RegenerateEffect {
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self { target, duration }
    }

    pub fn source(duration: Until) -> Self {
        Self::new(ChooseSpec::Source, duration)
    }

    pub fn target_creature(duration: Until) -> Self {
        Self::new(ChooseSpec::creature(), duration)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaFromCommanderColorIdentityEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl AddManaFromCommanderColorIdentityEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfAnyColorEffect {
    pub amount: Value,
    pub player: PlayerFilter,
    pub available_colors: Option<Vec<Color>>,
}

impl AddManaOfAnyColorEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
            available_colors: None,
        }
    }

    pub fn restricted(
        amount: impl Into<Value>,
        player: PlayerFilter,
        available_colors: Vec<Color>,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            available_colors: Some(available_colors),
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }

    pub fn you_restricted(amount: impl Into<Value>, available_colors: Vec<Color>) -> Self {
        Self::restricted(amount, PlayerFilter::You, available_colors)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfAnyOneColorEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl AddManaOfAnyOneColorEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaEffect {
    pub mana: Vec<ManaSymbol>,
    pub player: PlayerFilter,
}

impl AddManaEffect {
    pub fn new(mana: Vec<ManaSymbol>, player: PlayerFilter) -> Self {
        Self { mana, player }
    }

    pub fn you(mana: Vec<ManaSymbol>) -> Self {
        Self::new(mana, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenownEffect {
    pub amount: u32,
}

impl RenownEffect {
    pub const fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BolsterEffect {
    pub amount: u32,
}

impl BolsterEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlipEffect {
    pub target: ChooseSpec,
}

impl FlipEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoseTheGameEffect {
    pub player: PlayerFilter,
}

impl LoseTheGameEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseColorEffect {
    pub chooser: PlayerFilter,
}

impl ChooseColorEffect {
    pub fn new(chooser: PlayerFilter) -> Self {
        Self { chooser }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCreatureTypeEffect {
    pub chooser: PlayerFilter,
    pub excluded_subtypes: Vec<Subtype>,
}

impl ChooseCreatureTypeEffect {
    pub fn new(chooser: PlayerFilter, excluded_subtypes: Vec<Subtype>) -> Self {
        Self {
            chooser,
            excluded_subtypes,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlipCoinEffect {
    pub player: PlayerFilter,
}

impl FlipCoinEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetLifeTotalEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl SetLifeTotalEffect {
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeLifeTotalsEffect {
    pub player1: PlayerFilter,
    pub player2: PlayerFilter,
}

impl ExchangeLifeTotalsEffect {
    pub fn new(player1: PlayerFilter, player2: PlayerFilter) -> Self {
        Self { player1, player2 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoubleManaPoolEffect {
    pub player: PlayerFilter,
}

impl DoubleManaPoolEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmptyManaPoolEffect {
    pub player: PlayerFilter,
}

impl EmptyManaPoolEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipTurnEffect {
    pub player: PlayerFilter,
}

impl SkipTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipDrawStepEffect {
    pub player: PlayerFilter,
}

impl SkipDrawStepEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipNextCombatPhaseThisTurnEffect {
    pub player: PlayerFilter,
}

impl SkipNextCombatPhaseThisTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdditionalLandPlaysEffect {
    pub count: Value,
    pub player: PlayerFilter,
    pub duration: Until,
}

impl AdditionalLandPlaysEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter, duration: Until) -> Self {
        Self {
            count: count.into(),
            player,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BecomeMonarchEffect {
    pub player: PlayerFilter,
}

impl BecomeMonarchEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RingTemptsYouEffect {
    pub player: PlayerFilter,
}

impl RingTemptsYouEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VentureIntoDungeonEffect {
    pub player: PlayerFilter,
    pub undercity_if_no_active: bool,
}

impl VentureIntoDungeonEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            undercity_if_no_active: false,
        }
    }

    pub fn via_initiative(player: PlayerFilter) -> Self {
        Self {
            player,
            undercity_if_no_active: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakeInitiativeEffect {
    pub player: PlayerFilter,
}

impl TakeInitiativeEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PoisonCountersEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl PoisonCountersEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlCombatChoicesThisTurnEffect {
    pub attackers: bool,
    pub blockers: bool,
}

impl ControlCombatChoicesThisTurnEffect {
    pub fn new(attackers: bool, blockers: bool) -> Self {
        Self {
            attackers,
            blockers,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RollDieEffect {
    pub player: PlayerFilter,
    pub sides: u32,
}

impl RollDieEffect {
    pub fn new(player: PlayerFilter, sides: u32) -> Self {
        Self { player, sides }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmitGiftGivenEffect {
    pub recipient: PlayerFilter,
}

impl EmitGiftGivenEffect {
    pub fn new(recipient: PlayerFilter) -> Self {
        Self { recipient }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNamedOptionEffect {
    pub chooser: PlayerFilter,
    pub options: Vec<String>,
}

impl ChooseNamedOptionEffect {
    pub fn new(chooser: PlayerFilter, options: Vec<String>) -> Self {
        Self { chooser, options }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SkipCombatPhasesEffect {
    pub player: PlayerFilter,
}

impl SkipCombatPhasesEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeZonesEffect {
    pub player: PlayerFilter,
    pub zone1: crate::zone::Zone,
    pub zone2: crate::zone::Zone,
}

impl ExchangeZonesEffect {
    pub fn new(player: PlayerFilter, zone1: crate::zone::Zone, zone2: crate::zone::Zone) -> Self {
        Self {
            player,
            zone1,
            zone2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuraSwapEffect;

impl AuraSwapEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeValuesEffect {
    pub left: ExchangeValueOperand,
    pub right: ExchangeValueOperand,
    pub duration: Until,
}

impl ExchangeValuesEffect {
    pub fn new(left: ExchangeValueOperand, right: ExchangeValueOperand, duration: Until) -> Self {
        Self {
            left,
            right,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfLandProducedTypesEffect {
    pub amount: Value,
    pub player: PlayerFilter,
    pub land_filter: ObjectFilter,
    pub allow_colorless: bool,
    pub same_type: bool,
}

impl AddManaOfLandProducedTypesEffect {
    pub fn new(
        amount: impl Into<Value>,
        player: PlayerFilter,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            land_filter,
            allow_colorless,
            same_type,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringDamageTargetEffect {
    pub tag: TagKey,
}

impl TagTriggeringDamageTargetEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConvertEffect {
    pub target: ChooseSpec,
}

impl ConvertEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutStickerEffect {
    pub target: ChooseSpec,
    pub action: crate::event_model::KeywordActionKind,
}

impl PutStickerEffect {
    pub fn new(target: ChooseSpec, action: crate::event_model::KeywordActionKind) -> Self {
        Self { target, action }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReorderGraveyardEffect {
    pub player: PlayerFilter,
}

impl ReorderGraveyardEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtraTurnEffect {
    pub player: PlayerFilter,
}

impl ExtraTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtraTurnAfterNextTurnEffect {
    pub player: PlayerFilter,
}

impl ExtraTurnAfterNextTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdditionalPhase {
    Combat,
    Main,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdditionalPhasesEffect {
    pub phases: Vec<AdditionalPhase>,
}

impl AdditionalPhasesEffect {
    pub fn new(phases: Vec<AdditionalPhase>) -> Self {
        Self { phases }
    }

    pub fn combat() -> Self {
        Self::new(vec![AdditionalPhase::Combat])
    }

    pub fn combat_then_main() -> Self {
        Self::new(vec![AdditionalPhase::Combat, AdditionalPhase::Main])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringObjectEffect {
    pub tag: TagKey,
}

impl TagTriggeringObjectEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringSourceEffect {
    pub tag: TagKey,
}

impl TagTriggeringSourceEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagAttachedToSourceEffect {
    pub tag: TagKey,
}

impl TagAttachedToSourceEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveAllCountersEffect {
    pub from: ChooseSpec,
    pub to: ChooseSpec,
}

impl MoveAllCountersEffect {
    pub fn new(from: ChooseSpec, to: ChooseSpec) -> Self {
        Self { from, to }
    }

    pub fn between_creatures() -> Self {
        Self::new(ChooseSpec::creature(), ChooseSpec::creature())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveOneCounterEffect {
    pub from: ChooseSpec,
    pub to: ChooseSpec,
}

impl MoveOneCounterEffect {
    pub fn new(from: ChooseSpec, to: ChooseSpec) -> Self {
        Self { from, to }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveCountersEffect {
    pub counter_type: crate::counter::CounterType,
    pub count: Value,
    pub from: ChooseSpec,
    pub to: ChooseSpec,
}

impl MoveCountersEffect {
    pub fn new(
        counter_type: crate::counter::CounterType,
        count: impl Into<Value>,
        from: ChooseSpec,
        to: ChooseSpec,
    ) -> Self {
        Self {
            counter_type,
            count: count.into(),
            from,
            to,
        }
    }

    pub fn plus_one_counters(count: impl Into<Value>) -> Self {
        Self::new(
            crate::counter::CounterType::PlusOnePlusOne,
            count,
            ChooseSpec::creature(),
            ChooseSpec::creature(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProliferateEffect {
    pub count: Value,
}

impl ProliferateEffect {
    pub fn new(count: impl Into<Value>) -> Self {
        Self {
            count: count.into(),
        }
    }
}

impl Default for ProliferateEffect {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WinTheGameEffect {
    pub player: PlayerFilter,
}

impl WinTheGameEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastSourceEffect {
    pub without_paying_mana_cost: bool,
    pub require_exile: bool,
}

impl CastSourceEffect {
    pub fn new() -> Self {
        Self {
            without_paying_mana_cost: false,
            require_exile: false,
        }
    }

    pub fn without_paying_mana_cost(mut self) -> Self {
        self.without_paying_mana_cost = true;
        self
    }

    pub fn require_exile(mut self) -> Self {
        self.require_exile = true;
        self
    }
}

impl Default for CastSourceEffect {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmitKeywordActionObjectTag {
    pub effect_id: EffectId,
    pub tag: TagKey,
    pub use_affected_memory: bool,
}

impl EmitKeywordActionObjectTag {
    pub fn affected(effect_id: EffectId, tag: impl Into<TagKey>) -> Self {
        Self {
            effect_id,
            tag: tag.into(),
            use_affected_memory: true,
        }
    }

    pub fn chosen(effect_id: EffectId, tag: impl Into<TagKey>) -> Self {
        Self {
            effect_id,
            tag: tag.into(),
            use_affected_memory: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmitKeywordActionEffect {
    pub action: crate::event_model::KeywordActionKind,
    pub amount: u32,
    pub object_tags: Vec<EmitKeywordActionObjectTag>,
}

impl EmitKeywordActionEffect {
    pub fn new(action: crate::event_model::KeywordActionKind, amount: u32) -> Self {
        Self {
            action,
            amount,
            object_tags: Vec::new(),
        }
    }

    pub fn with_affected_object_memory_tag(
        mut self,
        effect_id: EffectId,
        tag: impl Into<TagKey>,
    ) -> Self {
        self.object_tags
            .push(EmitKeywordActionObjectTag::affected(effect_id, tag));
        self
    }

    pub fn with_chosen_object_memory_tag(
        mut self,
        effect_id: EffectId,
        tag: impl Into<TagKey>,
    ) -> Self {
        self.object_tags
            .push(EmitKeywordActionObjectTag::chosen(effect_id, tag));
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscardHandEffect {
    pub player: PlayerFilter,
}

impl DiscardHandEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCardTypeEffect {
    pub chooser: PlayerFilter,
    pub options: Vec<CardType>,
}

impl ChooseCardTypeEffect {
    pub fn new(chooser: PlayerFilter, options: Vec<CardType>) -> Self {
        Self { chooser, options }
    }

    pub fn all_card_types() -> &'static [CardType] {
        &[
            CardType::Artifact,
            CardType::Battle,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Instant,
            CardType::Kindred,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Sorcery,
        ]
    }

    pub fn card_type_options(&self) -> Vec<CardType> {
        if self.options.is_empty() {
            Self::all_card_types().to_vec()
        } else {
            self.options.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCardNameEffect {
    pub chooser: PlayerFilter,
    pub filter: Option<ObjectFilter>,
    pub tag: TagKey,
}

impl ChooseCardNameEffect {
    pub fn new(
        chooser: PlayerFilter,
        filter: Option<ObjectFilter>,
        tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            chooser,
            filter,
            tag: tag.into(),
        }
    }
}

/// When a player-control effect starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerControlStart {
    /// Starts immediately when the effect resolves.
    Immediate,
    /// Starts at the beginning of the target player's next turn.
    NextTurn,
}

/// How long a player-control effect lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerControlDuration {
    /// Until end of the current turn.
    UntilEndOfTurn,
    /// Until the source leaves the battlefield.
    UntilSourceLeaves,
    /// No duration limit.
    Forever,
}

/// Effect that lets a player control another player's decisions.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlPlayerEffect {
    /// Which player is controlled.
    pub player: PlayerFilter,
    /// When control begins.
    pub start: PlayerControlStart,
    /// How long control lasts.
    pub duration: PlayerControlDuration,
    /// Target spec if this effect targets a player.
    pub target_spec: Option<ChooseSpec>,
}

impl ControlPlayerEffect {
    /// Create a new control player effect.
    pub fn new(
        player: PlayerFilter,
        start: PlayerControlStart,
        duration: PlayerControlDuration,
    ) -> Self {
        let target_spec = match &player {
            PlayerFilter::Target(inner) => {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            }
            _ => None,
        };
        Self {
            player,
            start,
            duration,
            target_spec,
        }
    }

    /// Control a player until end of turn.
    pub fn until_end_of_turn(player: PlayerFilter) -> Self {
        Self::new(
            player,
            PlayerControlStart::Immediate,
            PlayerControlDuration::UntilEndOfTurn,
        )
    }

    /// Control a player during their next turn.
    pub fn during_next_turn(player: PlayerFilter) -> Self {
        Self::new(
            player,
            PlayerControlStart::NextTurn,
            PlayerControlDuration::UntilEndOfTurn,
        )
    }

    /// Control a player indefinitely.
    pub fn forever(player: PlayerFilter) -> Self {
        Self::new(
            player,
            PlayerControlStart::Immediate,
            PlayerControlDuration::Forever,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CombatDamagePreventionTarget {
    All,
    Players,
    You,
    From(ChooseSpec),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllCombatDamageEffect {
    pub target: CombatDamagePreventionTarget,
    pub until: Until,
}

impl PreventAllCombatDamageEffect {
    pub fn new(target: CombatDamagePreventionTarget, until: Until) -> Self {
        Self { target, until }
    }
}

/// What a prevention shield protects.
#[derive(Debug, Clone, PartialEq)]
pub enum PreventionTarget {
    /// Protects a specific player.
    Player(crate::ids::PlayerId),
    /// Protects a specific permanent.
    Permanent(crate::ids::ObjectId),
    /// Protects all permanents matching a filter.
    PermanentsMatching(ObjectFilter),
    /// Protects all players.
    Players,
    /// Protects "you" (the shield's controller).
    You,
    /// Protects "you and permanents you control".
    YouAndPermanentsYouControl,
    /// Protects everything.
    All,
}

/// Filter for what kind of damage a prevention shield applies to.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DamageFilter {
    /// Only prevent combat damage.
    pub combat_only: bool,
    /// Only prevent noncombat damage.
    pub noncombat_only: bool,
    /// Only prevent damage from sources matching this filter.
    pub from_source: Option<ObjectFilter>,
    /// Only prevent damage from sources of these colors.
    pub from_colors: Option<Vec<Color>>,
    /// Only prevent damage from sources of these card types.
    pub from_card_types: Option<Vec<CardType>>,
    /// Only prevent damage from a specific source.
    pub from_specific_source: Option<crate::ids::ObjectId>,
}

impl DamageFilter {
    /// Create a filter that matches all damage.
    pub fn all() -> Self {
        Self::default()
    }

    /// Create a filter for combat damage only.
    pub fn combat() -> Self {
        Self {
            combat_only: true,
            ..Default::default()
        }
    }

    /// Create a filter for noncombat damage only.
    pub fn noncombat() -> Self {
        Self {
            noncombat_only: true,
            ..Default::default()
        }
    }

    /// Create a filter for damage from sources of a specific color.
    pub fn from_color(color: Color) -> Self {
        Self {
            from_colors: Some(vec![color]),
            ..Default::default()
        }
    }

    /// Create a filter for damage from a specific source.
    pub fn from_source(source: crate::ids::ObjectId) -> Self {
        Self {
            from_specific_source: Some(source),
            ..Default::default()
        }
    }

    /// Check if this filter matches the given damage parameters.
    pub fn matches(
        &self,
        is_combat: bool,
        source: crate::ids::ObjectId,
        source_colors: &ColorSet,
        source_card_types: &[CardType],
    ) -> bool {
        if self.combat_only && !is_combat {
            return false;
        }
        if self.noncombat_only && is_combat {
            return false;
        }
        if let Some(specific) = self.from_specific_source
            && source != specific
        {
            return false;
        }
        if let Some(ref colors) = self.from_colors {
            let matches_color = colors.iter().any(|c| source_colors.contains(*c));
            if !matches_color {
                return false;
            }
        }
        if let Some(ref types) = self.from_card_types {
            let matches_type = types.iter().any(|t| source_card_types.contains(t));
            if !matches_type {
                return false;
            }
        }
        true
    }
}

/// Effect that prevents all damage until a duration expires.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllDamageEffect {
    /// What this shield protects.
    pub target: PreventionTarget,
    /// What kinds of damage this shield prevents.
    pub damage_filter: DamageFilter,
    pub until: Until,
}

impl PreventAllDamageEffect {
    /// Create a new prevent-all-damage effect.
    pub fn new(target: PreventionTarget, damage_filter: DamageFilter, until: Until) -> Self {
        Self {
            target,
            damage_filter,
            until,
        }
    }

    /// Prevent all damage to everything.
    pub fn all(until: Until) -> Self {
        Self::new(PreventionTarget::All, DamageFilter::all(), until)
    }

    /// Prevent all damage to the controller.
    pub fn to_you(until: Until) -> Self {
        Self::new(PreventionTarget::You, DamageFilter::all(), until)
    }

    /// Prevent all damage to permanents matching the filter.
    pub fn matching(filter: ObjectFilter, until: Until) -> Self {
        Self::new(
            PreventionTarget::PermanentsMatching(filter),
            DamageFilter::all(),
            until,
        )
    }

    /// Prevent all damage to everything with a damage filter.
    pub fn all_with_filter(damage_filter: DamageFilter, until: Until) -> Self {
        Self::new(PreventionTarget::All, damage_filter, until)
    }

    /// Prevent all damage to permanents matching the filter with a damage filter.
    pub fn matching_with_filter(
        filter: ObjectFilter,
        damage_filter: DamageFilter,
        until: Until,
    ) -> Self {
        Self::new(
            PreventionTarget::PermanentsMatching(filter),
            damage_filter,
            until,
        )
    }

    /// Prevent all damage to creatures you control.
    pub fn your_creatures(until: Until) -> Self {
        Self::matching(ObjectFilter::creature().you_control(), until)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateEmblemEffect<E> {
    pub emblem: E,
}

impl<E> CreateEmblemEffect<E> {
    pub fn new(emblem: E) -> Self {
        Self { emblem }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantEffect<G, D> {
    pub grantable: G,
    pub target: ChooseSpec,
    pub duration: D,
}

impl<G, D> GrantEffect<G, D> {
    pub fn new(grantable: G, target: ChooseSpec, duration: D) -> Self {
        Self {
            grantable,
            target,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantBySpecEffect<S, D> {
    pub spec: S,
    pub player: PlayerFilter,
    pub duration: D,
}

impl<S, D> GrantBySpecEffect<S, D> {
    pub fn new(spec: S, player: PlayerFilter, duration: D) -> Self {
        Self {
            spec,
            player,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleObjectsIntoLibraryEffect {
    pub target: ChooseSpec,
    pub player: PlayerFilter,
}

impl ShuffleObjectsIntoLibraryEffect {
    pub fn new(target: ChooseSpec, player: PlayerFilter) -> Self {
        Self { target, player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeTextBoxesEffect {
    pub target: ChooseSpec,
}

impl ExchangeTextBoxesEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyCountersEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl EnergyCountersEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TicketCountersEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl TicketCountersEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoverEffect {
    pub count: Value,
    pub player: PlayerFilter,
}

impl DiscoverEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetBasePowerToughnessEffect {
    pub target: ChooseSpec,
    pub power: Value,
    pub toughness: Value,
    pub duration: Until,
}

impl SetBasePowerToughnessEffect {
    pub fn new(
        target: ChooseSpec,
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        duration: Until,
    ) -> Self {
        Self {
            target,
            power: power.into(),
            toughness: toughness.into(),
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CantEffect {
    pub restriction: Restriction,
    pub duration: Until,
}

impl CantEffect {
    pub fn new(restriction: Restriction, duration: Until) -> Self {
        Self {
            restriction,
            duration,
        }
    }

    pub fn until_end_of_turn(restriction: Restriction) -> Self {
        Self::new(restriction, Until::EndOfTurn)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModifyPowerToughnessForEachEffect {
    pub target: ChooseSpec,
    pub power_per: i32,
    pub toughness_per: i32,
    pub count: Value,
    pub duration: Until,
}

impl ModifyPowerToughnessForEachEffect {
    pub fn new(
        target: ChooseSpec,
        power_per: i32,
        toughness_per: i32,
        count: Value,
        duration: Until,
    ) -> Self {
        Self {
            target,
            power_per,
            toughness_per,
            count,
            duration,
        }
    }

    pub fn symmetric(target: ChooseSpec, per: i32, count: Value, duration: Until) -> Self {
        Self::new(target, per, per, count, duration)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveUpToAnyCountersEffect {
    pub max_count: Value,
    pub target: ChooseSpec,
}

impl RemoveUpToAnyCountersEffect {
    pub fn new(max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            max_count: max_count.into(),
            target,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveUpToCountersEffect {
    pub counter_type: crate::counter::CounterType,
    pub max_count: Value,
    pub target: ChooseSpec,
}

impl RemoveUpToCountersEffect {
    pub fn new(
        counter_type: crate::counter::CounterType,
        max_count: impl Into<Value>,
        target: ChooseSpec,
    ) -> Self {
        Self {
            counter_type,
            max_count: max_count.into(),
            target,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachToEffect {
    pub target: ChooseSpec,
}

impl AttachToEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReconfigureEffect {
    pub target: ChooseSpec,
}

impl ReconfigureEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachObjectsEffect {
    pub objects: ChooseSpec,
    pub target: ChooseSpec,
}

impl AttachObjectsEffect {
    pub fn new(objects: ChooseSpec, target: ChooseSpec) -> Self {
        Self { objects, target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevealTopEffect {
    pub player: PlayerFilter,
    pub tag: Option<TagKey>,
}

impl RevealTopEffect {
    pub fn new(player: PlayerFilter, tag: Option<TagKey>) -> Self {
        Self { player, tag }
    }

    pub fn tagged(player: PlayerFilter, tag: impl Into<TagKey>) -> Self {
        Self::new(player, Some(tag.into()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnAsAuraOptions {
    pub attachment_filter: ObjectFilter,
    pub remove_all_abilities: bool,
}

impl ReturnAsAuraOptions {
    pub fn new(attachment_filter: ObjectFilter) -> Self {
        Self {
            attachment_filter,
            remove_all_abilities: false,
        }
    }

    pub fn remove_all_abilities(mut self) -> Self {
        self.remove_all_abilities = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardToBattlefieldEffect {
    pub target: ChooseSpec,
    pub tapped: bool,
    pub as_aura: Option<ReturnAsAuraOptions>,
}

impl ReturnFromGraveyardToBattlefieldEffect {
    pub fn new(target: ChooseSpec, tapped: bool) -> Self {
        Self {
            target,
            tapped,
            as_aura: None,
        }
    }

    pub fn as_aura(mut self, attachment_filter: ObjectFilter) -> Self {
        self.as_aura = Some(ReturnAsAuraOptions::new(attachment_filter));
        self
    }

    pub fn as_aura_removing_all_abilities(mut self, attachment_filter: ObjectFilter) -> Self {
        self.as_aura = Some(ReturnAsAuraOptions::new(attachment_filter).remove_all_abilities());
        self
    }

    pub fn creature() -> Self {
        Self::new(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(crate::zone::Zone::Graveyard)),
            false,
        )
    }

    pub fn creature_tapped() -> Self {
        Self::new(
            ChooseSpec::Object(ObjectFilter::creature().in_zone(crate::zone::Zone::Graveyard)),
            true,
        )
    }

    pub fn any_card() -> Self {
        Self::new(
            ChooseSpec::card_in_zone(crate::zone::Zone::Graveyard),
            false,
        )
    }

    pub fn any_card_tapped() -> Self {
        Self::new(ChooseSpec::card_in_zone(crate::zone::Zone::Graveyard), true)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardToHandEffect {
    pub target: ChooseSpec,
    pub random: bool,
}

impl ReturnFromGraveyardToHandEffect {
    pub fn new(target: ChooseSpec, random: bool) -> Self {
        Self { target, random }
    }

    pub fn any_card() -> Self {
        Self::new(
            ChooseSpec::card_in_zone(crate::zone::Zone::Graveyard),
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutOntoBattlefieldEffect {
    pub target: ChooseSpec,
    pub tapped: bool,
    pub controller: PlayerFilter,
}

impl PutOntoBattlefieldEffect {
    pub fn new(target: ChooseSpec, tapped: bool, controller: PlayerFilter) -> Self {
        Self {
            target,
            tapped,
            controller,
        }
    }

    pub fn you_control(target: ChooseSpec, tapped: bool) -> Self {
        Self::new(target, tapped, PlayerFilter::You)
    }

    pub fn owner_control(target: ChooseSpec, tapped: bool) -> Self {
        Self::new(target, tapped, PlayerFilter::OwnerOf(ObjectRef::Target))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleLibraryEffect {
    pub player: PlayerFilter,
    pub target_spec: Option<ChooseSpec>,
}

impl ShuffleLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        let target_spec = match &player {
            PlayerFilter::Target(inner) => {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            }
            _ => None,
        };
        Self {
            player,
            target_spec,
        }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MayMoveToZoneEffect {
    pub target: ChooseSpec,
    pub zone: crate::zone::Zone,
    pub decider: PlayerFilter,
}

impl MayMoveToZoneEffect {
    pub fn new(target: ChooseSpec, zone: crate::zone::Zone, decider: PlayerFilter) -> Self {
        Self {
            target,
            zone,
            decider,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LookAtTopCardsEffect {
    pub player: PlayerFilter,
    pub count: Value,
    pub tag: TagKey,
    pub reveal: bool,
}

impl LookAtTopCardsEffect {
    pub fn new(player: PlayerFilter, count: impl Into<Value>, tag: impl Into<TagKey>) -> Self {
        Self {
            player,
            count: count.into(),
            tag: tag.into(),
            reveal: false,
        }
    }

    pub fn revealing(
        player: PlayerFilter,
        count: impl Into<Value>,
        tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            player,
            count: count.into(),
            tag: tag.into(),
            reveal: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonstrosityEffect {
    pub n: Value,
}

impl MonstrosityEffect {
    pub fn new(n: impl Into<Value>) -> Self {
        Self { n: n.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvolveEffect;

impl EvolveEffect {
    pub const fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SoulbondPairEffect;

impl SoulbondPairEffect {
    pub const fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformEffect {
    pub target: ChooseSpec,
}

impl TransformEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }

    pub fn target_permanent() -> Self {
        Self::new(ChooseSpec::permanent())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastTaggedEffect {
    pub tag: TagKey,
    pub player: PlayerFilter,
    pub allow_land: bool,
    pub as_copy: bool,
    pub without_paying_mana_cost: bool,
    pub cost_reduction: Option<ManaCost>,
}

impl CastTaggedEffect {
    pub fn new(tag: impl Into<TagKey>, player: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            player,
            allow_land: false,
            as_copy: false,
            without_paying_mana_cost: false,
            cost_reduction: None,
        }
    }

    pub fn allow_land(mut self) -> Self {
        self.allow_land = true;
        self
    }

    pub fn as_copy(mut self) -> Self {
        self.as_copy = true;
        self
    }

    pub fn without_paying_mana_cost(mut self) -> Self {
        self.without_paying_mana_cost = true;
        self
    }

    pub fn cost_reduction(mut self, reduction: ManaCost) -> Self {
        self.cost_reduction = Some(reduction);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryBottomOrder {
    Random,
    ChooserChooses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryConsultMode {
    Reveal,
    Exile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutTaggedRemainderOnLibraryBottomEffect {
    pub tag: TagKey,
    pub keep_tagged: Option<TagKey>,
    pub order: LibraryBottomOrder,
    pub player: PlayerFilter,
}

impl PutTaggedRemainderOnLibraryBottomEffect {
    pub fn new(
        tag: impl Into<TagKey>,
        keep_tagged: Option<TagKey>,
        order: LibraryBottomOrder,
        player: PlayerFilter,
    ) -> Self {
        Self {
            tag: tag.into(),
            keep_tagged,
            order,
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAnyCountersAmongEffect {
    pub count: u32,
    pub min_count: u32,
    pub dynamic_count: bool,
    pub display_x: bool,
    pub filter: ObjectFilter,
    pub counter_type: Option<crate::counter::CounterType>,
}

impl RemoveAnyCountersAmongEffect {
    pub fn new(count: u32, filter: ObjectFilter) -> Self {
        Self {
            count,
            min_count: count,
            dynamic_count: false,
            display_x: false,
            filter,
            counter_type: None,
        }
    }

    pub fn dynamic(min_count: u32, max_count: u32, filter: ObjectFilter, display_x: bool) -> Self {
        Self {
            count: max_count,
            min_count,
            dynamic_count: true,
            display_x,
            filter,
            counter_type: None,
        }
    }

    pub fn with_counter_type(mut self, counter_type: Option<crate::counter::CounterType>) -> Self {
        self.counter_type = counter_type;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SacrificePlayerEffect {
    pub filter: ObjectFilter,
    pub count: Value,
    pub player: PlayerFilter,
}

impl SacrificePlayerEffect {
    pub fn new(filter: ObjectFilter, count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            filter,
            count: count.into(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RearrangeLookedCardsInLibraryEffect {
    pub tag: TagKey,
    pub chooser: PlayerFilter,
    pub count: ChoiceCount,
}

impl RearrangeLookedCardsInLibraryEffect {
    pub fn new(tag: impl Into<TagKey>, chooser: PlayerFilter, count: ChoiceCount) -> Self {
        Self {
            tag: tag.into(),
            chooser,
            count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNewTargetsEffect {
    pub from_effect: EffectId,
    pub may: bool,
    pub chooser: Option<PlayerFilter>,
}

impl ChooseNewTargetsEffect {
    pub fn new(from_effect: EffectId, may: bool) -> Self {
        Self {
            from_effect,
            may,
            chooser: None,
        }
    }

    pub fn new_for_player(from_effect: EffectId, may: bool, chooser: PlayerFilter) -> Self {
        Self {
            from_effect,
            may,
            chooser: Some(chooser),
        }
    }

    pub fn may(from_effect: EffectId) -> Self {
        Self::new(from_effect, true)
    }

    pub fn may_for_player(from_effect: EffectId, chooser: PlayerFilter) -> Self {
        Self::new_for_player(from_effect, true, chooser)
    }

    pub fn must(from_effect: EffectId) -> Self {
        Self::new(from_effect, false)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileInsteadOfGraveyardEffect {
    pub player: PlayerFilter,
}

impl ExileInsteadOfGraveyardEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsultTopOfLibraryEffect {
    pub player: PlayerFilter,
    pub mode: LibraryConsultMode,
    pub filter: ObjectFilter,
    pub stop_rule: ConsultTopOfLibraryStopRule,
    pub all_tag: TagKey,
    pub match_tag: TagKey,
}

impl ConsultTopOfLibraryEffect {
    pub fn new(
        player: PlayerFilter,
        mode: LibraryConsultMode,
        filter: ObjectFilter,
        stop_rule: ConsultTopOfLibraryStopRule,
        all_tag: impl Into<TagKey>,
        match_tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            player,
            mode,
            filter,
            stop_rule,
            all_tag: all_tag.into(),
            match_tag: match_tag.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatProcessEffect<E> {
    pub effects: Vec<E>,
    pub condition: EffectId,
    pub predicate: EffectPredicate,
}

impl<E> RepeatProcessEffect<E> {
    pub fn new(effects: Vec<E>, condition: EffectId, predicate: EffectPredicate) -> Self {
        Self {
            effects,
            condition,
            predicate,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatEffectsEffect<E> {
    pub count: Value,
    pub effects: Vec<E>,
}

impl<E> RepeatEffectsEffect<E> {
    pub fn new(count: impl Into<Value>, effects: Vec<E>) -> Self {
        Self {
            count: count.into(),
            effects,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoseLifeEffect {
    pub amount: Value,
    pub player: PlayerFilter,
}

impl LoseLifeEffect {
    pub fn new(amount: Value, player: PlayerFilter) -> Self {
        Self { amount, player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatProcessPromptEffect {
    pub text: String,
}

impl RepeatProcessPromptEffect {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChoosePlayerEffect {
    pub chooser: PlayerFilter,
    pub filter: PlayerFilter,
    pub tag: crate::tag::TagKey,
    pub excluded_tags: Vec<crate::tag::TagKey>,
    pub random: bool,
    pub remember_as_chosen_player: bool,
}

impl ChoosePlayerEffect {
    pub fn new(
        chooser: PlayerFilter,
        filter: PlayerFilter,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            chooser,
            filter,
            tag: tag.into(),
            excluded_tags: Vec::new(),
            random: false,
            remember_as_chosen_player: false,
        }
    }

    pub fn excluding_tags(mut self, tags: Vec<crate::tag::TagKey>) -> Self {
        self.excluded_tags = tags;
        self
    }

    pub fn at_random(mut self) -> Self {
        self.random = true;
        self
    }

    pub fn remember_as_chosen_player(mut self) -> Self {
        self.remember_as_chosen_player = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceEffect<E> {
    pub effects: Vec<E>,
}

impl<E> SequenceEffect<E> {
    pub fn new(effects: Vec<E>) -> Self {
        Self { effects }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManaRestrictedEffect<E> {
    pub effects: Vec<E>,
    pub restrictions: Vec<crate::ManaUsageRestriction>,
}

impl<E> ManaRestrictedEffect<E> {
    pub fn new(effects: Vec<E>, restrictions: Vec<crate::ManaUsageRestriction>) -> Self {
        Self {
            effects,
            restrictions,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MayEffect<E> {
    pub decider: Option<PlayerFilter>,
    pub effects: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnlessPaysEffect<E> {
    pub player: PlayerFilter,
    pub effects: Vec<E>,
    pub cost: crate::cost_model::TotalCost<crate::cost_model::Cost<E>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CumulativeUpkeepEffect<E> {
    pub player: PlayerFilter,
    pub payment: Vec<E>,
    pub failure: Vec<E>,
}

impl<E> CumulativeUpkeepEffect<E> {
    pub fn new(player: PlayerFilter, payment: Vec<E>, failure: Vec<E>) -> Self {
        Self {
            player,
            payment,
            failure,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnlessActionEffect<E> {
    pub player: PlayerFilter,
    pub effects: Vec<E>,
    pub alternative: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForPlayersEffect<E> {
    pub filter: PlayerFilter,
    pub effects: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachTaggedEffect<E> {
    pub tag: crate::tag::TagKey,
    pub effects: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachControllerOfTaggedEffect<E> {
    pub tag: crate::tag::TagKey,
    pub effects: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachTaggedPlayerEffect<E> {
    pub tag: crate::tag::TagKey,
    pub effects: Vec<E>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflexiveTriggerEffect<E> {
    pub condition: EffectId,
    pub predicate: EffectPredicate,
    pub effects: Vec<E>,
    pub choices: Vec<ChooseSpec>,
}
