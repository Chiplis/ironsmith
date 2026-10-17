//! Claude package 33 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p33_compile(name: &str, text: &str) -> String {
    let definition = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
    format!("{definition:?}")
}

#[test]
fn goblin_festival_one_of_your_opponents_is_an_opponent_choice() {
    let debug = p33_compile(
        "Goblin Festival",
        "Mana cost: {1}{R}\nType: Enchantment\n{2}: This enchantment deals 1 damage to any target. Flip a coin. If you lose the flip, choose one of your opponents. That player gains control of this enchantment.",
    );
    assert!(debug.contains("Opponent"), "{debug}");
}

#[test]
fn grand_master_of_flowers_without_three_keywords_excludes_each() {
    let debug = p33_compile(
        "Grand Master of Flowers",
        "Mana cost: {2}{W}{W}\nType: Legendary Planeswalker — Bahamut\nLoyalty: 3\nAs long as Grand Master of Flowers has seven or more loyalty counters on him, he's a 7/7 Dragon God creature with flying and indestructible.\n+1: Target creature without first strike, double strike, or vigilance can't attack or block until your next turn.\n+1: Search your library and/or graveyard for a card named Monk of the Open Hand, reveal it, and put it into your hand. If you search your library this way, shuffle.",
    );
    assert!(
        debug.contains("excluded_static_abilities: [FirstStrike, DoubleStrike, Vigilance]"),
        "{debug}"
    );
}

#[test]
fn goblin_plate_mail_attaches_to_the_amassed_army() {
    let debug = p33_compile(
        "Goblin Plate Mail",
        "Mana cost: {1}{B/R}\nType: Artifact — Equipment\nWhen this Equipment enters, amass Goblins 1, then attach this Equipment to the amassed Army. (To amass Goblins 1, put a +1/+1 counter on an Army you control. It's also a Goblin. If you don't control an Army, create a 0/0 black Goblin Army creature token first.)\nEquipped creature gets +1/+0 and has menace.\nEquip {4}",
    );
    assert!(debug.contains("amassed"), "the attach names the amassed tag: {debug}");
}

#[test]
fn gurzigost_grants_the_optional_unblocked_assignment_this_turn() {
    let debug = p33_compile(
        "Gurzigost",
        "Mana cost: {3}{G}{G}\nType: Creature — Beast\nPower/Toughness: 6/8\nAt the beginning of your upkeep, sacrifice this creature unless you put two cards from your graveyard on the bottom of your library.\n{G}{G}, Discard a card: You may have this creature assign its combat damage this turn as though it weren't blocked.",
    );
    assert!(debug.contains("Unblocked"), "{debug}");
}

#[test]
fn hall_of_echoes_exempts_your_permanents_from_the_legend_rule_this_turn() {
    let debug = p33_compile(
        "Hall of Echoes",
        "Type: Land\n{T}: Add {C}.\n{5}: This land becomes a copy of target creature you control until end of turn. The \"legend rule\" doesn't apply to permanents you control this turn.",
    );
    assert!(debug.contains("LegendRuleDoesntApplyToController"), "{debug}");
}

#[test]
fn guardian_archon_protects_you_and_one_target_permanent_from_the_chosen_player() {
    let debug = p33_compile(
        "Guardian Archon",
        "Mana cost: {4}{W}{W}\nType: Creature — Archon\nPower/Toughness: 5/5\nFlying\nAs this creature enters, secretly choose an opponent.\nReveal the player you chose: You and target permanent you control each gain protection from the chosen player until end of turn. Activate only once.",
    );
    assert!(debug.contains("BeTargetedPlayerFrom"), "the player half: {debug}");
    assert!(debug.contains("ChosenPlayer"), "{debug}");
}

#[test]
fn gideon_jura_requires_the_target_opponents_creatures_to_attack_him_next_turn() {
    let debug = p33_compile(
        "Gideon Jura",
        "Mana cost: {3}{W}{W}\nType: Legendary Planeswalker — Gideon\nLoyalty: 6\n+2: During target opponent's next turn, creatures that player controls attack Gideon Jura if able.\n−2: Destroy target tapped creature.\n0: Until end of turn, Gideon Jura becomes a 6/6 Human Soldier creature that's still a planeswalker. Prevent all damage that would be dealt to him this turn.",
    );
    assert!(debug.contains("MustAttackPermanents"), "{debug}");
    assert!(debug.contains("NextTurn"), "the requirement starts next turn: {debug}");
}

#[test]
fn gollum_opponent_guesses_the_chosen_kind_of_the_top_card() {
    let debug = p33_compile(
        "Gollum, Scheming Guide",
        "Mana cost: {1}{B}\nType: Legendary Creature — Halfling Horror\nPower/Toughness: 2/1\nWhenever Gollum attacks, look at the top two cards of your library, put them back in any order, then choose land or nonland. An opponent guesses whether the top card of your library is the chosen kind. Reveal that card. If they guessed right, remove Gollum from combat. Otherwise, you draw a card and Gollum can't be blocked this turn.",
    );
    // Two kinds x two answers, each branching on the revealed top card.
    assert_eq!(debug.matches("TopCardOfYourLibraryMatches").count(), 4, "{debug}");
}

#[test]
fn guiding_spirit_moves_only_a_creature_card_on_top_of_the_graveyard() {
    let debug = p33_compile(
        "Guiding Spirit",
        "Mana cost: {1}{W}{U}\nType: Creature — Angel Spirit\nPower/Toughness: 2/2\nFlying\n{T}: If the top card of target player's graveyard is a creature card, put that card on top of that player's library.",
    );
    assert!(debug.contains("top_only: true"), "the top card is looked up unfiltered: {debug}");
    assert!(debug.contains("graveyard_top"), "{debug}");
}

#[test]
fn goldberry_moves_one_counter_of_each_absent_kind_and_one_or_more_counters() {
    let debug = p33_compile(
        "Goldberry, River-Daughter",
        "Mana cost: {1}{U}\nType: Legendary Creature — Nymph\nPower/Toughness: 1/3\n{T}: Move a counter of each kind not on Goldberry from another target permanent you control onto Goldberry.\n{U}, {T}: Move one or more counters from Goldberry onto another target permanent you control. If you do, draw a card.",
    );
    assert!(debug.contains("OneOfEachKindAbsentFromRecipient"), "{debug}");
    assert!(debug.contains("AnyNumberOfAnyKind"), "{debug}");
}

#[test]
fn grinning_totem_returns_the_unplayed_card_to_the_graveyard_next_upkeep() {
    let debug = p33_compile(
        "Grinning Totem",
        "Mana cost: {4}\nType: Artifact\n{2}, {T}, Sacrifice this artifact: Search target opponent's library for a card and exile it. Then that player shuffles. Until the beginning of your next upkeep, you may play that card. At the beginning of your next upkeep, if you haven't played it, put it into its owner's graveyard.",
    );
    assert!(debug.contains("TaggedWasCast") || debug.contains("WasCast"), "{debug}");
}

#[test]
fn hans_eriksson_puts_the_creature_attacking_the_defending_player() {
    let debug = p33_compile(
        "Hans Eriksson",
        "Mana cost: {2}{R}{G}\nType: Legendary Creature — Human Scout\nPower/Toughness: 3/3\nWhenever Hans Eriksson attacks, reveal the top card of your library. If it's a creature card, put it onto the battlefield tapped and attacking defending player or a planeswalker they control. Otherwise, put that card into your hand. When you put a creature card onto the battlefield this way, it fights Hans Eriksson.",
    );
    assert!(debug.contains("Fight"), "{debug}");
}

#[test]
fn goliath_hatchery_draws_the_chosen_creatures_total_toxic_value() {
    let debug = p33_compile(
        "Goliath Hatchery",
        "Mana cost: {4}{G}{G}\nType: Enchantment\nWhen this enchantment enters, create two 3/3 green Phyrexian Beast creature tokens with toxic 1. (Players dealt combat damage by them also get a poison counter.)\nCorrupted — At the beginning of your upkeep, if an opponent has three or more poison counters, choose a creature you control, then draw cards equal to its total toxic value.",
    );
    assert!(debug.contains("TotalToxicValueOf"), "{debug}");
}

#[test]
fn gorm_must_be_blocked_and_by_two_or_more_creatures_if_able() {
    let debug = p33_compile(
        "Gorm the Great",
        "Mana cost: {3}{G}\nType: Legendary Creature — Giant Warrior\nPower/Toughness: 2/7\nPartner with Virtus the Veiled (When this creature enters, target player may put Virtus into their hand from their library, then shuffle.)\nVigilance\nGorm must be blocked if able, and Gorm must be blocked by two or more creatures if able.",
    );
    assert!(debug.contains("MustBeBlockedByAtLeast"), "{debug}");
    assert!(debug.contains("minimum: 2"), "{debug}");
    assert!(debug.contains("MustBeBlocked("), "{debug}");
}
