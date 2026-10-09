//! Join forces (an ability word, CR 207.2c): the collective payment wrapper
//! binds no player, its body's own per-player loop does, so "Each player draws
//! X cards" is a bound iteration rather than an orphan "that player". The
//! payment head is accepted in both printed orders ("Starting with you, each
//! player may pay ..." and "each player starting with you may pay ...").
//! Frozen complete bodies; source-authored and UNRUN. The three sorceries'
//! gameplay is covered by `join_forces.rs`.
use ironsmith::effect::{Until, Value};
use ironsmith::effects::{CollectManaPaymentsEffect, ForPlayersEffect, ModifyPowerToughnessEffect};
use ironsmith::target::PlayerFilter;

#[path = "cf8_p08/support.rs"]
mod support;

const SORCERIES: &[(&str, &str)] = &[
    ("Minds Aglow", "Mana cost: {U}\nType: Sorcery\nJoin forces — Starting with you, each player may pay any amount of mana. Each player draws X cards, where X is the total amount of mana paid this way."),
    ("Collective Voyage", "Mana cost: {G}\nType: Sorcery\nJoin forces — Starting with you, each player may pay any amount of mana. Each player searches their library for up to X basic land cards, where X is the total amount of mana paid this way, puts them onto the battlefield tapped, then shuffles."),
    ("Alliance of Arms", "Mana cost: {W}\nType: Sorcery\nJoin forces — Starting with you, each player may pay any amount of mana. Each player creates X 1/1 white Soldier creature tokens, where X is the total amount of mana paid this way."),
];
const MANA_CHARGED_DRAGON: &str = "Mana cost: {4}{R}{R}\nType: Creature — Dragon\nPower/Toughness: 5/5\nFlying, trample\nJoin forces — Whenever this creature attacks or blocks, each player starting with you may pay any amount of mana. This creature gets +X/+0 until end of turn, where X is the total amount of mana paid this way.";

#[test]
fn each_player_body_is_a_bound_loop_inside_the_collective_payment() {
    for (name, body) in SORCERIES {
        for definition in support::definitions(name, body) {
            let collect = support::find_all::<CollectManaPaymentsEffect>(&definition);
            assert_eq!(collect.len(), 1, "{name}");
            let loops = support::find_all::<ForPlayersEffect>(&definition);
            assert_eq!(loops.len(), 1, "{name}");
            assert_eq!(loops[0].filter, PlayerFilter::Any, "{name}: every player");
        }
    }
}

#[test]
fn mana_charged_dragon_pumps_itself_by_the_collective_total() {
    for definition in support::definitions("Mana-Charged Dragon", MANA_CHARGED_DRAGON) {
        let collect = support::find_all::<CollectManaPaymentsEffect>(&definition);
        assert_eq!(collect.len(), 1);
        let pumps = support::find_all::<ModifyPowerToughnessEffect>(&definition);
        assert_eq!(pumps.len(), 1);
        assert_eq!(pumps[0].power, Value::X, "X is the accepted total");
        assert_eq!(pumps[0].toughness, Value::Fixed(0));
        assert_eq!(pumps[0].duration, Until::EndOfTurn);
        assert!(support::find_all::<ForPlayersEffect>(&definition).is_empty());
    }
}
