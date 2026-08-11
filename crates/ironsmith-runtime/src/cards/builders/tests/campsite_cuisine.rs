#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Whenever this enchantment or a legendary creature you control enters, create a Food token.\nWhenever you attack, you may sacrifice X Foods. When you do, up to X target attacking creatures each get +3/+3 and gain trample and indestructible until end of turn.";

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn food(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Food])
        .build()
}

fn attack_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::AttacksTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Campsite Cuisine should retain its attack trigger")
}

#[test]
fn campsite_cuisine_keeps_disjoint_entry_subjects_and_linked_plural_grants() {
    let definition = parse_oracle_card_definition("Campsite Cuisine");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let entry_trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::OrTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Campsite Cuisine should retain both entry branches");
    let entry_debug = format!("{:#?}", entry_trigger.trigger);
    assert!(entry_debug.contains("this enchantment"), "{entry_debug}");
    assert!(entry_debug.contains("Legendary"), "{entry_debug}");
    assert!(entry_debug.contains("Creature"), "{entry_debug}");

    let attack = attack_trigger(&definition);
    let debug = format!("{:#?}", attack.effects);
    assert!(debug.contains("ReflexiveTriggerEffect"), "{debug}");
    assert!(debug.contains("predicate: Happened"), "{debug}");
    assert!(debug.contains("dynamic_x: true"), "{debug}");
    assert!(debug.contains("attacking: true"), "{debug}");
    assert!(debug.contains("pumped_0"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
    assert!(debug.contains("Indestructible"), "{debug}");

    let [_, reflexive_segment] = attack.effects.segments.as_slice() else {
        panic!("expected sacrifice and reflexive segments: {attack:#?}");
    };
    let [reflexive_root] = reflexive_segment.default_effects.as_slice() else {
        panic!("expected one reflexive root: {reflexive_segment:#?}");
    };
    let reflexive = reflexive_root
        .downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
        .expect("typed reflexive result");
    let sequence = reflexive.effects[0]
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("coordinated pump and grant");
    let [pump_root, grant_root] = sequence.effects.as_slice() else {
        panic!("expected one pump and one grant: {sequence:#?}");
    };
    let pump_tag = pump_root
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the selected target set must be tagged")
        .tag
        .clone();
    let grant = grant_root
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the grant keeps result provenance")
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("typed keyword grant");
    assert!(matches!(
        grant.target_spec.as_ref().map(|spec| spec.unhinted()),
        Some(ChooseSpec::Tagged(tag)) if tag == &pump_tag
    ));
}

#[test]
fn campsite_cuisine_applies_every_grant_to_each_selected_attacker_only() {
    let definition = parse_oracle_card_definition("Campsite Cuisine");
    let attack = attack_trigger(&definition);
    let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let first = game.create_object_from_definition(&creature("First"), alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&creature("Second"), alice, Zone::Battlefield);
    let unselected =
        game.create_object_from_definition(&creature("Unselected"), alice, Zone::Battlefield);
    game.remove_summoning_sickness(first);
    game.remove_summoning_sickness(second);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    let bob = PlayerId::from_index(1);
    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[
            crate::decision::AttackerDeclaration {
                creature: first,
                target: crate::combat_state::AttackTarget::Player(bob),
            },
            crate::decision::AttackerDeclaration {
                creature: second,
                target: crate::combat_state::AttackTarget::Player(bob),
            },
        ],
    )
    .expect("the selected creatures should be attacking");
    let foods = [
        game.create_object_from_definition(&food("Food One"), alice, Zone::Battlefield),
        game.create_object_from_definition(&food("Food Two"), alice, Zone::Battlefield),
    ];
    let food_stable_ids = foods.map(|food| game.object(food).expect("Food exists").stable_id);

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    ctx.x_value = Some(2);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &attack.effects,
        None,
        &[],
    )
    .expect("the optional sacrifice should create its reflexive trigger");
    drop(ctx);
    assert_eq!(
        game.stack.len(),
        1,
        "the successful sacrifice creates one reflexive ability"
    );
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("the reflexive group grant should resolve");

    for stable_id in food_stable_ids {
        let food = game
            .find_object_by_stable_id(stable_id)
            .expect("sacrificed Food remains represented in the graveyard");
        assert_eq!(
            game.object(food).map(|object| object.zone),
            Some(Zone::Graveyard)
        );
    }
    for selected in [first, second] {
        assert_eq!(game.calculated_power(selected), Some(5));
        assert_eq!(game.calculated_toughness(selected), Some(5));
        assert!(game.object_has_static_ability_id(selected, StaticAbilityId::Trample));
        assert!(game.object_has_static_ability_id(selected, StaticAbilityId::Indestructible));
    }
    assert_eq!(game.calculated_power(unselected), Some(2));
    assert!(!game.object_has_static_ability_id(unselected, StaticAbilityId::Trample));
    assert!(!game.object_has_static_ability_id(unselected, StaticAbilityId::Indestructible));
}
