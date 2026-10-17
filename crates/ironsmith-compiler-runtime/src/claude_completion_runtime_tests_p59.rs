//! Claude package 59 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p59_compile(name: &str, text: &str) -> String {
    let definition = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
    format!("{definition:?}")
}

#[test]
fn gideon_battle_forged_target_attacks_him_during_its_controllers_next_turn() {
    let debug = p59_compile(
        "Gideon, Battle-Forged",
        "Type: Legendary Planeswalker — Gideon\nLoyalty: 3\n+2: Up to one target creature an opponent controls attacks Gideon during its controller's next turn if able.\n+1: Until your next turn, target creature gains indestructible. Untap that creature.\n0: Until end of turn, Gideon becomes a 4/4 Human Soldier creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn.",
    );
    assert!(debug.contains("MustAttackPermanents"), "{debug}");
    assert!(
        debug.contains("NextTurn(ControllerOf(Tagged("),
        "the requirement starts with the target's controller's next turn: {debug}"
    );
}

#[test]
fn kosei_serial_condition_requires_enchanted_equipped_and_a_counter() {
    let debug = p59_compile(
        "Kosei, Penitent Warlord",
        "Mana cost: {1}{G}{G}\nType: Legendary Creature — Ogre Samurai\nPower/Toughness: 0/5\nAs long as Kosei is enchanted, equipped, and has a counter on it, Kosei has \"Whenever Kosei deals combat damage to an opponent, you draw that many cards and Kosei deals that much damage to each other opponent.\"",
    );
    // Three arms of one conjunctive source-state condition.
    assert!(debug.contains("And("), "{debug}");
    assert!(debug.contains("SourceIsEnchanted"), "{debug}");
    assert!(debug.contains("SourceIsEquipped"), "{debug}");
    assert!(debug.contains("CountersOn(Source"), "{debug}");
}

#[test]
fn kozilek_each_target_player_manifests_two_cards_from_hand_and_you_draw_per_card() {
    let debug = p59_compile(
        "Kozilek, the Broken Reality",
        "Mana cost: {9}\nType: Legendary Creature — Eldrazi\nPower/Toughness: 9/9\nWhen you cast this spell, up to two target players each manifest two cards from their hands. For each card manifested this way, you draw a card. (To manifest a card, put it onto the battlefield face down as a 2/2 creature. Turn it face up any time for its mana cost if it's a creature card.)\nOther colorless creatures you control get +3/+2.",
    );
    assert!(debug.contains("ManifestObjectsEffect"), "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "each player chooses from their own hand: {debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
    assert!(debug.contains("PutOntoBattlefield"), "the draw counts the manifested cards: {debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn liars_pendulum_opponent_guesses_whether_the_named_card_is_in_your_hand() {
    let debug = p59_compile(
        "Liar's Pendulum",
        "Mana cost: {1}\nType: Artifact\n{2}, {T}: Choose a card name. Target opponent guesses whether a card with that name is in your hand. You may reveal your hand. If you do and your opponent guessed wrong, draw a card.",
    );
    assert!(debug.contains("ChooseCardNameEffect"), "{debug}");
    assert!(debug.contains("It's in their hand"), "the guess is a two-answer choice: {debug}");
    assert!(debug.contains("SameNameAsTagged"), "the guess is checked against the chosen name: {debug}");
    // One optional reveal and one draw per answer.
    assert_eq!(debug.matches("DrawCardsEffect").count(), 2, "{debug}");
}

#[test]
fn knowledge_pool_casts_another_pooled_card_not_the_one_just_exiled() {
    let debug = p59_compile(
        "Knowledge Pool",
        "Mana cost: {6}\nType: Artifact\nImprint — When this artifact enters, each player exiles the top three cards of their library.\nWhenever a player casts a spell from their hand, that player exiles it. If the player does, they may cast a spell from among other cards exiled with this artifact without paying its mana cost.",
    );
    assert!(debug.contains("pool_exiled"), "the exile's outcome is tagged: {debug}");
    assert!(debug.contains("IsNotTaggedObject"), "the just-exiled card is excluded: {debug}");
    assert!(debug.contains("__source_exiled__"), "{debug}");
    assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    assert!(debug.contains("chooser: IteratedPlayer"), "the casting player chooses: {debug}");
}

#[test]
fn lava_burst_installs_its_unpreventable_unredirectable_creature_damage_rule_first() {
    let debug = p59_compile(
        "Lava Burst",
        "Mana cost: {X}{R}\nType: Sorcery\nLava Burst deals X damage to any target. If Lava Burst would deal damage to a creature, that damage can't be prevented or dealt instead to another permanent or player.",
    );
    let rule = debug
        .find("PreventDamageFromTo")
        .unwrap_or_else(|| panic!("source+recipient rule: {debug}"));
    assert!(debug.contains("or_redirected: true"), "{debug}");
    let damage = debug.find("DealDamageEffect").expect("the damage instruction");
    assert!(rule < damage, "the rule applies before the damage is dealt: {debug}");
}

#[test]
fn land_equilibrium_qualifying_opponents_land_enters_then_they_sacrifice_a_land() {
    let debug = p59_compile(
        "Land Equilibrium",
        "Mana cost: {2}{U}{U}\nType: Enchantment\nIf an opponent who controls at least as many lands as you do would put a land onto the battlefield, that player instead puts that land onto the battlefield then sacrifices a land of their choice.",
    );
    assert!(debug.contains("EventReplacementWithEffects"), "{debug}");
    assert!(debug.contains("ZoneChange"), "{debug}");
    assert!(debug.contains("GreaterThanOrEqual"), "the land-count qualifier: {debug}");
    assert!(debug.contains("Sacrifice"), "{debug}");
}
