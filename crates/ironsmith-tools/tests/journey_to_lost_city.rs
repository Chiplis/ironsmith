use ironsmith::{
    CardBuilder, CardId, CardType, GameState, ManaCost, ManaSymbol, Phase, PlayerId,
    PowerToughness, Step, TriggerQueue, Zone, generate_and_queue_step_triggers,
    put_triggers_on_stack, resolve_stack_entry,
};
use ironsmith_compiler::{CardDefinitionBuilder as CompilerCardDefinitionBuilder, CardType as CompilerCardType};

const JOURNEY_TEXT: &str = "At the beginning of your upkeep, exile the top four cards of your library, then roll a d20.\n\
1—9 | You may put a land card from among those cards onto the battlefield.\n\
10—19 | Create a 2/2 green Wolf creature token, then put a +1/+1 counter on it for each creature card among those cards.\n\
20 | Put all permanent cards exiled with this enchantment onto the battlefield, then sacrifice it.";

fn journey_to_the_lost_city_definition() -> ironsmith::cards::CardDefinition {
    let builder = CompilerCardDefinitionBuilder::new(
        ironsmith_compiler::CardId::from_raw(91_681),
        "Journey to the Lost City",
    )
    .mana_cost(ironsmith_compiler::ManaCost::from_pips(vec![
        vec![ironsmith_compiler::ManaSymbol::Generic(3)],
        vec![ironsmith_compiler::ManaSymbol::Green],
    ]))
    .card_types(vec![CompilerCardType::Enchantment]);

    ironsmith_tools::compile_builder_to_runtime_definition(builder, JOURNEY_TEXT, false)
        .expect("Journey to the Lost City should compile strictly")
}

fn setup_journey_game() -> (GameState, PlayerId, ironsmith::ObjectId) {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let journey = journey_to_the_lost_city_definition();
    let journey_id = game.create_object_from_definition(&journey, alice, Zone::Battlefield);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Upkeep);
    (game, alice, journey_id)
}

fn make_card(id: u64, name: &str, card_types: Vec<CardType>) -> ironsmith::Card {
    let mut builder = CardBuilder::new(CardId::from_raw(id), name).card_types(card_types.clone());
    if card_types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

#[test]
fn journey_to_the_lost_city_strictly_compiles_and_renders_result_20_clause() {
    let definition = journey_to_the_lost_city_definition();
    let rendered = ironsmith::compiled_text::compiled_text_lines(&definition).join("\n");

    assert!(
        rendered.contains("If the result is 20"),
        "expected exact d20 result branch in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("return all permanent cards exiled with this enchantment to the battlefield")
            || rendered.contains("Return all permanent cards exiled with this enchantment to the battlefield"),
        "expected source-linked permanent return in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("Sacrifice this enchantment"),
        "expected source sacrifice in compiled text, got {rendered}"
    );
}

#[test]
fn journey_to_the_lost_city_roll_20_returns_linked_permanents_and_sacrifices_itself() {
    let (mut game, alice, journey_id) = setup_journey_game();
    let linked_creature = make_card(91_682, "Linked Grizzly Bears", vec![CardType::Creature]);
    let linked_land = make_card(91_683, "Linked Forest", vec![CardType::Land]);
    let linked_instant = make_card(91_684, "Linked Instant", vec![CardType::Instant]);
    let unlinked_land = make_card(91_685, "Unlinked Forest", vec![CardType::Land]);

    let linked_creature_id = game.create_object_from_card(&linked_creature, alice, Zone::Exile);
    let linked_land_id = game.create_object_from_card(&linked_land, alice, Zone::Exile);
    let linked_instant_id = game.create_object_from_card(&linked_instant, alice, Zone::Exile);
    let unlinked_land_id = game.create_object_from_card(&unlinked_land, alice, Zone::Exile);
    game.add_exiled_with_source_link(journey_id, linked_creature_id);
    game.add_exiled_with_source_link(journey_id, linked_land_id);
    game.add_exiled_with_source_link(journey_id, linked_instant_id);
    game.force_next_die_roll(20);

    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Journey upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Journey upkeep trigger should resolve");

    assert_eq!(game.object(linked_creature_id).map(|obj| obj.zone), Some(Zone::Battlefield));
    assert_eq!(game.object(linked_land_id).map(|obj| obj.zone), Some(Zone::Battlefield));
    assert_eq!(game.object(linked_instant_id).map(|obj| obj.zone), Some(Zone::Exile));
    assert_eq!(game.object(unlinked_land_id).map(|obj| obj.zone), Some(Zone::Exile));
    assert_eq!(game.object(journey_id).map(|obj| obj.zone), Some(Zone::Graveyard));
}

#[test]
fn journey_to_the_lost_city_non_20_roll_does_not_return_linked_permanents_or_sacrifice() {
    let (mut game, alice, journey_id) = setup_journey_game();
    let linked_land = make_card(91_686, "Linked Forest", vec![CardType::Land]);
    let linked_land_id = game.create_object_from_card(&linked_land, alice, Zone::Exile);
    game.add_exiled_with_source_link(journey_id, linked_land_id);
    game.force_next_die_roll(1);

    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Journey upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Journey upkeep trigger should resolve");

    assert_eq!(game.object(linked_land_id).map(|obj| obj.zone), Some(Zone::Exile));
    assert_eq!(game.object(journey_id).map(|obj| obj.zone), Some(Zone::Battlefield));
}

#[test]
fn journey_to_the_lost_city_exile_top_cards_are_linked_to_the_source() {
    let (mut game, alice, journey_id) = setup_journey_game();
    for idx in 0..4 {
        let card = make_card(91_690 + idx, "Exiled Library Creature", vec![CardType::Creature]);
        game.create_object_from_card(&card, alice, Zone::Library);
    }
    game.force_next_die_roll(20);

    let mut trigger_queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Journey upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Journey upkeep trigger should resolve");

    let returned_library_cards = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Exiled Library Creature")
        })
        .count();
    assert_eq!(returned_library_cards, 4);
    assert!(game.get_exiled_with_source_links(journey_id).is_empty());
}
