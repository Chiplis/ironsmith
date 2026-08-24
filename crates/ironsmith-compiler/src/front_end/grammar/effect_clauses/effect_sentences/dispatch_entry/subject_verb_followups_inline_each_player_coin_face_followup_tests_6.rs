use super::*;

#[test]
fn heads_and_tails_followups_keep_face_only_player_correlation() {
    for (face, expected_predicate) in [
        ("heads", IfResultPredicate::Did),
        ("tails", IfResultPredicate::DidNot),
    ] {
        let text = format!(
            "Each player flips a coin. Each player whose coin comes up {face} sacrifices a creature of their choice."
        );
        let tokens =
            crate::lexer::lex_line(&text, 0).expect("coin-face player sequence should lex");
        let effects =
            parse_effect_sentences_lexed(&tokens).expect("coin-face player sequence should parse");

        let [
            EffectAst::ForEachPlayer {
                effects: flip_effects,
            },
            EffectAst::ForEachPlayerDid {
                result_predicate,
                effects: followups,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected correlated flip/follow-up pair: {effects:#?}");
        };
        assert!(matches!(
            flip_effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::FlipCoinFaceOnly,
                ..
            })]
        ));
        assert_eq!(result_predicate, &expected_predicate);
        assert!(!followups.is_empty());
    }
}
