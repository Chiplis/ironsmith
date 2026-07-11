use super::*;

// ============================================================================
// Full Turn Execution
// ============================================================================

/// Execute a complete turn using a DecisionMaker.
///
/// This is the full-featured version that properly handles all combat decisions.
pub fn execute_turn_with(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    use crate::turn_runner::{TurnAction, TurnRunner};

    let mut runner = TurnRunner::new();

    loop {
        match runner.advance(game, trigger_queue)? {
            TurnAction::Continue => continue,

            TurnAction::RunPriority => {
                run_priority_loop_with(game, trigger_queue, decision_maker)?;
                runner.priority_done();
            }

            TurnAction::Decision(ctx) => {
                match ctx {
                    crate::decisions::context::DecisionContext::Attackers(ref actx) => {
                        let declarations: Vec<crate::decision::AttackerDeclaration> =
                            decision_maker
                                .decide_attackers(game, actx)
                                .into_iter()
                                .map(|d| crate::decision::AttackerDeclaration {
                                    creature: d.creature,
                                    target: d.target,
                                })
                                .collect();
                        runner.respond_attackers(declarations);
                    }
                    crate::decisions::context::DecisionContext::Blockers(ref bctx) => {
                        let defending_player = bctx.player;
                        let declarations: Vec<crate::decision::BlockerDeclaration> = decision_maker
                            .decide_blockers(game, bctx)
                            .into_iter()
                            .map(|d| crate::decision::BlockerDeclaration {
                                blocker: d.blocker,
                                blocking: d.blocking,
                            })
                            .collect();
                        runner.respond_blockers(declarations, defending_player);
                    }
                    crate::decisions::context::DecisionContext::SelectObjects(ref obj_ctx) => {
                        let cards = decision_maker.decide_objects(game, obj_ctx);
                        runner.respond_discard(cards);
                    }
                    _ => {
                        // Other decision types shouldn't appear during turn execution
                    }
                }
            }

            TurnAction::TurnComplete => {
                // Sync the runner's combat state back to the caller's combat ref
                *combat = runner.combat().clone();
                return Ok(());
            }

            TurnAction::GameOver(_) => {
                *combat = runner.combat().clone();
                return Err(GameLoopError::GameOver);
            }
        }
    }
}

/// Generate step trigger events and add them to the queue.
pub fn generate_and_queue_step_triggers(game: &mut GameState, trigger_queue: &mut TriggerQueue) {
    if let Some(event) = generate_step_trigger_events(game) {
        queue_triggers_from_event(game, trigger_queue, event, true);
    }
}

/// Generate damage trigger events from combat damage.
pub(super) fn generate_damage_triggers(
    game: &mut GameState,
    events: &[CombatDamageEvent],
    trigger_queue: &mut TriggerQueue,
) {
    game.clear_combat_damage_player_batch_hits();
    if events.is_empty() {
        return;
    }

    // The common large-board case has no damage/life-loss subscribers or
    // designation state whose matching depends on earlier events in this
    // batch. Build and check all of its events against one stable derived view
    // and trigger registry. Keep the ordered path below for mechanics whose
    // existing semantics intentionally update transient state between hits.
    if can_batch_combat_damage_trigger_events(game) {
        let mut trigger_events = Vec::with_capacity(events.len().saturating_mul(2));
        for event in events {
            let (damage_event, life_loss_event) = combat_damage_trigger_events(game, event);
            trigger_events.push(damage_event);
            trigger_events.extend(life_loss_event);
        }
        queue_triggers_for_simultaneous_events(game, trigger_queue, trigger_events);
        game.clear_combat_damage_player_batch_hits();
        return;
    }

    for event in events {
        let (damage_event, life_loss_event) = combat_damage_trigger_events(game, event);
        queue_triggers_from_event(game, trigger_queue, damage_event, false);
        if let Some(life_loss_event) = life_loss_event {
            queue_triggers_from_event(game, trigger_queue, life_loss_event, false);
        }

        if let DamageEventTarget::Player(player_id) = event.target
            && event.amount > 0
        {
            game.record_combat_damage_player_batch_hit(event.source, player_id);
        }
    }
    game.clear_combat_damage_player_batch_hits();
}

fn can_batch_combat_damage_trigger_events(game: &GameState) -> bool {
    !game.may_have_triggered_abilities_for_event_kind(crate::events::EventKind::Damage)
        && !game.may_have_triggered_abilities_for_event_kind(crate::events::EventKind::LifeLoss)
        && game.initiative.is_none()
        && !matches!(game.player_speed(game.turn.active_player), Some(1..=3))
}

fn combat_damage_trigger_events(
    game: &mut GameState,
    event: &CombatDamageEvent,
) -> (TriggerEvent, Option<TriggerEvent>) {
    let damage_target = match event.target {
        DamageEventTarget::Player(p) => EventDamageTarget::Player(p),
        DamageEventTarget::Object(o) => EventDamageTarget::Object(o),
    };
    let damage_event_provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::Damage);
    let cause = game
        .object(event.source)
        .map(|obj| {
            crate::events::cause::EventCause::from_combat_damage(
                event.source,
                game.controller_of(obj),
            )
        })
        .unwrap_or_else(crate::events::cause::EventCause::effect);
    let mut damage_event = DamageEvent::with_cause(
        event.source,
        damage_target,
        event.amount,
        true, // is_combat
        cause,
    );
    if let DamageEventTarget::Object(object_id) = event.target
        && let Some(obj) = game.object(object_id)
    {
        damage_event = damage_event.with_target_snapshot(
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game),
        );
    }
    let damage_event = TriggerEvent::new_with_provenance(damage_event, damage_event_provenance);

    let life_loss_event = match event.target {
        DamageEventTarget::Player(player_id) if event.life_lost > 0 => {
            let provenance = game
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::LifeLoss);
            Some(TriggerEvent::new_with_provenance(
                LifeLossEvent::new(player_id, event.life_lost, true),
                provenance,
            ))
        }
        _ => None,
    };

    (damage_event, life_loss_event)
}

/// Queue combat-damage and life-loss triggers for a batch of combat damage events.
///
/// This is shared by different runtime frontends (CLI/WASM) so they can execute
/// combat damage in step actions while keeping trigger emission consistent.
pub fn queue_combat_damage_triggers(
    game: &mut GameState,
    events: &[CombatDamageEvent],
    trigger_queue: &mut TriggerQueue,
) {
    generate_damage_triggers(game, events, trigger_queue);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscriber_free_combat_damage_batch_reuses_one_trigger_view() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let bob = PlayerId::from_index(1);
        let mut trigger_queue = TriggerQueue::new();
        game.refresh_continuous_state();
        assert!(can_batch_combat_damage_trigger_events(&game));
        let events = vec![
            CombatDamageEvent {
                source: ObjectId::from_raw(101),
                target: DamageEventTarget::Player(bob),
                amount: 3,
                life_lost: 3,
                result: DamageResult::default(),
            },
            CombatDamageEvent {
                source: ObjectId::from_raw(102),
                target: DamageEventTarget::Player(bob),
                amount: 4,
                life_lost: 4,
                result: DamageResult::default(),
            },
        ];
        let before = game.work_counters();

        generate_damage_triggers(&mut game, &events, &mut trigger_queue);

        let after = game.work_counters();
        assert_eq!(
            after.derived_view_rebuilds - before.derived_view_rebuilds,
            1
        );
        assert_eq!(
            game.trigger_event_kind_count_this_turn(crate::events::EventKind::Damage),
            2
        );
        assert_eq!(
            game.trigger_event_kind_count_this_turn(crate::events::EventKind::LifeLoss),
            2
        );
        assert_eq!(game.turn_store.turn_history.total_damage_to_player(bob), 7);
        assert!(trigger_queue.entries.is_empty());
    }

    #[test]
    fn initiative_keeps_incremental_combat_damage_trigger_path() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        game.initiative = Some(PlayerId::from_index(1));

        assert!(!can_batch_combat_damage_trigger_events(&game));
    }
}
