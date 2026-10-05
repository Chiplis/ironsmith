//! Authored source-only scenarios; execution is deferred by the campaign.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::{Color, ColorSet};
use ironsmith::object::CounterType;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/battlefield_population_conditions.json.fixture")).unwrap()
}
fn fixture_definitions(name: &str) -> [CardDefinition; 2] {
    let cards = fixtures();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", card["mana_cost"].as_str().unwrap(), card["type_line"].as_str().unwrap(), card["power"].as_str().unwrap(), card["toughness"].as_str().unwrap(), card["oracle_text"].as_str().unwrap());
    definitions(name, &text)
}
fn population(game: &mut GameState, player: PlayerId, colors: ColorSet, creature: bool, zone: Zone) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Population member")
        .card_types(vec![if creature { CardType::Creature } else { CardType::Enchantment }])
        .color_indicator(colors).power_toughness(PowerToughness::fixed(2, 2)).build();
    game.create_object_from_card(&card, player, zone)
}
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20) }
fn a() -> PlayerId { PlayerId::from_index(0) }
fn b() -> PlayerId { PlayerId::from_index(1) }
fn c() -> PlayerId { PlayerId::from_index(2) }

#[test]
fn all_eleven_complete_frozen_cards_are_strict_and_round_trip() {
    for card in fixtures() {
        let name = card["name"].as_str().unwrap();
        for definition in fixture_definitions(name) { assert_eq!(definition.card.name, name); }
    }
}

#[test]
fn common_color_includes_ties_multicolor_foreign_permanents_and_source_itself() {
    for (name, color, other) in [
        ("Goham Djinn", Color::Black, Color::Red),
        ("Halam Djinn", Color::Red, Color::Blue),
        ("Ruham Djinn", Color::White, Color::Black),
        ("Sulam Djinn", Color::Green, Color::White),
        ("Zanam Djinn", Color::Blue, Color::Green),
    ] {
        let cards = fixtures();
        let card = cards.iter().find(|card| card["name"] == name).unwrap();
        let printed: i32 = card["power"].as_str().unwrap().parse().unwrap();
        for definition in fixture_definitions(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(printed - 2), "{name}: source counts");
            population(&mut game, b(), ColorSet::from(other), false, Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(printed - 2), "{name}: tie");
            let extra = population(&mut game, c(), ColorSet::from(other), false, Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(printed), "{name}: other leads");
            population(&mut game, a(), ColorSet::from(color).with(other), false, Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(printed), "{name}: multicolor adds to both");
            population(&mut game, b(), ColorSet::from(color), false, Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(printed - 2), "{name}: regained tie");
            population(&mut game, b(), ColorSet::from(other), false, Zone::Graveyard);
            population(&mut game, b(), ColorSet::COLORLESS, false, Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(printed - 2), "{name}: scope");
            game.phase_out(extra);
            assert_eq!(game.calculated_power(host), Some(printed - 2));
            game.phase_in(extra);
            assert_eq!(game.calculated_power(host), Some(printed - 2));
        }
    }
}

#[test]
fn any_player_presence_tracks_zones_and_phase_out_without_controller_narrowing() {
    for (name, color) in [("Knight of Grace", Color::Black), ("Knight of Malice", Color::White)] {
        for definition in fixture_definitions(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(2));
            population(&mut game, a(), ColorSet::from(color), false, Zone::Graveyard);
            assert_eq!(game.calculated_power(host), Some(2));
            let other = population(&mut game, c(), ColorSet::from(color), false, Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(3));
            game.phase_out(other);
            assert_eq!(game.calculated_power(host), Some(2));
            game.phase_in(other);
            assert_eq!(game.calculated_power(host), Some(3));
            game.set_current_controller(host, b()).unwrap();
            assert_eq!(game.calculated_power(host), Some(3));
            game.move_object_by_effect(other, Zone::Graveyard).unwrap();
            assert_eq!(game.calculated_power(host), Some(2));
        }
    }
}

#[test]
fn absent_opponent_population_uses_all_opponents_and_the_hosts_live_controller() {
    for (name, printed, bonus) in [("Skittish Kavu", 1, 1), ("Vexing Beetle", 3, 3)] {
        for definition in fixture_definitions(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(printed + bonus));
            let own = population(&mut game, a(), ColorSet::WHITE, true, Zone::Battlefield);
            population(&mut game, b(), ColorSet::BLUE, false, Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(printed + bonus));
            let foreign = population(&mut game, c(), ColorSet::BLUE, true, Zone::Battlefield);
            assert_eq!(game.calculated_power(host), Some(printed));
            game.phase_out(foreign);
            assert_eq!(game.calculated_power(host), Some(printed + bonus));
            game.set_current_controller(host, b()).unwrap();
            assert_eq!(game.calculated_power(host), Some(printed), "Alice is now an opponent");
            game.move_object_by_effect(own, Zone::Graveyard).unwrap();
            assert_eq!(game.calculated_power(host), Some(printed + bonus));
            game.phase_in(foreign);
            assert_eq!(game.calculated_power(host), Some(printed));
        }
    }
    for definition in fixture_definitions("Kavu Runner") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Haste));
        population(&mut game, b(), ColorSet::GREEN, true, Zone::Battlefield);
        population(&mut game, a(), ColorSet::BLUE, true, Zone::Battlefield);
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Haste), "unmatched colors and own creatures do not count");
        let foreign = population(&mut game, b(), ColorSet::WHITE, true, Zone::Battlefield);
        assert!(!game.current_has_static_ability_id(host, StaticAbilityId::Haste));
        game.move_object_by_effect(foreign, Zone::Graveyard).unwrap();
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Haste));
    }
}

#[test]
fn creature_counter_presence_grants_real_keywords_and_revokes_them_live() {
    for definition in fixture_definitions("Tenacious Hunter") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let unrelated = population(&mut game, b(), ColorSet::RED, true, Zone::Battlefield);
        let not_creature = population(&mut game, c(), ColorSet::BLACK, false, Zone::Battlefield);
        game.add_counters(not_creature, CounterType::MinusOneMinusOne, 1).unwrap();
        game.add_counters(unrelated, CounterType::PlusOnePlusOne, 1).unwrap();
        for keyword in [StaticAbilityId::Vigilance, StaticAbilityId::Deathtouch] {
            assert!(!game.current_has_static_ability_id(host, keyword));
        }
        game.remove_counters(unrelated, CounterType::PlusOnePlusOne, 1, None, None);
        game.add_counters(unrelated, CounterType::MinusOneMinusOne, 1).unwrap();
        for keyword in [StaticAbilityId::Vigilance, StaticAbilityId::Deathtouch] {
            assert!(game.current_has_static_ability_id(host, keyword));
            assert!(!game.current_has_static_ability_id(unrelated, keyword));
        }
        game.phase_out(unrelated);
        assert!(!game.current_has_static_ability_id(host, StaticAbilityId::Vigilance));
        game.phase_in(unrelated);
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Vigilance));
        game.remove_counters(unrelated, CounterType::MinusOneMinusOne, 1, None, None);
        assert!(!game.current_has_static_ability_id(host, StaticAbilityId::Deathtouch));
        game.add_counters(host, CounterType::MinusOneMinusOne, 1).unwrap();
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Deathtouch), "the source can be the counted creature");
    }
}

#[test]
fn color_changing_layer_is_visible_to_the_conditional_power_layer() {
    for definition in fixture_definitions("Halam Djinn") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        assert_eq!(game.calculated_power(host), Some(4));
        for painter in definitions("Unlisted Blue Painter", "Type: Enchantment\nAll permanents are blue.") {
            let painting = game.create_object_from_definition(&painter, b(), Zone::Battlefield);
            assert_eq!(game.current_colors(host), Some(ColorSet::BLUE));
            assert_eq!(game.calculated_power(host), Some(6));
            game.move_object_by_effect(painting, Zone::Graveyard).unwrap();
            assert_eq!(game.calculated_power(host), Some(4));
        }
        for painter in definitions("Unlisted Colorless Painter", "Type: Enchantment\nAll permanents are colorless.") {
            let painting = game.create_object_from_definition(&painter, b(), Zone::Battlefield);
            assert_eq!(game.current_colors(host), Some(ColorSet::COLORLESS));
            assert_eq!(game.calculated_power(host), Some(6), "no color is present among permanents");
            game.move_object_by_effect(painting, Zone::Graveyard).unwrap();
            assert_eq!(game.calculated_power(host), Some(4));
        }
    }
}
