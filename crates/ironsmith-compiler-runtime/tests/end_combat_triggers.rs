use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::game_loop::{
    generate_and_queue_step_triggers, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_core::trigger_model::TriggerKind;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (artifact, direct) =
        compile_to_artifact(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/end_combat_triggers.json.fixture"
    ))
    .unwrap()
}

fn fixture(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|card| card["name"] == name)
        .unwrap();
    definitions(name, row["text"].as_str().unwrap())
}

fn stack(game: &mut GameState, queue: &mut TriggerQueue) {
    put_triggers_on_stack_with_dm(game, queue, &mut SelectFirstDecisionMaker).unwrap();
}

fn resolve(game: &mut GameState, queue: &mut TriggerQueue) {
    for _ in 0..20 {
        stack(game, queue);
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    panic!("end-combat work did not settle");
}

#[test]
fn end_combat_all_six_exact_oracle_cards_have_typed_round_tripped_events() {
    assert_eq!(fixtures().len(), 6);
    for card in fixtures() {
        for definition in definitions(
            card["name"].as_str().unwrap(),
            card["text"].as_str().unwrap(),
        ) {
            assert!(definition.abilities.iter().any(|ability| {
                matches!(&ability.kind, AbilityKind::Triggered(triggered)
                    if triggered.trigger.compiled_model().is_some_and(|model| model.kind == TriggerKind::EndOfCombat))
            }), "{} must have the actual end-of-combat event", definition.name());
        }
    }
}

#[test]
fn end_combat_fires_at_each_actual_end_combat_step_and_keeps_its_controller() {
    for definition in definitions(
        "Combat event probe",
        "Type: Enchantment\nAt end of combat, you gain 1 life.",
    ) {
        for active in [A, B] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
            game.turn.active_player = active;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::BeginCombat);
            let mut queue = TriggerQueue::new();
            loop {
                generate_and_queue_step_triggers(&mut game, &mut queue);
                if game.turn.step == Some(Step::EndCombat) {
                    break;
                }
                assert!(
                    queue.entries.is_empty(),
                    "the phase event must not fire early"
                );
                ironsmith::turn::advance_step(&mut game).unwrap();
            }
            assert_eq!(queue.entries.len(), 1);
            stack(&mut game, &mut queue);
            assert_eq!(game.stack.len(), 1);
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            resolve(&mut game, &mut queue);
            assert_eq!(
                game.player(B).unwrap().life,
                21,
                "removing the source does not remove its queued ability"
            );
            assert_eq!(game.player(A).unwrap().life, 20);
            ironsmith::turn::advance_step(&mut game).unwrap();
            assert_eq!(game.turn.phase, Phase::NextMain);
            generate_and_queue_step_triggers(&mut game, &mut queue);
            assert!(queue.entries.is_empty());
        }
    }
}

struct CombatBoard {
    game: GameState,
    source: ironsmith::ObjectId,
    partner: ironsmith::ObjectId,
    unrelated_attacker: ironsmith::ObjectId,
    unrelated_blocker: ironsmith::ObjectId,
}

fn combat_board(definition: &CardDefinition, source_attacks: bool) -> CombatBoard {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration};
    use ironsmith::game_loop::{
        apply_attacker_declarations, apply_blocker_declarations, execute_combat_damage_step,
    };
    use ironsmith::{CardId, CardType};

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 4;
    game.turn.active_player = A;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    game.mark_combat_phase_started();
    let source = game.create_object_from_definition(
        definition,
        if source_attacks { A } else { B },
        Zone::Battlefield,
    );
    let mut vanilla = |name: &str, player: PlayerId| {
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(0, 10))
                .build(),
            player,
            Zone::Battlefield,
        )
    };
    let partner = vanilla("Source combat partner", if source_attacks { B } else { A });
    let unrelated_attacker = vanilla("Other attacker", A);
    let unrelated_blocker = vanilla("Other blocker", B);
    for id in [source, partner, unrelated_attacker, unrelated_blocker] {
        game.remove_summoning_sickness(id);
    }
    let attacker = if source_attacks { source } else { partner };
    let blocker = if source_attacks { partner } else { source };
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut queue,
        &[
            AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(B),
            },
            AttackerDeclaration {
                creature: unrelated_attacker,
                target: AttackTarget::Player(B),
            },
        ],
    )
    .unwrap();
    game.combat = Some(combat.clone());
    resolve(&mut game, &mut queue);
    ironsmith::turn::advance_step(&mut game).unwrap();
    apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut queue,
        &[
            BlockerDeclaration {
                blocker,
                blocking: attacker,
            },
            BlockerDeclaration {
                blocker: unrelated_blocker,
                blocking: unrelated_attacker,
            },
        ],
        B,
    )
    .unwrap();
    game.combat = Some(combat.clone());
    resolve(&mut game, &mut queue);
    ironsmith::turn::advance_step(&mut game).unwrap();
    assert_eq!(game.turn.step, Some(Step::CombatDamage));
    let _damage = execute_combat_damage_step(&mut game, &combat, false);
    game.combat = Some(combat);
    CombatBoard {
        game,
        source,
        partner,
        unrelated_attacker,
        unrelated_blocker,
    }
}

fn queue_combat_end(board: &mut CombatBoard) -> TriggerQueue {
    ironsmith::turn::advance_step(&mut board.game).unwrap();
    assert_eq!(board.game.turn.step, Some(Step::EndCombat));
    let mut queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut board.game, &mut queue);
    queue
}

#[test]
fn end_combat_wall_exiles_only_its_own_blocked_attacker_and_returns_that_card() {
    for definition in fixture("Wall of Nets") {
        let mut board = combat_board(&definition, false);
        let partner_stable = board.game.object(board.partner).unwrap().stable_id;
        let mut queue = queue_combat_end(&mut board);
        assert_eq!(queue.entries.len(), 1);
        resolve(&mut board.game, &mut queue);
        let exiled = board.game.find_object_by_stable_id(partner_stable).unwrap();
        assert_eq!(board.game.object(exiled).unwrap().zone, Zone::Exile);
        for other in [
            board.source,
            board.unrelated_attacker,
            board.unrelated_blocker,
        ] {
            assert_eq!(board.game.object(other).unwrap().zone, Zone::Battlefield);
        }
        board
            .game
            .move_object_by_effect(board.source, Zone::Graveyard)
            .unwrap();
        resolve(&mut board.game, &mut queue);
        let returned = board.game.find_object_by_stable_id(partner_stable).unwrap();
        assert_ne!(
            returned, board.partner,
            "returning creates a new battlefield identity"
        );
        assert_eq!(board.game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(board.game.current_controller(returned), Some(A));
    }
}

#[test]
fn end_combat_ferrets_use_directional_turn_history_and_do_not_follow_new_identities() {
    for definition in fixture("Joven's Ferrets") {
        for blink_partner in [false, true] {
            let mut board = combat_board(&definition, true);
            let partner_stable = board.game.object(board.partner).unwrap().stable_id;
            if blink_partner {
                let hand = board
                    .game
                    .move_object_by_effect(board.partner, Zone::Hand)
                    .unwrap();
                board
                    .game
                    .move_object_by_effect(hand, Zone::Battlefield)
                    .unwrap();
            }
            let mut queue = queue_combat_end(&mut board);
            stack(&mut board.game, &mut queue);
            board
                .game
                .move_object_by_effect(board.source, Zone::Hand)
                .unwrap();
            resolve(&mut board.game, &mut queue);
            let partner = board.game.find_object_by_stable_id(partner_stable).unwrap();
            assert_eq!(board.game.is_tapped(partner), !blink_partner);
            assert!(!board.game.is_tapped(board.unrelated_blocker));
            if !blink_partner {
                board.game.next_turn();
                board.game.turn.phase = Phase::Beginning;
                board.game.turn.step = Some(Step::Untap);
                assert_eq!(board.game.turn.active_player, B);
                ironsmith::turn::execute_untap_step(&mut board.game);
                assert!(
                    board.game.is_tapped(partner),
                    "the same blocker skips its controller's next untap step"
                );
            }
        }
    }
}

#[test]
fn end_combat_symmetric_counter_and_destroy_bodies_restrict_the_source_partner() {
    use ironsmith::object::CounterType;
    for name in ["Greater Werewolf", "Kjeldoran Frostbeast", "Dread Wight"] {
        for definition in fixture(name) {
            for source_attacks in [false, true] {
                let mut board = combat_board(&definition, source_attacks);
                let partner_stable = board.game.object(board.partner).unwrap().stable_id;
                let mut queue = queue_combat_end(&mut board);
                assert_eq!(queue.entries.len(), 1);
                resolve(&mut board.game, &mut queue);
                let current = board.game.find_object_by_stable_id(partner_stable).unwrap();
                match name {
                    "Kjeldoran Frostbeast" => {
                        assert_eq!(board.game.object(current).unwrap().zone, Zone::Graveyard)
                    }
                    "Greater Werewolf" => assert_eq!(
                        board
                            .game
                            .object(current)
                            .unwrap()
                            .counters
                            .get(&CounterType::MinusZeroMinusTwo),
                        Some(&1)
                    ),
                    _ => {
                        let paralysis = CounterType::Named("paralyzation".into());
                        assert_eq!(
                            board.game.object(current).unwrap().counters.get(&paralysis),
                            Some(&1)
                        );
                        assert!(
                            board.game.is_tapped(current),
                            "Dread Wight must really tap its affected combat partner"
                        );
                        assert!(!board.game.can_untap_during_step(
                            current,
                            board.game.current_controller(current).unwrap()
                        ));
                        assert!(
                            board
                                .game
                                .current_abilities(current)
                                .unwrap()
                                .iter()
                                .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
                            "the same recipient must gain the counter-removal ability"
                        );
                    }
                }
                for other in [
                    board.source,
                    board.unrelated_attacker,
                    board.unrelated_blocker,
                ] {
                    assert_eq!(board.game.object(other).unwrap().zone, Zone::Battlefield);
                    assert!(
                        board.game.object(other).unwrap().counters.is_empty(),
                        "unrelated combat objects must not receive the counter"
                    );
                }
            }
        }
    }
}

#[test]
fn end_combat_wretched_only_takes_its_blockers_and_ends_control_when_source_leaves() {
    for definition in fixture("The Wretched") {
        let mut board = combat_board(&definition, true);
        let mut queue = queue_combat_end(&mut board);
        resolve(&mut board.game, &mut queue);
        assert_eq!(board.game.current_controller(board.partner), Some(A));
        assert_eq!(
            board.game.current_controller(board.unrelated_blocker),
            Some(B)
        );
        board
            .game
            .move_object_by_effect(board.source, Zone::Graveyard)
            .unwrap();
        assert_eq!(board.game.current_controller(board.partner), Some(B));
    }
}

#[test]
fn end_combat_dread_wight_recipient_keeps_and_can_pay_its_counter_removal_ability() {
    use ironsmith::decision::{GameProgress, LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    use ironsmith::mana::ManaSymbol;
    use ironsmith::object::CounterType;
    for definition in fixture("Dread Wight") {
        let mut board = combat_board(&definition, true);
        let mut queue = queue_combat_end(&mut board);
        resolve(&mut board.game, &mut queue);
        board
            .game
            .move_object_by_effect(board.source, Zone::Graveyard)
            .unwrap();
        resolve(&mut board.game, &mut queue);
        board.game.turn.priority_player = Some(B);
        board
            .game
            .player_mut(B)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 4);
        let action = compute_legal_actions(&board.game, B).unwrap().into_iter()
            .find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == board.partner))
            .expect("the recipient, not Dread Wight, owns the counter-removal activation");
        let mut state = PriorityLoopState::new(2);
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            &mut board.game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..20 {
            if state.pending_activation.is_none() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("counter-removal cost did not complete");
            };
            progress = apply_decision_context_with_dm(
                &mut board.game,
                &mut queue,
                &mut state,
                &context,
                &mut dm,
            )
            .unwrap();
        }
        assert!(state.pending_activation.is_none());
        resolve(&mut board.game, &mut queue);
        assert_eq!(board.game.player(B).unwrap().mana_pool.total(), 0);
        assert_eq!(
            board
                .game
                .object(board.partner)
                .unwrap()
                .counters
                .get(&CounterType::Named("paralyzation".into()))
                .copied()
                .unwrap_or(0),
            0
        );
        assert!(board.game.can_untap_during_step(board.partner, B));
        board.game.next_turn();
        assert_eq!(board.game.turn.active_player, B);
        ironsmith::turn::execute_untap_step(&mut board.game);
        assert!(!board.game.is_tapped(board.partner));
    }
}

#[test]
fn end_combat_source_lki_uses_this_combat_without_reusing_earlier_partners() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration};
    use ironsmith::game_loop::{
        apply_attacker_declarations, apply_blocker_declarations, execute_combat_damage_step,
    };
    use ironsmith::object::CounterType;
    for definition in fixture("Greater Werewolf") {
        let mut board = combat_board(&definition, true);
        let mut queue = queue_combat_end(&mut board);
        resolve(&mut board.game, &mut queue);
        assert_eq!(
            board
                .game
                .object(board.partner)
                .unwrap()
                .counters
                .get(&CounterType::MinusZeroMinusTwo),
            Some(&1)
        );
        let new_partner_def = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Second-combat blocker",
            "Type: Creature\nPower/Toughness: 0/10",
            false,
        )
        .unwrap();
        let new_partner =
            board
                .game
                .create_object_from_definition(&new_partner_def, B, Zone::Battlefield);
        board.game.untap(board.source);
        board.game.turn.phase = Phase::Combat;
        board.game.turn.step = Some(Step::DeclareAttackers);
        board.game.mark_combat_phase_started();
        let mut combat = CombatState::default();
        apply_attacker_declarations(
            &mut board.game,
            &mut combat,
            &mut queue,
            &[AttackerDeclaration {
                creature: board.source,
                target: AttackTarget::Player(B),
            }],
        )
        .unwrap();
        board.game.combat = Some(combat.clone());
        ironsmith::turn::advance_step(&mut board.game).unwrap();
        apply_blocker_declarations(
            &mut board.game,
            &mut combat,
            &mut queue,
            &[BlockerDeclaration {
                blocker: new_partner,
                blocking: board.source,
            }],
            B,
        )
        .unwrap();
        board.game.combat = Some(combat.clone());
        ironsmith::turn::advance_step(&mut board.game).unwrap();
        let _damage = execute_combat_damage_step(&mut board.game, &combat, false);
        board.game.combat = Some(combat);
        let mut queue = queue_combat_end(&mut board);
        stack(&mut board.game, &mut queue);
        board
            .game
            .move_object_by_effect(board.source, Zone::Graveyard)
            .unwrap();
        resolve(&mut board.game, &mut queue);
        assert_eq!(
            board
                .game
                .object(new_partner)
                .unwrap()
                .counters
                .get(&CounterType::MinusZeroMinusTwo),
            Some(&1)
        );
        assert_eq!(
            board
                .game
                .object(board.partner)
                .unwrap()
                .counters
                .get(&CounterType::MinusZeroMinusTwo),
            Some(&1),
            "an earlier combat is not the departed source's current-combat LKI"
        );
    }
}
