#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn creature(name: &str, subtype: Subtype, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn permanent_card(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

#[test]
fn akawalli_keeps_both_descend_lines_and_stacked_executable_bonuses() {
    let definition = parse_oracle_card_definition("Akawalli, the Seething Tower");
    assert_eq!(
        definition.card.power_toughness,
        Some(PowerToughness::fixed(3, 3)),
        "the cards.json regression loader must retain printed base P/T"
    );
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Descend 4 — As long as there are four or more permanent cards in your graveyard, Akawalli gets +2/+2 and has trample.",
            "Descend 8 — As long as there are eight or more permanent cards in your graveyard, Akawalli gets an additional +2/+2 and can't be blocked by more than one creature.",
        ]
    );
    let debug = format!("{:#?}", definition.abilities);
    let source_line_groups = definition
        .abilities
        .iter()
        .filter(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.compiled_model().is_some_and(|model| matches!(
                        &model.payload,
                        ironsmith_core::StaticAbilityPayload::SourceLineStaticGroup { .. }
                    ))
            )
        })
        .count();
    assert_eq!(source_line_groups, 2, "{debug}");
    assert!(debug.contains("additional_surface: true"), "{debug}");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let akawalli = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let calculated = game.calculated_characteristics(akawalli);
    let object_snapshot = game.object(akawalli).map(|object| {
        (
            object.zone,
            object.card_types.clone(),
            object.power(),
            object.toughness(),
        )
    });
    assert_eq!(
        calculated.as_ref().and_then(|chars| chars.power),
        Some(3),
        "printed={:?}; object={object_snapshot:?}; calculated={calculated:#?}",
        definition.card.power_toughness,
    );
    assert!(!game.object_has_static_ability_id(akawalli, StaticAbilityId::Trample));

    for index in 0..4 {
        game.create_object_from_definition(
            &permanent_card(&format!("First Descend Card {index}")),
            alice,
            Zone::Graveyard,
        );
    }
    assert_eq!(game.calculated_power(akawalli), Some(5));
    assert!(game.object_has_static_ability_id(akawalli, StaticAbilityId::Trample));

    for index in 0..4 {
        game.create_object_from_definition(
            &permanent_card(&format!("Second Descend Card {index}")),
            alice,
            Zone::Graveyard,
        );
    }
    assert_eq!(game.calculated_power(akawalli), Some(7));
    assert_eq!(game.calculated_toughness(akawalli), Some(7));
}

#[test]
fn butchers_cleaver_grants_lifelink_to_an_attached_human_not_the_equipment() {
    let definition = parse_oracle_card_definition("Butcher's Cleaver");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Equipped creature gets +3/+0.",
            "As long as equipped creature is a Human, it has lifelink.",
            "Equip {3}",
        ]
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let human = game.create_object_from_definition(
        &creature("Cleaver Human", Subtype::Human, 2, 2),
        alice,
        Zone::Battlefield,
    );
    let elf = game.create_object_from_definition(
        &creature("Cleaver Elf", Subtype::Elf, 2, 2),
        alice,
        Zone::Battlefield,
    );
    let cleaver = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    assert!(game.attach_object_to_target(cleaver, crate::object::AttachmentTarget::Object(human),));
    assert!(game.object_has_static_ability_id(human, StaticAbilityId::Lifelink));
    assert!(!game.object_has_static_ability_id(cleaver, StaticAbilityId::Lifelink));
    assert!(game.detach_object_from_current_target(cleaver));
    assert!(game.attach_object_to_target(cleaver, crate::object::AttachmentTarget::Object(elf),));
    assert!(!game.object_has_static_ability_id(elf, StaticAbilityId::Lifelink));
    assert!(!game.object_has_static_ability_id(cleaver, StaticAbilityId::Lifelink));
}

fn nested_token_copy(effect: &Effect) -> Option<CreateTokenCopyEffect> {
    if let Some(copy) = effect.downcast_ref::<CreateTokenCopyEffect>() {
        return Some(copy.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = nested_token_copy(child);
        }
    });
    found
}

#[test]
fn mirror_room_adds_reflection_without_replacing_existing_creature_types() {
    let definition = parse_oracle_card_definition("Mirror Room");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "When you unlock this door, create a token that's a copy of target creature you control, except it's a Reflection in addition to its other creature types."
        ]
    );
    let copy = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(nested_token_copy),
            _ => None,
        })
        .expect("the unlock trigger should create a typed token copy");
    assert_eq!(copy.added_subtypes, [Subtype::Reflection]);
    assert!(copy.set_subtypes.is_none());

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let target = game.create_object_from_definition(
        &creature("Mirror Elf", Subtype::Elf, 3, 3),
        alice,
        Zone::Battlefield,
    );
    let room = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = crate::effects::ExecutionContext::new_default(room, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    let result = copy
        .execute(&mut game, &mut ctx)
        .expect("the typed token copy should resolve");
    let crate::effect::OutcomeValue::Objects(created) = result.value else {
        panic!("token copy should return its created object");
    };
    let token = game.object(created[0]).expect("copy token should exist");
    assert!(token.has_subtype(Subtype::Elf));
    assert!(token.has_subtype(Subtype::Reflection));
}

fn resolve_dire_tactics(control_human: bool) -> (crate::GameState, StableId) {
    let definition = parse_oracle_card_definition("Dire Tactics");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    if control_human {
        game.create_object_from_definition(
            &creature("Friendly Human", Subtype::Human, 1, 1),
            alice,
            Zone::Battlefield,
        );
    }
    let target = game.create_object_from_definition(
        &creature("Dire Target", Subtype::Beast, 2, 5),
        bob,
        Zone::Battlefield,
    );
    let stable_id = game.object(target).expect("target exists").stable_id;
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Object(target)]),
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("Dire Tactics should resolve");
    (game, stable_id)
}

#[test]
fn dire_tactics_uses_a_human_control_condition_not_exile_failure() {
    let definition = parse_oracle_card_definition("Dire Tactics");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Exile target creature. If you don't control a Human, you lose life equal to that creature's toughness."
        ]
    );
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(debug.contains("ConditionalEffect"), "{debug}");
    assert!(!debug.contains("IfEffect"), "{debug}");

    let (without_human, exiled) = resolve_dire_tactics(false);
    assert_eq!(
        without_human.player(PlayerId::from_index(0)).unwrap().life,
        15
    );
    assert_eq!(
        without_human
            .find_object_by_stable_id(exiled)
            .and_then(|object_id| without_human.object(object_id))
            .map(|object| object.zone),
        Some(Zone::Exile)
    );

    let (with_human, exiled) = resolve_dire_tactics(true);
    assert_eq!(with_human.player(PlayerId::from_index(0)).unwrap().life, 20);
    assert_eq!(
        with_human
            .find_object_by_stable_id(exiled)
            .and_then(|object_id| with_human.object(object_id))
            .map(|object| object.zone),
        Some(Zone::Exile)
    );
}
