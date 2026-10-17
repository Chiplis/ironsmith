//! Claude package 51 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

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
