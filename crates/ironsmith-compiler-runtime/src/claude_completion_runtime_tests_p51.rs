//! Claude package 51 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::{PlayerId, Zone};

fn p51_activated_abilities(
    definition: &ironsmith::cards::CardDefinition,
) -> Vec<&ironsmith::ability::ActivatedAbility> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .collect()
}

const CRYPTIC_SPIRES_TEXT: &str = "Type: Land\nAs you create your deck, circle two of the colors below.\nThis land enters tapped.\n{T}: Add one mana of either of the circled colors.";

/// "circle two of the colors below" is deck-construction rule text; the
/// colors are a per-card (stable identity) deck-construction note that
/// follows the card between zones, with a deterministic WUBRG default, and
/// the mana ability offers exactly those colors.
#[test]
fn p51_cryptic_spires_adds_only_the_colors_circled_for_that_card() {
    use ironsmith::color::{Color, ColorSet};
    let spires = compile_to_runtime_definition("Cryptic Spires", CRYPTIC_SPIRES_TEXT, false)
        .expect("Cryptic Spires should compile");
    assert_eq!(
        ironsmith::game_state::deck_construction_color_choice_count(&spires.abilities),
        Some(2),
        "{:?}",
        spires.abilities
    );
    let mana = p51_activated_abilities(&spires);
    assert_eq!(mana.len(), 1);
    let add = mana[0]
        .effects
        .all_effects()
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<ironsmith::effects::AddManaOfAnyColorEffect>()
                .cloned()
        })
        .expect("circled-color mana production");
    assert!(add.available_colors_chosen_at_deck_construction, "{add:?}");

    let alice = PlayerId::from_index(0);
    let run = |game: &mut ironsmith::GameState, source| {
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in mana[0].effects.all_effects() {
            ironsmith::effects::execute_effect(game, effect, &mut ctx).unwrap();
        }
        let pool = &game.player(alice).unwrap().mana_pool;
        (pool.white, pool.blue, pool.black, pool.red, pool.green)
    };

    // No note recorded: the deterministic default is the first two WUBRG
    // colors, and the first-choice decision maker takes white.
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&spires, alice, Zone::Battlefield);
    assert_eq!(
        game.deck_construction_chosen_colors(source),
        ColorSet::from_color(Color::White).union(ColorSet::from_color(Color::Blue))
    );
    assert_eq!(run(&mut game, source), (1, 0, 0, 0, 0));

    // A note made with the deck (while the card is in the library) must name
    // exactly two different colors, and survives the move to the battlefield.
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let in_library = game.create_object_from_definition(&spires, alice, Zone::Library);
    for bad in ["red", "red, red", "red, green, blue", "red, purple"] {
        assert!(game.record_deck_construction_note(in_library, bad).is_err(), "{bad}");
    }
    game.record_deck_construction_note(in_library, "Green and black").unwrap();
    let circled = ColorSet::from_color(Color::Black).union(ColorSet::from_color(Color::Green));
    let source = game
        .move_object(in_library, Zone::Battlefield, ironsmith::events::EventCause::effect())
        .expect("land moves to the battlefield");
    assert_ne!(source, in_library);
    assert_eq!(game.deck_construction_chosen_colors(source), circled);
    assert_eq!(run(&mut game, source), (0, 0, 1, 0, 0));

    // A second copy has its own (default) circles.
    let other = game.create_object_from_definition(&spires, alice, Zone::Battlefield);
    assert_eq!(
        game.deck_construction_chosen_colors(other),
        ColorSet::from_color(Color::White).union(ColorSet::from_color(Color::Blue))
    );
}

const CYCLOPEAN_TOMB_TEXT: &str = "Mana cost: {4}\nType: Artifact\n{2}, {T}: Put a mire counter on target non-Swamp land. That land is a Swamp for as long as it has a mire counter on it. Activate only during your upkeep.\nWhen this artifact is put into a graveyard from the battlefield, at the beginning of each of your upkeeps for the rest of the game, remove all mire counters from a land that a mire counter was put onto with this artifact but that a mire counter has not been removed from with this artifact.";

/// The tomb's counters are a lasting per-source record that survives the
/// tomb leaving play, and "at the beginning of each of your upkeeps for the
/// rest of the game" is a delayed trigger that never expires: each upkeep it
/// clears one land the tomb mired and has not cleared yet.
#[test]
fn p51_cyclopean_tomb_clears_one_land_it_mired_each_upkeep_forever() {
    use ironsmith::object::CounterType;
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let tomb = compile_to_runtime_definition("Cyclopean Tomb", CYCLOPEAN_TOMB_TEXT, false)
        .expect("Cyclopean Tomb should compile");
    let debug = format!("{:?}", tomb.abilities);
    assert!(
        debug.contains(
            "counter_source_history: Some(CounterSourceHistoryConstraint { counter_type: Mire, put_by_source: true, not_removed_by_source: true"
        ),
        "{debug}"
    );
    assert!(debug.contains("one_shot: false"), "{debug}");
    assert!(debug.contains("BeginningOfUpkeep"), "{debug}");

    let activation = p51_activated_abilities(&tomb)
        .into_iter()
        .next()
        .expect("mire activation")
        .clone();
    let leaves = tomb
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("graveyard trigger");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let land = CardBuilder::new(CardId::new(), "Mired Plains")
        .card_types(vec![CardType::Land])
        .build();
    let first = game.create_object_from_card(&land, alice, Zone::Battlefield);
    let second = game.create_object_from_card(&land, alice, Zone::Battlefield);
    let untouched = game.create_object_from_card(&land, alice, Zone::Battlefield);
    game.add_counters(untouched, CounterType::Mire, 1);
    let source = game.create_object_from_definition(&tomb, alice, Zone::Battlefield);
    let tomb_identity = game.object(source).unwrap().stable_id;
    for land in [first, second] {
        game.push_to_stack(
            ironsmith::game_state::StackEntry::ability(source, alice, activation.effects.clone())
                .with_targets(vec![ironsmith::game_state::Target::Object(land)]),
        );
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.counter_count(land, CounterType::Mire), 1);
        assert!(game.source_put_counter_on_object(tomb_identity, land, CounterType::Mire));
    }
    assert!(!game.source_put_counter_on_object(tomb_identity, untouched, CounterType::Mire));

    // The tomb dies; its leaves trigger registers the repeating delayed trigger
    // from the graveyard card, which keeps the same stable identity.
    let in_graveyard = game
        .move_object(source, Zone::Graveyard, ironsmith::events::EventCause::effect())
        .expect("tomb goes to the graveyard");
    assert_eq!(game.object(in_graveyard).unwrap().stable_id, tomb_identity);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    }
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);

    let upkeep = |game: &mut ironsmith::GameState| {
        let event = ironsmith::triggers::TriggerEvent::new(
            ironsmith::events::BeginningOfUpkeepEvent::new(alice),
            Default::default(),
        );
        let entries = ironsmith::triggers::check_delayed_triggers(game, &event);
        assert_eq!(entries.len(), 1, "the delayed trigger fires every upkeep");
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for entry in entries {
            queue.add(entry);
        }
        ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry(game).unwrap();
        }
        [first, second].map(|land| game.counter_count(land, CounterType::Mire))
    };
    let after_first = upkeep(&mut game);
    assert_eq!(after_first.iter().sum::<u32>(), 1, "one mired land is cleared");
    let after_second = upkeep(&mut game);
    assert_eq!(after_second, [0, 0], "the other mired land is cleared next");
    for land in [first, second] {
        assert!(game.source_removed_counter_from_object(tomb_identity, land, CounterType::Mire));
    }
    // A mire counter the tomb did not put there is never touched, and the
    // trigger keeps repeating with nothing left to clear.
    assert_eq!(upkeep(&mut game), [0, 0]);
    assert_eq!(game.counter_count(untouched, CounterType::Mire), 1);
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
}
