//! Independent full-corpus membership oracle; complete physical linked inputs on
//! both strict native and restored artifact routes. Source presence is not a
//! gameplay completion claim.
#[path = "custom_final_next_wave_support.rs"]
mod support;
use std::collections::{BTreeMap, BTreeSet};
use serde_json::Value;
const CORPUS: &str = include_str!("../../../reports/cards/lackey-audit-2026-11-01/rev-complete-source.json");
fn word(text: &str, expected: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphabetic()).any(|part| part.eq_ignore_ascii_case(expected) || match expected { "invoke" => ["invokes", "invoked", "invoking"].iter().any(|form| part.eq_ignore_ascii_case(form)), "reflect" => ["reflects", "reflecting"].iter().any(|form| part.eq_ignore_ascii_case(form)), _ => false })
}
fn collect(family: &str, fixture: &str, expected: usize) {
    let corpus: Value = serde_json::from_str(CORPUS).unwrap();
    let raw: BTreeMap<String, &Value> = corpus["cards"].as_array().unwrap().iter()
        .filter(|card| ["rulesText", "rulesText2"].iter().any(|field| word(card[field].as_str().unwrap_or(""), family)))
        .map(|card| (format!("rev:{}", card["key"].as_str().unwrap()), card)).collect();
    assert_eq!(raw.len(), expected, "whole frozen corpus membership changed: re-audit declarations, consumers, modifiers and negatives");
    let fixture: Value = serde_json::from_str(fixture).unwrap();
    let cards: Vec<&Value> = fixture["mechanics"].as_array().unwrap().iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap()).collect();
    let keys: BTreeSet<String> = cards.iter().map(|card| format!("rev:{}", card["source_card_key"].as_str().unwrap())).collect();
    assert_eq!(cards.len(), keys.len(), "duplicate physical source");
    assert_eq!(keys, raw.keys().cloned().collect(), "source corpus coverage is not seed/representative coverage");
    let mut failures = Vec::new();
    for card in cards {
        let key = format!("rev:{}", card["source_card_key"].as_str().unwrap());
        assert_eq!(&card["original_source"], raw[&key], "immutable complete source changed: {key}");
        let expected_faces = if raw[&key]["cardName2"].as_str().is_some_and(|name| !name.is_empty()) {2} else {1};
        assert_eq!(card["source_faces"].as_array().unwrap().len(), expected_faces, "source sibling dropped: {key}");
        match support::linked_definitions(card) {
            Ok(routes) => {
                assert_eq!(routes.len(), expected_faces, "complete linked physical definitions: {key}");
                for pair in routes { assert_eq!(pair[0].card.name, pair[1].card.name, "native/wire face mismatch: {key}"); }
            }
            Err(error) => failures.push(format!("{key}: {error}")),
        }
    }
    assert!(failures.is_empty(), "{family}: strict complete source admission failures:\n{}", failures.join("\n"));
}
#[test]
fn invoke_all_thirty_physical_sources_including_seven_consumers_and_modifiers() {
    collect("invoke", include_str!("../../../fixtures/custom_mechanics/revolution-invoke-whole-source.json"), 30);
}
#[test]
fn reflect_all_twenty_physical_sources_with_linked_back_programs() {
    collect("reflect", include_str!("../../../fixtures/custom_mechanics/revolution-reflect-whole-source.json"), 20);
}
#[test]
fn reflected_all_twenty_physical_sources_with_linked_front_prices() {
    collect("reflected", include_str!("../../../fixtures/custom_mechanics/revolution-reflected-whole-source.json"), 20);
}
