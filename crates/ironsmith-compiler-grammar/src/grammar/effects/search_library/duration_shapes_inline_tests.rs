use crate::lexer::lex_line;

use super::*;

#[test]
fn parses_conditional_search_restriction_durations() {
    let tokens = lex_line(
        "for as long as you control this artifact, you may cast that card",
        0,
    )
    .unwrap();
    let parsed = parse_search_restriction_duration_shape_lexed(&tokens)
        .unwrap()
        .unwrap();
    assert_eq!(parsed.duration, Until::YouStopControllingThis);
    assert_eq!(parsed.placement, SearchRestrictionDurationPlacement::Prefix);
    assert!(!parsed.remainder.is_empty());

    let tokens = lex_line("you may play it this turn", 0).unwrap();
    let parsed = parse_search_restriction_duration_shape_lexed(&tokens)
        .unwrap()
        .unwrap();
    assert_eq!(parsed.duration, Until::EndOfTurn);
    assert_eq!(parsed.placement, SearchRestrictionDurationPlacement::Suffix);
}

#[test]
fn distinguishes_leading_and_trailing_animation_durations() {
    let leading = lex_line("Until end of turn, target land becomes a 4/4 creature", 0).unwrap();
    let trailing = lex_line(
            "target artifact becomes an artifact creature for as long as this creature remains on the battlefield",
            0,
        )
        .unwrap();

    assert_eq!(
        parse_search_restriction_duration_shape_lexed(&leading)
            .unwrap()
            .unwrap()
            .placement,
        SearchRestrictionDurationPlacement::Prefix
    );
    assert_eq!(
        parse_search_restriction_duration_shape_lexed(&trailing)
            .unwrap()
            .unwrap()
            .placement,
        SearchRestrictionDurationPlacement::Suffix
    );
}


#[test]
fn named_player_step_is_not_a_search_or_permission_duration() {
    for text in [
        "during that player's next untap step, you may play that card",
        "you may play that card during that player's next untap step",
        "until its controller's next untap step, you may play that card",
        "you may play that card until its controller's next untap step",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(parse_search_restriction_duration_shape_lexed(&tokens).unwrap().is_none());
    }
}
