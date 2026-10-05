use super::*;
use crate::target::ObjectFilter;
use ironsmith_core::{
    DamageHistoryQuery, DamageHistoryRecipients, DamageHistoryReduction, DamageHistorySources,
};

/// Numeric history reads the bound identity, not whether it can still be
/// selected for a new operation. It never follows stable card identity.
fn referenced_objects(
    game: &GameState,
    ctx: &ExecutionContext,
    spec: &ChooseSpec,
) -> Result<Vec<ObjectId>, ExecutionError> {
    let objects = match spec.base() {
        ChooseSpec::Source => vec![
            ctx.source_snapshot
                .as_ref()
                .map_or(ctx.source, |snapshot| snapshot.object_id),
        ],
        ChooseSpec::SpecificObject(id) => vec![*id],
        ChooseSpec::Tagged(tag) => ctx
            .get_tagged_all(&format!("__pre_move_history__{}", tag.as_str()))
            .or_else(|| ctx.get_tagged_all(tag))
            .map(|objects| objects.iter().map(|snapshot| snapshot.object_id).collect())
            .unwrap_or_default(),
        ChooseSpec::Object(_) | ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => {
            matching_object_targets_for_spec(game, spec, ctx)
        }
        _ => {
            return Err(ExecutionError::UnresolvableValue(
                "damage history requires an exact bound object reference".into(),
            ));
        }
    };
    if objects.is_empty() {
        return Err(ExecutionError::UnresolvableValue(
            "damage history object reference is unavailable".into(),
        ));
    }
    Ok(objects)
}

pub(super) fn resolve_damage_history(
    query: &DamageHistoryQuery,
    context: &EvaluationContext<'_, '_>,
) -> Result<i32, ExecutionError> {
    if let Some(ctx) = context.execution() {
        return resolve_history_in_context(context.game, query, ctx);
    }
    // Pure current-source/history queries are also readable by a CDA. A
    // resolution-only target/tag must still have a real binding; the helper
    // below rejects it rather than guessing a source or inventing zero.
    let ctx = ExecutionContext::new_default(context.source, context.controller);
    resolve_history_in_context(context.game, query, &ctx)
}

pub(crate) fn resolve_damage_history_for_comparison(
    game: &GameState,
    query: &DamageHistoryQuery,
    ctx: &ExecutionContext,
) -> Result<i64, ExecutionError> {
    i64::try_from(resolve_history_amount(game, query, ctx)?).map_err(|_| {
        ExecutionError::UnresolvableValue(
            "damage history comparison is outside the supported integer range".into(),
        )
    })
}

fn resolve_history_in_context(
    game: &GameState,
    query: &DamageHistoryQuery,
    ctx: &ExecutionContext,
) -> Result<i32, ExecutionError> {
    i32::try_from(resolve_history_amount(game, query, ctx)?).map_err(|_| {
        ExecutionError::UnresolvableValue(
            "damage history amount is outside the supported integer range".into(),
        )
    })
}

/// In a historical source/recipient class, "other" excludes the exact
/// resolving source. Ordinary selection filters also exclude announced targets
/// and follow stable card identities; neither rule belongs in this history query.
fn history_filter_matches(
    game: &GameState,
    filter: &ObjectFilter,
    snapshot: &crate::snapshot::ObjectSnapshot,
    ctx: &crate::filter::FilterContext,
    source: ObjectId,
) -> bool {
    if filter.other {
        if snapshot.object_id == source {
            return false;
        }
        let mut filter = filter.clone();
        filter.other = false;
        filter.matches_snapshot(snapshot, ctx, game)
    } else {
        filter.matches_snapshot(snapshot, ctx, game)
    }
}

fn resolve_history_amount(
    game: &GameState,
    query: &DamageHistoryQuery,
    ctx: &ExecutionContext,
) -> Result<u64, ExecutionError> {
    let filter_ctx = ctx.filter_context(game);
    let source_ids = match &query.sources {
        DamageHistorySources::Reference(spec) => Some(referenced_objects(game, ctx, spec)?),
        DamageHistorySources::SourceAttachedObject => {
            let attached =
                source_attachment_target_with_lki(game, ctx.source, ctx.source_snapshot.as_ref())
                    .and_then(|attached| attached.object_id());
            Some(vec![attached.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "damage history attached source is unavailable".into(),
                )
            })?])
        }
        _ => None,
    };
    let recipient_ids = match &query.recipients {
        DamageHistoryRecipients::Reference(spec) => Some(referenced_objects(game, ctx, spec)?),
        _ => None,
    };
    let overflow = || {
        ExecutionError::UnresolvableValue(
            "damage history amount is outside the supported integer range".into(),
        )
    };
    let mut total = 0u64;
    let mut largest_occurrence = 0u64;
    let mut sources = std::collections::HashMap::<ObjectId, u64>::new();
    for record in game.turn_store.turn_history.projected_records() {
        let Some(damage) = record.event.downcast::<crate::events::DamageEvent>() else {
            continue;
        };
        if damage.amount == 0
            || query
                .combat
                .is_some_and(|combat| combat != damage.is_combat)
        {
            continue;
        }
        if source_ids
            .as_ref()
            .is_some_and(|ids| !ids.contains(&damage.source))
        {
            continue;
        }
        if let DamageHistorySources::Matching(filter) = &query.sources {
            let snapshot = record
                .event
                .source_snapshot()
                .or(record.source_snapshot.as_ref())
                .filter(|snapshot| snapshot.object_id == damage.source)
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "damage history has no source characteristic receipt".into(),
                    )
                })?;
            if !history_filter_matches(game, filter, snapshot, &filter_ctx, ctx.source) {
                continue;
            }
        }
        let matches_recipient = match &query.recipients {
            DamageHistoryRecipients::Any => true,
            DamageHistoryRecipients::Reference(_) => {
                matches!(damage.target, crate::events::DamageTarget::Object(id) if recipient_ids.as_ref().is_some_and(|ids| ids.contains(&id)))
            }
            DamageHistoryRecipients::Players(players) => {
                matches!(damage.target,crate::events::DamageTarget::Player(id) if crate::filter::player_filter_matches_game(players,id,game,&filter_ctx))
            }
            DamageHistoryRecipients::MatchingObjects(filter) => {
                if let crate::events::DamageTarget::Object(id) = damage.target {
                    let snapshot = damage
                        .target_snapshot
                        .as_ref()
                        .filter(|snapshot| snapshot.object_id == id)
                        .ok_or_else(|| {
                            ExecutionError::UnresolvableValue(
                                "damage history has no recipient characteristic receipt".into(),
                            )
                        })?;
                    history_filter_matches(game, filter, snapshot, &filter_ctx, ctx.source)
                } else {
                    false
                }
            }
        };
        if !matches_recipient {
            continue;
        }
        if query.reduction == DamageHistoryReduction::LargestSourceRecipientOccurrence {
            let amount = match damage.completed_source_recipient_amount(query.combat) {
                Some(amount) => amount,
                // A legacy single unbatched receipt is its one occurrence.
                // Missing coalescing evidence for a grouped receipt is unknown.
                None if record.event.simultaneous_batch().is_none() => u128::from(damage.amount),
                None => {
                    return Err(ExecutionError::UnresolvableValue(
                        "damage history has no completed source/recipient occurrence receipt"
                            .into(),
                    ));
                }
            };
            largest_occurrence =
                largest_occurrence.max(u64::try_from(amount).map_err(|_| overflow())?);
        }
        let amount = u64::from(damage.amount);
        total = total.checked_add(amount).ok_or_else(overflow)?;
        let source_total = sources.entry(damage.source).or_default();
        *source_total = source_total.checked_add(amount).ok_or_else(overflow)?;
    }
    let amount = match query.reduction {
        DamageHistoryReduction::LargestSourceRecipientOccurrence => largest_occurrence,
        DamageHistoryReduction::Total => total,
        DamageHistoryReduction::LargestSourceTotal => sources.values().copied().max().unwrap_or(0),
        DamageHistoryReduction::DistinctSources => {
            u64::try_from(sources.len()).map_err(|_| overflow())?
        }
    };
    Ok(amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::CardId;
    use crate::types::CardType;
    fn object(game: &mut GameState, player: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 40))
            .build();
        game.create_object_from_card(&card, player, Zone::Battlefield)
    }
    fn apply(game: &mut GameState, source: ObjectId, effect: crate::effect::Effect) {
        let controller = game.current_controller(source).unwrap();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        crate::effects::execute_effect(
            game,
            &effect,
            &mut ExecutionContext::new(source, controller, &mut dm),
        )
        .unwrap();
    }
    fn damage(game: &mut GameState, source: ObjectId, target: ObjectId, amount: i32, combat: bool) {
        let mut damage =
            crate::effects::DealDamageEffect::new(amount, ChooseSpec::SpecificObject(target));
        damage.source_is_combat = combat;
        apply(game, source, crate::effect::Effect::new(damage));
    }
    fn total(recipient: ObjectId) -> DamageHistoryQuery {
        DamageHistoryQuery {
            sources: DamageHistorySources::Any,
            recipients: DamageHistoryRecipients::Reference(Box::new(ChooseSpec::SpecificObject(
                recipient,
            ))),
            combat: None,
            reduction: DamageHistoryReduction::Total,
        }
    }
    #[test]
    fn actual_damage_history_survives_healing_but_excludes_prevention_and_later_incarnations() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = object(&mut game, alice, "Damage source");
        let target = object(&mut game, bob, "History recipient");
        let ctx = ExecutionContext::new_default(source, alice);
        apply(
            &mut game,
            source,
            crate::effect::Effect::prevent_damage(
                3,
                ChooseSpec::SpecificObject(target),
                crate::effect::Until::EndOfTurn,
            ),
        );
        damage(&mut game, source, target, 5, false);
        assert_eq!(game.damage_on(target), 2);
        assert_eq!(
            resolve_history_in_context(&game, &total(target), &ctx).unwrap(),
            2
        );
        apply(
            &mut game,
            source,
            crate::effect::Effect::regenerate(
                ChooseSpec::SpecificObject(target),
                crate::effect::Until::EndOfTurn,
            ),
        );
        apply(
            &mut game,
            source,
            crate::effect::Effect::destroy(ChooseSpec::SpecificObject(target)),
        );
        assert_eq!(game.damage_on(target), 0);
        damage(&mut game, source, target, 3, true);
        assert_eq!(
            resolve_history_in_context(&game, &total(target), &ctx).unwrap(),
            5
        );
        let mut combat = total(target);
        combat.combat = Some(true);
        assert_eq!(resolve_history_in_context(&game, &combat, &ctx).unwrap(), 3);
        combat.combat = Some(false);
        assert_eq!(resolve_history_in_context(&game, &combat, &ctx).unwrap(), 2);
        let graveyard = game
            .move_object_by_game_rule(target, Zone::Graveyard)
            .unwrap();
        let returned = game
            .move_object_by_game_rule(graveyard, Zone::Battlefield)
            .unwrap();
        assert_eq!(
            resolve_history_in_context(&game, &total(target), &ctx).unwrap(),
            5
        );
        assert_eq!(
            resolve_history_in_context(&game, &total(returned), &ctx).unwrap(),
            0
        );
        game.turn_store.turn_history.clear_for_new_turn();
        assert_eq!(
            resolve_history_in_context(&game, &total(target), &ctx).unwrap(),
            0
        );
    }
    #[test]
    fn source_filters_use_damage_time_controller_and_reductions_distinguish_totals_from_sources() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let first = object(&mut game, alice, "First source");
        let second = object(&mut game, alice, "Second source");
        let target = object(&mut game, bob, "History recipient");
        damage(&mut game, first, target, 3, false);
        damage(&mut game, first, target, 2, false);
        damage(&mut game, second, target, 4, false);
        game.set_current_controller(first, bob).unwrap();
        damage(&mut game, first, target, 7, false);
        let mut query = total(target);
        query.sources = DamageHistorySources::Matching(ObjectFilter::default().you_control());
        let ctx = ExecutionContext::new_default(second, alice);
        assert_eq!(resolve_history_in_context(&game, &query, &ctx).unwrap(), 9);
        query.reduction = DamageHistoryReduction::LargestSourceTotal;
        assert_eq!(resolve_history_in_context(&game, &query, &ctx).unwrap(), 5);
        query.reduction = DamageHistoryReduction::DistinctSources;
        assert_eq!(resolve_history_in_context(&game, &query, &ctx).unwrap(), 2);
        let bob_ctx = ExecutionContext::new_default(first, bob);
        query.reduction = DamageHistoryReduction::Total;
        assert_eq!(
            resolve_history_in_context(&game, &query, &bob_ctx).unwrap(),
            7
        );
        let graveyard = game
            .move_object_by_game_rule(first, Zone::Graveyard)
            .unwrap();
        let returned = game
            .move_object_by_game_rule(graveyard, Zone::Battlefield)
            .unwrap();
        game.set_current_controller(returned, alice).unwrap();
        damage(&mut game, returned, target, 1, false);
        query.reduction = DamageHistoryReduction::DistinctSources;
        assert_eq!(resolve_history_in_context(&game, &query, &ctx).unwrap(), 3);
    }
    #[test]
    fn comparison_widens_actual_totals_but_scalar_overflow_is_explicit() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = object(&mut game, alice, "Wide damage source");
        let target = object(&mut game, bob, "Wide recipient");
        damage(&mut game, source, target, i32::MAX, false);
        damage(&mut game, source, target, 1, false);
        let ctx = ExecutionContext::new_default(source, alice);
        let query = total(target);
        assert_eq!(
            resolve_damage_history_for_comparison(&game, &query, &ctx).unwrap(),
            i64::from(i32::MAX) + 1
        );
        assert!(resolve_history_in_context(&game, &query, &ctx).is_err());
        let condition = crate::effect::Condition::ValueComparison {
            left: Value::DamageHistory(Box::new(query)),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(4),
        };
        assert!(
            crate::condition_eval::evaluate_condition_resolution(&game, &condition, &ctx).unwrap()
        );
    }
    #[test]
    fn exact_tagged_history_survives_departure_and_other_does_not_follow_card_identity() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = object(&mut game, alice, "Named source");
        let target = object(&mut game, bob, "Recipient");
        damage(&mut game, source, target, 3, false);
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(target).unwrap(), &game);
        let old_source = game.move_object_by_game_rule(source, Zone::Exile).unwrap();
        let source = game
            .move_object_by_game_rule(old_source, Zone::Battlefield)
            .unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("remembered_recipient", snapshot);
        let query = DamageHistoryQuery {
            sources: DamageHistorySources::Matching(
                ObjectFilter::default().other().named("Named source"),
            ),
            recipients: DamageHistoryRecipients::Reference(Box::new(ChooseSpec::Tagged(
                "remembered_recipient".into(),
            ))),
            combat: None,
            reduction: DamageHistoryReduction::Total,
        };
        assert_eq!(resolve_history_in_context(&game, &query, &ctx).unwrap(), 3);
        let moved = game.move_object_by_game_rule(target, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_game_rule(moved, Zone::Battlefield)
            .unwrap();
        damage(&mut game, source, returned, 5, false);
        assert_eq!(resolve_history_in_context(&game, &query, &ctx).unwrap(), 3);
        let no_binding = ExecutionContext::new_default(source, alice);
        assert!(resolve_history_in_context(&game, &query, &no_binding).is_err());
    }
    #[test]
    fn current_source_history_agrees_in_execution_and_continuous_contexts() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = object(&mut game, alice, "History source");
        let target = object(&mut game, bob, "Recipient");
        damage(&mut game, source, target, 4, false);
        let query = DamageHistoryQuery {
            sources: DamageHistorySources::Reference(Box::new(ChooseSpec::Source)),
            recipients: DamageHistoryRecipients::Any,
            combat: None,
            reduction: DamageHistoryReduction::Total,
        };
        let value = Value::DamageHistory(Box::new(query));
        let ctx = ExecutionContext::new_default(source, alice);
        let expected = crate::effects::helpers::resolve_value(&game, &value, &ctx).unwrap();
        assert_eq!(expected, 4);
        assert_eq!(
            crate::continuous::resolve_value_direct(
                &value,
                game.objects_map(),
                &[],
                &game.battlefield,
                &std::collections::HashSet::new(),
                source,
                alice,
                &game
            ),
            expected
        );
        game.turn_store.turn_history.clear_for_new_turn();
        assert_eq!(
            crate::continuous::resolve_value_direct(
                &value,
                game.objects_map(),
                &[],
                &game.battlefield,
                &std::collections::HashSet::new(),
                source,
                alice,
                &game
            ),
            0
        );
    }
    #[test]
    fn history_arithmetic_rejects_final_unsigned_overflow_before_applying_damage() {
        for received in [i32::MAX - 2, i32::MAX - 1, i32::MAX] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = object(&mut game, alice, "History arithmetic source");
            let history_recipient = object(&mut game, bob, "History recipient");
            let future_recipient = object(&mut game, bob, "Future recipient");
            damage(&mut game, source, history_recipient, received, false);
            let value = Value::Add(
                Box::new(Value::Fixed(6)),
                Box::new(Value::Scaled(
                    Box::new(Value::DamageHistory(Box::new(total(history_recipient)))),
                    2,
                )),
            );
            let mut ctx = ExecutionContext::new_default(source, alice);
            assert!(crate::effects::helpers::resolve_value(&game, &value, &ctx).is_err());
            let records = game.turn_store.turn_history.projected_records().count();
            let effect = crate::effect::Effect::new(crate::effects::DealDamageEffect::new(
                value,
                ChooseSpec::SpecificObject(future_recipient),
            ));
            let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
            assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
            assert_eq!(game.damage_on(future_recipient), 0);
            assert_eq!(
                game.turn_store.turn_history.projected_records().count(),
                records
            );
        }
    }
    #[test]
    fn independent_damage_instructions_retain_distinct_maxima_under_one_outer_action() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let dealer = object(&mut game, a, "Dealer");
        let recipient = object(&mut game, b, "Recipient");
        let ctx = ExecutionContext::new_default(dealer, a);
        let mut query = total(recipient);
        query.reduction = DamageHistoryReduction::LargestSourceRecipientOccurrence;
        let open = game.open_simultaneous_action();
        let mut reports = Vec::new();
        for amount in [2, 3] {
            let outcome = crate::effects::execute_effect(
                &mut game,
                &crate::effect::Effect::deal_damage(amount, ChooseSpec::SpecificObject(recipient)),
                &mut ExecutionContext::new_default(dealer, a),
            )
            .unwrap();
            reports.extend(outcome.events);
        }
        assert_eq!(
            resolve_history_in_context(&game, &query, &ctx).unwrap(),
            3,
            "staged history needs the original completion totals even while matching is held"
        );
        game.close_simultaneous_action(open);
        crate::game_loop::queue_triggers_from_reported_events(
            &mut game,
            &mut crate::triggers::TriggerQueue::new(),
            reports,
            true,
        );
        assert_eq!(resolve_history_in_context(&game, &query, &ctx).unwrap(), 3);
        query.reduction = DamageHistoryReduction::LargestSourceTotal;
        assert_eq!(resolve_history_in_context(&game, &query, &ctx).unwrap(), 5);
    }
}
