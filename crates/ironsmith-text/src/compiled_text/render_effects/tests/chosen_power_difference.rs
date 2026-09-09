use super::*;
use crate::card::PowerToughness;
use crate::ids::{CardId, PlayerId};
use crate::static_abilities::StaticAbilityId;
use crate::{CardDefinition, CardDefinitionBuilder};
const TEXT: &str = "Choose exactly two creatures you control. You draw X cards and the chosen creatures get +X/+X and gain trample until end of turn, where X is the difference between the chosen creatures' powers.";

#[test]
fn spry_and_mighty_resolves_power_gap_draw_pump_and_trample() {
    for (first_power, second_power) in [(2, 5), (5, 2), (2, 2), (-3, 2), (-5, -2)] {
        check_power_gap(first_power, second_power);
    }
}

fn check_power_gap(first_power: i32, second_power: i32) {
    let gap = (first_power - second_power).abs();
    fn test_creature(raw_id: u32, name: &str, power: i32, toughness: i32) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    }

    let definition = CardDefinitionBuilder::new(CardId::new(), "Spry and Mighty")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap();
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Spry and Mighty must retain its resolution program");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let small = game.create_object_from_definition(
        &test_creature(96_200, "Small Chosen Creature", first_power, 2),
        alice,
        Zone::Battlefield,
    );
    let large = game.create_object_from_definition(
        &test_creature(96_201, "Large Chosen Creature", second_power, 4),
        alice,
        Zone::Battlefield,
    );
    let opponent = game.create_object_from_definition(
        &test_creature(96_202, "Opponent Creature", 4, 4),
        bob,
        Zone::Battlefield,
    );
    let unchosen = game.create_object_from_definition(
        &test_creature(96_210, "Unchosen Friendly Creature", 9, 9),
        alice,
        Zone::Battlefield,
    );
    for _ in 0..gap {
        let card = CardDefinitionBuilder::new(CardId::new(), "Draw Card").build();
        game.create_object_from_definition(&card, alice, Zone::Library);
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut decisions);

    for effect in program {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("the complete chosen-pair program must resolve");
    }

    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        gap as usize,
        "draw must use the absolute power difference"
    );
    assert_eq!(
        (
            game.calculated_power(small),
            game.calculated_toughness(small)
        ),
        (Some(first_power + gap), Some(2 + gap))
    );
    assert_eq!(
        (
            game.calculated_power(large),
            game.calculated_toughness(large)
        ),
        (Some(second_power + gap), Some(4 + gap))
    );
    for chosen in [small, large] {
        assert!(
            game.current_has_static_ability_id(chosen, StaticAbilityId::Trample),
            "each chosen creature must gain trample"
        );
    }
    assert_eq!(
        (
            game.calculated_power(opponent),
            game.calculated_toughness(opponent)
        ),
        (Some(4), Some(4)),
        "an unchosen creature must not be pumped"
    );
    assert!(
        !game.current_has_static_ability_id(opponent, StaticAbilityId::Trample),
        "an unchosen creature must not gain trample"
    );
    assert_eq!(game.calculated_power(unchosen), Some(9));
    assert!(!game.current_has_static_ability_id(unchosen, StaticAbilityId::Trample));
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    assert_eq!(game.calculated_power(small), Some(first_power));
    assert_eq!(game.calculated_power(large), Some(second_power));
    for chosen in [small, large] {
        assert!(!game.current_has_static_ability_id(chosen, StaticAbilityId::Trample));
    }
}

#[test]
fn chosen_power_difference_text() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Spry and Mighty")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        TEXT
    );
}
