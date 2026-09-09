use super::*;

#[test]
fn activated_pump_and_keyword_share_the_leading_duration() {
    for text in [
        "Until end of turn, this creature gets +1/+1 for each experience counter you have and gains menace.",
        "Until end of turn, Azula gets +1/+1 for each experience counter you have and gains menace.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).expect("activated body should lex");
        let effects = parse_activated_effects_lexed("", &tokens, 0)
            .expect("activated pump-and-keyword body should parse");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("Pump"), "{debug}");
        assert!(debug.contains("PlayerCounters"), "{debug}");
        assert!(debug.contains("Menace"), "{debug}");
        assert!(!debug.contains("ExperienceCounters"), "{debug}");
    }
}

#[test]
fn next_turn_pump_and_activation_restriction_keeps_typed_duration_scope() {
    let tokens = crate::lexer::lex_line(
            "Until your next turn, up to one target creature gets -3/-0 and its activated abilities can't be activated.",
            0,
        )
        .expect("activated body should lex");
    let effects =
        parse_activated_effects_lexed("", &tokens, 0).expect("activated body should parse");
    let [EffectAst::ControlFlow(control)] = effects.as_slice() else {
        panic!("expected one duration control-flow node, got {effects:#?}");
    };
    let crate::model::ControlFlowNodeAst::Duration { duration, program } = &control.node else {
        panic!("expected a duration node, got {control:#?}");
    };
    assert_eq!(duration, &crate::model::CompilerDurationAst::UntilNextTurn);
    let program = control
        .program(*program)
        .expect("duration node should reference its effect program");
    assert!(program.effects.iter().any(|effect| matches!(
        effect,
        EffectAst::SubjectVerb(subject_verb)
            if matches!(&subject_verb.action, SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { .. }))
    )));
    assert!(program.effects.iter().any(|effect| matches!(
        effect,
        EffectAst::SubjectVerb(subject_verb)
            if matches!(&subject_verb.action, SubjectVerbActionAst::Cant { .. })
    )));
}

#[test]
fn activated_reveal_conditional_pump_preserves_preceding_action() {
    let tokens = crate::lexer::lex_line(
        "Reveal the top card of your library. If it's a land card, this creature gets +1/+0 and gains flying until end of turn.",
        0,
    ).expect("activated body should lex");
    let effects = parse_activated_effects_lexed("", &tokens, 0)
        .expect("reveal and conditional modifier should parse");
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("Reveal"),
        "preceding reveal was lost: {debug}"
    );
    assert!(debug.contains("Land"), "land predicate was lost: {debug}");
    assert!(debug.contains("Pump"), "{debug}");
    assert!(debug.contains("Flying"), "{debug}");
    assert!(
        debug.contains("Conditional") || debug.contains("If"),
        "conditional was lost: {debug}"
    );
}
