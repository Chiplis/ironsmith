use super::*;
use crate::lexer::lex_line;

#[test]
fn resolving_card_exile_registers_exact_one_shot_replacement_and_linked_return() {
    let first = lex_line(
        "Exile that card instead of putting it into your graveyard as it resolves.",
        0,
    )
    .expect("replacement sentence should lex");
    let second = lex_line(
        "If you do, return it to your hand at the beginning of the next end step.",
        1,
    )
    .expect("conditional return sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];

    let effects = parse_resolving_card_exile_then_return_next_end_step(&sentences, 0)
        .expect("linked replacement parser should not error")
        .expect("linked replacement sequence should match");
    assert!(matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RegisterZoneReplacement {
                target: TargetAst::Tagged(tag, _),
                from_zone: Some(Zone::Stack),
                to_zone: Some(Zone::Graveyard),
                replacement_zone: Zone::Exile,
                duration: ZoneReplacementDurationAst::OneShot,
                linked_exile_follow_up: Some(
                    ironsmith_core::LinkedExileFollowUp::ReturnToHandAtNextEndStep
                ),
                ..
            },
            ..
        })] if tag.as_str() == "triggering"
    ));
}
