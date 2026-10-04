use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::events::DamageTarget;
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event;
use ironsmith::game_loop::execute_combat_damage_step;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

fn setup_game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}

fn create_creature(
    game: &mut GameState,
    name: &str,
    owner: PlayerId,
    power: i32,
    toughness: i32,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

#[test]
fn repeated_static_heads_toughness_assignment_executes_for_each_subject_scope() {
    for (text, own_damage, opposing_damage) in [
        (
            "This creature assigns combat damage equal to its toughness rather than its power.",
            2,
            4,
        ),
        (
            "Each creature assigns combat damage equal to its toughness rather than its power.",
            5,
            1,
        ),
        (
            "Each creature you control assigns combat damage equal to its toughness rather than its power.",
            5,
            4,
        ),
    ] {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = compile_to_runtime_definition(
            "Toughness Rule",
            format!("Type: Creature\nPower/Toughness: 3/7\n{text}"),
            false,
        )
        .expect("toughness assignment must parse through document dispatch");
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let own = create_creature(&mut game, "Own Creature", alice, 2, 5);
        let opposing = create_creature(&mut game, "Opposing Creature", bob, 4, 1);
        for (attacker, defender, amount) in [
            (source, bob, 7),
            (own, bob, own_damage),
            (opposing, alice, opposing_damage),
        ] {
            let mut combat = CombatState::default();
            combat
                .attackers
                .push(ironsmith::combat_state::AttackerInfo {
                    creature: attacker,
                    target: AttackTarget::Player(defender),
                });
            combat.blockers.insert(attacker, Vec::new());
            let events = execute_combat_damage_step(&mut game, &combat, false);
            assert_eq!(events.len(), 1, "{text}");
            assert_eq!(events[0].amount, amount, "{text}: attacker {attacker:?}");
        }
        game.object_mut(source).unwrap().abilities_mut().clear();
        let mut combat = CombatState::default();
        combat
            .attackers
            .push(ironsmith::combat_state::AttackerInfo {
                creature: own,
                target: AttackTarget::Player(bob),
            });
        combat.blockers.insert(own, Vec::new());
        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].amount, 2,
            "removing the rule must restore power-based damage"
        );
    }
}

fn damage_amount(
    game: &mut GameState,
    source: ObjectId,
    target: DamageTarget,
    combat: bool,
) -> u32 {
    let controller = game.controller_of_id(source).unwrap();
    process_damage_assignments_with_event(
        game,
        source,
        target,
        3,
        combat,
        EventCause::from_effect(source, controller),
    )
    .expect("damage proposal must execute")
    .assignments
    .iter()
    .map(|assignment| assignment.amount)
    .sum()
}

#[test]
fn repeated_static_heads_attached_prevention_preserves_direction_combat_and_host() {
    for (text, outgoing_noncombat, outgoing_combat, incoming) in [
        (
            "Prevent all damage that would be dealt by enchanted creature.",
            0,
            0,
            3,
        ),
        (
            "Prevent all combat damage that would be dealt by enchanted creature.",
            3,
            0,
            3,
        ),
        (
            "Prevent all damage that would be dealt to enchanted creature.",
            3,
            3,
            0,
        ),
        (
            "Prevent all damage that would be dealt to and dealt by enchanted creature.",
            0,
            0,
            0,
        ),
    ] {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let host = create_creature(&mut game, "Opponent's Enchanted Creature", bob, 3, 5);
        let other = create_creature(&mut game, "Unenchanted Creature", alice, 3, 5);
        let aura = compile_to_runtime_definition(
            "Prevention Aura",
            format!("Type: Enchantment — Aura\nEnchant creature\n{text}"),
            false,
        )
        .expect("attached prevention must parse through document dispatch");
        let aura_id = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
        game.object_mut(aura_id).unwrap().attached_to =
            Some(ironsmith::object::AttachmentTarget::Object(host));
        game.object_mut(host).unwrap().attachments.push(aura_id);
        game.refresh_continuous_state()
            .expect("attachment effects must refresh");
        assert_eq!(
            damage_amount(&mut game, host, DamageTarget::Player(alice), false),
            outgoing_noncombat,
            "{text}"
        );
        assert_eq!(
            damage_amount(&mut game, host, DamageTarget::Player(alice), true),
            outgoing_combat,
            "{text}"
        );
        assert_eq!(
            damage_amount(&mut game, other, DamageTarget::Object(host), false),
            incoming,
            "{text}"
        );
        assert_eq!(
            damage_amount(&mut game, other, DamageTarget::Player(bob), false),
            3,
            "unrelated damage must remain unaffected: {text}"
        );
        game.move_object_by_effect(aura_id, Zone::Graveyard)
            .unwrap();
        assert_eq!(
            damage_amount(&mut game, host, DamageTarget::Player(alice), true),
            3,
            "prevention must stop when the Aura leaves: {text}"
        );
    }
}

#[test]
fn repeated_static_heads_land_animation_preserves_subtype_filter_and_land_type() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let rule = compile_to_runtime_definition(
        "Forest Animation",
        "Type: Enchantment\nAll Forests are 1/1 creatures that are still lands.",
        false,
    )
    .expect("land animation must parse through document dispatch");
    let source = game.create_object_from_definition(&rule, alice, Zone::Battlefield);
    // Public compilation keeps these fixtures independent of the handwritten
    // catalog, which is not enabled for external runtime integration tests.
    let forest = compile_to_runtime_definition("Forest", "Type: Basic Land — Forest", false).unwrap();
    let mountain = compile_to_runtime_definition("Mountain", "Type: Basic Land — Mountain", false).unwrap();
    let own = game.create_object_from_definition(
        &forest,
        alice,
        Zone::Battlefield,
    );
    let opposing = game.create_object_from_definition(
        &forest,
        bob,
        Zone::Battlefield,
    );
    let excluded = game.create_object_from_definition(
        &mountain,
        alice,
        Zone::Battlefield,
    );
    for forest in [own, opposing] {
        let characteristics = game.calculated_characteristics(forest).unwrap();
        assert!(characteristics.card_types.contains(&CardType::Creature));
        assert!(characteristics.card_types.contains(&CardType::Land));
        assert!(characteristics.subtypes.contains(&Subtype::Forest));
        assert_eq!(
            (characteristics.power, characteristics.toughness),
            (Some(1), Some(1))
        );
    }
    assert!(
        !game
            .calculated_characteristics(excluded)
            .unwrap()
            .card_types
            .contains(&CardType::Creature)
    );
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    assert!(
        !game
            .calculated_characteristics(own)
            .unwrap()
            .card_types
            .contains(&CardType::Creature)
    );
}

#[test]
fn repeated_static_heads_redirects_your_player_and_permanents_but_not_opponents() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let rule = compile_to_runtime_definition(
        "Damage Interceptor",
        "Type: Creature\nPower/Toughness: 2/8\nAll damage that would be dealt to you and other permanents you control is dealt to this creature instead.",
        false,
    )
        .expect("damage redirection must parse through document dispatch");
    let interceptor = game.create_object_from_definition(&rule, alice, Zone::Battlefield);
    let own = create_creature(&mut game, "Own Creature", alice, 2, 4);
    let opposing = create_creature(&mut game, "Opposing Creature", bob, 2, 4);
    for (target, expected) in [
        (
            DamageTarget::Player(alice),
            DamageTarget::Object(interceptor),
        ),
        (DamageTarget::Object(own), DamageTarget::Object(interceptor)),
        (
            DamageTarget::Object(interceptor),
            DamageTarget::Object(interceptor),
        ),
        (DamageTarget::Player(bob), DamageTarget::Player(bob)),
        (
            DamageTarget::Object(opposing),
            DamageTarget::Object(opposing),
        ),
    ] {
        let outcome = process_damage_assignments_with_event(
            &mut game,
            opposing,
            target,
            3,
            false,
            EventCause::from_effect(opposing, bob),
        )
        .unwrap();
        assert_eq!(outcome.assignments.len(), 1);
        assert_eq!(outcome.assignments[0].target, expected);
        assert_eq!(outcome.assignments[0].amount, 3);
    }
}

#[test]
fn repeated_static_heads_controller_redirection_keeps_destination_and_source_filter() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let rule = compile_to_runtime_definition(
        "Controller Redirection",
        "Type: Enchantment\nIf a creature would deal damage to you, it deals that damage to its controller instead.",
        false,
    )
        .expect("controller damage redirection must parse");
    let enchantment = game.create_object_from_definition(&rule, alice, Zone::Battlefield);
    let opposing = create_creature(&mut game, "Opposing Creature", bob, 2, 4);
    for (source, expected) in [
        (opposing, DamageTarget::Player(bob)),
        (enchantment, DamageTarget::Player(alice)),
    ] {
        let outcome = process_damage_assignments_with_event(
            &mut game,
            source,
            DamageTarget::Player(alice),
            3,
            false,
            EventCause::from_effect(source, alice),
        )
        .unwrap();
        assert_eq!(outcome.assignments.len(), 1);
        assert_eq!(outcome.assignments[0].target, expected);
    }
}

#[test]
fn repeated_static_heads_attachment_color_choice_executes_when_equipment_attaches() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let creature = create_creature(&mut game, "Equipped Creature", alice, 2, 4);
    let equipment = compile_to_runtime_definition(
        "Color Equipment",
        "Type: Artifact — Equipment\nAs this Equipment becomes attached to a creature, choose a color.",
        false,
    )
        .expect("attachment choice must parse");
    let source = game.create_object_from_definition(&equipment, alice, Zone::Battlefield);
    assert_eq!(game.chosen_color(source), None);
    let effect = Effect::attach_objects(ChooseSpec::Source, ChooseSpec::SpecificObject(creature));
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx)
        .expect("equipment must attach");
    assert_eq!(
        game.chosen_color(source),
        Some(ironsmith::color::Color::White)
    );
    assert_eq!(
        game.object(source).unwrap().attached_to,
        Some(ironsmith::object::AttachmentTarget::Object(creature))
    );
}
