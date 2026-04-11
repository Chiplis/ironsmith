use super::*;
use crate::cards::builders::compiler::util::tokenize_line;

#[test]
fn parse_graveyard_owner_prefix_handles_shared_phrases() {
    assert_eq!(
        parse_graveyard_owner_prefix(&["that", "player", "graveyard", "as", "you", "choose"]),
        Some((PlayerAst::That, 3))
    );
    assert_eq!(
        parse_graveyard_owner_prefix(&["its", "owner", "graveyard"]),
        Some((PlayerAst::ItsOwner, 3))
    );
}

#[test]
fn parse_target_player_graveyard_filter_uses_shared_owner_prefix() {
    let tokens = tokenize_line("target opponent graveyard", 0);
    let filter = parse_target_player_graveyard_filter(&tokens).expect("target graveyard");

    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert!(matches!(
        filter.owner,
        Some(PlayerFilter::Target(ref target)) if **target == PlayerFilter::Opponent
    ));
}

#[test]
fn parse_sacrifice_strips_his_or_her_choice_suffix() {
    let tokens = tokenize_line("creature of his or her choice", 0);
    let effect = parse_sacrifice(&tokens, None, None).expect("sacrifice should parse");

    let EffectAst::Sacrifice { filter, count, .. } = effect else {
        panic!("expected sacrifice effect");
    };
    assert_eq!(count, 1);
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.card_types, vec![crate::CardType::Creature]);
}

#[test]
fn parse_or_mana_color_choices_handles_symbol_lists_without_word_view() {
    let tokens = tokenize_line("{W}, {U}, or {B}", 0);

    assert_eq!(
        parse_or_mana_color_choices(&tokens).expect("or-choice mana colors should parse"),
        Some(vec![
            crate::color::Color::White,
            crate::color::Color::Blue,
            crate::color::Color::Black,
        ])
    );
}

#[test]
fn parse_any_combination_mana_colors_handles_symbol_lists_without_word_view() {
    let tokens = tokenize_line("any combination of {W}, {U}, or {R}", 0);

    assert_eq!(
        parse_any_combination_mana_colors(&tokens)
            .expect("any-combination mana colors should parse"),
        Some(vec![
            crate::color::Color::White,
            crate::color::Color::Blue,
            crate::color::Color::Red,
        ])
    );
}

#[test]
fn split_exile_face_down_suffix_keeps_face_down_before_then_clauses() {
    let tokens = tokenize_line("all cards from your library face down,", 0);
    let (prefix, face_down) = split_exile_face_down_suffix(&tokens);

    assert!(face_down);
    assert_eq!(
        crate::cards::builders::compiler::token_word_refs(prefix),
        vec!["all", "cards", "from", "your", "library"]
    );
}
