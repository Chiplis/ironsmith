//! Effect execution engine for MTG.
//!
//! This module provides the runtime execution of effects, including:
//! - Value resolution (X, counts, power/toughness, etc.)
//! - Target validation
//! - Effect execution with proper game state mutations

use std::collections::HashMap;

use crate::cost::OptionalCostsPaid;
use crate::decision::DecisionMaker;
use crate::effect::{Effect, EffectId, EffectOutcome, EffectResult, EventValueSpec, Value};
use crate::effects::helpers::resolve_objects_from_spec;
use crate::events::DamageEvent;
use crate::events::cause::EventCause;
use crate::events::combat::CreatureBecameBlockedEvent;
use crate::events::life::LifeGainEvent;
use crate::events::life::LifeLossEvent;
use crate::filter::ObjectRef;
use crate::game_event::DamageTarget;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::{SOURCE_EXILED_TAG, TagKey};
use crate::target::{ChooseSpec, FilterContext, PlayerFilter};

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
        }
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

/// Get the optional costs paid, preferring context but falling back to source object.
/// This allows ETB triggers to access kick count etc. from the permanent that entered.
fn get_optional_costs_paid<'a>(
    game: &'a GameState,
    ctx: &'a ExecutionContext,
) -> &'a OptionalCostsPaid {
    // If context has costs tracked, use those (for spell resolution)
    if !ctx.optional_costs_paid.costs.is_empty() {
        return &ctx.optional_costs_paid;
    }
    // Otherwise, try to get from the source object (for ETB triggers)
    if let Some(source) = game.object(ctx.source) {
        return &source.optional_costs_paid;
    }
    // Fallback to context (empty)
    &ctx.optional_costs_paid
}

fn candidate_ids_for_zone(game: &GameState, zone: Option<crate::zone::Zone>) -> Vec<ObjectId> {
    match zone {
        Some(crate::zone::Zone::Battlefield) => game.battlefield.clone(),
        Some(crate::zone::Zone::Graveyard) => game
            .players
            .iter()
            .flat_map(|player| player.graveyard.iter().copied())
            .collect(),
        Some(crate::zone::Zone::Hand) => game
            .players
            .iter()
            .flat_map(|player| player.hand.iter().copied())
            .collect(),
        Some(crate::zone::Zone::Library) => game
            .players
            .iter()
            .flat_map(|player| player.library.iter().copied())
            .collect(),
        Some(crate::zone::Zone::Stack) => game.stack.iter().map(|entry| entry.object_id).collect(),
        Some(crate::zone::Zone::Exile) => game.exile.clone(),
        Some(crate::zone::Zone::Command) => game.command_zone.clone(),
        None => game.battlefield.clone(),
    }
}

fn candidate_ids_for_filter(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
) -> Vec<ObjectId> {
    if let Some(zone) = filter.zone {
        return candidate_ids_for_zone(game, Some(zone));
    }
    if filter.any_of.is_empty() {
        return candidate_ids_for_zone(game, None);
    }

    let mut ids = std::collections::HashSet::new();
    for nested in &filter.any_of {
        for id in candidate_ids_for_zone(game, nested.zone) {
            ids.insert(id);
        }
    }
    if ids.is_empty() {
        candidate_ids_for_zone(game, None)
    } else {
        ids.into_iter().collect()
    }
}

/// Resolve a Value to a concrete i32.
pub fn resolve_value(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
) -> Result<i32, ExecutionError> {
    match value {
        Value::Fixed(n) => Ok(*n),
        Value::Add(left, right) => {
            Ok(resolve_value(game, left, ctx)? + resolve_value(game, right, ctx)?)
        }

        Value::X => ctx
            .x_value
            .map(|x| x as i32)
            .ok_or_else(|| ExecutionError::UnresolvableValue("X value not set".to_string())),

        Value::XTimes(multiplier) => ctx
            .x_value
            .map(|x| (x as i32) * multiplier)
            .ok_or_else(|| ExecutionError::UnresolvableValue("X value not set".to_string())),

        Value::Count(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let count = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            Ok(count as i32)
        }
        Value::CountScaled(filter, multiplier) => {
            let filter_ctx = ctx.filter_context(game);
            let count = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count() as i32;
            Ok(count * *multiplier)
        }
        Value::TotalPower(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let candidate_ids = candidate_ids_for_filter(game, filter);
            let total = candidate_ids
                .iter()
                .copied()
                .filter_map(|id| game.object(id).map(|obj| (id, obj)))
                .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                .map(|(id, obj)| {
                    game.calculated_power(id)
                        .or_else(|| obj.power())
                        .unwrap_or(0)
                })
                .sum();
            Ok(total)
        }
        Value::TotalToughness(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let candidate_ids = candidate_ids_for_filter(game, filter);
            let total = candidate_ids
                .iter()
                .copied()
                .filter_map(|id| game.object(id).map(|obj| (id, obj)))
                .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                .map(|(id, obj)| {
                    game.calculated_toughness(id)
                        .or_else(|| obj.toughness())
                        .unwrap_or(0)
                })
                .sum();
            Ok(total)
        }
        Value::TotalManaValue(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let candidate_ids = candidate_ids_for_filter(game, filter);
            let total = candidate_ids
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .map(|obj| {
                    obj.mana_cost
                        .as_ref()
                        .map(|cost| cost.mana_value() as i32)
                        .unwrap_or(0)
                })
                .sum();
            Ok(total)
        }
        Value::GreatestPower(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let candidate_ids = candidate_ids_for_filter(game, filter);
            let max = candidate_ids
                .iter()
                .copied()
                .filter_map(|id| game.object(id).map(|obj| (id, obj)))
                .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                .filter_map(|(id, obj)| game.calculated_power(id).or_else(|| obj.power()))
                .max()
                .unwrap_or(0);
            Ok(max)
        }
        Value::GreatestManaValue(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let candidate_ids = candidate_ids_for_filter(game, filter);
            let max = candidate_ids
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .filter_map(|obj| obj.mana_cost.as_ref().map(|cost| cost.mana_value() as i32))
                .max()
                .unwrap_or(0);
            Ok(max)
        }
        Value::BasicLandTypesAmong(filter) => {
            use std::collections::HashSet;

            let filter_ctx = ctx.filter_context(game);
            let mut seen = HashSet::new();
            for obj in game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
            {
                for subtype in &obj.subtypes {
                    if matches!(
                        subtype,
                        crate::types::Subtype::Plains
                            | crate::types::Subtype::Island
                            | crate::types::Subtype::Swamp
                            | crate::types::Subtype::Mountain
                            | crate::types::Subtype::Forest
                    ) {
                        seen.insert(subtype.clone());
                    }
                }
            }
            Ok(seen.len() as i32)
        }
        Value::ColorsAmong(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let mut has_white = false;
            let mut has_blue = false;
            let mut has_black = false;
            let mut has_red = false;
            let mut has_green = false;

            for obj in game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
            {
                let colors = obj.colors();
                has_white |= colors.contains(crate::color::Color::White);
                has_blue |= colors.contains(crate::color::Color::Blue);
                has_black |= colors.contains(crate::color::Color::Black);
                has_red |= colors.contains(crate::color::Color::Red);
                has_green |= colors.contains(crate::color::Color::Green);
            }

            Ok((has_white as i32)
                + (has_blue as i32)
                + (has_black as i32)
                + (has_red as i32)
                + (has_green as i32))
        }
        Value::DistinctNames(filter) => {
            use std::collections::HashSet;

            let filter_ctx = ctx.filter_context(game);
            let mut seen: HashSet<&str> = HashSet::new();
            for obj in game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
            {
                seen.insert(obj.name.as_str());
            }
            Ok(seen.len() as i32)
        }
        Value::CreaturesDiedThisTurn => Ok(game.creatures_died_this_turn as i32),
        Value::CreaturesDiedThisTurnControlledBy(player_filter) => {
            let filter_ctx = ctx.filter_context(game);
            let mut total = 0i32;
            for player in game.players.iter().filter(|p| p.is_in_game()) {
                if !player_filter.matches_player(player.id, &filter_ctx) {
                    continue;
                }
                total += game
                    .creatures_died_under_controller_this_turn
                    .get(&player.id)
                    .copied()
                    .unwrap_or(0) as i32;
            }
            Ok(total)
        }

        Value::CountPlayers(player_filter) => {
            let filter_ctx = ctx.filter_context(game);
            let count = game
                .players
                .iter()
                .filter(|p| p.is_in_game())
                .filter(|p| player_filter.matches_player(p.id, &filter_ctx))
                .count();
            Ok(count as i32)
        }
        Value::PartySize(player_filter) => {
            let player_id = resolve_player_filter(game, player_filter, ctx)?;
            let has_role = |role: crate::types::Subtype| {
                game.battlefield
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .any(|obj| {
                        obj.controller == player_id
                            && obj.has_card_type(crate::types::CardType::Creature)
                            && obj.has_subtype(role)
                    })
            };

            let mut size = 0i32;
            if has_role(crate::types::Subtype::Cleric) {
                size += 1;
            }
            if has_role(crate::types::Subtype::Rogue) {
                size += 1;
            }
            if has_role(crate::types::Subtype::Warrior) {
                size += 1;
            }
            if has_role(crate::types::Subtype::Wizard) {
                size += 1;
            }
            Ok(size)
        }

        Value::SourcePower => {
            let obj = game
                .object(ctx.source)
                .ok_or(ExecutionError::ObjectNotFound(ctx.source))?;
            game.calculated_power(ctx.source)
                .or_else(|| obj.power())
                .ok_or_else(|| ExecutionError::UnresolvableValue("Source has no power".to_string()))
        }

        Value::SourceToughness => {
            let obj = game
                .object(ctx.source)
                .ok_or(ExecutionError::ObjectNotFound(ctx.source))?;
            game.calculated_toughness(ctx.source)
                .or_else(|| obj.toughness())
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue("Source has no toughness".to_string())
                })
        }

        Value::PowerOf(_target_spec) => {
            let target_id = find_target_object(&ctx.targets)?;
            // Try to get current object, fall back to LKI snapshot
            if let Some(obj) = game.object(target_id) {
                game.calculated_power(target_id)
                    .or_else(|| obj.power())
                    .ok_or_else(|| {
                        ExecutionError::UnresolvableValue("Target has no power".to_string())
                    })
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                snapshot.power.ok_or_else(|| {
                    ExecutionError::UnresolvableValue("Target had no power".to_string())
                })
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }

        Value::ToughnessOf(_target_spec) => {
            let target_id = find_target_object(&ctx.targets)?;
            // Try to get current object, fall back to LKI snapshot
            if let Some(obj) = game.object(target_id) {
                game.calculated_toughness(target_id)
                    .or_else(|| obj.toughness())
                    .ok_or_else(|| {
                        ExecutionError::UnresolvableValue("Target has no toughness".to_string())
                    })
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                snapshot.toughness.ok_or_else(|| {
                    ExecutionError::UnresolvableValue("Target had no toughness".to_string())
                })
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }

        Value::ManaValueOf(_target_spec) => {
            let target_id = find_target_object(&ctx.targets)?;
            if let Some(obj) = game.object(target_id) {
                obj.mana_cost
                    .as_ref()
                    .map(|cost| cost.mana_value() as i32)
                    .ok_or_else(|| {
                        ExecutionError::UnresolvableValue("Target has no mana value".to_string())
                    })
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                snapshot
                    .mana_cost
                    .as_ref()
                    .map(|cost| cost.mana_value() as i32)
                    .ok_or_else(|| {
                        ExecutionError::UnresolvableValue("Target had no mana value".to_string())
                    })
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }

        Value::LifeTotal(player_spec) => {
            let player_id = resolve_player_filter(game, player_spec, ctx)?;
            let player = game
                .player(player_id)
                .ok_or(ExecutionError::PlayerNotFound(player_id))?;
            Ok(player.life)
        }
        Value::HalfLifeTotalRoundedUp(player_spec) => {
            let player_id = resolve_player_filter(game, player_spec, ctx)?;
            let player = game
                .player(player_id)
                .ok_or(ExecutionError::PlayerNotFound(player_id))?;
            Ok((player.life + 1).div_euclid(2))
        }
        Value::HalfLifeTotalRoundedDown(player_spec) => {
            let player_id = resolve_player_filter(game, player_spec, ctx)?;
            let player = game
                .player(player_id)
                .ok_or(ExecutionError::PlayerNotFound(player_id))?;
            Ok(player.life.div_euclid(2))
        }

        Value::CardsInHand(player_spec) => {
            let player_id = resolve_player_filter(game, player_spec, ctx)?;
            let player = game
                .player(player_id)
                .ok_or(ExecutionError::PlayerNotFound(player_id))?;
            Ok(player.hand.len() as i32)
        }

        Value::LifeGainedThisTurn(player_spec) => {
            let player_ids =
                resolve_player_filter_to_list(game, player_spec, &ctx.filter_context(game), ctx)?;
            let total: u32 = player_ids
                .iter()
                .map(|pid| game.life_gained_this_turn.get(pid).copied().unwrap_or(0))
                .sum();
            Ok(total as i32)
        }

        Value::NoncombatDamageDealtToPlayersThisTurn(player_spec) => {
            let player_ids =
                resolve_player_filter_to_list(game, player_spec, &ctx.filter_context(game), ctx)?;
            let total: u32 = player_ids
                .iter()
                .map(|pid| {
                    game.noncombat_damage_to_players_this_turn
                        .get(pid)
                        .copied()
                        .unwrap_or(0)
                })
                .sum();
            Ok(total as i32)
        }

        Value::MaxCardsInHand(player_spec) => {
            let player_ids =
                resolve_player_filter_to_list(game, player_spec, &ctx.filter_context(game), ctx)?;
            let mut max_count: Option<i32> = None;
            for pid in player_ids {
                let player = game
                    .player(pid)
                    .ok_or(ExecutionError::PlayerNotFound(pid))?;
                let count = player.hand.len() as i32;
                max_count = Some(max_count.map_or(count, |prev| prev.max(count)));
            }
            Ok(max_count.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "MaxCardsInHand requires a matching player".to_string(),
                )
            })?)
        }

        Value::CardsInGraveyard(player_spec) => {
            let player_id = resolve_player_filter(game, player_spec, ctx)?;
            let player = game
                .player(player_id)
                .ok_or(ExecutionError::PlayerNotFound(player_id))?;
            Ok(player.graveyard.len() as i32)
        }

        Value::SpellsCastThisTurn(player_spec) => {
            let player_ids =
                resolve_player_filter_to_list(game, player_spec, &ctx.filter_context(game), ctx)?;
            let count: u32 = player_ids
                .iter()
                .map(|pid| game.spells_cast_this_turn.get(pid).copied().unwrap_or(0))
                .sum();
            Ok(count as i32)
        }

        Value::SpellsCastBeforeThisTurn(player_spec) => {
            let player_ids =
                resolve_player_filter_to_list(game, player_spec, &ctx.filter_context(game), ctx)?;
            let count: i32 = player_ids
                .iter()
                .map(|pid| game.spells_cast_this_turn.get(pid).copied().unwrap_or(0) as i32)
                .sum();
            Ok((count - 1).max(0))
        }

        Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let player_ids =
                resolve_player_filter_to_list(game, player, &ctx.filter_context(game), ctx)?;
            let filter_ctx = ctx.filter_context(game);
            let mut count: i32 = 0;
            for snapshot in &game.spells_cast_this_turn_snapshots {
                if *exclude_source && snapshot.object_id == ctx.source {
                    continue;
                }
                if !player_ids.iter().any(|pid| *pid == snapshot.controller) {
                    continue;
                }
                if filter.matches_snapshot(snapshot, &filter_ctx, game) {
                    count = count.saturating_add(1);
                }
            }
            Ok(count)
        }

        Value::CardTypesInGraveyard(player_spec) => {
            use crate::types::CardType;

            let player_id = resolve_player_filter(game, player_spec, ctx)?;
            let player = game
                .player(player_id)
                .ok_or(ExecutionError::PlayerNotFound(player_id))?;

            let mut types: Vec<CardType> = Vec::new();
            for &card_id in &player.graveyard {
                let Some(obj) = game.object(card_id) else {
                    continue;
                };
                for card_type in &obj.card_types {
                    if !types.contains(card_type) {
                        types.push(*card_type);
                    }
                }
            }

            Ok(types.len() as i32)
        }

        Value::Devotion { player, color } => {
            let player_ids =
                resolve_player_filter_to_list(game, player, &ctx.filter_context(game), ctx)?;
            let devotion: usize = player_ids
                .iter()
                .map(|pid| game.devotion_to_color(*pid, *color))
                .sum();
            Ok(devotion as i32)
        }

        Value::ColorsOfManaSpentToCastThisSpell => {
            let Some(source_obj) = game.object(ctx.source) else {
                return Ok(0);
            };
            let spent = &source_obj.mana_spent_to_cast;
            let distinct_colors = [
                spent.white > 0,
                spent.blue > 0,
                spent.black > 0,
                spent.red > 0,
                spent.green > 0,
            ]
            .into_iter()
            .filter(|present| *present)
            .count();
            Ok(distinct_colors as i32)
        }

        Value::EffectValue(effect_id) => {
            let result = ctx
                .get_result(*effect_id)
                .ok_or(ExecutionError::EffectNotFound(*effect_id))?;
            Ok(result.count_or_zero())
        }
        Value::EffectValueOffset(effect_id, offset) => {
            let result = ctx
                .get_result(*effect_id)
                .ok_or(ExecutionError::EffectNotFound(*effect_id))?;
            Ok(result.count_or_zero() + *offset)
        }

        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => {
            let Some(triggering_event) = &ctx.triggering_event else {
                return Err(ExecutionError::UnresolvableValue(
                    "EventValue(Amount) requires a triggering event".to_string(),
                ));
            };
            if let Some(life_loss_event) = triggering_event.downcast::<LifeLossEvent>() {
                return Ok(life_loss_event.amount as i32);
            }
            if let Some(life_gain_event) = triggering_event.downcast::<LifeGainEvent>() {
                return Ok(life_gain_event.amount as i32);
            }
            if let Some(damage_event) = triggering_event.downcast::<DamageEvent>() {
                return Ok(damage_event.amount as i32);
            }
            Err(ExecutionError::UnresolvableValue(
                "EventValue(Amount) requires a life gain/loss or damage event".to_string(),
            ))
        }

        Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier }) => {
            let Some(triggering_event) = &ctx.triggering_event else {
                return Err(ExecutionError::UnresolvableValue(
                    "EventValue(BlockersBeyondFirst) requires a triggering event".to_string(),
                ));
            };
            if let Some(event) = triggering_event.downcast::<CreatureBecameBlockedEvent>() {
                let beyond_first = event.blocker_count.saturating_sub(1) as i32;
                return Ok(beyond_first * *multiplier);
            }
            Err(ExecutionError::UnresolvableValue(
                "EventValue(BlockersBeyondFirst) requires a creature-becomes-blocked event"
                    .to_string(),
            ))
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            let Some(triggering_event) = &ctx.triggering_event else {
                return Err(ExecutionError::UnresolvableValue(
                    "EventValue(Amount) requires a triggering event".to_string(),
                ));
            };
            let base = if let Some(life_loss_event) = triggering_event.downcast::<LifeLossEvent>() {
                life_loss_event.amount as i32
            } else if let Some(life_gain_event) = triggering_event.downcast::<LifeGainEvent>() {
                life_gain_event.amount as i32
            } else if let Some(damage_event) = triggering_event.downcast::<DamageEvent>() {
                damage_event.amount as i32
            } else {
                return Err(ExecutionError::UnresolvableValue(
                    "EventValue(Amount) requires a life gain/loss or damage event".to_string(),
                ));
            };
            Ok(base + *offset)
        }
        Value::EventValueOffset(EventValueSpec::BlockersBeyondFirst { multiplier }, offset) => {
            let Some(triggering_event) = &ctx.triggering_event else {
                return Err(ExecutionError::UnresolvableValue(
                    "EventValue(BlockersBeyondFirst) requires a triggering event".to_string(),
                ));
            };
            if let Some(event) = triggering_event.downcast::<CreatureBecameBlockedEvent>() {
                let beyond_first = event.blocker_count.saturating_sub(1) as i32;
                return Ok((beyond_first * *multiplier) + *offset);
            }
            Err(ExecutionError::UnresolvableValue(
                "EventValue(BlockersBeyondFirst) requires a creature-becomes-blocked event"
                    .to_string(),
            ))
        }

        Value::WasKicked => {
            // Check if kicker or multikicker was paid
            // First check ctx, then fall back to source object (for ETB triggers)
            let paid = get_optional_costs_paid(game, ctx);
            Ok(if paid.was_kicked() { 1 } else { 0 })
        }

        Value::WasBoughtBack => {
            // Check if buyback was paid
            let paid = get_optional_costs_paid(game, ctx);
            Ok(if paid.was_bought_back() { 1 } else { 0 })
        }

        Value::WasEntwined => {
            // Check if entwine was paid
            let paid = get_optional_costs_paid(game, ctx);
            Ok(if paid.was_entwined() { 1 } else { 0 })
        }

        Value::WasPaid(index) => {
            // Check if the optional cost at the given index was paid
            let paid = get_optional_costs_paid(game, ctx);
            Ok(if paid.was_paid(*index) { 1 } else { 0 })
        }

        Value::WasPaidLabel(label) => {
            // Check if the optional cost with the given label was paid
            let paid = get_optional_costs_paid(game, ctx);
            Ok(if paid.was_paid_label(label) { 1 } else { 0 })
        }

        Value::TimesPaid(index) => {
            // Get the number of times the optional cost was paid
            let paid = get_optional_costs_paid(game, ctx);
            Ok(paid.times_paid(*index) as i32)
        }

        Value::TimesPaidLabel(label) => {
            // Get the number of times the optional cost with the label was paid
            let paid = get_optional_costs_paid(game, ctx);
            Ok(paid.times_paid_label(label) as i32)
        }

        Value::KickCount => {
            // Get the number of times the kicker was paid
            let paid = get_optional_costs_paid(game, ctx);
            Ok(paid.kick_count() as i32)
        }
        Value::CountersOnSource(counter_type) => {
            // Get the number of counters of the specified type on the source
            if let Some(source) = game.object(ctx.source) {
                Ok(source.counters.get(counter_type).copied().unwrap_or(0) as i32)
            } else {
                Ok(0)
            }
        }
        Value::CountersOn(spec, counter_type) => {
            let object_ids = resolve_objects_from_spec(game, spec, ctx)?;
            let total = object_ids
                .into_iter()
                .filter_map(|id| game.object(id))
                .map(|obj| {
                    if let Some(counter_type) = counter_type {
                        obj.counters.get(counter_type).copied().unwrap_or(0) as i32
                    } else {
                        obj.counters.values().map(|count| *count as i32).sum()
                    }
                })
                .sum();
            Ok(total)
        }

        Value::TaggedCount => {
            // Get the count of tagged objects for the current controller
            // (set by ForEachControllerOfTaggedEffect during iteration)
            if let Some(result) = ctx.get_result(crate::effect::EffectId::TAGGED_COUNT) {
                Ok(result.count_or_zero())
            } else {
                Err(ExecutionError::UnresolvableValue(
                    "TaggedCount used outside ForEachControllerOfTagged loop".to_string(),
                ))
            }
        }
    }
}

/// Resolve a PlayerFilter to a concrete PlayerId.
fn resolve_player_filter(
    game: &GameState,
    spec: &PlayerFilter,
    ctx: &ExecutionContext,
) -> Result<PlayerId, ExecutionError> {
    match spec {
        PlayerFilter::You => Ok(ctx.controller),
        PlayerFilter::Any => {
            // "Any" player needs resolution from targets or defaults to controller
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(*id);
                }
            }
            Ok(ctx.controller)
        }
        PlayerFilter::NotYou => {
            for player in game.players.iter() {
                if player.id != ctx.controller && player.is_in_game() {
                    return Ok(player.id);
                }
            }
            Err(ExecutionError::UnresolvableValue(
                "NotYou filter requires another in-game player".to_string(),
            ))
        }
        PlayerFilter::Opponent => {
            // Single opponent - try to find one from targets, otherwise error
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(*id);
                }
            }
            Err(ExecutionError::UnresolvableValue(
                "Opponent filter requires a targeted player".to_string(),
            ))
        }
        PlayerFilter::Teammate => Err(ExecutionError::UnresolvableValue(
            "Teammate filter not supported in 2-player games".to_string(),
        )),
        PlayerFilter::Attacking => ctx.attacking_player.ok_or_else(|| {
            ExecutionError::UnresolvableValue("AttackingPlayer not set".to_string())
        }),
        PlayerFilter::DamagedPlayer => {
            let Some(triggering_event) = &ctx.triggering_event else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires a triggering event".to_string(),
                ));
            };
            let Some(damage_event) = triggering_event.downcast::<DamageEvent>() else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires a damage triggering event".to_string(),
                ));
            };
            let DamageTarget::Player(player_id) = damage_event.target else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires damage dealt to a player".to_string(),
                ));
            };
            Ok(player_id)
        }
        PlayerFilter::Target(_) => {
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(*id);
                }
            }
            Err(ExecutionError::InvalidTarget)
        }
        PlayerFilter::Excluding { .. } => {
            let filter_ctx = ctx.filter_context(game);
            let mut players = resolve_player_filter_to_list(game, spec, &filter_ctx, ctx)?;
            players
                .drain(..)
                .next()
                .ok_or_else(|| ExecutionError::UnresolvableValue("No matching players".to_string()))
        }
        PlayerFilter::Specific(id) => Ok(*id),
        PlayerFilter::ControllerOf(obj_ref) => resolve_controller_of(game, ctx, obj_ref),
        PlayerFilter::OwnerOf(obj_ref) => resolve_owner_of(game, ctx, obj_ref),
        PlayerFilter::Active => Ok(game.turn.active_player),
        PlayerFilter::Defending => ctx.defending_player.ok_or_else(|| {
            ExecutionError::UnresolvableValue("DefendingPlayer not set".to_string())
        }),
        PlayerFilter::IteratedPlayer => ctx.iterated_player.ok_or_else(|| {
            ExecutionError::UnresolvableValue(
                "IteratedPlayer not set (must be inside ForEachOpponent/ForEachPlayer)".to_string(),
            )
        }),
    }
}

/// Resolve a PlayerFilter to a list of PlayerIds.
fn resolve_player_filter_to_list(
    game: &GameState,
    filter: &PlayerFilter,
    _filter_ctx: &FilterContext,
    ctx: &ExecutionContext,
) -> Result<Vec<PlayerId>, ExecutionError> {
    match filter {
        PlayerFilter::You => Ok(vec![ctx.controller]),
        PlayerFilter::Any | PlayerFilter::Target(_) => {
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(vec![*id]);
                }
            }
            if matches!(filter, PlayerFilter::Any) {
                return Ok(game
                    .players
                    .iter()
                    .filter(|p| p.is_in_game())
                    .map(|p| p.id)
                    .collect());
            }
            Err(ExecutionError::InvalidTarget)
        }
        PlayerFilter::NotYou => Ok(game
            .players
            .iter()
            .filter(|p| p.id != ctx.controller && p.is_in_game())
            .map(|p| p.id)
            .collect()),
        PlayerFilter::Opponent => Ok(game
            .players
            .iter()
            .filter(|p| p.id != ctx.controller && p.is_in_game())
            .map(|p| p.id)
            .collect()),
        PlayerFilter::Specific(id) => Ok(vec![*id]),
        PlayerFilter::Active => Ok(vec![game.turn.active_player]),
        PlayerFilter::Defending => ctx.defending_player.map(|id| vec![id]).ok_or_else(|| {
            ExecutionError::UnresolvableValue("DefendingPlayer not set".to_string())
        }),
        PlayerFilter::Attacking => ctx.attacking_player.map(|id| vec![id]).ok_or_else(|| {
            ExecutionError::UnresolvableValue("AttackingPlayer not set".to_string())
        }),
        PlayerFilter::DamagedPlayer => {
            let Some(triggering_event) = &ctx.triggering_event else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires a triggering event".to_string(),
                ));
            };
            let Some(damage_event) = triggering_event.downcast::<DamageEvent>() else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires a damage triggering event".to_string(),
                ));
            };
            let DamageTarget::Player(player_id) = damage_event.target else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires damage dealt to a player".to_string(),
                ));
            };
            Ok(vec![player_id])
        }
        PlayerFilter::IteratedPlayer => ctx
            .iterated_player
            .map(|id| vec![id])
            .ok_or_else(|| ExecutionError::UnresolvableValue("IteratedPlayer not set".to_string())),
        PlayerFilter::Excluding { base, excluded } => {
            let mut base_players = resolve_player_filter_to_list(game, base, _filter_ctx, ctx)?;
            let excluded_players = resolve_player_filter_to_list(game, excluded, _filter_ctx, ctx)?;
            base_players.retain(|id| !excluded_players.contains(id));
            Ok(base_players)
        }
        PlayerFilter::ControllerOf(object_ref) => {
            Ok(vec![resolve_controller_of(game, ctx, object_ref)?])
        }
        PlayerFilter::OwnerOf(object_ref) => Ok(vec![resolve_owner_of(game, ctx, object_ref)?]),
        PlayerFilter::Teammate => Err(ExecutionError::UnresolvableValue(
            "Teammate filter not supported".to_string(),
        )),
    }
}

/// Find the first object target in the targets list.
fn find_target_object(targets: &[ResolvedTarget]) -> Result<ObjectId, ExecutionError> {
    for target in targets {
        if let ResolvedTarget::Object(id) = target {
            return Ok(*id);
        }
    }
    Err(ExecutionError::InvalidTarget)
}

/// Find the first player target in the targets list.
#[allow(dead_code)]
fn find_target_player(targets: &[ResolvedTarget]) -> Result<PlayerId, ExecutionError> {
    for target in targets {
        if let ResolvedTarget::Player(id) = target {
            return Ok(*id);
        }
    }
    Err(ExecutionError::InvalidTarget)
}

/// Resolve ControllerOf(ObjectRef) to a PlayerId.
///
/// Handles all three ObjectRef variants:
/// - `Target`: Uses the first object target in the targets list
/// - `Specific(id)`: Uses a specific object by ID
/// - `Tagged(tag)`: Uses a tagged object from a prior effect
fn resolve_controller_of(
    game: &GameState,
    ctx: &ExecutionContext,
    obj_ref: &ObjectRef,
) -> Result<PlayerId, ExecutionError> {
    match obj_ref {
        ObjectRef::Target => {
            // Find the first object target
            let target_id = find_target_object(&ctx.targets)?;
            // Try to get current object, fall back to LKI snapshot
            if let Some(obj) = game.object(target_id) {
                Ok(obj.controller)
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                Ok(snapshot.controller)
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }
        ObjectRef::Specific(object_id) => {
            // Use a specific object by ID
            if let Some(obj) = game.object(*object_id) {
                Ok(obj.controller)
            } else if let Some(snapshot) = ctx.target_snapshots.get(object_id) {
                Ok(snapshot.controller)
            } else {
                Err(ExecutionError::ObjectNotFound(*object_id))
            }
        }
        ObjectRef::Tagged(tag) => {
            // Use a tagged object from a prior effect (uses first if multiple)
            if let Some(snapshot) = ctx.get_tagged(tag) {
                Ok(snapshot.controller)
            } else {
                Err(ExecutionError::TagNotFound(tag.to_string()))
            }
        }
    }
}

/// Resolve OwnerOf(ObjectRef) to a PlayerId.
///
/// Handles all three ObjectRef variants:
/// - `Target`: Uses the first object target in the targets list
/// - `Specific(id)`: Uses a specific object by ID
/// - `Tagged(tag)`: Uses a tagged object from a prior effect
fn resolve_owner_of(
    game: &GameState,
    ctx: &ExecutionContext,
    obj_ref: &ObjectRef,
) -> Result<PlayerId, ExecutionError> {
    match obj_ref {
        ObjectRef::Target => {
            // Find the first object target
            let target_id = find_target_object(&ctx.targets)?;
            // Try to get current object, fall back to LKI snapshot
            if let Some(obj) = game.object(target_id) {
                Ok(obj.owner)
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                Ok(snapshot.owner)
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }
        ObjectRef::Specific(object_id) => {
            // Use a specific object by ID
            if let Some(obj) = game.object(*object_id) {
                Ok(obj.owner)
            } else if let Some(snapshot) = ctx.target_snapshots.get(object_id) {
                Ok(snapshot.owner)
            } else {
                Err(ExecutionError::ObjectNotFound(*object_id))
            }
        }
        ObjectRef::Tagged(tag) => {
            // Use a tagged object from a prior effect (uses first if multiple)
            if let Some(snapshot) = ctx.get_tagged(tag) {
                Ok(snapshot.owner)
            } else {
                Err(ExecutionError::TagNotFound(tag.to_string()))
            }
        }
    }
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
