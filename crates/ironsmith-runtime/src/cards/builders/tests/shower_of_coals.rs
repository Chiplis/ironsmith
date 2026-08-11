#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::card::PowerToughness;
use crate::effects::{DealDamageEffect, ExecutionContext, ResolvedTarget};
use crate::game_state::TargetAssignment;

const ORACLE: &str = "Shower of Coals deals 2 damage to each of up to three targets.\nThreshold — Shower of Coals deals 4 damage to each of those permanents and/or players instead if there are seven or more cards in your graveyard.";

fn damage_view(effect: &crate::effect::Effect) -> Option<&DealDamageEffect> {
    if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
        return damage_view(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        return damage_view(&tagged.effect);
    }
    effect.downcast_ref::<DealDamageEffect>()
}

fn sturdy_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(8, 8))
        .build()
}

#[test]
fn shower_of_coals_replacement_reuses_the_exact_announced_target_set() {
    let definition = parse_oracle_card_definition("Shower of Coals");
    assert_eq!(unprocessed_compiled_lines(&definition).join("\n"), ORACLE);

    let program = definition.spell_effect.as_ref().expect("spell program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one replacement segment: {program:#?}");
    };
    let [default_root] = segment.default_effects.as_slice() else {
        panic!("expected one default damage root: {segment:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one threshold replacement: {segment:#?}");
    };
    let [replacement_root] = branch.replacement_effects.as_slice() else {
        panic!("expected one replacement damage root: {branch:#?}");
    };
    let default_with_id = default_root
        .downcast_ref::<WithIdEffect>()
        .expect("default effect id wrapper");
    let replacement_with_id = replacement_root
        .downcast_ref::<WithIdEffect>()
        .expect("replacement effect id wrapper");
    let default_tagged = default_with_id
        .effect
        .downcast_ref::<TaggedEffect>()
        .expect("default damaged-set tag");
    let replacement_tagged = replacement_with_id
        .effect
        .downcast_ref::<TaggedEffect>()
        .expect("replacement damaged-set tag");
    let default_damage = default_tagged
        .effect
        .downcast_ref::<DealDamageEffect>()
        .expect("default typed damage");
    let replacement_damage = replacement_tagged
        .effect
        .downcast_ref::<DealDamageEffect>()
        .expect("replacement typed damage");

    assert_eq!(replacement_with_id.id, default_with_id.id);
    assert_eq!(replacement_tagged.tag, default_tagged.tag);
    assert_eq!(default_damage.amount, crate::effect::Value::Fixed(2));
    assert_eq!(replacement_damage.amount, crate::effect::Value::Fixed(4));
    assert_eq!(replacement_damage.target, default_damage.target);
    assert!(matches!(
        default_damage.target.unhinted(),
        ChooseSpec::WithCount(inner, count)
            if matches!(inner.unhinted(), ChooseSpec::AnyTarget)
                && *count == crate::effect::ChoiceCount::up_to(3)
    ));
}

#[test]
fn shower_of_coals_damages_objects_and_players_at_both_threshold_states() {
    let definition = parse_oracle_card_definition("Shower of Coals");
    let program = definition.spell_effect.as_ref().expect("spell program");
    let default_damage =
        damage_view(&program.segments[0].default_effects[0]).expect("default typed damage");

    for (graveyard_cards, expected_damage) in [(0, 2), (7, 4)] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let first = game.create_object_from_definition(
            &sturdy_creature("First Target"),
            bob,
            Zone::Battlefield,
        );
        let second = game.create_object_from_definition(
            &sturdy_creature("Second Target"),
            bob,
            Zone::Battlefield,
        );
        for index in 0..graveyard_cards {
            let filler =
                CardDefinitionBuilder::new(CardId::new(), format!("Threshold Filler {index}"))
                    .build();
            game.create_object_from_definition(&filler, alice, Zone::Graveyard);
        }

        let assignment = TargetAssignment {
            spec: default_damage.target.clone(),
            range: 0..3,
        };
        let mut context = ExecutionContext::new_default(source, alice)
            .with_targets(vec![
                ResolvedTarget::Object(first),
                ResolvedTarget::Object(second),
                ResolvedTarget::Player(bob),
            ])
            .with_target_assignments(vec![assignment.clone()]);
        context.snapshot_targets(&game);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut context,
            alice,
            source,
            program,
            None,
            &[assignment],
        )
        .expect("the shared target-set damage should resolve");

        assert_eq!(game.damage_on(first), expected_damage);
        assert_eq!(game.damage_on(second), expected_damage);
        assert_eq!(
            game.player(bob).expect("Bob exists").life,
            20 - expected_damage as i32,
            "the player member of the announced target set must receive the same branch amount"
        );
    }
}
