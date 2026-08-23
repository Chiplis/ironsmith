#![cfg(ironsmith_runtime_parser_tests)]

use super::*;

const ORACLE: &str = "Put X +1/+1 counters on target artifact you control. If it isn't a creature or Vehicle, it becomes a 0/0 Construct artifact creature.";

fn definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(97_100), "Lifecraft Awakening")
        .card_types(vec![CardType::Sorcery])
        .parse_text(ORACLE)
        .expect("Lifecraft Awakening should compile")
}

fn find_nested_effect<T: 'static>(effect: &crate::effect::Effect) -> Option<&T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found);
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested_effect::<T>(child).map(|value| value as *const T);
        }
    });
    // Child effects are owned by `effect`, so the pointer remains valid for
    // the lifetime of the borrowed root effect.
    unsafe { found.map(|pointer| &*pointer) }
}

fn resolve_for_target(
    game: &mut crate::game_state::GameState,
    definition: &CardDefinition,
    controller: PlayerId,
    target: ObjectId,
    x_value: u32,
) {
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Lifecraft Awakening should have a spell program");
    let target_spec = program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| find_nested_effect::<crate::effects::PutCountersEffect>(effect))
        .expect("the spell should put counters on its target")
        .target
        .clone();
    let source = game.create_object_from_definition(definition, controller, Zone::Stack);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, controller)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..1,
        }]);
    ctx.x_value = Some(x_value);
    ctx.snapshot_targets(game);

    for effect in program.flattened_default_effects() {
        crate::effects::execute_effect(game, effect, &mut ctx)
            .expect("Lifecraft Awakening should resolve");
    }
}

#[test]
fn lifecraft_awakening_round_trips_as_one_target_bound_conditional() {
    let definition = definition();
    assert!(
        definition.abilities.is_empty(),
        "the animation must not become a global static ability: {definition:#?}"
    );
    assert_eq!(canonical_compiled_lines(&definition).join(" "), ORACLE);
}

#[test]
fn lifecraft_awakening_animates_only_the_chosen_noncreature_nonvehicle_artifact() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let artifact = |raw_id, name| {
        CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
            .card_types(vec![CardType::Artifact])
            .build()
    };
    let chosen = game.create_object_from_definition(
        &artifact(97_101, "Chosen Implement"),
        alice,
        Zone::Battlefield,
    );
    let bystander = game.create_object_from_definition(
        &artifact(97_102, "Bystander Implement"),
        alice,
        Zone::Battlefield,
    );

    resolve_for_target(&mut game, &definition, alice, chosen, 3);

    assert_eq!(
        game.counter_count(chosen, crate::object::CounterType::PlusOnePlusOne),
        3
    );
    assert!(game.current_is_creature(chosen));
    assert!(game.current_has_card_type(chosen, CardType::Artifact));
    assert!(game.current_has_subtype(chosen, Subtype::Construct));
    assert_eq!(
        (
            game.calculated_power(chosen),
            game.calculated_toughness(chosen)
        ),
        (Some(3), Some(3))
    );

    assert_eq!(
        game.counter_count(bystander, crate::object::CounterType::PlusOnePlusOne),
        0
    );
    assert!(!game.current_is_creature(bystander));
    assert!(!game.current_has_subtype(bystander, Subtype::Construct));
}

#[test]
fn lifecraft_awakening_does_not_animate_a_vehicle_target() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let vehicle = CardDefinitionBuilder::new(CardId::from_raw(97_103), "Chosen Vehicle")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .build();
    let chosen = game.create_object_from_definition(&vehicle, alice, Zone::Battlefield);

    resolve_for_target(&mut game, &definition, alice, chosen, 2);

    assert_eq!(
        game.counter_count(chosen, crate::object::CounterType::PlusOnePlusOne),
        2
    );
    assert!(!game.current_is_creature(chosen));
    assert!(game.current_has_subtype(chosen, Subtype::Vehicle));
    assert!(!game.current_has_subtype(chosen, Subtype::Construct));
}
