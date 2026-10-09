//! Effect execution engine for MTG.
//!
//! This module provides the runtime execution of effects, including:
//! - Value resolution (X, counts, power/toughness, etc.)
//! - Target validation
//! - Effect execution with proper game state mutations

use std::collections::{HashMap, HashSet};

use crate::color::Color;
use crate::cost::OptionalCostsPaid;
use crate::decision::DecisionMaker;
use crate::effect::{EffectId, EffectOutcome};
use crate::effects::{SecretChoiceResult, VoteResult};
use crate::events::cause::EventCause;
use crate::game_state::{GameState, TargetAssignment, TargetDistribution};
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::replacement::{ReplacementEffect, ReplacementEffectId, ReplacementEffectKey};
use crate::snapshot::ObjectSnapshot;
use crate::tag::{SOURCE_EXILED_TAG, TagKey};
use crate::target::{ChooseSpec, FilterContext};
use crate::types::Subtype;

/// An optional public reveal whose legality depends on a private identity.
/// The enclosing conditional installs this only for the first reveal offer;
/// MayEffect consumes it before executing any children.
#[derive(Debug, Clone)]
pub(crate) struct OptionalIdentityGuard {
    pub object: ObjectId,
    pub filter: crate::target::ObjectFilter,
    pub filter_ctx: FilterContext,
    pub can_accept: bool,
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during effect execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    /// Target is invalid or no longer exists.
    InvalidTarget,
    /// CR 801 suppresses this effect instruction because its resolved subject
    /// is outside the resolving source controller's range of influence.
    OutOfRange,
    /// Could not resolve a value (e.g., X not set).
    UnresolvableValue(String),
    /// Effect is impossible to execute in current state.
    Impossible(String),
    /// The CR names an action but delegates its procedure to an external rules document/profile.
    ExternalRulesProfileRequired {
        action: &'static str,
        specification: &'static str,
    },
    /// Referenced player does not exist.
    PlayerNotFound(PlayerId),
    /// Referenced object does not exist.
    ObjectNotFound(ObjectId),
    /// Referenced effect ID not found in context.
    EffectNotFound(EffectId),
    /// Referenced tag not found in context (object not tagged by prior effect).
    TagNotFound(String),
    /// Continuous-effect discovery could not establish a complete snapshot.
    ContinuousDiscovery(crate::static_ability_processor::StaticEffectDiscoveryError),
    /// Internal error (should not happen).
    InternalError(String),
    /// The host could not compute the complete operation within its resource
    /// profile. This is not an impossible Magic action or a neutral outcome.
    ResourceLimitExceeded {
        resource: &'static str,
        requested: u128,
        maximum: u128,
    },
    ResourceAllocationFailed {
        resource: &'static str,
        requested: usize,
    },
    /// A query cannot preselect another player's required decision. Native
    /// execution can request it; absence of an answer is not payment failure.
    UnresolvedPlayerDecision {
        player: PlayerId,
        decision: &'static str,
    },
    /// Required retained facts are absent or inconsistent. Boolean queries
    /// must preserve this as incomplete execution, never infer false or zero.
    IncompleteEvidence(String),
}

impl ExecutionError {
    pub fn is_resource_exhaustion(&self) -> bool {
        matches!(
            self,
            Self::ResourceLimitExceeded { .. } | Self::ResourceAllocationFailed { .. }
        )
    }
    /// An incomplete engine calculation must survive boolean affordability
    /// adapters; it is not proof that the Magic payment is impossible.
    pub fn is_incomplete_execution(&self) -> bool {
        self.is_resource_exhaustion()
            || matches!(
                self,
                Self::ContinuousDiscovery(_)
                    | Self::UnresolvedPlayerDecision { .. }
                    | Self::IncompleteEvidence(_)
            )
    }
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::ResourceLimitExceeded {
                resource,
                requested,
                maximum,
            } => write!(
                f,
                "Incomplete execution: {resource} requires {requested}, host limit is {maximum}"
            ),
            ExecutionError::ResourceAllocationFailed {
                resource,
                requested,
            } => write!(
                f,
                "Incomplete execution: allocator could not reserve {requested} items for {resource}"
            ),
            ExecutionError::UnresolvedPlayerDecision { player, decision } => write!(
                f,
                "Incomplete calculation: {decision} requires a decision from player {:?}",
                player
            ),
            ExecutionError::IncompleteEvidence(message) => {
                write!(f, "Incomplete retained evidence: {message}")
            }
            ExecutionError::InvalidTarget => write!(f, "Invalid target"),
            ExecutionError::OutOfRange => write!(f, "Subject is outside range of influence"),
            ExecutionError::UnresolvableValue(msg) => write!(f, "Cannot resolve value: {}", msg),
            ExecutionError::Impossible(msg) => write!(f, "Effect impossible: {}", msg),
            ExecutionError::ExternalRulesProfileRequired {
                action,
                specification,
            } => write!(
                f,
                "{action} requires an enabled external rules profile defined by {specification}"
            ),
            ExecutionError::PlayerNotFound(id) => write!(f, "Player {:?} not found", id),
            ExecutionError::ObjectNotFound(id) => write!(f, "Object {:?} not found", id),
            ExecutionError::EffectNotFound(id) => write!(f, "Effect {:?} not found", id),
            ExecutionError::TagNotFound(tag) => write!(f, "Tag '{}' not found", tag),
            ExecutionError::ContinuousDiscovery(error) => write!(f, "{error}"),
            ExecutionError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ExecutionError {}

/// Errors that can occur during target resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetError {
    /// No valid targets available.
    NoValidTargets,
    /// Target is protected (hexproof, shroud, etc.).
    Protected,
    /// Target is in wrong zone.
    WrongZone,
    /// Target doesn't match the required spec.
    DoesntMatch,
}

// ============================================================================
// Execution Context
// ============================================================================

/// A resolved target - either a specific object or player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum ResolvedTarget {
    Object(ObjectId),
    Player(PlayerId),
}

/// Rebase a scoped set of target assignments onto a local targets slice.
pub fn rebase_target_scope(
    targets: &[ResolvedTarget],
    target_assignments: &[TargetAssignment],
) -> (Vec<ResolvedTarget>, Vec<TargetAssignment>) {
    let mut local_targets = Vec::new();
    let mut local_assignments = Vec::with_capacity(target_assignments.len());

    for assignment in target_assignments {
        let start = local_targets.len();
        local_targets.extend_from_slice(&targets[assignment.range.clone()]);
        let end = local_targets.len();
        local_assignments.push(TargetAssignment {
            spec: assignment.spec.clone(),
            range: start..end,
        });
    }

    (local_targets, local_assignments)
}

/// Iteration-specific state carried across nested effect execution.
#[derive(Debug, Clone, Copy, Default)]
pub struct IterationContext {
    /// Current player in a ForEachOpponent/ForEachPlayer iteration.
    pub iterated_player: Option<PlayerId>,
    /// Current object in a ForEach iteration.
    pub iterated_object: Option<ObjectId>,
}

/// A triggered ability's "Do this only once each turn" (or N times) limit.
///
/// The limit counts times the optional instruction was actually performed
/// (accepted), not times the ability triggered or resolved: declining leaves
/// the instruction available later that turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoThisLimit {
    pub source: ObjectId,
    pub trigger_identity: crate::triggers::TriggerIdentity,
    pub limit: u32,
}

impl DoThisLimit {
    /// The "Do this only once each turn" limit carried by a triggered
    /// ability's intervening condition, including one conjoined with an
    /// intervening "if" ("if Legolas is tapped, you may untap it").
    pub fn from_condition(
        condition: &crate::ConditionExpr,
        source: ObjectId,
        trigger_identity: crate::triggers::TriggerIdentity,
    ) -> Option<Self> {
        fn limit_of(condition: &crate::ConditionExpr) -> Option<u32> {
            match condition {
                crate::ConditionExpr::DoThisMaxTimesEachTurn(limit) => Some(*limit),
                crate::ConditionExpr::And(left, right) => {
                    limit_of(left).or_else(|| limit_of(right))
                }
                _ => None,
            }
        }
        limit_of(condition).map(|limit| Self {
            source,
            trigger_identity,
            limit,
        })
    }

    pub fn reached(&self, game: &crate::game_state::GameState) -> bool {
        game.do_this_action_count_this_turn(self.source, self.trigger_identity) >= self.limit
    }
}

/// Combat-linked player selections available during execution.
#[derive(Debug, Clone, Copy, Default)]
pub struct CombatExecutionContext {
    /// Schedule group created by this resolution's most recent added combat.
    pub last_added_combat_order: Option<u64>,
    /// The defending player for combat triggers.
    pub defending_player: Option<PlayerId>,
    pub defending_player_reference: Option<crate::combat_state::DefendingPlayerReference>,
    /// The attacking player for combat triggers.
    pub attacking_player: Option<PlayerId>,
    /// The chosen player linked to this source, if one was captured earlier.
    pub chosen_player: Option<PlayerId>,
}

/// Triggering combat-damage context available while resolving combat-damage triggers.
#[derive(Debug, Clone)]
pub struct CombatDamageEventContext {
    pub source: ObjectId,
    pub source_controller: Option<PlayerId>,
    pub source_snapshot: Option<ObjectSnapshot>,
    pub damaged_player: Option<PlayerId>,
    pub damaged_object: Option<ObjectSnapshot>,
    pub is_combat: bool,
    pub amount: u32,
}

/// Block-declaration context available while resolving block-related triggers.
#[derive(Debug, Clone)]
pub struct BlockEventContext {
    pub attacker: ObjectId,
    pub attacker_snapshot: Option<ObjectSnapshot>,
    pub blockers: Vec<ObjectId>,
    pub blocker_snapshots: Vec<ObjectSnapshot>,
    pub became_blocked: bool,
}

/// Mana-choice restrictions scoped to the current resolution path.
#[derive(Debug, Clone, Default)]
pub struct ManaExecutionContext {
    /// Optional color restriction for mana-choice decisions in this execution.
    pub mana_color_restriction: Option<Vec<Color>>,
    /// Optional spending restrictions for mana produced during this execution.
    pub mana_usage_restrictions: Vec<crate::ability::ManaUsageRestriction>,
    /// Chosen creature type snapshot for mana produced by the source.
    pub mana_source_chosen_creature_type: Option<Subtype>,
    /// Provenance marker for mana produced during this execution.
    pub production_provenance: crate::events::mana::ManaProductionProvenance,
    /// Optional per-unit retention applied to mana produced in this execution.
    pub retention: Option<ironsmith_core::ManaRetentionDuration>,
    /// Mana spent on the activation cost of the resolving ability.
    pub activation_payment: crate::player::ManaPool,
    /// More specific purpose for mana payments nested inside this effect.
    pub payment_reason: Option<crate::costs::PaymentReason>,
}

/// Ephemeral replacement effects scoped to the current resolution path.
#[derive(Debug, Clone)]
pub struct ReplacementExecutionContext {
    /// The unresolved zone proposal whose program is executing. This is not
    /// a completed zone-change trigger and cannot impose its destination zone.
    pub original_zone_event: Option<Box<crate::events::ZoneChangeEvent>>,
    /// Source counter additions being proposed during battlefield entry.
    /// Their replacements run on the combined ETB event, not the source-zone card.
    pub entry_counter_source: Option<ObjectId>,
    /// Prospective characteristics, including earlier copy replacements.
    pub entry_event: Option<Box<crate::events::EnterBattlefieldEvent>>,
    /// CR614.13: source objects reserved by simultaneous battlefield entry.
    /// Inherited by nested replacement payloads; never removed from zone indexes.
    pub entry_reserved_objects: HashSet<ObjectId>,
    pub additional_replacement_effects: Vec<ReplacementEffect>,
    pub suppressed_replacement_effects: HashSet<ReplacementEffectId>,
    pub suppressed_replacement_effect_keys: HashSet<ReplacementEffectKey>,
    /// CR 400.6 destination choices for objects moved by mutually exclusive
    /// parts of one simultaneous event (for example, a lethal Exquisite
    /// Archangel whose lose-game replacement also tries to exile itself).
    pub simultaneous_zone_destinations: HashMap<ObjectId, crate::zone::Zone>,
}

impl Default for ReplacementExecutionContext {
    fn default() -> Self {
        Self {
            original_zone_event: None,
            entry_counter_source: None,
            entry_event: None,
            entry_reserved_objects: HashSet::new(),
            additional_replacement_effects: Vec::new(),
            suppressed_replacement_effects: HashSet::new(),
            suppressed_replacement_effect_keys: HashSet::new(),
            simultaneous_zone_destinations: HashMap::new(),
        }
    }
}

/// Resolution-level control flow, independent of an instruction's result.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ResolutionControl {
    #[default]
    Continue,
    Stop,
}

/// Accepted object choices and the next request in a suspended selection.
/// Declines and empty candidate sets advance the cursor without adding objects.
/// This is instruction control state, not a completed game-action outcome.
#[derive(Debug, Clone, Default)]
pub(crate) struct ObjectSelectionProgress {
    pub next_request: usize,
    pub chosen: Vec<ObjectSnapshot>,
}

/// Source, controller, chooser, subject and authored selection tag bind a cursor.
pub(crate) type ObjectSelectionProgressKey = (ObjectId, PlayerId, PlayerId, PlayerId, TagKey);

/// Context for effect execution.
pub struct ExecutionContext<'a> {
    /// The source object (spell/ability on stack).
    pub source: ObjectId,
    /// Linked pair and rules-text acquisition captured when the ability was admitted.
    pub linked_exile_owner: Option<crate::linked_exile::LinkedExileOwner>,
    pub source_number_owner: Option<crate::linked_exile::LinkedExileOwner>,
    /// The controller of the source.
    pub controller: PlayerId,
    /// Resolved targets for the effect.
    pub targets: Vec<ResolvedTarget>,
    /// Original spell/ability targets while `targets` holds cost choices.
    pub announced_targets: Option<Vec<ResolvedTarget>>,
    /// True when `targets` carries preselected cost-payment choices rather than
    /// spell or ability targets.
    pub targets_are_cost_choices: bool,
    /// Internal cloned cost-admission simulation; never a completed payment.
    pub(crate) prospective_cost_payment: bool,
    /// Active target requirement assignments for the current execution scope.
    pub target_assignments: Vec<TargetAssignment>,
    /// Announced divisions not yet consumed by their resolving effects.
    pub target_distributions: Vec<TargetDistribution>,
    /// Target requirement assignments as announced (CR 601.2c), before
    /// illegal targets were dropped at resolution (CR 608.2b). Used where the
    /// announced target count fixes a result, e.g. "divided evenly" (601.2d).
    pub announced_target_assignments: Vec<TargetAssignment>,
    /// X value (for spells with X in cost).
    pub x_value: Option<u32>,
    pub activation_values: Vec<(crate::effect::Value, Option<i32>)>,
    /// False when some announced target became illegal before resolution
    /// ("if both targets are still legal as this ability resolves").
    pub all_targets_legal: bool,
    /// Outcomes of previously executed effects (for WithId/If).
    pub effect_outcomes: HashMap<EffectId, EffectOutcome>,
    /// The most recent vote result(s) available to this resolution path.
    pub vote_results: HashMap<ObjectId, VoteResult>,
    /// The most recent simultaneous secret-choice result(s) in this resolution path.
    pub secret_choice_results: HashMap<ObjectId, SecretChoiceResult>,
    /// Iteration-specific state for nested effect execution.
    pub iteration: IterationContext,
    /// Decision maker for handling player choices (May effects, searches, etc.).
    pub decision_maker: &'a mut dyn DecisionMaker,
    /// Which optional costs were paid (kicker, buyback, etc.).
    pub optional_costs_paid: OptionalCostsPaid,
    /// An accepted optional instruction is executing. Actions such as blight
    /// must choose objects on which that optional action can be performed.
    pub(crate) optional_action: bool,
    /// How the source spell was cast.
    pub casting_method: crate::alternative_cast::CastingMethod,
    /// Combat-linked player selections and context.
    pub combat: CombatExecutionContext,
    /// The destination captured by this particular Ninjutsu activation.
    pub ninjutsu_attack_target: Option<crate::combat_state::AttackTarget>,
    /// Last known information for target objects (for when they leave the battlefield).
    pub target_snapshots: HashMap<ObjectId, ObjectSnapshot>,
    /// Last known information for the source object.
    /// Used when source-dependent effects resolve after the source has left the battlefield.
    pub source_snapshot: Option<ObjectSnapshot>,
    /// Tagged object snapshots for cross-effect references.
    ///
    /// Effects can tag their targets using `Effect::tag("name")`, and subsequent effects
    /// can reference those objects using `PlayerFilter::ControllerOf(ObjectRef::tagged("name"))`.
    /// This enables patterns like "Destroy target permanent. Its controller creates a token."
    ///
    /// Multiple objects can be tagged under the same tag (e.g., "Destroy all creatures" would
    /// tag all destroyed creatures). Use `get_tagged_first()` for single-object patterns and
    /// `get_tagged_all()` for multi-object patterns.
    pub tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    /// Suspended instruction-local selection cursors. Context checkpoints own
    /// their rollback; payment frames start with their own selection state.
    pub(crate) object_selection_progress:
        HashMap<ObjectSelectionProgressKey, ObjectSelectionProgress>,
    /// Tagged players for cross-effect references.
    ///
    /// Effects can tag players using `ctx.tag_player("name", player_id)`, and subsequent effects
    /// can iterate over them using `Effect::for_each_tagged_player("name", effects)`.
    /// This enables patterns like voting where we track "players who voted for X".
    ///
    /// For triggered abilities, tags are populated from the triggering event (e.g.,
    /// PlayersFinishedVotingEvent provides "voted_with_you", "voted_against_you", etc.).
    pub tagged_players: HashMap<TagKey, Vec<PlayerId>>,
    /// Players who may continue to inspect specific hidden cards if they become
    /// exiled face down later in the same resolution.
    pub face_down_exile_viewers: HashMap<ObjectId, HashSet<PlayerId>>,
    pub(crate) optional_identity_guard: Option<OptionalIdentityGuard>,
    /// The event that triggered this ability (for triggered abilities).
    /// Contains information about what caused the trigger (e.g., which object entered the battlefield).
    pub triggering_event: Option<crate::triggers::TriggerEvent>,
    /// Numeric value computed by the trigger matcher for resolving "that many".
    pub event_value_amount: Option<i32>,
    /// Most recently registered prevention shield in this resolution path.
    /// Delayed effects that explicitly reference damage "prevented this way"
    /// capture this stable identity when they are scheduled.
    pub last_prevention_shield: Option<crate::prevention::PreventionShieldId>,
    /// Structural identity of the resolving triggered ability, when available.
    pub trigger_identity: Option<crate::triggers::TriggerIdentity>,
    /// "Do this only once each turn" gate for the resolving triggered ability.
    /// The first optional instruction it governs takes it, so only that
    /// choice is limited and counted.
    pub do_this_limit: Option<DoThisLimit>,
    /// Index of the resolving activated ability on its source object, when available.
    pub ability_index: Option<usize>,
    /// Exact admitted acquisition; copies retain it and never count as activations.
    pub activation_origin: Option<crate::continuous::AbilityOrigin>,
    pub activation_definition: Option<ironsmith_core::LinkedExileDefinition>,
    /// Pre-chosen modes for modal spells (set during casting per MTG rule 601.2b).
    /// If Some, ChooseModeEffect should use these instead of prompting.
    pub chosen_modes: Option<Vec<usize>>,
    /// The cause of this effect execution (cost vs effect).
    /// This enables replacement effects to match based on what caused an event
    /// (e.g., Library of Leng only applies to effect-caused discards, not cost-based).
    pub cause: EventCause,
    /// Provenance parent node for events emitted during this execution.
    pub provenance: ProvNodeId,
    /// Mana-choice restrictions scoped to this execution.
    pub mana: ManaExecutionContext,
    /// Ephemeral replacement effects scoped to this execution.
    pub replacement: ReplacementExecutionContext,
    /// Identity of the direct effect currently executing. This lets CR 805.8
    /// collapse one effect applied to multiple teammates without collapsing
    /// distinct printed effects that add or skip the same structure.
    pub(crate) executing_effect: Option<usize>,
    pub(crate) shared_team_structure_operations: HashSet<(usize, usize, &'static str)>,
    /// Queue slot of the extra turn most recently created by this resolution
    /// (`TurnStore::extra_turns`), so a following "that turn" delayed trigger
    /// can be bound to it (CR 500.7, 603.7).
    pub(crate) created_extra_turn_index: Option<usize>,
    /// This resolution restarted the game (CR 726): its later battlefield
    /// entries are deferred until the new game's first untap step.
    pub(crate) restarted_game: bool,
    pub(crate) resolution_control: ResolutionControl,
    /// The first object id allocated after this stack entry began resolving.
    ///
    /// Objects with an id at or above it were moved into their current zone
    /// by this resolution, so its instructions may find them (CR 400.7j).
    /// Any other move of a tagged object makes it a new object this
    /// resolution can't follow (CR 400.7, 603.6c, 603.7c). `None` outside a
    /// stack resolution keeps the unrestricted stable-identity lookup.
    pub resolution_object_id_floor: Option<ObjectId>,
    /// Tag of the library search the next instruction reveals ("search ...
    /// for up to two basic land cards, reveal those cards, ..."). Set by the
    /// enclosing instruction list only while that search executes, so its
    /// chosen cards are revealed publicly with the selection (opened on every
    /// peer before the answer is replayed).
    pub(crate) public_search_reveal_tag: Option<TagKey>,
    /// Destination of a "put ... onto the battlefield attached to X"
    /// instruction: the attach instruction that immediately follows the move
    /// in the same instruction list. Set only while that move executes, so an
    /// Aura enters attached to X (CR 303.4f) and stays in its zone when it
    /// can't legally enchant X (CR 303.4i) instead of choosing on its own.
    pub(crate) pending_entry_attachment: Option<crate::target::ChooseSpec>,
    /// Continuous effects this resolution has registered so far, in order.
    /// "You may pay [cost] to end this effect" (Licids) ends exactly these.
    pub(crate) created_continuous_effects: Vec<crate::continuous::ContinuousEffectId>,
}

// Keep the checkpoint's owned fields in one list. The exhaustive context
// destructure makes a newly added context field a compile error until its
// rollback behavior is specified here.
macro_rules! execution_context_checkpoint {
    ($($field:ident: $field_type:ty),* $(,)?) => {
        /// Owned resolution state retained while an effect awaits a decision.
        /// The decision maker keeps its prompt and answers outside rollback.
        pub(crate) struct ExecutionContextCheckpoint {
            $($field: $field_type,)*
        }

        impl ExecutionContextCheckpoint {
            pub(crate) fn capture(ctx: &ExecutionContext<'_>) -> Self {
                let ExecutionContext { $($field,)* decision_maker: _ } = ctx;
                Self { $($field: $field.clone(),)* }
            }

            pub(crate) fn restore(self, ctx: &mut ExecutionContext<'_>) {
                $(ctx.$field = self.$field;)*
            }
            pub(crate) fn restore_ref(&self, ctx: &mut ExecutionContext<'_>) {
                $(ctx.$field = self.$field.clone();)*
            }
        }
    };
}

impl ExecutionContextCheckpoint {
    pub(crate) fn resolution_stopped(&self) -> bool {
        self.resolution_control == ResolutionControl::Stop
    }
    /// Restore an execution frame within the same successful resolution.
    /// Transaction/query rollback must keep using the ordinary restore methods.
    pub(crate) fn restore_ref_preserving_resolution_control(&self, ctx: &mut ExecutionContext<'_>) {
        let stopped = ctx.resolution_stopped() || self.resolution_stopped();
        self.restore_ref(ctx);
        if stopped {
            ctx.stop_resolution();
        }
    }
    pub(crate) fn restore_preserving_resolution_control(self, ctx: &mut ExecutionContext<'_>) {
        let stopped = ctx.resolution_stopped() || self.resolution_stopped();
        self.restore(ctx);
        if stopped {
            ctx.stop_resolution();
        }
    }
    pub(crate) fn replacement_scope(&self) -> &ReplacementExecutionContext {
        &self.replacement
    }
    pub(crate) fn controller(&self) -> PlayerId {
        self.controller
    }
    pub(crate) fn provenance(&self) -> ProvNodeId {
        self.provenance
    }
    /// Reborrow a decision maker with every captured owned field restored.
    pub(crate) fn reborrow<'a>(&self, dm: &'a mut dyn DecisionMaker) -> ExecutionContext<'a> {
        let mut ctx = ExecutionContext::new(self.source, self.controller, dm);
        self.restore_ref(&mut ctx);
        ctx
    }
}

execution_context_checkpoint! {
    source: ObjectId,
    linked_exile_owner: Option<crate::linked_exile::LinkedExileOwner>,
    source_number_owner: Option<crate::linked_exile::LinkedExileOwner>,
    controller: PlayerId,
    targets: Vec<ResolvedTarget>,
    announced_targets: Option<Vec<ResolvedTarget>>,
    targets_are_cost_choices: bool,
    prospective_cost_payment: bool,
    target_assignments: Vec<TargetAssignment>,
    target_distributions: Vec<TargetDistribution>,
    announced_target_assignments: Vec<TargetAssignment>,
    x_value: Option<u32>,
    activation_values: Vec<(crate::effect::Value, Option<i32>)>,
    all_targets_legal: bool,
    effect_outcomes: HashMap<EffectId, EffectOutcome>,
    vote_results: HashMap<ObjectId, VoteResult>,
    secret_choice_results: HashMap<ObjectId, SecretChoiceResult>,
    iteration: IterationContext,
    optional_costs_paid: OptionalCostsPaid,
    optional_action: bool,
    casting_method: crate::alternative_cast::CastingMethod,
    combat: CombatExecutionContext,
    ninjutsu_attack_target: Option<crate::combat_state::AttackTarget>,
    target_snapshots: HashMap<ObjectId, ObjectSnapshot>,
    source_snapshot: Option<ObjectSnapshot>,
    tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    object_selection_progress: HashMap<ObjectSelectionProgressKey, ObjectSelectionProgress>,
    tagged_players: HashMap<TagKey, Vec<PlayerId>>,
    face_down_exile_viewers: HashMap<ObjectId, HashSet<PlayerId>>,
    optional_identity_guard: Option<OptionalIdentityGuard>,
    triggering_event: Option<crate::triggers::TriggerEvent>,
    event_value_amount: Option<i32>,
    last_prevention_shield: Option<crate::prevention::PreventionShieldId>,
    trigger_identity: Option<crate::triggers::TriggerIdentity>,
    do_this_limit: Option<DoThisLimit>,
    ability_index: Option<usize>,
    activation_origin: Option<crate::continuous::AbilityOrigin>,
    activation_definition: Option<ironsmith_core::LinkedExileDefinition>,
    chosen_modes: Option<Vec<usize>>,
    cause: EventCause,
    provenance: ProvNodeId,
    mana: ManaExecutionContext,
    replacement: ReplacementExecutionContext,
    executing_effect: Option<usize>,
    shared_team_structure_operations: HashSet<(usize, usize, &'static str)>,
    created_extra_turn_index: Option<usize>,
    restarted_game: bool,
    resolution_control: ResolutionControl,
    resolution_object_id_floor: Option<ObjectId>,
    public_search_reveal_tag: Option<TagKey>,
    pending_entry_attachment: Option<crate::target::ChooseSpec>,
    created_continuous_effects: Vec<crate::continuous::ContinuousEffectId>,
}

// Payment frames inherit value-resolution inputs, but bind their own payer,
// cause, cost objects and instruction control state. This exhaustive capture
// forces every new context field to be classified instead of silently dropped.
macro_rules! payment_execution_inputs {
    (inherited { $($field:ident: $field_type:ty,)* } local { $($local:ident,)* }) => {
        #[derive(Clone)]
        pub(crate) struct PaymentExecutionInputs {
            $($field: $field_type,)*
        }
        impl PaymentExecutionInputs {
            pub(crate) fn capture(ctx: &ExecutionContext<'_>) -> Self {
                let ExecutionContext { $($field,)* $($local: _,)* decision_maker: _ } = ctx;
                Self { $($field: $field.clone(),)* }
            }
            pub(crate) fn restore_ref(&self, ctx: &mut ExecutionContext<'_>) {
                let reason = ctx.mana.payment_reason;
                $(ctx.$field = self.$field.clone();)*
                // A receiving frame owns why its own payment is being made.
                ctx.mana.payment_reason = reason;
            }
        }
    };
}

payment_execution_inputs! {
    inherited {
        linked_exile_owner: Option<crate::linked_exile::LinkedExileOwner>,
        source_number_owner: Option<crate::linked_exile::LinkedExileOwner>,
        activation_origin: Option<crate::continuous::AbilityOrigin>,
        activation_definition: Option<ironsmith_core::LinkedExileDefinition>,
        prospective_cost_payment: bool,
        all_targets_legal: bool,
        announced_target_assignments: Vec<TargetAssignment>,
        vote_results: HashMap<ObjectId, VoteResult>,
        secret_choice_results: HashMap<ObjectId, SecretChoiceResult>,
        iteration: IterationContext,
        optional_costs_paid: OptionalCostsPaid,
        casting_method: crate::alternative_cast::CastingMethod,
        combat: CombatExecutionContext,
        ninjutsu_attack_target: Option<crate::combat_state::AttackTarget>,
        target_snapshots: HashMap<ObjectId, ObjectSnapshot>,
        tagged_players: HashMap<TagKey, Vec<PlayerId>>,
        face_down_exile_viewers: HashMap<ObjectId, HashSet<PlayerId>>,
        triggering_event: Option<crate::triggers::TriggerEvent>,
        activation_values: Vec<(crate::effect::Value, Option<i32>)>,
        event_value_amount: Option<i32>,
        last_prevention_shield: Option<crate::prevention::PreventionShieldId>,
        trigger_identity: Option<crate::triggers::TriggerIdentity>,
        ability_index: Option<usize>,
        mana: ManaExecutionContext,
        resolution_object_id_floor: Option<ObjectId>,
    }
    local {
        source, controller, targets, announced_targets, targets_are_cost_choices,
        target_assignments, target_distributions, x_value, effect_outcomes,
        optional_action, source_snapshot, tagged_objects, object_selection_progress, optional_identity_guard,
        do_this_limit, chosen_modes, cause, provenance, replacement,
        executing_effect, shared_team_structure_operations, created_extra_turn_index,
        restarted_game, resolution_control, public_search_reveal_tag, pending_entry_attachment,
        created_continuous_effects,
    }
}

impl std::fmt::Debug for ExecutionContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("source", &self.source)
            .field("controller", &self.controller)
            .field("targets", &self.targets)
            .field("targets_are_cost_choices", &self.targets_are_cost_choices)
            .field("target_assignments", &self.target_assignments)
            .field("target_distributions", &self.target_distributions)
            .field("x_value", &self.x_value)
            .field("effect_outcomes", &self.effect_outcomes)
            .field("secret_choice_results", &self.secret_choice_results)
            .field("iteration", &self.iteration)
            .field("decision_maker", &"<&mut dyn DecisionMaker>")
            .field("optional_costs_paid", &self.optional_costs_paid)
            .field("casting_method", &self.casting_method)
            .field("combat", &self.combat)
            .field("target_snapshots", &self.target_snapshots)
            .field("source_snapshot", &self.source_snapshot)
            .field(
                "tagged_objects",
                &self.tagged_objects.keys().collect::<Vec<_>>(),
            )
            .field(
                "tagged_players",
                &self.tagged_players.keys().collect::<Vec<_>>(),
            )
            .field("face_down_exile_viewers", &self.face_down_exile_viewers)
            .field("triggering_event", &self.triggering_event)
            .field("event_value_amount", &self.event_value_amount)
            .field("last_prevention_shield", &self.last_prevention_shield)
            .field("trigger_identity", &self.trigger_identity)
            .field("do_this_limit", &self.do_this_limit)
            .field("ability_index", &self.ability_index)
            .field("cause", &self.cause)
            .field("provenance", &self.provenance)
            .field("mana", &self.mana)
            .field(
                "additional_replacement_effects",
                &self.replacement.additional_replacement_effects.len(),
            )
            .field(
                "simultaneous_zone_destinations",
                &self.replacement.simultaneous_zone_destinations,
            )
            .finish()
    }
}

impl<'a> ExecutionContext<'a> {
    pub(crate) fn stop_resolution(&mut self) {
        self.resolution_control = ResolutionControl::Stop;
    }
    pub(crate) fn resolution_stopped(&self) -> bool {
        self.resolution_control == ResolutionControl::Stop
    }

    /// A speculative query may construct temporary bindings, but it cannot
    /// publish them into the instruction that asked the question. Restore on
    /// every return, including failed candidates and early query errors.
    pub(crate) fn with_query_scope<T>(&mut self, query: impl FnOnce(&mut Self) -> T) -> T {
        let checkpoint = ExecutionContextCheckpoint::capture(self);
        let result = query(self);
        checkpoint.restore(self);
        result
    }

    /// Create a new execution context with a decision maker.
    pub fn new(
        source: ObjectId,
        controller: PlayerId,
        decision_maker: &'a mut dyn DecisionMaker,
    ) -> Self {
        Self {
            source,
            linked_exile_owner: None,
            source_number_owner: None,
            controller,
            targets: Vec::new(),
            announced_targets: None,
            targets_are_cost_choices: false,
            prospective_cost_payment: false,
            target_assignments: Vec::new(),
            target_distributions: Vec::new(),
            announced_target_assignments: Vec::new(),
            x_value: None,
            activation_values: Vec::new(),
            all_targets_legal: true,
            effect_outcomes: HashMap::new(),
            vote_results: HashMap::new(),
            secret_choice_results: HashMap::new(),
            iteration: IterationContext::default(),
            decision_maker,
            optional_costs_paid: OptionalCostsPaid::default(),
            optional_action: false,
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            combat: CombatExecutionContext::default(),
            ninjutsu_attack_target: None,
            target_snapshots: HashMap::new(),
            source_snapshot: None,
            tagged_objects: HashMap::new(),
            object_selection_progress: HashMap::new(),
            tagged_players: HashMap::new(),
            face_down_exile_viewers: HashMap::new(),
            optional_identity_guard: None,
            triggering_event: None,
            event_value_amount: None,
            last_prevention_shield: None,
            trigger_identity: None,
            do_this_limit: None,
            ability_index: None,
            activation_origin: None,
            activation_definition: None,
            chosen_modes: None,
            cause: EventCause::from_effect(source, controller),
            provenance: ProvNodeId::default(),
            mana: ManaExecutionContext::default(),
            replacement: ReplacementExecutionContext::default(),
            executing_effect: None,
            shared_team_structure_operations: HashSet::new(),
            created_extra_turn_index: None,
            restarted_game: false,
            resolution_control: ResolutionControl::Continue,
            resolution_object_id_floor: None,
            public_search_reveal_tag: None,
            pending_entry_attachment: None,
            created_continuous_effects: Vec::new(),
        }
    }

    /// Create a new execution context with a default decision maker (SelectFirstDecisionMaker).
    ///
    /// This method leaks memory and should only be used in tests or situations where
    /// the decision maker's choices don't matter.
    /// For production code, use `new()` with an explicit decision maker.
    ///
    /// The default decision maker:
    /// - Accepts all "may" effects (boolean choices return true)
    /// - Selects the first valid option when choices are required
    pub fn new_default(source: ObjectId, controller: PlayerId) -> ExecutionContext<'static> {
        // Leak a default decision maker - acceptable for tests
        let dm: &'static mut dyn DecisionMaker =
            Box::leak(Box::new(crate::decision::SelectFirstDecisionMaker));
        ExecutionContext {
            source,
            linked_exile_owner: None,
            source_number_owner: None,
            controller,
            targets: Vec::new(),
            announced_targets: None,
            targets_are_cost_choices: false,
            prospective_cost_payment: false,
            target_assignments: Vec::new(),
            target_distributions: Vec::new(),
            announced_target_assignments: Vec::new(),
            x_value: None,
            activation_values: Vec::new(),
            all_targets_legal: true,
            effect_outcomes: HashMap::new(),
            vote_results: HashMap::new(),
            secret_choice_results: HashMap::new(),
            iteration: IterationContext::default(),
            decision_maker: dm,
            optional_costs_paid: OptionalCostsPaid::default(),
            optional_action: false,
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            combat: CombatExecutionContext::default(),
            ninjutsu_attack_target: None,
            target_snapshots: HashMap::new(),
            source_snapshot: None,
            tagged_objects: HashMap::new(),
            object_selection_progress: HashMap::new(),
            tagged_players: HashMap::new(),
            face_down_exile_viewers: HashMap::new(),
            optional_identity_guard: None,
            triggering_event: None,
            event_value_amount: None,
            last_prevention_shield: None,
            trigger_identity: None,
            do_this_limit: None,
            ability_index: None,
            activation_origin: None,
            activation_definition: None,
            chosen_modes: None,
            cause: EventCause::from_effect(source, controller),
            provenance: ProvNodeId::default(),
            mana: ManaExecutionContext::default(),
            replacement: ReplacementExecutionContext::default(),
            executing_effect: None,
            shared_team_structure_operations: HashSet::new(),
            created_extra_turn_index: None,
            restarted_game: false,
            resolution_control: ResolutionControl::Continue,
            resolution_object_id_floor: None,
            public_search_reveal_tag: None,
            pending_entry_attachment: None,
            created_continuous_effects: Vec::new(),
        }
    }

    /// Set a different decision maker, returning a new context.
    /// This consumes the old context and creates a new one with the provided decision maker.
    pub fn with_decision_maker<'b>(self, dm: &'b mut dyn DecisionMaker) -> ExecutionContext<'b> {
        ExecutionContext {
            source: self.source,
            linked_exile_owner: self.linked_exile_owner,
            source_number_owner: self.source_number_owner,
            controller: self.controller,
            targets: self.targets,
            announced_targets: self.announced_targets,
            targets_are_cost_choices: self.targets_are_cost_choices,
            prospective_cost_payment: self.prospective_cost_payment,
            target_assignments: self.target_assignments,
            target_distributions: self.target_distributions,
            announced_target_assignments: self.announced_target_assignments,
            x_value: self.x_value,
            activation_values: self.activation_values.clone(),
            all_targets_legal: self.all_targets_legal,
            effect_outcomes: self.effect_outcomes,
            vote_results: self.vote_results,
            secret_choice_results: self.secret_choice_results,
            iteration: self.iteration,
            decision_maker: dm,
            optional_costs_paid: self.optional_costs_paid,
            optional_action: self.optional_action,
            casting_method: self.casting_method,
            combat: self.combat,
            ninjutsu_attack_target: self.ninjutsu_attack_target.clone(),
            target_snapshots: self.target_snapshots,
            source_snapshot: self.source_snapshot,
            tagged_objects: self.tagged_objects,
            object_selection_progress: self.object_selection_progress,
            tagged_players: self.tagged_players,
            face_down_exile_viewers: self.face_down_exile_viewers,
            optional_identity_guard: self.optional_identity_guard,
            triggering_event: self.triggering_event,
            event_value_amount: self.event_value_amount,
            last_prevention_shield: self.last_prevention_shield,
            trigger_identity: self.trigger_identity,
            do_this_limit: self.do_this_limit,
            ability_index: self.ability_index,
            activation_origin: self.activation_origin.clone(),
            activation_definition: self.activation_definition,
            chosen_modes: self.chosen_modes,
            cause: self.cause,
            provenance: self.provenance,
            mana: self.mana,
            replacement: self.replacement,
            executing_effect: self.executing_effect,
            shared_team_structure_operations: self.shared_team_structure_operations,
            created_extra_turn_index: self.created_extra_turn_index,
            restarted_game: self.restarted_game,
            resolution_control: self.resolution_control,
            resolution_object_id_floor: self.resolution_object_id_floor,
            public_search_reveal_tag: self.public_search_reveal_tag,
            pending_entry_attachment: self.pending_entry_attachment,
            created_continuous_effects: self.created_continuous_effects,
        }
    }

    /// Claim one team-level structural operation for the currently executing
    /// effect. Ordinary games and direct executor calls retain legacy behavior.
    pub(crate) fn claim_shared_team_structure_operation(
        &mut self,
        game: &GameState,
        player: PlayerId,
        operation: &'static str,
    ) -> bool {
        if !game.shared_team_turns_enabled() {
            return true;
        }
        let Some(effect) = self.executing_effect else {
            return true;
        };
        let Some(team) = game.team_index_for(player) else {
            return true;
        };
        self.shared_team_structure_operations
            .insert((effect, team, operation))
    }

    pub fn additional_replacement_effects(&self) -> &[ReplacementEffect] {
        &self.replacement.additional_replacement_effects
    }

    pub fn additional_replacement_effects_snapshot(&self) -> Vec<ReplacementEffect> {
        self.replacement.additional_replacement_effects.clone()
    }

    /// Return the CR 400.6 destination selected for this object, if any.
    pub fn simultaneous_zone_destination(&self, object: ObjectId) -> Option<crate::zone::Zone> {
        self.replacement
            .simultaneous_zone_destinations
            .get(&object)
            .copied()
    }

    pub fn with_temp_additional_replacement_effects<R>(
        &mut self,
        effects: Vec<ReplacementEffect>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original_len = self.replacement.additional_replacement_effects.len();
        self.replacement
            .additional_replacement_effects
            .extend(effects);
        let result = f(self);
        self.replacement
            .additional_replacement_effects
            .truncate(original_len);
        result
    }

    /// Restrict mana color choices for effects executed in this context.
    pub fn with_mana_color_restriction(mut self, restriction: Option<Vec<Color>>) -> Self {
        self.mana.mana_color_restriction = restriction;
        self
    }

    /// Restrict how mana produced during this execution may be spent.
    pub fn with_mana_usage_restrictions(
        mut self,
        restrictions: Vec<crate::ability::ManaUsageRestriction>,
    ) -> Self {
        self.mana.mana_usage_restrictions = restrictions;
        self
    }

    /// Snapshot the source's chosen creature type for later mana spending checks.
    pub fn with_mana_source_chosen_creature_type(mut self, subtype: Option<Subtype>) -> Self {
        self.mana.mana_source_chosen_creature_type = subtype;
        self
    }

    /// Mark how mana produced during this execution was generated.
    pub fn with_mana_production_provenance(
        mut self,
        provenance: crate::events::mana::ManaProductionProvenance,
    ) -> Self {
        self.mana.production_provenance = provenance;
        self
    }

    /// Retain the resolving activated ability's mana payment.
    pub fn with_activation_mana_payment(mut self, payment: crate::player::ManaPool) -> Self {
        self.mana.activation_payment = payment;
        self
    }

    /// Retain the exact linked rules acquisition in this execution.
    pub fn with_linked_exile_owner(
        mut self,
        owner: Option<crate::linked_exile::LinkedExileOwner>,
    ) -> Self {
        self.linked_exile_owner = owner;
        self
    }

    pub fn with_source_number_owner(
        mut self,
        owner: Option<crate::linked_exile::LinkedExileOwner>,
    ) -> Self {
        self.source_number_owner = owner;
        self
    }

    /// Set provenance parent for emitted events.
    pub fn with_provenance(mut self, provenance: ProvNodeId) -> Self {
        self.provenance = provenance;
        self
    }

    /// Snapshot all object targets for "last known information".
    /// Call this before executing effects that may exile/destroy targets.
    pub fn snapshot_targets(&mut self, game: &GameState) {
        if let Err(error) = self.try_snapshot_targets(game) {
            game.record_token_resource_failure(&error);
        }
    }

    pub fn try_snapshot_targets(&mut self, game: &GameState) -> Result<(), super::ExecutionError> {
        let mut snapshots = self.target_snapshots.clone();
        for target in &self.targets {
            let ResolvedTarget::Object(obj_id) = target else {
                continue;
            };
            if let Some(obj) = game.object(*obj_id) {
                snapshots.insert(
                    *obj_id,
                    ObjectSnapshot::try_from_object_with_calculated_characteristics(obj, game)?,
                );
            } else if let Some(entry) = game.stack_ability_entry(*obj_id) {
                let snapshot = game
                    .object(entry.object_id)
                    .map(|source| {
                        ObjectSnapshot::try_from_object_with_calculated_characteristics(
                            source, game,
                        )
                    })
                    .transpose()?
                    .or_else(|| entry.source_snapshot.clone());
                if let Some(mut snapshot) = snapshot {
                    snapshot.controller = entry.controller;
                    snapshot.zone = crate::zone::Zone::Stack;
                    snapshots.insert(*obj_id, snapshot);
                }
            }
        }
        self.target_snapshots = snapshots;
        Ok(())
    }

    /// Refresh target LKI when a target object is about to leave its expected zone.
    pub fn refresh_target_snapshot(&mut self, snapshot: ObjectSnapshot) {
        let Some(key) = self.target_snapshots.iter().find_map(|(key, existing)| {
            (existing.stable_id == snapshot.stable_id && existing.zone == snapshot.zone)
                .then_some(*key)
        }) else {
            return;
        };
        self.target_snapshots.insert(key, snapshot);
    }

    /// Refresh source LKI when the source is about to leave its expected zone.
    pub fn refresh_source_snapshot(&mut self, snapshot: ObjectSnapshot) {
        if self.source_snapshot.as_ref().is_none_or(|existing| {
            existing.stable_id == snapshot.stable_id && existing.zone == snapshot.zone
        }) {
            self.source_snapshot = Some(snapshot);
        }
    }

    /// Set the defending player.
    pub fn with_defending_player(mut self, player: PlayerId) -> Self {
        self.combat.defending_player = Some(player);
        self
    }

    /// Set the attacking player.
    pub fn with_attacking_player(mut self, player: PlayerId) -> Self {
        self.combat.attacking_player = Some(player);
        self
    }

    /// Bind only when an instruction actually uses the role. Native choices
    /// and the existing enclosing checkpoint own suspension and rollback.
    pub(crate) fn bind_defending_player(
        &mut self,
        game: &GameState,
    ) -> Result<bool, ExecutionError> {
        let reference = self.combat.defending_player_reference.or_else(|| {
            self.combat
                .defending_player
                .is_none()
                .then(|| {
                    self.triggering_event
                        .as_ref()
                        .and_then(|event| game.defending_reference_for_event(event))
                })
                .flatten()
        });
        if matches!(
            reference,
            Some(
                crate::combat_state::DefendingPlayerReference::Selected(_)
                    | crate::combat_state::DefendingPlayerReference::KnownAbsent
            )
        ) {
            return Ok(true);
        }
        let players = if let Some(reference) = reference {
            game.defending_player_candidates(reference)?
        } else if let Some(player) = self.combat.defending_player {
            vec![player]
        } else {
            return Err(ExecutionError::IncompleteEvidence(
                "defending player has no combat reference or selected actor".into(),
            ));
        };
        let chosen = match players.as_slice() {
            [] => {
                self.combat.defending_player = None;
                self.combat.defending_player_reference =
                    Some(crate::combat_state::DefendingPlayerReference::KnownAbsent);
                return Ok(true);
            }
            [player] => *player,
            _ => {
                let options = players
                    .iter()
                    .filter_map(|id| {
                        game.player(*id)
                            .map(|player| (player.name.to_string(), *id))
                    })
                    .collect::<Vec<_>>();
                let choice = crate::decisions::ask_choose_one(
                    game,
                    &mut self.decision_maker,
                    self.controller,
                    self.source,
                    &options,
                );
                if self.decision_maker.awaiting_choice() {
                    return Ok(false);
                }
                choice.ok_or(ExecutionError::UnresolvedPlayerDecision {
                    player: self.controller,
                    decision: "choose the defending player",
                })?
            }
        };
        self.combat.defending_player = Some(chosen);
        self.combat.defending_player_reference = Some(
            crate::combat_state::DefendingPlayerReference::Selected(chosen),
        );
        Ok(true)
    }

    pub(crate) fn defending_players(
        &self,
        game: &GameState,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        if let Some(reference) = self.combat.defending_player_reference {
            return game.defending_player_candidates(reference);
        }
        self.combat
            .defending_player
            .map(|player| {
                if game
                    .player(player)
                    .is_some_and(|player| player.is_in_game())
                {
                    vec![player]
                } else {
                    Vec::new()
                }
            })
            .ok_or_else(|| {
                ExecutionError::IncompleteEvidence(
                    "defending player has no combat reference or selected actor".into(),
                )
            })
    }

    /// Set the X value.
    pub fn with_x(mut self, x: u32) -> Self {
        self.x_value = Some(x);
        self
    }

    /// Remember that `viewer` may continue to inspect these cards if they later
    /// become exiled face down during the current resolution.
    pub fn remember_face_down_exile_viewers(&mut self, cards: &[ObjectId], viewer: PlayerId) {
        for &card in cards {
            self.face_down_exile_viewers
                .entry(card)
                .or_default()
                .insert(viewer);
        }
    }

    /// Return the players remembered for a hidden card during this resolution.
    pub fn face_down_exile_viewers_for(&self, card: ObjectId) -> Option<&HashSet<PlayerId>> {
        self.face_down_exile_viewers.get(&card)
    }

    /// Set resolved targets.
    pub fn with_targets(mut self, targets: Vec<ResolvedTarget>) -> Self {
        self.targets = targets;
        self.targets_are_cost_choices = false;
        self.target_assignments.clear();
        self
    }

    /// Set object choices made during cost payment.
    ///
    /// Cost choices are still consumed through `targets` by generic move and
    /// sacrifice effects, but they should not become "target objects" in filter
    /// context. In particular, filters like "another creature" should compare
    /// against the source object, not against the object chosen to pay the cost.
    pub fn with_cost_choice_targets(mut self, targets: Vec<ResolvedTarget>) -> Self {
        self.targets = targets;
        self.targets_are_cost_choices = true;
        self.target_assignments.clear();
        self
    }

    /// Set active target assignments for this execution scope.
    pub fn with_target_assignments(mut self, target_assignments: Vec<TargetAssignment>) -> Self {
        self.target_assignments = target_assignments;
        self
    }

    /// Set the target divisions announced when this stack object was proposed.
    pub fn with_target_distributions(
        mut self,
        target_distributions: Vec<TargetDistribution>,
    ) -> Self {
        self.target_distributions = target_distributions;
        self
    }

    /// Record the target assignments as they were announced.
    pub fn with_announced_target_assignments(
        mut self,
        announced_target_assignments: Vec<TargetAssignment>,
    ) -> Self {
        self.announced_target_assignments = announced_target_assignments;
        self
    }

    /// Number of targets announced for the given target requirement, if known.
    pub fn announced_target_count(&self, spec: &ChooseSpec) -> Option<usize> {
        self.announced_target_assignments
            .iter()
            .find(|assignment| assignment.spec == *spec)
            .map(|assignment| assignment.range.len())
    }

    /// Consume the next announced division for the specified target requirement.
    pub fn take_target_distribution(&mut self, spec: &ChooseSpec) -> Option<TargetDistribution> {
        let index = self
            .target_distributions
            .iter()
            .position(|distribution| distribution.spec == *spec)?;
        Some(self.target_distributions.remove(index))
    }

    /// Temporarily override `targets` while running a closure, then restore.
    pub fn with_temp_targets<R>(
        &mut self,
        targets: Vec<ResolvedTarget>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original_announced = self.announced_targets.take();
        self.announced_targets = original_announced
            .clone()
            .or_else(|| (!self.targets_are_cost_choices).then(|| self.targets.clone()));
        let original_targets = std::mem::replace(&mut self.targets, targets);
        let original_target_assignments = std::mem::take(&mut self.target_assignments);
        let result = f(self);
        self.targets = original_targets;
        self.announced_targets = original_announced;
        self.target_assignments = original_target_assignments;
        result
    }

    /// Temporarily override active target assignments while running a closure.
    pub fn with_temp_target_assignments<R>(
        &mut self,
        target_assignments: Vec<TargetAssignment>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original_target_assignments =
            std::mem::replace(&mut self.target_assignments, target_assignments);
        let result = f(self);
        self.target_assignments = original_target_assignments;
        result
    }

    /// Temporarily override `iterated_player` while running a closure, then restore.
    pub fn with_temp_iterated_player<R>(
        &mut self,
        iterated_player: Option<PlayerId>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original_iterated_player =
            std::mem::replace(&mut self.iteration.iterated_player, iterated_player);
        let result = f(self);
        self.iteration.iterated_player = original_iterated_player;
        result
    }

    /// Temporarily override `iterated_object` while running a closure, then restore.
    pub fn with_temp_iterated_object<R>(
        &mut self,
        iterated_object: Option<ObjectId>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original_iterated_object =
            std::mem::replace(&mut self.iteration.iterated_object, iterated_object);
        let result = f(self);
        self.iteration.iterated_object = original_iterated_object;
        result
    }

    /// Resolve the first two context targets as object IDs.
    pub fn resolve_two_object_targets(&self) -> Option<(ObjectId, ObjectId)> {
        let first = match self.targets.first()? {
            ResolvedTarget::Object(id) => *id,
            _ => return None,
        };
        let second = match self.targets.get(1)? {
            ResolvedTarget::Object(id) => *id,
            _ => return None,
        };
        Some((first, second))
    }

    /// Resolve the first two context targets as player IDs.
    pub fn resolve_two_player_targets(&self) -> Option<(PlayerId, PlayerId)> {
        let first = match self.targets.first()? {
            ResolvedTarget::Player(id) => *id,
            _ => return None,
        };
        let second = match self.targets.get(1)? {
            ResolvedTarget::Player(id) => *id,
            _ => return None,
        };
        Some((first, second))
    }

    /// Set source snapshot for source-LKI lookups.
    pub fn with_source_snapshot(mut self, snapshot: ObjectSnapshot) -> Self {
        self.source_snapshot = Some(snapshot);
        self
    }

    /// Set optional costs paid.
    pub fn with_optional_costs_paid(mut self, paid: OptionalCostsPaid) -> Self {
        self.optional_costs_paid = paid;
        self
    }

    /// Set how the source spell was cast.
    pub fn with_casting_method(
        mut self,
        casting_method: crate::alternative_cast::CastingMethod,
    ) -> Self {
        self.casting_method = casting_method;
        self
    }

    /// Set the chosen player linked to this source.
    pub fn with_chosen_player(mut self, player: Option<PlayerId>) -> Self {
        self.combat.chosen_player = player;
        self
    }

    /// Set tagged objects from a pre-existing map.
    ///
    /// This is used to pass tags between cost effects, where the first effect
    /// may tag an object (e.g., "choose a creature") and a subsequent effect
    /// needs to reference it (e.g., "sacrifice the chosen creature").
    pub fn with_effect_outcomes(mut self, outcomes: HashMap<EffectId, EffectOutcome>) -> Self {
        self.effect_outcomes = outcomes;
        self
    }

    pub fn with_tagged_objects(mut self, tags: HashMap<TagKey, Vec<ObjectSnapshot>>) -> Self {
        self.tagged_objects = tags;
        if !self.tagged_objects.contains_key(&TagKey::from("__it__"))
            && let Some(triggering) = self
                .tagged_objects
                .get(&TagKey::from("triggering"))
                .cloned()
        {
            self.tagged_objects
                .insert(TagKey::from("__it__"), triggering);
        }
        self
    }

    /// Set the triggering event for this triggered ability.
    ///
    /// If the event is a `PlayersFinishedVotingEvent`, this method computes
    /// `tagged_players` from the perspective of THIS ability's controller (not the
    /// vote initiator). This is important because "voted_with_you" must be computed
    /// from the triggered ability controller's perspective.
    ///
    /// For example: Alice controls Tivit (vote initiator), Bob controls Model of Unity.
    /// When Model of Unity triggers, "voted_with_you" should contain players who
    /// voted with Bob, not players who voted with Alice.
    pub fn with_triggering_event(mut self, event: crate::triggers::TriggerEvent) -> Self {
        self.provenance = event.provenance();
        if self.combat.defending_player.is_none()
            && self.combat.defending_player_reference.is_none()
        {
            self.combat.defending_player_reference = event.defending_player_reference();
        }
        if let Some(attack) = event.downcast::<crate::events::PlayerAttackDeclarationEvent>() {
            self.combat.attacking_player = Some(attack.attacker);
            self.combat.defending_player = Some(attack.defender);
            self.set_tagged_players(
                ironsmith_core::tag::ATTACK_DECLARATION_ACTOR_TAG,
                vec![attack.attacker],
            );
            self.set_tagged_players(
                ironsmith_core::tag::ATTACK_DECLARATION_DEFENDER_TAG,
                vec![attack.defender],
            );
        }
        if let Some(snapshot) = event.snapshot() {
            let snapshots = vec![snapshot.clone()];
            self.set_tagged_objects("triggering", snapshots.clone());
            self.set_tagged_objects("it", snapshots.clone());
            self.set_tagged_objects("__it__", snapshots);
        }
        if self.iteration.iterated_player.is_none() {
            self.iteration.iterated_player = event.trigger_player();
        }

        for (tag, players) in event.player_tags() {
            self.set_tagged_players(tag.clone(), players.clone());
        }
        if let Some(damage) = event.downcast::<crate::events::DamageEvent>()
            && let Some(snapshot) = event
                .source_snapshot()
                .filter(|snapshot| snapshot.object_id == damage.source)
        {
            self.set_tagged_players(
                ironsmith_core::tag::DAMAGE_SOURCE_CONTROLLER_TAG,
                vec![snapshot.controller],
            );
        }
        if let Some(controller) = event.cause().and_then(|cause| cause.source_controller) {
            self.set_tagged_players(
                ironsmith_core::TRIGGERING_EVENT_CAUSE_CONTROLLER_TAG,
                vec![controller],
            );
        }
        if let Some(controller) = event.controller() {
            self.set_tagged_players(
                ironsmith_core::TRIGGERING_EVENT_CONTROLLER_TAG,
                vec![controller],
            );
        }

        // If the event is vote-related, compute tags from THIS ability controller's perspective.
        if let Some(voting_event) = event.downcast::<crate::events::PlayersFinishedVotingEvent>() {
            self.apply_voting_tags(
                &voting_event.votes,
                &voting_event.player_tags,
                &voting_event.voter_teams,
            );
        } else if let Some(action_event) = event.downcast::<crate::events::KeywordActionEvent>()
            && action_event.action == crate::events::KeywordActionKind::Vote
            && let Some(votes) = &action_event.votes
        {
            self.apply_voting_tags(votes, &action_event.player_tags, &action_event.voter_teams);
        }

        if let Some(action_event) = event.downcast::<crate::events::KeywordActionEvent>() {
            for (tag, snapshots) in &action_event.object_tags {
                self.set_tagged_objects(tag.clone(), snapshots.clone());
            }
        }
        if let Some(zone_change_event) = event.downcast::<crate::events::ZoneChangeEvent>() {
            for (tag, snapshots) in &zone_change_event.object_tags {
                self.set_tagged_objects(tag.clone(), snapshots.clone());
            }
        }

        self.triggering_event = Some(event);
        self
    }

    /// Set a numeric value computed by the trigger matcher for grouped events.
    pub fn with_event_value_amount(mut self, amount: i32) -> Self {
        self.event_value_amount = Some(amount);
        self
    }

    /// Set the structural identity for the resolving triggered ability.
    pub fn with_trigger_identity(
        mut self,
        trigger_identity: crate::triggers::TriggerIdentity,
    ) -> Self {
        self.trigger_identity = Some(trigger_identity);
        self
    }

    /// Retain the exact acquisition admitted by the activation owner.
    pub fn with_activation_origin(
        mut self,
        origin: Option<crate::continuous::AbilityOrigin>,
    ) -> Self {
        self.activation_origin = origin;
        self
    }

    pub fn with_activation_definition(
        mut self,
        definition: Option<ironsmith_core::LinkedExileDefinition>,
    ) -> Self {
        self.activation_definition = definition;
        self
    }

    /// Legacy ordinal used for selection and existing resolution-count readers.
    pub fn with_ability_index(mut self, ability_index: usize) -> Self {
        self.ability_index = Some(ability_index);
        self
    }

    fn apply_voting_tags(
        &mut self,
        votes: &[crate::events::PlayerVote],
        extra_tags: &HashMap<TagKey, Vec<PlayerId>>,
        voter_teams: &[(PlayerId, usize)],
    ) {
        use std::collections::{HashMap, HashSet};

        // Get options that THIS ability's controller voted for.
        let my_options: HashSet<usize> = votes
            .iter()
            .filter(|v| v.player == self.controller)
            .map(|v| v.option_index)
            .collect();

        // Build per-player options excluding this controller.
        let mut options_by_player: HashMap<PlayerId, HashSet<usize>> = HashMap::new();
        // "Each opponent who voted ...": teammates are never included.
        for vote in votes.iter().filter(|v| {
            crate::events::other::vote_event_players_are_opponents(
                voter_teams,
                self.controller,
                v.player,
            )
        }) {
            options_by_player
                .entry(vote.player)
                .or_default()
                .insert(vote.option_index);
        }

        let mut voted_with_me = Vec::new();
        let mut voted_against_me = Vec::new();

        // A player with several votes (CR 701.38d) can have voted both for a
        // choice this controller voted for and for one they didn't; they are
        // in both groups.
        for (player, player_options) in options_by_player {
            if !my_options.is_disjoint(&player_options) {
                voted_with_me.push(player);
            }
            if !my_options.is_empty() && !player_options.is_subset(&my_options) {
                voted_against_me.push(player);
            }
        }

        voted_with_me.sort_by_key(|p| p.0);
        voted_against_me.sort_by_key(|p| p.0);

        if !voted_with_me.is_empty() {
            self.set_tagged_players("voted_with_you", voted_with_me);
        } else {
            self.clear_player_tag("voted_with_you");
        }
        if !voted_against_me.is_empty() {
            self.set_tagged_players("voted_against_you", voted_against_me);
        } else {
            self.clear_player_tag("voted_against_you");
        }

        // Merge additional event-provided tags (for example per-option groupings).
        // Keep controller-relative voted_with/against computed above.
        for (tag, players) in extra_tags {
            if tag.as_str() == "voted_with_you" || tag.as_str() == "voted_against_you" {
                continue;
            }
            self.set_tagged_players(tag.clone(), players.clone());
        }
    }

    /// Set pre-chosen modes for modal spells (per MTG rule 601.2b).
    pub fn with_chosen_modes(mut self, modes: Option<Vec<usize>>) -> Self {
        self.chosen_modes = modes;
        self
    }

    /// Set the event cause (cost vs effect) for this execution.
    ///
    /// This enables replacement effects and triggers to distinguish between
    /// events caused by costs (e.g., discarding as activation cost) vs effects
    /// (e.g., discarding from a spell's resolution).
    pub fn with_cause(mut self, cause: EventCause) -> Self {
        self.cause = cause;
        self
    }

    /// Store a full effect outcome.
    pub fn store_outcome(&mut self, id: EffectId, outcome: EffectOutcome) {
        self.effect_outcomes.insert(id, outcome);
    }

    /// Get a stored effect outcome.
    pub fn get_outcome(&self, id: EffectId) -> Option<&EffectOutcome> {
        self.effect_outcomes.get(&id)
    }

    /// Tag an object for reference by subsequent effects.
    ///
    /// This stores a snapshot of the object under the given tag name.
    /// Multiple objects can be tagged under the same tag.
    /// Subsequent effects can reference these objects using
    /// `PlayerFilter::ControllerOf(ObjectRef::tagged(tag))` etc.
    pub fn tag_object(&mut self, tag: impl Into<TagKey>, snapshot: ObjectSnapshot) {
        self.tagged_objects
            .entry(tag.into())
            .or_default()
            .push(snapshot);
    }

    /// Record an object this resolution just exiled with its source.
    ///
    /// Resolution starts with the source-exiled tag seeded from every card
    /// ever exiled with the source ("cards exiled with ~"). A pronoun after an
    /// exile in the same resolution ("exile it with a stash counter on it")
    /// names only what this resolution exiled, so the first such exile
    /// replaces the seeded history. Filter references to "cards exiled with
    /// ~" still read the full link set through the filter context.
    pub fn tag_source_exiled_result(&mut self, snapshot: ObjectSnapshot) {
        if self
            .source_snapshot
            .as_ref()
            .is_some_and(|source| source.stable_id == snapshot.stable_id)
        {
            self.set_tagged_objects(crate::tag::SOURCE_EXILED_SELF_TAG, vec![snapshot.clone()]);
        }
        const RESOLUTION_MARKER: &str = crate::tag::SOURCE_EXILED_THIS_RESOLUTION_TAG;
        if !self.tagged_objects.contains_key(RESOLUTION_MARKER) {
            self.tagged_objects.remove(SOURCE_EXILED_TAG);
        }
        self.tag_object(RESOLUTION_MARKER, snapshot.clone());
        self.tag_object(SOURCE_EXILED_TAG, snapshot);
    }

    /// Tag multiple objects at once under the same tag.
    pub fn tag_objects(&mut self, tag: impl Into<TagKey>, snapshots: Vec<ObjectSnapshot>) {
        self.tagged_objects
            .entry(tag.into())
            .or_default()
            .extend(snapshots);
    }

    /// Append object snapshots under a tag, skipping objects already present.
    pub fn tag_objects_unique(&mut self, tag: impl Into<TagKey>, snapshots: Vec<ObjectSnapshot>) {
        let entry = self.tagged_objects.entry(tag.into()).or_default();
        for snapshot in snapshots {
            if !entry
                .iter()
                .any(|existing| existing.object_id == snapshot.object_id)
            {
                entry.push(snapshot);
            }
        }
    }

    /// Replace any existing object snapshots for a tag.
    pub(crate) fn with_object_tag<R>(
        &mut self,
        tag: impl Into<TagKey>,
        objects: Vec<ObjectSnapshot>,
        run: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let tag = tag.into();
        let previous = self.tagged_objects.insert(tag.clone(), objects);
        let result = run(self);
        match previous {
            Some(objects) => {
                self.tagged_objects.insert(tag, objects);
            }
            None => {
                self.tagged_objects.remove(&tag);
            }
        }
        result
    }

    pub fn set_tagged_objects(&mut self, tag: impl Into<TagKey>, snapshots: Vec<ObjectSnapshot>) {
        self.tagged_objects.insert(tag.into(), snapshots);
    }

    /// Clear a specific object tag.
    pub fn clear_object_tag(&mut self, tag: impl AsRef<str>) -> Option<Vec<ObjectSnapshot>> {
        self.tagged_objects.remove(tag.as_ref())
    }

    /// Get the first tagged object snapshot (for single-target patterns).
    ///
    /// This is the backwards-compatible method for patterns like
    /// "Destroy target permanent. Its controller creates a token."
    pub fn get_tagged(&self, tag: impl AsRef<str>) -> Option<&ObjectSnapshot> {
        self.tagged_objects
            .get(tag.as_ref())
            .and_then(|v| v.first())
    }

    /// Get all tagged object snapshots (for multi-target patterns).
    ///
    /// This is for patterns like "Destroy all creatures. Their controllers
    /// each create a token for each creature they controlled that was destroyed."
    pub fn get_tagged_all(&self, tag: impl AsRef<str>) -> Option<&Vec<ObjectSnapshot>> {
        self.tagged_objects.get(tag.as_ref())
    }

    /// Count tagged objects grouped by controller.
    ///
    /// Returns a map from controller PlayerId to the number of tagged objects they controlled.
    /// Useful for effects like "each player creates a token for each creature they controlled
    /// that was destroyed this way."
    pub fn count_tagged_by_controller(&self, tag: impl AsRef<str>) -> HashMap<PlayerId, usize> {
        let mut counts = HashMap::new();
        if let Some(snapshots) = self.tagged_objects.get(tag.as_ref()) {
            for snapshot in snapshots {
                *counts.entry(snapshot.controller).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Tag a player for reference by subsequent effects.
    ///
    /// This stores the player ID under the given tag name.
    /// Multiple players can be tagged under the same tag.
    /// Subsequent effects can iterate over these players using
    /// `Effect::for_each_tagged_player(tag, effects)`.
    pub fn tag_player(&mut self, tag: impl Into<TagKey>, player: PlayerId) {
        self.tagged_players
            .entry(tag.into())
            .or_default()
            .push(player);
    }

    /// Tag multiple players at once under the same tag.
    pub fn tag_players(&mut self, tag: impl Into<TagKey>, players: Vec<PlayerId>) {
        self.tagged_players
            .entry(tag.into())
            .or_default()
            .extend(players);
    }

    /// Replace any existing player list for a tag.
    pub fn set_tagged_players(&mut self, tag: impl Into<TagKey>, players: Vec<PlayerId>) {
        self.tagged_players.insert(tag.into(), players);
    }

    /// Clear a specific player tag.
    pub fn clear_player_tag(&mut self, tag: impl AsRef<str>) -> Option<Vec<PlayerId>> {
        self.tagged_players.remove(tag.as_ref())
    }

    /// Get all tagged players (for iteration patterns).
    ///
    /// This is for patterns like "Each player who voted for X may scry 2."
    pub fn get_tagged_players(&self, tag: impl AsRef<str>) -> Option<&Vec<PlayerId>> {
        self.tagged_players.get(tag.as_ref())
    }

    /// Build a filter context for evaluating filters.
    pub fn filter_context(&self, game: &GameState) -> FilterContext {
        let mut target_players = if self.targets_are_cost_choices {
            Vec::new()
        } else {
            self.targets
                .iter()
                .filter_map(|target| match target {
                    ResolvedTarget::Player(id) => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // An instruction's object-target scope still refers to the player
        // chosen by an earlier target group in this same resolution.
        if target_players.is_empty() && !self.targets_are_cost_choices {
            target_players.extend(
                self.announced_targets
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|target| match target {
                        ResolvedTarget::Player(player) => Some(*player),
                        _ => None,
                    }),
            );
        }
        let target_objects = if self.targets_are_cost_choices {
            Vec::new()
        } else {
            self.targets
                .iter()
                .filter_map(|target| match target {
                    ResolvedTarget::Object(id) => game
                        .object(*id)
                        .and_then(|obj| ObjectSnapshot::capture_for_execution(obj, game))
                        .or_else(|| self.target_snapshots.get(id).cloned()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let mut tagged_objects = self.tagged_objects.clone();
        let mut tagged_players = self.tagged_players.clone();
        // Present-tense "attacking that player" is a live relation. The
        // attacked player's identity stays bound to the declaration, while
        // removed attackers stop qualifying and later attacking entrants can
        // qualify (CR 508.6). Never derive this from the active-player seat.
        let mut attacking = Vec::new();
        if let Some(attack) = self
            .triggering_event
            .as_ref()
            .and_then(|event| event.downcast::<crate::events::PlayerAttackDeclarationEvent>())
            && attack.turn_number == game.turn.turn_number
            && attack.combat_phase == game.turn_store.combat_phases_started_this_turn
            && let Some(combat) = &game.combat
        {
            for info in &combat.attackers {
                if matches!(info.target, crate::combat_state::AttackTarget::Player(player) if player == attack.defender)
                    && let Some(player) = game.current_controller(info.creature)
                    && !attacking.contains(&player)
                {
                    attacking.push(player);
                }
            }
        }
        tagged_players.insert(
            ironsmith_core::tag::CURRENT_PLAYERS_ATTACKING_EVENT_DEFENDER_TAG.into(),
            attacking,
        );
        let linked = match &self.linked_exile_owner {
            Some(owner) => match game.linked_exile_pair_members(owner) {
                Ok(members) => members,
                Err(error) => {
                    game.record_token_resource_failure(&error);
                    &[]
                }
            },
            None => game.get_exiled_with_source_links(self.source),
        };
        let source_exiled = linked
            .iter()
            .filter_map(|id| {
                game.object(*id)
                    .and_then(|obj| ObjectSnapshot::capture_for_execution(obj, game))
            })
            .collect::<Vec<_>>();
        if self.linked_exile_owner.is_some() || !source_exiled.is_empty() {
            // A known empty pair must replace any stale source-wide tag.
            tagged_objects.insert(TagKey::from(SOURCE_EXILED_TAG), source_exiled);
        }
        let mut target_objects = target_objects;
        if let Some(triggering_event) = &self.triggering_event
            && triggering_event
                .downcast::<crate::events::DamageEvent>()
                .is_none()
            && let Some(object_id) = triggering_event.object_id()
            && let Some(snapshot) = triggering_event.snapshot().cloned().or_else(|| {
                game.object(object_id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
        {
            // A resolution prelude can bind a more specific participant
            // (for attachment events, the recipient instead of the attachment).
            tagged_objects
                .entry(TagKey::from("triggering"))
                .or_insert_with(|| vec![snapshot]);
            if let Some(entry) = game.stack.iter().find(|entry| entry.object_id == object_id) {
                // "that spell targets only a single opponent ... for each
                // other opponent": an ability with no player targets of its
                // own reads the triggering spell's targeted player, just as it
                // reads the spell's targeted objects below.
                if target_players.is_empty() {
                    target_players.extend(entry.targets.iter().filter_map(|target| match target {
                        crate::game_state::Target::Player(player) => Some(*player),
                        crate::game_state::Target::Object(_) => None,
                    }));
                }
                target_objects.extend(entry.targets.iter().filter_map(|target| {
                    match target {
                        crate::game_state::Target::Object(target_id) => game
                            .object(*target_id)
                            .and_then(|object| ObjectSnapshot::capture_for_execution(object, game)),
                        crate::game_state::Target::Player(_) => None,
                    }
                }));
            }
        }
        if let Some(damage) = self.damage_event_context(game) {
            if let Some(snapshot) = damage.source_snapshot {
                tagged_objects
                    .entry(TagKey::from("damage_source"))
                    .or_default()
                    .push(snapshot);
            }
            if let Some(snapshot) = damage.damaged_object {
                target_objects.push(snapshot.clone());
                tagged_objects
                    .entry(TagKey::from("damaged"))
                    .or_default()
                    .push(snapshot);
            }
            if let Some(player) = damage.damaged_player {
                tagged_players
                    .entry(TagKey::from("damaged_player"))
                    .or_default()
                    .push(player);
            }
        }
        if let Some(block_context) = self.block_event_context(game) {
            if let Some(snapshot) = block_context.attacker_snapshot {
                target_objects.push(snapshot.clone());
                tagged_objects
                    .entry(TagKey::from("blocked"))
                    .or_default()
                    .push(snapshot.clone());
                if block_context.became_blocked {
                    tagged_objects
                        .entry(TagKey::from("became_blocked"))
                        .or_default()
                        .push(snapshot);
                }
            }
            if !block_context.blocker_snapshots.is_empty() {
                tagged_objects
                    .entry(TagKey::from("blocking"))
                    .or_default()
                    .extend(block_context.blocker_snapshots);
            }
        }
        let chosen_player = self
            .combat
            .chosen_player
            .or_else(|| game.chosen_player(self.source));
        let mut filter_ctx = game
            .filter_context_for(self.controller, Some(self.source))
            .with_source_snapshot(self.source_snapshot.clone())
            .with_iterated_player(self.iteration.iterated_player)
            .with_x_value(self.x_value)
            .with_chosen_player(chosen_player)
            .with_target_players(target_players)
            .with_target_objects(target_objects)
            .with_tagged_objects(&tagged_objects)
            .with_tagged_players(&tagged_players)
            .with_effect_outcomes(&self.effect_outcomes);
        filter_ctx.source_number_owner = self.source_number_owner.clone();
        filter_ctx.active_player = game.singular_active_player(chosen_player);
        if self.combat.defending_player.is_some() {
            filter_ctx.defending_player = self.combat.defending_player;
            filter_ctx.defending_players.clear();
        }
        filter_ctx.defending_player_reference = self.combat.defending_player_reference;
        if let Some(reference) = self.combat.defending_player_reference {
            filter_ctx.defending_players = game
                .defending_player_candidates(reference)
                .unwrap_or_default();
        }
        if self.combat.attacking_player.is_some() {
            filter_ctx.attacking_player = self.combat.attacking_player;
            if self.triggering_event.as_ref().is_some_and(|event| {
                event
                    .downcast::<crate::events::PlayerAttackDeclarationEvent>()
                    .is_some()
            }) {
                // A declaration by one player stays singular even during
                // a turn shared with that player's teammates.
                filter_ctx.attacking_players.clear();
            }
        }
        filter_ctx
    }

    pub fn combat_damage_event_context(
        &self,
        game: &GameState,
    ) -> Option<CombatDamageEventContext> {
        let context = self.damage_event_context(game)?;
        if !context.is_combat {
            return None;
        }
        Some(context)
    }

    fn damage_event_context(&self, game: &GameState) -> Option<CombatDamageEventContext> {
        let triggering_event = self.triggering_event.as_ref()?;
        let damage = triggering_event.downcast::<crate::events::DamageEvent>()?;
        let source_snapshot = triggering_event.source_snapshot().cloned().or_else(|| {
            game.object(damage.source)
                .and_then(|obj| ObjectSnapshot::capture_for_execution(obj, game))
        });
        let source_controller = game
            .object(damage.source)
            .map(|obj| game.controller_of(obj))
            .or_else(|| source_snapshot.as_ref().map(|snapshot| snapshot.controller));
        let (damaged_player, damaged_object) = match damage.target {
            crate::events::DamageTarget::Player(player) => (Some(player), None),
            crate::events::DamageTarget::Object(object_id) => {
                let snapshot = damage.target_snapshot.clone().or_else(|| {
                    game.object(object_id)
                        .and_then(|obj| ObjectSnapshot::capture_for_execution(obj, game))
                });
                (None, snapshot)
            }
        };
        Some(CombatDamageEventContext {
            source: damage.source,
            source_controller,
            source_snapshot,
            damaged_player,
            damaged_object,
            is_combat: damage.is_combat,
            amount: damage.amount,
        })
    }

    pub fn block_event_context(&self, game: &GameState) -> Option<BlockEventContext> {
        let triggering_event = self.triggering_event.as_ref()?;
        if let Some(blocked) =
            triggering_event.downcast::<crate::events::combat::CreatureBlockedEvent>()
        {
            let attacker_snapshot = blocked.attacker_snapshot.clone().or_else(|| {
                game.object(blocked.attacker)
                    .and_then(|obj| ObjectSnapshot::capture_for_execution(obj, game))
            });
            let blocker_snapshot = blocked.blocker_snapshot.clone().or_else(|| {
                game.object(blocked.blocker)
                    .and_then(|obj| ObjectSnapshot::capture_for_execution(obj, game))
            });
            return Some(BlockEventContext {
                attacker: blocked.attacker,
                attacker_snapshot,
                blockers: vec![blocked.blocker],
                blocker_snapshots: blocker_snapshot.into_iter().collect(),
                became_blocked: false,
            });
        }
        if let Some(blocked) =
            triggering_event.downcast::<crate::events::combat::CreatureBecameBlockedEvent>()
        {
            let attacker_snapshot = blocked.attacker_snapshot.clone().or_else(|| {
                game.object(blocked.attacker)
                    .and_then(|obj| ObjectSnapshot::capture_for_execution(obj, game))
            });
            let blocker_snapshots = if blocked.blocker_snapshots.is_empty() {
                blocked
                    .blockers
                    .iter()
                    .filter_map(|blocker| {
                        game.object(*blocker)
                            .and_then(|obj| ObjectSnapshot::capture_for_execution(obj, game))
                    })
                    .collect()
            } else {
                blocked.blocker_snapshots.clone()
            };
            return Some(BlockEventContext {
                attacker: blocked.attacker,
                attacker_snapshot,
                blockers: blocked.blockers.clone(),
                blocker_snapshots,
                became_blocked: true,
            });
        }
        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::cause::EventCause;
    use crate::events::{DamageEvent, DamageTarget};
    use crate::filter::ObjectFilterExt;
    use crate::ids::CardId;
    use crate::provenance::ProvNodeId;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn non_damage_triggering_object_is_tagged_without_becoming_an_announced_target() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Source", alice);
        let triggering_object = create_creature(&mut game, "Triggering object", alice);
        let triggering_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(triggering_object).expect("triggering object"),
            &game,
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::ZoneChangeEvent::with_cause(
                triggering_object,
                Zone::Hand,
                Zone::Battlefield,
                EventCause::effect(),
                Some(triggering_snapshot),
            ),
            ProvNodeId::default(),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = ExecutionContext::new(source, alice, &mut dm).with_triggering_event(event);
        let filter_ctx = ctx.filter_context(&game);

        assert!(filter_ctx.target_objects.is_empty());
        assert!(
            filter_ctx
                .tagged_objects
                .get(&TagKey::from("triggering"))
                .is_some_and(|snapshots| snapshots
                    .iter()
                    .any(|snapshot| snapshot.object_id == triggering_object))
        );
        let other_creature = ObjectFilter::creature().you_control().other();
        assert!(!other_creature.matches(game.object(source).expect("source"), &filter_ctx, &game));
        assert!(other_creature.matches(
            game.object(triggering_object).expect("triggering object"),
            &filter_ctx,
            &game
        ));

        let explicitly_targeted = create_creature(&mut game, "Explicit target", alice);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let targeted_ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(explicitly_targeted)]);
        let filter_ctx = targeted_ctx.filter_context(&game);
        assert!(!other_creature.matches(
            game.object(explicitly_targeted).expect("explicit target"),
            &filter_ctx,
            &game
        ));
        assert!(other_creature.matches(game.object(source).expect("source"), &filter_ctx, &game));
    }

    #[test]
    fn combat_damage_event_context_exposes_source_player_and_amount() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Attacker", alice);

        let event = crate::triggers::TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source,
                DamageTarget::Player(bob),
                3,
                true,
                EventCause::combat_damage(source),
            ),
            ProvNodeId::default(),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = ExecutionContext::new(source, alice, &mut dm).with_triggering_event(event);

        let combat = ctx
            .combat_damage_event_context(&game)
            .expect("combat damage context");
        assert_eq!(combat.source, source);
        assert_eq!(combat.source_controller, Some(alice));
        assert_eq!(combat.damaged_player, Some(bob));
        assert_eq!(combat.amount, 3);
        assert!(combat.is_combat);

        let filter_ctx = ctx.filter_context(&game);
        assert_eq!(
            filter_ctx
                .tagged_players
                .get(&TagKey::from("damaged_player"))
                .cloned()
                .unwrap_or_default(),
            vec![bob]
        );
        assert!(
            filter_ctx
                .tagged_objects
                .get(&TagKey::from("damage_source"))
                .is_some_and(|snapshots| snapshots
                    .iter()
                    .any(|snapshot| snapshot.object_id == source))
        );
    }

    #[test]
    fn combat_damage_event_context_exposes_damaged_object_snapshot() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Attacker", alice);
        let damaged = create_creature(&mut game, "Blocker", bob);
        let damaged_snapshot = game
            .object(damaged)
            .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, &game))
            .expect("damaged object snapshot");

        let event = crate::triggers::TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source,
                DamageTarget::Object(damaged),
                2,
                true,
                EventCause::combat_damage(source),
            )
            .with_target_snapshot(damaged_snapshot),
            ProvNodeId::default(),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = ExecutionContext::new(source, alice, &mut dm).with_triggering_event(event);

        let combat = ctx
            .combat_damage_event_context(&game)
            .expect("combat damage context");
        assert_eq!(
            combat
                .damaged_object
                .as_ref()
                .map(|snapshot| snapshot.object_id),
            Some(damaged)
        );

        let filter_ctx = ctx.filter_context(&game);
        assert!(
            filter_ctx
                .tagged_objects
                .get(&TagKey::from("damaged"))
                .is_some_and(|snapshots| snapshots
                    .iter()
                    .any(|snapshot| snapshot.object_id == damaged))
        );
        assert!(
            filter_ctx
                .target_objects
                .iter()
                .any(|snapshot| snapshot.object_id == damaged)
        );
    }

    #[test]
    fn block_event_context_tags_blocked_and_blocking_objects() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_creature(&mut game, "Attacker", alice);
        let blocker = create_creature(&mut game, "Blocker", bob);
        let attacker_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(attacker).unwrap(),
            &game,
        );
        let blocker_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(blocker).unwrap(),
            &game,
        );
        let event = crate::triggers::TriggerEvent::new(
            crate::events::combat::CreatureBecameBlockedEvent::with_target_and_blockers(
                attacker,
                vec![blocker],
                None,
                Some(attacker_snapshot),
                vec![blocker_snapshot],
            ),
            ProvNodeId::default(),
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let ctx = ExecutionContext::new(attacker, alice, &mut dm).with_triggering_event(event);

        let filter_ctx = ctx.filter_context(&game);
        assert_eq!(
            filter_ctx.tagged_objects[&TagKey::from("became_blocked")][0].object_id,
            attacker
        );
        assert_eq!(
            filter_ctx.tagged_objects[&TagKey::from("blocking")][0].object_id,
            blocker
        );
    }
}
