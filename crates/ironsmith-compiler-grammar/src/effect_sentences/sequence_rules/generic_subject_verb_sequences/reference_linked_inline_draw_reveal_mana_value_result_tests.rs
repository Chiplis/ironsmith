use super::*;
use crate::lexer::lex_line;

fn parse(first: &str, second: &str) -> Option<Vec<EffectAst>> {
    let first = lex_line(first, 0).expect("first sentence should lex");
    let second = lex_line(second, 1).expect("second sentence should lex");
    parse_draw_reveal_then_triggering_creature_mana_value_result(
        &[
            SentenceInput::from_lexed(&first),
            SentenceInput::from_lexed(&second),
        ],
        0,
    )
    .expect("draw/reveal sequence parser should not error")
}

#[test]
fn exports_the_drawn_card_without_rebinding_the_triggering_creature() {
    let effects = parse(
            "Draw a card and reveal it",
            "The creature gets +X/+X until end of turn and you lose X life, where X is that card's mana value",
        )
        .expect("exact draw/reveal result family should parse");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("__drawn_revealed_card__"), "{debug}");
    assert_eq!(debug.matches("ManaValueOf").count(), 3, "{debug}");
    let [
        _,
        EffectAst::SourceSentence {
            effects: result, ..
        },
    ] = effects.as_slice()
    else {
        panic!("expected two authored source sentences: {effects:#?}");
    };
    let [
        EffectAst::Coordinated {
            effects: result, ..
        },
    ] = result.as_slice()
    else {
        panic!("expected a coordinated result sentence: {result:#?}");
    };
    assert!(matches!(
        result.as_slice(),
        [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Pump {
                    target: TargetAst::Tagged(tag, None),
                    ..
                },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LoseLife { .. },
                ..
            }),
        ] if tag.as_str() == "triggering"
    ));
}

#[test]
fn changed_result_subject_or_value_source_is_not_claimed() {
    assert!(
            parse(
                "Draw a card and reveal it",
                "Creatures get +X/+X until end of turn and you lose X life, where X is that card's mana value",
            )
            .is_none()
        );
    assert!(
            parse(
                "Draw a card and reveal it",
                "The creature gets +X/+X until end of turn and you lose X life, where X is that card's power",
            )
            .is_none()
        );
}
