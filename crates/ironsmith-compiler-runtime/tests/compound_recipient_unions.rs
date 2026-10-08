//! "X and each Y" recipient unions outside the damage family, source-authored
//! and UNRUN:
//! - prevention: "Prevent the next 1 damage that would be dealt to each
//!   creature and each player this turn" gives every creature and every
//!   player its own 1-point shield (Kitsune Palliator, CR 615.7);
//! - protection: "you and each permanent you control gain protection from the
//!   color of your choice" is one color choice granted to the player and to
//!   every permanent they control (Faith's Shield, CR 702.16);
//! - combat prevention source set: "by it and each creature it's blocking" is
//!   the target plus the attackers it blocks (Sewers of Estark, CR 506.4).
use ironsmith::effects::PreventDamageEffect;
use ironsmith::{Target, Zone};

#[path = "cf8_p08/support.rs"]
mod support;
#[path = "cf8_p08/play.rs"]
mod play;

const KITSUNE_PALLIATOR: &str = "Mana cost: {2}{W}\nType: Creature — Fox Cleric\nPower/Toughness: 0/2\n{T}: Prevent the next 1 damage that would be dealt to each creature and each player this turn.";
const FAITHS_SHIELD: &str = "Mana cost: {W}\nType: Instant\nTarget permanent you control gains protection from the color of your choice until end of turn.\nFateful hour — If you have 5 or less life, instead you and each permanent you control gain protection from the color of your choice until end of turn.";
const SEWERS_OF_ESTARK: &str = "Mana cost: {2}{B}{B}\nType: Instant\nChoose target creature. If it's attacking, it can't be blocked this turn. If it's blocking, prevent all combat damage that would be dealt this combat by it and each creature it's blocking.";

#[test]
fn kitsune_palliator_shields_each_creature_and_each_player() {
    for definition in support::definitions("Kitsune Palliator", KITSUNE_PALLIATOR) {
        let shields = support::find_all::<PreventDamageEffect>(&definition);
        assert_eq!(shields.len(), 2, "one per-object and one per-player shield");
        assert!(shields.iter().all(|shield| !shield.target.is_target()));
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("ForPlayers") || debug.contains("ForEachPlayer"), "{debug}");
        assert!(debug.contains("Iterated"), "{debug}");

        let mut game = play::game();
        let source = game.create_object_from_definition(&definition, play::A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let ours = game.create_object_from_definition(
            &play::vanilla("Ours", "{1}", "Bear", 2, 4),
            play::A,
            Zone::Battlefield,
        );
        let theirs = game.create_object_from_definition(
            &play::vanilla("Theirs", "{1}", "Bear", 2, 4),
            play::B,
            Zone::Battlefield,
        );
        let dealer = game.create_object_from_definition(
            &play::vanilla("Dealer", "{R}", "Goblin", 1, 1),
            play::C,
            Zone::Battlefield,
        );
        let mut dm = play::Script::default();
        play::activate(&mut game, source, play::activated_at(&definition, 0), &mut dm);
        play::resolve_all(&mut game, &mut dm);

        play::damage(&mut game, dealer, Target::Object(ours), 3);
        play::damage(&mut game, dealer, Target::Object(theirs), 3);
        assert_eq!(game.damage_on(ours), 2);
        assert_eq!(game.damage_on(theirs), 2);
        for player in [play::A, play::B, play::C] {
            let before = play::life(&game, player);
            play::damage(&mut game, dealer, Target::Player(player), 3);
            assert_eq!(play::life(&game, player), before - 2, "{player:?}");
            // Each shield is spent by its first damage event.
            let before = play::life(&game, player);
            play::damage(&mut game, dealer, Target::Player(player), 3);
            assert_eq!(play::life(&game, player), before - 3, "{player:?}");
        }
    }
}

#[test]
fn faiths_shield_fateful_hour_protects_you_and_each_permanent_with_one_choice() {
    for definition in support::definitions("Faith's Shield", FAITHS_SHIELD) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("ChooseModeEffect"), "{debug}");
        assert!(debug.contains("lock_filter_at_resolution: true"), "{debug}");

        let mut game = play::game();
        play::give_mana(&mut game, play::A, ironsmith::mana::ManaSymbol::White, 1);
        game.player_mut(play::A).unwrap().life = 5;
        let first = game.create_object_from_definition(
            &play::vanilla("First", "{1}", "Bear", 2, 2),
            play::A,
            Zone::Battlefield,
        );
        let second = game.create_object_from_definition(
            &play::vanilla("Second", "{1}", "Bear", 2, 2),
            play::A,
            Zone::Battlefield,
        );
        let white = game.create_object_from_definition(
            &play::vanilla("White dealer", "{W}", "Soldier", 1, 1),
            play::B,
            Zone::Battlefield,
        );
        let mut dm = play::Script {
            targets: vec![Target::Object(first)],
            ..Default::default()
        };
        play::cast(&mut game, play::A, &definition, &mut dm);
        play::resolve_all(&mut game, &mut dm);
        let mode_prompts = dm
            .option_prompts
            .iter()
            .filter(|(_, description)| description.starts_with("Choose mode"))
            .count();
        assert_eq!(mode_prompts, 1, "one color choice for every recipient");

        // The first offered color is white; white damage to the player and to
        // both permanents is prevented.
        play::damage(&mut game, white, Target::Player(play::A), 2);
        assert_eq!(play::life(&game, play::A), 5);
        play::damage(&mut game, white, Target::Object(first), 1);
        play::damage(&mut game, white, Target::Object(second), 1);
        assert_eq!(game.damage_on(first), 0);
        assert_eq!(game.damage_on(second), 0);
    }
}

#[test]
fn sewers_of_estark_blocking_branch_prevents_the_target_and_the_creatures_it_blocks() {
    for definition in support::definitions("Sewers of Estark", SEWERS_OF_ESTARK) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("is_target_object: true"), "{debug}");
        assert!(debug.contains("in_combat_with: Some(Target)"), "{debug}");
        assert!(debug.contains("EndOfCombat"), "{debug}");
        assert!(debug.contains("blocking"), "the branch is gated on blocking: {debug}");
    }
}
