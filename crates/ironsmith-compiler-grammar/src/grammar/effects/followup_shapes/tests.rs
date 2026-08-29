use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_create_more_prior_token_shape() {
    let tokens = lex_line("If you do, create two of those tokens instead.", 0).unwrap();
    let shape = parse_create_more_prior_tokens(&tokens).expect("shape");
    assert_eq!(shape.count, 2);
    assert!(!shape.predicate_tokens.is_empty());
    assert!(shape.instead);

    let additive = lex_line("If you do, create two of those tokens.", 0).unwrap();
    assert!(!parse_create_more_prior_tokens(&additive).unwrap().instead);
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

    let tokens = lex_line(
        "If you win, that creature gets an additional +2/+2 and gains trample until end of turn.",
        0,
    )
    .unwrap();
    let shape = parse_conditional_followup(&tokens).expect("clash result shape");
    assert_eq!(shape.kind, ConditionalFollowupKind::IfYouWin);
    assert!(!shape.continuation_tokens.is_empty());

    let clash = lex_line("If you win the clash, draw a card.", 0).unwrap();
    let shape = parse_conditional_followup(&clash).expect("explicit clash result shape");
    assert_eq!(shape.kind, ConditionalFollowupKind::IfYouWinClash);

    let flip = lex_line("If you win the flip, draw a card.", 0).unwrap();
    let shape = parse_conditional_followup(&flip).expect("explicit coin-flip result shape");
    assert_eq!(shape.kind, ConditionalFollowupKind::IfYouWinFlip);

    let game = lex_line("If you win the game, draw a card.", 0).unwrap();
    assert!(parse_conditional_followup(&game).is_none());
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

#[test]
fn parses_moved_object_entry_modifier_followup_only_with_temporary_grant() {
    let tokens = lex_line(
        "It enters tapped and attacking and gains indestructible until end of turn.",
        0,
    )
    .unwrap();
    let shape = parse_moved_object_entry_followup_shape(&tokens).expect("typed follow-up");
    assert!(tokens[shape.grant_verb_token_idx].is_word("gains"));

    for near_miss in [
        "It enters tapped and attacking.",
        "It enters tapped and gains indestructible until end of turn.",
        "It enters tapped and attacking and gains indestructible.",
        "This creature enters tapped and attacking and gains indestructible until end of turn.",
    ] {
        let tokens = lex_line(near_miss, 0).unwrap();
        assert!(
            parse_moved_object_entry_followup_shape(&tokens).is_none(),
            "near miss must not claim {near_miss:?}"
        );
    }
}

#[test]
fn parses_counter_linked_land_subtype_followup_to_typed_facts() {
    let tokens = lex_line(
        "That land is an Island in addition to its other types for as long as it has a flood counter on it.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_counter_linked_land_subtype_followup(&tokens),
        Some(CounterLinkedLandSubtypeFollowupShape {
            subtype: crate::types::Subtype::Island,
            counter_type: crate::object::CounterType::Flood,
        })
    );
}
