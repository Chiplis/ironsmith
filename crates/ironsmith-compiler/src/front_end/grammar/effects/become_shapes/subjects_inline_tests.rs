use crate::lexer::lex_line;

use super::*;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("lex fixture")
}

#[test]
fn classifies_typed_target_subjects_and_duration_recovery() {
    let subject = lex("all");
    let body = lex("blue");
    assert!(matches!(
        parse_become_target_subject_shape(&subject, &body),
        BecomeTargetSubjectShape::Mass(BecomeMassTargetKind::Creature)
    ));

    let tagged = lex("each of those creatures");
    assert!(matches!(
        parse_become_target_subject_shape(&tagged, &body),
        BecomeTargetSubjectShape::Tagged
    ));
    assert_eq!(
        become_subject_set_quantifier_surface(&lex("they")),
        Some(ironsmith_core::SetQuantifierSurface::They)
    );
    assert_eq!(become_subject_set_quantifier_surface(&lex("it")), None);

    let duration = lex("until end of turn, target artifact");
    assert_eq!(
        parse_leading_duration_target_tokens(&duration)
            .map(parser_token_word_refs)
            .unwrap(),
        ["target", "artifact"]
    );

    let context = crate::parse_context::ParseContext::for_fragment(
        "Sarkhan, Soul Aflame",
        Vec::new(),
        Vec::new(),
        "Sarkhan",
    );
    let named_source = lex("Sarkhan");
    assert_eq!(
        parse_become_target_subject_shape_with_context(context.view(), &named_source, &body),
        BecomeTargetSubjectShape::Source(SourceReferenceSurface::ShortName("Sarkhan".to_string()))
    );
}
