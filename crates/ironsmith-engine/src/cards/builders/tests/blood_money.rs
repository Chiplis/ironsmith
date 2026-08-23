#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn blood_money_preserves_the_nontoken_destroyed_result_filter() {
    let definition = parse_oracle_card_definition("Blood Money");
    let debug = format!("{definition:#?}");
    let compiled = unprocessed_compiled_lines(&definition);

    assert!(
        debug.contains("ForEachTaggedEffect")
            && debug.contains("TaggedObjectMatchedLastKnown")
            && debug.contains("nontoken: true"),
        "Blood Money must filter the actual destroyed result set by nontoken LKI: {debug}"
    );
    assert_eq!(
        compiled.len(),
        1,
        "Blood Money should remain one spell line"
    );
    assert_eq!(
        compiled[0],
        "Destroy all creatures. For each nontoken creature destroyed this way, you create a tapped Treasure token.",
        "Blood Money must render the typed nontoken result gate in its authored compact form"
    );
}

fn vanilla_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn blood_money_destroys_tokens_but_creates_treasures_only_for_destroyed_nontokens() {
    let definition = parse_oracle_card_definition("Blood Money");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Blood Money should have a spell resolution program");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);

    let alice_nontoken = game.create_object_from_definition(
        &vanilla_creature("Alice Nontoken"),
        alice,
        Zone::Battlefield,
    );
    let bob_nontoken = game.create_object_from_definition(
        &vanilla_creature("Bob Nontoken"),
        bob,
        Zone::Battlefield,
    );
    let token = game.create_object_from_definition(
        &vanilla_creature("Creature Token"),
        bob,
        Zone::Battlefield,
    );
    game.object_mut(token)
        .expect("test token should exist")
        .kind = crate::object::ObjectKind::Token;

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Blood Money should resolve");

    for destroyed in [alice_nontoken, bob_nontoken, token] {
        assert!(
            !game.battlefield.contains(&destroyed),
            "Blood Money must destroy every creature, including tokens"
        );
    }
    let treasures = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == "Treasure" && game.controller_of(object) == alice
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        treasures.len(),
        2,
        "only the two destroyed nontoken creatures should produce Treasures"
    );
    assert!(
        treasures.iter().all(|id| game.is_tapped(*id)),
        "Blood Money's Treasure tokens must enter tapped"
    );
}
