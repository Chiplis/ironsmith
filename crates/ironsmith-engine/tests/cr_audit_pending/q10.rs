//! CR audit (in-review completion pass, package q10): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::decisions::context::BooleanContext;
use ironsmith::{DecisionMaker, EffectExecutor, PlayerFilter};

fn q10_artifact(game: &mut GameState, owner: PlayerId, name: &str) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    let def = CardDefinitionBuilder::new(ironsmith::CardId::new(), name)
        .card_types(vec![ironsmith::CardType::Artifact])
        .build();
    game.create_object_from_definition(&def, owner, ironsmith::Zone::Battlefield)
}

fn q10_pass_twice(game: &mut GameState) {
    use ironsmith::{LegalAction, PriorityLoopState, PriorityResponse};
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    for _ in 0..2 {
        ironsmith::game_loop::apply_priority_response_with_dm(
            game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            &mut dm,
        )
        .unwrap();
    }
}

/// CR 724.1d: phases added earlier in the turn are skipped when the turn ends.
#[test]
fn cr_724_1d_ending_the_turn_skips_previously_added_phases() {
    use ironsmith::effects::{AdditionalPhasesEffect, EndTurnEffect};
    use ironsmith::{Effect, Phase, StackEntry, TurnAction, TurnRunner, TurnRunnerState};
    for caster in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let source = q10_artifact(&mut game, alice, "Extra phase source");
        let other_source = q10_artifact(&mut game, caster, "Ending source");
        game.stack.push(StackEntry::ability(
            source,
            alice,
            vec![Effect::new(AdditionalPhasesEffect::combat_then_main())],
        ));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(
            !game.turn_store.additional_phases.is_empty(),
            "fixture: an additional combat and main phase were added"
        );
        game.stack.push(StackEntry::ability(
            other_source,
            caster,
            vec![Effect::new(EndTurnEffect::new(PlayerFilter::Active))],
        ));
        game.turn.priority_player = Some(alice);
        q10_pass_twice(&mut game);
        assert!(game.stack.is_empty());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::FirstMainPriority);
        let mut visited = Vec::new();
        let mut complete = false;
        let mut priority_windows = Vec::new();
        for _ in 0..40 {
            let action = runner.advance(&mut game, &mut queue).unwrap();
            visited.push((game.turn.phase, game.turn.step));
            match action {
                TurnAction::Continue => {}
                TurnAction::TurnComplete => {
                    complete = true;
                    break;
                }
                TurnAction::RunPriority => {
                    priority_windows.push((game.turn.phase, game.turn.step));
                    runner.priority_done();
                }
                other => panic!("unexpected runner action {other:?}"),
            }
        }
        assert!(complete, "CR 724.1d: the ended turn completes; visited={visited:?}");
        assert!(
            !visited
                .iter()
                .any(|(phase, _)| matches!(phase, Phase::Combat | Phase::NextMain)),
            "CR 724.1d: the previously added combat/main phases between the ending point and cleanup are skipped; visited={visited:?}"
        );
        assert_eq!(
            game.turn_store.combat_phases_started_this_turn, 0,
            "CR 724.1d: no added combat phase begins"
        );
        assert!(
            priority_windows.is_empty(),
            "CR 724.1d/724.1f: no priority in the quiet ending; got {priority_windows:?}"
        );
    }
}

/// CR 724.1d / 724.2d: ending the turn or the combat phase removes creatures and
/// planeswalkers from combat; ending combat proceeds straight to the next phase,
/// which is an added combat phase when one was scheduled after this one.
#[test]
fn cr_724_1d_2d_ending_procedures_remove_attacked_planeswalkers_and_go_to_next_scheduled_phase() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{AdditionalPhasesEffect, EndCombatPhaseEffect, EndTurnEffect};
    use ironsmith::{
        AttackTarget, AttackerDeclaration, CardId, CardType, Effect, Phase, PowerToughness,
        StackEntry, Step, TurnAction, TurnRunner, TurnRunnerState, Zone,
    };
    for end_combat in [false, true] {
        for added_combat in [false, true] {
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareAttackers);
            let attacker_def = CardDefinitionBuilder::new(CardId::new(), "Walker attacker")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .haste()
                .build();
            let attacker = game.create_object_from_definition(&attacker_def, alice, Zone::Battlefield);
            let walker_def = CardDefinitionBuilder::new(CardId::new(), "Attacked walker")
                .card_types(vec![CardType::Planeswalker])
                .loyalty(5)
                .build();
            let walker = game.create_object_from_definition(&walker_def, bob, Zone::Battlefield);
            let source = q10_artifact(&mut game, alice, "Ending source");
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut combat = ironsmith::combat_state::new_combat();
            ironsmith::game_loop::apply_attacker_declarations(
                &mut game,
                &mut combat,
                &mut queue,
                &[AttackerDeclaration {
                    creature: attacker,
                    target: AttackTarget::Planeswalker(walker),
                }],
            )
            .unwrap();
            game.combat = Some(combat);
            assert_eq!(
                ironsmith::combat_state::attackers_targeting_planeswalker(
                    game.combat.as_ref().unwrap(),
                    walker
                ),
                vec![attacker],
                "fixture: the planeswalker is being attacked"
            );
            if added_combat {
                game.stack.push(StackEntry::ability(
                    source,
                    alice,
                    vec![Effect::new(AdditionalPhasesEffect::combat())],
                ));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert!(!game.turn_store.additional_phases.is_empty());
            }
            let ending = if end_combat {
                Effect::new(EndCombatPhaseEffect)
            } else {
                Effect::new(EndTurnEffect::new(PlayerFilter::Active))
            };
            game.stack.push(StackEntry::ability(source, alice, vec![ending]));
            game.turn.priority_player = Some(alice);
            q10_pass_twice(&mut game);
            assert!(game.stack.is_empty());
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut runner =
                TurnRunner::from_state_for_sync(TurnRunnerState::DeclareAttackersPriority);
            let mut first_priority = None;
            let mut completed = false;
            for _ in 0..40 {
                match runner.advance(&mut game, &mut queue).unwrap() {
                    TurnAction::Continue => {}
                    TurnAction::RunPriority => {
                        first_priority = Some((game.turn.phase, game.turn.step));
                        break;
                    }
                    TurnAction::TurnComplete => {
                        completed = true;
                        break;
                    }
                    other => panic!("unexpected runner action {other:?}"),
                }
            }
            let combat = game.combat.as_ref().unwrap();
            assert!(
                !ironsmith::combat_state::is_attacking(combat, attacker)
                    && ironsmith::combat_state::attackers_targeting_planeswalker(combat, walker)
                        .is_empty(),
                "CR 724.1d/724.2d: end_combat={end_combat}, added={added_combat}: creatures and planeswalkers are removed from combat"
            );
            if end_combat {
                let expected = if added_combat {
                    (Phase::Combat, Some(Step::BeginCombat))
                } else {
                    (Phase::NextMain, None)
                };
                assert_eq!(
                    first_priority,
                    Some(expected),
                    "CR 724.2d: added={added_combat}: the game skips straight to the next phase"
                );
            } else {
                assert!(
                    completed && first_priority.is_none(),
                    "CR 724.1d: added={added_combat}: the game skips to cleanup and the turn ends quietly; first_priority={first_priority:?}"
                );
                assert_eq!(game.turn_store.combat_phases_started_this_turn <= 1, true);
            }
        }
    }
}

/// CR 727.3 / 729.3: a mulligan (hand shuffled back, seven redrawn) does not let a
/// player with fewer than seven library cards avoid the first-upkeep loss.
#[test]
fn cr_727_3_729_3_mulligan_does_not_save_a_short_library() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::RestartGameEffect;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{CardId, CardType, Step, TurnAction, TurnRunner, TurnRunnerState, Zone};
    for subgame in [false, true] {
        for cards in [3usize, 6, 7] {
            let short = PlayerId::from_index(0);
            let healthy = PlayerId::from_index(1);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition = CardDefinitionBuilder::new(CardId::new(), "Mulligan boundary card")
                .card_types(vec![CardType::Artifact])
                .build();
            let source = game.create_object_from_definition(&definition, healthy, Zone::Battlefield);
            for _ in 0..cards {
                game.create_object_from_definition(&definition, short, Zone::Library);
            }
            for _ in 0..10 {
                game.create_object_from_definition(&definition, healthy, Zone::Library);
            }
            if subgame {
                game.begin_subgame(None, healthy, vec![]).unwrap();
            } else {
                let mut ctx = EffectContext::new_default(source, healthy);
                RestartGameEffect::new(None).execute(&mut game, &mut ctx).unwrap();
            }
            // Mulligan: shuffle the hand back into the library and draw seven again.
            let hand = game.player(short).unwrap().hand.clone();
            for card in hand {
                game.move_object_by_effect(card, Zone::Library).unwrap();
            }
            assert_eq!(game.player(short).unwrap().library.len(), cards);
            game.draw_cards(short, 7);
            assert_eq!(game.player(short).unwrap().hand.len(), cards.min(7));
            if subgame {
                game.complete_subgame_starting_procedure();
            } else {
                game.complete_restart_starting_procedure();
            }
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Untap);
            let mut queue = TriggerQueue::new();
            let mut upkeep = false;
            for _ in 0..10 {
                match runner.advance(&mut game, &mut queue).unwrap() {
                    TurnAction::Continue => {}
                    TurnAction::RunPriority => {
                        upkeep = true;
                        break;
                    }
                    other => panic!("unexpected opening-turn action {other:?}"),
                }
            }
            assert!(upkeep);
            assert_eq!(game.turn.step, Some(Step::Upkeep));
            ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
            assert_eq!(
                game.player(short).unwrap().has_lost,
                cards < 7,
                "CR 727.3/729.3: subgame={subgame}, cards={cards}: fewer than seven cards loses at the first upkeep regardless of mulligans"
            );
            assert!(game.player(healthy).unwrap().is_in_game());
        }
    }
}

/// CR 800.4b: a player can't be controlled by a player who has left the game.
#[test]
fn cr_800_4b_departed_player_cannot_control_another_player() {
    use ironsmith::effects::ControlPlayerEffect;
    use ironsmith::game_state::{PlayerControlDuration, PlayerControlStart};
    for departed in [false, true] {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let source = q10_artifact(&mut game, carol, "Player control source");
        if departed {
            game.concede_game(bob).unwrap();
            assert!(!game.player(bob).unwrap().is_in_game());
        }
        let effect = ControlPlayerEffect {
            player: PlayerFilter::Specific(alice),
            start: PlayerControlStart::Immediate,
            duration: PlayerControlDuration::UntilEndOfTurn,
            target_spec: None,
            controller: Some(PlayerFilter::Specific(bob)),
        };
        let mut ctx = EffectContext::new_default(source, carol);
        let _ = effect.execute(&mut game, &mut ctx);
        let expected = if departed { alice } else { bob };
        assert_eq!(
            game.controlling_player_for(alice),
            expected,
            "CR 800.4b: departed={departed}: a player is not controlled by a player who has left"
        );
    }
}

/// CR 810.8a: players win and lose only as a team.
#[test]
fn cr_810_8a_one_heads_loss_or_win_applies_to_its_whole_team() {
    use ironsmith::{Effect, StackEntry};
    for mode in 0..3 {
        for head in 0..2usize {
            let players = (0..4).map(|i| PlayerId::from_index(i)).collect::<Vec<_>>();
            let teams = vec![vec![players[0], players[2]], vec![players[1], players[3]]];
            let mut game = GameState::new((0..4).map(|i| format!("Head{i}")).collect(), 20);
            game.enable_two_headed_giant(teams.clone()).unwrap();
            let actor = teams[0][head];
            let source = q10_artifact(&mut game, actor, "Team outcome source");
            match mode {
                0 => {
                    game.concede_game(actor).unwrap();
                }
                1 => {
                    game.stack.push(StackEntry::ability(
                        source,
                        actor,
                        vec![Effect::lose_the_game()],
                    ));
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
                _ => {
                    game.stack.push(StackEntry::ability(
                        source,
                        actor,
                        vec![Effect::win_the_game()],
                    ));
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
            }
            ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
            let team_lost = mode != 2;
            for member in &teams[0] {
                assert_eq!(
                    game.player(*member).unwrap().has_lost,
                    team_lost,
                    "CR 810.8a: mode={mode}, head={head}: member {member:?} of the acting team"
                );
            }
            for member in &teams[1] {
                assert_eq!(
                    game.player(*member).unwrap().has_lost,
                    !team_lost,
                    "CR 810.8a: mode={mode}, head={head}: member {member:?} of the opposing team"
                );
            }
        }
    }
}

/// CR 902.7: triggered and activated abilities of a face-up vanguard function
/// from the command zone.
#[test]
fn cr_902_7_vanguard_triggered_and_activated_abilities_function_from_command() {
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        Ability, CardId, CardType, Effect, LegalAction, Phase, TotalCost, TurnAction, TurnRunner,
        TurnRunnerState,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let cards = [alice, bob]
        .into_iter()
        .map(|owner| {
            let mut builder = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                format!("Functional vanguard {owner:?}"),
            )
            .card_types(vec![CardType::Vanguard]);
            if owner == alice {
                builder = builder
                    .with_ability(Ability::activated(
                        TotalCost::free(),
                        vec![Effect::gain_life(1)],
                    ))
                    .with_ability(Ability::triggered(
                        Trigger::beginning_of_upkeep(PlayerFilter::You),
                        vec![Effect::gain_life(3)],
                    ));
            }
            (owner, builder.build())
        })
        .collect();
    game.enable_vanguard(cards).unwrap();
    let avatar = game.vanguard_card(alice).unwrap();
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let actions = ironsmith::compute_legal_actions(&game, alice).unwrap();
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == avatar)),
        "CR 902.7: the vanguard's activated ability may be activated from the command zone; actions={actions:?}"
    );
    let control_def = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Battlefield upkeep control",
    )
    .card_types(vec![CardType::Artifact])
    .with_ability(Ability::triggered(
        Trigger::beginning_of_upkeep(PlayerFilter::You),
        vec![Effect::gain_life(3)],
    ))
    .build();
    let control =
        game.create_object_from_definition(&control_def, alice, ironsmith::Zone::Battlefield);
    game.turn.phase = Phase::Beginning;
    game.turn.step = None;
    game.turn.priority_player = None;
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Upkeep);
    let mut queue = TriggerQueue::new();
    let mut reached = false;
    for _ in 0..10 {
        match runner.advance(&mut game, &mut queue).unwrap() {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                reached = true;
                break;
            }
            other => panic!("unexpected upkeep action {other:?}"),
        }
    }
    assert!(reached);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert!(
        game.stack.iter().any(|entry| entry.object_id == control),
        "fixture: the same upkeep trigger on a battlefield permanent triggers"
    );
    assert!(
        game.stack.iter().any(|entry| entry.object_id == avatar),
        "CR 902.7: the vanguard's upkeep trigger triggers from the command zone"
    );
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    }
    assert_eq!(game.player(alice).unwrap().life, 26, "CR 902.7: both upkeep triggers resolve");
}

/// CR 904.10: a face-up non-ongoing scheme stays face up while a scheme's
/// triggered ability is on the stack, then turns face down.
#[test]
fn cr_904_10_scheme_waits_for_pending_scheme_triggers_before_turning_face_down() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{ArchenemyVariant, CardId, CardType, Effect, StackEntry, Supertype, TriggerQueue, Zone};
    for other_scheme_ability in [false, true] {
        let actor = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let deck = (0..20)
            .map(|index| {
                CardDefinitionBuilder::new(CardId::new(), format!("Timing scheme {index}"))
                    .card_types(vec![CardType::Scheme])
                    .supertypes(if index == 19 && other_scheme_ability {
                        vec![Supertype::Ongoing]
                    } else {
                        vec![]
                    })
                    .build()
            })
            .collect();
        game.enable_archenemy(ArchenemyVariant::Default, vec![(actor, deck)])
            .unwrap();
        let mut triggers = TriggerQueue::new();
        let mut ability_source = None;
        if other_scheme_ability {
            // The top of the deck is the last element; set the ongoing one in motion first.
            let ongoing = game.set_scheme_in_motion(actor).unwrap();
            ironsmith::put_triggers_on_stack(&mut game, &mut triggers).unwrap();
            if game.scheme_is_ongoing(ongoing) {
                ability_source = Some(ongoing);
            } else {
                ironsmith::apply_state_based_actions(&mut game).unwrap();
            }
        }
        let scheme = game.set_scheme_in_motion(actor).unwrap();
        ironsmith::put_triggers_on_stack(&mut game, &mut triggers).unwrap();
        let source = ability_source.unwrap_or(scheme);
        game.stack.push(StackEntry::ability(
            source,
            actor,
            vec![Effect::gain_life(2)],
        ));
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        assert!(
            game.is_face_up_scheme(scheme) && game.object(scheme).is_some(),
            "CR 904.10: other={other_scheme_ability}: the scheme stays face up while a scheme ability is on the stack"
        );
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        let stable_down = game
            .scheme_deck(actor)
            .unwrap()
            .first()
            .copied()
            .unwrap();
        assert!(
            !game.is_face_up_scheme(stable_down),
            "CR 904.10: the scheme then goes to the bottom face down"
        );
        assert!(
            game.object(scheme).is_none(),
            "CR 904.10: other={other_scheme_ability}: the non-ongoing scheme is turned face down once no scheme ability is pending"
        );
        assert_eq!(game.object(stable_down).unwrap().zone, Zone::Command);
    }
}

/// CR 903.9b: the commander replacement still applies after another replacement
/// changed a hand event into a library event (it may apply to the modified event).
#[test]
fn cr_903_9b_commander_replacement_applies_after_another_hand_replacement() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::events::zones::matchers::WouldGoToHandMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::{
        CardId, CardType, ChooseSpec, Effect, PowerToughness, StackEntry, Supertype, Zone,
    };
    struct Accept {
        calls: usize,
    }
    impl DecisionMaker for Accept {
        fn decide_boolean(&mut self, _: &GameState, _ctx: &BooleanContext) -> bool {
            self.calls += 1;
            true
        }
    }
    for owner in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let def = CardDefinitionBuilder::new(CardId::new(), "Bounced commander")
            .card_types(vec![CardType::Creature])
            .supertypes(vec![Supertype::Legendary])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let commander = game.create_object_from_definition(&def, owner, Zone::Battlefield);
        let stable = game.object(commander).unwrap().stable_id;
        game.set_as_commander(commander, owner);
        let source = q10_artifact(&mut game, owner, "Hand-to-library replacement source");
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                owner,
                WouldGoToHandMatcher::any_player(),
                ReplacementAction::ChangeDestination(Zone::Library),
            ),
        );
        game.stack.push(StackEntry::ability(
            source,
            owner,
            vec![Effect::new(MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(commander),
                Zone::Hand,
                false,
            ))],
        ));
        let mut dm = Accept { calls: 0 };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let now = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(now).unwrap().zone,
            Zone::Command,
            "CR 903.9b: owner={owner:?}: the commander may be put into the command zone instead of hand or library"
        );
    }
}

/// CR 805.10a-b: in shared team turns the active team declares one combined
/// attack with creatures of every teammate, each choosing a defending player.
#[test]
fn cr_805_10a_b_active_team_declares_one_combined_attack() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{
        AttackTarget, AttackerDeclaration, CardId, CardType, Phase, PowerToughness, Step, Zone,
    };
    for swap in [false, true] {
        let players = (0..4).map(|i| PlayerId::from_index(i)).collect::<Vec<_>>();
        let mut teams = vec![vec![players[0], players[2]], vec![players[1], players[3]]];
        if swap {
            teams.swap(0, 1);
        }
        let mut game = GameState::new((0..4).map(|i| format!("Head{i}")).collect(), 20);
        game.enable_two_headed_giant(teams.clone()).unwrap();
        let active = game.turn.active_player;
        let own = game.team_index_for(active).unwrap();
        let attackers_team = teams[own].clone();
        let defenders = teams[1 - own].clone();
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let def = CardDefinitionBuilder::new(CardId::new(), "Team attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .haste()
            .build();
        let a = game.create_object_from_definition(&def, attackers_team[0], Zone::Battlefield);
        let b = game.create_object_from_definition(&def, attackers_team[1], Zone::Battlefield);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut combat = ironsmith::combat_state::new_combat();
        let result = ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[
                AttackerDeclaration {
                    creature: a,
                    target: AttackTarget::Player(defenders[0]),
                },
                AttackerDeclaration {
                    creature: b,
                    target: AttackTarget::Player(defenders[1]),
                },
            ],
        );
        assert!(
            result.is_ok(),
            "CR 805.10b: swap={swap}: a combined attack by both active heads is legal: {result:?}"
        );
        game.combat = Some(combat.clone());
        assert!(
            ironsmith::combat_state::is_attacking(&combat, a)
                && ironsmith::combat_state::is_attacking(&combat, b),
            "CR 805.10b: both teammates' creatures attack in the one combined attack"
        );
        for player in &attackers_team {
            assert!(
                ironsmith::combat_state::is_attacking_player(&combat, *player, &game),
                "CR 805.10a: swap={swap}: each active-team player is an attacking player"
            );
        }
        for player in &defenders {
            assert!(
                ironsmith::combat_state::is_defending_player(&combat, *player),
                "CR 805.10a: swap={swap}: each nonactive-team player is a defending player"
            );
        }
    }
}
