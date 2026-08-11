#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effects::ExecutionContext;
use ironsmith_core::ValueSurfaceHint;

const ORACLE: &str = "Vigilance\nWhen this creature enters, draw cards equal to the number of Zombies you control or the number of Zombie cards in your graveyard, whichever is greater.";

fn trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Prophet should have an enters trigger")
}

#[test]
fn prophet_of_the_scarab_keeps_the_greater_of_two_zombie_counts() {
    let definition = parse_oracle_card_definition("Prophet of the Scarab");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let draw = trigger(&definition)
        .effects
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DrawCardsEffect>())
        .expect("the trigger should draw a dynamic number of cards");
    assert!(
        draw.count
            .has_surface_hint(ValueSurfaceHint::WhicheverIsGreater),
        "{draw:#?}"
    );
    assert!(matches!(
        draw.count.unhinted(),
        Value::Add(total, negative_minimum)
            if matches!(total.as_ref(), Value::Add(_, _))
                && matches!(negative_minimum.as_ref(), Value::Scaled(minimum, -1) if matches!(minimum.as_ref(), Value::Min(_, _)))
    ));
}

#[test]
fn prophet_draws_whichever_zombie_set_is_larger() {
    let definition = parse_oracle_card_definition("Prophet of the Scarab");
    let program = &trigger(&definition).effects;
    let zombie = CardDefinitionBuilder::new(CardId::new(), "Zombie")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .build();
    let filler = CardDefinitionBuilder::new(CardId::new(), "Draw Card").build();

    // Prophet is itself a Zombie on the battlefield, so it contributes to
    // the battlefield count in both scenarios.
    for (battlefield_count, graveyard_count, expected_draws) in [(2, 4, 4), (5, 2, 6)] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for _ in 0..battlefield_count {
            game.create_object_from_definition(&zombie, alice, Zone::Battlefield);
        }
        for _ in 0..graveyard_count {
            game.create_object_from_definition(&zombie, alice, Zone::Graveyard);
        }
        for _ in 0..8 {
            game.create_object_from_definition(&filler, alice, Zone::Library);
        }
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
        .expect("Prophet's enters effect should resolve");

        assert_eq!(
            game.player(alice).expect("Alice exists").hand.len(),
            hand_before + expected_draws
        );
    }
}
