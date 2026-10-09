//! "An opponent chooses one —", source-authored and UNRUN. The modes are
//! chosen while the spell is cast (CR 601.2b) by an opponent (CR 700.2); with
//! several opponents the caster picks which. That opponent is the spell's
//! chosen player: "that player" in the modes and in their target
//! restrictions names them, and the caster still chooses targets (601.2c).
use ironsmith::effects::ChooseModeEffect;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::PlayerFilter;
use ironsmith::{Target, Zone};

#[path = "cf8_p08/support.rs"]
mod support;
#[path = "cf8_p08/play.rs"]
mod play;

const FATAL_LORE: &str = "Mana cost: {2}{B}{B}\nType: Sorcery\nAn opponent chooses one —\n• You draw three cards.\n• You destroy up to two target creatures that player controls. They can't be regenerated. That player draws up to three cards.";
const LIBRARY_OF_LAT_NAM: &str = "Mana cost: {4}{U}\nType: Sorcery\nAn opponent chooses one —\n• You draw three cards at the beginning of the next turn's upkeep.\n• You search your library for a card, put that card into your hand, then shuffle.";
const MISFORTUNE: &str = "Mana cost: {1}{B}{R}{G}\nType: Sorcery\nAn opponent chooses one —\n• You put a +1/+1 counter on each creature you control and gain 4 life.\n• You put a -1/-1 counter on each creature that player controls and Misfortune deals 4 damage to that player.";

fn modal(definition: &ironsmith::cards::CardDefinition) -> ChooseModeEffect {
    let modes = support::find_all::<ChooseModeEffect>(definition);
    assert_eq!(modes.len(), 1);
    modes[0].clone()
}

#[test]
fn every_opponent_chosen_modal_spell_names_an_opponent_cast_chooser() {
    for (name, body) in [
        ("Fatal Lore", FATAL_LORE),
        ("Library of Lat-Nam", LIBRARY_OF_LAT_NAM),
        ("Misfortune", MISFORTUNE),
    ] {
        for definition in support::definitions(name, body) {
            let choice = modal(&definition);
            assert_eq!(choice.cast_chooser, Some(PlayerFilter::Opponent), "{name}");
            assert_eq!(choice.chooser, None, "{name}: not a resolution-time choice");
            assert_eq!(choice.modes.len(), 2, "{name}");
            assert!(support::rendered(&definition).contains("An opponent chooses one"), "{name}");
        }
    }
    for definition in support::definitions("Misfortune", MISFORTUNE) {
        let debug = format!("{:?}", modal(&definition).modes[1].effects);
        assert!(debug.contains("ChosenPlayer"), "{debug}");
    }
}

#[test]
fn misfortune_mode_prompt_goes_to_the_chosen_opponent() {
    for definition in support::definitions("Misfortune", MISFORTUNE) {
        let mut game = play::game();
        for symbol in [ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            play::give_mana(&mut game, play::A, symbol, 1);
        }
        let mine = game.create_object_from_definition(
            &play::vanilla("Mine", "{1}", "Bear", 2, 2),
            play::A,
            Zone::Battlefield,
        );
        let bobs = game.create_object_from_definition(
            &play::vanilla("Bob's", "{1}", "Bear", 2, 2),
            play::B,
            Zone::Battlefield,
        );
        let charlies = game.create_object_from_definition(
            &play::vanilla("Charlie's", "{1}", "Bear", 2, 2),
            play::C,
            Zone::Battlefield,
        );
        let mut dm = play::Script {
            modes: vec![1],
            ..Default::default()
        };
        play::cast(&mut game, play::A, &definition, &mut dm);
        // Two opponents: the caster picks the chooser (the first offered,
        // Bob), and Bob gets the mode prompt.
        let chooser_prompt = dm
            .option_prompts
            .iter()
            .position(|(_, description)| description.starts_with("Choose a player to choose the mode"))
            .expect("caster picks which opponent chooses");
        assert_eq!(dm.option_prompts[chooser_prompt].0, play::A);
        let mode_prompts = dm
            .option_prompts
            .iter()
            .filter(|(_, description)| description.starts_with("Choose mode"))
            .collect::<Vec<_>>();
        assert_eq!(mode_prompts.len(), 1);
        assert_eq!(mode_prompts[0].0, play::B, "the opponent chooses the mode");
        play::resolve_all(&mut game, &mut dm);
        assert_eq!(play::life(&game, play::B), 16, "that player is the chooser");
        assert_eq!(play::life(&game, play::C), 20);
        assert_eq!(play::life(&game, play::A), 20);
        let counters =
            |id| game.counter_count(id, ironsmith::CounterType::MinusOneMinusOne);
        assert_eq!(counters(bobs), 1);
        assert_eq!(counters(charlies), 0);
        assert_eq!(counters(mine), 0);
    }
}

#[test]
fn fatal_lore_targets_only_creatures_the_choosing_opponent_controls() {
    for definition in support::definitions("Fatal Lore", FATAL_LORE) {
        let mut game = play::game();
        play::give_mana(&mut game, play::A, ManaSymbol::Black, 2);
        play::give_mana(&mut game, play::A, ManaSymbol::Colorless, 2);
        let bobs = game.create_object_from_definition(
            &play::vanilla("Bob's", "{1}", "Bear", 2, 2),
            play::B,
            Zone::Battlefield,
        );
        let charlies = game.create_object_from_definition(
            &play::vanilla("Charlie's", "{1}", "Bear", 2, 2),
            play::C,
            Zone::Battlefield,
        );
        let mut dm = play::Script {
            modes: vec![1],
            targets: vec![Target::Object(bobs)],
            numbers: vec![0],
            ..Default::default()
        };
        play::cast(&mut game, play::A, &definition, &mut dm);
        let offered = dm.offered_targets.first().expect("the caster chooses targets");
        assert!(offered.contains(&Target::Object(bobs)));
        assert!(!offered.contains(&Target::Object(charlies)), "that player controls");
        play::resolve_all(&mut game, &mut dm);
        assert!(game.object(bobs).is_none_or(|object| object.zone != Zone::Battlefield));
        assert!(game.object(charlies).is_some_and(|object| object.zone == Zone::Battlefield));
    }
}
