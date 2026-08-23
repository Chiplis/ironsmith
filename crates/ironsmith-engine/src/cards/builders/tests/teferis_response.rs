#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn nested_counter(effect: &crate::effect::Effect) -> Option<crate::effects::CounterEffect> {
    if let Some(counter) = effect.downcast_ref::<crate::effects::CounterEffect>() {
        return Some(counter.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = nested_counter(child);
        }
    });
    found
}

fn response_counter(definition: &CardDefinition) -> crate::effects::CounterEffect {
    definition
        .spell_effect
        .as_ref()
        .expect("Teferi's Response spell program")
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(nested_counter)
        .expect("Teferi's Response counter effect")
}

fn permanent(name: &str, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

fn targeting_spell(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    target: ObjectId,
) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build();
    let spell = game.create_object_from_definition(&definition, controller, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, controller)
            .with_targets(vec![crate::game_state::Target::Object(target)]),
    );
    spell
}

fn targeting_ability(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    target: ObjectId,
) -> ObjectId {
    let source = game.create_object_from_definition(
        &permanent(name, CardType::Artifact),
        controller,
        Zone::Battlefield,
    );
    game.push_to_stack(
        crate::game_state::StackEntry::ability(
            source,
            controller,
            vec![crate::effect::Effect::draw(1)],
        )
        .with_targets(vec![crate::game_state::Target::Object(target)]),
    );
    source
}

#[test]
fn teferis_response_keeps_both_stack_kinds_and_the_shared_land_relation() {
    let definition = parse_oracle_card_definition("Teferi's Response");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Counter target spell or ability an opponent controls that targets a land you control. If a permanent's ability is countered this way, destroy that permanent.",
            "Draw two cards.",
        ],
        "{definition:#?}",
    );

    let counter = response_counter(&definition);
    let ChooseSpec::Object(filter) = counter.target.base() else {
        panic!(
            "counter target should be an object union: {:#?}",
            counter.target
        );
    };
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    for branch in &filter.any_of {
        assert_eq!(
            branch.controller,
            Some(PlayerFilter::Opponent),
            "{branch:#?}"
        );
        let land = branch
            .targets_object
            .as_ref()
            .expect("both stack branches share the land-target relation");
        assert_eq!(land.card_types, [CardType::Land], "{land:#?}");
        assert_eq!(land.controller, Some(PlayerFilter::You), "{land:#?}");
        assert!(branch.targets_only_object.is_none(), "{branch:#?}");
    }
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| { branch.stack_kind == Some(crate::filter::StackObjectKind::Spell) })
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| { branch.stack_kind == Some(crate::filter::StackObjectKind::Ability) })
    );

    let debug = format!("{definition:#?}");
    assert!(debug.contains("PriorEffectResult"), "{debug}");
    assert!(debug.contains("action: Countered"), "{debug}");
    assert!(debug.contains("countered_0"), "{debug}");
    assert!(debug.contains("DestroyEffect"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn teferis_response_legality_requires_opponent_control_and_own_land_target() {
    let definition = parse_oracle_card_definition("Teferi's Response");
    let counter = response_counter(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let response = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let alice_land = game.create_object_from_definition(
        &permanent("Alice Land", CardType::Land),
        alice,
        Zone::Battlefield,
    );
    let bob_land = game.create_object_from_definition(
        &permanent("Bob Land", CardType::Land),
        bob,
        Zone::Battlefield,
    );
    let alice_creature = game.create_object_from_definition(
        &permanent("Alice Creature", CardType::Creature),
        alice,
        Zone::Battlefield,
    );

    let eligible_spell =
        targeting_spell(&mut game, bob, "Bob Spell Targeting Alice Land", alice_land);
    let eligible_ability = targeting_ability(
        &mut game,
        bob,
        "Bob Ability Targeting Alice Land",
        alice_land,
    );
    let wrong_controller = targeting_spell(
        &mut game,
        alice,
        "Alice Spell Targeting Alice Land",
        alice_land,
    );
    let wrong_land = targeting_spell(&mut game, bob, "Bob Spell Targeting Bob Land", bob_land);
    let wrong_object = targeting_spell(
        &mut game,
        bob,
        "Bob Spell Targeting Alice Creature",
        alice_creature,
    );

    let legal =
        crate::game_loop::compute_legal_targets(&game, &counter.target, alice, Some(response));
    assert!(legal.contains(&crate::game_state::Target::Object(eligible_spell)));
    assert!(legal.contains(&crate::game_state::Target::Object(eligible_ability)));
    assert!(!legal.contains(&crate::game_state::Target::Object(wrong_controller)));
    assert!(!legal.contains(&crate::game_state::Target::Object(wrong_land)));
    assert!(!legal.contains(&crate::game_state::Target::Object(wrong_object)));
}

#[test]
fn teferis_response_destroys_only_the_source_of_the_countered_ability_and_draws() {
    let definition = parse_oracle_card_definition("Teferi's Response");
    let counter = response_counter(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alice_land = game.create_object_from_definition(
        &permanent("Alice Land", CardType::Land),
        alice,
        Zone::Battlefield,
    );
    let ability_source = targeting_ability(&mut game, bob, "Countered Permanent", alice_land);
    let ability_stable = game
        .object(ability_source)
        .expect("ability source")
        .stable_id;
    for name in ["First Draw", "Second Draw"] {
        game.create_object_from_definition(
            &permanent(name, CardType::Artifact),
            alice,
            Zone::Library,
        );
    }
    let hand_before = game.objects_in_zone(Zone::Hand).len();
    let response = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(response, alice)
            .with_targets(vec![crate::game_state::Target::Object(ability_source)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: counter.target,
                range: 0..1,
            }]),
    );

    crate::game_loop::resolve_stack_entry(&mut game).expect("Teferi's Response should resolve");
    let source_after = game
        .find_object_by_stable_id(ability_stable)
        .expect("countered ability source remains tracked");
    assert_eq!(
        game.object(source_after)
            .expect("countered ability source")
            .zone,
        Zone::Graveyard,
    );
    assert_eq!(game.objects_in_zone(Zone::Hand).len(), hand_before + 2);
}
