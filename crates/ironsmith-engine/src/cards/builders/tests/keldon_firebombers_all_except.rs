#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str =
    "When this creature enters, each player sacrifices all lands they control except for three.";

fn entry_program(definition: &CardDefinition) -> &crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(&triggered.effects),
            _ => None,
        })
        .expect("Keldon Firebombers must retain its entry trigger")
}

fn land_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Firebombers Land")
        .card_types(vec![CardType::Land])
        .build()
}

#[test]
fn keldon_firebombers_keeps_the_exact_all_except_three_program() {
    let definition = parse_oracle_card_definition("Keldon Firebombers");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let debug = format!("{:#?}", entry_program(&definition));
    let compact = debug.split_whitespace().collect::<String>();
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("SacrificePlayerEffect"), "{debug}");
    assert!(debug.contains("Add"), "{debug}");
    assert!(compact.contains("Fixed(-3,)"), "{debug}");
    assert!(
        compact.contains("controller:Some(IteratedPlayer,)"),
        "{debug}"
    );
}

#[test]
fn all_except_three_keeps_small_land_sets_and_reduces_larger_sets_to_three() {
    let definition = parse_oracle_card_definition("Keldon Firebombers");
    let land = land_definition();

    for (alice_lands, bob_lands) in [(0usize, 2usize), (3, 4), (5, 7)] {
        let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for _ in 0..alice_lands {
            game.create_object_from_definition(&land, alice, Zone::Battlefield);
        }
        for _ in 0..bob_lands {
            game.create_object_from_definition(&land, bob, Zone::Battlefield);
        }

        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            source,
            entry_program(&definition),
            None,
            &[],
        )
        .expect("all-except chosen-set program should resolve");

        let controlled_lands = |player| {
            game.objects_in_zone(Zone::Battlefield)
                .into_iter()
                .filter_map(|id| game.object(id))
                .filter(|object| object.is_land() && game.controller_of(object) == player)
                .count()
        };
        assert_eq!(controlled_lands(alice), alice_lands.min(3));
        assert_eq!(controlled_lands(bob), bob_lands.min(3));
    }
}

#[test]
fn all_lands_without_the_exception_remains_a_true_sacrifice_all() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Sacrifice All Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Each player sacrifices all lands they control.")
        .expect("ordinary sacrifice-all should parse");
    let rendered = canonical_compiled_lines(&definition).join("\n");

    assert_eq!(rendered, "Each player sacrifices all lands they control.");
    assert!(!rendered.contains("except for"));
}
