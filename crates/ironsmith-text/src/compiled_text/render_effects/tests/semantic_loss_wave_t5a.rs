//! Semantic-loss fix wave (2026-10-21), package T5A: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

fn compile(name: &str, card_types: Vec<CardType>, text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} failed to compile: {error:?}"))
}

fn abilities_debug(definition: &crate::cards::CardDefinition) -> String {
    format!("{:?}", definition.abilities)
}

fn spell_debug(definition: &crate::cards::CardDefinition) -> String {
    format!("{:?}", definition.spell_effect)
}

// --- Turn-window trigger restriction (Brotherhood Scribe, Dovin's Acuity) ---

#[test]
fn energy_trigger_keeps_its_during_your_turn_restriction() {
    let definition = compile(
        "Brotherhood Scribe",
        vec![CardType::Artifact, CardType::Creature],
        "Whenever you get one or more {E} during your turn, creatures you control get +1/+1 until end of turn.",
    );
    let debug = abilities_debug(&definition);
    assert!(debug.contains("ConditionQualifiedTrigger"), "{debug}");
    assert!(debug.contains("PlayerGetsCountersTrigger"), "{debug}");
    assert!(debug.contains("condition: YourTurn"), "{debug}");
}

#[test]
fn cast_trigger_keeps_its_during_your_main_phase_restriction() {
    let definition = compile(
        "Dovin's Acuity",
        vec![CardType::Enchantment],
        "Whenever you cast an instant spell during your main phase, you may return this enchantment to its owner's hand.",
    );
    let debug = abilities_debug(&definition);
    assert!(debug.contains("ConditionQualifiedTrigger"), "{debug}");
    assert!(debug.contains("during your main phase"), "{debug}");
}

#[test]
fn unrestricted_energy_trigger_is_not_turn_qualified() {
    let definition = compile(
        "Plain Energy",
        vec![CardType::Creature],
        "Whenever you get one or more {E}, creatures you control get +1/+1 until end of turn.",
    );
    let debug = abilities_debug(&definition);
    assert!(!debug.contains("ConditionQualifiedTrigger"), "{debug}");
}

// --- Champions of the Shoal: enters OR becomes tapped ---

#[test]
fn enters_or_becomes_tapped_is_two_events_not_enters_tapped() {
    let definition = compile(
        "Champions of the Shoal",
        vec![CardType::Creature],
        "Whenever this creature enters or becomes tapped, tap up to one target creature and put a stun counter on it.",
    );
    let debug = abilities_debug(&definition);
    assert!(!debug.contains("EntersBattlefieldTappedTrigger"), "{debug}");
    assert!(debug.contains("OrTrigger"), "{debug}");
    assert!(debug.contains("BecomesTapped"), "{debug}");
}

// --- Harnesser of Storms: optional exile ---

#[test]
fn optional_exile_top_then_play_keeps_the_may() {
    let definition = compile(
        "Harnesser of Storms",
        vec![CardType::Creature],
        "Whenever you cast a noncreature or Otter spell, you may exile the top card of your library. Until end of turn, you may play that card. This ability triggers only once each turn.",
    );
    let debug = abilities_debug(&definition);
    let may = debug.find("MayEffect").unwrap_or_else(|| panic!("no may: {debug}"));
    let exile = debug
        .find("ExileTopOfLibraryEffect")
        .unwrap_or_else(|| panic!("no exile: {debug}"));
    assert!(may < exile, "exile must be nested in the may: {debug}");
}

#[test]
fn mandatory_exile_top_then_play_stays_mandatory() {
    let definition = compile(
        "Mandatory Impulse",
        vec![CardType::Creature],
        "When this creature enters, exile the top card of your library. You may play that card this turn.",
    );
    let debug = abilities_debug(&definition);
    assert!(!debug.contains("MayEffect"), "{debug}");
    assert!(debug.contains("ExileTopOfLibraryEffect"), "{debug}");
}

// --- Renowned object-filter adjective (Aragorn, Hornburg Hero) ---

#[test]
fn renowned_creature_trigger_subject_requires_renown() {
    let definition = compile(
        "Aragorn, Hornburg Hero",
        vec![CardType::Creature],
        "Whenever a renowned creature you control deals combat damage to a player, double the number of +1/+1 counters on it.",
    );
    let debug = abilities_debug(&definition);
    assert!(debug.contains("DealsCombatDamageToPlayerTrigger"), "{debug}");
    assert!(debug.contains("renowned: true"), "{debug}");
}

// --- Curse of Silence: only the enchanted player's spells are taxed ---

#[test]
fn spell_tax_is_limited_to_the_enchanted_players_spells() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Curse of Silence")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Curse])
        .parse_text(
            "Enchant player\nAs this Aura enters, choose a card name.\nSpells with the chosen name enchanted player casts cost {2} more to cast.",
        )
        .unwrap();
    let debug = abilities_debug(&definition);
    let increase = debug
        .find("CostIncrease(")
        .unwrap_or_else(|| panic!("no cost increase: {debug}"));
    let payload = &debug[increase..];
    assert!(payload.contains("cast_by: Some(TaggedPlayer("), "{payload}");
    assert!(payload.contains("enchanted"), "{payload}");
}

// --- Flames of the Blood Hand: planeswalker controller can't gain life ---

#[test]
fn no_life_gain_names_the_target_player_or_planeswalker_controller() {
    let definition = compile(
        "Flames of the Blood Hand",
        vec![CardType::Instant],
        "Flames of the Blood Hand deals 4 damage to target player or planeswalker. The damage can't be prevented. If that player or that planeswalker's controller would gain life this turn, that player gains no life instead.",
    );
    let debug = spell_debug(&definition);
    assert!(
        debug.contains("GainLife(TargetPlayerOrControllerOfTarget)"),
        "{debug}"
    );
    assert!(!debug.contains("GainLife(DamagedPlayer)"), "{debug}");
}

// --- Goblin Ski Patrol: delayed self-sacrifice has a resolvable actor ---

#[test]
fn its_controller_sacrifices_it_does_not_read_a_missing_target() {
    let definition = compile(
        "Goblin Ski Patrol",
        vec![CardType::Creature],
        "{1}{R}: This creature gets +2/+0 and gains flying. Its controller sacrifices it at the beginning of the next end step. Activate only once.",
    );
    let debug = abilities_debug(&definition);
    let sacrifice = debug
        .find("SacrificeTargetEffect")
        .unwrap_or_else(|| panic!("no sacrifice: {debug}"));
    let tail = &debug[sacrifice..debug.len().min(sacrifice + 200)];
    assert!(tail.contains("target: Source"), "{tail}");
    assert!(!tail.contains("ControllerOf(Target)"), "{tail}");
}

// --- Gyruda: choose among the cards every player milled ---

#[test]
fn each_player_mill_selection_chooses_among_milled_cards() {
    let definition = compile(
        "Gyruda, Doom of Depths",
        vec![CardType::Creature],
        "When this creature enters, each player mills four cards. Put a creature card with an even mana value from among the milled cards onto the battlefield under your control.",
    );
    let debug = abilities_debug(&definition);
    let choose = debug
        .find("ChooseObjectsEffect")
        .unwrap_or_else(|| panic!("no milled-card choice: {debug}"));
    let choose_tail = &debug[choose..];
    assert!(choose_tail.contains("zone: Some(Graveyard)"), "{choose_tail}");
    assert!(choose_tail.contains("milled"), "{choose_tail}");
    assert!(choose_tail.contains("chooser: You"), "{choose_tail}");
}

#[test]
fn each_player_mill_selection_never_takes_a_battlefield_permanent() {
    let definition = compile(
        "Gyruda, Doom of Depths",
        vec![CardType::Creature],
        "When this creature enters, each player mills four cards. Put a creature card with an even mana value from among the milled cards onto the battlefield under your control.",
    );
    let debug = abilities_debug(&definition);
    let move_effect = debug
        .find("MoveToZoneEffect")
        .unwrap_or_else(|| panic!("no move: {debug}"));
    // The moved object is the chosen milled card, not an object filter over
    // the battlefield.
    let tail = &debug[move_effect..debug.len().min(move_effect + 120)];
    assert!(!tail.contains("zone: Some(Battlefield)"), "{tail}");
    assert!(debug.contains("battlefield_controller: You"), "{debug}");
}
