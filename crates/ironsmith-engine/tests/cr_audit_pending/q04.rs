//! CR audit (in-review completion pass, package q04): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};
use ironsmith::{DecisionMaker, EffectExecutor, PlayerFilter, SelectOptionsContext};

fn q04_finish_priority_action(game: &mut GameState, action: ironsmith::decision::LegalAction) {
    use ironsmith::decision::GameProgress;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::triggers::TriggerQueue;
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let before = game.stack.len();
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .expect("legal priority action must start");
    for _ in 0..32 {
        if game.stack.len() > before {
            return;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("priority action stopped before reaching the stack: {progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &context, &mut dm,
        )
        .expect("priority action must finish");
    }
    panic!("priority action did not finish");
}

#[test]
fn cr_307_1_sorcery_cast_only_in_own_main_phase_with_empty_stack() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType, Effect, LegalAction, ManaCost, Phase, StackEntry, Step, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    // (active player, phase, step, stack occupied, expected castable)
    let cases = [
        (alice, Phase::FirstMain, None, false, true),
        (alice, Phase::NextMain, None, false, true),
        (alice, Phase::FirstMain, None, true, false),
        (alice, Phase::Combat, Some(Step::BeginCombat), false, false),
        (alice, Phase::Beginning, Some(Step::Upkeep), false, false),
        (alice, Phase::Ending, Some(Step::End), false, false),
        (bob, Phase::FirstMain, None, false, false),
        (bob, Phase::NextMain, None, false, false),
    ];
    let mut mismatches = Vec::new();
    for (index, (active, phase, step, occupied, expected)) in cases.into_iter().enumerate() {
        for card_type in [CardType::Sorcery, CardType::Instant] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = active;
            game.turn.priority_player = Some(alice);
            game.turn.phase = phase;
            game.turn.step = step;
            let definition = CardDefinitionBuilder::new(CardId::new(), "Timing subject")
                .card_types(vec![card_type])
                .mana_cost(ManaCost::from_symbols(vec![]))
                .with_spell_effect(vec![Effect::gain_life(1)])
                .build();
            let card = game.create_object_from_definition(&definition, alice, Zone::Hand);
            if occupied {
                let holder = CardDefinitionBuilder::new(CardId::new(), "Stack holder")
                    .card_types(vec![CardType::Artifact])
                    .build();
                let holder = game.create_object_from_definition(&holder, bob, Zone::Battlefield);
                game.stack
                    .push(StackEntry::ability(holder, bob, vec![Effect::gain_life(1)]));
            }
            let castable = ironsmith::compute_legal_actions(&game, alice)
                .unwrap()
                .iter()
                .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card));
            let required = if card_type == CardType::Instant { true } else { expected };
            if castable != required {
                mismatches.push(format!(
                    "case={index} type={card_type:?}: castable={castable}, required={required}"
                ));
            }
            if castable && card_type == CardType::Sorcery {
                let action = ironsmith::compute_legal_actions(&game, alice)
                    .unwrap()
                    .into_iter()
                    .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
                    .unwrap();
                q04_finish_priority_action(&mut game, action);
                let top = game.stack.last().unwrap();
                assert!(!top.is_ability, "CR 307.1: casting a sorcery uses the stack");
                assert_eq!(game.object(top.object_id).unwrap().zone, Zone::Stack);
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 307.1: a sorcery is castable only by a player with priority in their own main phase with an empty stack: {mismatches:#?}"
    );
}

#[test]
fn cr_405_1_spell_card_goes_on_stack_but_abilities_have_no_card() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        Ability, CardId, CardType, Effect, LegalAction, ManaCost, Phase, Step, TurnAction,
        TurnRunner, TurnRunnerState, Zone,
    };
    let alice = PlayerId::from_index(0);
    // Spell: the physical card itself moves to the stack.
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    let definition = CardDefinitionBuilder::new(CardId::new(), "Stack spell subject")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![]))
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let card = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let action = ironsmith::compute_legal_actions(&game, alice)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
        .unwrap();
    q04_finish_priority_action(&mut game, action);
    assert_eq!(game.stack.len(), 1);
    let entry = game.stack[0].clone();
    assert!(!entry.is_ability, "CR 405.1: a cast spell is a card on the stack");
    let on_stack = game.object(entry.object_id).unwrap();
    assert_eq!(on_stack.zone, Zone::Stack, "CR 405.1: the physical card is on the stack");
    assert_eq!(on_stack.stable_id, stable, "CR 405.1: the stack object is the same physical card");
    assert!(game.player(alice).unwrap().hand.is_empty());

    // Activated ability: goes on the stack without moving any card.
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    let source = CardDefinitionBuilder::new(CardId::new(), "Activated source")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::activated(TotalCost::free(), vec![Effect::gain_life(1)]))
        .build();
    let source = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let action = ironsmith::compute_legal_actions(&game, alice)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
        .expect("free activated ability is legal");
    q04_finish_priority_action(&mut game, action);
    assert_eq!(game.stack.len(), 1);
    assert!(game.stack[0].is_ability, "CR 405.1: an activated ability is not a card on the stack");
    assert_eq!(
        game.object(source).map(|o| o.zone),
        Some(Zone::Battlefield),
        "CR 405.1: activating an ability does not put its source card on the stack"
    );
    assert!(
        game.objects_in_deterministic_order().iter().all(|o| o.zone != Zone::Stack),
        "CR 405.1: no card is associated with an activated ability on the stack"
    );

    // Triggered ability: same.
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.turn_number = 2;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Draw);
    let observer = CardDefinitionBuilder::new(CardId::new(), "Trigger source")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(
            Trigger::beginning_of_draw_step(PlayerFilter::You),
            vec![Effect::gain_life(1)],
        ))
        .build();
    let observer = game.create_object_from_definition(&observer, alice, Zone::Battlefield);
    let top = CardDefinitionBuilder::new(CardId::new(), "Top card")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_definition(&top, alice, Zone::Library);
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Draw);
    let mut queue = TriggerQueue::new();
    assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::RunPriority));
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert!(game.stack[0].is_ability, "CR 405.1: a triggered ability is not a card on the stack");
    assert_eq!(game.object(observer).map(|o| o.zone), Some(Zone::Battlefield));
    assert!(
        game.objects_in_deterministic_order().iter().all(|o| o.zone != Zone::Stack),
        "CR 405.1: no card is associated with a triggered ability on the stack"
    );
}

#[test]
fn cr_504_1_2_active_player_draws_without_stack_then_gets_priority() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{CardId, CardType, Phase, Step, TurnAction, TurnRunner, TurnRunnerState, Zone};
    for active_index in [0u8, 1] {
        let active = PlayerId::from_index(active_index);
        let other = PlayerId::from_index(1 - active_index);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = active;
        game.turn.turn_number = 3;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        for player in [active, other] {
            for _ in 0..2 {
                let card = CardDefinitionBuilder::new(CardId::new(), "Library card")
                    .card_types(vec![CardType::Artifact])
                    .build();
                game.create_object_from_definition(&card, player, Zone::Library);
            }
        }
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Draw);
        let mut queue = TriggerQueue::new();
        let mut action = runner.advance(&mut game, &mut queue).unwrap();
        for _ in 0..4 {
            if !matches!(action, TurnAction::Continue) {
                break;
            }
            action = runner.advance(&mut game, &mut queue).unwrap();
        }
        assert_eq!(game.player(active).unwrap().hand.len(), 1, "CR 504.1: the active player draws a card");
        assert_eq!(game.player(other).unwrap().hand.len(), 0, "CR 504.1: only the active player draws");
        assert!(game.stack.is_empty(), "CR 504.1: the draw doesn't use the stack");
        assert!(matches!(action, TurnAction::RunPriority), "CR 504.2: priority follows the draw, got {action:?}");
        assert_eq!(game.turn.step, Some(Step::Draw));
        assert_eq!(game.turn.priority_player, Some(active), "CR 504.2: the active player gets priority");
    }
}

#[test]
fn cr_314_6_pending_scheme_ability_delays_recycling_of_every_nonongoing_scheme() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{ArchenemyVariant, CardId, CardType, Effect, StackEntry, Supertype, TriggerQueue, Zone};
    let alice = PlayerId::from_index(0);
    for ability_on_ongoing in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let deck = (0..20)
            .map(|index| {
                CardDefinitionBuilder::new(CardId::new(), format!("Pending scheme {index}"))
                    .card_types(vec![CardType::Scheme])
                    .supertypes(if index % 2 == 0 { vec![Supertype::Ongoing] } else { vec![] })
                    .build()
            })
            .collect();
        game.enable_archenemy(ArchenemyVariant::Default, vec![(alice, deck)])
            .unwrap();
        let mut triggers = TriggerQueue::new();
        let mut face_up = Vec::new();
        while !(face_up.iter().any(|o| game.scheme_is_ongoing(*o))
            && face_up.iter().any(|o| !game.scheme_is_ongoing(*o)))
        {
            face_up.push(game.set_scheme_in_motion(alice).unwrap());
            ironsmith::put_triggers_on_stack(&mut game, &mut triggers).unwrap();
            assert!(game.stack.is_empty());
        }
        let ongoing = *face_up.iter().find(|o| game.scheme_is_ongoing(**o)).unwrap();
        let nonongoing: Vec<_> = face_up
            .iter()
            .copied()
            .filter(|o| !game.scheme_is_ongoing(*o))
            .collect();
        let source = if ability_on_ongoing { ongoing } else { nonongoing[0] };
        game.stack
            .push(StackEntry::ability(source, alice, vec![Effect::gain_life(1)]));
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        for scheme in &nonongoing {
            assert!(
                game.is_face_up_scheme(*scheme),
                "CR 314.6: a triggered ability of a scheme on the stack keeps every non-ongoing scheme face up (source ongoing={ability_on_ongoing})"
            );
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.stack.is_empty());
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        for scheme in &nonongoing {
            assert!(
                !game.is_face_up_scheme(*scheme) && game.object(*scheme).is_none(),
                "CR 314.6: once no scheme ability is pending, non-ongoing schemes are turned face down"
            );
        }
        assert!(game.is_face_up_scheme(ongoing), "CR 314.6: ongoing schemes stay face up");
        let deck = game.scheme_deck(alice).unwrap();
        for index in 0..nonongoing.len() {
            let recycled = deck[index];
            assert_eq!(game.object(recycled).unwrap().zone, Zone::Command);
            assert!(!game.scheme_is_ongoing(recycled), "CR 314.6: recycled schemes go to the bottom");
        }
    }
}

#[test]
fn cr_400_4b_planes_phenomena_and_schemes_cannot_leave_the_command_zone() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::{ArchenemyVariant, CardId, CardType, ChooseSpec, PlanarCardKind, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut failures = Vec::new();
    for destination in [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Stack,
        Zone::Battlefield,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let planar = |prefix: &str| {
            (0..10)
                .map(|index| {
                    let kind = if index < 2 { PlanarCardKind::Phenomenon } else { PlanarCardKind::Plane };
                    let card_type = if index < 2 { CardType::Phenomenon } else { CardType::Plane };
                    (
                        CardDefinitionBuilder::new(CardId::new(), format!("{prefix} planar {index}"))
                            .card_types(vec![card_type])
                            .build(),
                        kind,
                    )
                })
                .collect::<Vec<_>>()
        };
        game.enable_planechase(vec![(alice, planar("A")), (bob, planar("B"))])
            .unwrap();
        let face_up_plane = game.reveal_starting_plane().unwrap();
        let schemes = (0..20)
            .map(|index| {
                CardDefinitionBuilder::new(CardId::new(), format!("Command scheme {index}"))
                    .card_types(vec![CardType::Scheme])
                    .build()
            })
            .collect();
        game.enable_archenemy(ArchenemyVariant::Default, vec![(bob, schemes)])
            .unwrap();
        let face_up_scheme = game.set_scheme_in_motion(bob).unwrap();
        let mut subjects = vec![("face-up plane", face_up_plane), ("face-up scheme", face_up_scheme)];
        let deck = game.planar_deck(bob).unwrap().to_vec();
        if let Some(p) = deck.iter().copied().find(|o| game.planar_card_kind(*o) == Some(PlanarCardKind::Phenomenon)) {
            subjects.push(("face-down phenomenon", p));
        }
        if let Some(p) = deck.iter().copied().find(|o| game.planar_card_kind(*o) == Some(PlanarCardKind::Plane)) {
            subjects.push(("face-down plane", p));
        }
        subjects.push(("face-down scheme", game.scheme_deck(bob).unwrap()[0]));
        for (label, object) in subjects {
            let controller = game.object(object).unwrap().owner;
            let _ = MoveToZoneEffect::new(ChooseSpec::SpecificObject(object), destination, false)
                .execute(&mut game, &mut EffectContext::new_default(object, controller));
            let zone = game.object(object).map(|o| o.zone);
            if zone != Some(Zone::Command) {
                failures.push(format!("{label} -> {destination:?}: now {zone:?}"));
            }
        }
        game.validate_zone_consistency().unwrap();
    }
    assert!(
        failures.is_empty(),
        "CR 400.4b: plane, phenomenon and scheme cards that would leave the command zone remain there: {failures:#?}"
    );
}

#[test]
fn cr_506_4c_510_1b_attacker_of_departed_battle_keeps_attacking_but_deals_no_damage() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::{
        Ability, AttackerDeclaration, CardId, CardType, ChooseSpec, CounterType, Phase,
        PowerToughness, StaticAbility, Step, Subtype, TurnAction, TurnRunner, TurnRunnerState,
        Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (case, departure) in [Zone::Exile, Zone::Graveyard, Zone::Hand].into_iter().enumerate() {
        for blocked in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareAttackers);
            let battle = CardDefinitionBuilder::new(CardId::new(), "Attacked siege")
                .card_types(vec![CardType::Battle])
                .subtypes(vec![Subtype::Siege])
                .defense(5)
                .build();
            let battle = game.create_object_from_definition(&battle, alice, Zone::Battlefield);
            assert!(game.set_battle_protector(battle, bob));
            let attacker = CardDefinitionBuilder::new(CardId::new(), "Battle attacker")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(3, 10))
                .with_ability(Ability::static_ability(StaticAbility::haste()))
                .build();
            let attacker = game.create_object_from_definition(&attacker, alice, Zone::Battlefield);
            let blocker = CardDefinitionBuilder::new(CardId::new(), "Battle defender")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 10))
                .build();
            let blocker = game.create_object_from_definition(&blocker, bob, Zone::Battlefield);
            let mut combat = combat_state::new_combat();
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::apply_attacker_declarations(
                &mut game,
                &mut combat,
                &mut queue,
                &[AttackerDeclaration { creature: attacker, target: AttackTarget::Battle(battle) }],
            )
            .unwrap();
            game.combat = Some(combat);
            assert_eq!(game.counter_count(battle, CounterType::Defense), 5);
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(battle), departure, false)
                .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                .unwrap();
            game.refresh_continuous_state().unwrap();
            let mut combat = game.combat.take().unwrap();
            assert!(
                combat_state::is_attacking(&combat, attacker),
                "CR 506.4c: removing the attacked battle doesn't remove the attacker (case={case})"
            );
            let target = combat_state::get_attack_target(&combat, attacker).cloned();
            assert!(
                matches!(target, Some(AttackTarget::Nothing { .. })),
                "CR 506.4c: the attacker is attacking nothing, got {target:?}"
            );
            game.turn.step = Some(Step::DeclareBlockers);
            let blocks = if blocked { vec![(blocker, attacker)] } else { vec![] };
            combat_state::declare_blockers(&game, &mut combat, blocks).expect("CR 506.4c: it may be blocked");
            assert_eq!(combat_state::is_blocked(&combat, attacker), blocked);
            game.combat = Some(combat.clone());
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::CombatDamageRegular);
            *runner.combat_mut() = combat;
            for _ in 0..6 {
                match runner.advance(&mut game, &mut queue).unwrap() {
                    TurnAction::Continue => {}
                    TurnAction::Decision(ironsmith::decisions::context::DecisionContext::Distribute(c)) => {
                        let answer = if c.total == 0 { vec![] } else { vec![(c.targets[0].target.clone(), c.total)] };
                        runner.respond_distribute(answer);
                    }
                    TurnAction::RunPriority => break,
                    other => panic!("unexpected action {other:?}"),
                }
            }
            assert_eq!(game.player(bob).unwrap().life, 20, "CR 510.1b: an unblocked creature attacking nothing deals no combat damage");
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(
                game.damage_on(blocker),
                if blocked { 3 } else { 0 },
                "CR 506.4c: a blocked creature attacking nothing still damages its blocker"
            );
        }
    }
}

#[test]
fn cr_509_1e_defender_activates_mana_abilities_to_pay_a_blocking_mana_cost() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::decisions::context::ManaPaymentContext;
    use ironsmith::mana_payment::ManaPaymentResponse;
    use ironsmith::{
        Ability, AttackerDeclaration, BlockerDeclaration, CardId, CardType, ManaCost, ManaSymbol,
        ObjectFilter, Phase, PowerToughness, StaticAbility, Step, Zone,
    };
    struct Pay;
    impl DecisionMaker for Pay {
        fn decide_mana_payment(&mut self, game: &GameState, ctx: &ManaPaymentContext) -> ManaPaymentResponse {
            assert_eq!(ctx.player, PlayerId::from_index(1));
            assert!(game.stack.is_empty());
            ManaPaymentResponse::Confirm { plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash }
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    let attacker = CardDefinitionBuilder::new(CardId::new(), "Taxed attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 8))
        .with_ability(Ability::static_ability(StaticAbility::haste()))
        .build();
    let attacker = game.create_object_from_definition(&attacker, alice, Zone::Battlefield);
    let tax = CardDefinitionBuilder::new(CardId::new(), "Blocking tax")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::block_cost(
            ObjectFilter::creature(),
            ObjectFilter::creature(),
            TotalCost::from_costs(vec![Cost::mana(ManaCost::new().add_generic(1))]),
            "Creatures can't block unless their controller pays {1} for each",
        )))
        .build();
    game.create_object_from_definition(&tax, alice, Zone::Battlefield);
    let land = CardDefinitionBuilder::new(CardId::new(), "Defender's mana rock")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Colorless]))
        .build();
    let land = game.create_object_from_definition(&land, bob, Zone::Battlefield);
    let blocker = CardDefinitionBuilder::new(CardId::new(), "Taxed blocker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 8))
        .build();
    let blocker = game.create_object_from_definition(&blocker, bob, Zone::Battlefield);
    let mut combat = combat_state::new_combat();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut queue,
        &[AttackerDeclaration { creature: attacker, target: AttackTarget::Player(bob) }],
    )
    .unwrap();
    game.combat = Some(combat.clone());
    game.turn.step = Some(Step::DeclareBlockers);
    game.turn.priority_player = None;
    let mut dm = Pay;
    let transaction = ironsmith::game_loop::begin_multiplayer_blocker_declaration_transaction(
        &game,
        &combat,
        &queue,
        &[BlockerDeclaration { blocker, blocking: attacker }],
        &mut dm,
    )
    .unwrap();
    assert_eq!(
        transaction.mana_cost_payers(),
        vec![bob],
        "CR 509.1e: the defending player gets a mana-ability window for the blocking mana cost"
    );
    transaction.stage_proposed_combat_for_payment(&mut game).unwrap();
    assert_eq!(game.player(bob).unwrap().mana_pool.colorless, 0);
    ironsmith::special_actions::perform_activate_mana_ability(&mut game, bob, land, 0, &mut dm)
        .expect("CR 509.1e: the defending player may activate a mana ability during the blocking-cost window");
    assert!(game.stack.is_empty(), "CR 605: the mana ability doesn't use the stack");
    assert_eq!(game.player(bob).unwrap().mana_pool.colorless, 1);
    ironsmith::game_loop::finish_blocker_declaration_transaction(
        transaction,
        &mut game,
        &mut combat,
        &mut queue,
        &mut dm,
    )
    .unwrap();
    game.combat = Some(combat);
    assert!(game.is_tapped(land));
    assert_eq!(game.player(bob).unwrap().mana_pool.colorless, 0, "CR 509.1e/f: produced mana pays the blocking cost");
    assert!(combat_state::is_blocking(game.combat.as_ref().unwrap(), blocker));
}

#[test]
fn cr_510_1c_d_multiple_blockers_and_multi_blocking_creature_divide_damage_as_controller_chooses() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::game_state::Target;
    use ironsmith::{
        Ability, CardId, CardType, Phase, PowerToughness, StaticAbility, Step, TurnAction,
        TurnRunner, TurnRunnerState, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature = |game: &mut GameState, owner, power, extra: Option<Ability>| {
        let mut b = CardDefinitionBuilder::new(CardId::new(), "Division participant")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, 10))
            .with_ability(Ability::static_ability(StaticAbility::haste()));
        if let Some(extra) = extra {
            b = b.with_ability(extra);
        }
        game.create_object_from_definition(&b.build(), owner, Zone::Battlefield)
    };
    // 510.1c: one attacker (power 4) blocked by two creatures; split 1/3 and 3/1.
    for split in [(1u32, 3u32), (3, 1), (0, 4)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let attacker = creature(&mut game, alice, 4, None);
        let b1 = creature(&mut game, bob, 2, None);
        let b2 = creature(&mut game, bob, 2, None);
        let mut combat = combat_state::new_combat();
        combat_state::declare_attackers(&mut game, &mut combat, vec![(attacker, AttackTarget::Player(bob))]).unwrap();
        game.turn.step = Some(Step::DeclareBlockers);
        combat_state::declare_blockers(&game, &mut combat, vec![(b1, attacker), (b2, attacker)]).unwrap();
        game.combat = Some(combat.clone());
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::CombatDamageRegular);
        *runner.combat_mut() = combat;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut prompted = false;
        for _ in 0..10 {
            match runner.advance(&mut game, &mut queue).unwrap() {
                TurnAction::Continue => {}
                TurnAction::Decision(DecisionContext::Distribute(c)) => {
                    if c.total == 4 && c.targets.len() >= 2 {
                        assert_eq!(c.player, alice, "CR 510.1c: the attacker's controller divides its damage");
                        let names: Vec<_> = c.targets.iter().map(|t| t.target.clone()).collect();
                        assert!(names.contains(&Target::Object(b1)) && names.contains(&Target::Object(b2)));
                        assert!(!names.contains(&Target::Player(bob)), "CR 510.1c: a blocked creature without trample can't assign to the player");
                        let mut answer = Vec::new();
                        if split.0 > 0 { answer.push((Target::Object(b1), split.0)); }
                        if split.1 > 0 { answer.push((Target::Object(b2), split.1)); }
                        runner.respond_distribute(answer);
                        prompted = true;
                    } else {
                        let answer = if c.total == 0 { vec![] } else { vec![(c.targets[0].target.clone(), c.total)] };
                        runner.respond_distribute(answer);
                    }
                }
                TurnAction::RunPriority => break,
                other => panic!("unexpected action {other:?}"),
            }
        }
        assert!(prompted, "CR 510.1c: two blockers require the attacking player's division");
        assert_eq!(
            (game.damage_on(b1), game.damage_on(b2)),
            split,
            "CR 510.1c: damage divided as the attacking player chose"
        );
        assert_eq!(game.damage_on(attacker), 4, "CR 510.1d: each blocker deals its damage to the attacker");
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
    // 510.1d: one blocker (power 3) blocking two attackers; split 1/2 and 2/1.
    for split in [(1u32, 2u32), (2, 1)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let a1 = creature(&mut game, alice, 1, None);
        let a2 = creature(&mut game, alice, 1, None);
        let blocker = creature(
            &mut game,
            bob,
            3,
            Some(Ability::static_ability(StaticAbility::can_block_additional_creature_each_combat(1))),
        );
        let mut combat = combat_state::new_combat();
        combat_state::declare_attackers(
            &mut game,
            &mut combat,
            vec![(a1, AttackTarget::Player(bob)), (a2, AttackTarget::Player(bob))],
        )
        .unwrap();
        game.turn.step = Some(Step::DeclareBlockers);
        combat_state::declare_blockers(&game, &mut combat, vec![(blocker, a1), (blocker, a2)]).unwrap();
        game.combat = Some(combat.clone());
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::CombatDamageRegular);
        *runner.combat_mut() = combat;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut prompted = false;
        for _ in 0..10 {
            match runner.advance(&mut game, &mut queue).unwrap() {
                TurnAction::Continue => {}
                TurnAction::Decision(DecisionContext::Distribute(c)) => {
                    if c.total == 3 && c.targets.len() >= 2 {
                        assert_eq!(c.player, bob, "CR 510.1d: the blocker's controller divides its damage");
                        runner.respond_distribute(vec![(Target::Object(a1), split.0), (Target::Object(a2), split.1)]);
                        prompted = true;
                    } else {
                        let answer = if c.total == 0 { vec![] } else { vec![(c.targets[0].target.clone(), c.total)] };
                        runner.respond_distribute(answer);
                    }
                }
                TurnAction::RunPriority => break,
                other => panic!("unexpected action {other:?}"),
            }
        }
        assert!(prompted, "CR 510.1d: a creature blocking two attackers requires its controller's division");
        assert_eq!((game.damage_on(a1), game.damage_on(a2)), split, "CR 510.1d: damage divided as the blocking player chose");
        assert_eq!(game.damage_on(blocker), 2);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
}

#[test]
fn cr_508_3a_508_4c_entering_attacker_ignores_attack_restrictions_and_never_attacked() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::effects::{CreateTokenEffect, MoveToZoneEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, AttackerDeclaration, CardId, CardType, ChooseSpec, Effect, Phase, PowerToughness,
        StaticAbility, Step, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for token in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let declared = CardDefinitionBuilder::new(CardId::new(), "Declared attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_ability(Ability::static_ability(StaticAbility::haste()))
            .with_ability(Ability::triggered(
                Trigger::this_attacks(),
                vec![Effect::energy_counters_player(1, PlayerFilter::You)],
            ))
            .build();
        let declared = game.create_object_from_definition(&declared, alice, Zone::Battlefield);
        let mut combat = combat_state::new_combat();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[AttackerDeclaration { creature: declared, target: AttackTarget::Player(bob) }],
        )
        .unwrap();
        assert_eq!(queue.entries.len(), 1, "CR 508.3a: a declared attacker's attacks ability triggers");
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        game.combat = Some(combat);
        let entering = CardDefinitionBuilder::new(CardId::new(), "Restricted entering attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 3))
            .with_ability(Ability::static_ability(StaticAbility::cant_attack()))
            .with_ability(Ability::triggered(
                Trigger::this_attacks(),
                vec![Effect::energy_counters_player(5, PlayerFilter::You)],
            ))
            .build();
        let before = game.battlefield.clone();
        if token {
            CreateTokenEffect::one(entering)
                .attacking_player(PlayerFilter::Specific(bob))
                .execute(&mut game, &mut EffectContext::new_default(declared, alice))
                .unwrap();
        } else {
            let in_hand = game.create_object_from_definition(&entering, alice, Zone::Hand);
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(in_hand), Zone::Battlefield, false)
                .under_you_control()
                .attacking_player_only(PlayerFilter::Specific(bob))
                .execute(&mut game, &mut EffectContext::new_default(declared, alice))
                .unwrap();
        }
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        let new: Vec<_> = game.battlefield.iter().copied().filter(|id| !before.contains(id)).collect();
        assert_eq!(new.len(), 1);
        let new = new[0];
        assert!(game.is_summoning_sick(new));
        assert!(
            combat_state::is_attacking(game.combat.as_ref().unwrap(), new),
            "CR 508.4c: a creature put onto the battlefield attacking ignores can't-attack and summoning sickness (token={token})"
        );
        assert!(
            queue.entries.is_empty(),
            "CR 508.3a: an attacks ability doesn't trigger for a creature put onto the battlefield attacking (token={token})"
        );
        assert_eq!(game.player(alice).unwrap().energy_counters, 1);
    }
}
