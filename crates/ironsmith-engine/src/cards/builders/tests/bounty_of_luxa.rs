#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effect::EffectPredicate;
use crate::effects::{ExecutionContext, IfEffect};

const ORACLE: &str = "At the beginning of your first main phase, remove all flood counters from this enchantment. If no counters were removed this way, put a flood counter on this enchantment and draw a card. Otherwise, add {C}{G}{U}.";

fn trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Bounty should have one beginning-of-main-phase trigger")
}

fn if_effect(effect: &crate::effect::Effect) -> Option<(&crate::effect::EffectId, &IfEffect)> {
    let with_id = effect.downcast_ref::<WithIdEffect>()?;
    let conditional = with_id.effect.downcast_ref::<IfEffect>()?;
    Some((&with_id.id, conditional))
}

#[test]
fn bounty_of_luxa_keeps_the_negated_result_and_inverse_otherwise_ids() {
    let definition = parse_oracle_card_definition("Bounty of the Luxa");
    assert_eq!(
        unprocessed_compiled_lines(&definition),
        vec![ORACLE.to_string()]
    );

    let program = &trigger(&definition).effects;
    let [remove_segment, empty_segment, otherwise_segment] = program.segments.as_slice() else {
        panic!("expected the exact three-stage result program: {program:#?}");
    };
    let [remove_root] = remove_segment.default_effects.as_slice() else {
        panic!("expected one counter-removal producer: {remove_segment:#?}");
    };
    let remove_id = remove_root
        .downcast_ref::<WithIdEffect>()
        .expect("counter removal result id")
        .id;
    let [empty_root] = empty_segment.default_effects.as_slice() else {
        panic!("expected one empty-result branch: {empty_segment:#?}");
    };
    let (empty_id, empty) = if_effect(empty_root).expect("ID-wrapped empty-result branch");
    assert_eq!(empty.condition, remove_id);
    assert_eq!(empty.predicate, EffectPredicate::DidNotHappen);
    assert!(empty.else_.is_empty());

    let [otherwise_root] = otherwise_segment.default_effects.as_slice() else {
        panic!("expected one otherwise branch: {otherwise_segment:#?}");
    };
    let otherwise = otherwise_root
        .downcast_ref::<IfEffect>()
        .expect("otherwise result branch");
    assert_eq!(otherwise.condition, *empty_id);
    assert_eq!(otherwise.predicate, EffectPredicate::DidNotHappen);
    assert!(otherwise.else_.is_empty());
}

#[test]
fn bounty_of_luxa_alternates_draw_and_mana_from_actual_counter_removal() {
    let definition = parse_oracle_card_definition("Bounty of the Luxa");
    let program = &trigger(&definition).effects;

    for starts_with_counter in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        if starts_with_counter {
            game.add_counters(source, crate::CounterType::Flood, 1)
                .expect("source exists");
        }
        let draw_card = CardDefinitionBuilder::new(CardId::new(), "Draw Card").build();
        game.create_object_from_definition(&draw_card, alice, Zone::Library);
        let hand_before = game.player(alice).expect("Alice exists").hand.len();

        let mut context = ExecutionContext::new_default(source, alice);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut context,
            alice,
            source,
            program,
            None,
            &[],
        )
        .expect("Bounty's result program should resolve");

        let player = game.player(alice).expect("Alice exists");
        if starts_with_counter {
            assert_eq!(game.counter_count(source, crate::CounterType::Flood), 0);
            assert_eq!(player.hand.len(), hand_before);
            assert_eq!(player.mana_pool.colorless, 1);
            assert_eq!(player.mana_pool.green, 1);
            assert_eq!(player.mana_pool.blue, 1);
        } else {
            assert_eq!(game.counter_count(source, crate::CounterType::Flood), 1);
            assert_eq!(player.hand.len(), hand_before + 1);
            assert_eq!(player.mana_pool.total(), 0);
        }
    }
}
