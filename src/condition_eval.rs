use crate::effect::Condition;
use crate::effect::Value;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::effects::helpers::resolve_value;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::target::PlayerFilter;
use crate::zone::Zone;

use crate::triggers::{TriggerEvent, TriggerIdentity};

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

#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalEvaluationOptions {
    /// If true, treat timing restrictions as satisfied.
    pub ignore_timing: bool,
    /// If true, treat per-turn activation limits as satisfied.
    pub ignore_activation_limits: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalEvaluationContext<'a> {
    pub controller: PlayerId,
    pub source: ObjectId,
    /// The `FilterContext.source` used when matching ObjectFilters.
    ///
    /// This is intentionally configurable to preserve legacy semantics:
    /// - Intervening-if checks historically passed `None` so `other` filters do not exclude the source.
    /// - Most other checks should pass `Some(source)`.
    pub filter_source: Option<ObjectId>,
    pub triggering_event: Option<&'a TriggerEvent>,
    pub trigger_identity: Option<TriggerIdentity>,
    pub ability_index: Option<usize>,
    pub options: ExternalEvaluationOptions,
}

/// Evaluate a condition outside of effect resolution (trigger checks, activation gating, statics).
pub fn evaluate_condition_external(
    game: &GameState,
    condition: &Condition,
    ctx: &ExternalEvaluationContext<'_>,
) -> bool {
    use crate::types::{CardType, Subtype};

    match condition {
        Condition::Not(inner) => !evaluate_condition_external(game, inner, ctx),
        Condition::And(a, b) => {
            evaluate_condition_external(game, a, ctx) && evaluate_condition_external(game, b, ctx)
        }
        Condition::Or(a, b) => {
            evaluate_condition_external(game, a, ctx) || evaluate_condition_external(game, b, ctx)
        }

        Condition::Unmodeled(_) => true,
        Condition::Custom(_) => false,
        Condition::XValueAtLeast(_) => false, // X not available in static context
        Condition::ThisSpellWasKicked => game
            .object(ctx.source)
            .is_some_and(|obj| obj.optional_costs_paid.was_kicked()),

        Condition::YouControl(filter) => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            game.battlefield.iter().any(|&obj_id| {
                game.object(obj_id).is_some_and(|obj| {
                    obj.controller == ctx.controller && filter.matches(obj, &filter_ctx, game)
                })
            })
        }
        Condition::OpponentControls(filter) => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            let opponents = &filter_ctx.opponents;
            game.battlefield.iter().any(|&obj_id| {
                game.object(obj_id).is_some_and(|obj| {
                    opponents.contains(&obj.controller) && filter.matches(obj, &filter_ctx, game)
                })
            })
        }
        Condition::LifeTotalOrLess(threshold) => game
            .player(ctx.controller)
            .map(|p| p.life <= *threshold)
            .unwrap_or(false),
        Condition::LifeTotalOrGreater(threshold) => game
            .player(ctx.controller)
            .map(|p| p.life >= *threshold)
            .unwrap_or(false),
        Condition::YourTurn => game.turn.active_player == ctx.controller,
        Condition::CreatureDiedThisTurn => game.creatures_died_this_turn > 0,
        Condition::CastSpellThisTurn => game.spells_cast_this_turn.values().any(|&count| count > 0),
        Condition::AttackedThisTurn => game.players_attacked_this_turn.contains(&ctx.controller),
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, ctx.controller, player)
            else {
                return false;
            };
            game.players_tapped_land_for_mana_this_turn
                .contains(&player_id)
        }
        Condition::NoSpellsWereCastLastTurn => game.spells_cast_last_turn_total == 0,
        Condition::CardsInHandOrMore(threshold) => game
            .player(ctx.controller)
            .map(|p| p.hand.len() as i32 >= *threshold)
            .unwrap_or(false),
        Condition::PlayerCardsInHandOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, ctx.controller, player)
            else {
                return false;
            };
            game.player(player_id)
                .map(|p| p.hand.len() as i32 >= *count)
                .unwrap_or(false)
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, ctx.controller, player)
            else {
                return false;
            };
            game.player(player_id)
                .map(|p| p.hand.len() as i32 <= *count)
                .unwrap_or(false)
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, ctx.controller, player)
            else {
                return false;
            };
            let you_life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            let other_life = game.player(player_id).map(|p| p.life).unwrap_or(0);
            other_life < you_life
        }

        Condition::FirstTimeThisTurn => ctx
            .trigger_identity
            .map(|id| game.trigger_fire_count_this_turn(ctx.source, id) == 0)
            .unwrap_or(true),
        Condition::MaxTimesEachTurn(limit) => ctx
            .trigger_identity
            .map(|id| game.trigger_fire_count_this_turn(ctx.source, id) < *limit)
            .unwrap_or(true),
        Condition::TriggeringObjectWasEnchanted => ctx
            .triggering_event
            .and_then(|event| event.snapshot())
            .is_some_and(|snapshot| snapshot.was_enchanted),
        Condition::TriggeringObjectHadCounters {
            counter_type,
            min_count,
        } => ctx
            .triggering_event
            .and_then(|event| event.snapshot())
            .is_some_and(|snapshot| {
                snapshot.counters.get(counter_type).copied().unwrap_or(0) >= *min_count
            }),

        Condition::ControlLandWithSubtype(required_subtypes) => {
            game.battlefield.iter().any(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == ctx.controller
                        && obj.is_land()
                        && required_subtypes
                            .iter()
                            .any(|subtype| obj.has_subtype(*subtype))
                })
            })
        }
        Condition::ControlAtLeastArtifacts(required_count) => {
            let controlled_artifacts = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    obj.controller == ctx.controller && obj.card_types.contains(&CardType::Artifact)
                })
                .count() as u32;
            controlled_artifacts >= *required_count
        }
        Condition::ControlAtLeastLands(required_count) => {
            let controlled_lands = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == ctx.controller && obj.is_land())
                .count() as u32;
            controlled_lands >= *required_count
        }
        Condition::ControlCreatureWithPowerAtLeast(required_power) => {
            game.battlefield.iter().any(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == ctx.controller
                        && obj.is_creature()
                        && obj
                            .power()
                            .is_some_and(|power| power >= *required_power as i32)
                })
            })
        }
        Condition::ControlCreaturesTotalPowerAtLeast(required_power) => {
            let total_power = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == ctx.controller && obj.is_creature())
                .map(|obj| obj.power().unwrap_or(0).max(0))
                .sum::<i32>();
            total_power >= *required_power as i32
        }
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            use crate::types::Subtype;
            use std::collections::HashSet;

            let Some(player_id) = resolve_condition_player_simple(game, ctx.controller, player)
            else {
                return false;
            };

            let mut seen: HashSet<Subtype> = HashSet::new();
            for obj in game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == player_id && obj.is_land())
            {
                for subtype in game.calculated_subtypes(obj.id) {
                    if matches!(
                        subtype,
                        Subtype::Plains
                            | Subtype::Island
                            | Subtype::Swamp
                            | Subtype::Mountain
                            | Subtype::Forest
                    ) {
                        seen.insert(subtype);
                    }
                }
            }
            seen.len() >= *count as usize
        }
        Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, ctx.controller, player)
            else {
                return false;
            };
            count_distinct_card_types_in_graveyard(game, player_id) >= *count as usize
        }
        Condition::CardInYourGraveyard {
            card_types,
            subtypes,
        } => game.player(ctx.controller).is_some_and(|player_state| {
            player_state.graveyard.iter().any(|&card_id| {
                let Some(card) = game.object(card_id) else {
                    return false;
                };
                let card_type_match = card_types.is_empty()
                    || card_types
                        .iter()
                        .any(|card_type| card.card_types.contains(card_type));
                let subtype_match = subtypes.is_empty()
                    || subtypes.iter().any(|subtype| card.has_subtype(*subtype));
                card_type_match && subtype_match
            })
        }),
        Condition::ActivationTiming(timing) => {
            if ctx.options.ignore_timing {
                return true;
            }
            match timing {
                crate::ability::ActivationTiming::AnyTime => true,
                crate::ability::ActivationTiming::DuringCombat => {
                    matches!(game.turn.phase, crate::game_state::Phase::Combat)
                }
                crate::ability::ActivationTiming::SorcerySpeed => {
                    game.turn.active_player == ctx.controller
                        && matches!(
                            game.turn.phase,
                            crate::game_state::Phase::FirstMain
                                | crate::game_state::Phase::NextMain
                        )
                        && game.stack_is_empty()
                }
                crate::ability::ActivationTiming::OncePerTurn => {
                    let Some(ability_index) = ctx.ability_index else {
                        return false;
                    };
                    game.ability_activation_count_this_turn(ctx.source, ability_index) == 0
                }
                crate::ability::ActivationTiming::DuringYourTurn => {
                    game.turn.active_player == ctx.controller
                }
                crate::ability::ActivationTiming::DuringOpponentsTurn => {
                    game.turn.active_player != ctx.controller
                }
            }
        }
        Condition::MaxActivationsPerTurn(limit) => {
            if ctx.options.ignore_activation_limits {
                return true;
            }
            let Some(ability_index) = ctx.ability_index else {
                return false;
            };
            game.ability_activation_count_this_turn(ctx.source, ability_index) < *limit
        }

        Condition::SourceIsEquipped => game.object(ctx.source).is_some_and(|source_obj| {
            source_obj.attachments.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.subtypes.contains(&Subtype::Equipment))
            })
        }),
        Condition::SourceIsEnchanted => game.object(ctx.source).is_some_and(|source_obj| {
            source_obj.attachments.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.subtypes.contains(&Subtype::Aura))
            })
        }),
        Condition::EquippedCreatureTapped => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| game.is_tapped(attached)),
        Condition::EquippedCreatureUntapped => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| !game.is_tapped(attached)),
        Condition::CountComparison {
            count, comparison, ..
        } => comparison.evaluate(crate::static_abilities::resolve_anthem_count_expression(
            count,
            game,
            ctx.source,
            ctx.controller,
        )),
        Condition::OwnsCardExiledWithCounter(counter) => game.exile.iter().any(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.owner == ctx.controller && obj.counters.get(counter).copied().unwrap_or(0) > 0
            })
        }),

        Condition::SourceAttackedThisTurn => game.creature_attacked_this_turn(ctx.source),
        Condition::SourceIsTapped => game.is_tapped(ctx.source),
        Condition::SourceIsUntapped => !game.is_tapped(ctx.source),
        Condition::SourceHasNoCounter(counter_type) => game
            .object(ctx.source)
            .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) == 0)
            .unwrap_or(false),
        Condition::SourceHasCounterAtLeast {
            counter_type,
            count,
        } => game
            .object(ctx.source)
            .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) >= *count)
            .unwrap_or(false),
        Condition::SourceIsAttacking => game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_attacking(combat, ctx.source)),
        Condition::SourceIsBlocking => game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_blocking(combat, ctx.source)),

        Condition::PlayerGraveyardHasCardsAtLeast { player, count } => game
            .player(*player)
            .is_some_and(|p| p.graveyard.len() >= *count),

        Condition::YouControlCommander => {
            if let Some(player) = game.player(ctx.controller) {
                let commanders = player.get_commanders();
                for &commander_id in commanders {
                    if game.battlefield.contains(&commander_id)
                        && let Some(obj) = game.object(commander_id)
                        && obj.controller == ctx.controller
                    {
                        return true;
                    }
                    for &bf_id in &game.battlefield {
                        if let Some(obj) = game.object(bf_id)
                            && obj.controller == ctx.controller
                            && obj.stable_id == StableId::from(commander_id)
                        {
                            return true;
                        }
                    }
                }
            }
            false
        }

        // Conditions requiring targets / effect execution context are not evaluable here.
        Condition::TaggedObjectMatches(_, _)
        | Condition::PlayerTaggedObjectMatches { .. }
        | Condition::TargetIsTapped
        | Condition::TargetIsAttacking
        | Condition::TargetIsBlocked
        | Condition::TargetWasKicked
        | Condition::TargetSpellCastOrderThisTurn(_)
        | Condition::TargetSpellControllerIsPoisoned
        | Condition::TargetSpellManaSpentToCastAtLeast { .. }
        | Condition::YouControlMoreCreaturesThanTargetSpellController
        | Condition::TargetHasGreatestPowerAmongCreatures
        | Condition::TargetManaValueLteColorsSpentToCastThisSpell
        | Condition::ManaSpentToCastThisSpellAtLeast { .. }
        | Condition::PlayerControls { .. }
        | Condition::PlayerControlsAtLeast { .. }
        | Condition::PlayerControlsExactly { .. }
        | Condition::PlayerControlsAtLeastWithDifferentPowers { .. }
        | Condition::PlayerControlsMost { .. }
        | Condition::PlayerOwnsCardNamedInZones { .. } => false,
    }
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

fn count_distinct_card_types_in_graveyard(game: &GameState, player_id: PlayerId) -> usize {
    use std::collections::HashSet;

    let Some(player_state) = game.player(player_id) else {
        return 0;
    };

    let mut seen = HashSet::new();
    for &object_id in &player_state.graveyard {
        for card_type in game.calculated_card_types(object_id) {
            seen.insert(card_type);
        }
    }
    seen.len()
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
        Condition::ThisSpellWasKicked => game
            .object(source)
            .is_some_and(|obj| obj.optional_costs_paid.was_kicked()),
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
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            use crate::types::Subtype;
            use std::collections::HashSet;

            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };

            let mut seen: HashSet<Subtype> = HashSet::new();
            for obj in game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == player_id && obj.is_land())
            {
                for subtype in game.calculated_subtypes(obj.id) {
                    if matches!(
                        subtype,
                        Subtype::Plains
                            | Subtype::Island
                            | Subtype::Swamp
                            | Subtype::Mountain
                            | Subtype::Forest
                    ) {
                        seen.insert(subtype);
                    }
                }
            }
            seen.len() >= *count as usize
        }
        Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            count_distinct_card_types_in_graveyard(game, player_id) >= *count as usize
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
        Condition::PlayerCardsInHandOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            hand >= *count as usize
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            let hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            hand <= *count as usize
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
        Condition::SourceHasCounterAtLeast {
            counter_type,
            count,
        } => game
            .object(source)
            .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) >= *count)
            .unwrap_or(false),
        Condition::FirstTimeThisTurn | Condition::MaxTimesEachTurn(_) => true,
        Condition::TriggeringObjectWasEnchanted | Condition::TriggeringObjectHadCounters { .. } => {
            false
        }
        Condition::ControlLandWithSubtype(_)
        | Condition::ControlAtLeastArtifacts(_)
        | Condition::ControlAtLeastLands(_)
        | Condition::ControlCreatureWithPowerAtLeast(_)
        | Condition::ControlCreaturesTotalPowerAtLeast(_)
        | Condition::CardInYourGraveyard { .. }
        | Condition::ActivationTiming(_)
        | Condition::MaxActivationsPerTurn(_)
        | Condition::SourceIsEquipped
        | Condition::SourceIsEnchanted
        | Condition::EquippedCreatureTapped
        | Condition::EquippedCreatureUntapped
        | Condition::CountComparison { .. }
        | Condition::OwnsCardExiledWithCounter(_)
        | Condition::SourceAttackedThisTurn
        | Condition::SourceIsUntapped
        | Condition::SourceIsAttacking
        | Condition::SourceIsBlocking
        | Condition::PlayerGraveyardHasCardsAtLeast { .. }
        | Condition::Custom(_)
        | Condition::XValueAtLeast(_) => false,
        Condition::Unmodeled(_) => true,
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
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count } => {
            use crate::types::Subtype;
            use std::collections::HashSet;

            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let mut seen: HashSet<Subtype> = HashSet::new();
            for obj in game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == player_id && obj.is_land())
            {
                for subtype in game.calculated_subtypes(obj.id) {
                    if matches!(
                        subtype,
                        Subtype::Plains
                            | Subtype::Island
                            | Subtype::Swamp
                            | Subtype::Mountain
                            | Subtype::Forest
                    ) {
                        seen.insert(subtype);
                    }
                }
            }
            Ok(seen.len() >= *count as usize)
        }
        Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(count_distinct_card_types_in_graveyard(game, player_id) >= *count as usize)
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
        Condition::PlayerCardsInHandOrMore { player, count } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let hand_count = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            Ok(hand_count >= *count as usize)
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            let hand_count = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            Ok(hand_count <= *count as usize)
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
        Condition::ThisSpellWasKicked => Ok(resolve_value(game, &Value::WasKicked, ctx)? != 0),
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
        Condition::SourceHasCounterAtLeast {
            counter_type,
            count,
        } => Ok(game
            .object(ctx.source)
            .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) >= *count)
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
        Condition::FirstTimeThisTurn | Condition::MaxTimesEachTurn(_) => Ok(true),
        Condition::TriggeringObjectWasEnchanted => Ok(ctx
            .triggering_event
            .as_ref()
            .and_then(|event| event.snapshot())
            .is_some_and(|snapshot| snapshot.was_enchanted)),
        Condition::TriggeringObjectHadCounters {
            counter_type,
            min_count,
        } => Ok(ctx
            .triggering_event
            .as_ref()
            .and_then(|event| event.snapshot())
            .is_some_and(|snapshot| {
                snapshot.counters.get(counter_type).copied().unwrap_or(0) >= *min_count
            })),
        Condition::ControlLandWithSubtype(required_subtypes) => {
            Ok(game.battlefield.iter().any(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == ctx.controller
                        && obj.is_land()
                        && required_subtypes
                            .iter()
                            .any(|subtype| obj.has_subtype(*subtype))
                })
            }))
        }
        Condition::ControlAtLeastArtifacts(required_count) => Ok(game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| {
                obj.controller == ctx.controller
                    && obj.card_types.contains(&crate::types::CardType::Artifact)
            })
            .count() as u32
            >= *required_count),
        Condition::ControlAtLeastLands(required_count) => Ok(game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == ctx.controller && obj.is_land())
            .count() as u32
            >= *required_count),
        Condition::ControlCreatureWithPowerAtLeast(required_power) => {
            Ok(game.battlefield.iter().any(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == ctx.controller
                        && obj.is_creature()
                        && obj
                            .power()
                            .is_some_and(|power| power >= *required_power as i32)
                })
            }))
        }
        Condition::ControlCreaturesTotalPowerAtLeast(required_power) => {
            let total_power = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == ctx.controller && obj.is_creature())
                .map(|obj| obj.power().unwrap_or(0).max(0))
                .sum::<i32>();
            Ok(total_power >= *required_power as i32)
        }
        Condition::CardInYourGraveyard {
            card_types,
            subtypes,
        } => Ok(game.player(ctx.controller).is_some_and(|player_state| {
            player_state.graveyard.iter().any(|&card_id| {
                let Some(card) = game.object(card_id) else {
                    return false;
                };
                let card_type_match = card_types.is_empty()
                    || card_types
                        .iter()
                        .any(|card_type| card.card_types.contains(card_type));
                let subtype_match = subtypes.is_empty()
                    || subtypes.iter().any(|subtype| card.has_subtype(*subtype));
                card_type_match && subtype_match
            })
        })),
        Condition::ActivationTiming(_) | Condition::MaxActivationsPerTurn(_) => Ok(false),
        Condition::SourceIsEquipped => Ok(game.object(ctx.source).is_some_and(|source_obj| {
            source_obj.attachments.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.subtypes.contains(&crate::types::Subtype::Equipment))
            })
        })),
        Condition::SourceIsEnchanted => Ok(game.object(ctx.source).is_some_and(|source_obj| {
            source_obj.attachments.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.subtypes.contains(&crate::types::Subtype::Aura))
            })
        })),
        Condition::EquippedCreatureTapped => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| game.is_tapped(attached))),
        Condition::EquippedCreatureUntapped => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| !game.is_tapped(attached))),
        Condition::CountComparison {
            count, comparison, ..
        } => Ok(
            comparison.evaluate(crate::static_abilities::resolve_anthem_count_expression(
                count,
                game,
                ctx.source,
                ctx.controller,
            )),
        ),
        Condition::OwnsCardExiledWithCounter(counter) => Ok(game.exile.iter().any(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.owner == ctx.controller && obj.counters.get(counter).copied().unwrap_or(0) > 0
            })
        })),
        Condition::SourceAttackedThisTurn => Ok(game.creature_attacked_this_turn(ctx.source)),
        Condition::SourceIsUntapped => Ok(!game.is_tapped(ctx.source)),
        Condition::SourceIsAttacking => Ok(game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_attacking(combat, ctx.source))),
        Condition::SourceIsBlocking => Ok(game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_blocking(combat, ctx.source))),
        Condition::PlayerGraveyardHasCardsAtLeast { player, count } => Ok(game
            .player(*player)
            .is_some_and(|p| p.graveyard.len() >= *count)),
        Condition::Custom(_) => Ok(false),
        Condition::XValueAtLeast(min) => Ok(ctx.x_value.unwrap_or(0) >= *min),
        Condition::Unmodeled(_) => Ok(true),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::types::{CardType, Subtype};

    #[test]
    fn player_controls_basic_land_types_among_lands_or_more_counts_distinct_types() {
        use crate::target::PlayerFilter;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;

        let mk_land = |id: u32, name: &str, subtype: Subtype| {
            CardBuilder::new(CardId::from_raw(id), name)
                .card_types(vec![CardType::Land])
                .subtypes(vec![subtype])
                .build()
        };
        game.create_object_from_card(
            &mk_land(1, "Plains", Subtype::Plains),
            bob,
            Zone::Battlefield,
        );
        game.create_object_from_card(
            &mk_land(2, "Island", Subtype::Island),
            bob,
            Zone::Battlefield,
        );
        game.create_object_from_card(&mk_land(3, "Swamp", Subtype::Swamp), bob, Zone::Battlefield);
        game.create_object_from_card(
            &mk_land(4, "Mountain", Subtype::Mountain),
            bob,
            Zone::Battlefield,
        );

        let condition = Condition::PlayerControlsBasicLandTypesAmongLandsOrMore {
            player: PlayerFilter::Active,
            count: 4,
        };
        let ctx = ExternalEvaluationContext {
            controller: alice,
            source: ObjectId::from_raw(999),
            filter_source: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: ExternalEvaluationOptions::default(),
        };

        assert!(evaluate_condition_external(&game, &condition, &ctx));

        let condition = Condition::PlayerControlsBasicLandTypesAmongLandsOrMore {
            player: PlayerFilter::Active,
            count: 5,
        };
        assert!(!evaluate_condition_external(&game, &condition, &ctx));
    }

    #[test]
    fn player_has_card_types_in_graveyard_or_more_counts_distinct_types() {
        use crate::target::PlayerFilter;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let mk_card = |id: u32, name: &str, card_types: Vec<CardType>| {
            CardBuilder::new(CardId::from_raw(id), name)
                .card_types(card_types)
                .build()
        };

        game.create_object_from_card(
            &mk_card(11, "Artifact Shard", vec![CardType::Artifact]),
            alice,
            Zone::Graveyard,
        );
        game.create_object_from_card(
            &mk_card(12, "Creature Husk", vec![CardType::Creature]),
            alice,
            Zone::Graveyard,
        );
        game.create_object_from_card(
            &mk_card(13, "Sorcery Echo", vec![CardType::Sorcery]),
            alice,
            Zone::Graveyard,
        );
        game.create_object_from_card(
            &mk_card(14, "Enchantment Memory", vec![CardType::Enchantment]),
            alice,
            Zone::Graveyard,
        );

        let ctx = ExternalEvaluationContext {
            controller: alice,
            source: ObjectId::from_raw(999),
            filter_source: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: ExternalEvaluationOptions::default(),
        };

        let delirium = Condition::PlayerHasCardTypesInGraveyardOrMore {
            player: PlayerFilter::You,
            count: 4,
        };
        assert!(evaluate_condition_external(&game, &delirium, &ctx));

        let too_many = Condition::PlayerHasCardTypesInGraveyardOrMore {
            player: PlayerFilter::You,
            count: 5,
        };
        assert!(!evaluate_condition_external(&game, &too_many, &ctx));
    }
}
