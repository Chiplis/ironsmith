//! "Pay any amount of mana" payments, source-authored and UNRUN:
//! - one payer, colored ("you may pay any amount of {R}. When you do, it
//!   deals that much damage to any target", Leyline Tyrant): a {X} payment
//!   whose X may be paid only with red mana (CR 107.3), read by the reflexive
//!   trigger (CR 603.12);
//! - one payer, then "Prevent X of that damage" (Errant Minion, Power Leak):
//!   the payment scope binds X and a shield covers only the wrapped damage
//!   (CR 615.7);
//! - every player, then each player's own amount (Liege of the Hollows): all
//!   payments happen in APNAP order (CR 101.4) before any tokens, and each
//!   player's program reads that player's payment;
//! - Karn, Living Legacy's "Pay any amount of mana. Look at that many cards".
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::{
    CollectManaPaymentsEffect, DealDamageEffect, GainLifeEffect, PayManaEffect,
    PreventDamagePortionEffect, ReflexiveTriggerEffect,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::Zone;

#[path = "cf8_p08/support.rs"]
mod support;
#[path = "cf8_p08/play.rs"]
mod play;

const LEYLINE_TYRANT: &str = "Mana cost: {2}{R}{R}\nType: Creature — Dragon\nPower/Toughness: 4/4\nFlying\nYou don't lose unspent red mana as steps and phases end.\nWhen this creature dies, you may pay any amount of {R}. When you do, it deals that much damage to any target.";
const LIEGE_OF_THE_HOLLOWS: &str = "Mana cost: {2}{G}{G}\nType: Creature — Spirit\nPower/Toughness: 3/4\nWhen this creature dies, each player may pay any amount of mana. Then each player creates a number of 1/1 green Squirrel creature tokens equal to the amount of mana they paid this way.";
const ERRANT_MINION: &str = "Mana cost: {2}{U}\nType: Enchantment — Aura\nEnchant creature\nAt the beginning of the upkeep of enchanted creature's controller, that player may pay any amount of mana. This Aura deals 2 damage to that player. Prevent X of that damage, where X is the amount of mana that player paid this way.";
const POWER_LEAK: &str = "Mana cost: {1}{U}\nType: Enchantment — Aura\nEnchant enchantment\nAt the beginning of the upkeep of enchanted enchantment's controller, that player may pay any amount of mana. This Aura deals 2 damage to that player. Prevent X of that damage, where X is the amount of mana that player paid this way.";
const KARN_LIVING_LEGACY: &str = "Mana cost: {4}\nType: Legendary Planeswalker — Karn\nLoyalty: 4\n+1: Create a tapped Powerstone token. (It's an artifact with \"{T}: Add {C}. This mana can't be spent to cast a nonartifact spell.\")\n−1: Pay any amount of mana. Look at that many cards from the top of your library, then put one of those cards into your hand and the rest on the bottom of your library in a random order.\n−7: You get an emblem with \"Tap an untapped artifact you control: This emblem deals 1 damage to any target.\"";

#[test]
fn leyline_tyrant_pays_red_only_x_and_its_reflexive_trigger_deals_that_much() {
    for definition in support::definitions("Leyline Tyrant", LEYLINE_TYRANT) {
        let payments = support::find_all::<PayManaEffect>(&definition);
        assert_eq!(payments.len(), 1);
        let payment = &payments[0];
        assert!(payment.cost.has_x(), "{:?}", payment.cost);
        assert!(payment.cost.has_x_spending_restriction(), "red only: {:?}", payment.cost);
        assert!(payment.x_value.is_none() && payment.x_maximum.is_none());
        let reflexive = support::find_all::<ReflexiveTriggerEffect>(&definition);
        assert_eq!(reflexive.len(), 1, "When you do");
        let damage = support::find_all::<DealDamageEffect>(&definition);
        assert_eq!(damage.len(), 1);
        assert!(
            matches!(
                damage[0].amount.unhinted(),
                Value::EffectValue(_) | Value::X
            ),
            "that much: {:?}",
            damage[0].amount
        );
        assert!(damage[0].target.is_target(), "any target");
    }
}

#[test]
fn errant_minion_and_power_leak_prevent_the_paid_amount_of_that_damage() {
    for (name, body) in [("Errant Minion", ERRANT_MINION), ("Power Leak", POWER_LEAK)] {
        for definition in support::definitions(name, body) {
            let collect = support::find_all::<CollectManaPaymentsEffect>(&definition);
            assert_eq!(collect.len(), 1, "{name}");
            assert!(collect[0].payers.is_some(), "{name}: only that player pays");
            assert!(!collect[0].per_payer);
            let portion = support::find_all::<PreventDamagePortionEffect>(&definition);
            assert_eq!(portion.len(), 1, "{name}");
            assert_eq!(portion[0].amount, Value::X, "{name}");
            let damage = support::find_all::<DealDamageEffect>(&definition);
            assert_eq!(damage.len(), 1, "{name}");
            assert_eq!(damage[0].amount, Value::Fixed(2), "{name}");
        }
    }
}

#[test]
fn a_portion_shield_covers_only_the_wrapped_damage() {
    for (prevented, first_loss) in [(1, 1), (5, 0)] {
        let mut game = play::game();
        let source = game.create_object_from_definition(
            &play::vanilla("Source", "{U}", "Spirit", 1, 1),
            play::A,
            Zone::Battlefield,
        );
        let deal = |amount| {
            Effect::new(DealDamageEffect::new(
                amount,
                ChooseSpec::SpecificPlayer(play::B),
            ))
        };
        play::apply(
            &mut game,
            source,
            Effect::new(PreventDamagePortionEffect::new(
                Value::Fixed(prevented),
                vec![deal(2)],
            )),
        );
        assert_eq!(play::life(&game, play::B), 20 - first_loss);
        // An unused remainder never reaches later damage.
        play::apply(&mut game, source, deal(2));
        assert_eq!(play::life(&game, play::B), 20 - first_loss - 2);
        assert!(game.effect_store.prevention_effects.shields().is_empty());
    }
}

#[test]
fn liege_of_the_hollows_pays_in_apnap_order_then_each_player_uses_their_own_amount() {
    for definition in support::definitions("Liege of the Hollows", LIEGE_OF_THE_HOLLOWS) {
        let collect = support::find_all::<CollectManaPaymentsEffect>(&definition);
        assert_eq!(collect.len(), 1);
        assert!(collect[0].per_payer && collect[0].apnap_order);
        assert!(collect[0].payers.is_none(), "each player");
        let debug = format!("{:?}", collect[0].effects);
        assert!(debug.contains("Squirrel"), "{debug}");
        assert!(debug.contains("IteratedPlayer"), "{debug}");
        assert!(!debug.contains("ForPlayers"), "the payer loop owns the iteration: {debug}");
    }

    // The engine scope: A pays 2, B pays 1, C pays nothing; each gains their
    // own amount.
    let mut game = play::game();
    play::give_mana(&mut game, play::A, ManaSymbol::Green, 2);
    play::give_mana(&mut game, play::B, ManaSymbol::Green, 1);
    let source = game.create_object_from_definition(
        &play::vanilla("Source", "{G}", "Spirit", 1, 1),
        play::A,
        Zone::Battlefield,
    );
    let mut dm = play::Script {
        numbers: vec![2, 1, 0],
        ..Default::default()
    };
    play::apply_with(
        &mut game,
        source,
        Effect::new(
            CollectManaPaymentsEffect::new(vec![Effect::new(GainLifeEffect::with_filter(
                Value::X,
                PlayerFilter::IteratedPlayer,
            ))])
            .in_apnap_order()
            .per_payer(),
        ),
        &mut dm,
    );
    assert_eq!(play::life(&game, play::A), 22);
    assert_eq!(play::life(&game, play::B), 21);
    assert_eq!(play::life(&game, play::C), 20);
    let payers = dm
        .number_prompts
        .iter()
        .map(|(player, _)| *player)
        .collect::<Vec<_>>();
    assert_eq!(payers, vec![play::A, play::B, play::C], "APNAP from the active player");
}

#[test]
fn karn_pays_any_amount_and_looks_at_that_many() {
    for definition in support::definitions("Karn, Living Legacy", KARN_LIVING_LEGACY) {
        let payments = support::find_all::<PayManaEffect>(&definition);
        assert_eq!(payments.len(), 1);
        assert!(payments[0].cost.has_x());
        assert!(!payments[0].cost.has_x_spending_restriction());
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("EffectValue") || debug.contains("ManaPaid"), "{debug}");
    }
}
