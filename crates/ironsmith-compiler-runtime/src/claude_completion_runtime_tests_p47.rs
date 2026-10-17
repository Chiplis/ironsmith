//! Claude package 47 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

/// "Enchanted creature gets +3/+3 unless <condition>": the anthem keeps the
/// as-long-as condition machinery but negates the condition.
#[test]
fn anthem_unless_tail_negates_the_static_condition() {
    let definition = compile_to_runtime_definition(
        "Heroic Defiance",
        "Mana cost: {2}{W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +3/+3 unless it shares a color with the most common color among all permanents or a color tied for most common.",
        false,
    )
    .unwrap_or_else(|error| panic!("Heroic Defiance should compile: {error:?}"));
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Not("), "{debug}");
    assert!(debug.contains("SharesMostCommonPermanentColor"), "{debug}");
}

/// "any other player may pay that spell's mana cost. If a player does,
/// counter the spell.": a turn-order offer to everyone but the caster, paying
/// the triggering spell's exact mana cost, and "the spell" is the triggering
/// spell rather than a new target.
#[test]
fn any_other_player_may_pay_the_triggering_spells_mana_cost() {
    let definition = compile_to_runtime_definition(
        "Ice Cave",
        "Mana cost: {3}{U}{U}\nType: Enchantment\nWhenever a player casts a spell, any other player may pay that spell's mana cost. If a player does, counter the spell. (Mana cost includes color.)",
        false,
    )
    .unwrap_or_else(|error| panic!("Ice Cave should compile: {error:?}"));
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("Excluding"), "the caster is not offered the payment: {debug}");
    assert!(debug.contains("mana_cost_of: Some(Tagged"), "{debug}");
    assert!(debug.contains("CounterEffect { target: Tagged"), "the spell is the triggering spell, not a target: {debug}");
    assert!(!debug.contains("CounterEffect { target: Target"), "{debug}");
}

/// "If this spell was cast from a graveyard, you may play cards this way
/// without paying their mana costs.": the same until-end-of-your-next-turn
/// permission is granted again behind the condition with an empty casting
/// price.
#[test]
fn conditional_free_play_this_way_regrants_the_prior_permission_for_free() {
    let definition = compile_to_runtime_definition(
        "Ignite the Future",
        "Mana cost: {3}{R}\nType: Sorcery\nExile the top three cards of your library. Until the end of your next turn, you may play those cards. If this spell was cast from a graveyard, you may play cards this way without paying their mana costs.\nFlashback {7}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        false,
    )
    .unwrap_or_else(|error| panic!("Ignite the Future should compile: {error:?}"));
    let debug = format!("{:?}", definition.abilities);
    assert_eq!(debug.matches("GrantPlayTaggedEffect").count(), 2, "{debug}");
    assert!(debug.matches("UntilYourNextTurnEnd").count() >= 2, "{debug}");
    assert!(debug.contains("alternative_cost: Some"), "{debug}");
    assert!(debug.contains("Graveyard"), "the free grant is gated on the cast origin: {debug}");
}

/// "If you control Equipment named A, B, and C, create <token>. Attach those
/// Equipment to it.": three independent named-control conditions, and the
/// attachment names exactly those Equipment rather than the created token.
#[test]
fn serial_named_control_condition_attaches_those_equipment_to_the_token() {
    let definition = compile_to_runtime_definition(
        "Helm of Kaldra",
        "Mana cost: {3}\nType: Legendary Artifact — Equipment\nEquipped creature has first strike, trample, and haste.\n{1}: If you control Equipment named Helm of Kaldra, Sword of Kaldra, and Shield of Kaldra, create Kaldra, a legendary 4/4 colorless Avatar creature token. Attach those Equipment to it.\nEquip {2}",
        false,
    )
    .unwrap_or_else(|error| panic!("Helm of Kaldra should compile: {error:?}"));
    let debug = format!("{:?}", definition.abilities).to_ascii_lowercase();
    for name in ["helm of kaldra", "sword of kaldra", "shield of kaldra"] {
        assert!(debug.matches(name).count() >= 2, "{name} is both required and attached: {debug}");
    }
    assert!(debug.contains("attachobjectseffect"), "{debug}");
}
