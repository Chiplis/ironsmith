use super::*;
use crate::lexer::lex_line;

fn parse(first: &str, second: &str) -> Option<Vec<EffectAst>> {
    let first = lex_line(first, 0).expect("first sentence should lex");
    let second = lex_line(second, 1).expect("second sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    parse_target_opponent_may_copy_triggering_spell_then_retarget(&sentences, 0)
        .expect("target-opponent copy parser should not error")
}

#[test]
fn keeps_selected_opponent_as_copier_and_new_target_chooser() {
    let effects = parse(
        "Up to one target opponent may also copy that spell",
        "They may choose new targets for that copy",
    )
    .expect("the exact two-sentence copy family should parse");
    let [
        EffectAst::SubjectVerb(target),
        EffectAst::MayByPlayer { player, effects },
    ] = effects.as_slice()
    else {
        panic!("expected a target declaration and opponent-scoped offer: {effects:#?}");
    };
    assert!(matches!(
        target.action,
        SubjectVerbActionAst::TargetOnly {
            explicit_declaration: true,
            ..
        }
    ));
    assert_eq!(*player, PlayerAst::TargetOpponent);
    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell {
                target: TargetAst::Tagged(tag, None),
                player: PlayerAst::TargetOpponent,
                may_choose_new_targets: true,
                ..
            },
            ..
        })] if tag.as_str() == "triggering"
    ));
}

#[test]
fn changed_recipient_or_missing_retarget_sentence_is_not_claimed() {
    assert!(
        parse(
            "Up to one target player may also copy that spell",
            "They may choose new targets for that copy",
        )
        .is_none()
    );
    assert!(
        parse(
            "Up to one target opponent may also copy that spell",
            "Draw a card",
        )
        .is_none()
    );
}
