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

#[test]
fn artifact_offering_keeps_a_controlled_artifact_sacrifice() {
    let line = line("Artifact offering");
    let payload = parse_alternative_cast(&line, &line.tokens, &line.info.source_tokens)
        .unwrap().expect("artifact offering should parse");
    let debug = format!("{payload:?}");
    assert!(debug.contains("Sacrifice"), "{debug}");
    assert!(debug.contains("card_types: [Artifact]"), "{debug}");
    assert!(debug.contains("controller: Some(You)"), "{debug}");
    assert!(debug.contains("subtypes: []"), "{debug}");
}

#[test]
fn flashback_cost_modifiers_defer_to_static_cost_rules() {
    for text in ["Flashback costs you pay cost {2} less.", "Flashback costs your opponents pay cost {2} more."] {
        let line = line(text);
        assert!(parse_flashback(&line, &line.tokens, &line.info.source_tokens).unwrap().is_none());
        let ability = crate::keyword_static::parse_static_ability_ast_line_lexed(&line.tokens)
            .expect("static modifier should parse").expect("static modifier should match");
        let debug = format!("{ability:?}");
        assert!(debug.contains("Flashback"), "{debug}");
        assert!(debug.contains("Fixed(2)"), "{debug}");
        assert!(debug.contains(if text.contains("opponents") { "Opponent" } else { "You" }), "{debug}");
    }
}

#[test]
fn repeated_additional_mana_payment_and_count_share_a_reference() {
    let line = line("As an additional cost to cast this spell, you may pay {1}{G} any number of times.");
    let payload = parse_additional_cost(&line, &line.tokens, &line.info.source_tokens).unwrap().unwrap();
    let KeywordLinePayload::Ast(ast) = payload else { panic!("optional cost AST required"); };
    let LineAst::OptionalCost(cost) = *ast else { panic!("optional cost required"); };
    assert!(cost.repeatable);
    assert_eq!(cost.cost.mana_cost().unwrap().to_oracle(), "{1}{G}");
    let (value, used) = crate::util::parse_for_each_count_value_words(&["for", "each", "additional", "1", "g", "you", "paid"]).unwrap();
    assert_eq!(used, 7);
    assert!(matches!(value.unhinted(), Value::TimesPaidLabel(reference) if *reference == cost.reference));
    let (other, _) = crate::util::parse_for_each_count_value_words(&["for", "each", "additional", "1", "r", "you", "paid"]).unwrap();
    assert!(matches!(other.unhinted(), Value::TimesPaidLabel(reference) if *reference != cost.reference));
    let tokens = crate::lexer::lex_line("You gain 3 life plus an additional 3 life for each additional {1}{G} you paid.", 0).unwrap();
    let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens).unwrap();
    let [EffectAst::SubjectVerb(effect)] = effects.as_slice() else { panic!("one life-gain effect required: {effects:?}"); };
    let crate::cards::builders::SubjectVerbActionAst::LifeResources(crate::cards::builders::LifeResourceActionAst::GainLife { amount }) = &effect.action
        else { panic!("life gain required: {effects:?}"); };
    let Value::Add(base, extra) = amount.unhinted() else { panic!("base plus repeated increment required: {amount:?}"); };
    assert_eq!(base.unhinted(), &Value::Fixed(3));
    let Value::Scaled(count, 3) = extra.unhinted() else { panic!("three life per payment required: {extra:?}"); };
    assert!(matches!(count.unhinted(), Value::TimesPaidLabel(reference) if *reference == cost.reference));

}
