use crate::cards::builders::ZoneMoveActionAst;
use super::*;
use crate::lexer::lex_line;

#[test]
fn participant_relative_secret_object_choices_keep_the_revealed_result_set() {
    let first = lex_line(
        "You and target opponent each secretly choose a creature that player controls.",
        0,
    )
    .expect("secret object choice should lex");
    let second = lex_line(
        "Then those choices are revealed, and that player sacrifices those creatures.",
        1,
    )
    .expect("reveal-and-sacrifice follow-up should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];

    let effects = parse_participant_secret_object_choice_then_reveal_and_sacrifice(&sentences, 0)
        .expect("secret object choice parser should not error")
        .expect("secret object choice sequence should match");
    let [
        EffectAst::Votes(VoteEffectAst::SecretChoiceStart {
            participants,
            object_choice: Some(object_choice),
            ..
        }),
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged { tag, effects }),
    ] = effects.as_slice()
    else {
        panic!("expected a secret selection and tagged sacrifice: {effects:#?}");
    };
    assert_eq!(
        participants,
        &[PlayerFilter::You, PlayerFilter::target_opponent()]
    );
    assert_eq!(object_choice.tag, tag.key);
    assert!(object_choice.reveal_after_choice);
    assert_eq!(
        object_choice.filter.controller,
        Some(PlayerFilter::IteratedPlayer)
    );
    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
                target: Some(TargetAst::Tagged(iterated, _)),
                ..
            }),
            ..
        })] if iterated.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
    ));
}
