use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

#[test]
fn parses_standard_pair_and_when_surfaces() {
    let standard = lex_line("The next creature spell you cast this turn has convoke.", 0).unwrap();
    let parsed = parse_next_spell_grant_tokens(&standard).unwrap().unwrap();
    assert_eq!(parsed.player, PlayerAst::You);
    assert_eq!(parsed.filters.len(), 1);

    let pair = lex_line(
        "The next instant spell and the next sorcery spell you cast this turn each have cascade.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_next_spell_grant_tokens(&pair)
            .unwrap()
            .unwrap()
            .filters
            .len(),
        2
    );

    let when = lex_line(
        "When you next cast an artifact spell this turn, it gains sunburst.",
        0,
    )
    .unwrap();
    assert!(parse_next_spell_grant_tokens(&when).unwrap().is_some());
}

#[test]
fn parses_uncounterable_surface() {
    let tokens = lex_line(
        "The next creature spell you cast this turn can't be countered.",
        0,
    )
    .unwrap();
    let parsed = parse_next_spell_grant_tokens(&tokens).unwrap().unwrap();
    assert_eq!(
        parsed.ability,
        NextSpellGrantAbilitySurface::CantBeCountered
    );

    let protection = lex_line("protection from red", 0).unwrap();
    assert!(matches!(
        parse_next_spell_keyword_action_tokens(&protection),
        Some(NextSpellKeywordActionShape::Known(
            KeywordAction::ProtectionFrom(_)
        ))
    ));
}
