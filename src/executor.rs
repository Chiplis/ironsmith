//! Effect execution engine for MTG.
//!
//! This module provides the runtime execution of effects, including:
//! - Value resolution (X, counts, power/toughness, etc.)
//! - Target validation
//! - Effect execution with proper game state mutations

use std::collections::HashMap;

use crate::color::Color;
use crate::cost::OptionalCostsPaid;
use crate::decision::DecisionMaker;
use crate::effect::{Effect, EffectId, EffectOutcome, EffectResult, Value};
use crate::events::cause::EventCause;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::{SOURCE_EXILED_TAG, TagKey};
use crate::target::{ChooseSpec, FilterContext};

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

/// Context for effect execution.
pub struct ExecutionContext<'a> {
    /// The source object (spell/ability on stack).
    pub source: ObjectId,
    /// The controller of the source.
    pub controller: PlayerId,
    /// Resolved targets for the effect.
    pub targets: Vec<ResolvedTarget>,
    /// X value (for spells with X in cost).
    pub x_value: Option<u32>,
    /// Results of previously executed effects (for WithId/If).
    pub effect_results: HashMap<EffectId, EffectResult>,
    /// Current player in a ForEachOpponent/ForEachPlayer iteration.
    pub iterated_player: Option<PlayerId>,
    /// Current object in a ForEach iteration.
    pub iterated_object: Option<ObjectId>,
    /// Decision maker for handling player choices (May effects, searches, etc.).
    pub decision_maker: &'a mut dyn DecisionMaker,
    /// Which optional costs were paid (kicker, buyback, etc.).
    pub optional_costs_paid: OptionalCostsPaid,
    /// The defending player for combat triggers.
    pub defending_player: Option<PlayerId>,
    /// The attacking player for combat triggers.
    pub attacking_player: Option<PlayerId>,
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
    /// The event that triggered this ability (for triggered abilities).
    /// Contains information about what caused the trigger (e.g., which object entered the battlefield).
    pub triggering_event: Option<crate::triggers::TriggerEvent>,
    /// Pre-chosen modes for modal spells (set during casting per MTG rule 601.2b).
    /// If Some, ChooseModeEffect should use these instead of prompting.
    pub chosen_modes: Option<Vec<usize>>,
    /// The cause of this effect execution (cost vs effect).
    /// This enables replacement effects to match based on what caused an event
    /// (e.g., Library of Leng only applies to effect-caused discards, not cost-based).
    pub cause: EventCause,
    /// Optional color restriction for mana-choice decisions in this execution.
    pub mana_color_restriction: Option<Vec<Color>>,
}

impl std::fmt::Debug for ExecutionContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("source", &self.source)
            .field("controller", &self.controller)
            .field("targets", &self.targets)
            .field("x_value", &self.x_value)
            .field("effect_results", &self.effect_results)
            .field("iterated_player", &self.iterated_player)
            .field("iterated_object", &self.iterated_object)
            .field("decision_maker", &"<&mut dyn DecisionMaker>")
            .field("optional_costs_paid", &self.optional_costs_paid)
            .field("defending_player", &self.defending_player)
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
            .field("triggering_event", &self.triggering_event)
            .field("cause", &self.cause)
            .field("mana_color_restriction", &self.mana_color_restriction)
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
            x_value: None,
            effect_results: HashMap::new(),
            iterated_player: None,
            iterated_object: None,
            decision_maker,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            attacking_player: None,
            target_snapshots: HashMap::new(),
            source_snapshot: None,
            tagged_objects: HashMap::new(),
            tagged_players: HashMap::new(),
            triggering_event: None,
            chosen_modes: None,
            cause: EventCause::default(),
            mana_color_restriction: None,
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
            x_value: None,
            effect_results: HashMap::new(),
            iterated_player: None,
            iterated_object: None,
            decision_maker: dm,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            attacking_player: None,
            target_snapshots: HashMap::new(),
            source_snapshot: None,
            tagged_objects: HashMap::new(),
            tagged_players: HashMap::new(),
            triggering_event: None,
            chosen_modes: None,
            cause: EventCause::default(),
            mana_color_restriction: None,
        }
    }

    /// Set a different decision maker, returning a new context.
    /// This consumes the old context and creates a new one with the provided decision maker.
    pub fn with_decision_maker<'b>(self, dm: &'b mut dyn DecisionMaker) -> ExecutionContext<'b> {
        ExecutionContext {
            source: self.source,
            controller: self.controller,
            targets: self.targets,
            x_value: self.x_value,
            effect_results: self.effect_results,
            iterated_player: self.iterated_player,
            iterated_object: self.iterated_object,
            decision_maker: dm,
            optional_costs_paid: self.optional_costs_paid,
            defending_player: self.defending_player,
            attacking_player: self.attacking_player,
            target_snapshots: self.target_snapshots,
            source_snapshot: self.source_snapshot,
            tagged_objects: self.tagged_objects,
            tagged_players: self.tagged_players,
            triggering_event: self.triggering_event,
            chosen_modes: self.chosen_modes,
            cause: self.cause,
            mana_color_restriction: self.mana_color_restriction,
        }
    }

    /// Restrict mana color choices for effects executed in this context.
    pub fn with_mana_color_restriction(mut self, restriction: Option<Vec<Color>>) -> Self {
        self.mana_color_restriction = restriction;
        self
    }

    /// Snapshot all object targets for "last known information".
    /// Call this before executing effects that may exile/destroy targets.
    pub fn snapshot_targets(&mut self, game: &GameState) {
        for target in &self.targets {
            if let ResolvedTarget::Object(obj_id) = target
                && let Some(obj) = game.object(*obj_id)
            {
                self.target_snapshots
                    .insert(*obj_id, ObjectSnapshot::from_object(obj, game));
            }
        }
    }

    /// Set the defending player.
    pub fn with_defending_player(mut self, player: PlayerId) -> Self {
        self.defending_player = Some(player);
        self
    }

    /// Set the X value.
    pub fn with_x(mut self, x: u32) -> Self {
        self.x_value = Some(x);
        self
    }

    /// Set resolved targets.
    pub fn with_targets(mut self, targets: Vec<ResolvedTarget>) -> Self {
        self.targets = targets;
        self
    }

    /// Temporarily override `targets` while running a closure, then restore.
    pub fn with_temp_targets<R>(
        &mut self,
        targets: Vec<ResolvedTarget>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original_targets = std::mem::replace(&mut self.targets, targets);
        let result = f(self);
        self.targets = original_targets;
        result
    }

    /// Temporarily override `iterated_player` while running a closure, then restore.
    pub fn with_temp_iterated_player<R>(
        &mut self,
        iterated_player: Option<PlayerId>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original_iterated_player =
            std::mem::replace(&mut self.iterated_player, iterated_player);
        let result = f(self);
        self.iterated_player = original_iterated_player;
        result
    }

    /// Temporarily override `iterated_object` while running a closure, then restore.
    pub fn with_temp_iterated_object<R>(
        &mut self,
        iterated_object: Option<ObjectId>,
        f: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let original_iterated_object =
            std::mem::replace(&mut self.iterated_object, iterated_object);
        let result = f(self);
        self.iterated_object = original_iterated_object;
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

    /// Set tagged objects from a pre-existing map.
    ///
    /// This is used to pass tags between cost effects, where the first effect
    /// may tag an object (e.g., "choose a creature") and a subsequent effect
    /// needs to reference it (e.g., "sacrifice the chosen creature").
    pub fn with_tagged_objects(mut self, tags: HashMap<TagKey, Vec<ObjectSnapshot>>) -> Self {
        self.tagged_objects = tags;
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
        if let Some(snapshot) = event.snapshot() {
            let snapshots = vec![snapshot.clone()];
            self.set_tagged_objects("triggering", snapshots.clone());
            self.set_tagged_objects("it", snapshots);
        }
        if self.iterated_player.is_none() {
            self.iterated_player = event.player();
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

        self.triggering_event = Some(event);
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

    /// Store an effect result.
    pub fn store_result(&mut self, id: EffectId, result: EffectResult) {
        self.effect_results.insert(id, result);
    }

    /// Get a stored effect result.
    pub fn get_result(&self, id: EffectId) -> Option<&EffectResult> {
        self.effect_results.get(&id)
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
        let target_players = self
            .targets
            .iter()
            .filter_map(|target| match target {
                ResolvedTarget::Player(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut tagged_objects = self.tagged_objects.clone();
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
        let mut filter_ctx = game
            .filter_context_for(self.controller, Some(self.source))
            .with_iterated_player(self.iterated_player)
            .with_target_players(target_players)
            .with_tagged_objects(&tagged_objects);
        if self.defending_player.is_some() {
            filter_ctx.defending_player = self.defending_player;
        }
        if self.attacking_player.is_some() {
            filter_ctx.attacking_player = self.attacking_player;
        }
        filter_ctx
    }
}

// ============================================================================
// Value Resolution
// ============================================================================

/// Resolve a Value to a concrete i32.
pub fn resolve_value(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
) -> Result<i32, ExecutionError> {
    crate::effects::helpers::resolve_value(game, value, ctx)
}

// ============================================================================
// Target Validation
// ============================================================================

/// Validate that a resolved target matches a target spec.
pub fn validate_target(
    game: &GameState,
    target: &ResolvedTarget,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> bool {
    let filter_ctx = ctx.filter_context(game);

    match (target, spec) {
        (ResolvedTarget::Object(id), ChooseSpec::Object(filter)) => {
            if let Some(obj) = game.object(*id) {
                filter.matches(obj, &filter_ctx, game)
            } else {
                false
            }
        }
        (ResolvedTarget::Player(id), ChooseSpec::Player(filter)) => {
            game.can_target_player(*id) && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Player(id), ChooseSpec::PlayerOrPlaneswalker(filter)) => {
            game.can_target_player(*id) && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::PlayerOrPlaneswalker(_)) => game
            .object(*id)
            .is_some_and(|obj| obj.has_card_type(crate::types::CardType::Planeswalker)),
        (ResolvedTarget::Object(id), ChooseSpec::AnyTarget) => game.object(*id).is_some(),
        (ResolvedTarget::Player(id), ChooseSpec::AnyTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game()) && game.can_target_player(*id)
        }
        (ResolvedTarget::Object(id), ChooseSpec::SpecificObject(expected)) => id == expected,
        (ResolvedTarget::Player(id), ChooseSpec::SpecificPlayer(expected)) => id == expected,
        _ => false,
    }
}

// ============================================================================
// Effect Execution
// ============================================================================

/// Execute an effect and return the outcome (result + events).
pub fn execute_effect(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    // All effects implement EffectExecutor via the wrapped trait object
    effect.0.execute(game, ctx)
}

// ============================================================================
// Tests
// ============================================================================
