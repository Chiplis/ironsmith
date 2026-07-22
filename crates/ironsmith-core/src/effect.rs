//! Shared effect-domain metadata.
//!
//! These types describe effect identity and selection cardinality without
//! pulling in the runtime execution engine.

use crate::filter_model::{AlternativeCastKind, ObjectFilter, ObjectRef, PlayerFilter};
use crate::mana::{ManaCost, ManaSymbol};
use crate::tag::TagKey;
use crate::target_model::ChooseSpec;
use crate::types::{CardType, Subtype, Supertype};
use crate::value_model::{PriorEffectAction, Restriction, Value};
use crate::{Color, ColorSet, CounterType, SourceReferenceSurface};

mod ascend;
mod mana_damage_and_control;
pub use ascend::*;
pub use mana_damage_and_control::*;

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

/// Aggregate value used to constrain a group of chosen objects.
///
/// Unlike an [`ObjectFilter`], this applies to the selection as a whole. For
/// example, "choose any number of creatures with total power 4 or less" uses
/// `Power` with a maximum of 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceAggregateMetric {
    Power,
    Toughness,
    ManaValue,
}

/// Upper bound on an aggregate characteristic of a group of chosen objects.
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceAggregateConstraint {
    pub metric: ChoiceAggregateMetric,
    pub maximum: Value,
}

impl ChoiceAggregateConstraint {
    pub fn at_most(metric: ChoiceAggregateMetric, maximum: impl Into<Value>) -> Self {
        Self {
            metric,
            maximum: maximum.into(),
        }
    }

    pub fn total_power_at_most(maximum: impl Into<Value>) -> Self {
        Self::at_most(ChoiceAggregateMetric::Power, maximum)
    }

    pub fn total_mana_value_at_most(maximum: impl Into<Value>) -> Self {
        Self::at_most(ChoiceAggregateMetric::ManaValue, maximum)
    }
}

/// Object identity used by a predicate-bearing resolution duration.
///
/// Semantic references are materialized to `Specific` when the resolving
/// spell or ability creates its continuous effect. This keeps the duration's
/// operands distinct from both the effect source and the fixed affected set.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContinuousDurationObject {
    Source,
    AffectedObject,
    Tagged(TagKey),
    Specific(crate::ObjectId),
}

/// Player identity used by a predicate-bearing resolution duration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContinuousDurationPlayer {
    EffectController,
    ControllerOf(ContinuousDurationObject),
    Tagged(TagKey),
    Specific(crate::PlayerId),
}

/// A reusable current-state predicate for CR 611.2b durations.
///
/// These predicates are evaluated against current game state, but their
/// duration is latched: a false initial value starts no effect, and a false
/// value after the effect starts expires it permanently.
#[derive(Debug, Clone, PartialEq)]
pub enum ContinuousDurationPredicate {
    All(Vec<ContinuousDurationPredicate>),
    ObjectOnBattlefield(ContinuousDurationObject),
    ObjectTapped(ContinuousDurationObject),
    ObjectControlledBy {
        object: ContinuousDurationObject,
        player: ContinuousDurationPlayer,
    },
    ObjectHasCounter {
        object: ContinuousDurationObject,
        counter_type: CounterType,
        minimum: u32,
    },
    ObjectAttachedTo {
        attachment: ContinuousDurationObject,
        attached_to: ContinuousDurationObject,
    },
    ObjectIsEnchanted(ContinuousDurationObject),
    PlayerIsMonarch(ContinuousDurationPlayer),
    ObjectPowerAtMostObject {
        lesser: ContinuousDurationObject,
        greater: ContinuousDurationObject,
    },
}

impl ContinuousDurationPredicate {
    pub fn all(predicates: impl IntoIterator<Item = Self>) -> Self {
        Self::All(predicates.into_iter().collect())
    }

    pub fn affected_object_has_counter(counter_type: CounterType) -> Self {
        Self::ObjectHasCounter {
            object: ContinuousDurationObject::AffectedObject,
            counter_type,
            minimum: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Until {
    #[default]
    Forever,
    EndOfTurn,
    YourNextTurn,
    YourNextTurnEnd,
    YourNextUpkeep,
    ControllersNextUntapStep,
    EndOfCombat,
    ThisLeavesTheBattlefield,
    SourceUntaps,
    YouStopControllingThis,
    /// A CR 611.2b duration whose predicate is materialized at resolution and
    /// whose first transition to false is permanent.
    ForAsLongAs(ContinuousDurationPredicate),
    TurnsPass(crate::value_model::Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectPredicate {
    Succeeded,
    Failed,
    Happened,
    DidNotHappen,
    SearchedLibrary,
    HappenedNotReplaced,
    ExcessDamageDealt,
    DealtDamageToPlayer,
    AffectedObjectMatchesCardType {
        card_type: CardType,
        negated: bool,
    },
    /// A result predicate over the captured objects produced by one prior
    /// action, preserving the authored `... this way` relationship.
    ///
    /// Unlike the legacy card-type-only predicate, this retains the complete
    /// object filter plus voice and quantifier presentation. Runtime matching
    /// is still performed against the antecedent's last-known-information
    /// memory rather than live objects.
    PriorEffectResult(PriorEffectResultSurface),
    Value(crate::effect_model::Comparison),
    Chosen,
    WasDeclined,
}

/// Authored grammatical subject for a prior-result predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorEffectResultActor {
    /// Passive surface such as "a creature card is exiled this way."
    Passive,
    /// Active controller surface such as "you discard a card this way."
    You,
    /// Active iterated-player surface such as "that player discards a card this way."
    ThatPlayer,
    /// Reflexive source surface such as "it connives this way."
    It,
}

/// Authored cardinality for a prior-result predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorEffectResultQuantifier {
    /// An ordinary singular result ("a creature card").
    One,
    /// An explicit nonzero plural result ("one or more nonland cards").
    OneOrMore,
    /// The action itself is the predicate and has no object noun surface.
    ActionOnly,
}

/// Typed presentation and filtering data for a `... this way` result gate.
#[derive(Debug, Clone, PartialEq)]
pub struct PriorEffectResultSurface {
    pub action: PriorEffectAction,
    pub filter: ObjectFilter,
    pub actor: PriorEffectResultActor,
    pub quantifier: PriorEffectResultQuantifier,
}

impl PriorEffectResultSurface {
    pub fn new(
        action: PriorEffectAction,
        filter: ObjectFilter,
        actor: PriorEffectResultActor,
        quantifier: PriorEffectResultQuantifier,
    ) -> Self {
        Self {
            action,
            filter,
            actor,
            quantifier,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantPlayTaggedDuration {
    UntilEndOfTurn,
    UntilYourNextTurnEnd,
    UntilYourNextEndStep,
    ForAsLongAsExiled,
    ForAsLongAsYouControlSource,
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
    ChoiceMatching(crate::filter_model::ObjectFilter),
    Target(ChooseSpec),
    Filter(crate::filter_model::ObjectFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreventNextTimeDamageTarget {
    AnyTarget,
    Omitted,
    You,
    Target(ChooseSpec),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectNextTimeDamageSource {
    Choice,
    Filter(crate::filter_model::ObjectFilter),
    Target(crate::target_model::ChooseSpec),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectNextTimeDamageDestination {
    SourceObject,
    Controller,
    SourceController,
    TargetObject,
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
    BeginningOfCombat(PlayerFilter),
    BeginningOfMainPhase(PlayerFilter),
    BeginningOfPrecombatMainPhase(PlayerFilter),
    BeginningOfPostcombatMainPhase(PlayerFilter),
    EndOfCombat,
    SourceControllerLosesControl {
        source_description: String,
    },
    ThisEntersBattlefield,
    ThisEntersBattlefieldWithSurface {
        surface: SourceReferenceSurface,
        subject_number: crate::trigger_model::TriggerSubjectNumber,
    },
    EntersBattlefield {
        filter: ObjectFilter,
        cause_filter: Option<crate::cause_model::CauseFilter>,
        count: crate::trigger_model::CountMode,
        tapped: Option<bool>,
    },
    ThisDies,
    ThisLeavesBattlefield,
    ThisAttacksAndIsntBlocked,
    Attacks(ObjectFilter),
    AttacksAndIsntBlocked(ObjectFilter),
    AttacksOneOrMore(ObjectFilter),
    Blocks(ObjectFilter),
    BlocksOneOrMore(ObjectFilter),
    BecomesBlocked(ObjectFilter),
    LeavesBattlefield(ObjectFilter),
    Dies(ObjectFilter),
    PermanentBecomesTapped(ObjectFilter),
    DealsCombatDamage(ObjectFilter),
    DealsCombatDamageTo {
        source: ObjectFilter,
        target: ObjectFilter,
    },
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
        timing: Option<crate::trigger_model::TriggerTimingRestriction>,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
        first_spell_of_game: bool,
    },
    PlayerPlaysLand {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    AbilityActivated {
        activator: PlayerFilter,
        filter: ObjectFilter,
        non_mana_only: bool,
        loyalty_only: bool,
        activation_cost_has_tap: Option<bool>,
    },
    Either(Box<DelayedTriggerSpec>, Box<DelayedTriggerSpec>),
}

/// Lifetime policy for a delayed trigger registration.
///
/// The runtime anchors turn-relative variants when the scheduling effect
/// resolves, so intervening extra turns do not change their meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DelayedTriggerDuration {
    #[default]
    Forever,
    EndOfTurn,
    EndOfCombat,
    UntilControllerNextTurn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleDelayedTriggerEffect<E> {
    pub trigger: DelayedTriggerSpec,
    pub effects: Vec<E>,
    pub one_shot: bool,
    pub start_next_turn: bool,
    pub duration: DelayedTriggerDuration,
    pub until_end_of_turn: bool,
    pub until_end_of_combat: bool,
    pub watch_ability_source: bool,
    /// Capture every object target chosen for the resolving spell or ability
    /// and register one watcher per object.
    pub watch_all_object_targets: bool,
    /// Preserve the authored set-reference surface for a watched tagged set,
    /// such as "either of those creatures".
    pub either_of_watched_objects: bool,
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
            duration: DelayedTriggerDuration::Forever,
            until_end_of_turn: false,
            until_end_of_combat: false,
            watch_ability_source: false,
            watch_all_object_targets: false,
            either_of_watched_objects: false,
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
        self.duration = DelayedTriggerDuration::EndOfTurn;
        self.until_end_of_turn = true;
        self.until_end_of_combat = false;
        self
    }

    pub fn until_end_of_combat(mut self) -> Self {
        self.duration = DelayedTriggerDuration::EndOfCombat;
        self.until_end_of_combat = true;
        self.until_end_of_turn = false;
        self
    }

    pub fn until_controller_next_turn(mut self) -> Self {
        self.duration = DelayedTriggerDuration::UntilControllerNextTurn;
        self.until_end_of_turn = false;
        self.until_end_of_combat = false;
        self
    }

    pub fn with_either_of_watched_objects_surface(mut self) -> Self {
        self.either_of_watched_objects = true;
        self
    }

    pub fn watch_ability_source(mut self) -> Self {
        self.watch_ability_source = true;
        self
    }

    pub fn watch_all_object_targets(mut self) -> Self {
        self.watch_all_object_targets = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetQuantifierSurface {
    All,
    Each,
}

/// Oracle surface used when a type-changing effect preserves an object's
/// existing types. Both variants have the same rules meaning, but they render
/// differently and must remain distinguishable after lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRetentionSurface {
    InAdditionToOtherTypes,
    /// The effect adds the creature card type, but Oracle expresses the
    /// animation as a P/T plus creature subtype (for example, "a 1/1
    /// Spirit") without spelling out the `creature` noun.
    InAdditionToOtherTypesImplicitCreature,
    StillALand,
}

/// Oracle surface used to express the power and toughness portion of an
/// animation effect. Both forms create the same base-power/toughness layer,
/// but authored leading P/T ("a 4/4 Angel creature") must not be rewritten as
/// an explicit base-P/T clause ("an Angel creature with base power and
/// toughness 4/4").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationPtSurface {
    LeadingPowerToughness,
    ExplicitBasePowerToughness,
}

/// Oracle placement of an animation effect's duration. Absence retains the
/// legacy trailing-duration surface, while this marker preserves authored
/// leading durations such as "Until end of turn, target land becomes ...".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDurationSurface {
    Leading,
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
    pub source_reference_surface: Option<SourceReferenceSurface>,
    pub set_quantifier_surface: Option<SetQuantifierSurface>,
    pub type_retention_surface: Option<TypeRetentionSurface>,
    pub animation_pt_surface: Option<AnimationPtSurface>,
    pub animation_duration_surface: Option<AnimationDurationSurface>,
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
            source_reference_surface: None,
            set_quantifier_surface: None,
            type_retention_surface: None,
            animation_pt_surface: None,
            animation_duration_surface: None,
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
            source_reference_surface: None,
            set_quantifier_surface: None,
            type_retention_surface: None,
            animation_pt_surface: None,
            animation_duration_surface: None,
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
            source_reference_surface: None,
            set_quantifier_surface: None,
            type_retention_surface: None,
            animation_pt_surface: None,
            animation_duration_surface: None,
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
            source_reference_surface: None,
            set_quantifier_surface: None,
            type_retention_surface: None,
            animation_pt_surface: None,
            animation_duration_surface: None,
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

    pub fn with_source_reference_surface(mut self, surface: SourceReferenceSurface) -> Self {
        self.source_reference_surface = Some(surface);
        self
    }

    pub fn with_set_quantifier_surface(mut self, surface: Option<SetQuantifierSurface>) -> Self {
        self.set_quantifier_surface = surface;
        self
    }

    pub fn with_type_retention_surface(mut self, surface: Option<TypeRetentionSurface>) -> Self {
        self.type_retention_surface = surface;
        self
    }

    pub fn with_animation_pt_surface(mut self, surface: Option<AnimationPtSurface>) -> Self {
        self.animation_pt_surface = surface;
        self
    }

    pub fn with_animation_duration_surface(
        mut self,
        surface: Option<AnimationDurationSurface>,
    ) -> Self {
        self.animation_duration_surface = surface;
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
    /// "The damage can't be prevented." rider on this damage.
    pub unpreventable: bool,
}

impl DealDamageEffect {
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            target,
            source_is_combat: false,
            unpreventable: false,
        }
    }

    pub fn with_combat(mut self, is_combat: bool) -> Self {
        self.source_is_combat = is_combat;
        self
    }

    pub fn with_unpreventable(mut self, unpreventable: bool) -> Self {
        self.unpreventable = unpreventable;
        self
    }
}

/// Remove an exact amount, or all, of the damage marked on a permanent.
///
/// `amount == None` represents the CR 701.69a surface "damage ... is healed,"
/// which removes all marked damage from the permanent.
#[derive(Debug, Clone, PartialEq)]
pub struct HealDamageEffect {
    pub target: ChooseSpec,
    pub amount: Option<Value>,
}

impl HealDamageEffect {
    pub fn exact(target: ChooseSpec, amount: impl Into<Value>) -> Self {
        Self {
            target,
            amount: Some(amount.into()),
        }
    }

    pub fn all(target: ChooseSpec) -> Self {
        Self {
            target,
            amount: None,
        }
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NoteLifeTotalEffect;

#[derive(Debug, Clone, PartialEq)]
pub struct TargetOnlyEffect {
    pub target: ChooseSpec,
    /// The player who makes this target choice when Oracle assigns the choice
    /// to someone other than the spell or ability's controller.
    pub chooser: Option<PlayerFilter>,
    /// Whether this target declaration was an authored standalone clause
    /// (for example, "Choose target opponent.") rather than a synthetic
    /// prelude introduced by lowering so later effects can share a target.
    pub explicit_declaration: bool,
}

impl TargetOnlyEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            chooser: None,
            explicit_declaration: false,
        }
    }

    pub fn explicit(target: ChooseSpec) -> Self {
        Self {
            target,
            chooser: None,
            explicit_declaration: true,
        }
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = Some(chooser);
        self
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

#[derive(Debug, Clone, PartialEq)]
pub struct DoubleCountersEffect {
    pub counter_type: Option<crate::counter::CounterType>,
    pub target: ChooseSpec,
}

impl DoubleCountersEffect {
    pub fn new(counter_type: Option<crate::counter::CounterType>, target: ChooseSpec) -> Self {
        Self {
            counter_type,
            target,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConditionalSurface {
    #[default]
    LeadingIf,
    TrailingIf,
    TrailingUnless,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalEffect<E> {
    pub condition: crate::value_model::Condition,
    pub if_true: Vec<E>,
    pub if_false: Vec<E>,
    pub surface: ConditionalSurface,
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
            surface: ConditionalSurface::LeadingIf,
        }
    }

    pub fn if_only(condition: crate::value_model::Condition, if_true: Vec<E>) -> Self {
        Self::new(condition, if_true, vec![])
    }

    /// A resolution-time condition printed after its effect as
    /// "... unless <condition>". The stored condition remains the executable
    /// gate for `if_true`, so it is the negation of the printed condition.
    pub fn trailing_unless(condition: crate::value_model::Condition, effects: Vec<E>) -> Self {
        Self {
            condition: crate::value_model::Condition::Not(Box::new(condition)),
            if_true: effects,
            if_false: vec![],
            surface: ConditionalSurface::TrailingUnless,
        }
    }

    /// A resolution-time condition authored after its effect as
    /// "... if <condition>".
    pub fn trailing_if(condition: crate::value_model::Condition, effects: Vec<E>) -> Self {
        Self {
            condition,
            if_true: effects,
            if_false: vec![],
            surface: ConditionalSurface::TrailingIf,
        }
    }

    pub fn with_surface(mut self, surface: ConditionalSurface) -> Self {
        self.surface = surface;
        self
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
    pub source_text: String,
    pub effects: Vec<E>,
}

/// A mode-selection range enabled by a later optional-cost announcement.
///
/// CR 601.4 allows an earlier mode choice to consider an optional cost that
/// will be chosen later in the same proposal (for example, kicker enabling
/// "choose any number instead").
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalModeRange {
    pub required_optional_cost: crate::cost_model::OptionalCostRef,
    pub min_modes: Value,
    pub max_modes: Value,
}

impl ConditionalModeRange {
    pub fn new(
        required_optional_cost: impl Into<crate::cost_model::OptionalCostRef>,
        min_modes: impl Into<Value>,
        max_modes: impl Into<Value>,
    ) -> Self {
        Self {
            required_optional_cost: required_optional_cost.into(),
            min_modes: min_modes.into(),
            max_modes: max_modes.into(),
        }
    }
}

impl<E> EffectMode<E> {
    pub fn new(source_text: impl Into<String>, effects: Vec<E>) -> Self {
        Self {
            source_text: source_text.into(),
            effects,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseModeEffect<E> {
    pub modes: Vec<EffectMode<E>>,
    /// When present, this player makes the mode choice during resolution.
    /// Ordinary modal spells leave this unset and use their controller's
    /// casting-time mode selection.
    pub chooser: Option<PlayerFilter>,
    pub min: Value,
    pub max: Value,
    pub allow_repeat: bool,
    pub random: bool,
    pub choose_count: Value,
    pub min_choose_count: Value,
    pub allow_repeated_modes: bool,
    pub mode_point_costs: Vec<u32>,
    /// Whether these are Spree modes whose selected labels are mandatory
    /// additional costs paid during the spell-casting transaction.
    pub spree: bool,
    /// Additional mana cost associated with each mode. Non-Spree modal
    /// effects leave this empty.
    pub mode_additional_mana_costs: Vec<ManaCost>,
    pub disallow_previously_chosen_modes: bool,
    pub disallow_previously_chosen_modes_this_turn: bool,
    /// Each chosen mode must declare a player target different from every other chosen mode.
    pub distinct_player_targets_per_mode: bool,
    /// Alternate mode range that becomes legal if a later optional cost is announced.
    pub conditional_mode_range: Option<ConditionalModeRange>,
}

impl<E> ChooseModeEffect<E> {
    pub fn new(modes: Vec<EffectMode<E>>, min: Value, max: Value, allow_repeat: bool) -> Self {
        let choose_count = max.clone();
        let min_choose_count = min.clone();
        let mode_point_costs = vec![1; modes.len()];
        Self {
            modes,
            chooser: None,
            min,
            max,
            allow_repeat,
            random: false,
            choose_count,
            min_choose_count,
            allow_repeated_modes: allow_repeat,
            mode_point_costs,
            spree: false,
            mode_additional_mana_costs: Vec::new(),
            disallow_previously_chosen_modes: false,
            disallow_previously_chosen_modes_this_turn: false,
            distinct_player_targets_per_mode: false,
            conditional_mode_range: None,
        }
    }

    pub fn choose_one(modes: Vec<EffectMode<E>>) -> Self {
        Self::new(modes, Value::Fixed(1), Value::Fixed(1), false)
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = Some(chooser);
        self
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

    pub fn with_random_mode_choice(mut self) -> Self {
        self.random = true;
        self
    }

    pub fn with_mode_point_costs(mut self, costs: Vec<u32>) -> Self {
        self.mode_point_costs = costs;
        self
    }

    pub fn with_spree_mana_costs(mut self, costs: Vec<ManaCost>) -> Self {
        self.spree = true;
        self.mode_additional_mana_costs = costs;
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

    pub fn with_distinct_player_targets_per_mode(mut self) -> Self {
        self.distinct_player_targets_per_mode = true;
        self
    }

    pub fn with_conditional_mode_range(mut self, range: ConditionalModeRange) -> Self {
        self.conditional_mode_range = Some(range);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VillainousChoiceEffect<E> {
    pub player: PlayerFilter,
    pub player_surface: Option<String>,
    pub modes: Vec<EffectMode<E>>,
}

impl<E> VillainousChoiceEffect<E> {
    pub fn new(player: PlayerFilter, modes: Vec<EffectMode<E>>) -> Self {
        Self {
            player,
            player_surface: None,
            modes,
        }
    }

    pub fn with_player_surface(mut self, surface: impl Into<String>) -> Self {
        self.player_surface = Some(surface.into());
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

/// Oracle-facing noun used to refer back to the single card found by a
/// library search. This is presentation-only metadata; the searched object is
/// still identified by the effect itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchResultReferenceSurface {
    #[default]
    ThatCard,
    TheCard,
}

impl SearchResultReferenceSurface {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ThatCard => "that card",
            Self::TheCard => "the card",
        }
    }
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
    pub result_reference_surface: SearchResultReferenceSurface,
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
            result_reference_surface: SearchResultReferenceSurface::ThatCard,
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

    pub fn with_result_reference_surface(mut self, surface: SearchResultReferenceSurface) -> Self {
        self.result_reference_surface = surface;
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

#[derive(Debug, Clone, PartialEq)]
pub struct LookAtObjectsEffect {
    pub filter: ObjectFilter,
    pub viewer: PlayerFilter,
    pub subject: PlayerFilter,
}

impl LookAtObjectsEffect {
    pub fn new(filter: ObjectFilter, viewer: PlayerFilter, subject: PlayerFilter) -> Self {
        Self {
            filter,
            viewer,
            subject,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RevealSourceFromHandDuration {
    #[default]
    Momentary,
    UntilUpkeepEndsOrLeavesHand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealSourceFromHandEffect {
    pub duration: RevealSourceFromHandDuration,
}

impl RevealSourceFromHandEffect {
    pub fn new() -> Self {
        Self {
            duration: RevealSourceFromHandDuration::Momentary,
        }
    }

    pub fn until_upkeep_ends_or_leaves_hand() -> Self {
        Self {
            duration: RevealSourceFromHandDuration::UntilUpkeepEndsOrLeavesHand,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevealFromHandEffect {
    pub count: Value,
    pub card_type: Option<CardType>,
    pub color_filter: Option<ColorSet>,
}

impl RevealFromHandEffect {
    pub fn new(count: impl Into<Value>, card_type: Option<CardType>) -> Self {
        Self::with_color_filter(count, card_type, None)
    }

    pub fn with_color_filter(
        count: impl Into<Value>,
        card_type: Option<CardType>,
        color_filter: Option<ColorSet>,
    ) -> Self {
        Self {
            count: count.into(),
            card_type,
            color_filter,
        }
    }

    fn count_display(&self) -> String {
        match self.count {
            Value::Fixed(1) => "a".to_string(),
            Value::Fixed(count) => count.to_string(),
            Value::X => "X".to_string(),
            _ => format!("{:?}", self.count),
        }
    }

    fn color_display(&self) -> Option<&'static str> {
        let colors = self.color_filter?;
        if colors.count() != 1 {
            return None;
        }
        Color::ALL
            .iter()
            .find(|&&color| colors.contains(color))
            .map(|&color| color.name())
    }

    pub fn cost_display(&self) -> String {
        let mut card_desc = String::new();
        if let Some(color) = self.color_display() {
            card_desc.push_str(color);
            card_desc.push(' ');
        }
        card_desc.push_str(self.card_type.map_or("card", |ct| ct.card_phrase()));

        if self.count == Value::Fixed(1) {
            format!("Reveal a {card_desc} from your hand")
        } else {
            format!(
                "Reveal {} {card_desc}s from your hand",
                self.count_display()
            )
        }
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
pub struct BecomeSaddledUntilEotEffect;

impl BecomeSaddledUntilEotEffect {
    pub fn new() -> Self {
        Self
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
    /// Also move every battlefield permanent owned by that player.
    pub include_owned_permanents: bool,
}

impl ShuffleHandAndGraveyardIntoLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            include_owned_permanents: false,
        }
    }

    pub fn including_owned_permanents(player: PlayerFilter) -> Self {
        Self {
            player,
            include_owned_permanents: true,
        }
    }
}

/// Oracle-facing cardinality for a zone move of cards linked to the source
/// that exiled them. The runtime selection remains a `ChooseSpec`; this only
/// preserves distinctions that an aggregate selection cannot recover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExiledWithSourceSubjectSurface {
    AllCards,
    EachCard,
    /// "The owner of each card exiled with this source puts that card on the
    /// bottom of their library."
    OwnerOfEachCard,
    OneCard,
    TheExiledCard,
    TheExiledCards,
    TheCards,
    /// A typed or qualified card noun whose semantic filter is already
    /// carried by the zone-move target (for example, "target creature card
    /// with mana value X" or "each creature card").
    Custom(String),
}

/// Oracle-facing reference to the object that exiled the moved cards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExiledWithSourceReferenceSurface {
    Source(SourceReferenceSurface),
    It,
    Omitted,
}

/// Oracle-facing agreement for an owner-relative zone destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExiledWithSourceDestinationSurface {
    ContextualPlayer,
    ItsOwner,
    TheirOwner,
    TheirOwners,
}

/// Oracle-facing verb used for a zone move of cards linked to the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExiledWithSourceMoveVerbSurface {
    Put,
    Return,
}

/// Presentation metadata for a `put ... exiled with ... into ...` clause.
/// Object identity, source linkage, and the destination zone continue to live
/// in the ordinary filter and zone-move fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExiledWithSourceMoveSurface {
    pub verb: ExiledWithSourceMoveVerbSurface,
    pub subject: ExiledWithSourceSubjectSurface,
    pub source: ExiledWithSourceReferenceSurface,
    pub destination: ExiledWithSourceDestinationSurface,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnToHandEffect {
    pub spec: ChooseSpec,
    /// Contextual player named by the oracle destination (for example,
    /// "your hand" or "their hand"). The zone change itself still follows
    /// the rules and moves the object to its owner's hand.
    pub destination_player_surface: Option<PlayerFilter>,
    pub exiled_with_source_surface: Option<ExiledWithSourceMoveSurface>,
    /// Oracle-facing set agreement for references whose rules target has
    /// already been lowered to a tag (for example, "return those creatures").
    pub set_quantifier_surface: Option<SetQuantifierSurface>,
    /// The corresponding oracle noun phrase when a tag no longer carries it.
    /// This is presentation metadata only; `spec` remains authoritative.
    pub set_reference_surface: Option<String>,
}

impl ReturnToHandEffect {
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self {
            spec,
            destination_player_surface: None,
            exiled_with_source_surface: None,
            set_quantifier_surface: None,
            set_reference_surface: None,
        }
    }

    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
            destination_player_surface: None,
            exiled_with_source_surface: None,
            set_quantifier_surface: None,
            set_reference_surface: None,
        }
    }

    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
            destination_player_surface: None,
            exiled_with_source_surface: None,
            set_quantifier_surface: None,
            set_reference_surface: None,
        }
    }

    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
            destination_player_surface: None,
            exiled_with_source_surface: None,
            set_quantifier_surface: None,
            set_reference_surface: None,
        }
    }

    pub fn with_destination_player_surface(mut self, player: PlayerFilter) -> Self {
        self.destination_player_surface = Some(player);
        self
    }

    pub fn with_exiled_with_source_surface(mut self, surface: ExiledWithSourceMoveSurface) -> Self {
        self.exiled_with_source_surface = Some(surface);
        self
    }

    pub fn with_set_quantifier_surface(mut self, surface: Option<SetQuantifierSurface>) -> Self {
        self.set_quantifier_surface = surface;
        self
    }

    pub fn with_set_reference_surface(mut self, surface: Option<String>) -> Self {
        self.set_reference_surface = surface;
        self
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
    /// `None` means each object's owner chooses, matching the common surface.
    pub chooser: Option<PlayerFilter>,
}

impl MoveToLibraryTopOrBottomChoiceEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            chooser: None,
        }
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = Some(chooser);
        self
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

#[derive(Debug, Clone, PartialEq)]
pub struct DirectionalAdjacentPlayerControlEffect {
    pub filter: ObjectFilter,
    pub left_option: String,
    pub right_option: String,
}

impl DirectionalAdjacentPlayerControlEffect {
    pub fn new(
        filter: ObjectFilter,
        left_option: impl Into<String>,
        right_option: impl Into<String>,
    ) -> Self {
        Self {
            filter,
            left_option: left_option.into(),
            right_option: right_option.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlefieldController {
    Preserve,
    Owner,
    You,
}

/// Oracle wording used for a possessive player reference on a zone
/// destination. This is presentation-only; the associated player filter
/// remains the semantic destination antecedent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationPlayerReferenceSurface {
    Pronoun,
    ThatPlayer,
}

/// Oracle verb retained for a generic zone move.
///
/// `Canonical` lets compiled text choose the usual wording from the source and
/// destination zones. Parser-produced moves use `Put` or `Return` so a tagged
/// object does not silently change an oracle `put` into `return` (or vice
/// versa). This is presentation-only; zone-change execution is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveToZoneVerbSurface {
    Canonical,
    Put,
    Return,
}

/// Oracle presentation retained for counters that are part of a one-shot
/// battlefield-entry event.
///
/// Every variant has the same executable meaning: the counters are supplied
/// to ETB replacement processing before the object changes zones. The surface
/// only records how the originating sentence related the entry condition to
/// the move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlefieldEntryCounterSurface {
    /// "It enters with ..."
    Inline,
    /// "Each of them enters with ..."
    EachOfThemEnters,
    /// "If a creature enters this way, it enters with ..."
    IfObjectEntersThisWay,
    /// "If it enters as a creature, it enters with ..."
    IfItEntersAsObject,
    /// "It enters with ... if it's a creature."
    ItEntersIfObject,
    /// "If <condition>, that creature enters with ..."
    ThatObjectEntersIfCondition,
}

/// Counters supplied as part of a one-shot move onto the battlefield.
///
/// This is distinct from [`PutCountersEffect`]: these counters exist on the
/// enter event itself, so replacement effects such as Doubling Season see and
/// modify them at the correct time.
#[derive(Debug, Clone, PartialEq)]
pub struct BattlefieldEntryCounterSpec {
    pub counter_type: crate::counter::CounterType,
    pub amount: Value,
    /// A resolution-time gate known before the zone change (for example,
    /// whether green mana was spent to cast the resolving spell).
    pub condition: Option<crate::value_model::Condition>,
    /// Characteristics the entering object must have for this counter grant.
    pub object_filter: Option<ObjectFilter>,
    pub surface: BattlefieldEntryCounterSurface,
}

impl BattlefieldEntryCounterSpec {
    pub fn new(
        counter_type: crate::counter::CounterType,
        amount: impl Into<Value>,
        surface: BattlefieldEntryCounterSurface,
    ) -> Self {
        Self {
            counter_type,
            amount: amount.into(),
            condition: None,
            object_filter: None,
            surface,
        }
    }

    pub fn with_condition(mut self, condition: crate::value_model::Condition) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn for_matching_object(mut self, filter: ObjectFilter) -> Self {
        self.object_filter = Some(filter);
        self
    }
}

/// How a multi-card move to the top or bottom of a library is ordered.
///
/// The choosing player is explicit because the player performing the
/// instruction is not necessarily the controller of the effect (for example,
/// "that player puts the cards ... in any order").
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryPlacementOrder {
    Random,
    ChosenBy(PlayerFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveToZoneEffect {
    pub target: ChooseSpec,
    pub zone: crate::zone::Zone,
    pub to_top: bool,
    pub library_order: Option<LibraryPlacementOrder>,
    pub verb_surface: MoveToZoneVerbSurface,
    /// Whether the source text refers to the moved tagged result as a set
    /// (for example, "those cards" or "the exiled cards"). Tagged specs can
    /// resolve more than one object, but do not otherwise retain that surface.
    pub target_plural_surface: bool,
    /// Explicit player who performs the oracle instruction. The rules engine
    /// still moves the same objects to the same zones; this only preserves
    /// surfaces such as "that player puts" and "each player puts".
    pub actor_surface: Option<PlayerFilter>,
    /// Explicit contextual player named by the oracle destination (for
    /// example, "your graveyard" or "that player's hand"). Nonbattlefield
    /// zone changes still follow the rules and use the object's owner; this is
    /// retained only so compiled text can preserve an equivalent surface.
    pub destination_player_surface: Option<PlayerFilter>,
    pub destination_player_reference_surface: Option<DestinationPlayerReferenceSurface>,
    pub exiled_with_source_surface: Option<ExiledWithSourceMoveSurface>,
    pub battlefield_controller: BattlefieldController,
    /// Whether the oracle text explicitly named the battlefield controller.
    /// This affects presentation only; `battlefield_controller` remains the
    /// executable controller choice.
    pub controller_surface_explicit: bool,
    /// Counters that are part of this object's battlefield-entry event.
    pub enters_with_counters: Vec<BattlefieldEntryCounterSpec>,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
    pub attack_target_mode: Option<MoveToZoneAttackTargetMode>,
    pub enters_face_down: bool,
    /// Whether a transforming double-faced card enters with its back face up.
    /// This is part of the zone-change instruction, not a later transform action.
    pub enters_transformed: bool,
    pub transfer_exiled_with_source_links: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveToZoneAttackTargetMode {
    PlayerOrPlaneswalkerControlledBy(PlayerFilter),
}

impl MoveToZoneEffect {
    pub fn new(target: ChooseSpec, zone: crate::zone::Zone, to_top: bool) -> Self {
        Self {
            target,
            zone,
            to_top,
            library_order: None,
            verb_surface: MoveToZoneVerbSurface::Canonical,
            target_plural_surface: false,
            actor_surface: None,
            destination_player_surface: None,
            destination_player_reference_surface: None,
            exiled_with_source_surface: None,
            battlefield_controller: BattlefieldController::Preserve,
            controller_surface_explicit: false,
            enters_with_counters: Vec::new(),
            enters_tapped: false,
            enters_attacking: false,
            attack_target_mode: None,
            enters_face_down: false,
            enters_transformed: false,
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

    pub fn with_library_order(mut self, order: LibraryPlacementOrder) -> Self {
        self.library_order = Some(order);
        self
    }

    pub fn with_verb_surface(mut self, surface: MoveToZoneVerbSurface) -> Self {
        self.verb_surface = surface;
        self
    }

    pub fn with_target_plural_surface(mut self) -> Self {
        self.target_plural_surface = true;
        self
    }

    pub fn with_actor_surface(mut self, actor: PlayerFilter) -> Self {
        self.actor_surface = Some(actor);
        self
    }

    pub fn with_exiled_with_source_surface(mut self, surface: ExiledWithSourceMoveSurface) -> Self {
        self.exiled_with_source_surface = Some(surface);
        self
    }

    pub fn with_destination_player_surface(mut self, player: PlayerFilter) -> Self {
        self.destination_player_surface = Some(player);
        self
    }

    pub fn with_destination_player_reference_surface(
        mut self,
        surface: DestinationPlayerReferenceSurface,
    ) -> Self {
        self.destination_player_reference_surface = Some(surface);
        self
    }

    pub fn to_graveyard(target: ChooseSpec) -> Self {
        Self::new(target, crate::zone::Zone::Graveyard, false)
    }

    pub fn under_owner_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::Owner;
        self.controller_surface_explicit = true;
        self
    }

    pub fn transfer_exiled_with_source_links(mut self) -> Self {
        self.transfer_exiled_with_source_links = true;
        self
    }

    pub fn under_you_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::You;
        self.controller_surface_explicit = true;
        self
    }

    pub fn with_entry_counter(mut self, counter: BattlefieldEntryCounterSpec) -> Self {
        self.enters_with_counters.push(counter);
        self
    }

    pub fn tapped(mut self) -> Self {
        self.enters_tapped = true;
        self
    }

    pub fn attacking(mut self) -> Self {
        self.enters_attacking = true;
        self
    }

    pub fn attack_target_mode(mut self, mode: MoveToZoneAttackTargetMode) -> Self {
        self.enters_attacking = true;
        self.attack_target_mode = Some(mode);
        self
    }

    pub fn attacking_player_or_planeswalker_controlled_by(self, player: PlayerFilter) -> Self {
        self.attack_target_mode(
            MoveToZoneAttackTargetMode::PlayerOrPlaneswalkerControlledBy(player),
        )
    }

    pub fn face_down(mut self) -> Self {
        self.enters_face_down = true;
        self
    }

    pub fn transformed(mut self) -> Self {
        self.enters_transformed = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnAllToBattlefieldEffect {
    pub filter: ObjectFilter,
    pub tapped: bool,
    pub face_down: bool,
    pub battlefield_controller: BattlefieldController,
    /// Whether the oracle text explicitly named the battlefield controller.
    /// This affects presentation only; `battlefield_controller` remains the
    /// executable controller choice.
    pub controller_surface_explicit: bool,
    pub verb_surface: MoveToZoneVerbSurface,
}

impl ReturnAllToBattlefieldEffect {
    pub fn new(filter: ObjectFilter, tapped: bool) -> Self {
        Self {
            filter,
            tapped,
            face_down: false,
            battlefield_controller: BattlefieldController::Owner,
            controller_surface_explicit: false,
            verb_surface: MoveToZoneVerbSurface::Return,
        }
    }

    pub fn under_owner_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::Owner;
        self.controller_surface_explicit = true;
        self
    }

    pub fn under_you_control(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::You;
        self.controller_surface_explicit = true;
        self
    }

    pub fn under_you_control_implicitly(mut self) -> Self {
        self.battlefield_controller = BattlefieldController::You;
        self.controller_surface_explicit = false;
        self
    }

    pub fn face_down(mut self) -> Self {
        self.face_down = true;
        self
    }

    pub fn with_verb_surface(mut self, surface: MoveToZoneVerbSurface) -> Self {
        self.verb_surface = surface;
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

/// "Turn the exiled card face up." / "Turn it face up."
#[derive(Debug, Clone, PartialEq)]
pub struct TurnFaceUpEffect {
    pub target: ChooseSpec,
}

impl TurnFaceUpEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

/// "It becomes foretold. Its foretell cost is its mana cost reduced by {N}."
#[derive(Debug, Clone, PartialEq)]
pub struct BecomeForetoldEffect {
    pub target: ChooseSpec,
    /// Generic-mana reduction applied to the card's mana cost to form its
    /// foretell cost.
    pub cost_reduction: u32,
}

impl BecomeForetoldEffect {
    pub fn new(target: ChooseSpec, cost_reduction: u32) -> Self {
        Self {
            target,
            cost_reduction,
        }
    }
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
    pub aggregate_constraint: Option<ChoiceAggregateConstraint>,
    pub chooser: PlayerFilter,
    pub zone: Option<crate::zone::Zone>,
    pub additional_zones: Vec<crate::zone::Zone>,
    pub tag: crate::tag::TagKey,
    pub description: String,
    pub is_search: bool,
    pub reveal: bool,
    pub search_mode: SearchSelectionMode,
    pub top_only: bool,
    pub bottom_only: bool,
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
            aggregate_constraint: None,
            chooser,
            zone: None,
            additional_zones: Vec::new(),
            tag: tag.into(),
            description: "Choose".to_string(),
            is_search: false,
            reveal: false,
            search_mode: SearchSelectionMode::Exact,
            top_only: false,
            bottom_only: false,
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

    pub fn with_aggregate_constraint(mut self, constraint: ChoiceAggregateConstraint) -> Self {
        self.aggregate_constraint = Some(constraint);
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
        self.bottom_only = false;
        self
    }

    pub fn bottom_only(mut self) -> Self {
        self.bottom_only = true;
        self.top_only = false;
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
    pub next_end_step_player: PlayerFilter,
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
            next_end_step_player: PlayerFilter::Any,
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

    pub fn next_end_step_player(mut self, player: PlayerFilter) -> Self {
        self.next_end_step_player = player;
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
    pub x_value: Option<Value>,
}

impl PayManaEffect {
    pub fn new(cost: crate::mana::ManaCost, player: ChooseSpec) -> Self {
        Self {
            cost,
            player,
            x_value: None,
        }
    }

    pub fn with_x_value(mut self, x_value: Value) -> Self {
        self.x_value = Some(x_value);
        self
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
    pub generic_reduction: Option<Value>,
    pub applies_to_all_matching_this_turn: bool,
    pub duration: Until,
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
            generic_reduction: None,
            applies_to_all_matching_this_turn: false,
            duration: Until::EndOfTurn,
        }
    }

    pub fn all_matching_this_turn(
        player: PlayerFilter,
        filter: ObjectFilter,
        generic_reduction: impl Into<Value>,
    ) -> Self {
        Self {
            player,
            filter,
            reduction: crate::mana::ManaCost::new(),
            generic_reduction: Some(generic_reduction.into()),
            applies_to_all_matching_this_turn: true,
            duration: Until::EndOfTurn,
        }
    }

    pub fn all_matching_until(
        player: PlayerFilter,
        filter: ObjectFilter,
        generic_reduction: impl Into<Value>,
        duration: Until,
    ) -> Self {
        Self {
            player,
            filter,
            reduction: crate::mana::ManaCost::new(),
            generic_reduction: Some(generic_reduction.into()),
            applies_to_all_matching_this_turn: true,
            duration,
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
    /// Cloak uses the same face-down/top-card operation as manifest, but the
    /// resulting 2/2 creature also has ward {2}.
    pub cloak: bool,
}

/// Put an arbitrary collection of cards onto the battlefield face down as
/// manifested or cloaked creatures.
#[derive(Debug, Clone, PartialEq)]
pub struct ManifestObjectsEffect {
    pub target: ChooseSpec,
    pub controller: PlayerFilter,
    pub cloak: bool,
    pub tapped: bool,
    pub shuffle: bool,
}

impl ManifestObjectsEffect {
    pub fn new(target: ChooseSpec, controller: PlayerFilter) -> Self {
        Self {
            target,
            controller,
            cloak: false,
            tapped: false,
            shuffle: false,
        }
    }

    pub fn cloak(mut self) -> Self {
        self.cloak = true;
        self
    }

    pub fn tapped(mut self) -> Self {
        self.tapped = true;
        self
    }

    pub fn shuffled(mut self) -> Self {
        self.shuffle = true;
        self
    }
}

impl ManifestTopCardOfLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            cloak: false,
        }
    }

    pub fn cloak(player: PlayerFilter) -> Self {
        Self {
            player,
            cloak: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestCardFromHandEffect;

impl ManifestCardFromHandEffect {
    pub fn new() -> Self {
        Self
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
    pub duration: Until,
}

impl GoadEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self::with_duration(target, Until::YourNextTurn)
    }

    pub fn with_duration(target: ChooseSpec, duration: Until) -> Self {
        Self { target, duration }
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

/// Source-level placement of non-keyword abilities granted to a created token.
///
/// Token characteristics alone cannot distinguish `with "..."` from a later
/// `It has "..."` sentence, so the compiler carries that surface choice to the
/// renderer explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenAbilityPresentation {
    InlineWith,
    SeparateSentence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateTokenEffect<D> {
    pub token: D,
    pub count: Value,
    pub controller: PlayerFilter,
    pub controller_target: Option<ChooseSpec>,
    /// Whether the authored text explicitly named `you` as the actor of the
    /// create action. This is presentation-only; `controller` remains the
    /// semantic source of truth for who creates and controls the token.
    pub actor_surface_explicit: bool,
    pub suppress_aura_attachment_choice: bool,
    pub ability_presentation: Option<TokenAbilityPresentation>,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
    pub exile_at_end_of_combat: bool,
    pub sacrifice_at_end_of_combat: bool,
    pub sacrifice_at_next_end_step: bool,
    pub exile_at_next_end_step: bool,
    pub next_end_step_player: PlayerFilter,
}

impl<D> CreateTokenEffect<D> {
    pub fn new(token: D, count: impl Into<Value>, controller: PlayerFilter) -> Self {
        let count = count.into();
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
            actor_surface_explicit: false,
            suppress_aura_attachment_choice: false,
            ability_presentation: None,
            enters_tapped: false,
            enters_attacking: false,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
        }
    }

    pub fn you(token: D, count: impl Into<Value>) -> Self {
        Self::new(token, count, PlayerFilter::You)
    }

    pub fn one(token: D) -> Self {
        Self::you(token, 1)
    }

    pub fn with_explicit_actor_surface(mut self) -> Self {
        self.actor_surface_explicit = true;
        self
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

    pub fn with_ability_presentation(mut self, presentation: TokenAbilityPresentation) -> Self {
        self.ability_presentation = Some(presentation);
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

    pub fn next_end_step_player(mut self, player: PlayerFilter) -> Self {
        self.next_end_step_player = player;
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
            count: count.into(),
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
    /// Original quoted copiable ability text when the cleanup instruction was
    /// granted to the token as part of the copy exception.
    pub sacrifice_at_next_end_step_ability_text: Option<String>,
    pub exile_at_next_end_step: bool,
    pub next_end_step_player: PlayerFilter,
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
            sacrifice_at_next_end_step_ability_text: None,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
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

    pub fn sacrifice_at_next_end_step_ability_text(mut self, text: Option<String>) -> Self {
        self.sacrifice_at_next_end_step_ability_text = text;
        self
    }

    pub fn exile_at_next_end_step(mut self, value: bool) -> Self {
        self.exile_at_next_end_step = value;
        self
    }

    pub fn next_end_step_player(mut self, player: PlayerFilter) -> Self {
        self.next_end_step_player = player;
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

#[derive(Clone, PartialEq)]
pub struct TagMatchingObjectsEffect {
    pub filter: ObjectFilter,
    pub zone: Option<crate::zone::Zone>,
    pub additional_zones: Vec<crate::zone::Zone>,
    pub tag: crate::tag::TagKey,
    /// When nonempty, build the output from these existing tagged snapshots
    /// instead of rescanning the current game zones. This is used for unions
    /// of objects that were actually affected by preceding effects, including
    /// objects that have since changed zones.
    pub source_tags: Vec<crate::tag::TagKey>,
}

impl std::fmt::Debug for TagMatchingObjectsEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("TagMatchingObjectsEffect");
        debug
            .field("filter", &self.filter)
            .field("zone", &self.zone)
            .field("additional_zones", &self.additional_zones)
            .field("tag", &self.tag);
        if !self.source_tags.is_empty() {
            debug.field("source_tags", &self.source_tags);
        }
        debug.finish()
    }
}

impl TagMatchingObjectsEffect {
    pub fn new(filter: ObjectFilter, tag: impl Into<crate::tag::TagKey>) -> Self {
        Self {
            filter,
            zone: None,
            additional_zones: Vec::new(),
            tag: tag.into(),
            source_tags: Vec::new(),
        }
    }

    /// Use the union of existing tagged snapshots as this capture's source.
    /// The filter and zones remain descriptive metadata for later consumers
    /// and compiled-text rendering.
    pub fn from_tagged_sources(
        mut self,
        source_tags: impl IntoIterator<Item = crate::tag::TagKey>,
    ) -> Self {
        self.source_tags = source_tags.into_iter().collect();
        self
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
    /// Preserve the older two-sentence Oracle surface that explicitly says
    /// to return the exiled card when the source leaves the battlefield.
    pub explicit_return_surface: bool,
}

impl ExileUntilEffect {
    pub fn new(spec: ChooseSpec, duration: ExileUntilDuration) -> Self {
        Self {
            spec,
            duration,
            return_zone: crate::zone::Zone::Battlefield,
            face_down: false,
            explicit_return_surface: false,
        }
    }

    pub fn with_face_down(mut self, face_down: bool) -> Self {
        self.face_down = face_down;
        self
    }

    pub fn with_explicit_return_surface(mut self, explicit: bool) -> Self {
        self.explicit_return_surface = explicit;
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
    Players {
        filter: PlayerFilter,
        exclude_voter: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoteEffect<E> {
    pub choice: VoteChoice<E>,
    pub controller_extra_votes: u32,
    pub controller_optional_extra_votes: u32,
    pub secret: bool,
    /// Whether the vote starts with the effect's controller before proceeding
    /// in turn order. This is executable vote-order semantics, not merely a
    /// rendering hint.
    pub starting_with_controller: bool,
}

impl<E> VoteEffect<E> {
    pub fn new(options: Vec<VoteOption<E>>, controller_extra_votes: u32) -> Self {
        Self {
            choice: VoteChoice::NamedOptions(options),
            controller_extra_votes,
            controller_optional_extra_votes: 0,
            secret: false,
            starting_with_controller: false,
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
            starting_with_controller: false,
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
            starting_with_controller: false,
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
            starting_with_controller: false,
        }
    }

    pub fn vote_players(
        filter: PlayerFilter,
        exclude_voter: bool,
        controller_extra_votes: u32,
    ) -> Self {
        Self::vote_players_with_optional_extra(filter, exclude_voter, controller_extra_votes, 0)
    }

    pub fn vote_players_with_optional_extra(
        filter: PlayerFilter,
        exclude_voter: bool,
        controller_extra_votes: u32,
        controller_optional_extra_votes: u32,
    ) -> Self {
        Self {
            choice: VoteChoice::Players {
                filter,
                exclude_voter,
            },
            controller_extra_votes,
            controller_optional_extra_votes,
            secret: false,
            starting_with_controller: false,
        }
    }

    pub fn basic(options: Vec<VoteOption<E>>) -> Self {
        Self::new(options, 0)
    }

    pub fn councils_dilemma(options: Vec<VoteOption<E>>) -> Self {
        Self::with_optional_extra(options, 0, 1).starting_with_controller(true)
    }

    pub fn with_secret(mut self, secret: bool) -> Self {
        self.secret = secret;
        self
    }

    pub fn starting_with_controller(mut self, starting_with_controller: bool) -> Self {
        self.starting_with_controller = starting_with_controller;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecretChoiceEffect {
    pub options: Vec<String>,
    pub participants: Vec<PlayerFilter>,
    pub participant_target: Option<ChooseSpec>,
}

impl SecretChoiceEffect {
    pub fn new(options: Vec<String>, participants: Vec<PlayerFilter>) -> Self {
        let participant_target = participants.iter().find_map(|participant| {
            if let PlayerFilter::Target(inner) = participant {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            } else {
                None
            }
        });
        Self {
            options,
            participants,
            participant_target,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantTaggedSpellFreeCastUntilEndOfTurnEffect {
    pub tag: crate::tag::TagKey,
    pub player: PlayerFilter,
    pub duration: GrantPlayTaggedDuration,
    pub while_on_top_of_library: bool,
    pub zone: Option<crate::zone::Zone>,
}

impl GrantTaggedSpellFreeCastUntilEndOfTurnEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>, player: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            player,
            duration: GrantPlayTaggedDuration::UntilEndOfTurn,
            while_on_top_of_library: false,
            zone: Some(crate::zone::Zone::Exile),
        }
    }

    pub fn while_on_top_of_library(mut self) -> Self {
        self.while_on_top_of_library = true;
        self
    }

    pub fn for_as_long_as_exiled(mut self) -> Self {
        self.duration = GrantPlayTaggedDuration::ForAsLongAsExiled;
        self
    }

    pub fn from_current_zone(mut self) -> Self {
        self.zone = None;
        self
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
pub enum MayCastMatchingSpellPayment {
    WithoutPayingManaCost,
    AlternativeCost(AlternativeCastKind),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MayCastMatchingSpellWithoutPayingManaCostEffect {
    pub player: PlayerFilter,
    pub zone_owner: PlayerFilter,
    pub filter: ObjectFilter,
    pub zone: crate::Zone,
    pub payment: MayCastMatchingSpellPayment,
}

impl MayCastMatchingSpellWithoutPayingManaCostEffect {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, zone: crate::Zone) -> Self {
        Self {
            zone_owner: player.clone(),
            player,
            filter,
            zone,
            payment: MayCastMatchingSpellPayment::WithoutPayingManaCost,
        }
    }

    pub fn with_zone_owner(mut self, owner: PlayerFilter) -> Self {
        self.zone_owner = owner;
        self
    }

    pub fn with_alternative_cost(mut self, kind: AlternativeCastKind) -> Self {
        self.payment = MayCastMatchingSpellPayment::AlternativeCost(kind);
        self
    }
}
