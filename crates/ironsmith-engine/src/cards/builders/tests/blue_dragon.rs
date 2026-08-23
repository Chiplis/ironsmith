#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::AutoPassDecisionMaker;

fn lightning_breath(definition: &CardDefinition) -> &TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Blue Dragon should have Lightning Breath")
}

fn test_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build()
}

#[test]
fn blue_dragon_keeps_all_three_modifiers_and_the_shared_next_turn_duration() {
    let definition = parse_oracle_card_definition("Blue Dragon");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Flying",
            "Lightning Breath — When this creature enters, until your next turn, target creature an opponent controls gets -3/-0, up to one other target creature gets -2/-0, and up to one other target creature gets -1/-0.",
        ]
    );

    let triggered = lightning_breath(&definition);
    assert_eq!(triggered.choices.len(), 3, "{triggered:#?}");
    let [sequence] = triggered.effects.flattened_default_effects() else {
        panic!("expected one coordinated modifier sequence: {triggered:#?}");
    };
    let sequence = sequence
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the modifiers should retain their coordinated source clause");
    assert_eq!(
        sequence.surface,
        ironsmith_core::SequenceSurface::CoordinatedLeadingDuration
    );
    assert_eq!(sequence.effects.len(), 3);

    let mut powers = Vec::new();
    for (index, effect) in sequence.effects.iter().enumerate() {
        let tagged = effect
            .downcast_ref::<crate::effects::TaggedEffect>()
            .expect("each target slot should have an independent tag");
        let apply = tagged
            .effect
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("each target should receive one typed continuous modifier");
        assert_eq!(apply.until, crate::effect::Until::YourNextTurn);
        let [
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness: ironsmith_core::Value::Fixed(0),
            },
        ] = apply.runtime_modifications.as_slice()
        else {
            panic!("expected a pure power modifier: {apply:#?}");
        };
        let ironsmith_core::Value::Fixed(power) = power.unhinted() else {
            panic!("expected a fixed power modifier: {apply:#?}");
        };
        powers.push(*power);

        let spec = apply.target_spec.as_ref().expect("target spec");
        let ChooseSpec::Object(filter) = spec.base() else {
            panic!("expected a creature target: {spec:#?}");
        };
        if index == 0 {
            assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
            assert!(!filter.other);
            assert_eq!(spec.count().min, 1);
        } else {
            assert!(filter.other);
            assert_eq!(spec.count().min, 0);
            assert_eq!(spec.count().max, Some(1));
        }
    }
    assert_eq!(powers, [-3, -2, -1]);
}

#[test]
fn blue_dragon_applies_each_modifier_to_its_own_selected_creature() {
    let definition = parse_oracle_card_definition("Blue Dragon");
    let triggered = lightning_breath(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let first = game.create_object_from_definition(
        &test_creature("Required Opponent Target"),
        bob,
        Zone::Battlefield,
    );
    let second = game.create_object_from_definition(
        &test_creature("First Optional Target"),
        alice,
        Zone::Battlefield,
    );
    let third = game.create_object_from_definition(
        &test_creature("Second Optional Target"),
        bob,
        Zone::Battlefield,
    );
    let bystander = game.create_object_from_definition(
        &test_creature("Untargeted Bystander"),
        bob,
        Zone::Battlefield,
    );

    let assignments = triggered
        .choices
        .iter()
        .enumerate()
        .map(|(index, spec)| crate::game_state::TargetAssignment {
            spec: spec.clone(),
            range: index..index + 1,
        })
        .collect();
    let mut decisions = AutoPassDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(first),
            crate::effects::ResolvedTarget::Object(second),
            crate::effects::ResolvedTarget::Object(third),
        ])
        .with_target_assignments(assignments);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Lightning Breath should resolve");

    assert_eq!(game.calculated_power(first), Some(2));
    assert_eq!(game.calculated_power(second), Some(3));
    assert_eq!(game.calculated_power(third), Some(4));
    assert_eq!(game.calculated_power(bystander), Some(5));
}
