use super::super::super::lexer::lex_line;
use super::*;

#[test]
fn fallback_dispatch_kind_accepts_surface_variants() {
    for (text, expected) in [
        ("Aftermath", KeywordFallbackKind::Aftermath),
        (
            "Basic landcycling {2}",
            KeywordFallbackKind::BasicLandcycling,
        ),
        ("Encore {3}{B}", KeywordFallbackKind::Encore),
        ("Jump-start", KeywordFallbackKind::JumpStart),
        ("Jump start", KeywordFallbackKind::JumpStart),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert_eq!(parse_keyword_fallback_kind_tokens(&tokens), Some(expected));
    }
}

#[test]
fn full_dispatch_hint_parser_owns_direct_and_fallback_recognition() {
    for (text, expected) in [
        ("Buyback {2}", KeywordDispatchHint::Buyback),
        ("Basic landcycling {2}", KeywordDispatchHint::Cycling),
        ("Islandcycling {2}", KeywordDispatchHint::Cycling),
        ("Jump-start", KeywordDispatchHint::AlternativeOrExertFamily),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert_eq!(parse_keyword_dispatch_hint_tokens(&tokens), Some(expected));
    }
}

#[test]
fn keyword_prefix_and_special_forms_are_typed() {
    let prefix = lex_line("Freerunning {1}{B}", 0).unwrap();
    assert_eq!(
        parse_keyword_prefix_shape_tokens(&prefix),
        Some(KeywordPrefixShape::Freerunning)
    );

    let blitz = lex_line(
        "You may cast this card from your graveyard using its blitz ability",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_keyword_special_form_shape_tokens(&blitz),
        Some(KeywordSpecialFormShape::BlitzFromGraveyard)
    );

    let sneak = lex_line("It enters tapped and attacking", 0).unwrap();
    assert_eq!(
        parse_keyword_special_form_shape_tokens(&sneak),
        Some(KeywordSpecialFormShape::PermanentSneak)
    );

    let exert = lex_line(
        "If this creature hasn't been exerted this turn, you may exert it as it attacks.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_keyword_special_form_shape_tokens(&exert),
        Some(KeywordSpecialFormShape::ExertAttack)
    );
}
