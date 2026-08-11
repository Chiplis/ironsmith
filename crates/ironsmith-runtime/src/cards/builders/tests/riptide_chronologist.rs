#![cfg(ironsmith_runtime_parser_tests)]

use super::*;
use crate::decision::DecisionMaker;

const ORACLE: &str =
    "{U}, Sacrifice this creature: Untap all creatures of the creature type of your choice.";

struct ChooseGoblin;

impl DecisionMaker for ChooseGoblin {
    fn decide_options(
        &mut self,
        _game: &crate::GameState,
        context: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        context
            .options
            .iter()
            .find(|option| option.description.eq_ignore_ascii_case("goblin"))
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
}

fn definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Riptide Chronologist")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .power_toughness(crate::card::PowerToughness::fixed(1, 3))
        .parse_text(ORACLE)
        .expect("inline chosen-type untap should parse")
}

fn creature(name: &str, subtype: Subtype) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn riptide_chronologist_selects_and_untaps_only_the_chosen_creature_type() {
    let definition = definition();
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("ChooseCreatureTypeEffect"), "{debug}");
    assert!(debug.contains("chosen_creature_type: true"), "{debug}");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
        panic!("expected activated ability: {:#?}", definition.abilities);
    };
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let goblin = game.create_object_from_definition(
        &creature("Goblin Probe", Subtype::Goblin),
        alice,
        Zone::Battlefield,
    );
    let elf = game.create_object_from_definition(
        &creature("Elf Probe", Subtype::Elf),
        alice,
        Zone::Battlefield,
    );
    game.tap(goblin);
    game.tap(elf);
    game.push_to_stack(crate::game_state::StackEntry::ability(
        source,
        alice,
        activated.effects.clone(),
    ));

    let mut decisions = ChooseGoblin;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("chosen-type untap should resolve");

    assert!(!game.is_tapped(goblin));
    assert!(game.is_tapped(elf));
    assert_eq!(game.chosen_creature_type(source), Some(Subtype::Goblin));
}
