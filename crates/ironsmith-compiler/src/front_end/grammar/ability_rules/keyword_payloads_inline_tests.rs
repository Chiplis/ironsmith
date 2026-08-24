use super::*;
use crate::cards::builders::{LineInfo, NormalizedLine};
use crate::lexer::lex_line;

fn line(text: &str) -> PreprocessedLine {
    let tokens = lex_line(text, 0).expect("keyword test line should lex");
    PreprocessedLine {
        info: LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine {
                original: text.to_string(),
                normalized: text.to_ascii_lowercase(),
                char_map: Vec::new(),
            },
            semantic_facts: Default::default(),
        },
        tokens,
    }
}

#[test]
fn kicker_parser_carries_cost_without_lowering_reparse() {
    let line = line("Kicker {2}{R}");
    let payload = parse_kicker(&line, &line.tokens, &line.info.source_tokens)
        .expect("kicker parser should succeed")
        .expect("kicker should match");
    assert!(matches!(payload, KeywordLinePayloadCst::Kicker { .. }));
}

#[test]
fn blitz_cost_modifier_is_not_claimed_as_keyword_payload() {
    let line = line("Blitz costs you pay cost {1} less.");
    assert!(
        parse_blitz(&line, &line.tokens, &line.info.source_tokens)
            .expect("blitz parser should not fail")
            .is_none()
    );
}
