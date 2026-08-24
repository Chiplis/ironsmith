use super::*;

fn parsed_row(text: &str) -> EffectAst {
    let tokens = crate::lexer::lex_line(text, 0).expect("numeric result row should lex");
    let mut effects =
        parse_effect_sentences_lexed(&tokens).expect("numeric result row should parse");
    assert_eq!(effects.len(), 1, "{effects:#?}");
    effects.pop().expect("one parsed row")
}

#[test]
fn exact_numeric_result_row_retains_its_authored_inner_label() {
    let effect = parsed_row("1 | Trapped! — You lose 3 life.");
    let EffectAst::IfResult {
        predicate: IfResultPredicate::Value(crate::effect::Comparison::Equal(1)),
        effects,
    } = effect
    else {
        panic!("expected exact numeric result predicate: {effect:#?}");
    };
    let [EffectAst::ResultBranchLabel { label, effects }] = effects.as_slice() else {
        panic!("expected one typed labeled result body: {effects:#?}");
    };
    assert_eq!(label, "Trapped!");
    assert!(!effects.is_empty());
}

#[test]
fn unlabeled_numeric_result_row_does_not_gain_a_label_wrapper() {
    let effect = parsed_row("1 | You lose 3 life.");
    let EffectAst::IfResult { effects, .. } = effect else {
        panic!("expected numeric result branch: {effect:#?}");
    };
    assert!(
        !matches!(effects.as_slice(), [EffectAst::ResultBranchLabel { .. }]),
        "{effects:#?}"
    );
}
