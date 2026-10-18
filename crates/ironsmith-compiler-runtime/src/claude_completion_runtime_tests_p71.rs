//! Claude package 71 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p71_compile(name: &str, text: &str) -> String {
    let definition = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
    format!("{definition:?}")
}

/// "as long as an instant card and a sorcery card are in your graveyard":
/// one reading (two independent graveyard-card requirements), no ambiguity.
#[test]
fn p71_bloodfire_enforcers_needs_instant_and_sorcery_in_graveyard() {
    let debug = p71_compile(
        "Bloodfire Enforcers",
        "Mana cost: {3}{R}\nType: Creature — Human Monk\nPower/Toughness: 5/2\nThis creature has first strike and trample as long as an instant card and a sorcery card are in your graveyard.",
    );
    assert!(debug.contains("Instant"), "{debug}");
    assert!(debug.contains("Sorcery"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
}

/// "Enchanted land is the chosen type." is the chosen basic land type, not
/// Conspiracy's chosen creature type.
#[test]
fn p71_enchanted_land_is_chosen_basic_land_type() {
    for name in ["Convincing Mirage", "Phantasmal Terrain"] {
        let debug = p71_compile(
            name,
            "Mana cost: {1}{U}\nType: Enchantment — Aura\nEnchant land\nAs this Aura enters, choose a basic land type.\nEnchanted land is the chosen type.",
        );
        assert!(debug.contains("EnchantedLandIsChosenType"), "{debug}");
        assert!(!debug.contains("AddChosenCreatureType"), "{debug}");
    }
}

/// The copular animation bundle keeps both the 0/0 creature type change and
/// the annihilator grant.
#[test]
fn p71_idol_of_false_gods_animates_with_annihilator() {
    let debug = p71_compile(
        "Idol of False Gods",
        "Mana cost: {2}\nType: Kindred Artifact — Eldrazi\n{1}{C}, {T}: Create a 0/1 colorless Eldrazi Spawn creature token with \"Sacrifice this token: Add {C}.\"\nWhenever another Eldrazi you control dies, put a +1/+1 counter on this artifact.\nAs long as this artifact has eight or more +1/+1 counters on it, it's a 0/0 creature in addition to its other types and it has annihilator 2.",
    );
    assert!(debug.to_lowercase().contains("annihilator"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
}

/// "target opponent destroys four lands they control": the opponent is a
/// target and chooses among their own lands.
#[test]
fn p71_burning_of_xinye_target_opponent_chooses_lands() {
    let debug = p71_compile(
        "Burning of Xinye",
        "Mana cost: {4}{R}{R}\nType: Sorcery\nYou destroy four lands you control, then target opponent destroys four lands they control. Then Burning of Xinye deals 4 damage to each creature.",
    );
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("Target(Opponent)"), "{debug}");
    assert!(!debug.contains("controller: Some(IteratedPlayer)"), "{debug}");
    assert!(debug.contains("DealDamageEachEffect"), "{debug}");
}

/// "they draw a card": the attacking player, not an unbound loop player.
#[test]
fn p71_firemane_commando_other_attacker_draws() {
    let debug = p71_compile(
        "Firemane Commando",
        "Mana cost: {3}{W}\nType: Creature — Angel Soldier\nPower/Toughness: 4/3\nFlying\nWhenever you attack with two or more creatures, draw a card.\nWhenever another player attacks with two or more creatures, they draw a card if none of those creatures attacked you.",
    );
    assert!(debug.contains("AliasedControllerOf"), "{debug}");
    assert!(!debug.contains("player: IteratedPlayer"), "{debug}");
}

/// The flying branch deals twice the power-based amount to the same creature.
#[test]
fn p71_cut_propulsion_doubles_self_damage_with_flying() {
    let debug = p71_compile(
        "Cut Propulsion",
        "Mana cost: {2}{R}\nType: Instant\nTarget creature deals damage to itself equal to its power. If that creature has flying, it deals twice that much damage to itself instead.",
    );
    assert!(debug.contains("Scaled("), "{debug}");
    assert!(debug.contains("Flying"), "{debug}");
    assert!(!debug.contains("EventValue(Amount)"), "{debug}");
}

/// "sacrifice it unless you pay its mana cost reduced by {2}": the payment
/// reads the creature put onto the battlefield, reduced by two generic.
#[test]
fn p71_flash_sacrifices_unless_reduced_mana_cost_paid() {
    let debug = p71_compile(
        "Flash",
        "Mana cost: {1}{U}\nType: Instant\nYou may put a creature card from your hand onto the battlefield. If you do, sacrifice it unless you pay its mana cost reduced by {2}.",
    );
    assert!(debug.contains("UnlessPaysEffect"), "{debug}");
    assert!(debug.contains("generic_reduction: 2"), "{debug}");
    assert!(debug.contains("mana_cost_of: Some("), "{debug}");
}

/// Only the top card of the graveyard is inspected and moved.
#[test]
fn p71_guiding_spirit_moves_only_top_creature_card() {
    let debug = p71_compile(
        "Guiding Spirit",
        "Mana cost: {1}{W}{U}\nType: Creature — Angel Spirit\nPower/Toughness: 1/2\nFlying\n{T}: If the top card of target player's graveyard is a creature card, put that card on top of that player's library.",
    );
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("Library"), "{debug}");
}

/// "put that many +1/+1 counters": the prevented amount of the event.
#[test]
fn p71_selfless_squire_counts_prevented_damage() {
    let debug = p71_compile(
        "Selfless Squire",
        "Mana cost: {3}{W}\nType: Creature — Human Soldier\nPower/Toughness: 1/1\nFlash\nWhen this creature enters, prevent all damage that would be dealt to you this turn.\nWhenever damage that would be dealt to you is prevented, put that many +1/+1 counters on this creature.",
    );
    assert!(debug.contains("EventValue(Amount)"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
}

/// The granted mana-ability trigger counts the mana produced by the event.
#[test]
fn p71_tyvar_mana_ability_trigger_uses_event_amount() {
    let debug = p71_compile(
        "Tyvar the Bellicose",
        "Mana cost: {2}{B}{G}\nType: Legendary Creature — Elf Warrior\nPower/Toughness: 5/4\nWhenever one or more Elves you control attack, they gain deathtouch until end of turn.\nEach creature you control has \"Whenever a mana ability of this creature resolves, put a number of +1/+1 counters on it equal to the amount of mana this creature produced. This ability triggers only once each turn.\"",
    );
    assert!(debug.contains("EventValue(Amount)"), "{debug}");
}

/// The searched card's mana value is bounded by the sacrifice's result.
#[test]
fn p71_hunger_tide_rises_search_bound_by_sacrificed_count() {
    let debug = p71_compile(
        "The Hunger Tide Rises",
        "Mana cost: {2}{G}\nType: Enchantment — Saga\n(As this Saga enters and after your draw step, add a lore counter. Sacrifice after IV.)\nI, II, III — Create a 1/1 black and green Insect creature token.\nIV — Sacrifice any number of creatures. Search your library and/or graveyard for a creature card with mana value less than or equal to the number of creatures sacrificed this way and put it onto the battlefield. If you search your library this way, shuffle.",
    );
    assert!(debug.contains("PriorEffectMetric"), "{debug}");
    assert!(!debug.contains("PendingPriorEffectMetric"), "{debug}");
}

/// X is the greatest mana value among the exiled cards (the exile's result).
#[test]
fn p71_kenriths_funeral_draws_greatest_exiled_mana_value() {
    let debug = p71_compile(
        "The Kenriths' Royal Funeral",
        "Mana cost: {2}{W}{B}\nType: Legendary Enchantment\nWhen The Kenriths' Royal Funeral enters, exile up to two target legendary creature cards from your graveyard. You draw X cards and you lose X life, where X is the greatest mana value among cards exiled this way.\nLegendary spells you cast cost {1} less to cast for each card exiled with The Kenriths' Royal Funeral.",
    );
    assert!(debug.contains("GreatestManaValue"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    assert!(debug.contains("LoseLifeEffect"), "{debug}");
    assert!(!debug.contains("PendingPriorEffectMetric"), "{debug}");
}

/// The repeated mill is the producer the "were put into that graveyard this
/// way" result gate reads.
#[test]
fn p71_helm_of_obedience_gate_reads_the_repeated_mill() {
    let debug = p71_compile(
        "Helm of Obedience",
        "Mana cost: {4}\nType: Artifact\n{X}, {T}: Target opponent mills a card, then repeats this process until a creature card or X cards have been put into their graveyard this way, whichever comes first. If one or more creature cards were put into that graveyard this way, sacrifice this artifact and put one of them onto the battlefield under your control. X can't be 0.",
    );
    assert!(debug.contains("IfEffect"), "{debug}");
    assert!(debug.contains("Mill"), "{debug}");
}

/// "up to that many": the life gained this turn named by the intervening-if.
#[test]
fn p71_lathiel_distributes_up_to_life_gained_this_turn() {
    let debug = p71_compile(
        "Lathiel, the Bounteous Dawn",
        "Mana cost: {2}{G}{W}\nType: Legendary Creature — Unicorn\nPower/Toughness: 2/2\nLifelink\nAt the beginning of each end step, if you gained life this turn, distribute up to that many +1/+1 counters among any number of other target creatures.",
    );
    assert!(debug.contains("LifeGainedThisTurn"), "{debug}");
    assert!(!debug.contains("EventValue(Amount)"), "{debug}");
}
