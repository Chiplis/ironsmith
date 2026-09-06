use super::*;
use crate::cards::builders::PredicateAst;
use crate::lexer::lex_line;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("restriction should lex")
}

#[test]
fn activation_fact_carries_timing_condition_and_residual_text() {
    let parsed = parse_activation_restriction_tokens(&lex(
        "Activate only once each turn and only if this creature attacked this turn.",
    ))
    .expect("activation restriction should parse");

    assert_eq!(parsed.timing, Some(ActivationTiming::OncePerTurn));
    assert_eq!(parsed.condition, Some(PredicateAst::Source(SourcePredicateAst::SourceAttackedThisTurn)));
    assert_eq!(
        parsed.text_only_condition,
        Some(PredicateAst::Source(SourcePredicateAst::SourceAttackedThisTurn))
    );
    assert_eq!(
        parsed.normalization,
        ActivationRestrictionNormalizationFact::Residual(
            "only if this creature attacked this turn".to_string()
        )
    );
    assert!(!parsed.once_per_turn_after_other_restrictions);
}

#[test]
fn once_per_turn_fact_does_not_duplicate_the_equivalent_condition() {
    let parsed = parse_activation_restriction_tokens(&lex("Activate only once each turn."))
        .expect("activation restriction should parse");

    assert_eq!(parsed.timing, Some(ActivationTiming::OncePerTurn));
    assert_eq!(parsed.condition, None);
    assert_eq!(
        parsed.normalization,
        ActivationRestrictionNormalizationFact::Redundant
    );
    assert!(!parsed.once_per_turn_after_other_restrictions);
}

#[test]
fn activation_fact_records_a_trailing_once_per_turn_clause() {
    let parsed = parse_activation_restriction_tokens(&lex(
        "Activate only if an opponent lost life this turn and only once each turn.",
    ))
    .expect("activation restriction should parse");

    assert_eq!(parsed.timing, Some(ActivationTiming::OncePerTurn));
    assert!(parsed.once_per_turn_after_other_restrictions);
}

#[test]
fn trigger_and_mana_facts_are_typed_before_lowering() {
    let trigger =
        parse_trigger_restriction_tokens(&lex("This ability triggers only twice each turn."))
            .expect("trigger restriction should parse");
    assert_eq!(trigger.max_times_each_turn, Some(2));

    let mana = parse_mana_restriction_tokens(&lex("Spend this mana only to cast artifact spells."))
        .expect("mana restriction should parse");
    assert!(mana.usage_restriction.is_some());
}
