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
            normalized: NormalizedLine::from_char_map(text, text.to_ascii_lowercase(), Vec::new()),
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
    assert!(matches!(payload, KeywordLinePayload::Kicker { .. }));
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

#[test]
fn flashback_uses_the_full_source_line_for_its_qualified_reduction() {
    let text = "Flashback {8}{B}{B}. This spell costs {X} less to cast this way, where X is the greatest mana value of a commander you own on the battlefield or in the command zone.";
    let line = line(text);
    let sentences = split_lexed_sentences(&line.tokens);
    let payload = parse_flashback(&line, sentences[0], &line.info.source_tokens)
        .expect("compound flashback parser should succeed")
        .expect("compound flashback line should match");
    let KeywordLinePayload::Ast(ref ast) = payload else {
        panic!("compound flashback should retain both typed clauses: {payload:#?}");
    };
    let LineAst::Multiple(chunks) = ast.as_ref() else {
        panic!("compound flashback should retain both typed clauses: {payload:#?}");
    };
    assert_eq!(chunks.len(), 2, "{chunks:#?}");
    assert!(matches!(
        &chunks[0],
        LineAst::AlternativeCastingMethod(
            crate::model::CompilerAlternativeCastingMethod::Flashback { .. }
        )
    ));
    let LineAst::StaticAbility(StaticAbilityAst::Static(ability)) = &chunks[1] else {
        panic!("qualified reduction should remain a typed static ability: {chunks:#?}");
    };
    let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) = &ability.payload
    else {
        panic!("expected a typed this-spell reduction: {ability:#?}");
    };
    assert_eq!(
        reduction.alternative_cast,
        Some(crate::filter::AlternativeCastKind::Flashback)
    );
}
