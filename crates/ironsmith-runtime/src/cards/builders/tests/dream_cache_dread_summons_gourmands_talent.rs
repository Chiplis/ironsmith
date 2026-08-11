#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const DREAM_CACHE: &str = "Draw three cards, then put two cards from your hand both on top of your library or both on the bottom of your library.";
const DREAD_SUMMONS: &str = "Each player mills X cards. For each creature card put into a graveyard this way, you create a tapped 2/2 black Zombie creature token. (To mill a card, a player puts the top card of their library into their graveyard.)";
const GOURMANDS_TALENT: &str = "(Gain the next level as a sorcery to add its ability.)\nDuring your turn, artifacts you control are Foods in addition to their other types and have \"{2}, {T}, Sacrifice this artifact: You gain 3 life.\"\n{2}{G}: Level 2\nWhenever you gain life for the first time each turn, create a 3/3 green Raccoon creature token.\n{3}{G}: Level 3\nWhenever you gain life for the first time each turn, put a +1/+1 counter on each creature you control.";

fn simple_card(name: &str, card_types: Vec<CardType>) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(card_types.clone());
    if card_types.contains(&CardType::Creature) {
        builder.power_toughness(PowerToughness::fixed(2, 2)).build()
    } else {
        builder.build()
    }
}

#[test]
fn frozen_three_card_surfaces_and_typed_programs_are_preserved() {
    let dream = parse_oracle_card_definition("Dream Cache");
    assert_eq!(canonical_compiled_lines(&dream).join("\n"), DREAM_CACHE);
    let dream_debug = format!("{:#?}", dream.spell_effect);
    assert!(
        dream_debug.contains("ChooseModeEffect")
            && dream_debug.contains("zone: Library")
            && dream_debug.contains("to_top: true")
            && dream_debug.contains("to_top: false"),
        "Dream Cache must retain both executable destinations: {dream_debug}"
    );

    let dread = parse_oracle_card_definition("Dread Summons");
    assert_eq!(canonical_compiled_lines(&dread).join("\n"), DREAD_SUMMONS);
    let dread_debug = format!("{:#?}", dread.spell_effect);
    assert!(
        dread_debug.contains("ForEachTaggedEffect")
            && dread_debug.contains("TaggedObjectMatchedLastKnown")
            && dread_debug.contains("Creature"),
        "Dread Summons must gate each mill result by creature-card LKI: {dread_debug}"
    );

    let gourmand = parse_oracle_card_definition("Gourmand's Talent");
    assert_eq!(
        canonical_compiled_lines(&gourmand).join("\n"),
        GOURMANDS_TALENT
    );
    let gourmand_debug = format!("{:#?}", gourmand.abilities);
    assert_eq!(
        gourmand_debug.matches("DuringYourTurn").count(),
        2,
        "both halves of Gourmand's first line must share the turn condition: {gourmand_debug}"
    );
    assert!(gourmand_debug.contains("AddSubtypes"));
    assert!(gourmand_debug.contains("GrantObjectAbilityForFilter"));
}

#[test]
fn dread_summons_counts_only_creature_cards_across_each_players_mill_packet() {
    let definition = parse_oracle_card_definition("Dread Summons");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (owner, prefix) in [(alice, "Alice"), (bob, "Bob")] {
        game.create_object_from_definition(
            &simple_card(&format!("{prefix} Creature"), vec![CardType::Creature]),
            owner,
            Zone::Library,
        );
        game.create_object_from_definition(
            &simple_card(&format!("{prefix} Noncreature"), vec![CardType::Sorcery]),
            owner,
            Zone::Library,
        );
    }
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(spell, alice, &mut decisions).with_x(2);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        spell,
        definition
            .spell_effect
            .as_ref()
            .expect("Dread Summons program"),
        None,
        &[],
    )
    .expect("Dread Summons should resolve");

    let zombies = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.owner == alice
                    && object.kind == crate::object::ObjectKind::Token
                    && object.subtypes.contains(&Subtype::Zombie)
                    && game.is_tapped(*id)
            })
        })
        .count();
    assert_eq!(zombies, 2);
}

#[test]
fn gourmands_type_and_activation_apply_only_during_its_controllers_turn() {
    let definition = parse_oracle_card_definition("Gourmand's Talent");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let artifact = game.create_object_from_definition(
        &simple_card("Plain Artifact", vec![CardType::Artifact]),
        alice,
        Zone::Battlefield,
    );

    game.turn.active_player = alice;
    game.refresh_continuous_state();
    let during = game
        .calculated_characteristics(artifact)
        .expect("artifact characteristics during Alice's turn");
    assert!(during.subtypes.contains(&Subtype::Food));
    assert!(
        during
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
    );

    game.turn.active_player = bob;
    game.mark_continuous_state_dirty();
    game.refresh_continuous_state();
    let outside = game
        .calculated_characteristics(artifact)
        .expect("artifact characteristics during Bob's turn");
    assert!(
        !outside.subtypes.contains(&Subtype::Food),
        "Food subtype must expire outside Alice's turn: {:#?}",
        definition.abilities
    );
    assert!(
        !outside
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
    );
}
