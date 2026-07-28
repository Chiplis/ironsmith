#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn compiled_lines(name: &str) -> Vec<String> {
    unprocessed_compiled_lines(&parse_oracle_card_definition(name))
}

#[test]
fn attack_and_block_triggers_keep_authored_group_cardinality() {
    for (name, expected_prefix) in [
        (
            "Flummoxed Cyclops",
            "Whenever two or more creatures your opponents control attack,",
        ),
        (
            "Inniaz, the Gale Force",
            "Whenever three or more creatures you control with flying attack,",
        ),
        (
            "Lairwatch Giant",
            "Whenever this creature blocks two or more creatures,",
        ),
    ] {
        let lines = compiled_lines(name);
        assert!(
            lines.iter().any(|line| line.starts_with(expected_prefix)),
            "{name} should retain {expected_prefix:?}; got {lines:#?}"
        );
    }
}

#[test]
fn spell_history_and_source_counter_activation_thresholds_survive_lowering() {
    let loan_shark = compiled_lines("Loan Shark");
    assert!(
        loan_shark.iter().any(|line| {
            line
                == "When this creature enters, if you've cast two or more spells this turn, draw a card."
        }),
        "Loan Shark should retain its two-spell gate; got {loan_shark:#?}"
    );

    let pyramid = compiled_lines("Pyramid of the Pantheon");
    assert!(
        pyramid.iter().any(|line| {
            line.contains("Add three mana of any one color.")
                && line.contains(
                    "Activate only if there are three or more brick counters on this artifact.",
                )
        }),
        "Pyramid should retain its brick-counter activation restriction; got {pyramid:#?}"
    );
}

#[test]
fn pyramid_brick_threshold_controls_mana_ability_legality() {
    use crate::special_actions::{SpecialAction, can_perform_check};

    let definition = parse_oracle_card_definition("Pyramid of the Pantheon");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let action = SpecialAction::ActivateManaAbility {
        permanent_id: source,
        ability_index: 1,
    };

    assert!(
        can_perform_check(&action, &game, alice).is_err(),
        "the three-mana ability must be unavailable with no brick counters"
    );
    game.object_mut(source)
        .expect("Pyramid should exist")
        .counters
        .insert(crate::CounterType::Named("brick"), 2);
    assert!(
        can_perform_check(&action, &game, alice).is_err(),
        "two brick counters are below the authored threshold"
    );
    game.object_mut(source)
        .expect("Pyramid should exist")
        .counters
        .insert(crate::CounterType::Named("brick"), 3);
    assert!(
        can_perform_check(&action, &game, alice).is_ok(),
        "the ability should become legal at three brick counters"
    );
}

#[test]
fn transform_followups_keep_their_threshold_and_sequence() {
    let dowsing = compiled_lines("Dowsing Device // Geode Grotto");
    assert!(
        dowsing.iter().any(|line| {
            line.contains("Then transform this artifact if you control four or more artifacts.")
        }),
        "Dowsing Device should retain its conditional transform; got {dowsing:#?}"
    );

    let foreboding = compiled_lines("Foreboding Statue // Forsaken Thresher");
    assert!(
        foreboding.iter().any(|line| {
            line.contains("if there are three or more omen counters on this creature,")
                && line.contains("untap")
                && line.contains("then transform")
        }),
        "Foreboding Statue should retain the complete untap-transform sequence; got {foreboding:#?}"
    );
}

#[test]
fn player_count_values_keep_the_hand_threshold_inside_the_player_set() {
    let definition = parse_oracle_card_definition("Wolfcaller's Howl");
    let lines = unprocessed_compiled_lines(&definition);
    assert!(
        lines.iter().any(|line| {
            line
                == "At the beginning of your upkeep, create X 2/2 green Wolf creature tokens, where X is the number of your opponents with four or more cards in hand."
        }),
        "Wolfcaller's Howl should count qualified opponents, not cards; got {lines:#?}"
    );

    let create = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .flat_map(|triggered| &triggered.effects.segments)
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
        .expect("Wolfcaller's Howl should lower its trigger to token creation");
    assert!(
        matches!(
            create.count.unhinted(),
            crate::effect::Value::CountPlayersWithCardsInHandAtLeast(PlayerFilter::Opponent, 4)
        ),
        "the token count must retain both participant and hand-size filters: {create:#?}"
    );
}

#[test]
fn wolfcallers_howl_runtime_counts_qualified_opponents_instead_of_hand_cards() {
    let definition = parse_oracle_card_definition("Wolfcaller's Howl");
    let create_effect = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .flat_map(|triggered| &triggered.effects.segments)
        .flat_map(|segment| &segment.default_effects)
        .find(|effect| effect.downcast_ref::<CreateTokenEffect>().is_some())
        .expect("Wolfcaller's Howl should lower its trigger to token creation");

    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let dana = PlayerId::from_index(3);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Hand Card")
        .card_types(vec![CardType::Instant])
        .build();

    for (player, count) in [(bob, 4), (charlie, 3), (dana, 5)] {
        for _ in 0..count {
            game.create_object_from_definition(&filler, player, Zone::Hand);
        }
    }

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::effects::execute_effect(&mut game, create_effect, &mut ctx)
        .expect("Wolfcaller's Howl token creation should resolve");

    let wolves = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| {
            object.kind == crate::object::ObjectKind::Token
                && object.name == "Wolf"
                && game.controller_of(object) == alice
        })
        .count();
    assert_eq!(
        wolves, 2,
        "only Bob and Dana meet the four-card threshold; the token count must be two"
    );
}
