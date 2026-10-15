//! Delirium reads every graveyard card's characteristics. Before the
//! continuous state is refreshed, each of those reads regenerated every
//! continuous effect, and each regeneration asked delirium again: a random
//! game with two Channelers and a small graveyard never finished loading.
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId::from_index(0);

fn card(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Two Channelers on the battlefield, one in the graveyard, and `fillers`
/// instants and sorceries beside it. Nothing has been refreshed yet.
fn unrefreshed_channeler_board(fillers: usize) -> (GameState, ObjectId) {
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
    (game, channeler_id)
}

/// Full characteristic recomputes for one read of a battlefield Channeler's
/// power before any refresh.
fn recomputes_for_unrefreshed_read(fillers: usize) -> u64 {
    let (game, channeler_id) = unrefreshed_channeler_board(fillers);
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

/// CR 613.8 dependency sorts for one read of a Channeler against an explicit
/// effect list, as state-based-action checks read through a derived view.
fn dependency_sorts_for_explicit_effect_read(fillers: usize) -> u64 {
    let (game, channeler_id) = unrefreshed_channeler_board(fillers);
    let effects = game.all_continuous_effects();
    let before = game.work_counters().dependency_sorts;
    let chars = game
        .calculated_characteristics_with_effects(channeler_id, &effects)
        .expect("the Channeler has characteristics");
    assert_eq!(chars.power, Some(1));
    game.work_counters().dependency_sorts - before
}

#[test]
fn graveyard_lookups_do_not_sort_layers_that_cannot_reach_them() {
    // Each graveyard card read inside the Channeler's delirium used to build
    // every layer's dependency baseline over the whole game, which read the
    // battlefield Channelers' delirium again. Effects that can only reach
    // their own source never affect a graveyard card, so those layers need
    // no ordering for it.
    let small = dependency_sorts_for_explicit_effect_read(2);
    let large = dependency_sorts_for_explicit_effect_read(6);
    // Before: 80 -> 292. With the reach check: 5 -> 5.
    assert!(
        large <= small * 3,
        "graveyard size multiplied the dependency sorts: {small} -> {large}"
    );
}
