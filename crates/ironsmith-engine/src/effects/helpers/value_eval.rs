//! Shared interpreter for execution and continuous-effect values.
use super::*;
use crate::continuous::value_context::LayerValueContext;
mod context;
pub(crate) use context::EvaluationContext;
pub(crate) use context::NumericProperty;
use context::Reduction;

pub(crate) fn resolve_continuous(value: &Value, layer: LayerValueContext<'_, '_>) -> i32 {
    let context = EvaluationContext::continuous(layer);
    resolve(value, &context)
        .unwrap_or_else(|error| panic!("unsupported continuous-effect value {value:?}: {error:?}"))
}

pub(crate) fn resolve(
    value: &Value,
    context: &EvaluationContext<'_, '_>,
) -> Result<i32, ExecutionError> {
    let game = context.game;
    match value {
        Value::SurfaceHinted { value, .. } => resolve(value, context),
        Value::Fixed(n) => Ok(*n),
        Value::Add(left, right) => Ok(resolve(left, context)? + resolve(right, context)?),
        Value::X => context.x(),
        Value::XTimes(multiplier) => Ok(context.x()? * *multiplier),
        Value::Scaled(value, multiplier) => Ok(resolve(value, context)? * *multiplier),
        Value::DividedRoundedDown(value, divisor) => {
            if *divisor == 0 {
                return context.division_by_zero(value);
            }
            Ok(resolve(value, context)?.div_euclid(*divisor))
        }
        Value::Min(left, right) => Ok(resolve(left, context)?.min(resolve(right, context)?)),
        Value::Count(filter) => Ok(context.count_objects(filter, true)),
        Value::PlayersWhoControlMoreThanYou { players, filter } => {
            let yours = context.controlled_object_count(filter, context.controller);
            Ok(context
                .matching_player_ids(players)
                .into_iter()
                .filter(|player| context.controlled_object_count(filter, *player) > yours)
                .count() as i32)
        }
        Value::PlayersWhoControlAtLeastMoreThanYou {
            players,
            filter,
            minimum_difference,
        } => {
            let yours = context.controlled_object_count(filter, context.controller);
            Ok(context
                .matching_player_ids(players)
                .into_iter()
                .filter(|player| {
                    context
                        .controlled_object_count(filter, *player)
                        .saturating_sub(yours)
                        >= *minimum_difference as usize
                })
                .count() as i32)
        }
        Value::CountScaled(filter, multiplier) => {
            Ok(context.count_objects(filter, false) * *multiplier)
        }
        Value::GreatestCount(filter) => Ok(context.greatest_per_controller(filter, false)),
        Value::GreatestSharedCreatureTypeCount(filter) => {
            Ok(context.greatest_per_controller(filter, true))
        }
        Value::TotalPower(filter) => {
            Ok(context.aggregate(filter, NumericProperty::Power, Reduction::Sum))
        }
        Value::TotalToughness(filter) => {
            Ok(context.aggregate(filter, NumericProperty::Toughness, Reduction::Sum))
        }
        Value::TotalManaValue(filter) => {
            Ok(context.aggregate(filter, NumericProperty::ManaValue, Reduction::Sum))
        }
        Value::GreatestPower(filter) => {
            Ok(context.aggregate(filter, NumericProperty::Power, Reduction::Max))
        }
        Value::GreatestToughness(filter) => {
            Ok(context.aggregate(filter, NumericProperty::Toughness, Reduction::Max))
        }
        Value::GreatestManaValue(filter) => {
            if filter.cast_this_turn && filter.zone == Some(Zone::Stack) {
                let filter_ctx = context.filter_context(game);
                return Ok(game
                    .turn_store
                    .turn_history
                    .spell_cast_snapshot_history()
                    .iter()
                    .filter(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
                    .map(crate::filter::snapshot_mana_value_for_filter)
                    .max()
                    .unwrap_or(0));
            }
            Ok(context.aggregate(filter, NumericProperty::ManaValue, Reduction::Max))
        }
        Value::LeastPower(filter) => {
            Ok(context.aggregate(filter, NumericProperty::Power, Reduction::Min))
        }
        Value::LeastToughness(filter) => {
            Ok(context.aggregate(filter, NumericProperty::Toughness, Reduction::Min))
        }
        Value::LeastManaValue(filter) => {
            Ok(context.aggregate(filter, NumericProperty::ManaValue, Reduction::Min))
        }
        Value::BasicLandTypesAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                for subtype in object.subtypes(game, false) {
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
            Ok(seen.len() as i32)
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
            Ok(seen.len() as i32)
        }
        Value::CardTypesAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.extend(object.card_types(game));
            });
            Ok(seen.len() as i32)
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
            Ok(seen.len() as i32)
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
            Ok(seen.len() as i32)
        }
        Value::ColorPairsAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                let colors = object.colors();
                if colors.count() == 2 {
                    seen.insert(colors);
                }
            });
            Ok(seen.len() as i32)
        }
        Value::DistinctCounterTypesAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.extend(object.counters().keys().copied());
            });
            Ok(seen.len() as i32)
        }
        Value::DistinctNames(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.insert(object.name().to_string());
            });
            Ok(seen.len() as i32)
        }
        Value::DistinctManaValues(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.insert(object.filter_mana_value());
            });
            Ok(seen.len() as i32)
        }
        Value::DistinctPowers(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                if let Some(power) = object.power(game) {
                    seen.insert(power);
                }
            });
            Ok(seen.len() as i32)
        }
        Value::TurnHistoryCount(query) => Ok(crate::turn_history::resolve_turn_history_count(
            game,
            query,
            &context.filter_context(game),
            context
                .execution()
                .and_then(|ctx| ctx.triggering_event.as_ref()),
        )),
        Value::CreaturesDiedThisTurn => Ok(game
            .turn_store
            .turn_history
            .total_creatures_died_this_turn() as i32),
        Value::CreaturesDiedThisTurnControlledBy(player_filter) => {
            let filter_ctx = context.filter_context(game);
            let mut total = 0i32;
            for player in game.players.iter().filter(|p| p.is_in_game()) {
                if !player_filter.matches_player(player.id, &filter_ctx) {
                    continue;
                }
                total += game
                    .turn_store
                    .turn_history
                    .creatures_died_under_controller(player.id) as i32;
            }
            Ok(total)
        }
        Value::PlayersBeingAttacked => Ok(game
            .combat
            .as_ref()
            .map(crate::combat_state::defending_players)
            .map(|players| players.len() as i32)
            .unwrap_or(0)),
        Value::CountPlayers(player_filter) => {
            Ok(context.matching_player_ids(player_filter).len() as i32)
        }
        Value::CountPlayersWithCardsInHandAtLeast(player_filter, minimum) => Ok(context
            .matching_player_ids(player_filter)
            .into_iter()
            .filter(|id| {
                game.player(*id)
                    .is_some_and(|player| player.hand.len() >= *minimum as usize)
            })
            .count()
            as i32),
        Value::PartySize(player_filter) => {
            if let Some(ctx) = context.execution() {
                {
                    let player_id = resolve_player_filter(game, player_filter, ctx)?;
                    Ok(crate::party::party_size(game, player_id))
                }
            } else {
                Ok(context.layer().party_size(value, player_filter))
            }
        }
        Value::SourcePower => context.source_number(NumericProperty::Power),
        Value::SourceToughness => context.source_number(NumericProperty::Toughness),
        Value::PowerOf(target_spec) => context.object_number(target_spec, NumericProperty::Power),
        Value::ToughnessOf(target_spec) => {
            context.object_number(target_spec, NumericProperty::Toughness)
        }
        Value::ManaValueOf(target_spec) => {
            context.object_number(target_spec, NumericProperty::ManaValue)
        }
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
                            .count() as i32
                    };

                    if matches!(target_spec.base(), ChooseSpec::All(_)) {
                        return Ok(resolve_objects_from_spec(game, target_spec, ctx)?
                            .into_iter()
                            .filter_map(|id| game.object(id))
                            .filter_map(|object| object.mana_cost.as_deref())
                            .map(count_symbols)
                            .sum());
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
                Ok(context
                    .layer()
                    .mana_symbols_in_mana_cost_of(value, target_spec, color))
            }
        }
        Value::NameStickerCharacterCountOnSource { character, .. } => {
            Ok(game.name_sticker_character_count_on_object(context.source, *character) as i32)
        }
        Value::LifeTotal(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(player.life)
        }
        Value::LifeTotalAsTurnBegan(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            let history = &game.turn_store.turn_history;
            Ok(
                player.life + history.total_life_lost_for_players(&[player.id]) as i32
                    - history.total_life_gained_for_players(&[player.id]) as i32,
            )
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
            Ok(maximum - minimum)
        }
        Value::Speed(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(player.speed.unwrap_or(0) as i32)
        }
        Value::StartingLifeTotal(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(player.starting_life)
        }
        Value::HalfLifeTotalRoundedUp(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok((player.life + 1).div_euclid(2))
        }
        Value::HalfLifeTotalRoundedDown(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(player.life.div_euclid(2))
        }
        Value::HalfStartingLifeTotalRoundedUp(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok((player.starting_life + 1).div_euclid(2))
        }
        Value::HalfStartingLifeTotalRoundedDown(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(player.starting_life.div_euclid(2))
        }
        Value::CardsInHand(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(player.hand.len() as i32)
        }
        Value::CardsInLibrary(player_spec) => {
            let players = context.library_player_ids(value, player_spec)?;
            players
                .into_iter()
                .map(|id| {
                    game.player(id)
                        .map(|player| player.library.len() as i32)
                        .ok_or(ExecutionError::PlayerNotFound(id))
                })
                .sum()
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
            Ok(players
                .into_iter()
                .map(|id| game.devotion_to_color(id, chosen) as i32)
                .sum())
        }
        Value::LifeGainedThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_life_gained_for_players(&player_ids);
            Ok(total as i32)
        }
        Value::LifeLostThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_life_lost_for_players(&player_ids);
            Ok(total as i32)
        }
        Value::CardsDiscardedThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_cards_discarded_for_players(&player_ids);
            Ok(total as i32)
        }
        Value::AttractionsVisitedThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            Ok(game
                .turn_store
                .turn_history
                .total_attractions_visited_for_players(&player_ids) as i32)
        }
        Value::DamageDealtToPlayersThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_damage_to_players(&player_ids);
            Ok(total as i32)
        }
        Value::NoncombatDamageDealtToPlayersThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let total = game
                .turn_store
                .turn_history
                .total_noncombat_damage_to_players(&player_ids);
            Ok(total as i32)
        }
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, colors } => {
            let player_ids = context.player_ids(value, player)?;
            let total = game
                .turn_store
                .turn_history
                .total_noncombat_damage_dealt_by_sources_controlled_by(&player_ids, *colors);
            Ok(total as i32)
        }
        Value::MaxCardsInHand(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
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
        Value::MaxCardsDrawnThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            if player_ids.is_empty() {
                return Err(ExecutionError::UnresolvableValue(
                    "MaxCardsDrawnThisTurn requires a matching player".to_string(),
                ));
            }
            Ok(game
                .turn_store
                .turn_history
                .max_cards_drawn_for_players(&player_ids) as i32)
        }
        Value::MaxDiceRolledThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            if player_ids.is_empty() {
                return Err(ExecutionError::UnresolvableValue(
                    "MaxDiceRolledThisTurn requires a matching player".to_string(),
                ));
            }
            Ok(game
                .turn_store
                .turn_history
                .max_die_rolls_for_players(&player_ids) as i32)
        }
        Value::LandsEnteredBattlefieldThisTurn(player_spec) => {
            let player_ids = context.counter_player_ids(value, player_spec)?;
            Ok(game
                .turn_store
                .turn_history
                .total_lands_entered_for_players(&player_ids) as i32)
        }
        Value::CardsInGraveyard(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let mut max_count: Option<i32> = None;
            for player_id in player_ids {
                let player = game
                    .player(player_id)
                    .ok_or(ExecutionError::PlayerNotFound(player_id))?;
                let count = player.graveyard.len() as i32;
                max_count = Some(max_count.map_or(count, |prev| prev.max(count)));
            }
            Ok(max_count.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "CardsInGraveyard requires a matching player".to_string(),
                )
            })?)
        }
        Value::SpellsCastThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            Ok(game
                .turn_store
                .turn_history
                .total_spells_cast_for_players(&player_ids) as i32)
        }
        Value::SpellsCastBeforeThisTurn(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let count = game
                .turn_store
                .turn_history
                .total_spells_cast_for_players(&player_ids) as i32;
            Ok((count - 1).max(0))
        }
        Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let player_ids = context.player_ids(value, player)?;
            let filter_ctx = context.filter_context(game);
            let mut count: i32 = 0;
            for snapshot in game.turn_store.turn_history.spell_cast_snapshot_history() {
                if *exclude_source && snapshot.object_id == context.source {
                    continue;
                }
                if !player_ids.contains(&snapshot.controller) {
                    continue;
                }
                if filter.matches_snapshot(&snapshot, &filter_ctx, game) {
                    count = context.add_spell_metric(count, 1);
                }
            }
            Ok(count)
        }
        Value::TotalManaValueOfSpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let player_ids = context.player_ids(value, player)?;
            let filter_ctx = context.filter_context(game);
            let mut total: i32 = 0;
            for snapshot in game.turn_store.turn_history.spell_cast_snapshot_history() {
                if *exclude_source && snapshot.object_id == context.source {
                    continue;
                }
                if !player_ids.contains(&snapshot.controller) {
                    continue;
                }
                if filter.matches_snapshot(&snapshot, &filter_ctx, game) {
                    total = context.add_spell_metric(total, snapshot.mana_value() as i32);
                }
            }
            Ok(total)
        }
        Value::CommanderCastCount(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            Ok(player_ids
                .into_iter()
                .map(|player_id| game.commander_cast_count_for_player(player_id) as i32)
                .sum())
        }
        Value::ThisAbilityResolvedThisTurnCount => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            {
                if let Some(ability_index) = ctx.ability_index {
                    return Ok(game
                        .activated_ability_resolution_count_this_turn(ctx.source, ability_index)
                        as i32);
                }
                if let Some(trigger_identity) = ctx.trigger_identity {
                    return Ok(game
                        .triggered_ability_resolution_count_this_turn(ctx.source, trigger_identity)
                        as i32);
                }
                Err(ExecutionError::UnresolvableValue(
                    "this ability resolution count requires a resolving ability context"
                        .to_string(),
                ))
            }
        }
        Value::SourceRegeneratedThisTurnCount => {
            Ok(game.regenerated_this_turn_count(context.source) as i32)
        }
        Value::SourceMutationCount => Ok(game.mutation_count(context.source) as i32),
        Value::DamageDealtThisTurnByTaggedSpellCast(tag) => {
            let id = context.tagged_spell_id(value, tag)?;
            Ok(game
                .turn_store
                .turn_history
                .damage_dealt_by_spell_this_turn(game.provenance_graph(), id) as i32)
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
                    for card_type in &obj.card_types {
                        types.insert(*card_type);
                    }
                }
            }

            Ok(types.len() as i32)
        }
        Value::Devotion { player, color } => {
            let player_ids = context.player_ids(value, player)?;
            let devotion: usize = player_ids
                .iter()
                .map(|pid| game.devotion_to_color(*pid, *color))
                .sum();
            Ok(devotion as i32)
        }
        Value::ManaSpentToCastThisSpell => {
            let Some(source_obj) = game.object(context.source) else {
                return Ok(0);
            };
            Ok(source_obj.mana_spent_to_cast.total() as i32)
        }
        Value::ManaSymbolSpentToCastThisSpell { symbol, .. } => {
            let Some(source_obj) = game.object(context.source) else {
                return Ok(0);
            };
            Ok(source_obj.mana_spent_to_cast.amount(*symbol) as i32)
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
                        return Ok(0);
                    };
                    let filter_ctx = ctx.filter_context(game);
                    Ok(snapshots
                        .iter()
                        .filter(|snapshot| {
                            source_filter.matches_snapshot(snapshot, &filter_ctx, game)
                        })
                        .count() as i32)
                }
            } else {
                Ok(context
                    .layer()
                    .mana_from_source_spent_to_cast_this_spell(value, source_filter))
            }
        }
        Value::ManaSpentToCastTriggeringObject => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            {
                let Some(triggering_event) = &ctx.triggering_event else {
                    return Ok(0);
                };
                let Some(spell_cast) = triggering_event.downcast::<crate::events::SpellCastEvent>()
                else {
                    return Ok(0);
                };
                if let Some(snapshot) = spell_cast.snapshot.as_ref() {
                    return Ok(snapshot.mana_spent_to_cast.total() as i32);
                }
                Ok(game
                    .object(spell_cast.spell)
                    .map(|object| object.mana_spent_to_cast.total() as i32)
                    .unwrap_or(0))
            }
        }
        Value::UnspentMana(player) => {
            let player_ids = context.player_ids(value, player)?;
            let total = player_ids
                .iter()
                .filter_map(|player_id| game.player(*player_id))
                .map(|player| player.mana_pool.total() as i32)
                .sum();
            Ok(total)
        }
        Value::ColorsOfManaSpentToCastThisSpell => {
            let Some(source_obj) = game.object(context.source) else {
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
        Value::MagicGamesLostToOpponentsSinceLastWin => {
            let _ctx = context.require_execution(value, RESOLUTION_ONLY);
            Ok(0)
        }
        Value::DraftNotedHighestNumber { card_name } => Ok(game
            .draft_noted_highest_number(context.controller, card_name)
            .try_into()
            .unwrap_or(i32::MAX)),
        Value::LastNotedLifeTotal => game
            .noted_life_total_for_source(context.source)
            .map(Ok)
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
                let outcome = ctx
                    .get_outcome(*effect_id)
                    .ok_or(ExecutionError::EffectNotFound(*effect_id))?;
                Ok(outcome.count_or_zero())
            }
        }
        Value::EffectValueOffset(effect_id, offset) => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            {
                let outcome = ctx
                    .get_outcome(*effect_id)
                    .ok_or(ExecutionError::EffectNotFound(*effect_id))?;
                Ok(outcome.count_or_zero() + *offset)
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
            Ok(resolve_effect_metric(game, ctx, *effect_id, *source, *metric)? + *offset)
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
        Value::PendingPriorEffectMetric(_) => {
            let _ctx = context.require_execution(value, RESOLUTION_ONLY);
            Err(ExecutionError::UnresolvableValue(
                "pending effect metric was not bound to a prior effect".to_string(),
            ))
        }
        Value::HalfRoundedDown(inner) => Ok(resolve(inner, context)?.div_euclid(2)),
        Value::EventValue(spec) => {
            resolve_event_value(context.require_execution(value, RESOLUTION_ONLY), spec)
        }
        Value::EventValueOffset(spec, offset) => Ok(resolve_event_value(
            context.require_execution(value, RESOLUTION_ONLY),
            spec,
        )? + *offset),
        Value::WasKicked => {
            // Check if kicker or multikicker was paid
            // First check ctx, then fall back to source object (for ETB triggers)
            let paid = context.optional_costs_paid(value);
            Ok(if paid.was_kicked() { 1 } else { 0 })
        }
        Value::WasBoughtBack => {
            // Check if buyback was paid
            let paid = context.optional_costs_paid(value);
            Ok(if paid.was_bought_back() { 1 } else { 0 })
        }
        Value::WasEntwined => {
            // Check if entwine was paid
            let paid = context.optional_costs_paid(value);
            Ok(if paid.was_entwined() { 1 } else { 0 })
        }
        Value::WasPaid(index) => {
            // Check if the optional cost at the given index was paid
            let paid = context.optional_costs_paid(value);
            Ok(if paid.was_paid(*index) { 1 } else { 0 })
        }
        Value::WasPaidLabel(label) => {
            // Check if the optional cost with the given label was paid
            let paid = context.optional_costs_paid(value);
            Ok(if paid.was_paid_label(label.clone()) {
                1
            } else {
                0
            })
        }
        Value::TimesPaid(index) => {
            // Get the number of times the optional cost was paid
            let paid = context.optional_costs_paid(value);
            Ok(paid.times_paid(*index) as i32)
        }
        Value::TimesPaidLabel(label) => {
            // Get the number of times the optional cost with the label was paid
            let paid = context.optional_costs_paid(value);
            Ok(paid.times_paid_label(label.clone()) as i32)
        }
        Value::KickCount => {
            // Get the number of times the kicker was paid
            let paid = context.optional_costs_paid(value);
            Ok(paid.kick_count() as i32)
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
            Ok(player_ids
                .into_iter()
                .filter_map(|player_id| game.player(player_id))
                .map(|player| player.counter_count(*counter_type) as i32)
                .sum())
        }
        Value::CountersOnSource(counter_type) => {
            if let Some(ctx) = context.execution() {
                {
                    // Get the number of counters of the specified type on the source
                    if let Some(snapshot) = source_lki_for_moved_current_object(game, ctx) {
                        Ok(snapshot.counters.get(counter_type).copied().unwrap_or(0) as i32)
                    } else if let Some(source) = game.object(ctx.source) {
                        Ok(source.counters.get(counter_type).copied().unwrap_or(0) as i32)
                    } else if let Some(snapshot) = &ctx.source_snapshot {
                        Ok(snapshot.counters.get(counter_type).copied().unwrap_or(0) as i32)
                    } else {
                        Ok(0)
                    }
                }
            } else {
                Ok(context.layer().counters_on_source(value, counter_type))
            }
        }
        Value::CountersOn(spec, counter_type) => {
            if let Some(ctx) = context.execution() {
                {
                    if let Some(snapshots) = tagged_snapshots_for_choose_spec(ctx, spec) {
                        return Ok(snapshots
                            .iter()
                            .map(|snapshot| snapshot_counter_total(snapshot, counter_type))
                            .sum());
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
                            snapshot.counters.get(counter_type).copied().unwrap_or(0) as i32
                        } else {
                            snapshot.counters.values().map(|count| *count as i32).sum()
                        };
                        return Ok(total);
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
                                    snapshot.counters.get(counter_type).copied().unwrap_or(0) as i32
                                } else {
                                    snapshot.counters.values().map(|count| *count as i32).sum()
                                }
                            } else if let Some(obj) = game.object(id) {
                                if let Some(counter_type) = counter_type {
                                    obj.counters.get(counter_type).copied().unwrap_or(0) as i32
                                } else {
                                    obj.counters.values().map(|count| *count as i32).sum()
                                }
                            } else if let Some(snapshot) = object_lki_snapshot(ctx, id) {
                                if let Some(counter_type) = counter_type {
                                    snapshot.counters.get(counter_type).copied().unwrap_or(0) as i32
                                } else {
                                    snapshot.counters.values().map(|count| *count as i32).sum()
                                }
                            } else {
                                0
                            }
                        })
                        .sum();
                    Ok(total)
                }
            } else {
                Ok(context.layer().counters_on(value, spec, counter_type))
            }
        }
        Value::TaggedCount => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            {
                // Get the count of tagged objects for the current controller
                // (set by ForEachControllerOfTaggedEffect during iteration)
                if let Some(outcome) = ctx.get_outcome(crate::effect::EffectId::TAGGED_COUNT) {
                    Ok(outcome.count_or_zero())
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
            Ok(ctx
                .vote_results
                .get(&ctx.source)
                .map(|result| result.count_for_option(option) as i32)
                .unwrap_or(0))
        }
        Value::PlayerVoteCount(filter) => {
            let ctx =
                context.require_execution(value, "vote totals require a resolving vote context");
            {
                let resolved_filter = resolve_player_filter(game, filter, ctx)?;
                Ok(ctx
                    .vote_results
                    .get(&ctx.source)
                    .map(|result| {
                        result.count_for_player_filter(&crate::target::PlayerFilter::Specific(
                            resolved_filter,
                        )) as i32
                    })
                    .unwrap_or(0))
            }
        }
    }
}

const RESOLUTION_ONLY: &str =
    "value requires resolution, trigger, loop, or out-of-game context that layers do not retain";
fn resolve_event_value(
    ctx: &ExecutionContext,
    spec: &EventValueSpec,
) -> Result<i32, ExecutionError> {
    match spec {
        EventValueSpec::Amount | EventValueSpec::LifeAmount => {
            if let Some(amount) = ctx.event_value_amount {
                return Ok(amount);
            }
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
            if let Some(prevented_event) =
                triggering_event.downcast::<crate::events::DamagePreventedEvent>()
            {
                return Ok(prevented_event.amount as i32);
            }
            if let Some(markers_event) = triggering_event.downcast::<MarkersChangedEvent>() {
                return Ok(markers_event.amount as i32);
            }
            if let Some(counter_event) = triggering_event.downcast::<CounterPlacedEvent>() {
                return Ok(counter_event.amount as i32);
            }
            if let Some(zone_change_event) = triggering_event.downcast::<ZoneChangeEvent>() {
                return Ok(zone_change_event.count() as i32);
            }
            if let Some(keyword_action_event) = triggering_event.downcast::<KeywordActionEvent>() {
                return Ok(keyword_action_event.amount as i32);
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
                let beyond_first = event.blocker_count.saturating_sub(1) as i32;
                return Ok(beyond_first * *multiplier);
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
