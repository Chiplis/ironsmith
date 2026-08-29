use super::*;

#[test]
fn singular_return_result_gets_a_one_shot_enter_watcher() {
    let lexed = crate::lexer::lex_line(
            "Return target permanent card from an opponent's graveyard to the battlefield. When that permanent enters, return up to one target permanent card from your graveyard to the battlefield.",
            0,
        )
        .expect("linked return should lex");
    let parsed =
        parse_effect_sentences_lexed(&lexed).expect("linked return should parse structurally");

    let [
        EffectAst::SubjectVerb(first),
        EffectAst::DelayedTriggerForDuration {
            trigger:
                crate::cards::builders::TriggerSpec::ThisEntersBattlefieldWithSurface {
                    surface: crate::target::SourceReferenceSurface::ThisPermanentType(surface),
                    ..
                },
            effects,
            one_shot: true,
            duration: Until::Forever,
            ..
        },
    ] = parsed.as_slice()
    else {
        panic!("expected return followed by linked enter watcher: {parsed:#?}");
    };
    assert!(matches!(
        first.action,
        SubjectVerbActionAst::ReturnToBattlefield { .. }
    ));
    assert_eq!(surface, "that permanent");
    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnToBattlefield { .. },
            ..
        })]
    ));
}

#[test]
fn anaphor_and_singular_return_guards_reject_near_misses() {
    for text in [
        "Return target permanent card from an opponent's graveyard to the battlefield. When that card enters, return target permanent card from your graveyard to the battlefield.",
        "Return up to two target permanent cards from an opponent's graveyard to the battlefield. When that permanent enters, return target permanent card from your graveyard to the battlefield.",
    ] {
        let lexed = crate::lexer::lex_line(text, 0).expect("near miss should lex");
        let parsed = parse_effect_sentences_lexed(&lexed).expect("near miss should still parse");
        assert!(
            !parsed
                .iter()
                .any(|effect| matches!(effect, EffectAst::DelayedTriggerForDuration { .. })),
            "near miss must not acquire linked delayed semantics: {parsed:#?}"
        );
    }
}
