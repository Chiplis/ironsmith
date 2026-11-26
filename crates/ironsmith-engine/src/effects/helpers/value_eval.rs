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

/// A characteristic-setting scalar uses the same checked boundary as later P/T
/// arithmetic. Its provisional value cannot be published by a checked query
/// when this records a range error; in particular an aggregate counter total
/// must not wrap or panic before that boundary can inspect it.
pub(crate) fn resolve_continuous_characteristic(
    value: &Value,
    layer: LayerValueContext<'_, '_>,
    error: &mut Option<(&'static str, i128)>,
    evidence_error: &mut Option<&'static str>,
) -> i32 {
    let context = EvaluationContext::continuous(layer);
    match resolve_wide(value, &context) {
        Ok(exact) => ironsmith_core::quantity::clamp_i32(exact),
        Err(ExecutionError::ResourceLimitExceeded {
            resource,
            requested,
            ..
        }) => {
            error.get_or_insert((resource, i128::try_from(requested).unwrap_or(i128::MAX)));
            0
        }
        Err(_) if crate::source_numbers::contains_value(value) => {
            evidence_error.get_or_insert("the source numeric pair or acquisition is unrecorded");
            0
        }
        Err(_) => match resolve_wide_zeroing_unavailable(value, &context) {
            // CR 208.2a: a number the ability uses that can't be determined
            // is zero; the rest of the arithmetic still applies.
            Some(exact) => ironsmith_core::quantity::clamp_i32(exact),
            None => {
                evidence_error.get_or_insert("the continuous characteristic value is unavailable");
                0
            }
        },
    }
}

/// Evaluate `value` substituting zero for each operand that can't be
/// determined (CR 208.2a). Arithmetic overflow and resource limits are not
/// "undeterminable" and still fail.
fn resolve_wide_zeroing_unavailable(
    value: &Value,
    context: &EvaluationContext<'_, '_>,
) -> Option<i64> {
    let operand = |value: &Value| resolve_wide_zeroing_unavailable(value, context);
    match value {
        Value::Add(left, right) => Some(operand(left)?.saturating_add(operand(right)?)),
        Value::Scaled(value, multiplier) => {
            Some(operand(value)?.saturating_mul(i64::from(*multiplier)))
        }
        Value::Min(left, right) => Some(operand(left)?.min(operand(right)?)),
        Value::DividedRoundedDown(value, divisor) if *divisor != 0 => {
            Some(ironsmith_core::quantity::div_floor(operand(value)?, i64::from(*divisor)))
        }
        _ => match resolve_wide(value, context) {
            Ok(value) => Some(value),
            // Only an as-yet-unchosen player is undeterminable; a reference to
            // a player that doesn't exist stays missing evidence.
            Err(ExecutionError::UnresolvableValue(_))
                if format!("{value:?}").contains("ChosenPlayer") =>
            {
                Some(0)
            }
            Err(_) => None,
        },
    }
}

pub(crate) fn resolve(
    value: &Value,
    context: &EvaluationContext<'_, '_>,
) -> Result<i32, ExecutionError> {
    // Game quantities saturate at the signed characteristic range.
    resolve_wide(value, context).map(ironsmith_core::quantity::clamp_i32)
}

pub(crate) fn resolve_wide(
    value: &Value,
    context: &EvaluationContext<'_, '_>,
) -> Result<i64, ExecutionError> {
    let game = context.game;
    use ironsmith_core::tag::{SacrificeCostTag, TagKeyWalk};
    let mut missing_original_sacrifice = false;
    value.for_each_tag_key(&mut |tag| {
        if matches!(
            SacrificeCostTag::parse(tag),
            Some(SacrificeCostTag::OriginalResult(_))
        ) && context
            .execution()
            .is_none_or(|ctx| !ctx.tagged_objects.contains_key(tag))
        {
            missing_original_sacrifice = true;
        }
    });
    if missing_original_sacrifice {
        let error = ExecutionError::IncompleteEvidence(
            "sacrifice-result quantity has no completed original-action receipt".into(),
        );
        game.record_token_resource_failure(&error);
        return Err(error);
    }
    match value {
        Value::SurfaceHinted { hints, .. }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::AsYouActivateThisAbility) => {
                context.require_execution(value, "activation samples require an execution context")
                    .activation_values.iter().find(|(expression, _)| expression == value)
                    .and_then(|(_, sampled)| *sampled).map(i64::from)
                    .ok_or_else(|| ExecutionError::IncompleteEvidence(
                        "activation-time expression has no announced sample".into()))
            }
        Value::SurfaceHinted { value, .. } => resolve_wide(value, context),
        Value::Fixed(n) => Ok(i64::from(*n)),
        Value::Add(left, right) => {
            Ok(resolve_wide(left, context)?.saturating_add(resolve_wide(right, context)?))
        }
        Value::X => context.x(),
        Value::XTimes(multiplier) => {
            Ok(context.x()?.saturating_mul(i64::from(*multiplier)))
        }
        Value::Scaled(value, multiplier) => {
            Ok(resolve_wide(value, context)?.saturating_mul(i64::from(*multiplier)))
        }
        Value::DividedRoundedDown(value, divisor) => {
            if *divisor == 0 {
                return context.division_by_zero(value);
            }
            Ok(ironsmith_core::quantity::div_floor(
                resolve_wide(value, context)?,
                i64::from(*divisor),
            ))
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
            i64::from(context.count_objects(filter, false)).saturating_mul(i64::from(*multiplier)),
        )),
        Value::GreatestCount(filter) => {
            Ok(i64::from(context.greatest_per_controller(filter, false)))
        }
        Value::LeastCount(filter) => Ok(i64::from(context.least_per_player(filter))),
        Value::GreatestSharedCreatureTypeCount(filter) => {
            Ok(i64::from(context.greatest_per_controller(filter, true)))
        }
        Value::GreatestSharedNameCount(filter) => {
            context.greatest_shared_name_count(filter)
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
                for &subtype in object.subtypes() {
                    if subtype.is_creature_type() {
                        seen.insert(subtype);
                    }
                }
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::PlaneswalkerTypesAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                for &subtype in object.subtypes() {
                    if subtype.is_planeswalker_subtype() {
                        seen.insert(subtype);
                    }
                }
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::SubtypesAmong { family, filter } => {
            let mut seen = HashSet::new();
            context.visit_current_subtypes(filter, |subtypes| {
                for &subtype in subtypes {
                    if family.is_none_or(|family| family.contains(subtype)) {
                        seen.insert(subtype);
                    }
                }
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::CardTypesSharedByAll(filter) => {
            let mut shared: Option<HashSet<_>> = None;
            context.visit_property_objects(filter, |object| {
                let types: HashSet<_> = object.card_types().iter().copied().collect();
                shared = Some(match shared.take() {
                    None => types,
                    Some(prior) => prior.intersection(&types).copied().collect(),
                });
            });
            Ok(i64::from(shared.map_or(0, |types| types.len()) as i64))
        }
        Value::CardTypesAmong(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.extend(object.card_types().iter().copied());
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::PermanentTypesAmong(filter) => {
            // CR 110.4: only artifact, battle, creature, enchantment, land,
            // and planeswalker are permanent types (Kindred is not).
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.extend(
                    object
                        .card_types()
                        .iter().copied()
                        .filter(|card_type| card_type.is_permanent_type()),
                );
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
            let mut colors = crate::color::ColorSet::COLORLESS;
            context.visit_property_objects(filter, |object| colors = colors.union(object.colors()));
            Ok(i64::from(colors.count()))
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
        Value::DistinctNames(filter) => context.distinct_names(filter),
        Value::DistinctManaValues(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                seen.insert(object.filter_mana_value());
            });
            Ok(i64::from(seen.len() as i64))
        }
        // CR 202.1: mana costs compare as the multiset of their symbols; an
        // object without a mana cost has none to contribute.
        Value::DistinctManaCosts(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                let Some(cost) = object.mana_cost() else {
                    return;
                };
                // Compare costs, not their spelling: combined halves such as
                // {2}{U} + {1}{R} are the cost {3}{U}{R} (CR 202.1, 709.4b).
                let generic: u32 = cost
                    .pips()
                    .iter()
                    .filter_map(|pip| match pip.as_slice() {
                        [crate::mana::ManaSymbol::Generic(amount)] => Some(u32::from(*amount)),
                        _ => None,
                    })
                    .sum();
                let mut key = cost
                    .pips()
                    .iter()
                    .filter(|pip| !matches!(pip.as_slice(), [crate::mana::ManaSymbol::Generic(_)]))
                    .map(|pip| {
                        let mut symbols = pip.iter().map(|symbol| format!("{symbol:?}")).collect::<Vec<_>>();
                        symbols.sort();
                        symbols.join("/")
                    })
                    .collect::<Vec<_>>();
                if generic > 0 {
                    key.push(format!("Generic({generic})"));
                }
                key.sort();
                seen.insert(key);
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::UnlockedDoorsAmong(filter) => {
            let mut doors = 0i64;
            let mut names = std::collections::HashSet::new();
            context.visit_property_objects(filter, |object| {
                let context::PropertyObject::Current(object, _) = object
                else {
                    return;
                };
                if filter.distinct_names {
                    for door in crate::special_actions::unlocked_room_doors(game, object.id) {
                        if let Some(name) = crate::special_actions::room_door_name(game, object.id, door) {
                            let half = crate::continuous::room_half_name(&name);
                            names.insert(crate::filter::interchangeable_name_key(half).to_owned());
                        }
                    }
                } else {
                    doors += i64::from(crate::effects::helpers::room_unlocked_door_count(game, object));
                }
            });
            Ok(if filter.distinct_names { names.len() as i64 } else { doors })
        }
        Value::DistinctPowers(filter) => {
            let mut seen = HashSet::new();
            context.visit_property_objects(filter, |object| {
                if let Some(power) = object.power() {
                    seen.insert(power);
                }
            });
            Ok(i64::from(seen.len() as i64))
        }
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::CardsDrawn(player)) => {
            context.validate_history_player_reference(value, player)?;
            let filter_ctx = context.filter_context(game);
            game.turn_store
                .turn_history
                .cards_drawn_matching_players_wide(|id| player.matches_player(id, &filter_ctx))
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
                total = total.saturating_add(i64::from(
                    game.turn_store
                        .turn_history
                        .creatures_died_under_controller(player.id),
                ));
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
        Value::CountPlayersWithCardsInLibraryAtMost(player_filter, maximum) => Ok(i64::from(
            context
                .matching_player_ids(player_filter)
                .into_iter()
                .filter(|id| {
                    game.player(*id)
                        .is_some_and(|player| player.library.len() <= *maximum as usize)
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
        Value::PowerOf(target_spec) => {
            if noncreature_permanent_reference(game, context, target_spec) {
                return Ok(0);
            }
            context.object_number_wide(target_spec, NumericProperty::Power)
        }
        Value::BasePowerOf(target_spec) => context
            .object_number_wide(target_spec, NumericProperty::BasePower),
        Value::StationContributionOf(target_spec) => {
            // CR 702.184c / 608.2h: a contributor that left the battlefield
            // uses its last known information, including the static
            // abilities that changed which characteristic it stations with
            // or multiplied the power it stations with.
            let (uses_toughness, multiplier) = context
                .execution()
                .map(|ctx| {
                    let stationed_is_spacecraft = match game.current_subtypes(ctx.source) {
                        Some(subtypes) => subtypes.contains(&crate::types::Subtype::Spacecraft),
                        None => game.object(ctx.source).is_some_and(|object| {
                            object.has_subtype(crate::types::Subtype::Spacecraft)
                        }),
                    };
                    match resolve_primary_object_from_value_spec(game, target_spec, ctx).ok() {
                        Some(id)
                            if game.object(id).is_some_and(|object| {
                                object.zone == crate::zone::Zone::Battlefield
                            }) =>
                        {
                            (
                                crate::effects::permanents::stations_using_toughness(game, id),
                                crate::effects::permanents::station_power_multiplier_on_battlefield(
                                    game,
                                    id,
                                    stationed_is_spacecraft,
                                ),
                            )
                        }
                        _ => match target_spec.base() {
                            ChooseSpec::Tagged(tag) => {
                                let mechanics = ctx
                                    .tagged_objects
                                    .get(tag)
                                    .and_then(|snapshots| snapshots.first())
                                    .map(|snapshot| {
                                        snapshot
                                            .abilities
                                            .iter()
                                            .filter_map(|ability| match &ability.kind {
                                                crate::ability::AbilityKind::Static(
                                                    static_ability,
                                                ) => static_ability.keyword_marker_mechanic(),
                                                _ => None,
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                (
                                    mechanics.contains(
                                        &ironsmith_core::KeywordMarkerMechanic::StationsUsingToughness,
                                    ),
                                    crate::effects::permanents::station_power_multiplier(
                                        mechanics.iter(),
                                        stationed_is_spacecraft,
                                    ),
                                )
                            }
                            _ => (false, 1),
                        },
                    }
                })
                .unwrap_or((false, 1));
            if uses_toughness {
                context.object_number_wide(target_spec, NumericProperty::Toughness)
            } else {
                context
                    .object_number_wide(target_spec, NumericProperty::Power)
                    .map(|power| power.saturating_mul(multiplier))
            }
        }
        Value::ToughnessOf(target_spec) => {
            if noncreature_permanent_reference(game, context, target_spec) {
                return Ok(0);
            }
            context.object_number_wide(target_spec, NumericProperty::Toughness)
        }
        // "the number of times you chose a mode for that spell" (Riku of
        // Many Paths): the committed modes of that spell's stack entry.
        Value::ModeSelectionsOf(target_spec) => {
            let ctx = context.require_execution(
                value,
                "mode-selection counts require a resolving context",
            );
            let id = resolve_primary_object_from_value_spec(game, target_spec, ctx)?;
            Ok(game
                .stack
                .iter()
                .find(|entry| entry.object_id == id)
                .and_then(|entry| entry.chosen_modes.as_ref())
                .map_or(0, |modes| modes.len() as i64))
        }
        Value::TimesPaidLabelOf {object: spec,reference} => {
            let ctx = context.require_execution(value, "referenced optional payments require a resolving context");
            // The event snapshot is authoritative even after the referenced
            // spell leaves the stack or the physical card changes zones.
            if let ChooseSpec::Tagged(tag) = spec.base()
                && let Some(snapshot) = ctx.get_tagged(tag)
            {
                return Ok(i64::from(snapshot.optional_costs_paid.times_paid_label(reference.clone())));
            }
            let id = resolve_primary_object_from_value_spec(game, spec, ctx)?;
            let object = game.object(id).ok_or(ExecutionError::ObjectNotFound(id))?;
            Ok(i64::from(object.optional_costs_paid.times_paid_label(reference.clone())))
        }
        Value::KicksPaidOf(target_spec) => context
            .object_number(target_spec, NumericProperty::KickerCount)
            .map(i64::from),
        Value::ManaSpentToCast(target_spec) => context
            .object_number(target_spec, NumericProperty::ManaSpent)
            .map(i64::from),
        Value::ManaValueOf(target_spec) => {
            context.object_number_wide(target_spec, NumericProperty::ManaValue)
        }
        Value::ColorsOf(target_spec) => context
            .object_number_wide(target_spec, NumericProperty::ColorCount),
        // "for each of the chosen colors it is" (Tablet of the Guilds): the
        // referenced object's colors among the source's chosen colors.
        Value::ChosenColorsOf(target_spec) => {
            let ctx = context.require_execution(
                value,
                "chosen-color counts require a resolving context",
            );
            let chosen = game
                .chosen_colors(ctx.source)
                .unwrap_or(crate::color::ColorSet::COLORLESS);
            let id = resolve_primary_object_from_value_spec(game, target_spec, ctx)?;
            let colors = if let Some(object) = game.object(id) {
                game.current_colors(id).unwrap_or_else(|| object.colors())
            } else if let ChooseSpec::Tagged(tag) = target_spec.base()
                && let Some(snapshot) = ctx.get_tagged(tag)
            {
                snapshot.colors
            } else {
                return Err(ExecutionError::ObjectNotFound(id));
            };
            Ok(i64::from(colors.intersection(chosen).count()))
        }
        // "for each creature type noted for target permanent ..." (Volo,
        // Itinerant Scholar): the referenced object's noted-creature-type set.
        Value::NotedCreatureTypeCount(target_spec) => {
            let ctx = context.require_execution(
                value,
                "noted creature type counts require a resolving context",
            );
            let id = resolve_primary_object_from_value_spec(game, target_spec, ctx)?;
            Ok(game.noted_creature_types_for_source(id).len() as i64)
        }
        // "for each supertype, card type, and subtype it has" (Embiggen):
        // the referenced object's current characteristics (CR 205).
        // CR 702.164c: the total toxic value is the sum of the N of each toxic
        // ability the object has (Goliath Hatchery).
        Value::TotalToxicValueOf(target_spec) => {
            let ctx = context.require_execution(
                value,
                "toxic totals require a resolving context",
            );
            let id = resolve_primary_object_from_value_spec(game, target_spec, ctx)?;
            if let Some(chars) = game.calculated_characteristics_arc(id) {
                return Ok(chars
                    .static_abilities
                    .iter()
                    .filter_map(|ability| ability.toxic_amount())
                    .map(i64::from)
                    .sum());
            }
            if let ChooseSpec::Tagged(tag) = target_spec.base()
                && let Some(snapshot) = ctx.get_tagged(tag)
            {
                return Ok(snapshot
                    .abilities
                    .iter()
                    .filter_map(|ability| match &ability.kind {
                        crate::ability::AbilityKind::Static(ability) => ability.toxic_amount(),
                        _ => None,
                    })
                    .map(i64::from)
                    .sum());
            }
            Err(ExecutionError::ObjectNotFound(id))
        }
        Value::TypeCountOf(target_spec) => {
            if let Some(total) = context.linked_exile_characteristic_sum(target_spec, |chars| {
                (chars.supertypes.len() + chars.card_types.len() + chars.subtypes.len()) as i64
            })? {
                return Ok(total);
            }
            let ctx = context.require_execution(
                value,
                "type counts require a resolving context",
            );
            let id = resolve_primary_object_from_value_spec(game, target_spec, ctx)?;
            if let Some(chars) = game.calculated_characteristics_arc(id) {
                return Ok((chars.supertypes.len() + chars.card_types.len() + chars.subtypes.len())
                    as i64);
            }
            if let ChooseSpec::Tagged(tag) = target_spec.base()
                && let Some(snapshot) = ctx.get_tagged(tag)
            {
                return Ok((snapshot.supertypes.len()
                    + snapshot.card_types.len()
                    + snapshot.subtypes.len()) as i64);
            }
            Err(ExecutionError::ObjectNotFound(id))
        }
        // "the amount of generic mana in that spell's mana cost": the
        // generic-only pips of the current mana cost (CR 107.4b); on the
        // stack each {X} is the value chosen for X (CR 107.3).
        Value::GenericManaInManaCostOf(target_spec) => {
            let generic = |cost: &crate::mana::ManaCost, x_value: Option<u32>| -> i64 {
                let x_pips = cost
                    .pips()
                    .iter()
                    .filter(|pip| pip.len() == 1 && matches!(pip[0], crate::mana::ManaSymbol::X))
                    .count() as i64;
                i64::from(cost.generic_mana_total())
                    .saturating_add(x_pips.saturating_mul(i64::from(x_value.unwrap_or(0))))
            };
            if let Some(total) = context.linked_exile_characteristic_sum(target_spec, |chars| {
                chars.mana_cost.as_ref().map_or(0, |cost| generic(cost, None))
            })? {
                return Ok(total);
            }
            let ctx = context.require_execution(
                value,
                "generic mana amounts require a resolving context",
            );
            let id = resolve_primary_object_from_value_spec(game, target_spec, ctx)?;
            if let Some(object) = game.object(id) {
                let x_value = (object.zone == crate::zone::Zone::Stack)
                    .then_some(object.x_value)
                    .flatten();
                let cost = game
                    .calculated_characteristics_arc(id)
                    .and_then(|chars| chars.mana_cost.as_deref().cloned())
                    .or_else(|| object.mana_cost.as_deref().cloned());
                return Ok(cost.as_ref().map_or(0, |cost| generic(cost, x_value)));
            }
            if let ChooseSpec::Tagged(tag) = target_spec.base()
                && let Some(snapshot) = ctx.get_tagged(tag)
            {
                let x_value = (snapshot.zone == crate::zone::Zone::Stack)
                    .then_some(snapshot.x_value)
                    .flatten();
                return Ok(snapshot
                    .mana_cost
                    .as_ref()
                    .map_or(0, |cost| generic(cost, x_value)));
            }
            Err(ExecutionError::ObjectNotFound(id))
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
                            .count() as i64
                    };

                    if let Some(total) = context.linked_exile_characteristic_sum(target_spec, |chars| {
                        chars.mana_cost.as_deref().map_or(0, count_symbols)
                    })? {
                        return Ok(total);
                    }

                    if matches!(target_spec.base(), ChooseSpec::All(_)) {
                        return Ok(i64::from(
                            resolve_objects_from_spec(game, target_spec, ctx)?
                                .into_iter()
                                .filter_map(|id| crate::filter::object_current_mana_cost(game, id))
                                .map(|cost| count_symbols(&cost))
                                .sum::<i64>(),
                        ));
                    }

                    if matches!(target_spec.base(), ChooseSpec::Source)
                        && let Some(snapshot) = source_characteristic_lki_for_execution(game, ctx)
                    {
                        return Ok(snapshot.mana_cost.as_ref().map_or(0, count_symbols));
                    }
                    let target_id =
                        resolve_primary_object_from_value_spec(game, target_spec.as_ref(), ctx)?;
                    let tagged_snapshot = if let ChooseSpec::Tagged(tag) = target_spec.base() {
                        ctx.get_tagged(tag)
                    } else {
                        None
                    };

                    if let Some(snapshot) = tagged_snapshot
                        && game
                            .object(snapshot.object_id)
                            .is_none_or(|object| object.zone != snapshot.zone)
                    {
                        Ok(latest_tagged_lki_snapshot(game, snapshot)
                            .unwrap_or(snapshot).mana_cost.as_ref().map_or(0, count_symbols))
                    } else if game.object(target_id).is_some() {
                        Ok(crate::filter::object_current_mana_cost(game, target_id)
                            .as_ref().map_or(0, count_symbols))
                    } else if let Some(snapshot) = tagged_snapshot {
                        Ok(snapshot.mana_cost.as_ref().map_or(0, count_symbols))
                    } else if let Some(snapshot) = object_lki_snapshot(ctx, target_id) {
                        Ok(snapshot.mana_cost.as_ref().map_or(0, count_symbols))
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
        Value::StickerCharacteristicTotal { filter, toughness } => {
            let filter_ctx = context.filter_context(game);
            Ok(game.battlefield.iter().filter_map(|id| game.object(*id))
                .filter(|object| filter.matches(object, &filter_ctx, game))
                .map(|object| game.sticker_characteristic_total(object.id, *toughness)).sum())
        }
        Value::StickersOnSource { action, .. } => Ok(i64::from(
            game.sticker_count_on_object(context.source, *action, None),
        )),
        Value::NameStickersOnSourceStartingWithChosenLetter { .. } => Ok(game
            .chosen_named_option(context.source)
            .and_then(|letter| letter.chars().next())
            .map_or(0, |letter| {
                i64::from(game.name_sticker_count_starting_with(context.source, letter))
            })),
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
            Ok(i64::from(player.life)
                .saturating_add(i64::from(history.total_life_lost_for_players(&[player.id])))
                .saturating_sub(i64::from(history.total_life_gained_for_players(&[player.id]))))
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
            Ok(i64::from(maximum).saturating_sub(i64::from(minimum)))
        }
        Value::Speed(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.speed.unwrap_or(0) as i64))
        }
        // The highest speed among the matching players (0 when none match).
        Value::MaxSpeed(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            let mut max_speed = 0i64;
            for pid in player_ids {
                let player = game
                    .player(pid)
                    .ok_or(ExecutionError::PlayerNotFound(pid))?;
                max_speed = max_speed.max(i64::from(player.speed.unwrap_or(0)));
            }
            Ok(max_speed)
        }
        Value::StartingLifeTotal(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.starting_life))
        }
        // Not recorded (a state built without a deck setup): the library
        // and command-zone commanders as they stand.
        Value::StartingDeckSize(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.starting_deck_size.unwrap_or_else(|| {
                u32::try_from(player.library.len().saturating_add(player.commanders.len()))
                    .unwrap_or(u32::MAX)
            })))
        }
        Value::HalfLifeTotalRoundedUp(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok((i64::from(player.life) + 1).div_euclid(2))
        }
        Value::HalfLifeTotalRoundedDown(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok(i64::from(player.life.div_euclid(2)))
        }
        Value::HalfStartingLifeTotalRoundedUp(player_spec) => {
            let player = context.single_player(value, player_spec)?;
            Ok((i64::from(player.starting_life) + 1).div_euclid(2))
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
            let player_ids = context.aggregate_player_ids(value, player_spec)?;
            for player in &player_ids {
                game.player(*player)
                    .ok_or(ExecutionError::PlayerNotFound(*player))?;
            }
            game.turn_store
                .turn_history
                .max_cards_drawn_for_players_wide(&player_ids)
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
            Ok(count.saturating_sub(1).max(0))
        }
        Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let player_ids = context.player_ids(value, player)?;
            let filter_ctx = context.filter_context(game);
            let mut count: i64 = 0;
            for (_, snapshot) in game
                .turn_store
                .turn_history
                .checked_spell_cast_history(&player_ids, exclude_source.then_some(context.source))?
            {
                if filter.matches_snapshot(&snapshot, &filter_ctx, game) {
                    count = context.add_spell_metric(count, 1)?;
                }
            }
            Ok(i64::from(count))
        }
        Value::CardTypesAmongSpellsCastThisTurn { player, filter } => {
            let player_ids = context.player_ids(value, player)?;
            let filter_ctx = context.filter_context(game);
            let mut seen = HashSet::new();
            for (_, snapshot) in game
                .turn_store
                .turn_history
                .checked_spell_cast_history(&player_ids, None)?
            {
                if filter.matches_snapshot(&snapshot, &filter_ctx, game) {
                    seen.extend(snapshot.card_types.iter().copied());
                }
            }
            Ok(seen.len() as i64)
        }
        Value::DistinctAmongSpellsCastThisTurn {
            characteristic,
            player,
            filter,
        } => {
            let player_ids = context.player_ids(value, player)?;
            let filter_ctx = context.filter_context(game);
            let mut seen = HashSet::new();
            for (_, snapshot) in game
                .turn_store
                .turn_history
                .checked_spell_cast_history(&player_ids, None)?
            {
                if filter.matches_snapshot(&snapshot, &filter_ctx, game) {
                    seen.extend(snapshot.distinct_characteristic_keys(*characteristic));
                }
            }
            Ok(seen.len() as i64)
        }
        Value::TotalManaValueOfSpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let player_ids = context.player_ids(value, player)?;
            let filter_ctx = context.filter_context(game);
            let mut total: i64 = 0;
            for (_, snapshot) in game
                .turn_store
                .turn_history
                .checked_spell_cast_history(&player_ids, exclude_source.then_some(context.source))?
            {
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
        Value::CommandersCastFromCommandZoneBy(player_spec) => {
            let player_ids = context.player_ids(value, player_spec)?;
            Ok(player_ids
                .into_iter()
                .map(|player_id| i64::from(game.commanders_cast_from_command_zone_by(player_id)))
                .sum::<i64>())
        }
        // "the number of times it's been cast from the command zone this
        // game": the referenced commander's own count. In a cost-reduction
        // context the iterated object is the spell being priced (Myth
        // Unbound); for an entering permanent it is the source (Opal Palace).
        Value::CommanderCastCountOf(target_spec) => {
            let id = match target_spec.base() {
                ChooseSpec::Source => context.source,
                _ => {
                    let ctx = context.require_execution(
                        value,
                        "commander cast counts of a referenced object require a resolving context",
                    );
                    resolve_primary_object_from_value_spec(game, target_spec, ctx)?
                }
            };
            Ok(i64::from(game.commander_cast_count(id)))
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
        Value::ThisAbilityManaProductionCountThisTurn => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            if ctx.ability_index.is_none() && ctx.trigger_identity.is_none() {
                return Err(ExecutionError::UnresolvableValue(
                    "this ability mana history requires an ability context".into()));
            }
            Ok(i64::from(game.ability_mana_production_count_this_turn(
                ctx.source, ctx.controller, ctx.ability_index, ctx.trigger_identity,
            )))
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
                game.player(player_id).ok_or(ExecutionError::PlayerNotFound(player_id))?;
                let filter = crate::target::ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::Specific(player_id));
                context.visit_property_objects(&filter, |object| {
                    types.extend(object.card_types().iter().copied());
                });
            }

            Ok(i64::from(types.len() as i64))
        }
        Value::DevotionToColors { player, colors } => {
            let players = context.player_ids(value, player)?;
            Ok(players.iter().map(|player| game.devotion_to_colors(*player, *colors) as i64).sum())
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
            Ok(i64::from(allocation.of_color(*color)))
        }
        Value::ManaSymbolSpentToCastThisSpell { symbol, .. } => {
            let Some(source_obj) = game.object(context.source) else {
                return Ok(i64::from(0));
            };
            // CR 107.4h: {S} counts units produced by snow sources, which the
            // payment records separately at spend time.
            let amount = if *symbol == crate::mana::ManaSymbol::Snow {
                source_obj.snow_mana_spent_to_cast.total()
            } else {
                source_obj.mana_spent_to_cast.amount(*symbol)
            };
            Ok(i64::from(amount as i64))
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
            Ok(i64::from(spent))
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
        Value::ColorsOfManaSpentToCastTriggeringObject => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            let Some(triggering_event) = &ctx.triggering_event else {
                return Ok(i64::from(0));
            };
            let Some(spell_cast) = triggering_event.downcast::<crate::events::SpellCastEvent>()
            else {
                return Ok(i64::from(0));
            };
            let spent = match spell_cast.snapshot.as_ref() {
                Some(snapshot) => snapshot.mana_spent_to_cast.clone(),
                None => match game.object(spell_cast.spell) {
                    Some(object) => object.mana_spent_to_cast.clone(),
                    None => return Ok(i64::from(0)),
                },
            };
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
        Value::UnspentMana(player) => {
            let player_ids = context.player_ids(value, player)?;
            let total = player_ids
                .iter()
                .filter_map(|player_id| game.player(*player_id))
                .filter_map(|player| game.player_mana_pool(player.id))
                .map(|pool| pool.total_wide() as i64)
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
        Value::DraftCorrectGuessCount { card_name } => Ok(i64::from(
            game.draft_correct_guess_count(context.controller, card_name)
                .try_into()
                .unwrap_or(i32::MAX),
        )),
        Value::DraftNotedHighestNumber { card_name } => Ok(i64::from(
            game.draft_noted_highest_number(context.controller, card_name)
                .try_into()
                .unwrap_or(i32::MAX),
        )),
        Value::SourceChosenNumber { if_unset, pair } => {
            let retained = context
                .execution()
                .and_then(|ctx| ctx.source_snapshot.as_ref());
            let continuous_owner = context.numeric_origin().and_then(|origin| {
                crate::source_numbers::capture(context.source, *pair, Some(origin))
            });
            let explicit = context
                .execution()
                .and_then(|ctx| ctx.source_number_owner.as_ref())
                .or(continuous_owner.as_ref());
            crate::source_numbers::read(game, context.source, *pair, explicit, retained)?
                .map(i64::from)
                .or_else(|| if_unset.map(i64::from))
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue("source has never chosen a number".into())
                })
        }
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
                game.source_departure_snapshot(context.source)
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
            if *effect_id == crate::effect::EffectId::ACTIVATION_COUNTER_COST
                && ctx.get_outcome(*effect_id).is_none()
            {
                return Err(ExecutionError::IncompleteEvidence(
                    "activation counter payment has no completed receipt".into(),
                ));
            }
            {
                // "That many" of an instruction that never ran is zero.
                Ok(i64::from(ctx.get_outcome(*effect_id).map_or(
                    0,
                    |outcome| {
                        // "You may pay {X}{R}. If you do, ... deals X damage"
                        // (Flameblast Dragon): the result is the X paid
                        // (CR 107.3), not the mana the payment's own mana
                        // abilities added (four Mountains for X=3 plus {R}).
                        let paid_x = outcome.instruction_result().execution_facts().iter().rev().find_map(
                            |fact| match fact {
                                crate::effect::ExecutionFact::ManaPaid { x_value } => {
                                    Some(i64::from(*x_value))
                                }
                                _ => None,
                            },
                        );
                        paid_x.or_else(|| outcome.as_count()).unwrap_or_else(|| {
                            outcome
                                .explicit_objects()
                                .map(|objects| objects.len() as i64)
                                // "You may tap X untapped Myr you control. If
                                // you do, ... X damage": an optional action's
                                // result is the objects it acted on.
                                .or_else(|| {
                                    let mut saw = false;
                                    let count = outcome.instruction_result().execution_facts().iter().filter_map(
                                        |fact| match fact {
                                            crate::effect::ExecutionFact::ActionObjects { objects, .. } => {
                                                saw = true;
                                                Some(objects.len() as i64)
                                            }
                                            _ => None,
                                        },
                                    ).sum::<i64>();
                                    saw.then_some(count)
                                })
                                // "You may pay {X}. When you do, ... deals that
                                // much damage": an acknowledged payment keeps
                                // the X it paid as an execution fact.
                                .or_else(|| {
                                    outcome.instruction_result().execution_facts().iter().rev().find_map(
                                        |fact| match fact {
                                            crate::effect::ExecutionFact::ManaPaid { x_value } => {
                                                Some(i64::from(*x_value))
                                            }
                                            _ => None,
                                        },
                                    )
                                })
                                .unwrap_or(0)
                        })
                    },
                )))
            }
        }
        Value::EffectValueOffset(effect_id, offset) => {
            let ctx = context.require_execution(value, RESOLUTION_ONLY);
            if *effect_id == crate::effect::EffectId::ACTIVATION_COUNTER_COST
                && ctx.get_outcome(*effect_id).is_none()
            {
                return Err(ExecutionError::IncompleteEvidence(
                    "activation counter payment has no completed receipt".into(),
                ));
            }
            {
                Ok(i64::from(
                    ctx.get_outcome(*effect_id).map_or(0, |outcome| {
                        outcome.as_count().unwrap_or_else(|| {
                            outcome
                                .explicit_objects()
                                .map_or(0, |objects| objects.len() as i64)
                        })
                    }).saturating_add(i64::from(*offset)),
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
                    .saturating_add(i64::from(*offset)),
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
        Value::PowerOfTwo(inner) => {
            let exponent = resolve_wide(inner, context)?;
            if exponent < 0 {
                return Err(ExecutionError::UnresolvableValue(
                    "power-of-two exponent is negative".into(),
                ));
            }
            // Game quantities saturate: an exponent past the wide range is i64::MAX.
            Ok(ironsmith_core::quantity::pow(
                2,
                u32::try_from(exponent).unwrap_or(u32::MAX),
            ))
        }
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
            )?
            .saturating_add(i64::from(*offset)),
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
            if label.requires_current_turn()
                && paid.costs.is_empty()
                && paid.cast_payment_turn.is_none()
                && context.execution().is_some_and(|ctx| {
                    game.object(ctx.source).is_none()
                        && !ctx
                            .source_snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.object_id == ctx.source)
                })
            {
                return Err(ExecutionError::IncompleteEvidence(
                    "payment source and receipt are unavailable".into(),
                ));
            }
            Ok(i64::from(
                crate::condition_eval::evaluate_paid_cost_receipt(
                    paid,
                    label,
                    game.turn.turn_number,
                    context.controller,
                )?,
            ))
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
        Value::PlayerNamedActionCount { player, label, window } => {
            let players = context.counter_player_ids(value, player)?;
            Ok(players
                .into_iter()
                .map(|player| i64::from(game.player_named_action_count(player, label, *window)))
                .sum())
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
                if let Some(snapshot)=crate::condition_eval::projected_condition_source_numeric_facts(ctx) {
                    return Ok(i64::from(snapshot.counters.get(counter_type).copied().unwrap_or(0)));
                }
                let amount = if let Some(snapshot) =
                    source_characteristic_lki_for_execution(game, ctx)
                {
                    snapshot.counters.get(counter_type).copied().unwrap_or(0)
                } else if let Some(source) = game.object(ctx.source) {
                    source.counters.get(counter_type).copied().unwrap_or(0)
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
        Value::NamedActionCount {object,label,this_turn} => {
            if let Some(ctx)=context.execution() {
                if matches!(object.base(),ChooseSpec::Source) && ctx.replacement.entry_counter_source==Some(ctx.source) {
                    let pending=ctx.replacement.entry_named_actions.iter().filter(|(name,_)|name.trim().eq_ignore_ascii_case(label.trim())).count();
                    return Ok(i64::try_from(pending).unwrap_or(i64::MAX));
                }
                if matches!(object.base(),ChooseSpec::Source) && game.object(ctx.source).is_none()
                    && let Some(snapshot)=source_characteristic_lki_for_execution(game,ctx) {
                    let history=snapshot.named_action_history.as_ref().ok_or_else(||ExecutionError::IncompleteEvidence("named action source history is unavailable".into()))?;
                    let count=history.get(label).map(|(total,turn,count)|if *this_turn {if *turn==game.turn.turn_number {*count}else{0}}else{*total}).unwrap_or(0);
                    return Ok(i64::from(count));
                }
                let objects=resolve_objects_from_spec(game,object,ctx)?;
                Ok(objects.into_iter().map(|id|i64::from(game.named_action_count(id,label,*this_turn))).sum())
            }else{context.layer().named_action_count(object,label,*this_turn)}
        }
        Value::CountersOn(spec, counter_type) => {
            if matches!(spec.base(),ChooseSpec::Source)
                && let Some(ctx)=context.execution()
                && let Some(snapshot)=crate::condition_eval::projected_condition_source_numeric_facts(ctx) {
                return Ok(match counter_type {
                    Some(kind)=>i64::from(snapshot.counters.get(kind).copied().unwrap_or(0)),
                    None=>snapshot.counters.values().map(|count|i64::from(*count)).sum(),
                });
            }
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
                        // CR 608.2h: a chosen object still in the public zone
                        // where it was chosen is read with current information
                        // (counters placed on it later in this resolution).
                        return Ok(i64::from(
                            snapshots
                                .iter()
                                .map(|snapshot| match game.object(snapshot.object_id) {
                                    Some(object)
                                        if object.stable_id == snapshot.stable_id
                                            && object.zone == snapshot.zone
                                            && object.zone.is_public() =>
                                    {
                                        match counter_type {
                                            Some(kind) => {
                                                object.counters.get(kind).copied().unwrap_or(0)
                                                    as i64
                                            }
                                            None => object
                                                .counters
                                                .values()
                                                .map(|count| *count as i64)
                                                .sum(),
                                        }
                                    }
                                    _ => snapshot_counter_total(snapshot, counter_type),
                                })
                                .sum::<i64>(),
                        ));
                    }

                    if matches!(spec.base(), ChooseSpec::Source)
                        && let Some(snapshot) = source_characteristic_lki_for_execution(game, ctx)
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
                                    source_characteristic_lki_for_execution(game, ctx)
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
                context.layer().counters_on(value, spec, counter_type)
            }
        }
        Value::TaggedSnapshotCount { tag, filter } => {
            let Some(ctx) = context.execution() else {
                return Err(ExecutionError::UnresolvableValue("snapshot count requires a resolution context".into()));
            };
            let filter_ctx = ctx.filter_context(game);
            // Persistent entry receipts are owned by the source incarnation;
            // a queued trigger's explicit frozen tags take precedence.
            let snapshots=if tag.as_str().starts_with("__entry_action_exiled_by__:") {
                crate::filter::source_entry_exiled_receipt(game,ctx.source,ctx.source_snapshot.as_ref(),tag.as_str())
            } else {ctx.get_tagged_all(tag).or_else(||
                tag.as_str().starts_with("__entry_action_").then(||filter_ctx.tagged_objects.get(tag)).flatten())};
            let count = snapshots.into_iter().flatten()
                .filter(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game)).count();
            Ok(i64::try_from(count).unwrap_or(i64::MAX))
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
        Value::GreatestEqualStoredDieResultsOnSource => Ok(i64::from(
            game.greatest_equal_stored_die_result_count(context.source),
        )),
        Value::GreatestManaValueExiledThisTurnSinceSourceEntered => Ok(i64::from(
            game.turn_store
                .turn_history
                .greatest_mana_value_exiled_this_turn_since_entry(context.source),
        )),
        // CR 700.3: one per distinct spell or ability that made players guess
        // or group cards or permanents into piles this turn.
        Value::SpellsOrAbilitiesThatMadePilesOrGuessesThisTurn => Ok(i64::try_from(
            game.turn_store
                .turn_history
                .pile_or_guess_instructions_this_turn
                .len(),
        )
        .unwrap_or(i64::MAX)),
        Value::IteratedPlayerChosenNumber => {
            let ctx = context.require_execution(
                value,
                "a participant's chosen number requires a resolving per-player number choice",
            );
            let player = ctx.iteration.iterated_player;
            Ok(ctx
                .secret_choice_results
                .get(&ctx.source)
                .and_then(|result| {
                    result
                        .number_choices
                        .iter()
                        .find(|(chooser, _)| Some(*chooser) == player)
                        .map(|(_, number)| i64::from(*number))
                })
                .unwrap_or(0))
        }
        Value::PlayersChosenNumberExtreme { highest, guess } => {
            let ctx = context.require_execution(
                value,
                "chosen-number extremes require a resolving per-player number choice",
            );
            let numbers = ctx
                .secret_choice_results
                .get(&ctx.source)
                .map(|result| {
                    (if *guess { &result.guesses } else { &result.number_choices })
                        .iter()
                        .map(|(_, number)| *number)
                });
            Ok(numbers
                .and_then(|numbers| if *highest { numbers.max() } else { numbers.min() })
                .map(i64::from)
                .unwrap_or(0))
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
/// CR 107.2/208.3: a noncreature permanent on the battlefield has no power or
/// toughness, so an effect using that undefined number uses zero.
fn noncreature_permanent_reference(
    game: &GameState,
    context: &EvaluationContext<'_, '_>,
    spec: &ChooseSpec,
) -> bool {
    if matches!(spec.base(), ChooseSpec::Tagged(_)) {
        return false;
    }
    let Some(ctx) = context.execution() else {
        return false;
    };
    let Ok(id) = resolve_primary_object_from_value_spec(game, spec, ctx) else {
        return false;
    };
    game.object(id).is_some_and(|object| {
        if object.zone == crate::zone::Zone::Battlefield {
            !game.current_is_creature(id)
        } else {
            // CR 208.3: off the battlefield a noncreature object has power and
            // toughness only if printed; with none, CR 107.2 uses zero.
            !game.current_is_creature(id) && game.current_power(id).is_none()
        }
    })
}

fn resolve_event_value(
    game: &GameState,
    ctx: &ExecutionContext,
    spec: &EventValueSpec,
) -> Result<i64, ExecutionError> {
    match spec {
        EventValueSpec::ManaSpentOnKeyword { colors } => {
            let event = ctx.triggering_event.as_ref()
                .and_then(|event| event.downcast::<crate::events::KeywordActionEvent>())
                .ok_or_else(|| ExecutionError::IncompleteEvidence(
                    "keyword payment quantity requires its completed action receipt".into()))?;
            let mut amount = 0i64;
            for color in [crate::color::Color::White, crate::color::Color::Blue,
                crate::color::Color::Black, crate::color::Color::Red, crate::color::Color::Green] {
                if colors.contains(color) {
                    amount = amount.saturating_add(i64::from(event.mana_spent.amount(crate::mana::ManaSymbol::from_color(color))));
                }
            }
            Ok(amount)
        }
        EventValueSpec::CastSpell(quantity) => ctx
            .triggering_event
            .as_ref()
            .and_then(|event| event.downcast::<crate::events::spells::SpellCastEvent>())
            .ok_or_else(|| {
                ExecutionError::IncompleteEvidence(
                    "cast quantity requires its completed spell-cast event".into(),
                )
            })?
            .cast_quantity(*quantity),
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
            Ok(i64::try_from(u128::from(amount)).unwrap_or(i64::MAX))
        }
        EventValueSpec::LoyaltyCountersRemovedToActivate => {
            let activation = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::AbilityActivatedEvent>())
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "loyalty cost value requires its ability-activated event".into(),
                    )
                })?;
            let Some(crate::ability::AbilityKind::Activated(activated)) =
                activation.activated_ability.as_ref().map(|ability| &ability.kind)
            else {
                return Ok(0);
            };
            let removed = match activated.mana_cost.kind() {
                ironsmith_core::TotalCostKind::All(costs) => costs
                    .iter()
                    .filter_map(|cost| {
                        // CR 606.4: a loyalty cost removes loyalty counters
                        // from the source itself.
                        let effect = cost.effect_ref()?;
                        let effect = effect
                            .downcast_ref::<crate::effects::WithIdEffect>()
                            .map_or(effect, |observed| &observed.effect);
                        let effect = effect.downcast_ref::<crate::effects::RemoveCountersEffect>()?;
                        if effect.counter_type != crate::CounterType::Loyalty
                            || !matches!(effect.target.base(), crate::target::ChooseSpec::Source)
                        {
                            return None;
                        }
                        let Value::Fixed(count) = &effect.count else {
                            return None;
                        };
                        Some((*count).max(0).unsigned_abs())
                    })
                    .fold(0u32, u32::saturating_add),
                _ => 0,
            };
            Ok(i64::from(removed))
        }
        EventValueSpec::ExcessDamage => {
            let damage = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::DamageEvent>())
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "excess damage requires its triggering damage event".into(),
                    )
                })?;
            Ok(i64::from(damage.excess_damage))
        }
        EventValueSpec::PermanentsSacrificedToActivate => {
            let activation = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::AbilityActivatedEvent>())
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "sacrifice cost count requires its ability-activated event".into(),
                    )
                })?;
            Ok(i64::from(activation.permanents_sacrificed_to_activate))
        }
        EventValueSpec::LifePaidToActivate => {
            let activation = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::AbilityActivatedEvent>())
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "life cost value requires its ability-activated event".into(),
                    )
                })?;
            // Fixed "Pay N life" components of the activation cost, as a
            // floor for the life the payment receipts recorded (which also
            // include life paid for Phyrexian mana, CR 107.4f).
            let fixed_cost_life = match activation.activated_ability.as_ref().map(|ability| &ability.kind) {
                Some(crate::ability::AbilityKind::Activated(activated)) => match activated.mana_cost.kind() {
                    ironsmith_core::TotalCostKind::All(costs) => costs
                        .iter()
                        .filter_map(|cost| {
                            let effect = cost.effect_ref()?;
                            let effect = effect
                                .downcast_ref::<crate::effects::WithIdEffect>()
                                .map_or(effect, |observed| &observed.effect);
                            let effect = effect.downcast_ref::<crate::effects::PayLifeEffect>()?;
                            let Value::Fixed(amount) = &effect.amount else {
                                return None;
                            };
                            Some((*amount).max(0).unsigned_abs())
                        })
                        .fold(0u32, u32::saturating_add),
                    _ => 0,
                },
                _ => 0,
            };
            Ok(i64::from(activation.life_paid_to_activate.max(fixed_cost_life)))
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
                EventValueSpec::DieBatchTotal => roll
                    .numeric_batch_results()
                    .map(u128::from)
                    .fold(0u128, u128::saturating_add),
                EventValueSpec::DieResultsAtLeast(minimum) => roll
                    .numeric_batch_results()
                    .filter(|result| i64::from(*result) >= i64::from(*minimum))
                    .count() as u128,
                _ => unreachable!(),
            };
            Ok(i64::try_from(amount).unwrap_or(i64::MAX))
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
            Ok(i64::from(roll.result))
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
                return Ok(i64::try_from(u128::from(payment.amount)).unwrap_or(i64::MAX));
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
                return Ok(i64::from(placement.count));
            }
            if triggering_event.downcast::<MarkersChangedEvent>().is_some() {
                return triggering_event.counter_trigger_amount();
            }
            if let Some(counter_event) = triggering_event.downcast::<CounterPlacedEvent>() {
                return Ok(i64::from(counter_event.amount as i64));
            }
            if let Some(zone_change_event) = triggering_event.downcast::<ZoneChangeEvent>() {
                return Ok(i64::from(zone_change_event.count() as i64));
            }
            if let Some(draw) = triggering_event.downcast::<crate::events::DrawEvent>() {
                return Ok(i64::from(draw.count));
            }
            if let Some(keyword_action_event) = triggering_event.downcast::<KeywordActionEvent>() {
                return Ok(i64::from(keyword_action_event.amount as i64));
            }
            if let Some(created) = triggering_event.downcast::<crate::events::CreateTokensEvent>() {
                return Ok(i64::try_from(created.total_count()).unwrap_or(i64::MAX));
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
                return Ok(beyond_first.saturating_mul(i64::from(*multiplier)));
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
