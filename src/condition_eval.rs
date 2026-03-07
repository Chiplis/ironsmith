use crate::effect::Condition;
use crate::effect::Value;
use crate::effects::helpers::resolve_value;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::object_query::candidate_ids_for_zone;
use crate::target::PlayerFilter;
use crate::zone::Zone;

use crate::triggers::{TriggerEvent, TriggerIdentity};

fn source_was_cast(
    game: &GameState,
    source: ObjectId,
    triggering_event: Option<&TriggerEvent>,
) -> bool {
    if let Some(event) = triggering_event
        && let Some(etb) = event.downcast::<crate::events::EnterBattlefieldEvent>()
        && etb.object == source
    {
        return etb.from == Zone::Stack;
    }
    game.spell_cast_order_this_turn.contains_key(&source)
}

fn player_has_card_in_hand_matching(
    game: &GameState,
    player: PlayerId,
    filter: &crate::target::ObjectFilter,
    filter_source: Option<ObjectId>,
) -> bool {
    let filter_ctx = game.filter_context_for(player, filter_source);
    game.player(player).is_some_and(|state| {
        state.hand.iter().any(|&card_id| {
            game.object(card_id)
                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
        })
    })
}

#[derive(Debug, Clone, Copy)]
struct SharedConditionContext<'a> {
    controller: PlayerId,
    source: ObjectId,
    filter_source: Option<ObjectId>,
    triggering_event: Option<&'a TriggerEvent>,
}

fn evaluate_condition_shared_core(
    game: &GameState,
    condition: &Condition,
    ctx: SharedConditionContext<'_>,
) -> Option<bool> {
    match condition {
        Condition::LifeTotalOrLess(threshold) => Some(
            game.player(ctx.controller)
                .map(|p| p.life <= *threshold)
                .unwrap_or(false),
        ),
        Condition::LifeTotalOrGreater(threshold) => Some(
            game.player(ctx.controller)
                .map(|p| p.life >= *threshold)
                .unwrap_or(false),
        ),
        Condition::CardsInHandOrMore(threshold) => Some(
            game.player(ctx.controller)
                .map(|p| p.hand.len() as i32 >= *threshold)
                .unwrap_or(false),
        ),
        Condition::YouHaveCardInHandMatching(filter) => Some(player_has_card_in_hand_matching(
            game,
            ctx.controller,
            filter,
            ctx.filter_source,
        )),
        Condition::YourTurn => Some(game.turn.active_player == ctx.controller),
        Condition::CreatureDiedThisTurn => Some(game.creatures_died_this_turn > 0),
        Condition::CastSpellThisTurn => {
            Some(game.spells_cast_this_turn.values().any(|&count| count > 0))
        }
        Condition::AttackedThisTurn => {
            Some(game.players_attacked_this_turn.contains(&ctx.controller))
        }
        Condition::OpponentLostLifeThisTurn => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            Some(
                filter_ctx.opponents.iter().any(|opponent| {
                    game.life_lost_this_turn.get(opponent).copied().unwrap_or(0) > 0
                }),
            )
        }
        Condition::PermanentLeftBattlefieldUnderYourControlThisTurn => Some(
            game.permanents_left_battlefield_under_controller_this_turn
                .get(&ctx.controller)
                .copied()
                .unwrap_or(0)
                > 0,
        ),
        Condition::SourceWasCast => Some(source_was_cast(game, ctx.source, ctx.triggering_event)),
        Condition::NoSpellsWereCastLastTurn => Some(game.spells_cast_last_turn_total == 0),
        Condition::SpellsWereCastLastTurnOrMore(count) => {
            Some(game.spells_cast_last_turn_total >= *count)
        }
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            let Some(source_obj) = game.object(ctx.source) else {
                return Some(false);
            };
            let spent = if let Some(sym) = symbol {
                source_obj.mana_spent_to_cast.amount(*sym)
            } else {
                source_obj.mana_spent_to_cast.total()
            };
            Some(spent >= *amount)
        }
        Condition::ColorsOfManaSpentToCastThisSpellOrMore(amount) => {
            let Some(source_obj) = game.object(ctx.source) else {
                return Some(false);
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
            .count() as u32;
            Some(distinct_colors >= *amount)
        }
        Condition::SourceHasNoCounter(counter_type) => Some(
            game.object(ctx.source)
                .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) == 0)
                .unwrap_or(false),
        ),
        Condition::SourceHasCounterAtLeast {
            counter_type,
            count,
        } => Some(
            game.object(ctx.source)
                .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) >= *count)
                .unwrap_or(false),
        ),
        Condition::PlayerGraveyardHasCardsAtLeast { player, count } => Some(
            game.player(*player)
                .is_some_and(|p| p.graveyard.len() >= *count),
        ),
        Condition::YouControlCommander => {
            if let Some(player) = game.player(ctx.controller) {
                let commanders = player.get_commanders();
                for &commander_id in commanders {
                    if game.battlefield.contains(&commander_id)
                        && let Some(obj) = game.object(commander_id)
                        && obj.controller == ctx.controller
                    {
                        return Some(true);
                    }
                    for &bf_id in &game.battlefield {
                        if let Some(obj) = game.object(bf_id)
                            && obj.controller == ctx.controller
                            && obj.stable_id == StableId::from(commander_id)
                        {
                            return Some(true);
                        }
                    }
                }
            }
            Some(false)
        }
        Condition::Custom(_) => Some(false),
        Condition::Unmodeled(_) => Some(true),
        _ => None,
    }
}

fn assert_condition_variant_coverage(condition: &Condition) {
    match condition {
        Condition::YouControl(..) => {}
        Condition::OpponentControls(..) => {}
        Condition::PlayerControls { .. } => {}
        Condition::PlayerControlsAtLeast { .. } => {}
        Condition::PlayerControlsExactly { .. } => {}
        Condition::PlayerControlsAtLeastWithDifferentPowers { .. } => {}
        Condition::PlayerControlsMost { .. } => {}
        Condition::LifeTotalOrLess(..) => {}
        Condition::LifeTotalOrGreater(..) => {}
        Condition::CardsInHandOrMore(..) => {}
        Condition::YouHaveCardInHandMatching(..) => {}
        Condition::YourTurn => {}
        Condition::CreatureDiedThisTurn => {}
        Condition::CastSpellThisTurn => {}
        Condition::AttackedThisTurn => {}
        Condition::OpponentLostLifeThisTurn => {}
        Condition::PermanentLeftBattlefieldUnderYourControlThisTurn => {}
        Condition::SourceWasCast => {}
        Condition::NoSpellsWereCastLastTurn => {}
        Condition::SpellsWereCastLastTurnOrMore(..) => {}
        Condition::TargetIsTapped => {}
        Condition::TargetIsAttacking => {}
        Condition::TargetIsBlocked => {}
        Condition::TargetWasKicked => {}
        Condition::ThisSpellWasKicked => {}
        Condition::TargetSpellCastOrderThisTurn(..) => {}
        Condition::TargetSpellControllerIsPoisoned => {}
        Condition::TargetSpellManaSpentToCastAtLeast { .. } => {}
        Condition::YouControlMoreCreaturesThanTargetSpellController => {}
        Condition::TargetHasGreatestPowerAmongCreatures => {}
        Condition::TargetManaValueLteColorsSpentToCastThisSpell => {}
        Condition::SourceIsTapped => {}
        Condition::SourceIsFaceDown => {}
        Condition::SourceHasNoCounter(..) => {}
        Condition::SourceHasCounterAtLeast { .. } => {}
        Condition::ManaSpentToCastThisSpellAtLeast { .. } => {}
        Condition::ColorsOfManaSpentToCastThisSpellOrMore(..) => {}
        Condition::YouControlCommander => {}
        Condition::TaggedObjectMatches(..) => {}
        Condition::TaggedObjectIsSoulbondPaired(..) => {}
        Condition::EnchantedPermanentAttackedThisTurn => {}
        Condition::TargetMatches(..) => {}
        Condition::TargetIsSoulbondPaired => {}
        Condition::PlayerTaggedObjectMatches { .. } => {}
        Condition::PlayerOwnsCardNamedInZones { .. } => {}
        Condition::FirstTimeThisTurn => {}
        Condition::MaxTimesEachTurn(..) => {}
        Condition::TriggeringObjectWasEnchanted => {}
        Condition::TriggeringObjectHadCounters { .. } => {}
        Condition::ControlCreaturesTotalPowerAtLeast(..) => {}
        Condition::CardInYourGraveyard { .. } => {}
        Condition::ActivationTiming(..) => {}
        Condition::MaxActivationsPerTurn(..) => {}
        Condition::SourceIsEquipped => {}
        Condition::SourceIsEnchanted => {}
        Condition::EnchantedPermanentIsCreature => {}
        Condition::EnchantedPermanentIsEquipment => {}
        Condition::EnchantedPermanentIsVehicle => {}
        Condition::EquippedCreatureTapped => {}
        Condition::EquippedCreatureUntapped => {}
        Condition::CountComparison { .. } => {}
        Condition::OwnsCardExiledWithCounter(..) => {}
        Condition::SourceAttackedThisTurn => {}
        Condition::SourceIsUntapped => {}
        Condition::SourceIsAttacking => {}
        Condition::SourceIsBlocking => {}
        Condition::SourceIsSoulbondPaired => {}
        Condition::XValueAtLeast(..) => {}
        Condition::Custom(..) => {}
        Condition::Unmodeled(..) => {}
        Condition::Not(..) => {}
        Condition::And(..) => {}
        Condition::Or(..) => {}
        Condition::PlayerCastSpellsThisTurnOrMore { .. } => {}
        Condition::PlayerTappedLandForManaThisTurn { .. } => {}
        Condition::PlayerHadLandEnterBattlefieldThisTurn { .. } => {}
        Condition::PlayerCardsInHandOrMore { .. } => {}
        Condition::PlayerCardsInHandOrFewer { .. } => {}
        Condition::PlayerControlsBasicLandTypesAmongLandsOrMore { .. } => {}
        Condition::PlayerHasCardTypesInGraveyardOrMore { .. } => {}
        Condition::PlayerHasLessLifeThanYou { .. } => {}
        Condition::PlayerHasCitysBlessing { .. } => {}
        Condition::PlayerGraveyardHasCardsAtLeast { .. } => {}
    }
}

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
    /// Player currently being attacked (if evaluation occurs in an attack-defender context).
    pub defending_player: Option<PlayerId>,
    /// Player currently attacking (if different from `controller` in a delegated context).
    pub attacking_player: Option<PlayerId>,
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
    assert_condition_variant_coverage(condition);
    use crate::types::{CardType, Subtype};

    if let Condition::Not(inner) = condition {
        return !evaluate_condition_external(game, inner, ctx);
    }
    if let Condition::And(a, b) = condition {
        return evaluate_condition_external(game, a, ctx)
            && evaluate_condition_external(game, b, ctx);
    }
    if let Condition::Or(a, b) = condition {
        return evaluate_condition_external(game, a, ctx)
            || evaluate_condition_external(game, b, ctx);
    }
    if let Some(result) = evaluate_condition_shared_core(
        game,
        condition,
        SharedConditionContext {
            controller: ctx.controller,
            source: ctx.source,
            filter_source: ctx.filter_source,
            triggering_event: ctx.triggering_event,
        },
    ) {
        return result;
    }

    match condition {
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
        Condition::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let filter_ctx = game.filter_context_for(ctx.controller, ctx.filter_source);
            let players: Vec<PlayerId> = match player {
                PlayerFilter::You => vec![ctx.controller],
                PlayerFilter::Opponent => filter_ctx.opponents.clone(),
                PlayerFilter::Specific(id) => vec![*id],
                PlayerFilter::Any => game.players.iter().map(|p| p.id).collect(),
                PlayerFilter::NotYou => game
                    .players
                    .iter()
                    .filter_map(|p| (p.id != ctx.controller).then_some(p.id))
                    .collect(),
                _ => Vec::new(),
            };
            let cast_count: u32 = players
                .iter()
                .map(|pid| game.spells_cast_this_turn.get(pid).copied().unwrap_or(0))
                .sum();
            cast_count >= *count
        }
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.players_tapped_land_for_mana_this_turn
                .contains(&player_id)
        }
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            player_had_land_enter_battlefield_this_turn(game, player_id)
        }
        Condition::PlayerCardsInHandOrMore { player, count } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.player(player_id)
                .map(|p| p.hand.len() as i32 >= *count)
                .unwrap_or(false)
        }
        Condition::PlayerCardsInHandOrFewer { player, count } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.player(player_id)
                .map(|p| p.hand.len() as i32 <= *count)
                .unwrap_or(false)
        }
        Condition::PlayerHasLessLifeThanYou { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let you_life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            let other_life = game.player(player_id).map(|p| p.life).unwrap_or(0);
            other_life < you_life
        }
        Condition::PlayerHasCitysBlessing { player } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            game.has_citys_blessing(player_id)
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

            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
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
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
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
        Condition::PlayerControls { player, filter } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut filter_ctx = crate::filter::FilterContext::new(player_id)
                .with_source(ctx.source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                filter_ctx = filter_ctx.with_iterated_player(Some(player_id));
            }
            condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .any(|obj| filter.matches(obj, &filter_ctx, game))
        }
        Condition::PlayerControlsAtLeast {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut filter_ctx = crate::filter::FilterContext::new(player_id)
                .with_source(ctx.source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                filter_ctx = filter_ctx.with_iterated_player(Some(player_id));
            }
            let matches = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            matches >= *count as usize
        }
        Condition::PlayerControlsExactly {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut filter_ctx = crate::filter::FilterContext::new(player_id)
                .with_source(ctx.source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                filter_ctx = filter_ctx.with_iterated_player(Some(player_id));
            }
            let matches = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            matches == *count as usize
        }
        Condition::PlayerControlsAtLeastWithDifferentPowers {
            player,
            filter,
            count,
        } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut filter_ctx = crate::filter::FilterContext::new(player_id)
                .with_source(ctx.source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                filter_ctx = filter_ctx.with_iterated_player(Some(player_id));
            }
            count_distinct_matching_powers(game, player_id, filter, &filter_ctx) >= *count as usize
        }
        Condition::PlayerControlsMost { player, filter } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut filter_ctx = crate::filter::FilterContext::new(player_id)
                .with_source(ctx.source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                filter_ctx = filter_ctx.with_iterated_player(Some(player_id));
            }
            let your_count = condition_candidate_ids_for_zone(game, filter.zone)
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| condition_object_matches_player_zone(obj, player_id, filter.zone))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .count();
            game.players.iter().filter(|p| p.id != player_id).all(|p| {
                let other_id = p.id;
                let other_opponents: Vec<PlayerId> = game
                    .players
                    .iter()
                    .filter(|q| q.id != other_id)
                    .map(|q| q.id)
                    .collect();
                let mut other_ctx = crate::filter::FilterContext::new(other_id)
                    .with_source(ctx.source)
                    .with_opponents(other_opponents);
                if *player == PlayerFilter::IteratedPlayer {
                    other_ctx = other_ctx.with_iterated_player(Some(other_id));
                }
                let other_count = condition_candidate_ids_for_zone(game, filter.zone)
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| condition_object_matches_player_zone(obj, other_id, filter.zone))
                    .filter(|obj| filter.matches(obj, &other_ctx, game))
                    .count();
                your_count >= other_count
            })
        }
        Condition::PlayerOwnsCardNamedInZones {
            player,
            name,
            zones,
        } => {
            let Some(player_id) = resolve_condition_player_external(game, ctx, player) else {
                return false;
            };
            let opponents: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != player_id)
                .map(|p| p.id)
                .collect();
            let mut filter_ctx = crate::filter::FilterContext::new(player_id)
                .with_source(ctx.source)
                .with_opponents(opponents);
            if *player == PlayerFilter::IteratedPlayer {
                filter_ctx = filter_ctx.with_iterated_player(Some(player_id));
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
                    .any(|obj| filter.matches(obj, &filter_ctx, game));
                if !has_matching {
                    return false;
                }
            }
            true
        }
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
        Condition::EnchantedPermanentIsCreature => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| game.object_has_card_type(attached, CardType::Creature)),
        Condition::EnchantedPermanentIsEquipment => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| {
                game.calculated_subtypes(attached)
                    .contains(&crate::types::Subtype::Equipment)
            }),
        Condition::EnchantedPermanentIsVehicle => game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| {
                game.calculated_subtypes(attached)
                    .contains(&crate::types::Subtype::Vehicle)
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
        Condition::SourceIsFaceDown => game.is_face_down(ctx.source),
        Condition::SourceIsUntapped => !game.is_tapped(ctx.source),
        Condition::SourceIsAttacking => game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_attacking(combat, ctx.source)),
        Condition::SourceIsBlocking => game
            .combat
            .as_ref()
            .is_some_and(|combat| crate::combat_state::is_blocking(combat, ctx.source)),
        Condition::SourceIsSoulbondPaired => game.is_soulbond_paired(ctx.source),

        // Conditions requiring targets / effect execution context are not evaluable here.
        Condition::TaggedObjectMatches(_, _)
        | Condition::TaggedObjectIsSoulbondPaired(_)
        | Condition::EnchantedPermanentAttackedThisTurn
        | Condition::TargetMatches(_)
        | Condition::TargetIsSoulbondPaired
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
        | Condition::TargetManaValueLteColorsSpentToCastThisSpell => false,
        Condition::Custom(_)
        | Condition::Unmodeled(_)
        | Condition::LifeTotalOrLess(_)
        | Condition::LifeTotalOrGreater(_)
        | Condition::CardsInHandOrMore(_)
        | Condition::YouHaveCardInHandMatching(_)
        | Condition::YourTurn
        | Condition::CreatureDiedThisTurn
        | Condition::CastSpellThisTurn
        | Condition::AttackedThisTurn
        | Condition::OpponentLostLifeThisTurn
        | Condition::PermanentLeftBattlefieldUnderYourControlThisTurn
        | Condition::SourceWasCast
        | Condition::NoSpellsWereCastLastTurn
        | Condition::SpellsWereCastLastTurnOrMore(_)
        | Condition::SourceHasNoCounter(_)
        | Condition::SourceHasCounterAtLeast { .. }
        | Condition::ManaSpentToCastThisSpellAtLeast { .. }
        | Condition::ColorsOfManaSpentToCastThisSpellOrMore(_)
        | Condition::PlayerGraveyardHasCardsAtLeast { .. }
        | Condition::YouControlCommander
        | Condition::Not(_)
        | Condition::And(_, _)
        | Condition::Or(_, _) => unreachable!("handled before external match"),
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
    candidate_ids_for_zone(game, zone)
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

fn player_had_land_enter_battlefield_this_turn(game: &GameState, player_id: PlayerId) -> bool {
    game.objects_entered_battlefield_this_turn
        .iter()
        .any(|(stable_id, entry_controller)| {
            *entry_controller == player_id
                && game
                    .find_object_by_stable_id(*stable_id)
                    .is_some_and(|object_id| {
                        game.object_has_card_type(object_id, crate::types::CardType::Land)
                    })
        })
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
    assert_condition_variant_coverage(condition);
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

    if let Condition::Not(inner) = condition {
        return !evaluate_condition_simple(game, inner, controller, source);
    }
    if let Condition::And(a, b) = condition {
        return evaluate_condition_simple(game, a, controller, source)
            && evaluate_condition_simple(game, b, controller, source);
    }
    if let Condition::Or(a, b) = condition {
        return evaluate_condition_simple(game, a, controller, source)
            || evaluate_condition_simple(game, b, controller, source);
    }
    if let Some(result) = evaluate_condition_shared_core(
        game,
        condition,
        SharedConditionContext {
            controller,
            source,
            filter_source: Some(source),
            triggering_event: None,
        },
    ) {
        return result;
    }

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
        Condition::PlayerHasCitysBlessing { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.has_citys_blessing(player_id)
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
        Condition::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let filter_ctx = game.filter_context_for(controller, Some(source));
            let players: Vec<PlayerId> = match player {
                PlayerFilter::You => vec![controller],
                PlayerFilter::Opponent => filter_ctx.opponents,
                PlayerFilter::Specific(id) => vec![*id],
                PlayerFilter::Any => game.players.iter().map(|p| p.id).collect(),
                PlayerFilter::NotYou => game
                    .players
                    .iter()
                    .filter_map(|p| (p.id != controller).then_some(p.id))
                    .collect(),
                _ => Vec::new(),
            };
            let cast_count: u32 = players
                .iter()
                .map(|pid| game.spells_cast_this_turn.get(pid).copied().unwrap_or(0))
                .sum();
            cast_count >= *count
        }
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            game.players_tapped_land_for_mana_this_turn
                .contains(&player_id)
        }
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            let Some(player_id) = resolve_condition_player_simple(game, controller, player) else {
                return false;
            };
            player_had_land_enter_battlefield_this_turn(game, player_id)
        }
        Condition::FirstTimeThisTurn | Condition::MaxTimesEachTurn(_) => true,
        Condition::TriggeringObjectWasEnchanted | Condition::TriggeringObjectHadCounters { .. } => {
            false
        }
        Condition::ControlCreaturesTotalPowerAtLeast(_)
        | Condition::CardInYourGraveyard { .. }
        | Condition::ActivationTiming(_)
        | Condition::MaxActivationsPerTurn(_)
        | Condition::SourceIsEquipped
        | Condition::SourceIsEnchanted
        | Condition::EnchantedPermanentIsCreature
        | Condition::EnchantedPermanentIsEquipment
        | Condition::EnchantedPermanentIsVehicle
        | Condition::EquippedCreatureTapped
        | Condition::EquippedCreatureUntapped
        | Condition::CountComparison { .. }
        | Condition::OwnsCardExiledWithCounter(_)
        | Condition::SourceAttackedThisTurn
        | Condition::SourceIsUntapped
        | Condition::SourceIsAttacking
        | Condition::SourceIsBlocking
        | Condition::SourceIsSoulbondPaired
        | Condition::XValueAtLeast(_) => false,
        Condition::TaggedObjectMatches(_, _) => false,
        Condition::TaggedObjectIsSoulbondPaired(_) => false,
        Condition::EnchantedPermanentAttackedThisTurn => false,
        Condition::TargetMatches(_) => false,
        Condition::TargetIsSoulbondPaired => false,
        Condition::PlayerTaggedObjectMatches { .. } => false,
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
        | Condition::SourceIsTapped
        | Condition::SourceIsFaceDown => false,
        Condition::Custom(_)
        | Condition::Unmodeled(_)
        | Condition::LifeTotalOrLess(_)
        | Condition::LifeTotalOrGreater(_)
        | Condition::CardsInHandOrMore(_)
        | Condition::YouHaveCardInHandMatching(_)
        | Condition::YourTurn
        | Condition::CreatureDiedThisTurn
        | Condition::CastSpellThisTurn
        | Condition::AttackedThisTurn
        | Condition::OpponentLostLifeThisTurn
        | Condition::PermanentLeftBattlefieldUnderYourControlThisTurn
        | Condition::SourceWasCast
        | Condition::NoSpellsWereCastLastTurn
        | Condition::SpellsWereCastLastTurnOrMore(_)
        | Condition::SourceHasNoCounter(_)
        | Condition::SourceHasCounterAtLeast { .. }
        | Condition::ManaSpentToCastThisSpellAtLeast { .. }
        | Condition::ColorsOfManaSpentToCastThisSpellOrMore(_)
        | Condition::PlayerGraveyardHasCardsAtLeast { .. }
        | Condition::YouControlCommander
        | Condition::Not(_)
        | Condition::And(_, _)
        | Condition::Or(_, _) => {
            unreachable!("handled before cast-time match")
        }
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
        | PlayerFilter::TargetPlayerOrControllerOfTarget
        | PlayerFilter::Excluding { .. }
        | PlayerFilter::ControllerOf(_)
        | PlayerFilter::OwnerOf(_) => None,
    }
}

fn resolve_condition_player_external(
    game: &GameState,
    ctx: &ExternalEvaluationContext<'_>,
    player: &PlayerFilter,
) -> Option<PlayerId> {
    match player {
        PlayerFilter::Defending => ctx.defending_player,
        PlayerFilter::Attacking => Some(ctx.attacking_player.unwrap_or(ctx.controller)),
        _ => resolve_condition_player_simple(game, ctx.controller, player),
    }
}

/// Evaluate a condition.
fn evaluate_condition(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    assert_condition_variant_coverage(condition);

    if let Condition::Not(inner) = condition {
        let inner_result = evaluate_condition(game, inner, ctx)?;
        return Ok(!inner_result);
    }
    if let Condition::And(a, b) = condition {
        let a_result = evaluate_condition(game, a, ctx)?;
        if !a_result {
            return Ok(false);
        }
        return evaluate_condition(game, b, ctx);
    }
    if let Condition::Or(a, b) = condition {
        let a_result = evaluate_condition(game, a, ctx)?;
        if a_result {
            return Ok(true);
        }
        return evaluate_condition(game, b, ctx);
    }
    if let Some(result) = evaluate_condition_shared_core(
        game,
        condition,
        SharedConditionContext {
            controller: ctx.controller,
            source: ctx.source,
            filter_source: Some(ctx.source),
            triggering_event: ctx.triggering_event.as_ref(),
        },
    ) {
        return Ok(result);
    }

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
        Condition::PlayerHasCitysBlessing { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game.has_citys_blessing(player_id))
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
        Condition::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let filter_ctx = ctx.filter_context(game);
            let player_ids: Vec<PlayerId> = match player {
                PlayerFilter::You => vec![ctx.controller],
                PlayerFilter::Opponent => filter_ctx.opponents,
                PlayerFilter::Specific(id) => vec![*id],
                PlayerFilter::Any => game.players.iter().map(|p| p.id).collect(),
                PlayerFilter::NotYou => game
                    .players
                    .iter()
                    .filter_map(|p| (p.id != ctx.controller).then_some(p.id))
                    .collect(),
                _ => Vec::new(),
            };
            let cast_count: u32 = player_ids
                .iter()
                .map(|pid| game.spells_cast_this_turn.get(pid).copied().unwrap_or(0))
                .sum();
            Ok(cast_count >= *count)
        }
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(game
                .players_tapped_land_for_mana_this_turn
                .contains(&player_id))
        }
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            let player_id = crate::effects::helpers::resolve_player_filter(game, player, ctx)?;
            Ok(player_had_land_enter_battlefield_this_turn(game, player_id))
        }
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
        Condition::SourceIsFaceDown => Ok(game.is_face_down(ctx.source)),
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
        Condition::TaggedObjectMatches(tag, filter) => {
            let filter_ctx = ctx.filter_context(game);
            if let Some(snapshot) = ctx.get_tagged(tag.as_str()) {
                return Ok(filter.matches_snapshot(snapshot, &filter_ctx, game));
            }

            // Some compile-time conditional lowering paths synthesize a branch-local tag
            // (for example "countered_0") before runtime tagging exists. In these cases,
            // fall back to evaluating against the first object target.
            let synthetic_tag = tag.as_str().rsplit_once('_').is_some_and(|(head, suffix)| {
                !head.is_empty() && suffix.chars().all(|c| c.is_ascii_digit())
            });
            if !synthetic_tag {
                return Ok(false);
            }

            let Some(crate::executor::ResolvedTarget::Object(id)) = ctx.targets.first() else {
                return Ok(false);
            };
            if let Some(obj) = game.object(*id) {
                return Ok(filter.matches(obj, &filter_ctx, game));
            }
            if let Some(snapshot) = ctx.target_snapshots.get(id) {
                return Ok(filter.matches_snapshot(snapshot, &filter_ctx, game));
            }
            Ok(false)
        }
        Condition::TaggedObjectIsSoulbondPaired(tag) => {
            let tagged_id = ctx.get_tagged(tag.as_str()).map(|snapshot| snapshot.object_id);
            Ok(tagged_id.is_some_and(|id| game.is_soulbond_paired(id)))
        }
        Condition::EnchantedPermanentAttackedThisTurn => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached_to| game.creature_attacked_this_turn(attached_to))),
        Condition::TargetMatches(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let Some(crate::executor::ResolvedTarget::Object(id)) = ctx.targets.first() else {
                return Ok(false);
            };
            if let Some(obj) = game.object(*id) {
                return Ok(filter.matches(obj, &filter_ctx, game));
            }
            if let Some(snapshot) = ctx.target_snapshots.get(id) {
                return Ok(filter.matches_snapshot(snapshot, &filter_ctx, game));
            }
            Ok(false)
        }
        Condition::TargetIsSoulbondPaired => {
            let target_id = ctx.targets.iter().find_map(|target| match target {
                crate::executor::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            });
            Ok(target_id.is_some_and(|id| game.is_soulbond_paired(id)))
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
        Condition::EnchantedPermanentIsCreature => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| {
                game.object_has_card_type(attached, crate::types::CardType::Creature)
            })),
        Condition::EnchantedPermanentIsEquipment => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| {
                game.calculated_subtypes(attached)
                    .contains(&crate::types::Subtype::Equipment)
            })),
        Condition::EnchantedPermanentIsVehicle => Ok(game
            .object(ctx.source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| {
                game.calculated_subtypes(attached)
                    .contains(&crate::types::Subtype::Vehicle)
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
        Condition::SourceIsSoulbondPaired => Ok(game.is_soulbond_paired(ctx.source)),
        Condition::XValueAtLeast(min) => Ok(ctx.x_value.unwrap_or(0) >= *min),
        Condition::Custom(_)
        | Condition::Unmodeled(_)
        | Condition::LifeTotalOrLess(_)
        | Condition::LifeTotalOrGreater(_)
        | Condition::CardsInHandOrMore(_)
        | Condition::YouHaveCardInHandMatching(_)
        | Condition::YourTurn
        | Condition::CreatureDiedThisTurn
        | Condition::CastSpellThisTurn
        | Condition::AttackedThisTurn
        | Condition::OpponentLostLifeThisTurn
        | Condition::PermanentLeftBattlefieldUnderYourControlThisTurn
        | Condition::SourceWasCast
        | Condition::NoSpellsWereCastLastTurn
        | Condition::SpellsWereCastLastTurnOrMore(_)
        | Condition::SourceHasNoCounter(_)
        | Condition::SourceHasCounterAtLeast { .. }
        | Condition::ManaSpentToCastThisSpellAtLeast { .. }
        | Condition::ColorsOfManaSpentToCastThisSpellOrMore(_)
        | Condition::PlayerGraveyardHasCardsAtLeast { .. }
        | Condition::YouControlCommander
        | Condition::Not(_)
        | Condition::And(_, _)
        | Condition::Or(_, _) => {
            unreachable!("handled before resolution match")
        }
    }
}
