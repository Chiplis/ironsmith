use super::*;
use ironsmith::ability::Ability;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::effect::Restriction;
use ironsmith::game_state::{Phase, Step};
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::ObjectFilter;
use ironsmith::turn_runner::{TurnRunner, TurnState};

#[derive(Debug, Clone)]
struct FailedTriggerAnnouncement;

impl ironsmith::effects::EffectExecutor for FailedTriggerAnnouncement {
    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ironsmith::effects::EffectContext,
    ) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
        Ok(ironsmith::effect::EffectOutcome::count(0))
    }

    fn get_modal_spec_with_context(
        &self,
        game: &GameState,
        controller: PlayerId,
        source: ObjectId,
    ) -> Option<ironsmith::effects::ModalSpec> {
        use ironsmith::effect::{Condition, EffectId, EffectMetric, EffectMetricSource, Value};
        // Inject the original missing-receipt failure during trigger setup,
        // after attackers have already been tapped and their events queued.
        ironsmith::condition_eval::evaluate_condition_cast_time(
            game,
            &Condition::ValueComparison {
                left: Value::EffectMetric {
                    effect_id: EffectId(0),
                    source: EffectMetricSource::Outcome,
                    metric: EffectMetric::CoinFlipsWon,
                },
                operator: ironsmith_core::ValueComparisonOperator::Equal,
                right: Value::Fixed(5),
            },
            controller,
            source,
        );
        None
    }
}

fn creature(wasm: &mut WasmGame, controller: PlayerId, restricted: bool) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Combat retry probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(2, 2))
        .build();
    let id = wasm
        .game
        .create_object_from_definition(&definition, controller, Zone::Battlefield);
    if restricted {
        wasm.game.object_mut(id).unwrap().abilities =
            std::sync::Arc::new(vec![Ability::static_ability(StaticAbility::restriction(
                Restriction::AttackOrBlockAlone(ObjectFilter::source()),
                "This creature can't attack or block alone".into(),
            ))]);
    }
    wasm.game.remove_summoning_sickness(id);
    id
}

fn attack_command(creatures: &[ObjectId]) -> UiCommand {
    UiCommand::DeclareAttackers {
        declarations: creatures
            .iter()
            .map(|id| AttackerDeclarationInput {
                creature: id.0,
                target: AttackTargetInput::Player { player: 1 },
            })
            .collect(),
        bands: vec![],
    }
}

#[test]
fn rejected_lone_attacker_restores_prompt_and_accepts_corrected_declaration() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new_with_registry(CardRegistry::new());
    wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
    let restricted = creature(&mut wasm, PlayerId(0), true);
    let companion = creature(&mut wasm, PlayerId(0), false);
    wasm.game.turn.active_player = PlayerId(0);
    wasm.game.turn.phase = Phase::Combat;
    wasm.game.turn.step = Some(Step::DeclareAttackers);
    wasm.runner = Some(TurnRunner::from_state_for_sync(
        TurnState::DeclareAttackersDecision,
    ));
    wasm.advance_until_decision().unwrap();
    let ids_before = snapshot_id_counters();
    let context = wasm.pending_decision.take().unwrap();
    let DecisionContext::Attackers(attackers) = &context else {
        panic!("attackers prompt")
    };
    assert!(
        attackers
            .attacker_options
            .iter()
            .any(|option| option.creature == restricted)
    );
    wasm.pending_decision = Some(context);
    wasm.pending_decision_game = Some(Box::new(wasm.game.clone()));
    assert!(
        wasm.dispatch_routed_command(attack_command(&[restricted]), 0.0)
            .is_err()
    );
    assert!(wasm.pending_decision_game.is_some());
    assert!(matches!(
        wasm.pending_decision,
        Some(DecisionContext::Attackers(_))
    ));
    assert!(wasm.runner_pending_decision);
    assert!(matches!(
        wasm.runner.as_ref().unwrap().state(),
        TurnState::DeclareAttackersApply
    ));
    assert!(!wasm.game.is_tapped(restricted));
    assert!(!wasm.game.is_tapped(companion));
    assert!(wasm.trigger_queue.entries.is_empty());
    assert_eq!(snapshot_id_counters().object, ids_before.object);
    let context = wasm.pending_decision.take().unwrap();
    wasm.apply_runner_decision(context, attack_command(&[restricted, companion]))
        .unwrap();
    assert!(wasm.game.is_tapped(restricted));
    assert!(wasm.game.is_tapped(companion));
    assert_eq!(wasm.runner.as_ref().unwrap().combat().attackers.len(), 2);
    assert!(wasm.pending_decision.is_some());
}

#[test]
fn rejected_lone_blocker_restores_prompt_and_accepts_corrected_declaration() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new_with_registry(CardRegistry::new());
    wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
    let attacker = creature(&mut wasm, PlayerId(0), false);
    let restricted = creature(&mut wasm, PlayerId(1), true);
    let companion = creature(&mut wasm, PlayerId(1), false);
    wasm.game.turn.active_player = PlayerId(0);
    wasm.game.turn.phase = Phase::Combat;
    wasm.game.turn.step = Some(Step::DeclareAttackers);
    wasm.runner = Some(TurnRunner::from_state_for_sync(
        TurnState::DeclareAttackersDecision,
    ));
    wasm.advance_until_decision().unwrap();
    let context = wasm.pending_decision.take().unwrap();
    wasm.apply_runner_decision(context, attack_command(&[attacker]))
        .unwrap();
    // Move the completed attacking tenure to the blockers decision, retaining
    // the real combat declaration on both the game and the runner.
    let combat = wasm.runner.as_ref().unwrap().combat().clone();
    let mut runner = TurnRunner::from_state_for_sync(TurnState::DeclareBlockersCheck);
    *runner.combat_mut() = combat;
    wasm.runner = Some(runner);
    wasm.runner_awaiting_priority = false;
    wasm.advance_until_decision().unwrap();
    let context = wasm.pending_decision.take().unwrap();
    assert!(matches!(context, DecisionContext::Blockers(_)));
    let command = |blockers: &[ObjectId]| UiCommand::DeclareBlockers {
        declarations: blockers
            .iter()
            .map(|id| BlockerDeclarationInput {
                blocker: id.0,
                blocking: attacker.0,
            })
            .collect(),
    };
    wasm.pending_decision = Some(context);
    assert!(
        wasm.dispatch_routed_command(command(&[restricted]), 0.0)
            .is_err()
    );
    assert!(matches!(
        wasm.pending_decision,
        Some(DecisionContext::Blockers(_))
    ));
    assert!(wasm.runner_pending_decision);
    assert!(wasm.runner.as_ref().unwrap().combat().blockers.is_empty());
    let context = wasm.pending_decision.take().unwrap();
    wasm.apply_runner_decision(context, command(&[restricted, companion]))
        .unwrap();
    assert_eq!(
        wasm.runner.as_ref().unwrap().combat().blockers[&attacker].len(),
        2
    );
    assert!(wasm.pending_decision.is_some());
}

#[test]
fn failed_attack_trigger_rewinds_declaration_and_allows_no_attackers_retry() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new_with_registry(CardRegistry::new());
    wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
    let attacker = creature(&mut wasm, PlayerId(0), false);
    wasm.game.object_mut(attacker).unwrap().abilities =
        std::sync::Arc::new(vec![Ability::triggered(
            ironsmith::triggers::Trigger::this_attacks(),
            ironsmith::resolution::ResolutionProgram::from_effects(vec![
                ironsmith::effect::Effect::new(FailedTriggerAnnouncement),
            ]),
        )]);
    wasm.game.turn.active_player = PlayerId(0);
    wasm.game.turn.phase = Phase::Combat;
    wasm.game.turn.step = Some(Step::DeclareAttackers);
    wasm.runner = Some(TurnRunner::from_state_for_sync(
        TurnState::DeclareAttackersDecision,
    ));
    wasm.advance_until_decision().unwrap();
    let ids = snapshot_id_counters();
    let context = wasm.pending_decision.take().unwrap();
    wasm.pending_decision = Some(context);
    assert!(
        wasm.dispatch_routed_command(attack_command(&[attacker]), 0.0)
            .is_err()
    );
    assert!(!wasm.game.is_tapped(attacker));
    assert!(wasm.game.stack.is_empty());
    assert!(wasm.trigger_queue.entries.is_empty());
    assert!(wasm.runner.as_ref().unwrap().combat().attackers.is_empty());
    assert_eq!(snapshot_id_counters().object, ids.object);
    assert_eq!(snapshot_id_counters().card, ids.card);
    let context = wasm.pending_decision.take().unwrap();
    assert!(matches!(context, DecisionContext::Attackers(_)));
    wasm.apply_runner_decision(context, attack_command(&[]))
        .unwrap();
    assert!(wasm.pending_decision.is_some());
}
