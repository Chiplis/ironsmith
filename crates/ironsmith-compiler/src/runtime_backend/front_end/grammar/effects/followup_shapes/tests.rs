use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

#[test]
fn parses_create_more_prior_token_shape() {
    let tokens = lex_line("If you do, create two of those tokens instead.", 0).unwrap();
    let shape = parse_create_more_prior_tokens(&tokens).expect("shape");
    assert_eq!(shape.count, 2);
    assert!(!shape.predicate_tokens.is_empty());
}

#[test]
fn parses_conditional_followup_continuation() {
    let tokens = lex_line(
        "When one or more cards are milled this way, draw a card.",
        0,
    )
    .unwrap();
    let shape = parse_conditional_followup(&tokens).expect("shape");
    assert_eq!(shape.kind, ConditionalFollowupKind::WhenMilledThisWay);
    assert!(!shape.continuation_tokens.is_empty());
}

#[test]
fn parses_skip_tapped_source_turn_with_oracle_comma() {
    let tokens = lex_line(
        "If you would begin your turn while this artifact is tapped, you may skip that turn instead.",
        0,
    )
    .unwrap();
    assert!(is_skip_tapped_source_turn_replacement(&tokens));

    let followup = lex_line("If you do, untap this artifact.", 0).unwrap();
    assert!(is_if_did_untap_source_followup(&followup));
}

#[test]
fn parses_shuffle_and_damaged_player_followups() {
    let shuffle = lex_line("If you search your library this way, shuffle.", 0).unwrap();
    assert_eq!(
        parse_library_shuffle_followup_shape(&shuffle),
        Some(LibraryShuffleFollowupShape::IfSearchedThisWay)
    );

    let restriction = lex_line(
        "Players dealt damage this way can't cast noncreature spells this turn.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_damaged_player_followup_shape(&restriction),
        Some(DamagedPlayerFollowupShape::CantCastNoncreatureSpellsThisTurn)
    );
}

#[test]
fn parses_exact_object_followups() {
    assert!(is_tap_damaged_creatures_followup(
        &lex_line("Tap each creature dealt damage this way.", 0).unwrap()
    ));
    assert!(is_still_land_followup(
        &lex_line("They're still lands.", 0).unwrap()
    ));
    assert!(is_destroy_those_creatures_followup(
        &lex_line("Then destroy those creatures.", 0).unwrap()
    ));
}
