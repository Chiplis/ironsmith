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
fn generate_damage_triggers(
    game: &mut GameState,
    events: &[CombatDamageEvent],
    trigger_queue: &mut TriggerQueue,
) {
    game.clear_combat_damage_player_batch_hits();
    for event in events {
        let damage_target = match event.target {
            DamageEventTarget::Player(p) => EventDamageTarget::Player(p),
            DamageEventTarget::Object(o) => EventDamageTarget::Object(o),
        };
        let damage_event_provenance = game
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::Damage);
        let trigger_event = TriggerEvent::new_with_provenance(
            DamageEvent::new(
                event.source,
                damage_target,
                event.amount,
                true, // is_combat
            ),
            damage_event_provenance,
        );
        queue_triggers_from_event(game, trigger_queue, trigger_event, false);

        if let DamageEventTarget::Player(player_id) = event.target
            && event.life_lost > 0
        {
            let life_loss_event_provenance = game
                .provenance_graph
                .alloc_root_event(crate::events::EventKind::LifeLoss);
            let life_loss_event = TriggerEvent::new_with_provenance(
                LifeLossEvent::new(player_id, event.life_lost, true),
                life_loss_event_provenance,
            );
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
