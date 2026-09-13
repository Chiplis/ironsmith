//! Frozen atlas observation 25063045: verify the complete compiled card.
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::types::Subtype;
use ironsmith::{AbilityKind, CardDefinition, GameState, PlayerId, Zone};
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_payloads_by_name,
};

fn definition() -> CardDefinition {
    let payloads =
        load_card_payloads_by_name(default_cards_path().to_str().unwrap(), "A Killer Among Us")
            .unwrap();
    assert_eq!(payloads.len(), 1);
    let snapshot = compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(!snapshot.has_unimplemented && !snapshot.parse_lossy);
    compile_definition_from_payload(&payloads[0]).unwrap()
}

struct ChooseSubtype(Subtype);
impl DecisionMaker for ChooseSubtype {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.options.len() == 3
            && ctx
                .options
                .iter()
                .any(|o| o.description == self.0.to_string())
        {
            return vec![
                ctx.options
                    .iter()
                    .find(|o| o.description == self.0.to_string())
                    .unwrap()
                    .index,
            ];
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}

#[test]
fn a_killer_among_us_entry_creates_all_tokens_then_keeps_each_type_choice_private() {
    let definition = definition();
    assert_eq!(
        definition
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .count(),
        1
    );
    assert_eq!(
        definition
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Activated(_)))
            .count(),
        1
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for chosen in [Subtype::Human, Subtype::Merfolk, Subtype::Goblin] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let other = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.set_secret_chosen_subtype(other, alice, Subtype::Goblin);
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let source = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        assert_eq!(queue.entries.len(), 1);
        assert_eq!(game.secret_chosen_subtype(source, alice), None);
        ironsmith::game_loop::run_priority_loop_with(
            &mut game,
            &mut queue,
            &mut ChooseSubtype(chosen),
        )
        .unwrap();
        assert_eq!(game.secret_chosen_subtype(source, alice), Some(chosen));
        assert_eq!(game.secret_chosen_subtype(source, bob), None);
        assert_eq!(game.chosen_subtype(source), None);
        assert_eq!(
            game.secret_chosen_subtype(other, alice),
            Some(Subtype::Goblin)
        );
        let creatures = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|o| o.is_creature())
            .collect::<Vec<_>>();
        assert_eq!(creatures.len(), 3);
        for (subtype, color) in [
            (Subtype::Human, ironsmith::color::ColorSet::WHITE),
            (Subtype::Merfolk, ironsmith::color::ColorSet::BLUE),
            (Subtype::Goblin, ironsmith::color::ColorSet::RED),
        ] {
            let token = creatures
                .iter()
                .find(|o| o.has_subtype(subtype))
                .expect("each specified token");
            assert!(matches!(token.kind, ironsmith::object::ObjectKind::Token));
            assert_eq!(game.controller_of(token), alice);
            assert_eq!((token.power(), token.toughness()), (Some(1), Some(1)));
            assert_eq!(token.colors(), color);
        }
    }
}

fn source_with_choice(
    game: &mut GameState,
    definition: &CardDefinition,
    alice: PlayerId,
) -> ironsmith::ObjectId {
    let hand = game.create_object_from_definition(definition, alice, Zone::Hand);
    let source = game
        .move_object_with_etb_processing(hand, Zone::Battlefield)
        .unwrap()
        .new_id;
    let mut queue = TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    ironsmith::game_loop::run_priority_loop_with(
        game,
        &mut queue,
        &mut ChooseSubtype(Subtype::Human),
    )
    .unwrap();
    source
}

#[test]
fn a_killer_among_us_pays_before_resolving_and_checks_the_targets_current_type() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::decision::{GameProgress, LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::game_state::{Phase, Target};
    use ironsmith::object::CounterType;
    use ironsmith::static_abilities::StaticAbilityId;
    struct PickTarget(ironsmith::ObjectId);
    impl DecisionMaker for PickTarget {
        fn decide_targets(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            vec![Target::Object(self.0)]
        }
    }
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Change {
        TypeOnly,
        LeaveCombat,
        LeaveBattlefield,
        MoveSourceAgain,
    }
    for (initial_type, final_type, change) in [
        (Subtype::Human, Subtype::Human, Change::TypeOnly),
        (Subtype::Goblin, Subtype::Goblin, Change::TypeOnly),
        (Subtype::Goblin, Subtype::Human, Change::TypeOnly),
        (Subtype::Human, Subtype::Goblin, Change::TypeOnly),
        (Subtype::Human, Subtype::Human, Change::LeaveCombat),
        (Subtype::Human, Subtype::Human, Change::LeaveBattlefield),
        (Subtype::Human, Subtype::Human, Change::MoveSourceAgain),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = source_with_choice(&mut game, &definition, alice);
        let source_stable = game.object(source).unwrap().stable_id;
        let target = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|o| o.has_subtype(initial_type))
            })
            .unwrap();
        let bystander = game
            .battlefield
            .iter()
            .copied()
            .find(|id| *id != target && game.object(*id).is_some_and(|o| o.is_creature()))
            .unwrap();
        game.turn.phase = Phase::Combat;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: target,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);
        let action = compute_legal_actions(&game, alice)
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source))
            .expect("wrong-type attacking token must still be a legal target");
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut queue = TriggerQueue::new();
        let mut dm = PickTarget(target);
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..20 {
            if !game.stack.is_empty() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}")
            };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &ctx, &mut dm,
            )
            .unwrap();
        }
        assert_eq!(game.stack.len(), 1, "activation must finish payment");
        assert!(
            game.object(source).is_none(),
            "source must already be sacrificed"
        );
        assert_eq!(
            game.chosen_subtype(source),
            Some(Subtype::Human),
            "choice must be revealed during payment"
        );
        assert_eq!(
            game.object(target)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            None
        );
        game.object_mut(target).unwrap().subtypes = vec![final_type].into();
        let observed_target = if change == Change::LeaveBattlefield {
            game.move_object_by_effect(target, Zone::Graveyard).unwrap()
        } else {
            target
        };
        if change == Change::LeaveCombat {
            game.combat.as_mut().unwrap().attackers.clear();
        }
        if change == Change::MoveSourceAgain {
            let grave_source = game.find_object_by_stable_id(source_stable).unwrap();
            let new_source = game
                .move_object_by_effect(grave_source, Zone::Hand)
                .unwrap();
            assert_eq!(game.secret_chosen_subtype(new_source, alice), None);
            assert_eq!(game.chosen_subtype(new_source), None);
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let expected = if final_type == Subtype::Human
            && !matches!(change, Change::LeaveCombat | Change::LeaveBattlefield)
        {
            3
        } else {
            0
        };
        assert_eq!(
            game.object(observed_target)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            expected
        );
        assert_eq!(
            game.object_has_static_ability_id(observed_target, StaticAbilityId::Deathtouch),
            expected == 3
        );
        assert_eq!(
            game.object(bystander)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            None
        );
        if change == Change::LeaveBattlefield {
            continue;
        }
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(!game.object_has_static_ability_id(observed_target, StaticAbilityId::Deathtouch));
        assert_eq!(
            game.object(observed_target)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            expected
        );
    }
}

#[test]
fn a_killer_among_us_legality_requires_attacking_token_and_the_payers_own_choice() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_state::Phase;
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = source_with_choice(&mut game, &definition, alice);
    let no_choice = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let target = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|o| o.has_subtype(Subtype::Goblin))
        })
        .unwrap();
    game.turn.phase = Phase::Combat;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    let offered = |game: &GameState, player, source| {
        compute_legal_actions(game, player)
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source))
    };
    assert!(
        !offered(&game, alice, source),
        "a nonattacking token is not eligible"
    );
    game.set_current_controller(target, bob);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo {
        creature: target,
        target: AttackTarget::Player(alice),
    });
    game.combat = Some(combat);
    assert!(
        offered(&game, alice, source),
        "an opponent's wrong-type token is eligible"
    );
    assert!(
        !offered(&game, alice, no_choice),
        "cannot reveal a choice that was never made"
    );
    game.object_mut(target).unwrap().kind = ironsmith::object::ObjectKind::Card;
    assert!(
        !offered(&game, alice, source),
        "attacking nontokens are not eligible"
    );
    game.object_mut(target).unwrap().kind = ironsmith::object::ObjectKind::Token;
    game.set_current_controller(source, bob);
    game.turn.priority_player = Some(bob);
    assert!(
        !offered(&game, bob, source),
        "the new controller did not make this choice"
    );
}
