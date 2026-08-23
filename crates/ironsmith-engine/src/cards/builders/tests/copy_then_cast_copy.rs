#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

const COPY_CAST_CARDS: [&str; 6] = [
    "Reenact the Crime",
    "Roving Actuator",
    "Arcane Proxy",
    "Flawless Forgery",
    "Soundwave, Sonic Spy",
    "Shiko, Paragon of the Way",
];

#[test]
fn copy_then_cast_copy_cards_keep_the_copy_action_and_clear_the_floor() {
    for name in COPY_CAST_CARDS {
        let oracle = oracle_text_by_name()
            .get(name)
            .unwrap_or_else(|| panic!("missing oracle text for {name}"));
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("CastTaggedEffect") && debug.contains("as_copy: true"),
            "{name} must retain a typed copy-cast permission: {debug}"
        );

        let compiled = unprocessed_compiled_lines(&definition);
        let rendered = compiled.join("\n");
        assert!(
            rendered.contains("Copy it") || rendered.contains("Copy that card"),
            "{name} must explicitly render the AST-proven copy action: {rendered}"
        );
        assert!(
            rendered.contains("You may cast the copy without paying its mana cost"),
            "{name} must render the free-cast permission for that copy: {rendered}"
        );

        let (_, _, similarity, _, mismatch) =
            crate::semantic_compare::compare_card_semantics_scored(
                name,
                oracle,
                &compiled,
                crate::semantic_compare::report_embedding_config(),
            );
        assert!(
            similarity >= 0.99 && !mismatch,
            "{name} must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
        );
    }
}

#[test]
fn narset_graveyard_card_copy_uses_the_shared_copy_cast_program() {
    let name = "Narset, Enlightened Exile";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("CastTaggedEffect") && debug.contains("as_copy: true"),
        "Narset must cast a copy of the selected graveyard card: {debug}"
    );
    assert!(
        !debug.contains("CopySpellEffect"),
        "a graveyard card is not a spell on the stack and must not lower through CopySpellEffect: {debug}"
    );

    let compiled = unprocessed_compiled_lines(&definition);
    assert!(
        compiled.iter().any(|line| line
            == "Whenever Narset attacks, exile target noncreature, nonland card with mana value less than Narset's power from a graveyard and copy it. You may cast the copy without paying its mana cost."),
        "Narset's shared-provenance exile/copy/cast sequence should render atomically: {compiled:?}"
    );

    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "Narset must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}

#[test]
fn psionic_ritual_keeps_nonmana_replicate_and_graveyard_copy_cast_semantics() {
    let name = "Psionic Ritual";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("CastTaggedEffect") && debug.contains("as_copy: true"),
        "the exiled graveyard card must be cast as a copy: {debug}"
    );
    assert!(
        !debug.contains("CopySpellEffect"),
        "a graveyard card is not a spell on the stack: {debug}"
    );

    let compiled = unprocessed_compiled_lines(&definition);
    assert_eq!(compiled.join("\n"), oracle.as_str());
    assert_eq!(
        compiled.first().map(String::as_str),
        Some("Replicate—Tap an untapped Horror you control.")
    );
}

#[test]
fn renegade_bull_attack_keeps_the_graveyard_card_copy_cast_program() {
    let name = "Renegade Bull";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("CastTaggedEffect") && debug.contains("as_copy: true"),
        "the attack trigger must cast a copy of the selected graveyard card: {debug}"
    );
    assert!(
        !debug.contains("CopySpellEffect"),
        "the selected graveyard card is not a stack spell: {debug}"
    );

    let compiled = unprocessed_compiled_lines(&definition);
    assert_eq!(compiled.join("\n"), oracle.as_str());
}

#[test]
fn conditional_graveyard_copy_cast_keeps_shared_filter_scope_and_result_gate() {
    let name = "Jacob Frye";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("DealsCombatDamageToPlayerTrigger")
            && debug.contains("one_or_more: true")
            && debug.contains("Assassin")
            && debug.contains("freerunning")
            && debug.contains("CastTaggedEffect")
            && debug.contains("as_copy: true"),
        "conditional graveyard copy/cast must retain its typed trigger, filter, and copy action: {debug}"
    );
    assert!(
        !debug.contains("CopySpellEffect"),
        "a copied graveyard card must not use stack-spell copy semantics: {debug}"
    );

    let compiled = unprocessed_compiled_lines(&definition);
    assert!(compiled.iter().any(|line| {
        line == "Whenever one or more Assassins you control deal combat damage to a player, exile up to one target Assassin card or card with freerunning from your graveyard. If you do, copy it. You may cast the copy."
    }), "conditional shared-provenance copy/cast sequence should render exactly: {compiled:?}");

    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "{name} must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}

#[test]
fn crime_copy_cast_keeps_the_cast_result_life_loss_after_the_typed_copy() {
    let name = "Kaervek, the Punisher";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("KeywordActionTrigger")
            && debug.contains("CommitCrime")
            && debug.contains("CastTaggedEffect")
            && debug.contains("as_copy: true")
            && debug.contains("IfEffect")
            && debug.contains("LoseLifeEffect"),
        "crime copy/cast must retain the typed copied-card permission and its cast-result consequence: {debug}"
    );
    assert!(
        !debug.contains("CopySpellEffect"),
        "a black card selected in a graveyard is not a spell on the stack: {debug}"
    );

    let compiled = unprocessed_compiled_lines(&definition);
    assert_eq!(compiled.join("\n"), oracle.as_str());
    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "{name} must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}
