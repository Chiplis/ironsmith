#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const KABOOM_ORACLE: &str = "Choose any number of target players or planeswalkers. For each of them, reveal cards from the top of your library until you reveal a nonland card, Kaboom! deals damage equal to that card's mana value to that player or planeswalker, then you put the revealed cards on the bottom of your library in any order.";

#[test]
fn kaboom_renders_the_complete_per_target_procedure() {
    let definition = parse_oracle_card_definition("Kaboom!");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        KABOOM_ORACLE
    );
}

#[test]
fn kaboom_uses_one_target_declaration_and_disjoint_typed_iterations() {
    let definition = parse_oracle_card_definition("Kaboom!");
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("Kaboom! must retain its spell-resolution program")
        .flattened_default_effects();
    let debug = format!("{effects:#?}");

    assert_eq!(
        debug.matches("TargetOnlyEffect").count(),
        1,
        "the player and planeswalker members must share one target declaration: {debug}"
    );
    assert!(
        debug.contains("TaggedEffect")
            && debug.contains("PlayerOrPlaneswalker")
            && debug.contains("ForPlayersEffect")
            && debug.contains("AliasedTarget")
            && debug.contains("ForEachTaggedEffect"),
        "expected one mixed declaration with disjoint player/object iterators: {debug}"
    );
    assert_eq!(
        debug.matches("ConsultTopOfLibraryEffect").count(),
        2,
        "each typed iterator must retain the complete consult body: {debug}"
    );
    assert_eq!(
        debug.matches("DealDamageEffect").count(),
        2,
        "each typed iterator must retain its damage action: {debug}"
    );
    assert_eq!(
        debug
            .matches("PutTaggedRemainderOnLibraryBottomEffect")
            .count(),
        2,
        "each typed iterator must retain its revealed-card cleanup: {debug}"
    );
    assert!(
        debug.contains("Player(\n")
            && debug.contains("IteratedPlayer")
            && debug.contains("target: Iterated"),
        "damage must bind to the current player or current planeswalker member: {debug}"
    );
}

#[test]
fn kaboom_resolves_the_complete_consult_for_a_planeswalker_member() {
    fn library_card(
        game: &mut crate::game_state::GameState,
        owner: PlayerId,
        raw_id: u32,
        name: &str,
        card_types: Vec<CardType>,
        mana_value: Option<u8>,
    ) -> ObjectId {
        let mut builder =
            crate::card::CardBuilder::new(CardId::from_raw(raw_id), name).card_types(card_types);
        if let Some(mana_value) = mana_value {
            builder = builder.mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                mana_value,
            )]]));
        }
        game.create_object_from_card(&builder.build(), owner, Zone::Library)
    }

    let definition = parse_oracle_card_definition("Kaboom!");
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("Kaboom! must retain its spell-resolution program")
        .flattened_default_effects()
        .to_vec();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let planeswalker = game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(96_100), "Bob's Planeswalker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(5)
            .build(),
        bob,
        Zone::Battlefield,
    );

    let existing_bottom = library_card(
        &mut game,
        alice,
        96_101,
        "Existing Bottom",
        vec![CardType::Land],
        None,
    );
    let planeswalker_match = library_card(
        &mut game,
        alice,
        96_102,
        "Planeswalker Match",
        vec![CardType::Artifact],
        Some(2),
    );
    let planeswalker_land = library_card(
        &mut game,
        alice,
        96_103,
        "Planeswalker Land",
        vec![CardType::Land],
        None,
    );
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(planeswalker)]);
    ctx.snapshot_targets(&game);
    for effect in &effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("each Kaboom! resolution effect should execute");
    }

    assert_eq!(
        game.player(bob).expect("Bob exists").life,
        20,
        "the planeswalker branch must not redirect its damage to that planeswalker's controller"
    );
    assert_eq!(
        game.counter_count(planeswalker, crate::CounterType::Loyalty),
        3,
        "the object iteration must use the first matching card's mana value"
    );

    let library = &game.player(alice).expect("Alice exists").library;
    assert_eq!(
        library.last(),
        Some(&existing_bottom),
        "both consulted packets must be put on the bottom, exposing the old bottom card"
    );
    for revealed in [planeswalker_match, planeswalker_land] {
        assert!(
            library.contains(&revealed),
            "every revealed card must remain in Alice's library"
        );
    }
}
