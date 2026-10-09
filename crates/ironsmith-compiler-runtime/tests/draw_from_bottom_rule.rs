//! "You draw cards from the bottom of your library rather than the top."
//! (River Song, "Meet in Reverse"): a lasting rule on which card the
//! controller's draws take (CR 121.1). Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const RULE: &str = "Mana cost: {3}{U}{R}\nType: Legendary Creature — Time Lord\nPower/Toughness: 2/2\nYou draw cards from the bottom of your library rather than the top.";

#[test]
fn the_rule_lowers_to_a_controller_draw_from_bottom_restriction() {
    for definition in support::definitions("Draw Reverser", RULE) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("DrawFromBottom(You)"), "{debug}");
    }
}

fn library_card(game: &mut GameState, owner: PlayerId, name: &str) {
    let definition =
        compile_to_runtime_definition(name, "Type: Instant\nDraw a card.", false).unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Library);
}

#[test]
fn its_controller_draws_the_bottom_card_and_other_players_still_draw_the_top() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    for (owner, bottom, top) in [(A, "A bottom", "A top"), (B, "B bottom", "B top")] {
        library_card(&mut game, owner, bottom);
        library_card(&mut game, owner, top);
    }
    let reverser = compile_to_runtime_definition("Draw Reverser", RULE, false).unwrap();
    game.create_object_from_definition(&reverser, A, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();

    let drawn = game.draw_cards(A, 1);
    assert_eq!(game.object(drawn[0]).unwrap().name, "A bottom");
    let drawn = game.draw_cards(B, 1);
    assert_eq!(game.object(drawn[0]).unwrap().name, "B top");
}
