#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn source_line_keyword_group_counts(definition: &CardDefinition) -> Vec<usize> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let model = static_ability.compiled_model()?;
            let ironsmith_core::StaticAbilityPayload::SourceLineKeywordGroup { keyword_count } =
                &model.payload
            else {
                return None;
            };
            Some(*keyword_count)
        })
        .collect()
}

#[test]
fn same_source_line_keyword_siblings_keep_typed_group_provenance() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Same-Line Keyword Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Vigilance, reach")
        .expect("same-line keyword siblings should parse");

    assert_eq!(source_line_keyword_group_counts(&definition), vec![2]);
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["Vigilance, reach".to_string()]
    );
}

#[test]
fn separate_source_line_keywords_do_not_gain_group_provenance() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Separate-Line Keyword Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Reach\nDaybound")
        .expect("separate keyword lines should parse");

    assert!(source_line_keyword_group_counts(&definition).is_empty());
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["Reach".to_string(), "Daybound".to_string()]
    );
}

#[test]
fn finneas_keeps_the_exact_authored_two_keyword_line() {
    let definition = parse_oracle_card_definition("Finneas, Ace Archer");
    let lines = canonical_compiled_lines(&definition);

    assert_eq!(source_line_keyword_group_counts(&definition), vec![2]);
    assert_eq!(lines.first().map(String::as_str), Some("Vigilance, reach"));
    assert!(
        !lines.iter().any(|line| line == "Reach"),
        "the second same-line keyword must not be emitted as a separate Oracle line: {lines:?}"
    );
    assert_eq!(
        lines,
        vec![
            "Vigilance, reach".to_string(),
            "Whenever Finneas attacks, put a +1/+1 counter on each other creature you control that's a token or a Rabbit. Then if creatures you control have total power 10 or greater, draw a card.".to_string(),
        ]
    );
}
