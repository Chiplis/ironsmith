use crate::cards::builders::ReplacementActionAst;
use super::*;
use crate::lexer::lex_line;

fn parse(second: &str) -> Option<Vec<EffectAst>> {
    let first = lex_line("Counter target spell.", 0).expect("counter should lex");
    let second = lex_line(second, 1).expect("rider should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    parse_counter_spell_then_artifact_or_creature_enters_under_your_control(&sentences, 0)
        .expect("pair parser should not error")
}

#[test]
fn counter_destination_is_registered_before_countering() {
    let effects = parse("If an artifact or creature spell is countered this way, put that card onto the battlefield under your control instead of into its owner's graveyard.")
            .expect("exact counter replacement should match");
    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_true, .. }), EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Stack(StackActionAst::Counter { .. }), .. })]
                if matches!(
                    if_true.as_slice(),
                    [
                        EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement { .. }), .. }),
                        EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterEnterUnderControlReplacement { .. }), .. }),
                    ]
                )
        ),
        "{effects:#?}"
    );
}

#[test]
fn post_counter_graveyard_move_and_wrong_card_types_are_near_misses() {
    for second in [
        "If an artifact or creature spell is countered this way, return that card from its owner's graveyard to the battlefield under your control.",
        "If an enchantment spell is countered this way, put that card onto the battlefield under your control instead of into its owner's graveyard.",
    ] {
        assert!(parse(second).is_none(), "overclaimed: {second}");
    }
}
