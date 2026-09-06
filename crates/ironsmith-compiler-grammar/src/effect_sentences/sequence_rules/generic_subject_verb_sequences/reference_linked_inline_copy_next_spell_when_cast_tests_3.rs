use super::*;
use crate::lexer::lex_line;

fn parse(first: &str, second: &str) -> Option<Vec<EffectAst>> {
    let first = lex_line(first, 0).expect("first sentence should lex");
    let second = lex_line(second, 1).expect("second sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("copy-next-spell parser should not error")
}

#[test]
fn builds_one_shot_spell_cast_watcher_and_copies_that_spell() {
    let effects = parse(
        "Copy the next spell you cast this turn when you cast it",
        "You may choose new targets for the copy",
    )
    .expect("inverted delayed-copy wording should parse");
    let [
        EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
            trigger:
                TriggerSpec::SpellCast {
                    filter: None,
                    caster: PlayerFilter::You,
                    ..
                },
            effects: delayed,
            one_shot: true,
            until_end_of_combat: false,
            attach_to_previous_ability: false,
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one-shot spell watcher: {effects:#?}");
    };
    assert!(matches!(
        delayed.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Stack(StackActionAst::CopySpell {
                target: TargetAst::Tagged(tag, None),
                count: Value::Fixed(1),
                player: PlayerAst::You,
                may_choose_new_targets: true,
                ..
            }),
            ..
        })] if tag.as_str() == "triggering"
    ));
}

#[test]
fn does_not_claim_an_existing_stack_object_or_missing_retarget_sentence() {
    assert!(
        parse(
            "Copy target spell you control",
            "You may choose new targets for the copy"
        )
        .is_none()
    );
    assert!(
        parse(
            "Copy the next spell you cast this turn when you cast it",
            "Draw a card"
        )
        .is_none()
    );
}
