use crate::effect::Condition;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Condition evaluation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionEvaluationMode {
    /// Cast-time evaluation: no full execution context is available yet.
    CastTime {
        controller: PlayerId,
        source: ObjectId,
    },
    /// Resolution-time evaluation: full execution context is available.
    Resolution,
}

/// Shared dispatcher for condition evaluation.
pub fn evaluate_condition_with_mode(
    game: &GameState,
    condition: &Condition,
    mode: ConditionEvaluationMode,
    ctx: Option<&ExecutionContext>,
) -> Result<bool, ExecutionError> {
    match mode {
        ConditionEvaluationMode::CastTime { controller, source } => Ok(evaluate_condition_simple(
            game, condition, controller, source,
        )),
        ConditionEvaluationMode::Resolution => {
            let ctx = ctx.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "resolution condition evaluation requires execution context".to_string(),
                )
            })?;
            evaluate_condition(game, condition, ctx)
        }
    }
}

/// Evaluate a condition for cast-time decisions.
pub fn evaluate_condition_cast_time(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    evaluate_condition_with_mode(
        game,
        condition,
        ConditionEvaluationMode::CastTime { controller, source },
        None,
    )
    .unwrap_or(false)
}

/// Evaluate a condition during effect resolution.
pub fn evaluate_condition_resolution(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    evaluate_condition_with_mode(
        game,
        condition,
        ConditionEvaluationMode::Resolution,
        Some(ctx),
    )
}

fn condition_candidate_ids_for_zone(game: &GameState, zone: Option<Zone>) -> Vec<ObjectId> {
    match zone {
        Some(Zone::Battlefield) | None => game.battlefield.clone(),
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
    }
}

fn condition_object_matches_player_zone(
    obj: &crate::object::Object,
    player_id: PlayerId,
    zone: Option<Zone>,
) -> bool {
    match zone {
        Some(Zone::Battlefield) | None => obj.controller == player_id,
        _ => obj.owner == player_id,
    }
}

fn count_distinct_matching_powers(
    game: &GameState,
    player_id: PlayerId,
    filter: &crate::target::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
) -> usize {
    use std::collections::HashSet;

    let mut seen_powers = HashSet::new();
    for obj in condition_candidate_ids_for_zone(game, filter.zone)
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
        .filter(|obj| filter.matches(obj, filter_ctx, game))
    {
        if let Some(power) = game.calculated_power(obj.id).or_else(|| obj.power()) {
            seen_powers.insert(power);
        }
    }
    seen_powers.len()
}

/// Evaluate a condition with minimal context (for cast-time evaluation).
///
/// This simplified version is used during spell casting to evaluate conditions
/// like `YouControlCommander` before targets are chosen. It handles common
/// conditions that don't require targets or other context-dependent information.
fn evaluate_condition_simple(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    // Build a simple filter context with opponents
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != controller)
        .map(|p| p.id)
        .collect();
    let filter_ctx = crate::filter::FilterContext::new(controller)
        .with_source(source)
        .with_opponents(opponents.clone());

    match condition {
        Condition::YouControl(filter) => game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == controller)
            .any(|obj| filter.matches(obj, &filter_ctx, game)),
        Condition::OpponentControls(filter) => game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| opponents.contains(&obj.controller))
            .any(|obj| filter.matches(obj, &filter_ctx, game)),
        Condition::PlayerControls { player, filter } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut ctx = crate::filter::FilterContext::new(player_id)
                .with_source(source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                ctx = ctx.with_iterated_player(Some(player_id));
            }
            condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .any(|obj| filter.matches(obj, &ctx, game))
        }
        Condition::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut ctx = crate::filter::FilterContext::new(player_id)
                .with_source(source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                ctx = ctx.with_iterated_player(Some(player_id));
            }

            if zones.is_empty() {
                return false;
            }

            let mut filter = crate::target::ObjectFilter::default().named(name.clone());
            for zone in zones {
                filter.zone = Some(*zone);
                let has_matching = condition_candidate_ids_for_zone(game, Some(*zone))
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| obj.owner == player_id)
                    .any(|obj| filter.matches(obj, &ctx, game));
                if !has_matching {
                    return false;
                }
            }
            true
        }
        Condition::PlayerControlsAtLeast {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut ctx = crate::filter::FilterContext::new(player_id)
                .with_source(source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                ctx = ctx.with_iterated_player(Some(player_id));
            }
            let matches = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .filter(|obj| filter.matches(obj, &ctx, game))
                .count();
            matches >= *count as usize
        }
        Condition::PlayerControlsExactly {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut ctx = crate::filter::FilterContext::new(player_id)
                .with_source(source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                ctx = ctx.with_iterated_player(Some(player_id));
            }
            let matches = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .filter(|obj| filter.matches(obj, &ctx, game))
                .count();
            matches == *count as usize
        }
        Condition::PlayerControlsAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut ctx = crate::filter::FilterContext::new(player_id)
                .with_source(source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                ctx = ctx.with_iterated_player(Some(player_id));
            }
            count_distinct_matching_powers(game, player_id, filter, &ctx) >= *count as usize
        }
        Condition::PlayerControlsMost { player, filter } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };

            let count_for = |candidate: PlayerId| {
                let opponents: Vec<PlayerId> = game
                    .players
                    .iter()
                    .filter(|p| p.id != candidate)
                    .map(|p| p.id)
                    .collect();
                let mut ctx = crate::filter::FilterContext::new(candidate)
                    .with_source(source)
                    .with_opponents(opponents);
                if *player == PlayerFilter::IteratedPlayer {
                    ctx = ctx.with_iterated_player(Some(candidate));
                }
                condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| condition_object_matches_player_zone(obj, candidate, filter.zone))
                    .filter(|obj| filter.matches(obj, &ctx, game))
                    .count()
            };

            let current = count_for(player_id);
            let max_count = game
                .players
                .iter()
                .map(|p| count_for(p.id))
                .max()
                .unwrap_or(0);
            current == max_count
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let Some(you_life) = game.player(controller).map(|p| p.life) else {
                return false;
            };
            let Some(other_life) = game.player(player_id).map(|p| p.life) else {
                return false;
            };
            other_life < you_life
        }
        Condition::LifeTotalOrLess(threshold) => {
            let life = game.player(controller).map(|p| p.life).unwrap_or(0);
            life <= *threshold
        }
        Condition::LifeTotalOrGreater(threshold) => {
            let life = game.player(controller).map(|p| p.life).unwrap_or(0);
            life >= *threshold
        }
        Condition::CardsInHandOrMore(threshold) => {
            let count = game.player(controller).map(|p| p.hand.len()).unwrap_or(0);
            count >= *threshold as usize
        }
        Condition::YourTurn => game.turn.active_player == controller,
        Condition::CreatureDiedThisTurn => game.creatures_died_this_turn > 0,
        Condition::CastSpellThisTurn => game.spells_cast_this_turn.values().any(|&count| count > 0),
        Condition::AttackedThisTurn => game.players_attacked_this_turn.contains(&controller),
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.players_tapped_land_for_mana_this_turn
                .contains(&player_id)
        }
        Condition::NoSpellsWereCastLastTurn => game.spells_cast_last_turn_total == 0,
        Condition::YouControlCommander => {
            // Check if the player controls a commander on the battlefield
            if let Some(player) = game.player(controller) {
                let commanders = player.get_commanders();
                for &commander_id in commanders {
                    // First check: is the commander ID directly on battlefield?
                    if game.battlefield.contains(&commander_id)
                        && let Some(obj) = game.object(commander_id)
                        && obj.controller == controller
                    {
                        return true;
                    }
                    // Second check: is there an object on battlefield whose stable_id
                    // matches the commander ID? (handles zone transitions)
                    for &bf_id in &game.battlefield {
                        if let Some(obj) = game.object(bf_id)
                            && obj.controller == controller
                            && obj.stable_id == StableId::from(commander_id)
                        {
                            return true;
                        }
                    }
                }
            }
            false
        }
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            let Some(source_obj) = game.object(source) else {
                return false;
            };
            let spent = if let Some(sym) = symbol {
                source_obj.mana_spent_to_cast.amount(*sym)
            } else {
                source_obj.mana_spent_to_cast.total()
            };
            spent >= *amount
        }
        Condition::SourceHasNoCounter(counter_type) => game
            .object(source)
            .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) == 0)
            .unwrap_or(false),
        Condition::TaggedObjectMatches(_, _) => false,
        Condition::PlayerTaggedObjectMatches { .. } => false,
        Condition::Not(inner) => !evaluate_condition_simple(game, inner, controller, source),
        Condition::And(a, b) => {
            evaluate_condition_simple(game, a, controller, source)
                && evaluate_condition_simple(game, b, controller, source)
        }
        Condition::Or(a, b) => {
            evaluate_condition_simple(game, a, controller, source)
                || evaluate_condition_simple(game, b, controller, source)
        }
        // Target-dependent conditions default to false during casting
        Condition::TargetIsTapped
        | Condition::TargetIsAttacking
        | Condition::TargetIsBlocked
        | Condition::TargetWasKicked
        | Condition::TargetSpellCastOrderThisTurn(_)
        | Condition::TargetSpellControllerIsPoisoned
        | Condition::TargetSpellManaSpentToCastAtLeast { .. }
        | Condition::YouControlMoreCreaturesThanTargetSpellController
        | Condition::TargetHasGreatestPowerAmongCreatures
        | Condition::TargetManaValueLteColorsSpentToCastThisSpell
        | Condition::SourceIsTapped => false,
    }
}

fn resolve_condition_player_simple(
    game: &GameState,
    controller: PlayerId,
    player: &PlayerFilter,
) -> Option<PlayerId> {
    match player {
        PlayerFilter::You => Some(controller),
        PlayerFilter::Specific(id) => Some(*id),
        PlayerFilter::Active => Some(game.turn.active_player),
        PlayerFilter::NotYou => game.players.iter().find_map(|p| {
            if p.id != controller && p.is_in_game() {
                Some(p.id)
            } else {
                None
            }
        }),
        PlayerFilter::Opponent => game.players.iter().find_map(|p| {
            if p.id != controller && p.is_in_game() {
                Some(p.id)
            } else {
                None
            }
        }),
        PlayerFilter::Any
        | PlayerFilter::Target(_)
        | PlayerFilter::Teammate
        | PlayerFilter::Attacking
        | PlayerFilter::Defending
        | PlayerFilter::DamagedPlayer
        | PlayerFilter::IteratedPlayer
        | PlayerFilter::ControllerOf(_)
        | PlayerFilter::OwnerOf(_) => None,
    }
}

/// Evaluate a condition.
fn evaluate_condition(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    match condition {
        Condition::YouControl(filter) => {
            let filter_ctx = ctx.filter_context(game);

            let has_matching = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == ctx.controller)
                .any(|obj| filter.matches(obj, &filter_ctx, game));

            Ok(has_matching)
        }
        Condition::OpponentControls(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let opponents = &filter_ctx.opponents;

            let has_matching = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| opponents.contains(&obj.controller))
                .any(|obj| filter.matches(obj, &filter_ctx, game));

            Ok(has_matching)
        }
        Condition::PlayerControls { player, filter } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            let has_matching = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .any(|obj| filter.matches(obj, &filter_ctx, game));
            Ok(has_matching)
        }
        Condition::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);

            if zones.is_empty() {
                return Ok(false);
            }

            let mut filter = crate::target::ObjectFilter::default().named(name.clone());
            for zone in zones {
                filter.zone = Some(*zone);
                let has_matching = condition_candidate_ids_for_zone(game, Some(*zone))
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| obj.owner == player_id)
                    .any(|obj| filter.matches(obj, &filter_ctx, game));
                if !has_matching {
                    return Ok(false);
                }
            }

            Ok(true)
        }
        Condition::PlayerControlsAtLeast {
            player,
            filter,
            count,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            let matches = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            Ok(matches >= *count as usize)
        }
        Condition::PlayerControlsExactly {
            player,
            filter,
            count,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            let matches = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            Ok(matches == *count as usize)
        }
        Condition::PlayerControlsAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            let distinct = count_distinct_matching_powers(game, player_id, filter, &filter_ctx);
            Ok(distinct >= *count as usize)
        }
        Condition::PlayerControlsMost { player, filter } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let count_for = |candidate: PlayerId| {
                let mut filter_ctx = ctx.filter_context(game);
                filter_ctx.iterated_player = Some(candidate);
                condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| condition_object_matches_player_zone(obj, candidate, filter.zone))
                    .filter(|obj| filter.matches(obj, &filter_ctx, game))
                    .count()
            };
            let current = count_for(player_id);
            let max_count = game
                .players
                .iter()
                .map(|player| count_for(player.id))
                .max()
                .unwrap_or(0);
            Ok(current == max_count)
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let you_life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            let other_life = game.player(player_id).map(|p| p.life).unwrap_or(0);
            Ok(other_life < you_life)
        }
        Condition::LifeTotalOrLess(threshold) => {
            let life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            Ok(life <= *threshold)
        }
        Condition::LifeTotalOrGreater(threshold) => {
            let life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            Ok(life >= *threshold)
        }
        Condition::CardsInHandOrMore(threshold) => {
            let count = game
                .player(ctx.controller)
                .map(|p| p.hand.len())
                .unwrap_or(0);
            Ok(count >= *threshold as usize)
        }
        Condition::YourTurn => Ok(game.turn.active_player == ctx.controller),
        Condition::CreatureDiedThisTurn => {
            // Check if any creature died this turn
            Ok(game.creatures_died_this_turn > 0)
        }
        Condition::CastSpellThisTurn => {
            // Check if any spell was cast this turn by anyone
            Ok(game.spells_cast_this_turn.values().any(|&count| count > 0))
        }
        Condition::AttackedThisTurn => {
            Ok(game.players_attacked_this_turn.contains(&ctx.controller))
        }
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game
                .players_tapped_land_for_mana_this_turn
                .contains(&player_id))
        }
        Condition::NoSpellsWereCastLastTurn => Ok(game.spells_cast_last_turn_total == 0),
        Condition::TargetIsTapped => {
            // Check if the target is tapped
            if let Some(crate::executor::ResolvedTarget::Object(id)) = ctx.targets.first() {
                return Ok(game.is_tapped(*id));
            }
            Ok(false)
        }
        Condition::TargetWasKicked => {
            for target in &ctx.targets {
                if let crate::executor::ResolvedTarget::Object(id) = target
                    && let Some(obj) = game.object(*id)
                {
                    return Ok(obj.optional_costs_paid.was_kicked());
                }
            }
            Ok(false)
        }
        Condition::TargetSpellCastOrderThisTurn(order) => {
            for target in &ctx.targets {
                if let crate::executor::ResolvedTarget::Object(id) = target {
                    let actual = game
                        .spell_cast_order_this_turn
                        .get(id)
                        .copied()
                        .unwrap_or(0);
                    return Ok(actual == *order);
                }
            }
            Ok(false)
        }
        Condition::TargetSpellControllerIsPoisoned => {
            for target in &ctx.targets {
                if let crate::executor::ResolvedTarget::Object(id) = target
                    && let Some(obj) = game.object(*id)
                    && let Some(player) = game.player(obj.controller)
                {
                    return Ok(player.poison_counters > 0);
                }
            }
            Ok(false)
        }
        Condition::TargetSpellManaSpentToCastAtLeast { amount, symbol } => {
            for target in &ctx.targets {
                if let crate::executor::ResolvedTarget::Object(id) = target
                    && let Some(obj) = game.object(*id)
                {
                    let spent = if let Some(sym) = symbol {
                        obj.mana_spent_to_cast.amount(*sym)
                    } else {
                        obj.mana_spent_to_cast.total()
                    };
                    return Ok(spent >= *amount);
                }
            }
            Ok(false)
        }
        Condition::YouControlMoreCreaturesThanTargetSpellController => {
            let target_controller = ctx.targets.iter().find_map(|target| match target {
                crate::executor::ResolvedTarget::Object(id) => {
                    game.object(*id).map(|obj| obj.controller)
                }
                _ => None,
            });
            let Some(target_controller) = target_controller else {
                return Ok(false);
            };

            let you_count = game
                .battlefield
                .iter()
                .filter(|&&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == ctx.controller
                            && game.object_has_card_type(id, crate::types::CardType::Creature)
                    })
                })
                .count();
            let target_count = game
                .battlefield
                .iter()
                .filter(|&&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == target_controller
                            && game.object_has_card_type(id, crate::types::CardType::Creature)
                    })
                })
                .count();
            Ok(you_count > target_count)
        }
        Condition::TargetHasGreatestPowerAmongCreatures => {
            let target_id = ctx.targets.iter().find_map(|target| match target {
                crate::executor::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            });
            let Some(target_id) = target_id else {
                return Ok(false);
            };
            let Some(target_obj) = game.object(target_id) else {
                return Ok(false);
            };
            if !game.object_has_card_type(target_id, crate::types::CardType::Creature) {
                return Ok(false);
            }
            let Some(target_power) = game
                .calculated_power(target_id)
                .or_else(|| target_obj.power())
            else {
                return Ok(false);
            };
            let max_power = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| game.object_has_card_type(obj.id, crate::types::CardType::Creature))
                .filter_map(|obj| game.calculated_power(obj.id).or_else(|| obj.power()))
                .max();
            Ok(max_power.is_some_and(|max| target_power >= max))
        }
        Condition::TargetManaValueLteColorsSpentToCastThisSpell => {
            let target_id = ctx.targets.iter().find_map(|target| match target {
                crate::executor::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            });
            let Some(target_id) = target_id else {
                return Ok(false);
            };
            let Some(target_obj) = game.object(target_id) else {
                return Ok(false);
            };
            let Some(source_obj) = game.object(ctx.source) else {
                return Ok(false);
            };
            let target_mana_value = target_obj
                .mana_cost
                .as_ref()
                .map(|cost| cost.mana_value())
                .unwrap_or(0);
            let colors_spent = [
                source_obj.mana_spent_to_cast.white,
                source_obj.mana_spent_to_cast.blue,
                source_obj.mana_spent_to_cast.black,
                source_obj.mana_spent_to_cast.red,
                source_obj.mana_spent_to_cast.green,
            ]
            .into_iter()
            .filter(|amount| *amount > 0)
            .count() as u32;
            Ok(target_mana_value <= colors_spent)
        }
        Condition::SourceIsTapped => Ok(game.is_tapped(ctx.source)),
        Condition::SourceHasNoCounter(counter_type) => Ok(game
            .object(ctx.source)
            .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) == 0)
            .unwrap_or(false)),
        Condition::TargetIsAttacking => {
            // Check if the target is among declared attackers
            // Note: Combat attackers are tracked in game_loop, not game_state directly.
            // For now, check ctx.attacking_creatures if it exists
            if let Some(crate::executor::ResolvedTarget::Object(id)) = ctx.targets.first() {
                // Simplified: check if it's a creature that's tapped (attackers are usually tapped)
                // Full implementation would need access to combat state from game loop
                if let Some(obj) = game.object(*id) {
                    return Ok(
                        game.object_has_card_type(obj.id, crate::types::CardType::Creature)
                            && game.is_tapped(*id),
                    );
                }
            }
            Ok(false)
        }
        Condition::TargetIsBlocked => {
            if let Some(crate::executor::ResolvedTarget::Object(id)) = ctx.targets.first()
                && let Some(combat) = &game.combat
            {
                return Ok(crate::combat_state::is_blocked(combat, *id));
            }
            Ok(false)
        }
        Condition::YouControlCommander => {
            // Check if you control a commander on the battlefield
            // This matches the logic in GameState::player_controls_a_commander
            // which checks both direct ID and stable_id (important when commander
            // was cast from command zone and got a new object ID)
            if let Some(player) = game.player(ctx.controller) {
                let commanders = player.get_commanders();
                for &commander_id in commanders {
                    // First check: is the commander ID directly on battlefield?
                    if game.battlefield.contains(&commander_id)
                        && let Some(obj) = game.object(commander_id)
                        && obj.controller == ctx.controller
                    {
                        return Ok(true);
                    }
                    // Second check: is there an object on battlefield whose stable_id
                    // matches the commander ID? (handles zone transitions)
                    for &bf_id in &game.battlefield {
                        if let Some(obj) = game.object(bf_id)
                            && obj.controller == ctx.controller
                            && obj.stable_id == StableId::from(commander_id)
                        {
                            return Ok(true);
                        }
                    }
                }
            }
            Ok(false)
        }
        Condition::TaggedObjectMatches(tag, filter) => {
            let filter_ctx = ctx.filter_context(game);
            let Some(snapshot) = ctx.get_tagged(tag.as_str()) else {
                return Ok(false);
            };
            Ok(filter.matches_snapshot(snapshot, &filter_ctx, game))
        }
        Condition::PlayerTaggedObjectMatches {
            player,
            tag,
            filter,
        } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let Some(tagged) = ctx.get_tagged_all(tag.as_str()) else {
                return Ok(false);
            };
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.iterated_player = Some(player_id);
            for snapshot in tagged {
                if snapshot.controller != player_id {
                    continue;
                }
                if filter.matches_snapshot(snapshot, &filter_ctx, game) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            let Some(source_obj) = game.object(ctx.source) else {
                return Ok(false);
            };
            let spent = if let Some(sym) = symbol {
                source_obj.mana_spent_to_cast.amount(*sym)
            } else {
                source_obj.mana_spent_to_cast.total()
            };
            Ok(spent >= *amount)
        }
        Condition::Not(inner) => {
            let inner_result = evaluate_condition(game, inner, ctx)?;
            Ok(!inner_result)
        }
        Condition::And(a, b) => {
            let a_result = evaluate_condition(game, a, ctx)?;
            if !a_result {
                return Ok(false);
            }
            evaluate_condition(game, b, ctx)
        }
        Condition::Or(a, b) => {
            let a_result = evaluate_condition(game, a, ctx)?;
            if a_result {
                return Ok(true);
            }
            evaluate_condition(game, b, ctx)
        }
    }
}
