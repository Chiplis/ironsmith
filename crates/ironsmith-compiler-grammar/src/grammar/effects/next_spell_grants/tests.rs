use super::*;
use crate::lexer::lex_line;

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

    let from_hand = lex_line(
        "When you next cast an instant or sorcery spell from your hand this turn, it gains rebound.",
        0,
    )
    .unwrap();
    let parsed = parse_next_spell_grant_tokens(&from_hand).unwrap().unwrap();
    assert_eq!(parsed.filters.len(), 1);
    assert_eq!(parsed.filters[0].zone, Some(crate::zone::Zone::Hand));
    assert_eq!(
        parsed.filters[0].stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    assert_eq!(
        parsed.filters[0].cast_by,
        Some(crate::target::PlayerFilter::You)
    );
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

#[test]
fn cast_and_play_timing_surfaces_preserve_distinct_consumption_domains() {
    for (text, surface, card_type) in [
        ("The next sorcery spell you cast this turn can be cast as though it had flash.", NextSpellGrantAbilitySurface::CastTiming, Some(crate::types::CardType::Sorcery)),
        ("The next creature spell you cast this turn can be cast as though it had flash.", NextSpellGrantAbilitySurface::CastTiming, Some(crate::types::CardType::Creature)),
        ("The next creature card you play this turn can be played as though it had flash.", NextSpellGrantAbilitySurface::PlayTiming, Some(crate::types::CardType::Creature)),
        ("The next spell you cast this turn can be cast as though it had flash.", NextSpellGrantAbilitySurface::CastTiming, None),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let parsed = parse_next_spell_grant_tokens(&tokens).unwrap().unwrap();
        assert_eq!(parsed.ability, surface);
        let [filter] = parsed.filters.as_slice() else { panic!("single next selector required"); };
        assert_eq!(filter.card_types, card_type.into_iter().collect::<Vec<_>>());
        assert!(!filter.has_mana_cost, "a play permission can match a creature land without a mana cost");
        assert_eq!(filter.zone, (surface == NextSpellGrantAbilitySurface::CastTiming).then_some(Zone::Stack));
        assert_eq!(filter.stack_kind, (surface == NextSpellGrantAbilitySurface::CastTiming).then_some(StackObjectKind::Spell));
    }
    let chosen = lex_line("The next spell of the chosen type you cast this turn can be cast as though it had flash.", 0).unwrap();
    assert!(parse_next_spell_grant_tokens(&chosen).unwrap().unwrap().filters[0].chosen_creature_type);
    for text in [
        "The next creature card you play this turn can be cast as though it had flash.",
        "The next creature spell you cast this turn can be played as though it had flash.",
        "The next creature card you play this turn has cascade.",
        "The next creature card you play can be played as though it had flash.",
    ] {
        assert!(parse_next_spell_grant_tokens(&lex_line(text, 0).unwrap()).unwrap().is_none(), "{text}");
    }
}
