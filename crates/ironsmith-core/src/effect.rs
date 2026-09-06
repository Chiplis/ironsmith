//! Shared effect-domain metadata.
//!
//! These types describe effect identity and selection cardinality without
//! pulling in the runtime execution engine.

use crate::tag::TagKeyWalk;

use crate::filter_model::{
    AlternativeCastKind, ObjectFilter, ObjectRef, PlayerFilter, StackObjectKind,
};
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
    /// Whether the source explicitly authored the otherwise semantic-neutral
    /// word "exactly" before a fixed or dynamic count.
    pub explicit_exactly: bool,
}

/// Distinguishes exact, optional, and "all matching" search instructions.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ChoiceAggregateMetric {
    Power,
    Toughness,
    ManaValue,
}

/// Upper bound on an aggregate characteristic of a group of chosen objects.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ChoiceAggregateConstraint {
    pub metric: ChoiceAggregateMetric,
    /// Optional lower bound on the aggregate selection. This is used by
    /// costs such as "collect evidence N", where any number of cards may be
    /// selected but their total mana value must reach a threshold.
    pub minimum: Option<Value>,
    pub maximum: Value,
}

impl ChoiceAggregateConstraint {
    pub fn at_most(metric: ChoiceAggregateMetric, maximum: impl Into<Value>) -> Self {
        Self {
            metric,
            minimum: None,
            maximum: maximum.into(),
        }
    }

    pub fn at_least(metric: ChoiceAggregateMetric, minimum: impl Into<Value>) -> Self {
        Self {
            metric,
            minimum: Some(minimum.into()),
            maximum: Value::Fixed(i32::MAX),
        }
    }

    pub fn total_power_at_most(maximum: impl Into<Value>) -> Self {
        Self::at_most(ChoiceAggregateMetric::Power, maximum)
    }

    pub fn total_mana_value_at_most(maximum: impl Into<Value>) -> Self {
        Self::at_most(ChoiceAggregateMetric::ManaValue, maximum)
    }

    pub fn total_mana_value_at_least(minimum: impl Into<Value>) -> Self {
        Self::at_least(ChoiceAggregateMetric::ManaValue, minimum)
    }
}

/// Object identity used by a predicate-bearing resolution duration.
///
/// Semantic references are materialized to `Specific` when the resolving
/// spell or ability creates its continuous effect. This keeps the duration's
/// operands distinct from both the effect source and the fixed affected set.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
pub enum ContinuousDurationObject {
    Source,
    AffectedObject,
    Tagged(TagKey),
    Specific(crate::ObjectId),
}

/// Player identity used by a predicate-bearing resolution duration.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Default)]
#[expect(
    clippy::large_enum_variant,
    reason = "continuous durations preserve typed predicates inline"
)]
#[derive(TagKeyWalk)]
pub enum Until {
    #[default]
    Forever,
    EndOfTurn,
    /// The earlier of the current turn ending or any player rolling the
    /// specified result. `matching_rolls_observed` is materialized when the
    /// continuous effect resolves so rolls that happened earlier in the turn
    /// do not immediately expire it.
    EndOfTurnOrAnyPlayerRolls {
        result: u32,
        matching_rolls_observed: u32,
    },
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "effect predicates preserve typed object filters inline"
)]
#[derive(TagKeyWalk)]
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
    /// At least one object affected for a matching player has the greatest
    /// mana value among every object affected by the producer. Equal maxima
    /// intentionally satisfy the predicate.
    ///
    /// The producer must retain per-player affected-object partitions. This
    /// is the generic result shape used by simultaneous participant actions
    /// such as comparing what each player discarded.
    PlayerAffectedObjectHasGreatestManaValue {
        player: PlayerFilter,
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum PriorEffectResultQuantifier {
    /// An ordinary singular result ("a creature card").
    One,
    /// An explicit nonzero plural result ("one or more nonland cards").
    OneOrMore,
    /// The action itself is the predicate and has no object noun surface.
    ActionOnly,
}

/// Typed presentation and filtering data for a `... this way` result gate.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct PriorEffectResultSurface {
    pub action: PriorEffectAction,
    /// Authored present-tense zone-change wording for an exile result gate.
    #[cfg_attr(feature = "serde", serde(default))]
    pub put_into_exile_surface: bool,
    pub filter: ObjectFilter,
    pub actor: PriorEffectResultActor,
    pub quantifier: PriorEffectResultQuantifier,
    /// Explicit number of matching result objects required by the predicate.
    ///
    /// This is distinct from `quantifier`: it retains counted surfaces such
    /// as "two nonland cards were milled this way" and is evaluated against
    /// the prior effect's affected-object memory.
    pub required_count: Option<u32>,
    /// Characteristic that at least one pair among the matching result objects
    /// must share, for predicates such as "two cards that share a color".
    pub shared_characteristic: Option<crate::ObjectCharacteristic>,
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
            put_into_exile_surface: false,
            filter,
            actor,
            quantifier,
            required_count: None,
            shared_characteristic: None,
        }
    }

    pub fn with_count_sharing(
        mut self,
        required_count: u32,
        characteristic: crate::ObjectCharacteristic,
    ) -> Self {
        self.required_count = Some(required_count);
        self.shared_characteristic = Some(characteristic);
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum GrantPlayTaggedDuration {
    UntilEndOfTurn,
    UntilYourNextTurnEnd,
    UntilYourNextEndStep,
    /// The permission remains active until the same source object next exiles
    /// another card.
    UntilSourceExilesAnother,
    ForAsLongAsExiled,
    ForAsLongAsYouControlSource,
}

/// Oracle-facing noun phrase for a temporary permission over a tagged card
/// collection. Runtime identity remains carried by the grant's `tag`; this
/// only preserves distinctions that cannot be recovered from that tag.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum GrantPlayTaggedObjectSurface {
    It,
    ThatCard,
    /// Authored definite reference that repeats the card's current zone.
    ThatCardFromExile,
    ThatSpell,
    Them,
    ThoseCards,
    SpellsFromAmongThoseCards,
    SpellsFromAmongThoseExiledCards,
    SpellFromAmongCardsExiledWithSource {
        creature_spell: bool,
        source: SourceReferenceSurface,
    },
}

/// Oracle-facing reference used by a flexible-mana suffix on a temporary
/// tagged-card permission.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum GrantPlayTaggedManaReferenceSurface {
    It,
    ThatSpell,
    Them,
    ThoseSpells,
}

/// Presentation provenance for a temporary tagged-card permission.
///
/// The duration and playable set remain typed by `GrantPlayTaggedEffect`.
/// These fields preserve only authored placement and reference wording.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[derive(TagKeyWalk)]
pub struct GrantPlayTaggedSurface {
    pub leading_duration: bool,
    pub object: Option<GrantPlayTaggedObjectSurface>,
    pub mana_reference: Option<GrantPlayTaggedManaReferenceSurface>,
    /// Authored source noun in "for as long as you control this ...".
    /// Runtime duration semantics remain carried by
    /// `GrantPlayTaggedDuration::ForAsLongAsYouControlSource`.
    pub control_source: Option<SourceReferenceSurface>,
    /// Authored source noun in "until you exile another card with this ...".
    /// The event-bounded lifetime itself is carried by
    /// `GrantPlayTaggedDuration::UntilSourceExilesAnother`.
    pub until_source_exiles_another: Option<SourceReferenceSurface>,
}

impl GrantPlayTaggedSurface {
    pub fn with_leading_duration(mut self, leading: bool) -> Self {
        self.leading_duration = leading;
        self
    }

    pub fn with_object(mut self, object: GrantPlayTaggedObjectSurface) -> Self {
        self.object = Some(object);
        self
    }

    pub fn with_mana_reference(mut self, reference: GrantPlayTaggedManaReferenceSurface) -> Self {
        self.mana_reference = Some(reference);
        self
    }

    pub fn with_control_source(mut self, source: SourceReferenceSurface) -> Self {
        self.control_source = Some(source);
        self
    }

    pub fn with_until_source_exiles_another(mut self, source: SourceReferenceSurface) -> Self {
        self.until_source_exiles_another = Some(source);
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ReplacementApplyMode {
    OneShot,
    UntilEndOfTurn,
    Resolution,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum PreventNextTimeDamageSource {
    Choice,
    ChoiceMatching(crate::filter_model::ObjectFilter),
    Target(ChooseSpec),
    Filter(crate::filter_model::ObjectFilter),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "damage-target constraints preserve typed choices inline"
)]
#[derive(TagKeyWalk)]
pub enum PreventNextTimeDamageTarget {
    AnyTarget,
    Omitted,
    You,
    Target(ChooseSpec),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum RedirectNextTimeDamageSource {
    Choice,
    Filter(crate::filter_model::ObjectFilter),
    Target(crate::target_model::ChooseSpec),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum RedirectNextTimeDamageDestination {
    SourceObject,
    Controller,
    SourceController,
    TargetObject,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "retarget modes preserve typed choice specifications inline"
)]
#[derive(TagKeyWalk)]
pub enum RetargetMode {
    All,
    OneToFixed(crate::target_model::ChooseSpec),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum DelayedTriggerSpec {
    /// An event-time condition qualifies the delayed event matcher, rather
    /// than becoming an intervening condition checked again on resolution.
    ConditionQualified {
        trigger: Box<DelayedTriggerSpec>,
        condition: crate::value_model::Condition,
        surface: String,
    },
    AsPermanentsUntap {
        player: PlayerFilter,
        source_must_be_controlled: bool,
    },
    BeginningOfUpkeep(PlayerFilter),
    BeginningOfDrawStep(PlayerFilter),
    BeginningOfEndStep(PlayerFilter),
    BeginningOfCleanupStep(PlayerFilter),
    /// The first cleanup step that begins after this delayed trigger is
    /// registered. The scheduling effect is one-shot; the player filter
    /// distinguishes "the next cleanup step" from "your next cleanup step".
    BeginningOfNextCleanupStep(PlayerFilter),
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
    ThisBlocksObject {
        filter: ObjectFilter,
        min_blocked_objects: Option<u32>,
    },
    ThisBecomesBlockedByObject(ObjectFilter),
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
    DealsCombatDamageToPlayerOneOrMore {
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
    PlayerDrawsCard(PlayerFilter),
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[derive(TagKeyWalk)]
pub enum DelayedTriggerDuration {
    #[default]
    Forever,
    EndOfTurn,
    EndOfCombat,
    UntilControllerNextTurn,
}

/// A cost that may be paid while a delayed trigger is pending to cancel that
/// registration before it fires.
///
/// This models clauses such as "unless they pay {1} before that draw step".
/// The runtime resolves `player` when the delayed trigger is registered so the
/// payment window remains valid even after the originating source changes
/// zones.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct DelayedTriggerPrepayment<E> {
    pub player: PlayerFilter,
    pub cost: crate::cost_model::TotalCost<crate::cost_model::Cost<E>>,
}

impl<E> DelayedTriggerPrepayment<E> {
    pub fn new(
        player: PlayerFilter,
        cost: crate::cost_model::TotalCost<crate::cost_model::Cost<E>>,
    ) -> Self {
        Self { player, cost }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ScheduleDelayedTriggerEffect<E> {
    pub trigger: DelayedTriggerSpec,
    pub effects: Vec<E>,
    pub one_shot: bool,
    pub start_next_turn: bool,
    pub duration: DelayedTriggerDuration,
    pub until_end_of_turn: bool,
    pub until_end_of_combat: bool,
    /// Preserve an authored duration before the trigger clause, such as
    /// "Until end of turn, whenever ...", instead of moving it to the event.
    pub leading_duration_surface: bool,
    pub watch_ability_source: bool,
    /// Capture every object target chosen for the resolving spell or ability
    /// and register one watcher per object.
    pub watch_all_object_targets: bool,
    /// Preserve the authored set-reference surface for a watched tagged set,
    /// such as "either of those creatures".
    pub either_of_watched_objects: bool,
    /// A collection-scoped lifetime: the registration expires once none of
    /// the captured objects under `tag` remain in `zone`.
    pub while_any_tagged_object_in_zone: Option<(crate::tag::TagKey, crate::zone::Zone)>,
    pub target_choices: Vec<ChooseSpec>,
    pub target_tag: Option<crate::tag::TagKey>,
    pub target_filter: Option<ObjectFilter>,
    pub controller: PlayerFilter,
    /// Optional payment window that cancels this delayed registration.
    pub prepayment: Option<DelayedTriggerPrepayment<E>>,
    /// Resolve numeric `... damage prevented this way` values from the
    /// prevention shield created immediately before this registration.
    ///
    /// The runtime captures the shield identity when this effect resolves;
    /// the delayed ability therefore observes the amount actually prevented,
    /// even after the shield itself is exhausted and removed.
    pub event_value_from_prior_prevention: bool,
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
            leading_duration_surface: false,
            watch_ability_source: false,
            watch_all_object_targets: false,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: None,
            target_choices,
            target_tag: None,
            target_filter: None,
            controller,
            prepayment: None,
            event_value_from_prior_prevention: false,
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

    pub fn unless_paid_before_trigger(
        mut self,
        player: PlayerFilter,
        cost: crate::cost_model::TotalCost<crate::cost_model::Cost<E>>,
    ) -> Self {
        self.prepayment = Some(DelayedTriggerPrepayment::new(player, cost));
        self
    }

    pub fn with_prior_prevention_event_value(mut self) -> Self {
        self.event_value_from_prior_prevention = true;
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

    pub fn with_leading_duration_surface(mut self) -> Self {
        self.leading_duration_surface = true;
        self
    }

    pub fn with_either_of_watched_objects_surface(mut self) -> Self {
        self.either_of_watched_objects = true;
        self
    }

    pub fn while_any_tagged_object_in_zone(
        mut self,
        tag: impl Into<crate::tag::TagKey>,
        zone: crate::zone::Zone,
    ) -> Self {
        self.while_any_tagged_object_in_zone = Some((tag.into(), zone));
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum SetQuantifierSurface {
    All,
    Each,
    /// A plural pronoun referring to an already established object result.
    ///
    /// The result tag/filter remains the executable identity; this records
    /// only that Oracle authored the follow-up subject as `they`.
    They,
    /// A plural demonstrative reference to a previously established set.
    ///
    /// This is presentation-only, but unlike `Each` it also records that
    /// lowering must reuse the antecedent set rather than build a new set from
    /// the demonstrative noun alone.
    Those,
}

/// Oracle surface used when a type-changing effect preserves an object's
/// existing types. Both variants have the same rules meaning, but they render
/// differently and must remain distinguishable after lowering.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum TypeRetentionSurface {
    InAdditionToOtherTypes,
    /// The effect adds the creature card type, but Oracle expresses the
    /// animation as a P/T plus creature subtype (for example, "a 1/1
    /// Spirit") without spelling out the `creature` noun.
    InAdditionToOtherTypesImplicitCreature,
    StillALand,
    /// Oracle names another retained card type inline (for example,
    /// "that's still a planeswalker"). The executable effect still adds its
    /// new types instead of replacing the object's existing types.
    StillACardType(CardType),
}

/// Oracle surface used to express the power and toughness portion of an
/// animation effect. Both forms create the same base-power/toughness layer,
/// but authored leading P/T ("a 4/4 Angel creature") must not be rewritten as
/// an explicit base-P/T clause ("an Angel creature with base power and
/// toughness 4/4").
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum AnimationPtSurface {
    LeadingPowerToughness,
    ExplicitBasePowerToughness,
}

/// Oracle placement of an animation effect's duration. Absence retains the
/// legacy trailing-duration surface, while this marker preserves authored
/// leading durations such as "Until end of turn, target land becomes ...".
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum AnimationDurationSurface {
    Leading,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "retarget restrictions preserve typed object filters inline"
)]
#[derive(TagKeyWalk)]
pub enum NewTargetRestriction {
    Player(PlayerFilter),
    Object(ObjectFilter),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum SharedTypeConstraint {
    CardType,
    PermanentType,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
            explicit_exactly: false,
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
            explicit_exactly: false,
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
            explicit_exactly: false,
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
            explicit_exactly: false,
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
            explicit_exactly: false,
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
            explicit_exactly: false,
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

    /// Preserve an explicitly authored `exactly` without changing the
    /// cardinality enforced by this choice.
    pub const fn with_explicit_exactly(mut self) -> Self {
        self.explicit_exactly = true;
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[derive(TagKeyWalk)]
pub struct NoteLifeTotalEffect;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct PutCountersEffect {
    pub counter_type: crate::counter::CounterType,
    pub amount: Value,
    pub target: ChooseSpec,
    pub target_count: Option<ChoiceCount>,
    pub distributed: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[derive(TagKeyWalk)]
pub enum ConditionalSurface {
    #[default]
    LeadingIf,
    TrailingIf,
    TrailingUnless,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct IfEffect<E> {
    pub condition: EffectId,
    pub predicate: EffectPredicate,
    pub then: Vec<E>,
    pub else_: Vec<E>,
    /// Evaluate this result predicate independently for every participant in
    /// the antecedent's `PlayerCounts` fact, binding that participant as the
    /// iterated player while executing the selected branch.
    pub per_player_result: bool,
    /// This prior-result branch was authored as a self-replacement of the
    /// default arm ("draw one; if ..., draw two instead"), rather than as an
    /// ordinary if/otherwise choice. Resolution semantics live in `then` and
    /// `else_`; this flag preserves the authored surface for rendering.
    pub prior_result_replacement_surface: bool,
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
            per_player_result: false,
            prior_result_replacement_surface: false,
        }
    }

    pub fn with_per_player_result(mut self, enabled: bool) -> Self {
        self.per_player_result = enabled;
        self
    }

    pub fn with_prior_result_replacement_surface(mut self, enabled: bool) -> Self {
        self.prior_result_replacement_surface = enabled;
        self
    }

    pub fn if_then(condition: EffectId, predicate: EffectPredicate, then: Vec<E>) -> Self {
        Self::new(condition, predicate, then, vec![])
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct EffectMode<E> {
    pub source_text: String,
    pub effects: Vec<E>,
}

/// A mode-selection range enabled by a later optional-cost announcement.
///
/// CR 601.4 allows an earlier mode choice to consider an optional cost that
/// will be chosen later in the same proposal (for example, kicker enabling
/// "choose any number instead").
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ChooseModeEffect<E> {
    pub modes: Vec<EffectMode<E>>,
    /// Typed effects authored once in the modal header after the choice
    /// instruction. They resolve exactly once after modes are chosen and
    /// before any selected mode resolves.
    pub common_prefix_effects: Vec<E>,
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
    /// Whether the selected mode labels are mandatory additional costs paid
    /// during the spell-casting transaction (for example, Spree or Tiered).
    pub spree: bool,
    /// Whether this costed modal uses the Tiered “choose exactly one” surface.
    pub tiered: bool,
    /// Additional mana cost associated with each mode. Ordinary modal effects
    /// leave this empty.
    pub mode_additional_mana_costs: Vec<ManaCost>,
    /// Number of trailing effects in every mode that were authored once as a
    /// shared sentence in the modal header. The effects remain inside each
    /// mode for target-scoped execution; this records only their common
    /// presentation boundary.
    pub common_suffix_effect_count: usize,
    pub disallow_previously_chosen_modes: bool,
    pub disallow_previously_chosen_modes_this_turn: bool,
    /// Each chosen mode must declare a player target different from every other chosen mode.
    pub distinct_player_targets_per_mode: bool,
    /// Alternate mode range that becomes legal if a later optional cost is announced.
    pub conditional_mode_range: Option<ConditionalModeRange>,
    /// Authored ability-word label for a modal spell. Triggered modal labels
    /// live on the enclosing triggered ability instead.
    pub presentation_label: Option<crate::ability_model::PresentationLabel>,
}

impl<E> ChooseModeEffect<E> {
    pub fn new(modes: Vec<EffectMode<E>>, min: Value, max: Value, allow_repeat: bool) -> Self {
        let choose_count = max.clone();
        let min_choose_count = min.clone();
        let mode_point_costs = vec![1; modes.len()];
        Self {
            modes,
            common_prefix_effects: Vec::new(),
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
            tiered: false,
            mode_additional_mana_costs: Vec::new(),
            common_suffix_effect_count: 0,
            disallow_previously_chosen_modes: false,
            disallow_previously_chosen_modes_this_turn: false,
            distinct_player_targets_per_mode: false,
            conditional_mode_range: None,
            presentation_label: None,
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

    pub fn with_tiered_mana_costs(mut self, costs: Vec<ManaCost>) -> Self {
        self.spree = true;
        self.tiered = true;
        self.min = Value::Fixed(1);
        self.max = Value::Fixed(1);
        self.min_choose_count = Value::Fixed(1);
        self.choose_count = Value::Fixed(1);
        self.allow_repeat = false;
        self.allow_repeated_modes = false;
        self.mode_additional_mana_costs = costs;
        self
    }

    pub fn with_common_suffix_effect_count(mut self, effect_count: usize) -> Self {
        self.common_suffix_effect_count = effect_count;
        self
    }

    pub fn with_common_prefix_effects(mut self, effects: Vec<E>) -> Self {
        self.common_prefix_effects = effects;
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

    pub fn with_presentation_label(
        mut self,
        label: crate::ability_model::PresentationLabel,
    ) -> Self {
        self.presentation_label = Some(label);
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct SearchLibrarySlot {
    pub filter: ObjectFilter,
    pub optional: bool,
}

/// Oracle-facing wording used to refer back to cards found by a library
/// search. This is presentation-only metadata; the searched objects are still
/// identified by the effect's typed tag.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[derive(TagKeyWalk)]
pub enum SearchResultReferenceSurface {
    #[default]
    ThatCard,
    TheCard,
    It,
    ThoseCards,
    Them,
}

impl SearchResultReferenceSurface {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ThatCard => "that card",
            Self::TheCard => "the card",
            Self::It => "it",
            Self::ThoseCards => "those cards",
            Self::Them => "them",
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct RevealTaggedEffect {
    pub tag: crate::tag::TagKey,
}

impl RevealTaggedEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[derive(TagKeyWalk)]
pub enum RevealSourceFromHandDuration {
    #[default]
    Momentary,
    UntilUpkeepEndsOrLeavesHand,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct CrewCostEffect {
    pub required_power: u32,
}

impl CrewCostEffect {
    pub fn new(required_power: u32) -> Self {
        Self { required_power }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Default)]
#[derive(TagKeyWalk)]
pub struct BecomeSaddledUntilEotEffect;

impl BecomeSaddledUntilEotEffect {
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ShuffleGraveyardIntoLibraryEffect {
    pub player: PlayerFilter,
    /// Preserve the longer authored "all cards from ... graveyard" surface.
    pub explicit_all_cards_from: bool,
}

impl ShuffleGraveyardIntoLibraryEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            explicit_all_cards_from: false,
        }
    }

    pub fn with_all_cards_from_surface(player: PlayerFilter) -> Self {
        Self {
            player,
            explicit_all_cards_from: true,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ExiledWithSourceReferenceSurface {
    Source(SourceReferenceSurface),
    It,
    Omitted,
}

/// Oracle-facing agreement for an owner-relative zone destination.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ExiledWithSourceDestinationSurface {
    ContextualPlayer,
    ItsOwner,
    TheirOwner,
    TheirOwners,
}

/// Oracle-facing verb used for a zone move of cards linked to the source.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ExiledWithSourceMoveVerbSurface {
    Put,
    Return,
}

/// Presentation metadata for a `put ... exiled with ... into ...` clause.
/// Object identity, source linkage, and the destination zone continue to live
/// in the ordinary filter and zone-move fields.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct ExiledWithSourceMoveSurface {
    pub verb: ExiledWithSourceMoveVerbSurface,
    pub subject: ExiledWithSourceSubjectSurface,
    pub source: ExiledWithSourceReferenceSurface,
    pub destination: ExiledWithSourceDestinationSurface,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ReturnToHandEffect {
    pub spec: ChooseSpec,
    /// Player explicitly presented as performing the return action. Runtime
    /// movement remains object-based; this preserves causative clauses such
    /// as "you may have that player return ...".
    pub actor_surface: Option<PlayerFilter>,
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
            actor_surface: None,
            destination_player_surface: None,
            exiled_with_source_surface: None,
            set_quantifier_surface: None,
            set_reference_surface: None,
        }
    }

    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
            actor_surface: None,
            destination_player_surface: None,
            exiled_with_source_surface: None,
            set_quantifier_surface: None,
            set_reference_surface: None,
        }
    }

    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
            actor_surface: None,
            destination_player_surface: None,
            exiled_with_source_surface: None,
            set_quantifier_surface: None,
            set_reference_surface: None,
        }
    }

    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
            actor_surface: None,
            destination_player_surface: None,
            exiled_with_source_surface: None,
            set_quantifier_surface: None,
            set_reference_surface: None,
        }
    }

    pub fn with_actor_surface(mut self, player: PlayerFilter) -> Self {
        self.actor_surface = Some(player);
        self
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct MoveToLibraryNthFromTopEffect {
    pub target: ChooseSpec,
    pub position: Value,
}

impl MoveToLibraryNthFromTopEffect {
    pub fn new(target: ChooseSpec, position: Value) -> Self {
        Self { target, position }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum BattlefieldController {
    Preserve,
    Owner,
    You,
}

/// Oracle wording used for a possessive player reference on a zone
/// destination. This is presentation-only; the associated player filter
/// remains the semantic destination antecedent.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum LibraryPlacementOrder {
    Random,
    ChosenBy(PlayerFilter),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
    /// Authored singular reference to a structurally tagged moved object.
    /// This is presentation-only: `target` remains the executable identity.
    pub target_reference_surface: Option<SearchResultReferenceSurface>,
    /// Authored provenance for a tagged-set complement disposition. This is
    /// presentation-only; the enclosing tagged iteration remains the
    /// executable definition of which objects move.
    pub remainder_surface: Option<LibraryRemainderSurface>,
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
            target_reference_surface: None,
            remainder_surface: None,
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

    pub fn with_target_reference_surface(mut self, surface: SearchResultReferenceSurface) -> Self {
        self.target_reference_surface = Some(surface);
        self
    }

    pub fn with_remainder_surface(mut self, surface: LibraryRemainderSurface) -> Self {
        self.remainder_surface = Some(surface);
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct RetainManaUntilEndOfTurnEffect {
    pub player: PlayerFilter,
}

/// "Turn the exiled card face up." / "Turn it face up."
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct TurnFaceUpEffect {
    pub target: ChooseSpec,
}

impl TurnFaceUpEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

/// "It becomes foretold. Its foretell cost is its mana cost reduced by {N}."
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ReorderLibraryTopEffect {
    pub tag: crate::tag::TagKey,
    pub chooser: PlayerFilter,
}

impl ReorderLibraryTopEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self {
            tag: tag.into(),
            chooser: PlayerFilter::You,
        }
    }

    pub fn chosen_by(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ForEachObject<E> {
    pub filter: ObjectFilter,
    pub effects: Vec<E>,
}

impl<E> ForEachObject<E> {
    pub fn new(filter: ObjectFilter, effects: Vec<E>) -> Self {
        Self { filter, effects }
    }
}

/// Runs a tagged producer once for each matching source object, retains the
/// source-to-result association, then runs a consumer once for each retained
/// pair after every producer iteration has completed.
///
/// This preserves two-phase Oracle instructions such as "for each ...,
/// create ..." followed by "each of those ... [acts on] a different one of
/// those ...". The explicit binding tags keep the consumer composable without
/// overloading the ordinary `__it__` iterator reference.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ForEachObjectCorrelatedResultEffect<E> {
    pub filter: ObjectFilter,
    pub producer_effects: Vec<E>,
    pub result_tag: crate::tag::TagKey,
    pub source_binding_tag: crate::tag::TagKey,
    pub result_binding_tag: crate::tag::TagKey,
    pub consumer_effects: Vec<E>,
}

impl<E> ForEachObjectCorrelatedResultEffect<E> {
    pub fn new(
        filter: ObjectFilter,
        producer_effects: Vec<E>,
        result_tag: impl Into<crate::tag::TagKey>,
        source_binding_tag: impl Into<crate::tag::TagKey>,
        result_binding_tag: impl Into<crate::tag::TagKey>,
        consumer_effects: Vec<E>,
    ) -> Self {
        Self {
            filter,
            producer_effects,
            result_tag: result_tag.into(),
            source_binding_tag: source_binding_tag.into(),
            result_binding_tag: result_binding_tag.into(),
            consumer_effects,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
    /// Authored reference used by the reveal clause for the searched set.
    /// This is separate from `search_result_reference_surface` because Oracle
    /// may reveal "them" and later move "those cards", or vice versa.
    pub search_reveal_reference_surface: Option<SearchResultReferenceSurface>,
    /// Authored reference used by the action that consumes a search result.
    /// `None` preserves the generic number-aware pronoun for hand-built
    /// choose/move sequences.
    pub search_result_reference_surface: Option<SearchResultReferenceSurface>,
    /// Whether a plural move to the top of the searched library explicitly
    /// included the presentation-only tail "in any order".
    pub search_top_in_any_order_surface: Option<bool>,
    pub top_only: bool,
    pub bottom_only: bool,
    pub replace_tagged_objects: bool,
    /// Persist an exact singular choice on the source for authored references
    /// from a later ability ("the chosen creature", for example).
    pub remember_as_chosen_object: bool,
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
            search_reveal_reference_surface: None,
            search_result_reference_surface: None,
            search_top_in_any_order_surface: None,
            top_only: false,
            bottom_only: false,
            replace_tagged_objects: false,
            remember_as_chosen_object: false,
        }
    }

    pub fn remember_as_chosen_object(mut self) -> Self {
        self.remember_as_chosen_object = true;
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

    pub fn with_search_result_reference_surface(
        mut self,
        surface: SearchResultReferenceSurface,
    ) -> Self {
        self.search_result_reference_surface = Some(surface);
        self
    }

    pub fn with_search_reveal_reference_surface(
        mut self,
        surface: Option<SearchResultReferenceSurface>,
    ) -> Self {
        self.search_reveal_reference_surface = surface;
        self
    }

    pub fn with_search_top_in_any_order_surface(mut self, explicit: bool) -> Self {
        self.search_top_in_any_order_surface = Some(explicit);
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct BecomeColorChoiceEffect {
    pub target: ChooseSpec,
    pub duration: Until,
    pub chooser: PlayerFilter,
    pub allow_multiple: bool,
}

impl BecomeColorChoiceEffect {
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self {
            target,
            duration,
            chooser: PlayerFilter::You,
            allow_multiple: false,
        }
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }

    pub fn with_multiple_colors(mut self, allow_multiple: bool) -> Self {
        self.allow_multiple = allow_multiple;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct PayManaEffect {
    pub cost: crate::mana::ManaCost,
    pub player: ChooseSpec,
    /// An exact value for printed X, computed from game state rather than
    /// chosen by the paying player.
    pub x_value: Option<Value>,
    /// An inclusive upper bound for printed X. The paying player chooses X
    /// from the affordable values between zero and this resolved maximum.
    pub x_maximum: Option<Value>,
}

impl PayManaEffect {
    pub fn new(cost: crate::mana::ManaCost, player: ChooseSpec) -> Self {
        Self {
            cost,
            player,
            x_value: None,
            x_maximum: None,
        }
    }

    pub fn with_x_value(mut self, x_value: Value) -> Self {
        self.x_value = Some(x_value);
        self
    }

    pub fn with_x_maximum(mut self, x_maximum: Value) -> Self {
        self.x_maximum = Some(x_maximum);
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

    pub fn next_matching_this_turn(
        player: PlayerFilter,
        filter: ObjectFilter,
        generic_reduction: impl Into<Value>,
    ) -> Self {
        Self {
            player,
            filter,
            reduction: crate::mana::ManaCost::new(),
            generic_reduction: Some(generic_reduction.into()),
            applies_to_all_matching_this_turn: false,
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct FightEffect {
    pub creature1: ChooseSpec,
    pub creature2: ChooseSpec,
    pub mutual_surface: bool,
}

impl FightEffect {
    pub fn new(creature1: ChooseSpec, creature2: ChooseSpec) -> Self {
        Self {
            creature1,
            creature2,
            mutual_surface: false,
        }
    }

    pub fn with_mutual_surface(mut self) -> Self {
        self.mutual_surface = true;
        self
    }

    pub fn you_vs_opponent() -> Self {
        Self::new(ChooseSpec::creature(), ChooseSpec::creature())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ExploreEffect {
    pub target: ChooseSpec,
}

impl ExploreEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Default)]
#[derive(TagKeyWalk)]
pub struct ManifestDreadEffect;

impl ManifestDreadEffect {
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ManifestTopCardOfLibraryEffect {
    pub player: PlayerFilter,
    /// Cloak uses the same face-down/top-card operation as manifest, but the
    /// resulting 2/2 creature also has ward {2}.
    pub cloak: bool,
}

/// Put an arbitrary collection of cards onto the battlefield face down as
/// manifested or cloaked creatures.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Default)]
#[derive(TagKeyWalk)]
pub struct ManifestCardFromHandEffect;

impl ManifestCardFromHandEffect {
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct SupportEffect {
    pub amount: u32,
}

impl SupportEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct AmplifyEffect {
    pub amount: u32,
}

impl AmplifyEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct DevourEffect {
    pub multiplier: u32,
}

impl DevourEffect {
    pub fn new(multiplier: u32) -> Self {
        Self { multiplier }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct DetainEffect {
    pub target: ChooseSpec,
}

impl DetainEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct SuspectEffect {
    pub target: ChooseSpec,
}

impl SuspectEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Default)]
#[derive(TagKeyWalk)]
pub struct OpenAttractionEffect {
    pub reminder: bool,
}

impl OpenAttractionEffect {
    pub fn new() -> Self {
        Self { reminder: false }
    }

    pub fn with_reminder(mut self, reminder: bool) -> Self {
        self.reminder = reminder;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct AdaptEffect {
    pub amount: u32,
}

impl AdaptEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ClashOpponentMode {
    AnyOpponent,
    TargetOpponent,
    DefendingPlayer,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct EarthbendEffect {
    pub target: ChooseSpec,
    pub counters: u32,
}

impl EarthbendEffect {
    pub fn new(target: ChooseSpec, counters: u32) -> Self {
        Self { target, counters }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum TaggedLeavesAbilitySource {
    WatchedObject,
    CurrentSource,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum TokenAbilityPresentation {
    InlineWith,
    SeparateSentence,
    SeparateSentenceGain,
    /// The token had no intrinsic abilities before the authored `It has ...`
    /// sentence, so all grouped abilities belong to that single sentence.
    SeparateSentenceCombined,
    /// Gain-verb counterpart of `SeparateSentenceCombined`.
    SeparateSentenceGainCombined,
    InlineWithThenStandalone(usize),
    SeparateSentenceThenStandalone(usize),
    SeparateSentenceGainThenStandalone(usize),
    SeparateSentenceCombinedThenStandalone(usize),
    SeparateSentenceGainCombinedThenStandalone(usize),
    Standalone(usize),
}

impl TokenAbilityPresentation {
    /// Mark an authored post-create ability sentence as containing every
    /// ability currently present on an otherwise ability-free token.
    pub const fn combined_separate_sentence(self) -> Self {
        match self {
            Self::SeparateSentence => Self::SeparateSentenceCombined,
            Self::SeparateSentenceGain => Self::SeparateSentenceGainCombined,
            other => other,
        }
    }

    /// Record another authored ability sentence after the token-definition
    /// sentence. The count is presentation-only; all abilities remain on the
    /// token definition for execution.
    pub const fn with_added_standalone_tail(current: Option<Self>) -> Self {
        match current {
            Some(Self::InlineWith) => Self::InlineWithThenStandalone(1),
            Some(Self::SeparateSentence) => Self::SeparateSentenceThenStandalone(1),
            Some(Self::SeparateSentenceGain) => Self::SeparateSentenceGainThenStandalone(1),
            Some(Self::SeparateSentenceCombined) => Self::SeparateSentenceCombinedThenStandalone(1),
            Some(Self::SeparateSentenceGainCombined) => {
                Self::SeparateSentenceGainCombinedThenStandalone(1)
            }
            Some(Self::InlineWithThenStandalone(count)) => {
                Self::InlineWithThenStandalone(count + 1)
            }
            Some(Self::SeparateSentenceThenStandalone(count)) => {
                Self::SeparateSentenceThenStandalone(count + 1)
            }
            Some(Self::SeparateSentenceGainThenStandalone(count)) => {
                Self::SeparateSentenceGainThenStandalone(count + 1)
            }
            Some(Self::SeparateSentenceCombinedThenStandalone(count)) => {
                Self::SeparateSentenceCombinedThenStandalone(count + 1)
            }
            Some(Self::SeparateSentenceGainCombinedThenStandalone(count)) => {
                Self::SeparateSentenceGainCombinedThenStandalone(count + 1)
            }
            Some(Self::Standalone(count)) => Self::Standalone(count + 1),
            None => Self::Standalone(1),
        }
    }

    pub const fn standalone_tail_count(self) -> usize {
        match self {
            Self::InlineWithThenStandalone(count)
            | Self::SeparateSentenceThenStandalone(count)
            | Self::SeparateSentenceGainThenStandalone(count)
            | Self::SeparateSentenceCombinedThenStandalone(count)
            | Self::SeparateSentenceGainCombinedThenStandalone(count)
            | Self::Standalone(count) => count,
            Self::InlineWith
            | Self::SeparateSentence
            | Self::SeparateSentenceGain
            | Self::SeparateSentenceCombined
            | Self::SeparateSentenceGainCombined => 0,
        }
    }

    pub const fn grouped_presentation(self) -> Option<Self> {
        match self {
            Self::InlineWith | Self::InlineWithThenStandalone(_) => Some(Self::InlineWith),
            Self::SeparateSentence | Self::SeparateSentenceThenStandalone(_) => {
                Some(Self::SeparateSentence)
            }
            Self::SeparateSentenceGain | Self::SeparateSentenceGainThenStandalone(_) => {
                Some(Self::SeparateSentenceGain)
            }
            Self::SeparateSentenceCombined | Self::SeparateSentenceCombinedThenStandalone(_) => {
                Some(Self::SeparateSentenceCombined)
            }
            Self::SeparateSentenceGainCombined
            | Self::SeparateSentenceGainCombinedThenStandalone(_) => {
                Some(Self::SeparateSentenceGainCombined)
            }
            Self::Standalone(_) => None,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct CreateTokenEffect<D> {
    pub token: D,
    pub count: Value,
    pub controller: PlayerFilter,
    pub controller_target: Option<ChooseSpec>,
    /// Apply the source permanent's chosen color to the token definition as
    /// the token is created.
    pub use_source_chosen_color: bool,
    /// Apply the source permanent's chosen creature type to the token
    /// definition as the token is created.
    pub use_source_chosen_creature_type: bool,
    /// Whether the authored text explicitly named `you` as the actor of the
    /// create action. This is presentation-only; `controller` remains the
    /// semantic source of truth for who creates and controls the token.
    pub actor_surface_explicit: bool,
    pub suppress_aura_attachment_choice: bool,
    pub ability_presentation: Option<TokenAbilityPresentation>,
    pub enters_tapped: bool,
    pub enters_attacking: bool,
    /// When present, each created token enters attacking this player (or a
    /// legal planeswalker they control, depending on the mode) instead of
    /// copying the originating attacker's target.
    pub attack_target_mode: Option<CopyAttackTargetMode>,
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
            use_source_chosen_color: false,
            use_source_chosen_creature_type: false,
            actor_surface_explicit: false,
            suppress_aura_attachment_choice: false,
            ability_presentation: None,
            enters_tapped: false,
            enters_attacking: false,
            attack_target_mode: None,
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

    pub fn with_source_chosen_color(mut self) -> Self {
        self.use_source_chosen_color = true;
        self
    }

    pub fn with_source_chosen_creature_type(mut self) -> Self {
        self.use_source_chosen_creature_type = true;
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

    pub fn attack_target_mode(mut self, mode: CopyAttackTargetMode) -> Self {
        self.enters_attacking = true;
        self.attack_target_mode = Some(mode);
        self
    }

    pub fn attacking_player(mut self, player: PlayerFilter) -> Self {
        self.enters_attacking = true;
        self.attack_target_mode = Some(CopyAttackTargetMode::Player(player));
        self
    }

    pub fn attacking_player_or_planeswalker_controlled_by(mut self, player: PlayerFilter) -> Self {
        self.enters_attacking = true;
        self.attack_target_mode = Some(CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
            player,
        ));
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum CopyPtAdjustment {
    HalfRoundUp,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum CopyAttackTargetMode {
    Player(PlayerFilter),
    PlayerOrPlaneswalkerControlledBy(PlayerFilter),
}

/// Authored anaphor used by a sentence that follows a token-copy action.
///
/// `They` is grammatical-role aware: renderers use `they` as a subject and
/// `them` as an object. Keeping this typed lets the semantic model continue to
/// fold token follow-ups into the copy effect without losing their surface.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum TokenCopyReferenceSurface {
    It,
    They,
    ThatToken,
    ThoseTokens,
    TheToken,
    TheTokens,
    TokenCreatedThisWay,
    TokensCreatedThisWay,
}

impl TokenCopyReferenceSurface {
    pub fn is_plural(self) -> bool {
        matches!(
            self,
            Self::They | Self::ThoseTokens | Self::TheTokens | Self::TokensCreatedThisWay
        )
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct CreateTokenCopyEffect<A> {
    pub target: ChooseSpec,
    pub count: Value,
    pub controller: PlayerFilter,
    pub enters_tapped: bool,
    pub has_haste: bool,
    /// `None` means haste was authored as an inline copy exception. `Some`
    /// records the subject of a separate post-create haste sentence.
    pub haste_followup_reference_surface: Option<TokenCopyReferenceSurface>,
    pub enters_attacking: bool,
    /// The entry state was authored as a separate token-followup sentence.
    pub entry_tapped_attacking_followup: bool,
    pub attack_target_mode: Option<CopyAttackTargetMode>,
    pub exile_at_end_of_combat: bool,
    pub exile_at_end_of_combat_reference_surface: Option<TokenCopyReferenceSurface>,
    /// "except it has haste and loses soulbond" (Mirage Phalanx): the copy is
    /// created without the soulbond pairing ability.
    pub loses_soulbond: bool,
    pub sacrifice_at_next_end_step: bool,
    pub sacrifice_at_next_end_step_reference_surface: Option<TokenCopyReferenceSurface>,
    /// Original quoted copiable ability text when the cleanup instruction was
    /// granted to the token as part of the copy exception.
    pub sacrifice_at_next_end_step_ability_text: Option<String>,
    pub exile_at_next_end_step: bool,
    pub exile_at_next_end_step_reference_surface: Option<TokenCopyReferenceSurface>,
    pub next_end_step_player: PlayerFilter,
    pub pt_adjustment: Option<CopyPtAdjustment>,
    pub clear_mana_cost: bool,
    pub added_card_types: Vec<CardType>,
    pub added_subtypes: Vec<Subtype>,
    pub removed_supertypes: Vec<Supertype>,
    pub set_base_power_toughness: Option<(i32, i32)>,
    /// Dynamic copiable base power/toughness values evaluated as the token is
    /// created. This is distinct from a later continuous-effect modification:
    /// the resolved values are part of the token's copy exception.
    pub set_base_power_toughness_value: Option<(Value, Value)>,
    /// Explicit starting loyalty authored as part of the copy exception.
    /// This replaces both the copied base loyalty and the initial loyalty
    /// counters on the created token.
    pub starting_loyalty: Option<u32>,
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
            haste_followup_reference_surface: None,
            enters_attacking: false,
            entry_tapped_attacking_followup: false,
            attack_target_mode: None,
            exile_at_end_of_combat: false,
            exile_at_end_of_combat_reference_surface: None,
            loses_soulbond: false,
            sacrifice_at_next_end_step: false,
            sacrifice_at_next_end_step_reference_surface: None,
            sacrifice_at_next_end_step_ability_text: None,
            exile_at_next_end_step: false,
            exile_at_next_end_step_reference_surface: None,
            next_end_step_player: PlayerFilter::Any,
            pt_adjustment: None,
            clear_mana_cost: false,
            added_card_types: Vec::new(),
            added_subtypes: Vec::new(),
            removed_supertypes: Vec::new(),
            set_base_power_toughness: None,
            set_base_power_toughness_value: None,
            starting_loyalty: None,
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

    pub fn haste_followup_reference_surface(
        mut self,
        surface: Option<TokenCopyReferenceSurface>,
    ) -> Self {
        self.haste_followup_reference_surface = surface;
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

    pub fn attacking_player(mut self, player: PlayerFilter) -> Self {
        self.enters_attacking = true;
        self.attack_target_mode = Some(CopyAttackTargetMode::Player(player));
        self
    }

    pub fn exile_at_eoc(mut self, value: bool) -> Self {
        self.exile_at_end_of_combat = value;
        self
    }

    pub fn exile_at_end_of_combat_reference_surface(
        mut self,
        surface: Option<TokenCopyReferenceSurface>,
    ) -> Self {
        self.exile_at_end_of_combat_reference_surface = surface;
        self
    }

    pub fn loses_soulbond(mut self, value: bool) -> Self {
        self.loses_soulbond = value;
        self
    }

    pub fn sacrifice_at_next_end_step(mut self, value: bool) -> Self {
        self.sacrifice_at_next_end_step = value;
        self
    }

    pub fn sacrifice_at_next_end_step_reference_surface(
        mut self,
        surface: Option<TokenCopyReferenceSurface>,
    ) -> Self {
        self.sacrifice_at_next_end_step_reference_surface = surface;
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

    pub fn exile_at_next_end_step_reference_surface(
        mut self,
        surface: Option<TokenCopyReferenceSurface>,
    ) -> Self {
        self.exile_at_next_end_step_reference_surface = surface;
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
        self.set_base_power_toughness_value = None;
        self
    }

    pub fn set_base_power_toughness_value(
        mut self,
        power: impl Into<Value>,
        toughness: impl Into<Value>,
    ) -> Self {
        self.set_base_power_toughness = None;
        self.set_base_power_toughness_value = Some((power.into(), toughness.into()));
        self
    }

    pub fn starting_loyalty(mut self, loyalty: u32) -> Self {
        self.starting_loyalty = Some(loyalty);
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct RetargetStackObjectEffect {
    pub target: ChooseSpec,
    pub mode: RetargetMode,
    pub chooser: PlayerFilter,
    pub require_change: bool,
    pub new_target_restriction: Option<NewTargetRestriction>,
    /// The authored back-reference named a plural set of spell or ability
    /// copies ("the copies") rather than one copy.
    ///
    /// The target tag preserves identity but cannot preserve this surface:
    /// one delayed copy instruction may produce copies over several trigger
    /// events even when its per-event copy count is one.
    pub copy_reference_plural: bool,
}

impl RetargetStackObjectEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            mode: RetargetMode::All,
            chooser: PlayerFilter::You,
            require_change: false,
            new_target_restriction: None,
            copy_reference_plural: false,
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

    pub fn with_plural_copy_reference(mut self) -> Self {
        self.copy_reference_plural = true;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ExileEffect {
    pub spec: ChooseSpec,
    pub face_down: bool,
    /// Preserve an authored instruction to turn an already face-down object
    /// face up as it is exiled. Exile's rules semantics already expose the
    /// card; this flag retains only the explicit action surface.
    pub turn_face_up: bool,
}

impl ExileEffect {
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self {
            spec,
            face_down: false,
            turn_face_up: false,
        }
    }

    pub fn with_face_down(mut self, face_down: bool) -> Self {
        self.face_down = face_down;
        self
    }

    pub fn turn_face_up(mut self) -> Self {
        self.turn_face_up = true;
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ExileUntilDuration {
    SourceLeavesBattlefield,
    /// Return the exiled object the next time a player who is an opponent of
    /// the effect's controller becomes the monarch.
    OpponentBecomesMonarch,
    NextEndStep,
    EndOfCombat,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct ExileUntilEffect {
    pub spec: ChooseSpec,
    pub duration: ExileUntilDuration,
    /// A separately targeted permanent whose departure ends the duration.
    /// `None` keeps the traditional ability-source watcher.
    pub leave_watcher: Option<ChooseSpec>,
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
            leave_watcher: None,
            return_zone: crate::zone::Zone::Battlefield,
            face_down: false,
            explicit_return_surface: false,
        }
    }

    pub fn with_face_down(mut self, face_down: bool) -> Self {
        self.face_down = face_down;
        self
    }

    pub fn with_leave_watcher(mut self, watcher: ChooseSpec) -> Self {
        self.leave_watcher = Some(watcher);
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

/// Authored presentation of a spell-copy amount whose executable value is
/// stored independently in [`CopySpellEffect::count`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum CopyCountSurface {
    OncePlusAdditionalPerOpponentWhoCopiedThisWay,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct CopySpellEffect {
    pub target: ChooseSpec,
    /// The kind named by an authored stack-object back-reference.
    ///
    /// Tagged references intentionally carry identity only, so a phrase such
    /// as "that spell", "that ability", or "that spell or ability" needs
    /// separate typed provenance for compiled-text rendering.
    pub target_reference_kind: Option<StackObjectKind>,
    /// Whether the authored target back-reference was the pronoun `it`.
    ///
    /// Reference resolution may replace the pronoun's internal tag with the
    /// triggering stack-object tag. This independent surface bit preserves
    /// the authored wording without weakening that semantic identity.
    pub target_reference_pronoun: bool,
    pub count: Value,
    pub count_surface: Option<CopyCountSurface>,
    pub copier: PlayerFilter,
    pub removed_supertypes: Vec<Supertype>,
    /// Colors set as part of the copy effect's copiable values.
    pub set_colors: Option<ColorSet>,
    /// Card types added as part of the copy effect's copiable values.
    pub added_card_types: Vec<CardType>,
    /// Subtypes added as part of the copy effect's copiable values.
    pub added_subtypes: Vec<Subtype>,
    /// Base power/toughness set as part of the copy effect's copiable values.
    pub set_base_power_toughness: Option<(i32, i32)>,
}

impl CopySpellEffect {
    fn target_stack_kind(target: &ChooseSpec) -> Option<StackObjectKind> {
        match target {
            ChooseSpec::SurfaceHinted { spec, .. }
            | ChooseSpec::Target(spec)
            | ChooseSpec::WithCount(spec, _)
            | ChooseSpec::WithCountValue(spec, _, _) => Self::target_stack_kind(spec),
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter.stack_kind,
            _ => None,
        }
    }

    pub fn new(target: ChooseSpec, count: impl Into<Value>) -> Self {
        Self::new_for_player(target, count, PlayerFilter::You)
    }

    pub fn new_for_player(
        target: ChooseSpec,
        count: impl Into<Value>,
        copier: PlayerFilter,
    ) -> Self {
        // A bare stack-object filter in a copy instruction is the target of
        // that instruction. Identity references (source/tagged/iterated) and
        // set-valued `All` specs remain non-targeting, as required by copy
        // triggers and bulk-copy effects.
        let target = if matches!(target.base(), ChooseSpec::Object(filter) if filter.stack_kind.is_some())
        {
            ChooseSpec::target(target)
        } else {
            target
        };
        let target_reference_kind = Self::target_stack_kind(&target);
        Self {
            target,
            target_reference_kind,
            target_reference_pronoun: false,
            count: count.into(),
            count_surface: None,
            copier,
            removed_supertypes: Vec::new(),
            set_colors: None,
            added_card_types: Vec::new(),
            added_subtypes: Vec::new(),
            set_base_power_toughness: None,
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

    pub fn with_set_colors(mut self, colors: Option<ColorSet>) -> Self {
        self.set_colors = colors;
        self
    }

    pub fn with_added_card_types(mut self, card_types: Vec<CardType>) -> Self {
        for card_type in card_types {
            if !self.added_card_types.contains(&card_type) {
                self.added_card_types.push(card_type);
            }
        }
        self
    }

    pub fn with_added_subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        for subtype in subtypes {
            if !self.added_subtypes.contains(&subtype) {
                self.added_subtypes.push(subtype);
            }
        }
        self
    }

    pub fn with_set_base_power_toughness(mut self, value: Option<(i32, i32)>) -> Self {
        self.set_base_power_toughness = value;
        self
    }

    pub fn has_characteristic_modifiers(&self) -> bool {
        self.set_colors.is_some()
            || !self.added_card_types.is_empty()
            || !self.added_subtypes.is_empty()
            || self.set_base_power_toughness.is_some()
    }

    pub fn with_count_surface(mut self, surface: CopyCountSurface) -> Self {
        self.count_surface = Some(surface);
        self
    }

    pub fn with_target_reference_kind(mut self, kind: StackObjectKind) -> Self {
        self.target_reference_kind = Some(kind);
        self
    }

    pub fn with_optional_target_reference_kind(mut self, kind: Option<StackObjectKind>) -> Self {
        if let Some(kind) = kind {
            self.target_reference_kind = Some(kind);
        }
        self
    }

    pub fn with_target_reference_pronoun(mut self, pronoun: bool) -> Self {
        self.target_reference_pronoun = pronoun;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "vote choices preserve typed object filters inline"
)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct SecretObjectChoice {
    /// Objects each participant may choose. Relative player filters such as
    /// `IteratedPlayer` are evaluated for that participant.
    pub filter: ObjectFilter,
    /// Number of objects each participant chooses.
    pub count: ChoiceCount,
    /// Shared result-set tag populated only after every participant has made
    /// their hidden selection.
    pub tag: TagKey,
    /// Whether the authored procedure immediately reveals the selections.
    pub reveal_after_choice: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct SecretChoiceEffect {
    pub options: Vec<String>,
    pub participants: Vec<PlayerFilter>,
    pub participant_target: Option<ChooseSpec>,
    pub object_choice: Option<SecretObjectChoice>,
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
            object_choice: None,
        }
    }

    pub fn new_objects(participants: Vec<PlayerFilter>, object_choice: SecretObjectChoice) -> Self {
        let participant_target = participants.iter().find_map(|participant| {
            if let PlayerFilter::Target(inner) = participant {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            } else {
                None
            }
        });
        Self {
            options: Vec::new(),
            participants,
            participant_target,
            object_choice: Some(object_choice),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum MayCastMatchingSpellPayment {
    WithoutPayingManaCost,
    AlternativeCost(AlternativeCastKind),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
