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
use crate::tag::TagKey;
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
        game.filter_context_for(self.controller, Some(self.source))
            .with_iterated_player(self.iterated_player)
            .with_target_players(target_players)
            .with_tagged_objects(&self.tagged_objects)
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

/// Resolve a Value to a concrete i32.
pub fn resolve_value(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
) -> Result<i32, ExecutionError> {
    match value {
        Value::Fixed(n) => Ok(*n),

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
        Value::CreaturesDiedThisTurn => Ok(game.creatures_died_this_turn as i32),

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

        Value::CardsInHand(player_spec) => {
            let player_id = resolve_player_filter(game, player_spec, ctx)?;
            let player = game
                .player(player_id)
                .ok_or(ExecutionError::PlayerNotFound(player_id))?;
            Ok(player.hand.len() as i32)
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
            filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Player(id), ChooseSpec::PlayerOrPlaneswalker(filter)) => {
            filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::PlayerOrPlaneswalker(_)) => game
            .object(*id)
            .is_some_and(|obj| obj.has_card_type(crate::types::CardType::Planeswalker)),
        (ResolvedTarget::Object(id), ChooseSpec::AnyTarget) => game.object(*id).is_some(),
        (ResolvedTarget::Player(id), ChooseSpec::AnyTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cost::{OptionalCost, TotalCost};
    use crate::effect::EffectPredicate;
    use crate::events::combat::CreatureBecameBlockedEvent;
    use crate::events::life::LifeLossEvent;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::target::ObjectFilter;
    use crate::triggers::TriggerEvent;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn test_resolve_fixed_value() {
        let game = setup_game();
        let ctx = ExecutionContext::new_default(ObjectId::from_raw(1), PlayerId::from_index(0));

        let value = Value::Fixed(5);
        let result = resolve_value(&game, &value, &ctx);
        assert_eq!(result.unwrap(), 5);
    }

    #[test]
    fn test_resolve_x_value() {
        let game = setup_game();
        let ctx =
            ExecutionContext::new_default(ObjectId::from_raw(1), PlayerId::from_index(0)).with_x(3);

        let value = Value::X;
        let result = resolve_value(&game, &value, &ctx);
        assert_eq!(result.unwrap(), 3);

        let value = Value::XTimes(2);
        let result = resolve_value(&game, &value, &ctx);
        assert_eq!(result.unwrap(), 6);
    }

    #[test]
    fn test_resolve_x_value_not_set() {
        let game = setup_game();
        let ctx = ExecutionContext::new_default(ObjectId::from_raw(1), PlayerId::from_index(0));

        let value = Value::X;
        let result = resolve_value(&game, &value, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_count_value() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        create_creature(&mut game, "Bear 1", alice);
        create_creature(&mut game, "Bear 2", alice);

        let source = create_creature(&mut game, "Source", alice);
        let ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::creature().you_control();
        let value = Value::Count(filter);
        let result = resolve_value(&game, &value, &ctx);
        assert_eq!(result.unwrap(), 3);
    }

    #[test]
    fn test_resolve_event_value_amount_from_life_loss_event() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        let event = TriggerEvent::new(LifeLossEvent::new(alice, 4, true));
        let ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);

        let result = resolve_value(&game, &Value::EventValue(EventValueSpec::Amount), &ctx);
        assert_eq!(result.unwrap(), 4);
    }

    #[test]
    fn test_resolve_event_value_blockers_beyond_first() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(7);
        let event = TriggerEvent::new(CreatureBecameBlockedEvent::new(source, 3));
        let ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);

        let value = Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier: 2 });
        let result = resolve_value(&game, &value, &ctx);
        assert_eq!(result.unwrap(), 4);
    }

    #[test]
    fn test_execute_gain_life() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = Effect::gain_life(5);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Count(5));
        assert_eq!(game.player(alice).unwrap().life, 25);
    }

    #[test]
    fn test_execute_lose_life() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = Effect::lose_life(3);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).unwrap().life, 17);
    }

    #[test]
    fn test_execute_draw_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add cards to library
        for i in 1..=5 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Instant])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = Effect::draw(2);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        assert_eq!(game.player(alice).unwrap().library.len(), 3);
    }

    #[test]
    fn test_execute_tap() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = create_creature(&mut game, "Target", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        assert!(!game.is_tapped(creature_id));

        let effect = Effect::tap(ChooseSpec::creature());

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Resolved);
        assert!(game.is_tapped(creature_id));
    }

    #[test]
    fn test_execute_untap() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = create_creature(&mut game, "Target", alice);
        game.tap(creature_id);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = Effect::untap(ChooseSpec::creature());

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Resolved);
        assert!(!game.is_tapped(creature_id));
    }

    #[test]
    fn test_execute_add_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = Effect::add_mana(vec![ManaSymbol::Green, ManaSymbol::Green]);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert!(matches!(result.result, EffectResult::ManaAdded(_)));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 2);
    }

    #[test]
    fn test_execute_put_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = create_creature(&mut game, "Target", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = Effect::plus_one_counters(2, ChooseSpec::creature());

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Count(2));

        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature.counters.get(&CounterType::PlusOnePlusOne),
            Some(&2)
        );
        assert_eq!(creature.power(), Some(4)); // 2 base + 2 counters
        assert_eq!(creature.toughness(), Some(4));
    }

    #[test]
    fn test_execute_deal_damage_to_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_id = create_creature(&mut game, "Source", alice);

        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);

        let effect = Effect::new(crate::effects::DealDamageEffect {
            amount: Value::Fixed(3),
            target: ChooseSpec::any_player(),
            source_is_combat: false,
        });

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(bob).unwrap().life, 17);
    }

    #[test]
    fn test_execute_deal_damage_to_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_id = create_creature(&mut game, "Source", alice);
        let target_id = create_creature(&mut game, "Target", bob);

        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_targets(vec![ResolvedTarget::Object(target_id)]);

        let effect = Effect::new(crate::effects::DealDamageEffect {
            amount: Value::Fixed(2),
            target: ChooseSpec::creature(),
            source_is_combat: false,
        });

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.damage_on(target_id), 2);
    }

    #[test]
    fn test_execute_with_id_and_if() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // First, execute an effect with an ID
        let effect1 = Effect::with_id(0, Effect::gain_life(3));

        execute_effect(&mut game, &effect1, &mut ctx).unwrap();

        // Then, execute an If effect that checks the previous result
        let effect2 = Effect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(2)],
        );

        execute_effect(&mut game, &effect2, &mut ctx).unwrap();

        // Should have gained 3 + 2 = 5 life
        assert_eq!(game.player(alice).unwrap().life, 25);
    }

    #[test]
    fn test_validate_target_object() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = create_creature(&mut game, "Target", alice);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);

        let target = ResolvedTarget::Object(creature_id);
        let spec = ChooseSpec::creature();

        assert!(validate_target(&game, &target, &spec, &ctx));

        // Land filter should not match creature
        let land_spec = ChooseSpec::Object(ObjectFilter::land());
        assert!(!validate_target(&game, &target, &land_spec, &ctx));
    }

    #[test]
    fn test_validate_target_player() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(1);
        let ctx = ExecutionContext::new_default(source, alice);

        let target = ResolvedTarget::Player(bob);

        // Any player should match
        let any_player_spec = ChooseSpec::any_player();
        assert!(validate_target(&game, &target, &any_player_spec, &ctx));

        // "You" should not match opponent
        let you_spec = ChooseSpec::you();
        assert!(!validate_target(&game, &target, &you_spec, &ctx));
    }

    #[test]
    fn test_execute_monstrosity() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature
        let creature_id = create_creature(&mut game, "Dragon", alice);

        // Verify not monstrous initially
        assert!(!game.is_monstrous(creature_id));
        assert_eq!(
            game.object(creature_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            None
        );

        // Create context with the creature as source (monstrosity targets self)
        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        // Execute monstrosity 3
        let effect = Effect::monstrosity(3);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Verify result
        assert!(matches!(
            result.result,
            EffectResult::MonstrosityApplied { creature, n } if creature == creature_id && n == 3
        ));

        // Verify creature is now monstrous with 3 +1/+1 counters
        assert!(game.is_monstrous(creature_id));
        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature.counters.get(&CounterType::PlusOnePlusOne),
            Some(&3)
        );
        // 2/2 + 3 counters = 5/5
        assert_eq!(creature.power(), Some(5));
        assert_eq!(creature.toughness(), Some(5));
    }

    #[test]
    fn test_execute_monstrosity_already_monstrous() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature that's already monstrous
        let creature_id = create_creature(&mut game, "Dragon", alice);
        game.set_monstrous(creature_id);
        game.object_mut(creature_id)
            .unwrap()
            .counters
            .insert(CounterType::PlusOnePlusOne, 3);

        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        // Try to execute monstrosity again
        let effect = Effect::monstrosity(5);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Should return Count(0) indicating nothing happened
        assert_eq!(result.result, EffectResult::Count(0));

        // Counters should not have changed
        assert!(game.is_monstrous(creature_id));
        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature.counters.get(&CounterType::PlusOnePlusOne),
            Some(&3)
        );
    }

    #[test]
    fn test_execute_for_each_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Both players start at 20 life
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // ForEachOpponent: deal 3 damage to each opponent
        let effect =
            Effect::for_each_opponent(vec![Effect::new(crate::effects::DealDamageEffect {
                amount: Value::Fixed(3),
                target: ChooseSpec::Player(crate::target::PlayerFilter::IteratedPlayer),
                source_is_combat: false,
            })]);
        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Should return total damage dealt (3 to 1 opponent = 3)
        assert_eq!(result.result, EffectResult::Count(3));

        // Alice (controller) shouldn't have taken damage
        assert_eq!(game.player(alice).unwrap().life, 20);

        // Bob (opponent) should have taken 3 damage
        assert_eq!(game.player(bob).unwrap().life, 17);
    }

    #[test]
    fn test_execute_for_each_opponent_uses_iterated_player() {
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::mana::ManaCost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Give Bob some cards in hand
        for i in 1..=5 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Instant])
                .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
                .build();
            game.create_object_from_card(&card, bob, Zone::Hand);
        }
        assert_eq!(game.player(bob).unwrap().hand.len(), 5);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // ForEachOpponent: deal damage equal to cards in that player's hand
        // (This is what Stormbreath Dragon's trigger does)
        let effect =
            Effect::for_each_opponent(vec![Effect::new(crate::effects::DealDamageEffect {
                amount: Value::CardsInHand(PlayerFilter::IteratedPlayer),
                target: ChooseSpec::Player(crate::target::PlayerFilter::IteratedPlayer),
                source_is_combat: false,
            })]);
        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Should deal 5 damage (Bob has 5 cards)
        assert_eq!(result.result, EffectResult::Count(5));

        // Bob should have taken 5 damage
        assert_eq!(game.player(bob).unwrap().life, 15);
    }

    #[test]
    fn test_execute_create_token_enters_tapped() {
        use crate::card::PowerToughness;
        use crate::cards::CardDefinitionBuilder;
        use crate::color::ColorSet;
        use crate::effects::CreateTokenEffect;
        use crate::ids::CardId;
        use crate::object::ObjectKind;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = create_creature(&mut game, "Source", alice);
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Token definition - just describes what the token IS
        let angel_token = CardDefinitionBuilder::new(CardId::new(), "Angel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .color_indicator(ColorSet::from(crate::color::Color::White))
            .power_toughness(PowerToughness::fixed(4, 4))
            .flying()
            .build();

        // Effect describes HOW the token enters (tapped)
        let effect = Effect::new(CreateTokenEffect::one(angel_token).tapped());

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Should have created one token
        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let token_id = ids[0];

            // Verify token properties
            let token = game.object(token_id).unwrap();
            assert_eq!(token.name, "Angel");
            assert_eq!(token.kind, ObjectKind::Token);
            assert!(game.is_tapped(token_id), "Token should enter tapped");
            assert_eq!(token.power(), Some(4));
            assert_eq!(token.toughness(), Some(4));
            assert!(token.has_card_type(CardType::Creature));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_execute_create_token_with_exile_at_end_of_combat() {
        use crate::card::PowerToughness;
        use crate::cards::CardDefinitionBuilder;
        use crate::color::ColorSet;
        use crate::effects::CreateTokenEffect;
        use crate::ids::CardId;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = create_creature(&mut game, "Geist", alice);
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Token definition - just describes what the token IS
        let angel_token = CardDefinitionBuilder::new(CardId::new(), "Angel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .color_indicator(ColorSet::from(crate::color::Color::White))
            .power_toughness(PowerToughness::fixed(4, 4))
            .flying()
            .build();

        // Effect describes HOW the token enters (tapped, exiled at EOC)
        let effect = Effect::new(
            CreateTokenEffect::one(angel_token)
                .tapped()
                .exile_at_end_of_combat(),
        );

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Should have created one token
        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let token_id = ids[0];

            // Verify a delayed trigger was registered for end of combat exile
            assert_eq!(game.delayed_triggers.len(), 1);
            let delayed = &game.delayed_triggers[0];
            assert!(delayed.trigger.display().contains("end of combat"));
            assert_eq!(delayed.target_objects, vec![token_id]);
            assert!(delayed.one_shot);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_execute_create_token_enters_attacking() {
        use crate::card::PowerToughness;
        use crate::cards::CardDefinitionBuilder;
        use crate::color::ColorSet;
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
        use crate::effects::CreateTokenEffect;
        use crate::ids::CardId;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Geist", alice);

        // Set up combat with source attacking Bob
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Token definition - just describes what the token IS
        let angel_token = CardDefinitionBuilder::new(CardId::new(), "Angel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .color_indicator(ColorSet::from(crate::color::Color::White))
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();

        // Effect describes HOW the token enters (attacking)
        let effect = Effect::new(CreateTokenEffect::one(angel_token).attacking());

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Token should be created and added to combat
        if let EffectResult::Objects(ids) = result.result {
            assert_eq!(ids.len(), 1);
            let token_id = ids[0];

            // Verify the token was added to combat attackers
            let combat = game.combat.as_ref().expect("Combat should still be active");
            assert!(
                combat
                    .attackers
                    .iter()
                    .any(|info| info.creature == token_id),
                "Token should be in combat attackers"
            );
            // Token should be attacking the same target as source (Bob)
            let token_attacker = combat
                .attackers
                .iter()
                .find(|info| info.creature == token_id)
                .expect("Token should be attacking");
            assert_eq!(
                token_attacker.target,
                AttackTarget::Player(bob),
                "Token should attack the same player as source"
            );
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_execute_counter_spell() {
        use crate::game_state::StackEntry;
        use crate::mana::ManaCost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a spell card in Bob's hand and put it on the stack
        let spell_card = CardBuilder::new(CardId::from_raw(99), "Lightning Bolt")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Red,
            ]]))
            .build();
        let spell_id = game.create_object_from_card(&spell_card, bob, Zone::Stack);

        // Put the spell on the stack
        let stack_entry = StackEntry::new(spell_id, bob);
        game.push_to_stack(stack_entry);

        // Verify the spell is on the stack
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].object_id, spell_id);

        // Create Alice's counterspell source
        let counterspell_source = create_creature(&mut game, "Counterspell Source", alice);
        let mut ctx = ExecutionContext::new_default(counterspell_source, alice)
            .with_targets(vec![ResolvedTarget::Object(spell_id)]);

        // Execute the counter effect
        let effect = Effect::counter(ChooseSpec::AnyTarget);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Should resolve successfully
        assert_eq!(result.result, EffectResult::Resolved);

        // Stack should be empty
        assert!(game.stack.is_empty());

        // The spell should have been moved to the graveyard
        // (with a new ID due to zone change)
        let bob_graveyard = &game.player(bob).unwrap().graveyard;
        assert_eq!(bob_graveyard.len(), 1);

        // The graveyard object should be the countered spell
        let gy_obj = game.object(bob_graveyard[0]).unwrap();
        assert_eq!(gy_obj.name, "Lightning Bolt");
        assert_eq!(gy_obj.zone, Zone::Graveyard);
    }

    #[test]
    fn test_execute_counter_spell_target_not_on_stack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a creature (not a spell on the stack)
        let creature_id = create_creature(&mut game, "Target", bob);

        let counterspell_source = create_creature(&mut game, "Source", alice);
        let mut ctx = ExecutionContext::new_default(counterspell_source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Try to counter something not on the stack
        let effect = Effect::counter(ChooseSpec::AnyTarget);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Should return TargetInvalid since the target isn't on the stack
        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_was_kicked_value() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Test without kicker paid
        let ctx = ExecutionContext::new_default(source, alice);
        let result = resolve_value(&game, &Value::WasKicked, &ctx).unwrap();
        assert_eq!(result, 0, "WasKicked should be 0 when not kicked");

        // Test with kicker paid (using label)
        let mut paid = OptionalCostsPaid {
            costs: vec![("Kicker", 0)],
        };
        paid.pay(0);
        let ctx_kicked =
            ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);
        let result = resolve_value(&game, &Value::WasKicked, &ctx_kicked).unwrap();
        assert_eq!(result, 1, "WasKicked should be 1 when kicked");
    }

    #[test]
    fn test_was_entwined_value() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Test without entwine paid
        let ctx = ExecutionContext::new_default(source, alice);
        let result = resolve_value(&game, &Value::WasEntwined, &ctx).unwrap();
        assert_eq!(result, 0, "WasEntwined should be 0 when not entwined");

        // Test with entwine paid
        let mut paid = OptionalCostsPaid {
            costs: vec![("Entwine", 0)],
        };
        paid.pay(0);
        let ctx_entwined =
            ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);
        let result = resolve_value(&game, &Value::WasEntwined, &ctx_entwined).unwrap();
        assert_eq!(result, 1, "WasEntwined should be 1 when entwined");
    }

    #[test]
    fn test_was_bought_back_value() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Test with buyback paid
        let mut paid = OptionalCostsPaid {
            costs: vec![("Buyback", 0)],
        };
        paid.pay(0);
        let ctx = ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);
        let result = resolve_value(&game, &Value::WasBoughtBack, &ctx).unwrap();
        assert_eq!(result, 1, "WasBoughtBack should be 1 when buyback paid");
    }

    #[test]
    fn test_was_paid_label_value() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Test with multiple optional costs by label
        let mut paid = OptionalCostsPaid {
            costs: vec![("Kicker", 0), ("Buyback", 0), ("Entwine", 0)],
        };
        paid.pay_label("Kicker");
        paid.pay_label("Entwine");

        let ctx = ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);

        assert_eq!(
            resolve_value(&game, &Value::WasPaidLabel("Kicker"), &ctx).unwrap(),
            1,
            "WasPaidLabel(Kicker) should be 1"
        );
        assert_eq!(
            resolve_value(&game, &Value::WasPaidLabel("Buyback"), &ctx).unwrap(),
            0,
            "WasPaidLabel(Buyback) should be 0"
        );
        assert_eq!(
            resolve_value(&game, &Value::WasPaidLabel("Entwine"), &ctx).unwrap(),
            1,
            "WasPaid(2) should be 1"
        );
    }

    #[test]
    fn test_times_paid_value() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Test multikicker: paid 3 times
        let multikicker = OptionalCost::multikicker(TotalCost::mana(ManaCost::from_pips(vec![
            vec![ManaSymbol::Green],
        ])));
        let mut paid = OptionalCostsPaid::from_costs(&[multikicker]);
        paid.pay_times(0, 3);

        let ctx = ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);

        assert_eq!(
            resolve_value(&game, &Value::TimesPaid(0), &ctx).unwrap(),
            3,
            "TimesPaid(0) should be 3"
        );
        assert_eq!(
            resolve_value(&game, &Value::TimesPaidLabel("Multikicker"), &ctx).unwrap(),
            3,
            "TimesPaidLabel(Multikicker) should be 3"
        );
        assert_eq!(
            resolve_value(&game, &Value::KickCount, &ctx).unwrap(),
            3,
            "KickCount should be 3"
        );
    }
}
