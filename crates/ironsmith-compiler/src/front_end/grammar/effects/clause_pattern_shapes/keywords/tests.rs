use super::*;
use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

#[test]
fn parses_phase_all_shape() {
    let tokens = lex_line("Simultaneously, all phased-out creatures phase in.", 0).unwrap();
    assert!(matches!(
        parse_keyword_mechanic_tokens(&tokens),
        Some(KeywordMechanicShape::Phase {
            direction: PhaseDirectionShape::In,
            subject: PhaseSubjectShape::All(_),
        })
    ));
}

#[test]
fn phase_action_shape_rejects_negated_restriction_subjects() {
    for text in [
        "Permanents can't phase in.",
        "Permanents cannot phase in.",
        "Permanents can not phase in.",
        "Creatures don't phase out.",
        "This creature does not phase out.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(
            parse_keyword_mechanic_tokens(&tokens).is_none(),
            "{text} is a restriction, not a resolving phase action"
        );
    }
}

#[test]
fn parses_counted_manifest_dread() {
    let tokens = lex_line("Manifest dread three times.", 0).unwrap();
    assert!(matches!(
        parse_keyword_mechanic_tokens(&tokens),
        Some(KeywordMechanicShape::ManifestDread {
            repeat: KeywordRepeatShape::Count(_),
        })
    ));
}

#[test]
fn parses_bare_manifest_dread() {
    let tokens = lex_line("Manifest dread.", 0).unwrap();
    assert!(matches!(
        parse_keyword_mechanic_tokens(&tokens),
        Some(KeywordMechanicShape::ManifestDread {
            repeat: KeywordRepeatShape::Once,
        })
    ));
}

#[test]
fn parses_cloak_top_card_for_you_and_that_player() {
    for (text, expected_player) in [
        (
            "Cloak the top card of your library.",
            ManifestPlayerShape::You,
        ),
        (
            "Cloak the top card of that player's library.",
            ManifestPlayerShape::ThatPlayerOrTargetController,
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(matches!(
            parse_keyword_mechanic_tokens(&tokens),
            Some(KeywordMechanicShape::CloakTop { player }) if player == expected_player
        ));
    }
}

#[test]
fn parses_hyphenated_six_sided_dice() {
    let tokens = lex_line("Roll X six-sided dice.", 0).unwrap();
    assert!(matches!(
        parse_keyword_mechanic_tokens(&tokens),
        Some(KeywordMechanicShape::RollD6 { count_tokens })
            if TokenWordView::new(count_tokens).word_refs() == ["x"]
    ));
}

#[test]
fn parses_comma_delimited_odd_and_even_result_actions() {
    for (text, expected_odd, expected_action) in [
        ("For each odd result, create a token.", true, "create"),
        ("For each even result, put a counter on this.", false, "put"),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let Some(KeywordMechanicShape::OddEvenResult { odd, action_tokens }) =
            parse_keyword_mechanic_tokens(&tokens)
        else {
            panic!("expected odd/even result shape for {text}");
        };
        assert_eq!(odd, expected_odd);
        assert_eq!(
            TokenWordView::new(action_tokens)
                .word_refs()
                .first()
                .copied(),
            Some(expected_action)
        );
    }
}

#[test]
fn parses_subject_endure_shape() {
    let tokens = lex_line("Target creature endures X.", 0).unwrap();
    assert!(matches!(
        parse_keyword_mechanic_tokens(&tokens),
        Some(KeywordMechanicShape::Endure {
            subject: KeywordSubjectShape::Target(_),
            ..
        })
    ));

    let source_tokens = lex_line("This creature endures 1.", 0).unwrap();
    assert!(matches!(
        parse_keyword_mechanic_tokens(&source_tokens),
        Some(KeywordMechanicShape::Endure {
            subject: KeywordSubjectShape::Source(_),
            ..
        })
    ));
}

#[test]
fn parses_harness_with_named_source_tail() {
    let tokens = lex_line("Harness this.", 0).unwrap();
    assert!(matches!(
        parse_keyword_mechanic_tokens(&tokens),
        Some(KeywordMechanicShape::Harness)
    ));
}
