use super::*;
use crate::runtime_backend::lexer::lex_line;

#[test]
fn classifies_reveal_life_and_counter_reference_surfaces() {
    let reveal = lex_line("that card", 0).expect("lex");
    assert_eq!(
        parse_reveal_reference(&reveal),
        Some(RevealReferenceShape::Tagged)
    );

    assert_eq!(
        parse_life_equal_surface(&["equal", "to", "life", "lost", "this", "way"]),
        Some(LifeEqualSurface::LifeLostThisWay)
    );

    let counters = lex_line("for each charge counter on this creature", 0).expect("lex");
    assert!(matches!(
        parse_counter_reference(&counters),
        Some(CounterReferenceShape::Source { .. })
    ));
}

#[test]
fn returns_typed_payment_possessive_and_half_life_shapes() {
    let payment = lex_line("plus an additional {2} for each creature", 0).expect("lex");
    let payment = parse_additional_payment_head(&payment).expect("payment head");
    assert_eq!(payment.multiplier_token.parser_text(), "{2}");
    assert_eq!(
        TokenWordView::new(payment.filter_tokens).word_refs(),
        ["for", "each", "creature"]
    );

    let possessive = lex_line("target creature's power", 0).expect("lex");
    let possessive = parse_possessive_target_stat(&possessive).expect("possessive stat");
    assert_eq!(possessive.stat, TargetStatKind::Power);
    assert_eq!(
        TokenWordView::new(&possessive.target_tokens).word_refs(),
        ["target", "creature"]
    );

    assert_eq!(
        parse_half_life(&["half", "your", "life", "rounded", "down"]),
        Some(HalfLifeShape { rounded_down: true })
    );

    assert_eq!(
        parse_life_total_as_turn_began_words(&[
            "target",
            "opponent's",
            "life",
            "total",
            "as",
            "the",
            "turn",
            "began",
        ]),
        Some(Value::LifeTotalAsTurnBegan(PlayerFilter::target_opponent()))
    );
}
