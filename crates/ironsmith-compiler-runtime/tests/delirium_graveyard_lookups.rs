//! Delirium reads every graveyard card's characteristics. Before the
//! continuous state is refreshed, each of those reads regenerated every
//! continuous effect, and each regeneration asked delirium again: a random
//! game with two Channelers and a small graveyard never finished loading.
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId::from_index(0);

fn card(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Full characteristic recomputes for one read of a battlefield Channeler's
/// power before any refresh, with a second Channeler out, one in the
/// graveyard, and `fillers` instants and sorceries beside it.
fn recomputes_for_unrefreshed_read(fillers: usize) -> u64 {
    let channeler = card(
        "Dragon's Rage Channeler",
        "Mana cost: {R}\nType: Creature — Human Shaman\nPower/Toughness: 1/1\n\
         Whenever you cast a noncreature spell, surveil 1.\n\
         Delirium — As long as there are four or more card types among cards in your graveyard, \
         Dragon's Rage Channeler gets +2/+2, has flying, and attacks each combat if able.",
    );
    let fillers_by_type = [
        card("Preordain", "Mana cost: {U}\nType: Sorcery\nScry 2, then draw a card."),
        card(
            "Lightning Bolt",
            "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target.",
        ),
    ];
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let channeler_id = game.create_object_from_definition(&channeler, A, Zone::Battlefield);
    game.create_object_from_definition(&channeler, A, Zone::Battlefield);
    game.create_object_from_definition(&channeler, A, Zone::Graveyard);
    for index in 0..fillers {
        game.create_object_from_definition(&fillers_by_type[index % 2], A, Zone::Graveyard);
    }
    assert!(!game.continuous_state_is_clean_public());
    let before = game.work_counters().characteristics_full_recomputes;
    // Creature, sorcery and instant: three card types, so no delirium.
    assert_eq!(game.calculated_power(channeler_id), Some(1));
    game.work_counters().characteristics_full_recomputes - before
}

#[test]
fn unrefreshed_delirium_reads_stay_bounded_as_the_graveyard_grows() {
    let small = recomputes_for_unrefreshed_read(2);
    let large = recomputes_for_unrefreshed_read(6);
    // Before the layer-context memo: 1576 -> 12712. With it: 76 -> 288.
    assert!(
        large <= small * 5 && large <= 600,
        "unrefreshed delirium reads blew up with the graveyard: {small} -> {large}"
    );
}

