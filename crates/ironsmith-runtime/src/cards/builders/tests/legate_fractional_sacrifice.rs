#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE_LINES: &[&str] = &[
    "Decimate — When Legate Lanius enters, each opponent sacrifices a tenth of the creatures they control of their choice, rounded up.",
    "Whenever an opponent sacrifices a creature, put a +1/+1 counter on Legate Lanius.",
];

fn victim_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Fraction Victim")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

fn decimate_program(definition: &CardDefinition) -> &crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            matches!(
                triggered.presentation_label.as_ref(),
                Some(crate::ability::PresentationLabel::AbilityWord(label))
                    if label.eq_ignore_ascii_case("Decimate")
            )
            .then_some(&triggered.effects)
        })
        .expect("Legate must retain its labeled entry trigger")
}

#[test]
fn legate_keeps_the_exact_unit_fraction_chosen_set_program() {
    let definition = parse_oracle_card_definition("Legate Lanius, Caesar's Ace");
    assert_eq!(
        canonical_compiled_lines(&definition),
        ORACLE_LINES
            .iter()
            .map(|line| (*line).to_string())
            .collect::<Vec<_>>()
    );

    let debug = format!("{:#?}", decimate_program(&definition));
    let compact_debug = debug.split_whitespace().collect::<String>();
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("filter: Opponent"), "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("chooser: IteratedPlayer"), "{debug}");
    assert!(
        compact_debug.contains("controller:Some(IteratedPlayer,)"),
        "{debug}"
    );
    assert!(debug.contains("DividedRoundedDown"), "{debug}");
    assert!(compact_debug.contains("Fixed(9,)"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
}

#[test]
fn a_tenth_rounded_up_sacrifices_at_zero_one_ten_and_eleven() {
    let definition = parse_oracle_card_definition("Legate Lanius, Caesar's Ace");
    let victim = victim_definition();

    for (creature_count, expected_sacrificed) in [(0, 0), (1, 1), (10, 1), (11, 2)] {
        let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for _ in 0..creature_count {
            game.create_object_from_definition(&victim, bob, Zone::Battlefield);
        }

        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            source,
            decimate_program(&definition),
            None,
            &[],
        )
        .expect("Decimate chosen-set program should resolve");

        let remaining = game
            .objects_in_zone(Zone::Battlefield)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|object| object.name == "Fraction Victim" && game.controller_of(object) == bob)
            .count();
        assert_eq!(
            creature_count - remaining,
            expected_sacrificed,
            "ceil({creature_count}/10) creatures must be sacrificed"
        );
    }
}
