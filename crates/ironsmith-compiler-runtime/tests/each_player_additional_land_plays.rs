//! "Each player may play an additional land on/during each of their turns."
//! (CR 305.2): the land-play allowance raised for every player.
//! Source-authored, deliberately unrun.
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{GameState, PlayerId, Zone};

#[path = "p02_line_families/compile.rs"]
mod compile;

const CARDS: &[(&str, &str)] = &[
    (
        "Ghirapur Orrery",
        "Mana cost: {4}\nType: Artifact\nEach player may play an additional land on each of their turns.\nAt the beginning of each player's upkeep, if that player has no cards in hand, that player draws three cards.",
    ),
    (
        "Rites of Flourishing",
        "Mana cost: {2}{G}\nType: Enchantment\nAt the beginning of each player's draw step, that player draws an additional card.\nEach player may play an additional land on each of their turns.",
    ),
    (
        "Storm Cauldron",
        "Mana cost: {5}\nType: Artifact\nEach player may play an additional land during each of their turns.\nWhenever a land is tapped for mana, return it to its owner's hand.",
    ),
];

#[test]
fn each_player_land_allowance_compiles_on_both_routes() {
    for &(name, text) in CARDS {
        for definition in compile::compile_both(name, text) {
            let restrictions = compile::statics(&definition, StaticAbilityId::RuleRestriction);
            assert_eq!(restrictions.len(), 1, "{name}: one land-play restriction");
            assert!(
                restrictions[0].display().to_ascii_lowercase().contains("each player may play an additional land"),
                "{name}: {}",
                restrictions[0].display()
            );
        }
    }
}

#[test]
fn each_player_land_allowance_raises_every_players_land_plays() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for &(name, text) in CARDS {
        for definition in compile::compile_both(name, text) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.refresh_continuous_state().unwrap();
            let base_alice = game.player(alice).unwrap().land_plays_per_turn;
            let base_bob = game.player(bob).unwrap().land_plays_per_turn;
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.refresh_continuous_state().unwrap();
            assert_eq!(game.player(alice).unwrap().land_plays_per_turn, base_alice + 1, "{name}");
            assert_eq!(
                game.player(bob).unwrap().land_plays_per_turn,
                base_bob + 1,
                "{name}: the opponent gets the extra land play too"
            );
        }
    }
}
