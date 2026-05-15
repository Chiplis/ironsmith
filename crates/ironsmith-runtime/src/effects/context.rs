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
use crate::effects::VoteResult;
use crate::events::cause::EventCause;
use crate::game_state::{GameState, TargetAssignment};
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::replacement::{ReplacementEffect, ReplacementEffectId, ReplacementEffectKey};
use crate::snapshot::ObjectSnapshot;
use crate::tag::{SOURCE_EXILED_TAG, TagKey};
use crate::target::FilterContext;
use crate::types::Subtype;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during effect execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    /// Target is invalid or no longer exists.
    InvalidTarget,
    /// Could not resolve a value (e.g., X not set).
    UnresolvableValue(String),
    /// Effect is impossible to execute in current state.
    Impossible(String),
    /// Referenced player does not exist.
    PlayerNotFound(PlayerId),
    /// Referenced object does not exist.
    ObjectNotFound(ObjectId),
    /// Referenced effect ID not found in context.
    EffectNotFound(EffectId),
    /// Referenced tag not found in context (object not tagged by prior effect).
    TagNotFound(String),
    /// Internal error (should not happen).
    InternalError(String),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::InvalidTarget => write!(f, "Invalid target"),
            ExecutionError::UnresolvableValue(msg) => write!(f, "Cannot resolve value: {}", msg),
            ExecutionError::Impossible(msg) => write!(f, "Effect impossible: {}", msg),
            ExecutionError::PlayerNotFound(id) => write!(f, "Player {:?} not found", id),
            ExecutionError::ObjectNotFound(id) => write!(f, "Object {:?} not found", id),
            ExecutionError::EffectNotFound(id) => write!(f, "Effect {:?} not found", id),
            ExecutionError::TagNotFound(tag) => write!(f, "Tag '{}' not found", tag),
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

/// Combat-linked player selections available during execution.
#[derive(Debug, Clone, Copy, Default)]
pub struct CombatExecutionContext {
    /// The defending player for combat triggers.
    pub defending_player: Option<PlayerId>,
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
}

/// Ephemeral replacement effects scoped to the current resolution path.
#[derive(Debug, Clone, Default)]
pub struct ReplacementExecutionContext {
    pub additional_replacement_effects: Vec<ReplacementEffect>,
    pub suppressed_replacement_effects: HashSet<ReplacementEffectId>,
    pub suppressed_replacement_effect_keys: HashSet<ReplacementEffectKey>,
}

/// Context for effect execution.
pub struct ExecutionContext<'a> {
    /// The source object (spell/ability on stack).
    pub source: ObjectId,
    /// The controller of the source.
    pub controller: PlayerId,
    /// Resolved targets for the effect.
    pub targets: Vec<ResolvedTarget>,
    /// True when `targets` carries preselected cost-payment choices rather than
    /// spell or ability targets.
    pub targets_are_cost_choices: bool,
    /// Active target requirement assignments for the current execution scope.
    pub target_assignments: Vec<TargetAssignment>,
    /// X value (for spells with X in cost).
    pub x_value: Option<u32>,
    /// Outcomes of previously executed effects (for WithId/If).
    pub effect_outcomes: HashMap<EffectId, EffectOutcome>,
    /// The most recent vote result(s) available to this resolution path.
    pub vote_results: HashMap<ObjectId, VoteResult>,
    /// Iteration-specific state for nested effect execution.
    pub iteration: IterationContext,
    /// Decision maker for handling player choices (May effects, searches, etc.).
    pub decision_maker: &'a mut dyn DecisionMaker,
    /// Which optional costs were paid (kicker, buyback, etc.).
    pub optional_costs_paid: OptionalCostsPaid,
    /// How the source spell was cast.
    pub casting_method: crate::alternative_cast::CastingMethod,
    /// Combat-linked player selections and context.
    pub combat: CombatExecutionContext,
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
    /// The event that triggered this ability (for triggered abilities).
    /// Contains information about what caused the trigger (e.g., which object entered the battlefield).
    pub triggering_event: Option<crate::triggers::TriggerEvent>,
    /// Numeric value computed by the trigger matcher for resolving "that many".
    pub event_value_amount: Option<i32>,
    /// Structural identity of the resolving triggered ability, when available.
    pub trigger_identity: Option<crate::triggers::TriggerIdentity>,
    /// Index of the resolving activated ability on its source object, when available.
    pub ability_index: Option<usize>,
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
}

impl std::fmt::Debug for ExecutionContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("source", &self.source)
            .field("controller", &self.controller)
            .field("targets", &self.targets)
            .field("targets_are_cost_choices", &self.targets_are_cost_choices)
            .field("target_assignments", &self.target_assignments)
            .field("x_value", &self.x_value)
            .field("effect_outcomes", &self.effect_outcomes)
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
            .field("trigger_identity", &self.trigger_identity)
            .field("ability_index", &self.ability_index)
            .field("cause", &self.cause)
            .field("provenance", &self.provenance)
            .field("mana", &self.mana)
            .field(
                "additional_replacement_effects",
                &self.replacement.additional_replacement_effects.len(),
            )
            .finish()
    }
}

impl<'a> ExecutionContext<'a> {
    /// Create a new execution context with a decision maker.
    pub fn new(
        source: ObjectId,
        controller: PlayerId,
        decision_maker: &'a mut dyn DecisionMaker,
    ) -> Self {
        Self {
            source,
            controller,
            targets: Vec::new(),
            targets_are_cost_choices: false,
            target_assignments: Vec::new(),
            x_value: None,
            effect_outcomes: HashMap::new(),
            vote_results: HashMap::new(),
            iteration: IterationContext::default(),
            decision_maker,
            optional_costs_paid: OptionalCostsPaid::default(),
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            combat: CombatExecutionContext::default(),
            target_snapshots: HashMap::new(),
            source_snapshot: None,
            tagged_objects: HashMap::new(),
            tagged_players: HashMap::new(),
            face_down_exile_viewers: HashMap::new(),
            triggering_event: None,
            event_value_amount: None,
            trigger_identity: None,
            ability_index: None,
            chosen_modes: None,
            cause: EventCause::from_effect(source, controller),
            provenance: ProvNodeId::default(),
            mana: ManaExecutionContext::default(),
            replacement: ReplacementExecutionContext::default(),
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
            controller,
            targets: Vec::new(),
            targets_are_cost_choices: false,
            target_assignments: Vec::new(),
            x_value: None,
            effect_outcomes: HashMap::new(),
            vote_results: HashMap::new(),
            iteration: IterationContext::default(),
            decision_maker: dm,
            optional_costs_paid: OptionalCostsPaid::default(),
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            combat: CombatExecutionContext::default(),
            target_snapshots: HashMap::new(),
            source_snapshot: None,
            tagged_objects: HashMap::new(),
            tagged_players: HashMap::new(),
            face_down_exile_viewers: HashMap::new(),
            triggering_event: None,
            event_value_amount: None,
            trigger_identity: None,
            ability_index: None,
            chosen_modes: None,
            cause: EventCause::from_effect(source, controller),
            provenance: ProvNodeId::default(),
            mana: ManaExecutionContext::default(),
            replacement: ReplacementExecutionContext::default(),
        }
    }

    /// Set a different decision maker, returning a new context.
    /// This consumes the old context and creates a new one with the provided decision maker.
    pub fn with_decision_maker<'b>(self, dm: &'b mut dyn DecisionMaker) -> ExecutionContext<'b> {
        ExecutionContext {
            source: self.source,
            controller: self.controller,
            targets: self.targets,
            targets_are_cost_choices: self.targets_are_cost_choices,
            target_assignments: self.target_assignments,
            x_value: self.x_value,
            effect_outcomes: self.effect_outcomes,
            vote_results: self.vote_results,
            iteration: self.iteration,
            decision_maker: dm,
            optional_costs_paid: self.optional_costs_paid,
            casting_method: self.casting_method,
            combat: self.combat,
            target_snapshots: self.target_snapshots,
            source_snapshot: self.source_snapshot,
            tagged_objects: self.tagged_objects,
            tagged_players: self.tagged_players,
            face_down_exile_viewers: self.face_down_exile_viewers,
            triggering_event: self.triggering_event,
            event_value_amount: self.event_value_amount,
            trigger_identity: self.trigger_identity,
            ability_index: self.ability_index,
            chosen_modes: self.chosen_modes,
            cause: self.cause,
            provenance: self.provenance,
            mana: self.mana,
            replacement: self.replacement,
        }
    }

    pub fn additional_replacement_effects(&self) -> &[ReplacementEffect] {
        &self.replacement.additional_replacement_effects
    }

    pub fn additional_replacement_effects_snapshot(&self) -> Vec<ReplacementEffect> {
        self.replacement.additional_replacement_effects.clone()
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

    /// Set provenance parent for emitted events.
    pub fn with_provenance(mut self, provenance: ProvNodeId) -> Self {
        self.provenance = provenance;
        self
    }

    /// Snapshot all object targets for "last known information".
    /// Call this before executing effects that may exile/destroy targets.
    pub fn snapshot_targets(&mut self, game: &GameState) {
        for target in &self.targets {
            if let ResolvedTarget::Object(obj_id) = target
                && let Some(obj) = game.object(*obj_id)
            {
                self.target_snapshots.insert(
                    *obj_id,
                    ObjectSnapshot::from_object_with_calculated_characteristics(obj, game),
                );
            }
        }
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

    /// Temporarily override `targets` while running a closure, then restore.
    pub fn with_temp_targets<R>(
        &mut self,
        targets: Vec<ResolvedTarget>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original_targets = std::mem::replace(&mut self.targets, targets);
        let original_target_assignments = std::mem::take(&mut self.target_assignments);
        let result = f(self);
        self.targets = original_targets;
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
        if let Some(snapshot) = event.snapshot() {
            let snapshots = vec![snapshot.clone()];
            self.set_tagged_objects("triggering", snapshots.clone());
            self.set_tagged_objects("it", snapshots.clone());
            self.set_tagged_objects("__it__", snapshots);
        }
        if self.iteration.iterated_player.is_none() {
            self.iteration.iterated_player = event.trigger_player();
        }

        // If the event is vote-related, compute tags from THIS ability controller's perspective.
        if let Some(voting_event) = event.downcast::<crate::events::PlayersFinishedVotingEvent>() {
            self.apply_voting_tags(&voting_event.votes, &voting_event.player_tags);
        } else if let Some(action_event) = event.downcast::<crate::events::KeywordActionEvent>()
            && action_event.action == crate::events::KeywordActionKind::Vote
            && let Some(votes) = &action_event.votes
        {
            self.apply_voting_tags(votes, &action_event.player_tags);
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

    /// Set the activated ability index for the resolving activated ability.
    pub fn with_ability_index(mut self, ability_index: usize) -> Self {
        self.ability_index = Some(ability_index);
        self
    }

    fn apply_voting_tags(
        &mut self,
        votes: &[crate::events::PlayerVote],
        extra_tags: &HashMap<TagKey, Vec<PlayerId>>,
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
        for vote in votes.iter().filter(|v| v.player != self.controller) {
            options_by_player
                .entry(vote.player)
                .or_default()
                .insert(vote.option_index);
        }

        let mut voted_with_me = Vec::new();
        let mut voted_against_me = Vec::new();

        for (player, player_options) in options_by_player {
            if !my_options.is_disjoint(&player_options) {
                voted_with_me.push(player);
            } else if !my_options.is_empty() {
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

    /// Tag multiple objects at once under the same tag.
    pub fn tag_objects(&mut self, tag: impl Into<TagKey>, snapshots: Vec<ObjectSnapshot>) {
        self.tagged_objects
            .entry(tag.into())
            .or_default()
            .extend(snapshots);
    }

    /// Replace any existing object snapshots for a tag.
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
        let target_players = if self.targets_are_cost_choices {
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
        let target_objects = if self.targets_are_cost_choices {
            Vec::new()
        } else {
            self.targets
                .iter()
                .filter_map(|target| match target {
                    ResolvedTarget::Object(id) => game
                        .object(*id)
                        .map(|obj| {
                            ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                        })
                        .or_else(|| self.target_snapshots.get(id).cloned()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let mut tagged_objects = self.tagged_objects.clone();
        let mut tagged_players = self.tagged_players.clone();
        let source_exiled = game
            .get_exiled_with_source_links(self.source)
            .iter()
            .filter_map(|id| {
                game.object(*id).map(|obj| {
                    ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                })
            })
            .collect::<Vec<_>>();
        if !source_exiled.is_empty() {
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
            target_objects.push(snapshot.clone());
            tagged_objects
                .entry(TagKey::from("triggering"))
                .or_default()
                .push(snapshot);
            if let Some(entry) = game.stack.iter().find(|entry| entry.object_id == object_id) {
                target_objects.extend(entry.targets.iter().filter_map(|target| match target {
                    crate::game_state::Target::Object(target_id) => {
                        game.object(*target_id).map(|object| {
                            ObjectSnapshot::from_object_with_calculated_characteristics(
                                object, game,
                            )
                        })
                    }
                    crate::game_state::Target::Player(_) => None,
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
        let mut filter_ctx = game
            .filter_context_for(self.controller, Some(self.source))
            .with_iterated_player(self.iteration.iterated_player)
            .with_x_value(self.x_value)
            .with_chosen_player(
                self.combat
                    .chosen_player
                    .or_else(|| game.chosen_player(self.source)),
            )
            .with_target_players(target_players)
            .with_target_objects(target_objects)
            .with_tagged_objects(&tagged_objects)
            .with_tagged_players(&tagged_players)
            .with_effect_outcomes(&self.effect_outcomes);
        if self.combat.defending_player.is_some() {
            filter_ctx.defending_player = self.combat.defending_player;
        }
        if self.combat.attacking_player.is_some() {
            filter_ctx.attacking_player = self.combat.attacking_player;
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
                .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        });
        let source_controller = game
            .object(damage.source)
            .map(|obj| game.controller_of(obj))
            .or_else(|| source_snapshot.as_ref().map(|snapshot| snapshot.controller));
        let (damaged_player, damaged_object) = match damage.target {
            crate::events::DamageTarget::Player(player) => (Some(player), None),
            crate::events::DamageTarget::Object(object_id) => {
                let snapshot = damage.target_snapshot.clone().or_else(|| {
                    game.object(object_id).map(|obj| {
                        ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                    })
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
                game.object(blocked.attacker).map(|obj| {
                    ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                })
            });
            let blocker_snapshot = blocked.blocker_snapshot.clone().or_else(|| {
                game.object(blocked.blocker).map(|obj| {
                    ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                })
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
                game.object(blocked.attacker).map(|obj| {
                    ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                })
            });
            let blocker_snapshots = if blocked.blocker_snapshots.is_empty() {
                blocked
                    .blockers
                    .iter()
                    .filter_map(|blocker| {
                        game.object(*blocker).map(|obj| {
                            ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                        })
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
    use crate::ids::CardId;
    use crate::provenance::ProvNodeId;
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
