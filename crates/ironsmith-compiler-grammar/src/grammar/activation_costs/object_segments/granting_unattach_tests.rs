//! Source-authored regressions; UNRUN during the campaign execution pause.
use super::*;
use crate::lexer::lex_line;

#[test]
fn unattach_granting_attachment_keeps_identity_without_controller_or_host_substitution() {
    let tokens = lex_line("Unattach granting permanent", 0).unwrap();
    let ActivationCostSegmentCst::UnattachChosen { count, filter } =
        parse_unattach_segment_tokens(&tokens, |_| false).unwrap()
    else {
        panic!("expected the granting attachment as the unattach cost");
    };
    assert_eq!(count, 1);
    assert_eq!(filter, ObjectFilter::tagged(
        crate::tag::CompilerReferenceTag::GrantingSource.key(),
    ).in_zone(Zone::Battlefield));
    assert!(!filter.source);
    assert!(filter.controller.is_none());
    assert!(filter.attached_to_object.is_none());
}

#[test]
fn granting_attachment_marker_must_own_the_complete_unattach_operand() {
    for text in [
        "Unattach granting permanent and another Equipment",
        "Unattach granting permanent you control",
        "Unattach granting permanent this turn",
        "Unattach granting creature",
        "Unattach Blinding Powder",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(parse_unattach_segment_tokens(&tokens, |_| false).is_err(), "{text}");
    }
    let tokens = lex_line("Unattach this source", 0).unwrap();
    assert_eq!(
        parse_unattach_segment_tokens(&tokens, |words| {
            leaf::parse_leaf_this_source_reference_words(words).is_some()
        }).unwrap(),
        ActivationCostSegmentCst::UnattachChosen { count: 1, filter: ObjectFilter::source() },
    );
}
