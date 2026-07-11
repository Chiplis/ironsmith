use super::*;
use crate::runtime_backend::lexer::lex_line;

#[test]
fn parses_keyword_station_partner_and_kicker_shapes() {
    let surge = lex_line("Surge {1}{U} (Reminder.)", 0).expect("lex");
    assert!(
        !parse_keyword_body(&surge, &["surge"])
            .expect("surge body")
            .body_tokens
            .is_empty()
    );

    let station = lex_line("8+ | Flying", 0).expect("lex");
    assert_eq!(
        parse_station_threshold(&station)
            .expect("station")
            .threshold,
        8
    );

    let station_reminder =
        lex_line("Station (This artifact is an artifact creature at 12+.)", 0).expect("lex");
    assert_eq!(
        parse_station_creature_threshold(&station_reminder),
        Some(12)
    );

    let partner = lex_line("Partner — Friends forever", 0).expect("lex");
    assert!(parse_partner_variant(&partner).is_some());

    let kicker = lex_line("Kicker {1}{U} and/or {2}{B} (Reminder.)", 0).expect("lex");
    assert!(parse_kicker_branches(&kicker).is_some());

    let ticket_marker = lex_line("{TK}{TK} — Prize sticker", 0).expect("lex");
    assert!(parse_sticker_ticket_marker(&ticket_marker).is_some());
}

#[test]
fn classifies_document_effect_and_static_preference_shapes() {
    let multi = lex_line("Draw a card. Gain 2 life.", 0).expect("lex");
    assert!(parse_multi_sentence_effect_head(&multi).is_some());

    let prevention_then_trigger = lex_line(
        "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage and remove that many +1/+1 counters from it. When one or more counters are removed from this creature this way, it deals that much damage to any target.",
        0,
    )
    .expect("lex");
    let shape = parse_remove_counter_prevention_then_trigger(&prevention_then_trigger)
        .expect("typed prevention followed by trigger");
    assert_eq!(
        TokenWordView::new(shape.prevention_tokens)
            .word_refs()
            .first()
            .copied(),
        Some("if")
    );
    assert_eq!(
        TokenWordView::new(shape.trigger_tokens)
            .word_refs()
            .first()
            .copied(),
        Some("when")
    );

    let draw = lex_line("If you would draw a card, you may mill a card instead.", 0).expect("lex");
    assert_eq!(
        parse_statement_static_preference(&draw),
        Some(StatementStaticPreference::DrawReplacement)
    );

    let discard_or_redirect = lex_line(
        "If Mox Diamond would enter the battlefield, you may discard a land card instead. If you don't, put it into its owner's graveyard.",
        0,
    )
    .expect("lex");
    assert_eq!(
        parse_statement_static_preference(&discard_or_redirect),
        Some(StatementStaticPreference::DiscardOrRedirectReplacement)
    );

    let blocking = lex_line(
        "This creature can block an additional two creatures each combat.",
        0,
    )
    .expect("lex");
    assert_eq!(
        parse_statement_static_preference(&blocking),
        Some(StatementStaticPreference::BlocksAdditionalCreatures)
    );

    let filter_tail = lex_line("red, blue, or green permanents", 0).expect("lex");
    assert!(parse_filter_list_continuation(&filter_tail).is_some());
}
