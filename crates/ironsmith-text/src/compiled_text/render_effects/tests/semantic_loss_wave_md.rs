//! Semantic-loss fix wave (2026-10-21), package MD: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

fn md_compile(name: &str, types: Vec<CardType>, text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(types)
        .parse_text(text)
        .unwrap_or_else(|err| panic!("{name} should compile: {err:?}"))
}

fn md_debug(definition: &crate::cards::CardDefinition) -> String {
    format!("{definition:#?}")
}

// --- Kept "you may" on immediate tagged/source casts -----------------------

#[test]
fn may_cast_that_exiled_card_stays_optional() {
    // Synthesis Pod: the cast used to lower to a bare (mandatory) CastTaggedEffect.
    let definition = md_compile(
        "Synthesis Pod Probe",
        vec![CardType::Artifact],
        "{1}{U/P}, {T}, Exile a spell you control: Target opponent reveals cards from the top of their library until they reveal a card with mana value equal to 1 plus the exiled spell's mana value. Exile that card, then that player shuffles. You may cast that exiled card without paying its mana cost.",
    );
    let debug = md_debug(&definition);
    let may = debug.find("MayEffect").expect("authored may must survive lowering");
    let cast = debug.find("CastTaggedEffect").expect("tagged cast");
    assert!(may < cast, "the cast must be inside the may: {debug}");
}

#[test]
fn if_you_do_you_may_cast_this_card_stays_optional() {
    // Thunderblade Charge: "If you do, you may cast it" lowered to IfEffect[CastSourceEffect].
    let definition = md_compile(
        "Thunderblade Probe",
        vec![CardType::Sorcery],
        "Thunderblade Probe deals 3 damage to any target.\nWhenever one or more creatures you control deal combat damage to a player, if this card is in your graveyard, you may pay {2}{R}{R}{R}. If you do, you may cast it without paying its mana cost.",
    );
    let debug = md_debug(&definition);
    let cast = debug.find("CastSourceEffect").expect("source cast");
    let if_effect = debug.find("IfEffect").expect("if-you-do gate");
    let may_after_gate = debug[if_effect..cast].contains("MayEffect");
    assert!(may_after_gate, "the gated cast must be optional: {debug}");
}

#[test]
fn if_you_do_you_may_cast_the_exiled_card_stays_optional_in_text() {
    // Living Lore / Widespread Thieving shape.
    let definition = md_compile(
        "Living Lore Probe",
        vec![CardType::Creature],
        "As this creature enters, exile an instant or sorcery card from your graveyard.\nWhenever this creature deals combat damage, you may sacrifice it. If you do, you may cast the exiled card without paying its mana cost.",
    );
    let lines = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        lines.contains("If you do, you may cast"),
        "rendered cast must keep its may: {lines}"
    );
    assert!(!lines.contains("may may"), "{lines}");
}

// --- Consult stop filter keeps a serial card-type list ---------------------

#[test]
fn consult_stop_filter_keeps_every_listed_card_type() {
    let definition = md_compile(
        "Strago Probe",
        vec![CardType::Creature],
        "{2}{R}, {T}: Target opponent exiles cards from the top of their library until they exile an instant, sorcery, or creature card. You may cast that card without paying its mana cost. Activate only as a sorcery.",
    );
    let debug = md_debug(&definition);
    let consult = debug
        .find("ConsultTopOfLibraryEffect")
        .expect("consult traversal");
    let stop = &debug[consult..debug[consult..].find("stop_rule").map_or(debug.len(), |i| consult + i)];
    for card_type in ["Instant", "Sorcery", "Creature"] {
        assert!(stop.contains(card_type), "{card_type} missing from stop filter: {stop}");
    }
}

// --- "shuffle each <filter> into its owner's library" ----------------------

#[test]
fn shuffle_each_nonland_permanent_selects_every_match() {
    let definition = md_compile(
        "Teferi Probe",
        vec![CardType::Sorcery],
        "Target opponent chooses a permanent they control and returns it to its owner's hand. Then they shuffle each nonland permanent they control into its owner's library.",
    );
    let debug = md_debug(&definition);
    let shuffle = debug
        .find("ShuffleObjectsIntoLibraryEffect")
        .expect("shuffle into library");
    let tail = &debug[shuffle..];
    let target = tail.find("target:").expect("shuffle target");
    assert!(
        tail[target..].trim_start_matches("target:").trim_start().starts_with("All("),
        "each nonland permanent must be every match, not one chosen object: {}",
        &tail[..400.min(tail.len())]
    );
}

// --- Value::PlaneswalkerTypesAmong ----------------------------------------

#[test]
fn planeswalker_types_among_is_a_distinct_subtype_count() {
    let definition = md_compile(
        "Tam Probe",
        vec![CardType::Creature],
        "{W}{U}{B}{R}{G}, {T}: Proliferate X times, where X is the number of planeswalker types among planeswalkers you control.",
    );
    let debug = md_debug(&definition);
    assert!(debug.contains("PlaneswalkerTypesAmong"), "{debug}");
}

// --- not_put_into_graveyard_this_combat -----------------------------------

#[test]
fn wasnt_put_there_this_combat_reaches_the_target_filter() {
    let definition = md_compile(
        "Storrev Probe",
        vec![CardType::Creature],
        "Whenever this creature deals combat damage to a player, return to your hand target creature or planeswalker card in your graveyard that wasn't put there this combat.",
    );
    let debug = md_debug(&definition);
    assert!(
        debug.contains("not_put_into_graveyard_this_combat: true"),
        "combat exclusion dropped: {debug}"
    );
}

#[test]
fn graveyard_entries_are_scoped_to_the_current_combat() {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "History Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let before = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let during = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let before = game.move_object_by_effect(before, Zone::Graveyard).unwrap();
    // Publish the pre-combat movement before combat begins, as the game loop
    // does between steps.
    for event in game.take_pending_trigger_events() {
        game.turn_store.turn_history.record_event(&event, None, None);
    }
    let combat = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::BeginningOfCombatEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    game.turn_store.turn_history.record_event(&combat, None, None);
    let during = game.move_object_by_effect(during, Zone::Graveyard).unwrap();
    let history = &game.turn_store.turn_history;
    assert!(!history.graveyard_incarnation_entered_this_combat(before));
    assert!(history.graveyard_incarnation_entered_this_combat(during));
}

// --- returning_player ------------------------------------------------------

#[test]
fn creatures_controller_returns_the_aura() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Plague Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature\nWhen enchanted creature leaves the battlefield, that creature's controller returns this Aura from its owner's graveyard to the battlefield.",
        )
        .unwrap();
    let debug = md_debug(&definition);
    let returning = debug.find("returning_player: Some(").expect("actor must control the return");
    assert!(debug[returning..].trim_start_matches("returning_player: Some(").trim_start().starts_with("ControllerOf"), "{debug}");
}

// --- ExcessDamageDealtToCreature --------------------------------------------

#[test]
fn excess_damage_to_a_creature_gate_is_creature_specific() {
    let definition = md_compile(
        "Vikya Probe",
        vec![CardType::Creature],
        "{4}{R}, {T}, Discard a card: This creature deals damage equal to its power to any target. If excess damage was dealt to a creature this way, draw a card.",
    );
    let debug = md_debug(&definition);
    assert!(debug.contains("ExcessDamageDealtToCreature"), "{debug}");
}

#[test]
fn plain_excess_damage_gate_is_unchanged() {
    let definition = md_compile(
        "Excess Probe",
        vec![CardType::Instant],
        "This spell deals 4 damage to target creature. If excess damage was dealt this way, draw a card.",
    );
    let debug = md_debug(&definition);
    assert!(!debug.contains("ExcessDamageDealtToCreature"), "{debug}");
    assert!(debug.contains("ExcessDamageDealt"), "{debug}");
}

// --- "all <filter> revealed this way ... rest into your graveyard" ---------

#[test]
fn chosen_type_reveal_keeps_the_rest_to_graveyard() {
    let definition = md_compile(
        "Vigean Probe",
        vec![CardType::Instant],
        "Choose a card type, then reveal the top four cards of your library. Put all cards of the chosen type revealed this way into your hand and the rest into your graveyard.",
    );
    let debug = md_debug(&definition);
    assert!(debug.contains("zone: Hand"), "{debug}");
    assert!(
        debug.contains("zone: Graveyard"),
        "the rest of the revealed cards must go to the graveyard: {debug}"
    );
}

// --- "put a ... card from among them onto the battlefield" ------------------

#[test]
fn from_among_them_does_not_choose_a_battlefield_permanent() {
    let definition = md_compile(
        "Winter Probe",
        vec![CardType::Creature],
        "At the beginning of your end step, you may exile any number of cards from your graveyard. If you do, put a permanent card from among them onto the battlefield with a finality counter on it.",
    );
    let debug = md_debug(&definition);
    let moved = debug.find("MoveToZoneEffect").expect("put onto battlefield");
    let target = &debug[moved..debug[moved..].find("zone: Battlefield,\n").map_or(debug.len(), |i| moved + i)];
    assert!(target.contains("tagged_constraints: [\n"), "choice must be among the exiled cards: {target}");
    assert!(!target.contains("Battlefield"), "chooser filter must not be battlefield-zoned: {target}");
}

// --- "exiled under your opponents' control this turn" ----------------------

#[test]
fn exiled_this_turn_count_reads_turn_history() {
    let definition = md_compile(
        "Vren Probe",
        vec![CardType::Creature],
        "At the beginning of each end step, create X 1/1 black Rat creature tokens, where X is the number of creatures that were exiled under your opponents' control this turn.",
    );
    let debug = md_debug(&definition);
    let moved = debug.find("MovedZones").expect("turn-history count");
    let tail = &debug[moved..];
    assert!(tail.contains("Exile"), "{tail}");
    assert!(tail.contains("Opponent"), "{tail}");
}
