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
                    crate::decisions::context::DecisionContext::Boolean(ref boolean_ctx) => {
                        runner.respond_boolean(decision_maker.decide_boolean(game, boolean_ctx));
                    }
                    crate::decisions::context::DecisionContext::SelectOptions(ref options_ctx) => {
                        runner.respond_options(decision_maker.decide_options(game, options_ctx));
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
    for event in crate::triggers::check::generate_step_trigger_events_for_active_players(game) {
        let event = game.ensure_trigger_event_provenance(event);
        queue_triggers_from_event(game, trigger_queue, event.clone(), true);
        queue_inherent_radiation_trigger(game, trigger_queue, &event);
    }
}

/// CR 728.1 gives rad counters an inherent, sourceless intervening-if trigger.
fn queue_inherent_radiation_trigger(
    game: &GameState,
    trigger_queue: &mut TriggerQueue,
    event: &TriggerEvent,
) {
    let Some(precombat_main) =
        event.downcast::<crate::events::phase::BeginningOfPrecombatMainPhaseEvent>()
    else {
        return;
    };
    let controller = precombat_main.player;
    if game
        .player(controller)
        .map_or(0, |player| player.counter_count(CounterType::Rad))
        == 0
    {
        return;
    }

    let ability = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::beginning_of_precombat_main_phase(
            crate::target::PlayerFilter::Any,
        ),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::RadiationEffect::new(),
        )]),
        choices: Vec::new(),
        intervening_if: Some(crate::ConditionExpr::PlayerHasCountersOrMore {
            player: crate::target::PlayerFilter::You,
            counter_type: CounterType::Rad,
            count: 1,
        }),
        presentation_label: None,
    };
    let trigger_identity = crate::triggers::compute_trigger_identity(&ability);
    let source = ObjectId::from_raw(u64::MAX - 2);
    trigger_queue.add(TriggeredAbilityEntry {
        source,
        controller,
        x_value: None,
        event_value_amount: None,
        ability,
        triggering_event: event.clone(),
        source_stable_id: StableId::from(source),
        source_name: "Rad counters".to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: crate::triggers::TriggeredAbilitySourceKind::GameRule,
        trigger_identity,
    });
}

/// Generate damage trigger events from combat damage.
pub(super) fn generate_damage_triggers(
    game: &mut GameState,
    events: &[CombatDamageEvent],
    trigger_queue: &mut TriggerQueue,
) {
    game.clear_combat_damage_player_batch_hits();
    game.clear_combat_damage_object_batch_hits();
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
        game.clear_combat_damage_object_batch_hits();
        return;
    }

    let mut damage_batch_groups = std::collections::HashMap::new();
    for event in events {
        let (damage_event, life_loss_event) = combat_damage_trigger_events(game, event);
        queue_incremental_combat_damage_event(
            game,
            trigger_queue,
            damage_event,
            &mut damage_batch_groups,
        );
        if let Some(life_loss_event) = life_loss_event {
            queue_triggers_from_event(game, trigger_queue, life_loss_event, false);
        }

        if let DamageEventTarget::Player(player_id) = event.target
            && event.amount > 0
        {
            game.record_combat_damage_player_batch_hit(event.source, player_id);
        }
        if let DamageEventTarget::Object(object_id) = event.target
            && event.amount > 0
        {
            game.record_combat_damage_object_batch_hit(event.source, object_id);
        }
    }
    game.clear_combat_damage_player_batch_hits();
    game.clear_combat_damage_object_batch_hits();
}

type DamageBatchTriggerKey = (
    StableId,
    crate::triggers::TriggerIdentity,
    crate::triggers::matcher_trait::SimultaneousTriggerKey,
);

fn queue_incremental_combat_damage_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    event: TriggerEvent,
    damage_batch_groups: &mut std::collections::HashMap<DamageBatchTriggerKey, Vec<usize>>,
) {
    let mut candidates = TriggerQueue::new();
    queue_triggers_from_event(game, &mut candidates, event, false);

    // A single event may legitimately match multiple identical ability
    // instances on the same object. Delay publishing their group indices until
    // the whole event has been handled, matching the simultaneous-event path.
    let mut groups_from_this_event: std::collections::HashMap<DamageBatchTriggerKey, Vec<usize>> =
        std::collections::HashMap::new();
    for candidate in candidates.entries {
        let Some(crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageBatch) = candidate
            .ability
            .trigger
            .simultaneous_trigger_key(&candidate.triggering_event)
        else {
            trigger_queue.add(candidate);
            continue;
        };
        let key = (
            candidate.source_stable_id,
            candidate.trigger_identity,
            crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageBatch,
        );

        if let Some(existing_indices) = damage_batch_groups.get(&key) {
            if let Some(amount) = candidate.event_value_amount {
                for index in existing_indices {
                    if let Some(existing) = trigger_queue.entries.get_mut(*index) {
                        existing.event_value_amount = Some(
                            existing
                                .event_value_amount
                                .map_or(amount, |prior| prior.max(amount)),
                        );
                    }
                }
            }
            continue;
        }

        let index = trigger_queue.entries.len();
        trigger_queue.add(candidate);
        groups_from_this_event.entry(key).or_default().push(index);
    }

    for (key, indices) in groups_from_this_event {
        damage_batch_groups.entry(key).or_default().extend(indices);
    }
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
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{EventValueSpec, Value};
    use crate::ids::CardId;
    use crate::target::PlayerFilter;
    use crate::triggers::Trigger;

    fn create_zombie(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

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

    #[test]
    fn incremental_combat_coalesces_damage_batch_and_keeps_ordinary_triggers() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let trigger_source = create_zombie(&mut game, "Hordewing", alice);
        let attacker_one = create_zombie(&mut game, "Attacker One", alice);
        let attacker_two = create_zombie(&mut game, "Attacker Two", alice);
        let zombie_you_control = ObjectFilter::creature()
            .with_subtype(Subtype::Zombie)
            .controlled_by(PlayerFilter::You);

        let source = game
            .object_mut(trigger_source)
            .expect("trigger source should exist");
        source.abilities_mut().push(Ability::triggered(
            Trigger::deals_combat_damage_to_player_one_or_more(
                zombie_you_control.clone(),
                PlayerFilter::Opponent,
            ),
            vec![Effect::draw(Value::EventValue(EventValueSpec::Amount))],
        ));
        source.abilities_mut().push(Ability::triggered(
            Trigger::deals_combat_damage_to_player(zombie_you_control, PlayerFilter::Opponent),
            vec![Effect::draw(1)],
        ));
        game.refresh_continuous_state();

        let events = vec![
            CombatDamageEvent {
                source: attacker_one,
                target: DamageEventTarget::Player(bob),
                amount: 2,
                life_lost: 2,
                result: DamageResult::default(),
            },
            CombatDamageEvent {
                source: attacker_two,
                target: DamageEventTarget::Player(charlie),
                amount: 2,
                life_lost: 2,
                result: DamageResult::default(),
            },
        ];
        let mut trigger_queue = TriggerQueue::new();

        generate_damage_triggers(&mut game, &events, &mut trigger_queue);

        let grouped = trigger_queue
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .ability
                    .trigger
                    .simultaneous_trigger_key(&entry.triggering_event)
                    == Some(crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageBatch)
            })
            .collect::<Vec<_>>();
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].event_value_amount, Some(2));
        assert_eq!(
            trigger_queue.entries.len() - grouped.len(),
            2,
            "ordinary per-event combat-damage triggers should not be coalesced"
        );
    }
}
