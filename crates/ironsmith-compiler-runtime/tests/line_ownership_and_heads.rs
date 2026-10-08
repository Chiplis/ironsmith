//! Lines that failed because a narrower reading claimed them or a clause
//! head lacked its verb: a leading "As long as" condition on a don't-untap
//! line, a turn-scoped enter-tapped rule after other sentences, "you
//! earthbend N", "tap or untap target creature", a "Ward—Get N poison
//! counters" payment, a duration-scoped trigger "until the end of your next
//! turn", "draw a card for each player who was dealt combat damage this
//! turn", and "a card with a kicker ability". Source-authored, UNRUN.
#[path = "cf8_p08/support.rs"]
mod support;

const BODIES: &[(&str, &str, &str)] = &[
    ("Winter's Rest", "Mana cost: {1}{U}\nType: Snow Enchantment — Aura\nEnchant creature\nWhen this Aura enters, tap enchanted creature.\nAs long as you control another snow permanent, enchanted creature doesn't untap during its controller's untap step.", "untap"),
    ("Nahiri's Lithoforming", "Mana cost: {X}{R}{R}\nType: Sorcery\nSacrifice X lands. For each land sacrificed this way, draw a card. You may play X additional lands this turn. Lands you control enter tapped this turn.", "enter tapped"),
    ("Fatal Fissure", "Mana cost: {1}{B}\nType: Instant\nChoose target creature. When that creature dies this turn, you earthbend 4.", "arthbend"),
    ("Tolarian Kraken", "Mana cost: {4}{U}{U}\nType: Creature — Kraken\nPower/Toughness: 4/6\nWhenever you draw a card, you may pay {1}. When you do, you may tap or untap target creature.", "untap"),
    ("The Serpent Society", "Mana cost: {1}{B}{G}\nType: Legendary Creature — Human Snake Villain\nPower/Toughness: 3/4\nDeathtouch\nWard—Get five poison counters.\nWhenever another creature you control with deathtouch dies, each opponent sacrifices a nontoken creature of their choice.", "poison"),
    ("Season of the Bold", "Mana cost: {3}{R}{R}\nType: Sorcery\nChoose up to five {P} worth of modes. You may choose the same mode more than once.\n{P} — Create a tapped Treasure token.\n{P}{P} — Exile the top two cards of your library. Until the end of your next turn, you may play them.\n{P}{P}{P} — Until the end of your next turn, whenever you cast a spell, Season of the Bold deals 2 damage to up to one target creature.", "next turn"),
    ("Vivien's Stampede", "Mana cost: {4}{G}{G}\nType: Sorcery\nEach creature you control gains vigilance, trample, and melee until end of turn.\nAt the beginning of the next main phase this turn, draw a card for each player who was dealt combat damage this turn.", "combat damage"),
    ("Coralhelm Chronicler", "Mana cost: {2}{U}\nType: Creature — Merfolk Wizard\nPower/Toughness: 2/2\nWhenever you cast a kicked spell, draw a card, then discard a card.\nWhen this creature enters, look at the top five cards of your library. You may reveal a card with a kicker ability from among them and put it into your hand. Put the rest on the bottom of your library in a random order.", "kicker"),
];

#[test]
fn each_line_compiles_through_its_owner() {
    for (name, body, needle) in BODIES {
        for definition in support::definitions(name, body) {
            let text = support::rendered(&definition);
            assert!(text.to_ascii_lowercase().contains(&needle.to_ascii_lowercase()), "{name}: {text}");
        }
    }
}

#[test]
fn winters_rest_untap_rule_keeps_its_snow_condition() {
    for definition in support::definitions("Winter's Rest", BODIES[0].1) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("Snow"), "{debug}");
    }
}

#[test]
fn vivien_counts_every_player_not_only_opponents() {
    for definition in support::definitions("Vivien's Stampede", BODIES[6].1) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("PlayersDealtCombatDamageBy"), "{debug}");
        assert!(debug.contains("players: Any"), "{debug}");
    }
}

#[test]
fn kicker_ability_filter_is_a_typed_marker() {
    for definition in support::definitions("Coralhelm Chronicler", BODIES[7].1) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("\"kicker\""), "{debug}");
    }
}
