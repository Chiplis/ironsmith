//! Claude package 74 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p74_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

/// Wave of Indifference / Runed Arch / Open into Wonder: a plain "X target
/// creatures" restriction subject is the ordinary dynamic-X target count.
#[test]
fn p74_plain_x_target_count_restriction_subjects_compile() {
    let wave = p74_compile(
        "Wave of Indifference",
        "Mana cost: {X}{R}\nType: Sorcery\nX target creatures can't block this turn.",
    );
    let debug = format!("{:?}", wave.spell_effect);
    assert!(debug.contains("dynamic_x: true"), "X targets: {debug}");

    let arch = p74_compile(
        "Runed Arch",
        "Mana cost: {3}\nType: Artifact\nThis artifact enters tapped.\n{X}, {T}, Sacrifice this artifact: X target creatures with power 2 or less can't be blocked this turn.",
    );
    let debug = format!("{:?}", arch.abilities);
    assert!(debug.contains("dynamic_x: true"), "X targets: {debug}");
    assert!(debug.contains("power"), "power 2 or less filter: {debug}");

    let wonder = p74_compile(
        "Open into Wonder",
        "Mana cost: {X}{U}{U}\nType: Sorcery\nX target creatures can't be blocked this turn. Until end of turn, those creatures gain \"Whenever this creature deals combat damage to a player, draw a card.\"",
    );
    let debug = format!("{:?}", wonder.spell_effect);
    assert!(debug.contains("dynamic_x: true"), "X targets: {debug}");
}

/// Vhati il-Dal: the controller chooses base power 1 or base toughness 1.
#[test]
fn p74_vhati_chooses_base_power_or_base_toughness() {
    let definition = p74_compile(
        "Vhati il-Dal",
        "Mana cost: {2}{B}{G}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 3/3\n{T}: Until end of turn, target creature has base power 1 or base toughness 1.",
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("SetPower"), "{debug}");
    assert!(debug.contains("SetToughness"), "{debug}");
}

/// Multiclass Baldric: four keyword grants, each under its own party-class
/// condition.
#[test]
fn p74_multiclass_baldric_grants_each_keyword_under_its_own_condition() {
    let definition = p74_compile(
        "Multiclass Baldric",
        "Mana cost: {1}\nType: Artifact — Equipment\nEquipped creature has lifelink if you control a Cleric, deathtouch if you control a Rogue, haste if you control a Warrior, and flying if you control a Wizard.\nAs long as you have a full party, prevent all damage that would be dealt to equipped creature.\nEquip {2}",
    );
    let debug = format!("{:?}", definition.abilities);
    for word in ["Lifelink", "Deathtouch", "Haste", "Flying", "Cleric", "Rogue", "Warrior", "Wizard"] {
        assert!(debug.contains(word), "missing {word}: {debug}");
    }
}

/// Omnath, Locus of All: the any-combination mana clause and the following
/// "put it into your hand" are two coordinated actions.
#[test]
fn p74_omnath_any_combination_mana_splits_from_following_action() {
    let definition = p74_compile(
        "Omnath, Locus of All",
        "Mana cost: {W}{U}{B/P}{R}{G}\nType: Legendary Creature — Phyrexian Elemental\nPower/Toughness: 4/4\nIf you would lose unspent mana, that mana becomes black instead.\nAt the beginning of your first main phase, look at the top card of your library. You may reveal that card if it has three or more colored mana symbols in its mana cost. If you do, add three mana in any combination of its colors and put it into your hand. If you don't reveal it, put it into your hand.",
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("available_colors_among"), "colors read from the card: {debug}");
    assert!(debug.contains("Hand"), "the card goes to hand: {debug}");
}

/// Overwhelming Splendor: the enchanted player can still activate mana and
/// loyalty abilities.
#[test]
fn p74_overwhelming_splendor_bans_non_mana_non_loyalty_activations() {
    let definition = p74_compile(
        "Overwhelming Splendor",
        "Mana cost: {6}{W}{W}\nType: Enchantment — Aura Curse\nEnchant player\nCreatures enchanted player controls lose all abilities and have base power and toughness 1/1.\nEnchanted player can't activate abilities that aren't mana abilities or loyalty abilities.",
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ActivateNonManaNonLoyaltyAbilities"), "{debug}");
}

/// Pandemonium: the entering creature's controller makes the may decision and
/// chooses the target.
#[test]
fn p74_pandemonium_entering_controller_decides_and_targets() {
    let definition = p74_compile(
        "Pandemonium",
        "Mana cost: {3}{R}\nType: Enchantment\nWhenever a creature enters, that creature's controller may have it deal damage equal to its power to any target of their choice.",
    );
    let debug = format!("{definition:?}");
    assert!(debug.contains("MayEffect") && debug.contains("decider: Some(ControllerOf("), "{debug}");
    assert!(debug.contains("TargetOnlyEffect") && debug.contains("chooser: Some(ControllerOf("), "{debug}");
}

/// Serendib Djinn: the damage is gated on the sacrificed land being an Island.
#[test]
fn p74_serendib_djinn_island_sacrifice_gate() {
    let definition = p74_compile(
        "Serendib Djinn",
        "Mana cost: {2}{U}{U}\nType: Creature — Djinn\nPower/Toughness: 5/6\nFlying\nAt the beginning of your upkeep, sacrifice a land. If you sacrifice an Island this way, this creature deals 3 damage to you.\nWhen you control no lands, sacrifice this creature.",
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Island"), "{debug}");
    assert!(debug.contains("DealDamage"), "{debug}");
}

/// Soul Sculptor: both changes share the "until a player casts a creature
/// spell" duration.
#[test]
fn p74_soul_sculptor_becomes_enchantment_and_loses_abilities() {
    let definition = p74_compile(
        "Soul Sculptor",
        "Mana cost: {2}{W}\nType: Creature — Human\nPower/Toughness: 1/1\n{1}{W}, {T}: Target creature becomes an enchantment and loses all abilities until a player casts a creature spell.",
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Enchantment"), "{debug}");
    assert!(
        debug.matches("AnyPlayerCastsCreatureSpell").count() >= 2,
        "both changes end when a creature spell is cast: {debug}"
    );
}

/// Starving Revenant: the draw and life loss repeat per card kept on top.
#[test]
fn p74_starving_revenant_repeats_per_surveil_card_kept_on_top() {
    let definition = p74_compile(
        "Starving Revenant",
        "Mana cost: {2}{B}{B}\nType: Creature — Spirit Horror\nPower/Toughness: 4/4\nWhen this creature enters, surveil 2. Then for each card you put on top of your library, you draw a card and you lose 3 life.\nDescend 8 — Whenever you draw a card, if there are eight or more permanent cards in your graveyard, target opponent loses 1 life and you gain 1 life.",
    );
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("__surveil_kept_on_top__"), "{debug}");
}
