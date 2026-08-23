#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const LINE: &str = "Each player chooses a creature type. Destroy all creatures that aren't of a type chosen this way. They can't be regenerated.";

struct TypeChoices;

impl crate::decision::DecisionMaker for TypeChoices {
    fn decide_options(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let wanted = if ctx.player == PlayerId::from_index(0) {
            "Zombie"
        } else {
            "Goblin"
        };
        ctx.options
            .iter()
            .find(|option| option.description.eq_ignore_ascii_case(wanted))
            .map(|option| vec![option.index])
            .expect("requested creature type must be offered")
    }
}

fn typed_creature(name: &str, subtype: Subtype) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn harsh_mercy_keeps_multi_player_chosen_type_filter_and_exact_surface() {
    let definition = parse_oracle_card_definition("Harsh Mercy");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![LINE.to_string()]
    );
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("ChooseCreatureTypeEffect"), "{debug}");
    assert!(
        debug.contains("excluded_any_chosen_creature_type: true"),
        "the destroy filter must exclude the complete chosen set: {debug}"
    );
}

#[test]
fn harsh_mercy_preserves_every_players_choice_and_destroys_other_types() {
    let definition = parse_oracle_card_definition("Harsh Mercy");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let zombie = game.create_object_from_definition(
        &typed_creature("Chosen Zombie", Subtype::Zombie),
        alice,
        Zone::Battlefield,
    );
    let goblin = game.create_object_from_definition(
        &typed_creature("Chosen Goblin", Subtype::Goblin),
        bob,
        Zone::Battlefield,
    );
    let elf = game.create_object_from_definition(
        &typed_creature("Unchosen Elf", Subtype::Elf),
        alice,
        Zone::Battlefield,
    );
    let elf_stable = game.object(elf).expect("unchosen Elf exists").stable_id;

    let mut decisions = TypeChoices;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        definition
            .spell_effect
            .as_ref()
            .expect("Harsh Mercy has a spell effect"),
        None,
        &[],
    )
    .expect("Harsh Mercy should resolve");

    assert_eq!(
        game.object(zombie).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        game.object(goblin).map(|object| object.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        game.find_object_by_stable_id(elf_stable)
            .and_then(|object_id| game.object(object_id))
            .map(|object| object.zone),
        Some(Zone::Graveyard)
    );
}
