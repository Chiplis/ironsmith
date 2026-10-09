//! "Only during their own turns" casting/activation restrictions (City of
//! Solitude) and the counted "no more than N spells each turn" cast limit
//! (Fires of Invention). CR 601.3, 602.5. Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

const CITY: &str = "Mana cost: {2}{G}{G}\nType: Enchantment\nPlayers can cast spells and activate abilities only during their own turns.";
const FIRES: &str = "Mana cost: {3}{R}\nType: Enchantment\nYou can cast spells only during your turn and you can cast no more than two spells each turn.\nYou may cast spells with mana value less than or equal to the number of lands you control without paying their mana costs.";

#[test]
fn city_of_solitude_lowers_the_casting_and_both_activation_halves() {
    for definition in support::definitions("City of Solitude", CITY) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("CastSpellsMatching"), "{debug}");
        assert!(debug.contains("ActivateAbilities(Excluding"), "every ability, mana included: {debug}");
        assert!(debug.contains("Excluding"), "only non-active players: {debug}");
        assert!(debug.contains("Active"), "{debug}");
    }
}

#[test]
fn fires_of_invention_lowers_own_turn_casting_and_a_two_spell_cap() {
    for definition in support::definitions("Fires of Invention", FIRES) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("CastMoreThanNSpellsEachTurn"), "{debug}");
        assert!(debug.contains("maximum: 2"), "{debug}");
        assert!(debug.contains("Excluding"), "{debug}");
    }
}

fn permanent(game: &mut GameState, name: &str, text: &str, controller: PlayerId) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}

fn refresh_with_active(game: &mut GameState, active: PlayerId) {
    game.turn.active_player = active;
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
}

#[test]
fn city_of_solitude_stops_the_non_active_player_only() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let city = permanent(&mut game, "City of Solitude", CITY, A);
    let land = permanent(&mut game, "Forest", "Type: Basic Land — Forest", B);
    refresh_with_active(&mut game, A);
    assert!(game.can_cast_spells(A));
    assert!(game.can_activate_non_mana_abilities(A));
    assert!(game.can_activate_abilities_of(city));
    assert!(!game.can_cast_spells(B), "B can't cast on A's turn");
    assert!(!game.can_activate_non_mana_abilities(B));
    assert!(!game.can_activate_abilities(B), "B's mana abilities too, from any zone");
    let _ = land;

    refresh_with_active(&mut game, B);
    assert!(game.can_cast_spells(B));
    assert!(game.can_activate_abilities(B));
    assert!(!game.can_cast_spells(A), "the restriction binds its controller too");
}

#[test]
fn fires_of_invention_restricts_only_its_controller_and_caps_spells_at_two() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    permanent(&mut game, "Fires of Invention", FIRES, A);
    refresh_with_active(&mut game, A);
    assert!(game.can_cast_spells(A));
    assert_eq!(
        game.effect_store.cant_effects.counted_cast_limits_for_player(A),
        Some(&[(ironsmith::target::ObjectFilter::default(), 2)][..])
    );
    assert!(game.effect_store.cant_effects.counted_cast_limits_for_player(B).is_none());

    refresh_with_active(&mut game, B);
    assert!(!game.can_cast_spells(A), "A can't cast during B's turn");
    assert!(game.can_cast_spells(B));
}

#[test]
fn city_of_solitude_follows_the_active_player_across_a_real_turn_change() {
    // The cant tracker is rebuilt at every turn start (GameState::next_turn
    // ends with update_cant_effects), so the own-turn rules flip without a
    // manual refresh.
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    permanent(&mut game, "City of Solitude", CITY, A);
    let guide = compile_to_runtime_definition(
        "Spirit Guide Probe",
        "Type: Creature — Elf Spirit\nPower/Toughness: 2/2\nExile this card from your hand: Add {G}.",
        false,
    )
    .unwrap();
    game.create_object_from_definition(&guide, B, Zone::Hand);
    refresh_with_active(&mut game, A);
    assert!(!game.can_activate_abilities(B), "no hand mana ability off-turn");
    let starting_active = game.turn.active_player;
    game.next_turn();
    assert_ne!(game.turn.active_player, starting_active);
    let active = game.turn.active_player;
    let other = if active == A { B } else { A };
    assert!(game.can_activate_abilities(active));
    assert!(game.can_cast_spells(active));
    assert!(!game.can_activate_abilities(other));
    assert!(!game.can_cast_spells(other));
}
