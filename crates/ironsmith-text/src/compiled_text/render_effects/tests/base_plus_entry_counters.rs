use super::*;
const TEXT: &str = "This creature enters with a +1/+1 counter on it plus an additional +1/+1 counter on it for each other creature you control.\nPlot {1}{W}";
#[test]
fn base_plus_entry_counters_counts_only_other_controlled_creatures() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sheriff of Safe Passage")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(0, 0))
        .parse_text(TEXT).unwrap();
    for count in [0, 1, 3] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Counted Creature")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
        for _ in 0..count { game.create_object_from_card(&creature, alice, Zone::Battlefield); }
        for (player, zone) in [(bob, Zone::Battlefield), (bob, Zone::Battlefield), (alice, Zone::Graveyard), (alice, Zone::Hand)] {
            game.create_object_from_card(&creature, player, zone);
        }
        let artifact = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Uncounted Artifact").card_types(vec![CardType::Artifact]).build();
        game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let entered = game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut dm).unwrap().new_id;
        assert_eq!(game.counter_count(entered, CounterType::PlusOnePlusOne), 1 + count);
        assert_eq!(game.calculated_power(entered), Some(1 + count as i32));
        assert_eq!(game.calculated_toughness(entered), Some(1 + count as i32));
        game.create_object_from_card(&creature, alice, Zone::Battlefield);
        assert_eq!(game.counter_count(entered, CounterType::PlusOnePlusOne), 1 + count);
    }
}
#[test]
fn base_plus_entry_counters_renders_the_structured_sum() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sheriff of Safe Passage")
        .card_types(vec![CardType::Creature]).parse_text(TEXT).unwrap();
    let lines = crate::compiled_text::compiled_text_lines(&definition);
    assert_eq!(lines[0], TEXT.lines().next().unwrap());
    assert_eq!(lines[1].trim_end_matches('.'), "Plot {1}{W}");
}

#[test]
fn base_plus_entry_counter_wording_supports_plural_quantities_and_other_counter_types() {
    let count = Value::Add(Box::new(Value::Fixed(2)), Box::new(
        Value::CountScaled(ObjectFilter::artifact().controlled_by(PlayerFilter::You), 3)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)));
    assert_eq!(super::super::clause_and_ability_surfaces::describe_as_enters_counter_phrase_on_it(&count, CounterType::Charge),
        "two charge counters on it plus three additional charge counters on it for each artifact you control");
    let where_x = count.with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs);
    assert!(super::super::clause_and_ability_surfaces::describe_as_enters_counter_phrase_on_it(&where_x, CounterType::Charge).contains("where X is"));
}
