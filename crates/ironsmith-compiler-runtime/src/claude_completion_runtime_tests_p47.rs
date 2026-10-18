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
    let debug = format!("{:?}", definition.spell_effect);
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

/// "any number of target opponents each discard a card. For each of those
/// opponents who didn't discard a card with mana value 4 or greater, draw a
/// card.": one loop over the targeted opponents, checking each opponent's own
/// discarded card, with the draw made by the ability's controller.
#[test]
fn for_each_of_those_opponents_who_didnt_discard_checks_each_targeted_discard() {
    let definition = compile_to_runtime_definition(
        "Hollow Marauder",
        "Mana cost: {6}{B}\nType: Creature — Specter Rogue\nPower/Toughness: 4/2\nThis spell costs {1} less to cast for each creature card in your graveyard.\nFlying\nWhen this creature enters, any number of target opponents each discard a card. For each of those opponents who didn't discard a card with mana value 4 or greater, draw a card.",
        false,
    )
    .unwrap_or_else(|error| panic!("Hollow Marauder should compile: {error:?}"));
    let debug = format!("{:?}", definition.abilities);
    assert_eq!(debug.matches("ForPlayersEffect").count(), 1, "one per-target loop: {debug}");
    assert!(debug.contains("DiscardEffect"), "{debug}");
    assert!(debug.contains("PlayerTaggedObjectMatches"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

/// "put a charge counter on it or remove one from it. When you remove a
/// counter this way, choose one —": the reflexive modal trigger is attached to
/// the removal mode only, never to the put mode.
#[test]
fn when_you_remove_a_counter_this_way_is_a_reflexive_trigger_of_the_removal_mode() {
    let definition = compile_to_runtime_definition(
        "Immard, the Stormcleaver",
        "Mana cost: {2}{R}{W}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 3/3\nWhenever Immard, the Stormcleaver enters or attacks, put a charge counter on it or remove one from it. When you remove a counter this way, choose one —\n• Immard deals 4 damage to any target.\n• Immard gains lifelink and indestructible until end of turn.",
        false,
    )
    .unwrap_or_else(|error| panic!("Immard should compile: {error:?}"));
    let debug = format!("{:?}", definition.abilities);
    let remove = debug.find("RemoveCountersEffect").unwrap_or_else(|| panic!("{debug}"));
    let reflexive = debug.find("ReflexiveTriggerEffect").unwrap_or_else(|| panic!("{debug}"));
    let outer_prefix = debug.find("common_prefix_effects").unwrap_or_else(|| panic!("{debug}"));
    assert!(remove < reflexive, "{debug}");
    assert!(reflexive < outer_prefix, "the reflexive trigger is inside a mode of the counter choice: {debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
}
