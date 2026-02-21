//! Shared helper functions for effect execution.
//!
//! This module contains utility functions used by multiple effect implementations:
//! - Value resolution (X, counts, power/toughness, etc.)
//! - Player filter resolution
//! - Target finding and validation

use crate::cost::OptionalCostsPaid;
use crate::effect::{EffectOutcome, EffectResult, EventValueSpec, Value};
use crate::events::DamageEvent;
use crate::events::combat::{CreatureAttackedEvent, CreatureBecameBlockedEvent};
use crate::events::life::LifeGainEvent;
use crate::events::life::LifeLossEvent;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_event::DamageTarget;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::{ChooseSpec, FilterContext, ObjectRef, PlayerFilter};
use crate::triggers::AttackEventTarget;
use crate::types::{CardType, Subtype};
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
        Value::CountScaled(filter, multiplier) => {
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
                .count() as i32;
            Ok(count * *multiplier)
        }
        Value::BasicLandTypesAmong(filter) => {
            use std::collections::HashSet;

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

            let mut seen = HashSet::new();
            for obj in candidate_ids
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
            {
                for subtype in &obj.subtypes {
                    if matches!(
                        subtype,
                        Subtype::Plains
                            | Subtype::Island
                            | Subtype::Swamp
                            | Subtype::Mountain
                            | Subtype::Forest
                    ) {
                        seen.insert(subtype.clone());
                    }
                }
            }
            Ok(seen.len() as i32)
        }
        Value::ColorsAmong(filter) => {
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

            let mut has_white = false;
            let mut has_blue = false;
            let mut has_black = false;
            let mut has_red = false;
            let mut has_green = false;

            for obj in candidate_ids
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

        Value::PowerOf(target_spec) => {
            let target_id =
                resolve_primary_object_from_value_spec(game, target_spec.as_ref(), ctx)?;
            // Try to get current object, fall back to LKI snapshot
            if let Some(obj) = game.object(target_id) {
                game.calculated_power(target_id)
                    .or_else(|| obj.power())
                    .ok_or_else(|| {
                        ExecutionError::UnresolvableValue("Target has no power".to_string())
                    })
            } else if let ChooseSpec::Tagged(tag) = target_spec.as_ref()
                && let Some(snapshot) = ctx.get_tagged(tag)
            {
                snapshot.power.ok_or_else(|| {
                    ExecutionError::UnresolvableValue("Target had no power".to_string())
                })
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                snapshot.power.ok_or_else(|| {
                    ExecutionError::UnresolvableValue("Target had no power".to_string())
                })
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }

        Value::ToughnessOf(target_spec) => {
            let target_id =
                resolve_primary_object_from_value_spec(game, target_spec.as_ref(), ctx)?;
            // Try to get current object, fall back to LKI snapshot
            if let Some(obj) = game.object(target_id) {
                game.calculated_toughness(target_id)
                    .or_else(|| obj.toughness())
                    .ok_or_else(|| {
                        ExecutionError::UnresolvableValue("Target has no toughness".to_string())
                    })
            } else if let ChooseSpec::Tagged(tag) = target_spec.as_ref()
                && let Some(snapshot) = ctx.get_tagged(tag)
            {
                snapshot.toughness.ok_or_else(|| {
                    ExecutionError::UnresolvableValue("Target had no toughness".to_string())
                })
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                snapshot.toughness.ok_or_else(|| {
                    ExecutionError::UnresolvableValue("Target had no toughness".to_string())
                })
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }

        Value::ManaValueOf(target_spec) => {
            let target_id =
                resolve_primary_object_from_value_spec(game, target_spec.as_ref(), ctx)?;
            if let Some(obj) = game.object(target_id) {
                obj.mana_cost
                    .as_ref()
                    .map(|cost| cost.mana_value() as i32)
                    .ok_or_else(|| {
                        ExecutionError::UnresolvableValue("Target has no mana value".to_string())
                    })
            } else if let ChooseSpec::Tagged(tag) = target_spec.as_ref()
                && let Some(snapshot) = ctx.get_tagged(tag)
            {
                snapshot
                    .mana_cost
                    .as_ref()
                    .map(|cost| cost.mana_value() as i32)
                    .ok_or_else(|| {
                        ExecutionError::UnresolvableValue("Target had no mana value".to_string())
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
                return Err(ExecutionError::UnresolvableValue(
                    "TaggedCount used outside ForEachControllerOfTagged loop".to_string(),
                ));
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
fn attacked_target_from_trigger(ctx: &ExecutionContext) -> Option<AttackEventTarget> {
    let triggering_event = ctx.triggering_event.as_ref()?;
    if let Some(event) = triggering_event.downcast::<CreatureAttackedEvent>() {
        return Some(event.target);
    }
    if let Some(event) = triggering_event.downcast::<CreatureBecameBlockedEvent>() {
        return event.attack_target;
    }
    None
}

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
        ChooseSpec::PlayerOrPlaneswalker(filter) => {
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    let filter_ctx = ctx.filter_context(game);
                    if filter.matches_player(*id, &filter_ctx) {
                        return Ok(*id);
                    }
                }
            }
            resolve_player_filter(game, filter, ctx)
        }
        ChooseSpec::AttackedPlayerOrPlaneswalker => match attacked_target_from_trigger(ctx) {
            Some(AttackEventTarget::Player(player_id)) => Ok(player_id),
            Some(AttackEventTarget::Planeswalker(planeswalker_id)) => {
                let planeswalker = game
                    .object(planeswalker_id)
                    .ok_or(ExecutionError::ObjectNotFound(planeswalker_id))?;
                Ok(planeswalker.controller)
            }
            None => ctx.defending_player.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "Attacked player/planeswalker not set".to_string(),
                )
            }),
        },

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

/// Normalize object selections returned by a decision maker.
///
/// This guarantees:
/// - at most `required` objects are returned,
/// - every object is from `candidates`,
/// - there are no duplicates,
/// - if fewer than `required` valid selections were provided, the remainder is
///   filled deterministically from `candidates` order.
pub fn normalize_object_selection(
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    required: usize,
) -> Vec<ObjectId> {
    let mut selected = Vec::with_capacity(required);

    for id in chosen {
        if selected.len() == required {
            break;
        }
        if candidates.contains(&id) && !selected.contains(&id) {
            selected.push(id);
        }
    }

    if selected.len() < required {
        for &id in candidates {
            if selected.len() == required {
                break;
            }
            if !selected.contains(&id) {
                selected.push(id);
            }
        }
    }

    selected
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
        (ResolvedTarget::Player(id), ChooseSpec::PlayerOrPlaneswalker(filter)) => {
            filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::PlayerOrPlaneswalker(_)) => game
            .object(*id)
            .is_some_and(|obj| obj.has_card_type(CardType::Planeswalker)),
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

fn resolve_primary_object_from_value_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<ObjectId, ExecutionError> {
    let objects = resolve_objects_from_spec(game, spec, ctx)?;
    objects
        .first()
        .copied()
        .ok_or(ExecutionError::InvalidTarget)
}

/// Result shaping policy for applying operations to selected objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectApplyResultPolicy {
    /// Return `Count(applied_count)`.
    CountApplied,
    /// Return `Resolved` when at least one object was selected, else `TargetInvalid`.
    ///
    /// This preserves single-target semantics used by effects that resolve even when
    /// a selected object is no longer present or no state change occurred.
    SingleTargetResolvedOrInvalid,
}

/// Summary from applying an operation across selected objects.
#[derive(Debug)]
pub struct ObjectApplyResult {
    pub selected_count: usize,
    pub applied_count: usize,
    pub outcome: EffectOutcome,
}

/// Resolve objects from `spec`, apply an operation per object, and shape the result.
pub fn apply_to_selected_objects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
    result_policy: ObjectApplyResultPolicy,
    mut apply: impl FnMut(
        &mut GameState,
        &mut ExecutionContext,
        ObjectId,
    ) -> Result<bool, ExecutionError>,
) -> Result<ObjectApplyResult, ExecutionError> {
    let objects = resolve_objects_from_spec(game, spec, ctx)?;
    let selected_count = objects.len();
    let mut applied_count = 0usize;

    for object_id in objects {
        if apply(game, ctx, object_id)? {
            applied_count += 1;
        }
    }

    let outcome = match result_policy {
        ObjectApplyResultPolicy::CountApplied => EffectOutcome::count(applied_count as i32),
        ObjectApplyResultPolicy::SingleTargetResolvedOrInvalid => {
            if selected_count > 0 {
                EffectOutcome::resolved()
            } else {
                EffectOutcome::from_result(EffectResult::TargetInvalid)
            }
        }
    };

    Ok(ObjectApplyResult {
        selected_count,
        applied_count,
        outcome,
    })
}

/// Apply a single-target object operation using `ctx.targets` semantics.
///
/// This preserves the common single-target behavior:
/// - first object target is used,
/// - `None` means success (`Resolved`),
/// - `Some(result)` means short-circuit with that result,
/// - no object targets means `TargetInvalid`.
pub fn apply_single_target_object_from_context(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut apply: impl FnMut(
        &mut GameState,
        &mut ExecutionContext,
        ObjectId,
    ) -> Result<Option<EffectResult>, ExecutionError>,
) -> Result<EffectOutcome, ExecutionError> {
    for target in ctx.targets.clone() {
        if let ResolvedTarget::Object(object_id) = target {
            if let Some(result) = apply(game, ctx, object_id)? {
                return Ok(EffectOutcome::from_result(result));
            }
            return Ok(EffectOutcome::resolved());
        }
    }

    Ok(EffectOutcome::from_result(EffectResult::TargetInvalid))
}

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
        // Target wrapper - handle special cases then fall back to ctx.targets
        ChooseSpec::Target(inner) => {
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
        ChooseSpec::WithCount(inner, count) => {
            if inner.is_target() {
                return resolve_objects_from_spec(game, inner, ctx);
            }

            if let ChooseSpec::Object(filter) = inner.base() {
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
                    None => game
                        .battlefield
                        .iter()
                        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                        .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                        .map(|(id, _)| id)
                        .collect(),
                };

                let mut objects: Vec<ObjectId> = candidate_ids
                    .iter()
                    .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                    .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                    .map(|(id, _)| id)
                    .collect();

                if objects.is_empty() {
                    return Err(ExecutionError::InvalidTarget);
                }

                let max = if count.is_dynamic_x() {
                    ctx.x_value
                        .map(|x| x as usize)
                        .or(count.max)
                        .unwrap_or(objects.len())
                } else {
                    count.max.unwrap_or(objects.len())
                };
                objects.truncate(max);
                if objects.len() < count.min {
                    return Err(ExecutionError::InvalidTarget);
                }
                return Ok(objects);
            }

            resolve_objects_from_spec(game, inner, ctx)
        }

        // Object filter (non-targeted choice) - generally supplied via previous selection,
        // but some tags only effects resolve from tagged objects and filters.
        ChooseSpec::Object(filter) => {
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
                if filter.tagged_constraints.is_empty() {
                    return Err(ExecutionError::InvalidTarget);
                }

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
                    None => game
                        .battlefield
                        .iter()
                        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                        .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                        .map(|(id, _)| id)
                        .collect(),
                };

                let objects: Vec<ObjectId> = candidate_ids
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| filter.matches(obj, &filter_ctx, game))
                    .map(|obj| obj.id)
                    .collect();

                if objects.is_empty() {
                    return Err(ExecutionError::InvalidTarget);
                }

                return Ok(objects);
            }

            Ok(objects)
        }

        ChooseSpec::AnyTarget | ChooseSpec::PlayerOrPlaneswalker(_) => {
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
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            if let Some(AttackEventTarget::Planeswalker(planeswalker_id)) =
                attacked_target_from_trigger(ctx)
            {
                return Ok(vec![planeswalker_id]);
            }
            Err(ExecutionError::InvalidTarget)
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
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
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
        ChooseSpec::AttackedPlayerOrPlaneswalker => match attacked_target_from_trigger(ctx) {
            Some(AttackEventTarget::Player(player_id)) => Ok(vec![player_id]),
            Some(AttackEventTarget::Planeswalker(planeswalker_id)) => {
                let planeswalker = game
                    .object(planeswalker_id)
                    .ok_or(ExecutionError::ObjectNotFound(planeswalker_id))?;
                Ok(vec![planeswalker.controller])
            }
            None => {
                if let Some(defending) = ctx.defending_player {
                    Ok(vec![defending])
                } else {
                    Err(ExecutionError::UnresolvableValue(
                        "Attacked player/planeswalker not set".to_string(),
                    ))
                }
            }
        },

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
        PlayerFilter::NotYou => {
            let others: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != ctx.controller && p.is_in_game())
                .map(|p| p.id)
                .collect();
            Ok(others)
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

    #[test]
    fn test_normalize_object_selection_filters_invalid_and_dedups() {
        let candidates = vec![ObjectId(1), ObjectId(2), ObjectId(3)];
        let chosen = vec![ObjectId(2), ObjectId(99), ObjectId(2), ObjectId(3)];

        let selected = normalize_object_selection(chosen, &candidates, 2);
        assert_eq!(selected, vec![ObjectId(2), ObjectId(3)]);
    }

    #[test]
    fn test_normalize_object_selection_fills_missing_required() {
        let candidates = vec![ObjectId(10), ObjectId(11), ObjectId(12)];
        let chosen = vec![ObjectId(11)];

        let selected = normalize_object_selection(chosen, &candidates, 3);
        assert_eq!(selected, vec![ObjectId(11), ObjectId(10), ObjectId(12)]);
    }

    #[test]
    fn test_apply_to_selected_objects_count_policy() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let target_1 = game.new_object_id();
        let target_2 = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, player_id).with_targets(vec![
            ResolvedTarget::Object(target_1),
            ResolvedTarget::Object(target_2),
        ]);

        let spec = ChooseSpec::target(ChooseSpec::creature())
            .with_count(crate::effect::ChoiceCount::any_number());
        let mut seen = Vec::new();
        let result = apply_to_selected_objects(
            &mut game,
            &mut ctx,
            &spec,
            ObjectApplyResultPolicy::CountApplied,
            |_game, _ctx, object_id| {
                seen.push(object_id);
                Ok(object_id == target_1)
            },
        )
        .unwrap();

        assert_eq!(result.selected_count, 2);
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.outcome.result, EffectResult::Count(1));
        assert_eq!(seen, vec![target_1, target_2]);
    }

    #[test]
    fn test_apply_to_selected_objects_single_target_policy_resolves_when_selected() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let target_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Object(target_id)]);

        let spec = ChooseSpec::target(ChooseSpec::creature());
        let result = apply_to_selected_objects(
            &mut game,
            &mut ctx,
            &spec,
            ObjectApplyResultPolicy::SingleTargetResolvedOrInvalid,
            |_game, _ctx, _object_id| Ok(false),
        )
        .unwrap();

        assert_eq!(result.selected_count, 1);
        assert_eq!(result.applied_count, 0);
        assert_eq!(result.outcome.result, EffectResult::Resolved);
    }

    #[test]
    fn test_apply_single_target_object_from_context_resolves_on_none() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let target_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Object(target_id)]);

        let outcome = apply_single_target_object_from_context(
            &mut game,
            &mut ctx,
            |_game, _ctx, object_id| {
                assert_eq!(object_id, target_id);
                Ok(None)
            },
        )
        .unwrap();

        assert_eq!(outcome.result, EffectResult::Resolved);
    }

    #[test]
    fn test_apply_single_target_object_from_context_propagates_custom_result() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let target_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Object(target_id)]);

        let outcome = apply_single_target_object_from_context(
            &mut game,
            &mut ctx,
            |_game, _ctx, _object_id| Ok(Some(EffectResult::Prevented)),
        )
        .unwrap();

        assert_eq!(outcome.result, EffectResult::Prevented);
    }

    #[test]
    fn test_apply_single_target_object_from_context_target_invalid_without_object_target() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Player(PlayerId(1))]);

        let outcome = apply_single_target_object_from_context(
            &mut game,
            &mut ctx,
            |_game, _ctx, _object_id| Ok(None),
        )
        .unwrap();

        assert_eq!(outcome.result, EffectResult::TargetInvalid);
    }
}
