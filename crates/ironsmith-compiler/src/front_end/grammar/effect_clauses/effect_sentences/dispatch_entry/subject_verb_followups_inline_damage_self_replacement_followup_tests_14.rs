use super::*;

#[test]
fn it_deals_to_that_creature_ignores_prior_cost_object_provenance() {
    let lexed = crate::lexer::lex_line(
            "This deals 2 damage to target creature. It deals 4 damage to that creature instead if this spell's additional cost was paid.",
            0,
        )
        .expect("damage self-replacement should lex");
    let parsed =
        parse_effect_sentences_lexed(&lexed).expect("damage self-replacement should parse");
    let [
        EffectAst::SelfReplacement {
            if_true, if_false, ..
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one typed self-replacement: {parsed:#?}");
    };
    assert!(
        matches!(
            if_true.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::DealDamageEqualToPower {
                    source: TargetAst::Source(_),
                    target: TargetAst::Object(filter, None, Some(_)),
                    ..
                },
                ..
            })] if !filter.tagged_constraints.is_empty()
        ),
        "the replacement should directly reuse the default spell source and target: {if_true:#?}"
    );
    assert_eq!(if_false.len(), 1, "default damage should remain intact");
    assert!(
        !format!("{parsed:#?}").contains("TrailingIf"),
        "the authored trailing-if surface must be consumed by the typed self-replacement: {parsed:#?}"
    );

    let lowered = crate::compile_support::compile_statement_effects_with_imports(
        &parsed,
        &crate::model::reference_state::ReferenceImports::with_last_object_tag("counters_0"),
    )
    .expect("damage self-replacement should lower");
    let debug = format!("{lowered:#?}");
    assert!(debug.contains("ExecuteWithSourceEffect"), "{debug}");
    assert!(debug.contains("source: Source"), "{debug}");
    assert!(!debug.contains("ForEachObject"), "{debug}");
    assert!(!debug.contains("counters_0"), "{debug}");
}

#[test]
fn omitted_damage_target_reuses_the_default_target() {
    let lexed = crate::lexer::lex_line(
            "This deals 3 damage to target creature. It deals 5 damage instead if you control an artifact.",
            0,
        )
        .expect("damage self-replacement should lex");
    let parsed =
        parse_effect_sentences_lexed(&lexed).expect("damage self-replacement should parse");
    let [EffectAst::SelfReplacement { if_true, .. }] = parsed.as_slice() else {
        panic!("expected one typed self-replacement: {parsed:#?}");
    };
    assert!(
        matches!(
            if_true.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::DealDamageEqualToPower {
                    target: TargetAst::Object(_, Some(_), _),
                    ..
                },
                ..
            })]
        ),
        "the omitted replacement target should reuse the default creature target: {if_true:#?}"
    );
}
