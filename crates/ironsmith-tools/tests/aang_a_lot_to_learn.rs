//! Frozen atlas observation 24871519: dynamic vigilance and another-creature death trigger.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::CardId;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::types::Subtype;
use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, PlayerId, Zone};
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_payloads_by_name,
};
fn definition() -> CardDefinition {
    let payloads = load_card_payloads_by_name(
        default_cards_path().to_str().unwrap(),
        "Aang, A Lot to Learn",
    )
    .unwrap();
    assert_eq!(payloads.len(), 1);
    let snapshot = compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
    compile_definition_from_payload(&payloads[0]).unwrap()
}
#[test]
fn aang_vigilance_rechecks_lesson_zone_and_current_controller() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let lesson = CardDefinitionBuilder::new(CardId::new(), "Lesson fixture")
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Lesson])
        .build();
    let nonlesson = CardDefinitionBuilder::new(CardId::new(), "Non-Lesson fixture")
        .card_types(vec![CardType::Sorcery])
        .build();
    let vigil = |g: &GameState| g.object_has_static_ability_id(source, StaticAbilityId::Vigilance);
    assert!(!vigil(&game));
    game.create_object_from_definition(&nonlesson, alice, Zone::Graveyard);
    let opposing_lesson = game.create_object_from_definition(&lesson, bob, Zone::Graveyard);
    let own_lesson = game.create_object_from_definition(&lesson, alice, Zone::Hand);
    assert!(
        !vigil(&game),
        "wrong player's graveyard and own hand do not count"
    );
    let own_lesson = game
        .move_object_by_effect(own_lesson, Zone::Graveyard)
        .unwrap();
    assert!(vigil(&game));
    let second_lesson = game.create_object_from_definition(&lesson, alice, Zone::Graveyard);
    assert!(
        vigil(&game),
        "two Lessons still satisfy the existential condition"
    );
    game.set_current_controller(source, bob);
    assert!(vigil(&game));
    game.move_object_by_effect(opposing_lesson, Zone::Exile)
        .unwrap();
    assert!(!vigil(&game), "your follows controller, not owner");
    game.set_current_controller(source, alice);
    assert_eq!(game.controller_of_id(source), Some(alice));
    assert!(vigil(&game));
    game.move_object_by_effect(own_lesson, Zone::Exile).unwrap();
    assert!(vigil(&game), "one remaining Lesson is sufficient");
    game.move_object_by_effect(second_lesson, Zone::Exile)
        .unwrap();
    assert!(!vigil(&game), "static grant must disappear immediately");
}
#[test]
fn aang_conditional_vigilance_controls_attacking_tap() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::AttackerDeclaration;
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for has_lesson in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        if has_lesson {
            let lesson = CardDefinitionBuilder::new(CardId::new(), "Lesson fixture")
                .card_types(vec![CardType::Instant])
                .subtypes(vec![Subtype::Lesson])
                .build();
            game.create_object_from_definition(&lesson, alice, Zone::Graveyard);
        }
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut CombatState::default(),
            &mut TriggerQueue::new(),
            &[AttackerDeclaration {
                creature: source,
                target: AttackTarget::Player(bob),
            }],
        )
        .unwrap();
        assert_eq!(game.is_tapped(source), !has_lesson);
    }
}
#[test]
fn aang_death_trigger_keeps_another_creature_controller_and_source_identity() {
    use ironsmith::object::CounterType;
    let definition = definition();
    assert_eq!(
        definition
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .count(),
        1
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (is_creature, controller, owner, is_self, destination, leave_source, expected) in [
        (true, alice, alice, false, Zone::Graveyard, false, 1),
        (true, alice, bob, false, Zone::Graveyard, false, 1),
        (true, bob, alice, false, Zone::Graveyard, false, 0),
        (false, alice, alice, false, Zone::Graveyard, false, 0),
        (true, alice, alice, true, Zone::Graveyard, false, 0),
        (true, alice, alice, false, Zone::Exile, false, 0),
        (true, alice, alice, false, Zone::Graveyard, true, 1),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let fixture = CardDefinitionBuilder::new(CardId::new(), "Death fixture")
            .card_types(vec![if is_creature {
                CardType::Creature
            } else {
                CardType::Artifact
            }])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .build();
        let other = if is_self {
            source
        } else {
            game.create_object_from_definition(&fixture, owner, Zone::Battlefield)
        };
        game.set_current_controller(other, controller);
        game.move_object_by_effect(other, destination).unwrap();
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        assert_eq!(queue.entries.len(), expected);
        let departed_source =
            leave_source.then(|| game.move_object_by_effect(source, Zone::Exile).unwrap());
        ironsmith::game_loop::run_priority_loop_with(
            &mut game,
            &mut queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        if let Some(departed) = departed_source {
            assert_eq!(
                game.object(departed)
                    .unwrap()
                    .counters
                    .get(&CounterType::PlusOnePlusOne),
                None
            );
        } else if !is_self {
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&CounterType::PlusOnePlusOne)
                    .copied()
                    .unwrap_or(0),
                expected as u32
            );
        }
    }
}

#[test]
fn aang_sees_other_controlled_creatures_dying_simultaneously_with_it() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::object::{CounterType, ObjectKind};
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Simultaneous death fixture")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let token = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    game.object_mut(token).unwrap().kind = ObjectKind::Token;
    game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, alice, &mut dm);
    execute_effect(
        &mut game,
        &Effect::destroy_all(ironsmith::target::ObjectFilter::creature()),
        &mut ctx,
    )
    .unwrap();
    let mut queue = TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        for entry in check_triggers(&game, &event) {
            queue.add(entry);
        }
    }
    assert_eq!(
        queue.entries.len(),
        2,
        "both the other controlled token and nontoken trigger; self and opponent do not"
    );
    ironsmith::game_loop::run_priority_loop_with(
        &mut game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    for id in &game.player(alice).unwrap().graveyard {
        assert_eq!(
            game.object(*id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            None,
            "the departed source is not a battlefield recipient"
        );
    }
}
