//! Shared helper functions for effect execution.
//!
//! This module contains utility functions used by multiple effect implementations:
//! - Value resolution (X, counts, power/toughness, etc.)
//! - Player filter resolution
//! - Target finding and validation

use crate::cost::OptionalCostsPaid;
use crate::effect::Value;
use crate::events::DamageEvent;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_event::DamageTarget;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::{ChooseSpec, FilterContext, ObjectRef, PlayerFilter};
use crate::zone::Zone;

// ============================================================================
// Value Resolution
// ============================================================================

/// Get the optional costs paid, preferring context but falling back to source object.
/// This allows ETB triggers to access kick count etc. from the permanent that entered.
pub fn get_optional_costs_paid<'a>(
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
            let candidate_ids: Vec<ObjectId> = match filter.zone {
                Some(Zone::Battlefield) => game.battlefield.clone(),
                Some(Zone::Graveyard) => game
                    .players
                    .iter()
                    .flat_map(|player| player.graveyard.iter().copied())
                    .collect(),
                Some(Zone::Hand) => game
                    .players
                    .iter()
                    .flat_map(|player| player.hand.iter().copied())
                    .collect(),
                Some(Zone::Library) => game
                    .players
                    .iter()
                    .flat_map(|player| player.library.iter().copied())
                    .collect(),
                Some(Zone::Stack) => game.stack.iter().map(|entry| entry.object_id).collect(),
                Some(Zone::Exile) => game.exile.clone(),
                Some(Zone::Command) => game.command_zone.clone(),
                None => game.battlefield.clone(),
            };

            let count = candidate_ids
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            Ok(count as i32)
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

        Value::SourcePower => {
            let obj = game
                .object(ctx.source)
                .ok_or(ExecutionError::ObjectNotFound(ctx.source))?;
            obj.power()
                .ok_or_else(|| ExecutionError::UnresolvableValue("Source has no power".to_string()))
        }

        Value::SourceToughness => {
            let obj = game
                .object(ctx.source)
                .ok_or(ExecutionError::ObjectNotFound(ctx.source))?;
            obj.toughness().ok_or_else(|| {
                ExecutionError::UnresolvableValue("Source has no toughness".to_string())
            })
        }

        Value::PowerOf(_target_spec) => {
            let target_id = find_target_object(&ctx.targets)?;
            // Try to get current object, fall back to LKI snapshot
            if let Some(obj) = game.object(target_id) {
                obj.power().ok_or_else(|| {
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
                obj.toughness().ok_or_else(|| {
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

        Value::EffectValue(effect_id) => {
            let result = ctx
                .get_result(*effect_id)
                .ok_or(ExecutionError::EffectNotFound(*effect_id))?;
            Ok(result.count_or_zero())
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

// ============================================================================
// Player Filter Resolution
// ============================================================================

/// Resolve a ChooseSpec to a PlayerId.
///
/// This is the primary way to resolve "which player" from a ChooseSpec.
/// Handles targeting, filters, and special references.
pub fn resolve_player_from_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<PlayerId, ExecutionError> {
    match spec {
        // Target wrapper - look in ctx.targets for a player target
        ChooseSpec::Target(inner) => {
            // First check ctx.targets for a player
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(*id);
                }
            }
            // If no player in targets, try to resolve the inner spec
            resolve_player_from_spec(game, inner, ctx)
        }

        // Player filter - delegate to resolve_player_filter
        ChooseSpec::Player(filter) => resolve_player_filter(game, filter, ctx),

        // Source controller ("you" on a permanent's ability)
        ChooseSpec::SourceController => Ok(ctx.controller),

        // Source owner
        ChooseSpec::SourceOwner => {
            if let Some(obj) = game.object(ctx.source) {
                Ok(obj.owner)
            } else {
                Err(ExecutionError::ObjectNotFound(ctx.source))
            }
        }

        // Specific player
        ChooseSpec::SpecificPlayer(id) => Ok(*id),

        // Tagged - not typically used for players, but could be extended
        ChooseSpec::Tagged(_) => Err(ExecutionError::UnresolvableValue(
            "Tagged spec cannot be resolved to a player".to_string(),
        )),

        // EachPlayer - resolve all matching players (returns first for single resolution)
        ChooseSpec::EachPlayer(filter) => resolve_player_filter(game, filter, ctx),

        // WithCount wrapper - delegate to inner spec
        ChooseSpec::WithCount(inner, _) => resolve_player_from_spec(game, inner, ctx),

        // Iterated player (in ForEach loops)
        ChooseSpec::Iterated => ctx.iterated_player.ok_or_else(|| {
            ExecutionError::UnresolvableValue(
                "Iterated player not set (must be inside ForEach loop)".to_string(),
            )
        }),

        // Object specs can't be resolved to players
        ChooseSpec::Object(_)
        | ChooseSpec::SpecificObject(_)
        | ChooseSpec::Source
        | ChooseSpec::AnyTarget
        | ChooseSpec::All(_) => Err(ExecutionError::UnresolvableValue(
            "Object spec cannot be resolved to a player".to_string(),
        )),
    }
}

/// Resolve a PlayerFilter to a concrete PlayerId.
pub fn resolve_player_filter(
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
                    "DamagedPlayer not set".to_string(),
                ));
            };
            let Some(damage_event) = triggering_event.downcast::<DamageEvent>() else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires a damage event".to_string(),
                ));
            };
            let DamageTarget::Player(player_id) = damage_event.target else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires a player damage target".to_string(),
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
        PlayerFilter::ControllerOf(object_ref) => resolve_controller_of(game, ctx, object_ref),
        PlayerFilter::OwnerOf(object_ref) => resolve_owner_of(game, ctx, object_ref),
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

fn resolve_controller_of(
    game: &GameState,
    ctx: &ExecutionContext,
    object_ref: &ObjectRef,
) -> Result<PlayerId, ExecutionError> {
    match object_ref {
        ObjectRef::Target => {
            let target_id = find_target_object(&ctx.targets)?;
            if let Some(obj) = game.object(target_id) {
                Ok(obj.controller)
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                Ok(snapshot.controller)
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }
        ObjectRef::Specific(object_id) => {
            if let Some(obj) = game.object(*object_id) {
                Ok(obj.controller)
            } else if let Some(snapshot) = ctx.target_snapshots.get(object_id) {
                Ok(snapshot.controller)
            } else {
                Err(ExecutionError::ObjectNotFound(*object_id))
            }
        }
        ObjectRef::Tagged(tag) => {
            if let Some(snapshot) = ctx.get_tagged(tag) {
                Ok(snapshot.controller)
            } else {
                Err(ExecutionError::TagNotFound(tag.to_string()))
            }
        }
    }
}

fn resolve_owner_of(
    game: &GameState,
    ctx: &ExecutionContext,
    object_ref: &ObjectRef,
) -> Result<PlayerId, ExecutionError> {
    match object_ref {
        ObjectRef::Target => {
            let target_id = find_target_object(&ctx.targets)?;
            if let Some(obj) = game.object(target_id) {
                Ok(obj.owner)
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                Ok(snapshot.owner)
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }
        ObjectRef::Specific(object_id) => {
            if let Some(obj) = game.object(*object_id) {
                Ok(obj.owner)
            } else if let Some(snapshot) = ctx.target_snapshots.get(object_id) {
                Ok(snapshot.owner)
            } else {
                Err(ExecutionError::ObjectNotFound(*object_id))
            }
        }
        ObjectRef::Tagged(tag) => {
            if let Some(snapshot) = ctx.get_tagged(tag) {
                Ok(snapshot.owner)
            } else {
                Err(ExecutionError::TagNotFound(tag.to_string()))
            }
        }
    }
}

// ============================================================================
// Target Finding
// ============================================================================

/// Find the first object target in the targets list.
pub fn find_target_object(targets: &[ResolvedTarget]) -> Result<ObjectId, ExecutionError> {
    for target in targets {
        if let ResolvedTarget::Object(id) = target {
            return Ok(*id);
        }
    }
    Err(ExecutionError::InvalidTarget)
}

/// Find the first player target in the targets list.
pub fn find_target_player(targets: &[ResolvedTarget]) -> Result<PlayerId, ExecutionError> {
    for target in targets {
        if let ResolvedTarget::Player(id) = target {
            return Ok(*id);
        }
    }
    Err(ExecutionError::InvalidTarget)
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
        // Target wrapper - unwrap and validate the inner spec
        (_, ChooseSpec::Target(inner)) => validate_target(game, target, inner, ctx),
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
// Selection Resolution
// ============================================================================

/// Resolve a ChooseSpec to a list of ObjectIds.
///
/// For targeted/chosen specs, returns the objects from ctx.targets.
/// For All specs, filters objects on the battlefield.
/// For Source, returns the source object.
/// For Iterated, returns the current iterated object.
pub fn resolve_objects_from_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<Vec<ObjectId>, ExecutionError> {
    match spec {
        // Target/WithCount wrappers - handle special cases then fall back to ctx.targets
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            // Handle special cases where target is embedded in the spec
            match inner.base() {
                ChooseSpec::SpecificObject(id) => {
                    return Ok(vec![*id]);
                }
                ChooseSpec::Source => {
                    return Ok(vec![ctx.source]);
                }
                ChooseSpec::Tagged(tag) => {
                    let tagged = ctx
                        .get_tagged_all(tag)
                        .ok_or_else(|| ExecutionError::TagNotFound(tag.to_string()))?;
                    let objects: Vec<ObjectId> = tagged.iter().map(|s| s.object_id).collect();
                    if objects.is_empty() {
                        return Err(ExecutionError::InvalidTarget);
                    }
                    return Ok(objects);
                }
                _ => {}
            }

            // Extract object targets from the resolved targets
            let objects: Vec<ObjectId> = ctx
                .targets
                .iter()
                .filter_map(|t| {
                    if let ResolvedTarget::Object(id) = t {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();

            if objects.is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }

            Ok(objects)
        }

        // Object filter (non-targeted choice) - also uses ctx.targets
        ChooseSpec::Object(_) => {
            let objects: Vec<ObjectId> = ctx
                .targets
                .iter()
                .filter_map(|t| {
                    if let ResolvedTarget::Object(id) = t {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();

            if objects.is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }

            Ok(objects)
        }

        ChooseSpec::AnyTarget => {
            let objects: Vec<ObjectId> = ctx
                .targets
                .iter()
                .filter_map(|t| {
                    if let ResolvedTarget::Object(id) = t {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();

            if objects.is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }

            Ok(objects)
        }

        // All matching - filter battlefield
        ChooseSpec::All(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let candidate_ids: Vec<ObjectId> = match filter.zone {
                Some(Zone::Battlefield) => game.battlefield.clone(),
                Some(Zone::Graveyard) => game
                    .players
                    .iter()
                    .flat_map(|player| player.graveyard.iter().copied())
                    .collect(),
                Some(Zone::Hand) => game
                    .players
                    .iter()
                    .flat_map(|player| player.hand.iter().copied())
                    .collect(),
                Some(Zone::Library) => game
                    .players
                    .iter()
                    .flat_map(|player| player.library.iter().copied())
                    .collect(),
                Some(Zone::Stack) => game.stack.iter().map(|entry| entry.object_id).collect(),
                Some(Zone::Exile) => game.exile.clone(),
                Some(Zone::Command) => game.command_zone.clone(),
                None => game.battlefield.clone(),
            };

            let objects: Vec<ObjectId> = candidate_ids
                .iter()
                .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                .map(|(id, _)| id)
                .collect();

            Ok(objects)
        }

        // Source reference
        ChooseSpec::Source => Ok(vec![ctx.source]),

        // Specific object
        ChooseSpec::SpecificObject(id) => Ok(vec![*id]),

        // Tagged objects
        ChooseSpec::Tagged(tag) => {
            let Some(tagged) = ctx.get_tagged_all(tag) else {
                return Ok(Vec::new());
            };
            Ok(tagged.iter().map(|s| s.object_id).collect())
        }

        // Iterated object (ForEach loops)
        ChooseSpec::Iterated => ctx.iterated_object.map(|id| vec![id]).ok_or_else(|| {
            ExecutionError::UnresolvableValue(
                "Iterated object not set (must be inside ForEach loop)".to_string(),
            )
        }),

        // Player specs can't be resolved to objects
        ChooseSpec::Player(_)
        | ChooseSpec::SpecificPlayer(_)
        | ChooseSpec::SourceController
        | ChooseSpec::SourceOwner
        | ChooseSpec::EachPlayer(_) => Err(ExecutionError::UnresolvableValue(
            "Player spec cannot be resolved to objects".to_string(),
        )),
    }
}

/// Resolve a ChooseSpec to a list of PlayerIds.
///
/// For targeted/chosen player specs, returns the players from ctx.targets.
/// For EachPlayer specs, filters players in the game.
/// For SourceController, returns the controller.
/// For Iterated, returns the current iterated player.
pub fn resolve_players_from_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<Vec<PlayerId>, ExecutionError> {
    match spec {
        // Target/WithCount wrappers - delegate to inner
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            // First check ctx.targets for players
            let players: Vec<PlayerId> = ctx
                .targets
                .iter()
                .filter_map(|t| {
                    if let ResolvedTarget::Player(id) = t {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();

            if !players.is_empty() {
                return Ok(players);
            }

            // If no player targets, try to resolve the inner spec
            resolve_players_from_spec(game, inner, ctx)
        }

        // Player filter - resolve to matching players
        ChooseSpec::Player(filter) => {
            // First check targets
            let players: Vec<PlayerId> = ctx
                .targets
                .iter()
                .filter_map(|t| {
                    if let ResolvedTarget::Player(id) = t {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();

            if !players.is_empty() {
                return Ok(players);
            }

            // Fall back to filter resolution
            let filter_ctx = ctx.filter_context(game);
            resolve_player_filter_to_list(game, filter, &filter_ctx, ctx)
        }

        // Each player matching filter
        ChooseSpec::EachPlayer(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let players: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.is_in_game())
                .filter(|p| filter.matches_player(p.id, &filter_ctx))
                .map(|p| p.id)
                .collect();

            Ok(players)
        }

        // Source controller ("you")
        ChooseSpec::SourceController => Ok(vec![ctx.controller]),

        // Source owner
        ChooseSpec::SourceOwner => {
            if let Some(obj) = game.object(ctx.source) {
                Ok(vec![obj.owner])
            } else {
                Err(ExecutionError::ObjectNotFound(ctx.source))
            }
        }

        // Specific player
        ChooseSpec::SpecificPlayer(id) => Ok(vec![*id]),

        // Iterated player (ForEach loops)
        ChooseSpec::Iterated => ctx.iterated_player.map(|id| vec![id]).ok_or_else(|| {
            ExecutionError::UnresolvableValue(
                "Iterated player not set (must be inside ForEach loop)".to_string(),
            )
        }),

        // Object specs can't be resolved to players
        ChooseSpec::Object(_)
        | ChooseSpec::SpecificObject(_)
        | ChooseSpec::Source
        | ChooseSpec::Tagged(_)
        | ChooseSpec::All(_)
        | ChooseSpec::AnyTarget => Err(ExecutionError::UnresolvableValue(
            "Object spec cannot be resolved to players".to_string(),
        )),
    }
}

/// Helper to resolve a PlayerFilter to a list of PlayerIds.
fn resolve_player_filter_to_list(
    game: &GameState,
    filter: &PlayerFilter,
    _filter_ctx: &FilterContext,
    ctx: &ExecutionContext,
) -> Result<Vec<PlayerId>, ExecutionError> {
    match filter {
        PlayerFilter::You => Ok(vec![ctx.controller]),
        PlayerFilter::Any | PlayerFilter::Target(_) => {
            // For Any/Target, check targets first
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(vec![*id]);
                }
            }
            Err(ExecutionError::InvalidTarget)
        }
        PlayerFilter::Opponent => {
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != ctx.controller && p.is_in_game())
                .map(|p| p.id)
                .collect();
            Ok(opponents)
        }
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
                    "DamagedPlayer not set".to_string(),
                ));
            };
            let Some(damage_event) = triggering_event.downcast::<DamageEvent>() else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires a damage event".to_string(),
                ));
            };
            let DamageTarget::Player(player_id) = damage_event.target else {
                return Err(ExecutionError::UnresolvableValue(
                    "DamagedPlayer requires a player damage target".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};

    fn new_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_resolve_fixed_value() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, player_id);

        let value = Value::Fixed(5);
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 5);
    }

    #[test]
    fn test_resolve_x_value() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, player_id).with_x(3);

        let value = Value::X;
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 3);
    }

    #[test]
    fn test_resolve_x_times_value() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, player_id).with_x(3);

        let value = Value::XTimes(2);
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 6);
    }

    #[test]
    fn test_resolve_player_filter_you() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, player_id);

        let filter = PlayerFilter::You;
        assert_eq!(
            resolve_player_filter(&game, &filter, &ctx).unwrap(),
            player_id
        );
    }

    #[test]
    fn test_find_target_object_found() {
        let object_id = ObjectId(42);
        let targets = vec![ResolvedTarget::Object(object_id)];
        assert_eq!(find_target_object(&targets).unwrap(), object_id);
    }

    #[test]
    fn test_find_target_object_not_found() {
        let targets = vec![ResolvedTarget::Player(PlayerId(1))];
        assert!(find_target_object(&targets).is_err());
    }

    #[test]
    fn test_find_target_player_found() {
        let player_id = PlayerId(1);
        let targets = vec![ResolvedTarget::Player(player_id)];
        assert_eq!(find_target_player(&targets).unwrap(), player_id);
    }
}
