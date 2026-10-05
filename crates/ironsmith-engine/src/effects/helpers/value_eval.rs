//! Shared interpreter for execution and continuous-effect values.
use super::*;
use crate::continuous::value_context::LayerValueContext;
mod context;
mod damage_history;
pub(crate) use context::EvaluationContext;
pub(crate) use context::NumericProperty;
use context::Reduction;
pub(crate) use damage_history::resolve_damage_history_for_comparison;

pub(crate) fn resolve_continuous(value: &Value, layer: LayerValueContext<'_, '_>) -> i32 {
    let context = EvaluationContext::continuous(layer);
    resolve(value, &context)
        .unwrap_or_else(|error| panic!("unsupported continuous-effect value {value:?}: {error:?}"))
}

pub(crate) fn resolve(
    value: &Value,
    context: &EvaluationContext<'_, '_>,
) -> Result<i32, ExecutionError> {
    let resolved = resolve_wide(value, context)?;
    i32::try_from(resolved).map_err(|_| {
        if matches!(
            value.unhinted(),
            Value::UnspentMana(_)
                | Value::EventValue(
                    crate::effect::EventValueSpec::Amount
                        | crate::effect::EventValueSpec::LifeAmount
                )
        ) {
            ExecutionError::ResourceLimitExceeded {
                resource: if matches!(value.unhinted(), Value::UnspentMana(_)) {
                    "unspent mana scalar"
                } else {
                    "effect scalar"
                },
                requested: resolved.max(0) as u128,
                maximum: i32::MAX as u128,
            }
        } else {
            ExecutionError::UnresolvableValue(
                "resolved value exceeds the signed characteristic or cost range".into(),
            )
        }
    })
}

pub(crate) fn resolve_wide(
    value: &Value,
    context: &EvaluationContext<'_, '_>,
) -> Result<i64, ExecutionError> {
    let game = context.game;
    match value {
        Value::SurfaceHinted { value, .. } => resolve_wide(value, context),
        Value::Fixed(n) => Ok(i64::from(*n)),
        Value::Add(left, right) => resolve_wide(left, context)?
            .checked_add(resolve_wide(right, context)?)
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "numeric addition exceeds the wide value range".into(),
                )
            }),
        Value::X => context.x(),
        Value::XTimes(multiplier) => {
            context
                .x()?
                .checked_mul(i64::from(*multiplier))
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "numeric multiplication exceeds the wide value range".into(),
                    )
                })
        }
        Value::Scaled(value, multiplier) => resolve_wide(value, context)?
            .checked_mul(i64::from(*multiplier))
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "numeric multiplication exceeds the wide value range".into(),
                )
            }),
        Value::DividedRoundedDown(value, divisor) => {
            if *divisor == 0 {
                return context.division_by_zero(value);
            }
            resolve_wide(value, context)?
                .checked_div_euclid(i64::from(*divisor))
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "numeric division exceeds the wide value range".into(),
                    )
                })
        }
        Value::Min(left, right) => Ok(i64::from(
            resolve_wide(left, context)?.min(resolve_wide(right, context)?),
        )),
        Value::DamageHistory(query) => {
            if let Some(execution) = context.execution() {
                damage_history::resolve_damage_history_for_comparison(game, query, execution)
            } else {
                damage_history::resolve_damage_history_for_comparison(
                    game,
                    query,
                    &ExecutionContext::new_default(context.source, context.controller),
                )
            }
        }
        Value::Count(filter) if filter_reads_source_devoured(filter) => {
            Ok(i64::from(count_source_devoured(filter, context)))
        }
        Value::Count(filter) => Ok(i64::from(context.count_objects(filter, true))),
        Value::PlayersWhoControl { players, filter } => Ok(i64::from(
            context
                .matching_player_ids(players)
                .into_iter()
                .filter(|player| context.controlled_object_count(filter, *player) > 0)
                .count() as i64,
        )),
        Value::PlayersWhoControlMoreThanYou { players, filter } => {
            let yours = context.controlled_object_count(filter, context.controller);
            Ok(i64::from(
                context
                    .matching_player_ids(players)
                    .into_iter()
                    .filter(|player| context.controlled_object_count(filter, *player) > yours)
                    .count() as i64,
            ))
        }
        Value::PlayersWhoControlAtLeastMoreThanYou {
            players,
            filter,
            minimum_difference,
        } => {
            let yours = context.controlled_object_count(filter, context.controller);
            Ok(i64::from(
                context
                    .matching_player_ids(players)
                    .into_iter()
                    .filter(|player| {
                        context
                            .controlled_object_count(filter, *player)
                            .saturating_sub(yours)
                            >= *minimum_difference as usize
                    })
                    .count() as i64,
            ))
        }
        Value::CountScaled(filter, multiplier) => Ok(i64::from(
            i64::from(context.count_objects(filter, false)) * i64::from(*multiplier),
        )),
        Value::GreatestCount(filter) => {
            Ok(i64::from(context.greatest_per_controller(filter, false)))
        }
        Value::GreatestSharedCreatureTypeCount(filter) => {
            Ok(i64::from(context.greatest_per_controller(filter, true)))
        }
        Value::GreatestSharedNameCount(filter) => {
            Ok(i64::from(context.greatest_shared_name_count(filter)))
        }
        Value::TotalPower(filter) => Ok(i64::from(context.aggregate(
            filter,
            NumericProperty::Power,
            Reduction::Sum,
        ))),
        Value::TotalToughness(filter) => Ok(i64::from(context.aggregate(
            filter,
            NumericProperty::Toughness,
            Reduction::Sum,
        ))),
        Value::TotalManaValue(filter) => Ok(i64::from(context.aggregate(
            filter,
            NumericProperty::ManaValue,
            Reduction::Sum,
        ))),
        Value::AnnouncedTargetTotal(metric) => {
            let Some(ctx) = context.execution() else {
                return Ok(i64::from(0));
            };
            let targets = ctx.announced_targets.as_deref().unwrap_or(&ctx.targets);
            let ids: std::collections::HashSet<_> = targets
                .iter()
                .filter_map(|target| match target {
                    ResolvedTarget::Object(id) => Some(*id),
                    ResolvedTarget::Player(_) => None,
                })
                .collect();
            Ok(i64::from(crate::targeting::aggregate_object_set_value(
                game, ids, *metric,
            )))
        }
        Value::GreatestPower(filter) => Ok(i64::from(context.aggregate(
            filter,
            NumericProperty::Power,
            Reduction::Max,
        ))),
        Value::GreatestToughness(filter) => Ok(i64::from(context.aggregate(
            filter,
            NumericProperty::Toughness,
            Reduction::Max,
        ))),
        Value::GreatestManaValue(filter) => {
            if filter.cast_this_turn && filter.zone == Some(Zone::Stack) {
                let filter_ctx = context.filter_context(game);
                return Ok(i64::from(
                    game.turn_store
                        .turn_history
                        .spell_cast_snapshot_history()
                        .iter()
                        .filter(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
                        .map(crate::filter::snapshot_mana_value_for_filter)
                        .max()
                        .unwrap_or(0),
                ));
            }
            Ok(i64::from(context.aggregate(
                filter,
                NumericProperty::ManaValue,
                Reduction::Max,
            )))
        }
        Value::LeastPower(filter) => Ok(i64::from(context.aggregate(
            filter,
            NumericProperty::Power,
            Reduction::Min,
        ))),
        Value::LeastToughness(filter) => Ok(i64::from(context.aggregate(
            filter,
            NumericProperty::Toughness,
            Reduction::Min,
        ))),
        Value::LeastManaValue(filter) => Ok(i64::from(context.aggregate(
            filter,
            NumericProperty::ManaValue,
            Reduction::Min,
        ))),
        Value::BasicLandTypesAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_current_subtypes(filter, |subtypes| {
                for &subtype in subtypes {
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
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::CreatureTypesAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                for subtype in object.subtypes(game, true) {
                    if subtype.is_creature_type() {
                        seen.insert(subtype);
                    }
                }
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::CardTypesAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.extend(object.card_types(game));
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::StaticAbilitiesAmong { filter, abilities } => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                for ability in abilities {
                    if object.has_ability(game, *ability) {
                        seen.insert(*ability);
                    }
                }
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::ColorsAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                for color in [
                    crate::color::Color::White,
                    crate::color::Color::Blue,
                    crate::color::Color::Black,
                    crate::color::Color::Red,
                    crate::color::Color::Green,
                ] {
                    if object.colors().contains(color) {
                        seen.insert(color);
                    }
                }
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::ColorPairsAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                let colors = object.colors();
                if colors.count() == 2 {
                    seen.insert(colors);
                }
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::DistinctCounterTypesAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.extend(object.counters().keys().copied());
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::DistinctNames(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.insert(object.name().to_string());
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::DistinctManaValues(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.insert(object.filter_mana_value());
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::UnlockedDoorsAmong(filter) => {
            let mut doors = 0i64;
            context.visit_property_objects(filter, |object| {
                let (context::PropertyObject::Live(object)
                | context::PropertyObject::LayerBaseline(object)) = object
                else {
                    return;
                };
                doors += i64::from(crate::effects::helpers::room_unlocked_door_count(
                    game, object,
                ));
            });
            Ok(i64::from(doors))
        }
        Value::DistinctPowers(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                if let Some(power) = object.power(game) {
                    seen.insert(power);
                }
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::TurnHistoryCount(query) => {
            Ok(i64::from(crate::turn_history::resolve_turn_history_count(
                game,
                query,
                &context.filter_context(game),
                context
                    .execution()
                    .and_then(|ctx| ctx.triggering_event.as_ref()),
            )))
        }
        Value::CreaturesDiedThisTurn => Ok(i64::from(
            game.turn_store
                .turn_history
                .total_creatures_died_this_turn() as i64,
        )),
        Value::CreaturesDiedThisTurnControlledBy(player_filter) => {
            let filter_ctx = context.filter_context(game);
            let mut total = 0i64;
            for player in game.players.iter().filter(|p| p.is_in_game()) {
                if !player_filter.matches_player(player.id, &filter_ctx) {
                    continue;
                }
                total += game
                    .turn_store
                    .turn_history
                    .creatures_died_under_controller(player.id) as i64;
            }
            Ok(i64::from(total))
        }
        Value::PlayersBeingAttacked => Ok(i64::from(
            game.combat
                .as_ref()
                .map(crate::combat_state::defending_players)
                .map(|players| players.len() as i64)
                .unwrap_or(0),
        )),
        Value::CountPlayers(player_filter) => Ok(i64::from(
            context.matching_player_ids(player_filter).len() as i64,
        )),
        Value::CountPlayersWithPoisonCountersAtLeast(player_filter, minimum) => Ok(i64::from(
            context
                .matching_player_ids(player_filter)
                .into_iter()
                .filter(|id| {
                    game.player(*id)
                        .is_some_and(|player| player.poison_counters >= *minimum)
                })
                .count() as i64,
        )),
        Value::CountPlayersWithCardsInHandAtLeast(player_filter, minimum) => Ok(i64::from(
            context
                .matching_player_ids(player_filter)
                .into_iter()
                .filter(|id| {
                    game.player(*id)
                        .is_some_and(|player| player.hand.len() >= *minimum as usize)
                })
                .count() as i64,
        )),
        Value::CountPlayersWithCardsInGraveyardAtLeast(player_filter, minimum) => Ok(i64::from(
            context
                .matching_player_ids(player_filter)
                .into_iter()
                .filter(|id| {
                    game.player(*id)
                        .is_some_and(|player| player.graveyard.len() >= *minimum as usize)
                })
                .count() as i64,
        )),
        Value::PartySize(player_filter) => {
            if let Some(ctx) = context.execution() {
                {
                    let player_id = resolve_player_filter(game, player_filter, ctx)?;
                    Ok(i64::from(crate::party::party_size(game, player_id)))
                }
            } else {
                Ok(i64::from(context.layer().party_size(value, player_filter)))
            }
        }
        Value::SourcePower => context.source_number(NumericProperty::Power).map(i64::from),
        Value::SourceToughness => context
            .source_number(NumericProperty::Toughness)
            .map(i64::from),
        Value::PowerOf(target_spec) => context
            .object_number(target_spec, NumericProperty::Power)
            .map(i64::from),
        Value::BasePowerOf(target_spec) => context
            .object_number(target_spec, NumericProperty::BasePower)
            .map(i64::from),
        Value::ToughnessOf(target_spec) => context
            .object_number(target_spec, NumericProperty::Toughness)
            .map(i64::from),
        Value::KicksPaidOf(target_spec) => context
            .object_number(target_spec, NumericProperty::KickerCount)
            .map(i64::from),
        Value::ManaSpentToCast(target_spec) => context
            .object_number(target_spec, NumericProperty::ManaSpent)
            .map(i64::from),
        Value::ManaValueOf(target_spec) => context
            .object_number(target_spec, NumericProperty::ManaValue)
            .map(i64::from),
        Value::ColorsOf(target_spec) => context
            .object_number(target_spec, NumericProperty::ColorCount)
            .map(i64::from),
        Value::ManaSymbolsInManaCostOf {
            spec: target_spec,
            color,
        } => {
            if let Some(ctx) = context.execution() {
                {
                    let count_symbols = |cost: &crate::mana::ManaCost| {
                        let symbol = crate::mana::ManaSymbol::from_color(*color);
                        cost.pips()
                            .iter()
                            .filter(|pip| pip.contains(&symbol))
                            .count() as i64
                    };

                    if matches!(target_spec.base(), ChooseSpec::All(_)) {
                        return Ok(i64::from(
                            resolve_objects_from_spec(game, target_spec, ctx)?
                                .into_iter()
                                .filter_map(|id| game.object(id))
                                .filter_map(|object| object.mana_cost.as_deref())
                                .map(count_symbols)
                                .sum::<i64>(),
                        ));
                    }

                    let target_id =
                        resolve_primary_object_from_value_spec(game, target_spec.as_ref(), ctx)?;
                    let tagged_snapshot = if let ChooseSpec::Tagged(tag) = target_spec.base() {
                        ctx.get_tagged(tag)
                    } else {
                        None
                    };

                    if matches!(target_spec.base(), ChooseSpec::Source)
                        && let Some(snapshot) = source_lki_for_moved_current_object(game, ctx)
                    {
                        snapshot
                            .mana_cost
                            .as_ref()
                            .map(count_symbols)
                            .ok_or_else(|| {
                                ExecutionError::UnresolvableValue(
                                    "Target had no printed mana cost".to_string(),
                                )
                            })
                    } else if let Some(snapshot) = tagged_snapshot
                        && game
                            .object(snapshot.object_id)
                            .is_none_or(|object| object.zone != snapshot.zone)
                    {
                        latest_tagged_lki_snapshot(game, snapshot)
                            .unwrap_or(snapshot)
                            .mana_cost
                            .as_ref()
                            .map(count_symbols)
                            .ok_or_else(|| {
                                ExecutionError::UnresolvableValue(
                                    "Target had no printed mana cost".to_string(),
                                )
                            })
                    } else if let Some(obj) = game.object(target_id) {
                        obj.mana_cost.as_deref().map(count_symbols).ok_or_else(|| {
                            ExecutionError::UnresolvableValue(
                                "Target has no printed mana cost".to_string(),
                            )
                        })
                    } else if let Some(snapshot) = tagged_snapshot {
                        snapshot
                            .mana_cost
                            .as_ref()
                            .map(count_symbols)
                            .ok_or_else(|| {
                                ExecutionError::UnresolvableValue(
                                    "Target had no printed mana cost".to_string(),
                                )
                            })
                    } else if let Some(snapshot) = object_lki_snapshot(ctx, target_id) {
                        snapshot
                            .mana_cost
                            .as_ref()
                            .map(count_symbols)
                            .ok_or_else(|| {
                                ExecutionError::UnresolvableValue(
                                    "Target had no printed mana cost".to_string(),
                                )
                            })
                    } else {
                        Err(ExecutionError::ObjectNotFound(target_id))
                    }
                }
            } else {
                Ok(i64::from(context.layer().mana_symbols_in_mana_cost_of(
                    value,
                    target_spec,
                    color,
                )))
            }
        }
        Value::NameStickerCharacterCountOnSource { character, .. } => Ok(i64::from(
            game.name_sticker_character_count_on_object(context.source, *character) as i64,
        )),
        Value::MaximumLifeTotal(players) => Ok(context
            .aggregate_player_ids(value, players)?
            .into_iter()
            .filter_map(|id| {
                game.player(id)
                    .filter(|player| player.is_in_game())
                    .map(|player| i64::from(player.life))
            })
            .max()
            .unwrap_or(0)),
        Value::CountPlayersBelowHalfStartingLifeTotal(players) => Ok(context
            .aggregate_player_ids(value, players)?
            .into_iter()
            .filter(|id| {
                game.player(*id).is_some_and(|player| {
                    player.is_in_game()
                        && 2 * i64::from(player.life) < i64::from(player.starting_life)
                })
            })
            .count() as i64),
        Value::LifeTotal(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.life))
        }
        Value::LifeTotalAsTurnBegan(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            let history = &game.turn_store.turn_history;
            Ok(i64::from(
                i64::from(player.life) + history.total_life_lost_for_players(&[player.id]) as i64
                    - history.total_life_gained_for_players(&[player.id]) as i64,
            ))
        }
        Value::LifeTotalDifference(player_spec) => {
            let players = context.player_ids(value, player_spec)?;
            if players.len() < 2 {
                return context.unavailable(
                    value,
                    "LifeTotalDifference requires at least two players",
                    "life-total difference requires at least two matching players",
                );
            }
            let mut minimum = i32::MAX;
            let mut maximum = i32::MIN;
            for id in players {
                let life = game
                    .player(id)
                    .ok_or(ExecutionError::PlayerNotFound(id))?
                    .life;
                minimum = minimum.min(life);
                maximum = maximum.max(life);
            }
            Ok(i64::from(maximum - minimum))
        }
        Value::Speed(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.speed.unwrap_or(0) as i64))
        }
        Value::StartingLifeTotal(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.starting_life))
        }
        Value::HalfLifeTotalRoundedUp(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(((i64::from(player.life) + 1).div_euclid(2)))
        }
        Value::HalfLifeTotalRoundedDown(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.life.div_euclid(2)))
        }
        Value::HalfStartingLifeTotalRoundedUp(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(((i64::from(player.starting_life) + 1).div_euclid(2)))
        }
        Value::HalfStartingLifeTotalRoundedDown(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.starting_life.div_euclid(2)))
        }
        Value::CardsInHand(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.hand.len() as i64))
        }
        Value::CardsInLibrary(player_spec) => {
            let players = context.library_player_ids(value, player_spec)?;
            players
                .into_iter()
                .map(|id| {
                    game.player(id)
                        .map(|player| player.library.len() as i64)
                        .ok_or(ExecutionError::PlayerNotFound(id))
                })
                .sum::<Result<i64, ExecutionError>>()
        }
        Value::DevotionToChosenColor(player_spec) => {
            let Some(chosen) = game.chosen_color(context.source) else {
                return context.unavailable(
                    value,
                    "DevotionToChosenColor requires a previously chosen color",
                    "source has no chosen color",
                );
            };
            let players = context.player_ids(value, player_spec)?;
            Ok(i64::from(
                players
                    .into_iter()
                    .map(|id| game.devotion_to_color(id, chosen) as i64)
                    .sum::<i64>(),
            ))
        }
        Value::LifeGainedThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_life_gained_for_players(&player_ids);
            Ok(i64::from(total as i64))
        }
        Value::LifeLostThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_life_lost_for_players(&player_ids);
            Ok(i64::from(total as i64))
        }
        Value::CardsDiscardedThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_cards_discarded_for_players(&player_ids);
            Ok(i64::from(total as i64))
        }
        Value::AttractionsVisitedThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            Ok(i64::from(
                game.turn_store
                    .turn_history
                    .total_attractions_visited_for_players(&player_ids) as i64,
            ))
        }
        Value::DamageDealtToPlayersThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_damage_to_players(&player_ids);
            Ok(i64::from(total as i64))
        }
        Value::NoncombatDamageDealtToPlayersThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_noncombat_damage_to_players(&player_ids);
            Ok(i64::from(total as i64))
        }
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, colors } => {
            let player_ids = context.player_ids(value, player)?;
            let total = game
                .turn_store
                .turn_history
                .total_noncombat_damage_dealt_by_sources_controlled_by(&player_ids, *colors);
            Ok(i64::from(total as i64))
        }
        Value::MaxCardsInHand(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let mut max_count: Option<i64> = None;
            for pid in player_ids {
                let player = game
                    .player(pid)
                    .ok_or(ExecutionError::PlayerNotFound(pid))?;
                let count = player.hand.len() as i64;
                max_count = Some(max_count.map_or(count, |prev| prev.max(count)));
            }
            Ok(i64::from(max_count.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "MaxCardsInHand requires a matching player".to_string(),
                )
            })?))
        }
        Value::MaxCardsDrawnThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            if player_ids.is_empty() {
                return Err(ExecutionError::UnresolvableValue(
                    "MaxCardsDrawnThisTurn requires a matching player".to_string(),
                ));
            }
            Ok(i64::from(
                game.turn_store
                    .turn_history
                    .max_cards_drawn_for_players(&player_ids) as i64,
            ))
        }
        Value::MaxDiceRolledThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            if player_ids.is_empty() {
                return Err(ExecutionError::UnresolvableValue(
                    "MaxDiceRolledThisTurn requires a matching player".to_string(),
                ));
            }
            Ok(i64::from(
                game.turn_store
                    .turn_history
                    .max_die_rolls_for_players(&player_ids) as i64,
            ))
        }
        Value::LandsEnteredBattlefieldThisTurn(player_spec) => {
            let player_ids = context.counter_player_ids(value, player_spec)?;
            Ok(i64::from(
                game.turn_store
                    .turn_history
                    .total_lands_entered_for_players(&player_ids) as i64,
            ))
        }
        Value::CardsInGraveyard(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let mut max_count: Option<i64> = None;
            for player_id in player_ids {
                let player = game
                    .player(player_id)
                    .ok_or(ExecutionError::PlayerNotFound(player_id))?;
                let count = player.graveyard.len() as i64;
                max_count = Some(max_count.map_or(count, |prev| prev.max(count)));
            }
            Ok(i64::from(max_count.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "CardsInGraveyard requires a matching player".to_string(),
                )
            })?))
        }
        Value::SpellsCastThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            Ok(i64::from(
                game.turn_store
                    .turn_history
                    .total_spells_cast_for_players(&player_ids) as i64,
            ))
        }
        Value::SpellsCastBeforeThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            // CR 702.40a: count only spells cast *before* this spell, so spells
            // cast in response to the storm trigger don't add copies. The
            // boundary is the triggering cast when there is one, else the source.
            let boundary_spell = context
                .execution()
                .and_then(|ctx| ctx.triggering_event.as_ref())
                .and_then(|event| event.downcast::<crate::events::SpellCastEvent>())
                .map(|cast| cast.spell)
                .unwrap_or(context.source);
            let history = &game.turn_store.turn_history;
            if let Some(count) =
                history.spells_cast_before_spell_for_players(boundary_spell, &player_ids)
            {
                return Ok(i64::from(count as i64));
            }
            let count = history.total_spells_cast_for_players(&player_ids) as i64;
            Ok(i64::from((count - 1).max(0)))
        }
        Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let player_ids = context.player_ids(value, player)?;
            let filter_ctx = context.filter_context(game);
            let mut count: i64 = 0;
            for snapshot in game.turn_store.turn_history.spell_cast_snapshot_history() {
                if *exclude_source && snapshot.object_id == context.source {
                    continue;
                }
                if !player_ids.contains(&snapshot.controller) {
                    continue;
                }
                if filter.matches_snapshot(&snapshot, &filter_ctx, game) {
                    count = context.add_spell_metric(count, 1)?;
                }
            }
            Ok(i64::from(count))
        }
        Value::TotalManaValueOfSpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let player_ids = context.player_ids(value, player)?;
            let filter_ctx = context.filter_context(game);
            let mut total: i64 = 0;
            for snapshot in game.turn_store.turn_history.spell_cast_snapshot_history() {
                if *exclude_source && snapshot.object_id == context.source {
                    continue;
                }
                if !player_ids.contains(&snapshot.controller) {
                    continue;
                }
                if filter.matches_snapshot(&snapshot, &filter_ctx, game) {
                    total = context.add_spell_metric(total, snapshot.mana_value() as i64)?;
                }
            }
            Ok(i64::from(total))
        }
        Value::CommanderCastCount(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            Ok(i64::from(
                player_ids
                    .into_iter()
                    .map(|player_id| game.commander_cast_count_for_player(player_id) as i64)
                    .sum::<i64>(),
            ))
        }
        Value::CommanderColorIdentityColors(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            Ok(i64::from(
                player_ids
                    .into_iter()
                    .map(|player_id| game.get_commander_color_identity(player_id).count() as i64)
                    .sum::<i64>(),
            ))
        }
        Value::ThisAbilityResolvedThisTurnCount => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            {
                if let Some(ability_index) = ctx.ability_index {
                    return Ok(i64::from(
                        game.activated_ability_resolution_count_this_turn(ctx.source, ability_index)
                            as i64,
                    ));
                }
                if let Some(trigger_identity) = ctx.trigger_identity {
                    return Ok(i64::from(
                        game.triggered_ability_resolution_count_this_turn(
                            ctx.source,
                            trigger_identity,
                        ) as i64,
                    ));
                }
                Err(ExecutionError::UnresolvableValue(
                    "this ability resolution count requires a resolving ability context"
                        .to_string(),
                ))
            }
        }
        Value::SourceRegeneratedThisTurnCount => Ok(i64::from(
            game.regenerated_this_turn_count(context.source) as i64,
        )),
        Value::SourceMutationCount => Ok(i64::from(game.mutation_count(context.source) as i64)),
        Value::SourceDevouredCreatureCount => {
            Ok(i64::from(game.devoured_count(context.source) as i64))
        }
        Value::DamageDealtThisTurnByTaggedSpellCast(tag) => {
            let id = context.tagged_spell_id(value, tag)?;
            Ok(i64::from(
                game.turn_store
                    .turn_history
                    .damage_dealt_by_spell_this_turn(game.provenance_graph(), id)
                    as i64,
            ))
        }
        Value::CardTypesInGraveyard(player_spec) => {
            let player_ids = context.counter_player_ids(value, player_spec)?;
            let mut types = HashSet::new();
            for player_id in player_ids {
                let player = game
                    .player(player_id)
                    .ok_or(ExecutionError::PlayerNotFound(player_id))?;
                for &card_id in &player.graveyard {
                    let Some(obj) = game.object(card_id) else {
                        continue;
                    };
                    // CR 709.4: a split card has both halves' card types.
                    for card_type in obj.zone_card_types() {
                        types.insert(*card_type);
                    }
                }
            }

            Ok(i64::from(types.len() as i64))
        }
        Value::Devotion { player, color } => {
            let player_ids = context.player_ids(value, player)?;
            let devotion: i64 = player_ids
                .iter()
                .map(|pid| game.devotion_to_color(*pid, *color) as i64)
                .sum::<i64>();
            Ok(i64::from(devotion as i64))
        }
        Value::ManaSpentToCastThisSpell => {
            let spent = game
                .object(context.source)
                .map(|source| &source.mana_spent_to_cast)
                .or_else(|| {
                    context
                        .execution()
                        .and_then(|ctx| ctx.source_snapshot.as_ref())
                        .map(|snapshot| &snapshot.mana_spent_to_cast)
                });
            Ok(i64::from(spent.map_or(0, |mana| mana.total() as i64)))
        }
        Value::ManaSpentOnX(color) => {
            // A missing legacy receipt is unknown. Do not reinterpret an
            // imported paid spell as an unspent copy, or replace exact live
            // evidence with an older source snapshot.
            let allocation = if let Some(source) = game.object(context.source) {
                source.mana_spent_on_x
            } else {
                context
                    .execution()
                    .and_then(|ctx| ctx.source_snapshot.as_ref())
                    .and_then(|snapshot| snapshot.mana_spent_on_x)
            }
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "actual mana allocated to X was not retained".into(),
                )
            })?;
            let amount = allocation.of_color(*color);
            i64::try_from(amount).map_err(|_| ExecutionError::ResourceLimitExceeded {
                resource: "actual mana spent on X",
                requested: u128::from(amount),
                maximum: i64::MAX as u128,
            })
        }
        Value::ManaSymbolSpentToCastThisSpell { symbol, .. } => {
            let Some(source_obj) = game.object(context.source) else {
                return Ok(i64::from(0));
            };
            Ok(i64::from(
                source_obj.mana_spent_to_cast.amount(*symbol) as i64
            ))
        }
        Value::ManaFromSourceSpentToCastThisSpell {
            source_filter,
            reference,
            ..
        } => {
            if let Some(ctx) = context.execution() {
                {
                    let tag = ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG;
                    let snapshots = ctx.get_tagged_all(tag).map(Vec::as_slice).or_else(|| {
                        if *reference == ironsmith_core::ManaSpentCastReferenceSurface::ThisAbility
                        {
                            return None;
                        }
                        game.object(ctx.source)
                            .and_then(|source_obj| source_obj.cast_tagged_objects.get(tag))
                            .map(Vec::as_slice)
                    });
                    let Some(snapshots) = snapshots else {
                        return Ok(i64::from(0));
                    };
                    let filter_ctx = ctx.filter_context(game);
                    Ok(i64::from(
                        snapshots
                            .iter()
                            .filter(|snapshot| {
                                source_filter.matches_snapshot(snapshot, &filter_ctx, game)
                            })
                            .count() as i64,
                    ))
                }
            } else {
                Ok(i64::from(
                    context
                        .layer()
                        .mana_from_source_spent_to_cast_this_spell(value, source_filter),
                ))
            }
        }
        Value::CasterManaSpentToCastTriggeringObject => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            let spent = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::SpellCastEvent>())
                .and_then(|cast| match &cast.snapshot {
                    Some(snapshot) => snapshot.caster_mana_spent_to_cast,
                    None => game
                        .object(cast.spell)
                        .and_then(|object| object.caster_mana_spent_to_cast),
                });
            let spent = spent.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "triggering spell has no captured caster payment evidence".into(),
                )
            })?;
            i64::try_from(spent).map_err(|_| {
                ExecutionError::UnresolvableValue(
                    "triggering spell caster payment exceeds the integer value range".into(),
                )
            })
        }
        Value::ManaSpentToCastTriggeringObject => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            {
                let Some(triggering_event) = &ctx.triggering_event else {
                    return Ok(i64::from(0));
                };
                let Some(spell_cast) = triggering_event.downcast::<crate::events::SpellCastEvent>()
                else {
                    return Ok(i64::from(0));
                };
                if let Some(snapshot) = spell_cast.snapshot.as_ref() {
                    return Ok(i64::from(snapshot.mana_spent_to_cast.total() as i64));
                }
                Ok(i64::from(
                    game.object(spell_cast.spell)
                        .map(|object| object.mana_spent_to_cast.total() as i64)
                        .unwrap_or(0),
                ))
            }
        }
        Value::UnspentMana(player) => {
            let player_ids = context.player_ids(value, player)?;
            let total = player_ids
                .iter()
                .filter_map(|player_id| game.player(*player_id))
                .map(|player| player.mana_pool.total_wide() as i64)
                .sum::<i64>();
            Ok(i64::from(total))
        }
        Value::ColorsOfManaSpentToCastThisSpell => {
            let Some(source_obj) = game.object(context.source) else {
                return Ok(i64::from(0));
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
            Ok(i64::from(distinct_colors as i64))
        }
        Value::MagicGamesLostToOpponentsSinceLastWin => {
            let _ctx = context.require_execution(value, RESOLUTION_ONLY);
            Ok(i64::from(0))
        }
        Value::DraftRemovedCardCount { card_name } => Ok(i64::from(
            game.draft_removed_card_count(context.controller, card_name)
                .try_into()
                .unwrap_or(i32::MAX),
        )),
        Value::DraftNotedHighestNumber { card_name } => Ok(i64::from(
            game.draft_noted_highest_number(context.controller, card_name)
                .try_into()
                .unwrap_or(i32::MAX),
        )),
        Value::LastNotedLifeTotal => game
            .noted_life_total_for_source(context.source)
            .or_else(|| {
                // Live and explicitly re-noted values above are authoritative.
                // An absent source uses its exact departure receipt, never the
                // new incarnation of the same stable card. Pending stack LKI
                // also preserves the receipt across turn-history cleanup.
                if game.object(context.source).is_some() {
                    return None;
                }
                let ctx = context.execution()?;
                game.turn_store
                    .turn_history
                    .source_departure_snapshot(context.source)
                    .cloned()
                    .or_else(|| {
                        ctx.source_snapshot
                            .as_ref()
                            .filter(|snapshot| snapshot.object_id == context.source)
                            .cloned()
                    })
                    .and_then(|snapshot| snapshot.noted_life_total)
            })
            .map(|n| Ok(i64::from(n)))
            .unwrap_or_else(|| {
                context.unavailable(
                    value,
                    "last noted life total is not available for this source",
                    "source has no noted life total",
                )
            }),
        Value::EffectValue(effect_id) => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            {
                // "That many" of an instruction that never ran is zero.
                Ok(i64::from(ctx.get_outcome(*effect_id).map_or(
                    0,
                    |outcome| {
                        outcome.as_count().unwrap_or_else(|| {
                            outcome
                                .explicit_objects()
                                .map_or(0, |objects| objects.len() as i64)
                        })
                    },
                )))
            }
        }
        Value::EffectValueOffset(effect_id, offset) => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            {
                Ok(i64::from(
                    ctx.get_outcome(*effect_id).map_or(0, |outcome| {
                        outcome.as_count().unwrap_or_else(|| {
                            outcome
                                .explicit_objects()
                                .map_or(0, |objects| objects.len() as i64)
                        })
                    }) + i64::from(*offset),
                ))
            }
        }
        Value::EffectMetric {
            effect_id,
            source,
            metric,
        } => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            resolve_effect_metric(game, ctx, *effect_id, *source, *metric)
        }
        Value::EffectMetricOffset {
            effect_id,
            source,
            metric,
            offset,
        } => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            Ok(i64::from(
                resolve_effect_metric(game, ctx, *effect_id, *source, *metric)?
                    + i64::from(*offset),
            ))
        }
        Value::PriorEffectMetric { effect_id, query } => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            resolve_prior_effect_metric(game, ctx, *effect_id, query)
        }
        Value::PendingEffectMetric { .. } => {
            let _ctx = context.require_execution(value, RESOLUTION_ONLY);
            Err(ExecutionError::UnresolvableValue(
                "pending effect metric was not bound to a prior effect".to_string(),
            ))
        }
        Value::PendingEffectMetricOffset { .. } => {
            let _ctx = context.require_execution(value, RESOLUTION_ONLY);
            Err(ExecutionError::UnresolvableValue(
                "pending effect metric was not bound to a prior effect".to_string(),
            ))
        }
        Value::PendingComparisonLeft
        | Value::PendingComparisonRight
        | Value::PendingComparisonDifference => Err(ExecutionError::UnresolvableValue(
            "comparison reference was not bound by the compiler".into(),
        )),
        Value::PendingPriorEffectMetric(_) => {
            let _ctx = context.require_execution(value, RESOLUTION_ONLY);
            Err(ExecutionError::UnresolvableValue(
                "pending effect metric was not bound to a prior effect".to_string(),
            ))
        }
        Value::HalfRoundedDown(inner) => Ok(i64::from(resolve_wide(inner, context)?.div_euclid(2))),
        Value::EventValue(spec) => resolve_event_value(
            game,
            context.require_execution(value, RESOLUTION_ONLY),
            spec,
        ),
        Value::EventValueOffset(spec, offset) => Ok(i64::from(
            resolve_event_value(
                game,
                context.require_execution(value, RESOLUTION_ONLY),
                spec,
            )? + i64::from(*offset),
        )),
        Value::WasKicked => {
            // Check if kicker or multikicker was paid
            // First check ctx, then fall back to source object (for ETB triggers)
            let paid = context.optional_costs_paid(value);
            Ok(i64::from(if paid.was_kicked() { 1 } else { 0 }))
        }
        Value::WasBoughtBack => {
            // Check if buyback was paid
            let paid = context.optional_costs_paid(value);
            Ok(i64::from(if paid.was_bought_back() { 1 } else { 0 }))
        }
        Value::WasEntwined => {
            // Check if entwine was paid
            let paid = context.optional_costs_paid(value);
            Ok(i64::from(if paid.was_entwined() { 1 } else { 0 }))
        }
        Value::WasPaid(index) => {
            // Check if the optional cost at the given index was paid
            let paid = context.optional_costs_paid(value);
            Ok(i64::from(if paid.was_paid(*index) { 1 } else { 0 }))
        }
        Value::WasPaidLabel(label) => {
            // Check if the optional cost with the given label was paid
            let paid = context.optional_costs_paid(value);
            Ok(i64::from(if paid.was_paid_label(label.clone()) {
                1
            } else {
                0
            }))
        }
        Value::TimesPaid(index) => {
            // Get the number of times the optional cost was paid
            let paid = context.optional_costs_paid(value);
            Ok(i64::from(paid.times_paid(*index) as i64))
        }
        Value::TimesPaidLabel(label) => {
            // Get the number of times the optional cost with the label was paid
            let paid = context.optional_costs_paid(value);
            Ok(i64::from(paid.times_paid_label(label.clone()) as i64))
        }
        Value::KickCount => {
            // Get the number of times the kicker was paid
            let paid = context.optional_costs_paid(value);
            Ok(i64::from(paid.kick_count() as i64))
        }
        Value::PlayerCounters(player_spec, counter_type) => {
            let mut player_ids = context.counter_player_ids(value, player_spec)?;
            if matches!(counter_type, crate::object::CounterType::Poison)
                && game.two_headed_giant().is_some()
            {
                let mut seen_teams = HashSet::new();
                player_ids.retain(|player| {
                    game.team_index_for(*player)
                        .is_none_or(|team| seen_teams.insert(team))
                });
            }
            Ok(i64::from(
                player_ids
                    .into_iter()
                    .filter_map(|player_id| game.player(player_id))
                    .map(|player| player.counter_count(*counter_type) as i64)
                    .sum::<i64>(),
            ))
        }
        // Only meaningful while an enclosing object filter binds its
        // candidate (see `Comparison::bind_filter_candidate_counters`).
        Value::CountersOnFilterCandidate(_) => Ok(i64::from(0)),
        Value::CountersOnSource(counter_type) => {
            if let Some(ctx) = context.execution() {
                let amount = if let Some(snapshot) = source_lki_for_moved_current_object(game, ctx)
                {
                    snapshot.counters.get(counter_type).copied().unwrap_or(0)
                } else if let Some(source) = game.object(ctx.source) {
                    source.counters.get(counter_type).copied().unwrap_or(0)
                } else if let Some(snapshot) = &ctx.source_snapshot {
                    snapshot.counters.get(counter_type).copied().unwrap_or(0)
                } else {
                    return Err(ExecutionError::UnresolvableValue(
                        "counter source has neither current nor retained characteristics".into(),
                    ));
                };
                Ok(i64::from(amount))
            } else {
                Ok(i64::from(
                    context.layer().counters_on_source(value, counter_type),
                ))
            }
        }
        Value::CountersOn(spec, counter_type) => {
            if matches!(spec.base(), ChooseSpec::Source)
                && let Some(kind) = counter_type
            {
                return resolve_wide(&Value::CountersOnSource(*kind), context);
            }
            // Counters on players ("each counter among players and
            // permanents", Lumbering Megasloth, CR 122.1).
            if let ChooseSpec::EachPlayer(player_filter) = spec.base() {
                let player_ids = context.counter_player_ids(value, player_filter)?;
                return Ok(i64::from(
                    player_ids
                        .into_iter()
                        .filter_map(|player_id| game.player(player_id))
                        .map(|player| match counter_type {
                            Some(counter_type) => player.counter_count(*counter_type) as i64,
                            None => player
                                .counter_types_with_counters()
                                .into_iter()
                                .map(|counter_type| player.counter_count(counter_type) as i64)
                                .sum::<i64>(),
                        })
                        .sum::<i64>(),
                ));
            }
            if let Some(ctx) = context.execution() {
                {
                    if let Some(snapshots) = tagged_snapshots_for_choose_spec(ctx, spec) {
                        return Ok(i64::from(
                            snapshots
                                .iter()
                                .map(|snapshot| snapshot_counter_total(snapshot, counter_type))
                                .sum::<i64>(),
                        ));
                    }

                    if matches!(spec.base(), ChooseSpec::Source)
                        && let Some(snapshot) = source_lki_for_moved_current_object(game, ctx)
                            .or_else(|| {
                                ctx.source_snapshot
                                    .as_ref()
                                    .filter(|_| resolve_source_object_id(game, ctx).is_none())
                            })
                    {
                        let total = if let Some(counter_type) = counter_type {
                            snapshot.counters.get(counter_type).copied().unwrap_or(0) as i64
                        } else {
                            snapshot
                                .counters
                                .values()
                                .map(|count| *count as i64)
                                .sum::<i64>()
                        };
                        return Ok(i64::from(total));
                    }

                    let object_ids = resolve_objects_from_spec(game, spec, ctx)?;
                    let total = object_ids
                        .into_iter()
                        .map(|id| {
                            if matches!(spec.base(), ChooseSpec::Source)
                                && let Some(snapshot) =
                                    source_lki_for_moved_current_object(game, ctx)
                            {
                                if let Some(counter_type) = counter_type {
                                    snapshot.counters.get(counter_type).copied().unwrap_or(0) as i64
                                } else {
                                    snapshot
                                        .counters
                                        .values()
                                        .map(|count| *count as i64)
                                        .sum::<i64>()
                                }
                            } else if let Some(obj) = game.object(id) {
                                if let Some(counter_type) = counter_type {
                                    obj.counters.get(counter_type).copied().unwrap_or(0) as i64
                                } else {
                                    obj.counters
                                        .values()
                                        .map(|count| *count as i64)
                                        .sum::<i64>()
                                }
                            } else if let Some(snapshot) = object_lki_snapshot(ctx, id) {
                                if let Some(counter_type) = counter_type {
                                    snapshot.counters.get(counter_type).copied().unwrap_or(0) as i64
                                } else {
                                    snapshot
                                        .counters
                                        .values()
                                        .map(|count| *count as i64)
                                        .sum::<i64>()
                                }
                            } else {
                                0
                            }
                        })
                        .sum::<i64>();
                    Ok(i64::from(total))
                }
            } else {
                Ok(i64::from(context.layer().counters_on(
                    value,
                    spec,
                    counter_type,
                )))
            }
        }
        Value::TaggedCount => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            {
                // Get the count of tagged objects for the current controller
                // (set by ForEachControllerOfTaggedEffect during iteration)
                if let Some(outcome) = ctx.get_outcome(crate::effect::EffectId::TAGGED_COUNT) {
                    Ok(i64::from(outcome.count_or_zero()))
                } else {
                    Err(ExecutionError::UnresolvableValue(
                        "TaggedCount used outside ForEachControllerOfTagged loop".to_string(),
                    ))
                }
            }
        }
        Value::VoteCount(option) => {
            let ctx =
                context.require_execution(value, "vote totals require a resolving vote context");
            Ok(i64::from(
                ctx.vote_results
                    .get(&ctx.source)
                    .map(|result| result.count_for_option(option) as i64)
                    .unwrap_or(0),
            ))
        }
        Value::ObjectVoteCount(spec) => {
            let ctx =
                context.require_execution(value, "vote totals require a resolving vote context");
            let object_ids = resolve_objects_from_spec(game, spec, ctx)?;
            Ok(i64::from(
                ctx.vote_results
                    .get(&ctx.source)
                    .map(|result| {
                        object_ids
                            .iter()
                            .map(|id| result.object_counts.get(id).copied().unwrap_or(0) as i64)
                            .sum::<i64>()
                    })
                    .unwrap_or(0),
            ))
        }
        Value::PlayerVoteCount(filter) => {
            let ctx =
                context.require_execution(value, "vote totals require a resolving vote context");
            {
                let resolved_filter = resolve_player_filter(game, filter, ctx)?;
                Ok(i64::from(
                    ctx.vote_results
                        .get(&ctx.source)
                        .map(|result| {
                            result.count_for_player_filter(&crate::target::PlayerFilter::Specific(
                                resolved_filter,
                            )) as i64
                        })
                        .unwrap_or(0),
                ))
            }
        }
    }
}

const RESOLUTION_ONLY: &str =
    "value requires resolution, trigger, loop, or out-of-game context that layers do not retain";
fn resolve_event_value(
    game: &GameState,
    ctx: &ExecutionContext,
    spec: &EventValueSpec,
) -> Result<i64, ExecutionError> {
    match spec {
        EventValueSpec::LifeChange {
            gained,
            for_controller,
        } => {
            let event = ctx.triggering_event.as_ref().ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "life quantity requires its triggering event".into(),
                )
            })?;
            let (player, amount) = if *gained {
                let life = event.downcast::<LifeGainEvent>().ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "life-gain quantity requires a life-gain event".into(),
                    )
                })?;
                (life.player, life.amount)
            } else {
                let life = event.downcast::<LifeLossEvent>().ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "life-loss quantity requires a life-loss event".into(),
                    )
                })?;
                (life.player, life.amount)
            };
            if *for_controller && player != ctx.controller {
                return Err(ExecutionError::UnresolvableValue(
                    "life quantity names another participant".into(),
                ));
            }
            crate::events::damage::checked_damage_count(
                u128::from(amount),
                "life-change event scalar",
            )
            .map(i64::from)
        }
        EventValueSpec::DieBatchTotal | EventValueSpec::DieResultsAtLeast(_) => {
            let roll = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::other::DieRolledEvent>())
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "die batch value requires its completed roll event".into(),
                    )
                })?;
            let amount: u128 = match spec {
                EventValueSpec::DieBatchTotal => roll.numeric_batch_results().map(u128::from).sum(),
                EventValueSpec::DieResultsAtLeast(minimum) => roll
                    .numeric_batch_results()
                    .filter(|result| i64::from(*result) >= i64::from(*minimum))
                    .count() as u128,
                _ => unreachable!(),
            };
            i64::try_from(amount).map_err(|_| ExecutionError::ResourceLimitExceeded {
                resource: "die-roll batch quantity",
                requested: amount,
                maximum: i64::MAX as u128,
            })
        }
        EventValueSpec::DieResult => {
            let roll = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::other::DieRolledEvent>())
                .filter(|event| !event.is_planar)
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "EventValue(DieResult) requires a numeric die-roll triggering event"
                            .to_string(),
                    )
                })?;
            i64::try_from(roll.result).map_err(|_| {
                ExecutionError::UnresolvableValue(
                    "die-roll result exceeds the supported value range".to_string(),
                )
            })
        }
        EventValueSpec::Amount | EventValueSpec::LifeAmount => {
            if let Some(amount) = ctx.event_value_amount {
                return Ok(i64::from(amount));
            }
            let Some(triggering_event) = &ctx.triggering_event else {
                return Err(ExecutionError::UnresolvableValue(
                    "EventValue(Amount) requires a triggering event".to_string(),
                ));
            };
            if let Some(payment) = triggering_event.downcast::<crate::events::LifePaidEvent>() {
                return crate::events::damage::checked_damage_count(
                    u128::from(payment.amount),
                    "life payment event amount",
                )
                .map(i64::from);
            }
            if let Some(life_loss_event) = triggering_event.downcast::<LifeLossEvent>() {
                return Ok(i64::from(life_loss_event.amount as i64));
            }
            if let Some(life_gain_event) = triggering_event.downcast::<LifeGainEvent>() {
                return Ok(i64::from(life_gain_event.amount as i64));
            }
            if let Some(damage_event) = triggering_event.downcast::<DamageEvent>() {
                return Ok(i64::from(damage_event.amount as i64));
            }
            if let Some(prevented_event) =
                triggering_event.downcast::<crate::events::DamagePreventedEvent>()
            {
                return Ok(i64::from(prevented_event.amount as i64));
            }
            if let Some(placement) = triggering_event.downcast::<crate::events::PutCountersEvent>()
            {
                return i64::try_from(placement.count).map_err(|_| {
                    ExecutionError::UnresolvableValue(
                        "counter placement amount exceeds the supported value range".into(),
                    )
                });
            }
            if let Some(markers_event) = triggering_event.downcast::<MarkersChangedEvent>() {
                return Ok(i64::from(markers_event.amount as i64));
            }
            if let Some(counter_event) = triggering_event.downcast::<CounterPlacedEvent>() {
                return Ok(i64::from(counter_event.amount as i64));
            }
            if let Some(zone_change_event) = triggering_event.downcast::<ZoneChangeEvent>() {
                return Ok(i64::from(zone_change_event.count() as i64));
            }
            if let Some(keyword_action_event) = triggering_event.downcast::<KeywordActionEvent>() {
                return Ok(i64::from(keyword_action_event.amount as i64));
            }
            if let Some(created) = triggering_event.downcast::<crate::events::CreateTokensEvent>() {
                return i64::try_from(created.total_count()).map_err(|_| {
                    ExecutionError::UnresolvableValue(
                        "token creation amount exceeds the supported value range".into(),
                    )
                });
            }
            Err(ExecutionError::UnresolvableValue(
                "EventValue(Amount) requires a numeric triggering event".to_string(),
            ))
        }
        EventValueSpec::BlockersBeyondFirst { multiplier } => {
            let Some(triggering_event) = &ctx.triggering_event else {
                return Err(ExecutionError::UnresolvableValue(
                    "EventValue(BlockersBeyondFirst) requires a triggering event".to_string(),
                ));
            };
            if let Some(event) = triggering_event.downcast::<CreatureBecameBlockedEvent>() {
                // CR 702.23b: the rampage bonus is calculated when the ability
                // resolves, from the creatures blocking it at that time.
                let blocker_count = game
                    .combat
                    .as_ref()
                    .filter(|combat| crate::combat_state::is_attacking(combat, event.attacker))
                    .map(|combat| {
                        combat.blockers.get(&event.attacker).map_or(0, |blockers| {
                            blockers
                                .iter()
                                .filter(|blocker| game.object(**blocker).is_some())
                                .count() as u32
                        })
                    })
                    .unwrap_or(event.blocker_count);
                let beyond_first = blocker_count.saturating_sub(1) as i64;
                return Ok(i64::from(beyond_first * i64::from(*multiplier)));
            }
            Err(ExecutionError::UnresolvableValue(
                "EventValue(BlockersBeyondFirst) requires a creature-becomes-blocked event"
                    .to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests;

fn filter_reads_source_devoured(filter: &crate::filter::ObjectFilter) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == crate::tag::SOURCE_DEVOURED_TAG)
}

/// "the number of Goblins it devoured" (Voracious Dragon): count the
/// permanents the source sacrificed to its devour ability, as they last
/// existed on the battlefield (CR 702.82b).
fn count_source_devoured(
    filter: &crate::filter::ObjectFilter,
    context: &EvaluationContext<'_, '_>,
) -> i64 {
    use crate::filter::ObjectFilterExt as _;
    let game = context.game;
    let mut residual = filter.clone();
    residual
        .tagged_constraints
        .retain(|constraint| constraint.tag.as_str() != crate::tag::SOURCE_DEVOURED_TAG);
    residual.zone = None;
    let filter_ctx = context.filter_context(game);
    game.devoured_objects(context.source)
        .iter()
        .filter(|snapshot| residual.matches_snapshot(snapshot, &filter_ctx, game))
        .count() as i64
}
