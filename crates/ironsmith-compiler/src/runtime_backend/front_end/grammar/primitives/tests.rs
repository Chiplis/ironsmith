use super::*;
use crate::runtime_backend::lexer::lex_line;
use winnow::combinator::cut_err;

#[test]
fn words_match_prefix_basic() {
    let tokens = lex_line("target creature gets", 0).unwrap();
    assert!(matches!(
        match_word_prefix(&tokens, &["target", "creature"]),
        Some(_)
    ));
    assert!(match_word_prefix(&tokens, &["target", "gets"]).is_none());
    assert!(matches!(match_word_prefix(&tokens, &[]), Some(_)));
}

#[test]
fn words_match_suffix_basic() {
    let tokens = lex_line("target creature gets", 0).unwrap();
    assert!(match_word_suffix(&tokens, &["creature", "gets"]).is_some());
    assert!(match_word_suffix(&tokens, &["target", "gets"]).is_none());
}

#[test]
fn words_match_any_prefix_skips_leading_non_word_tokens() {
    let tokens = lex_line("\"At the beginning of the end step\"", 0).unwrap();
    let (matched, rest) = match_any_word_prefix(&tokens, &[&["at", "the", "beginning"]]).unwrap();

    assert_eq!(matched, &["at", "the", "beginning"]);
    assert_eq!(
        TokenWordView::new(rest).word_refs(),
        ["of", "the", "end", "step"]
    );
}

#[test]
fn words_split_once_basic() {
    let tokens = lex_line("exile target creature from graveyard", 0).unwrap();
    let (before, after) = words_split_once(&tokens, &["from"]).unwrap();
    assert_eq!(before.len(), 3); // "exile", "target", "creature"
    assert_eq!(after.len(), 1); // "graveyard"
}

#[test]
fn strip_lexed_prefix_phrases_returns_matched_phrase_and_rest() {
    let tokens = lex_line("choose a new target for target spell", 0).unwrap();
    let (matched, rest) = strip_lexed_prefix_phrases(
        &tokens,
        &[
            &["choose", "new", "targets", "for"],
            &["choose", "a", "new", "target", "for"],
        ],
    )
    .unwrap();

    assert_eq!(matched, &["choose", "a", "new", "target", "for"]);
    assert_eq!(TokenWordView::new(rest).word_refs(), ["target", "spell"]);
}

#[test]
fn starts_with_any_phrase_matches_any_prefix_choice() {
    let tokens = lex_line("for each opponent draw a card", 0).unwrap();
    assert!(starts_with_any_phrase(
        &tokens,
        &[
            &["each", "player"],
            &["for", "each", "opponent"],
            &["target", "opponent"],
        ],
    ));
}

#[test]
fn split_lexed_once_before_suffix_finds_prefix_before_full_tail_match() {
    let tokens = lex_line(
        "untap all creatures during each other player's untap step",
        0,
    )
    .unwrap();
    let remainder = match_word_prefix(&tokens, &["untap", "all"]).unwrap();
    let (subject_tokens, ()) = split_lexed_once_before_suffix(remainder, 1, || {
        phrase(&["during", "each", "other", "player's", "untap", "step"])
    })
    .unwrap();
    assert_eq!(
        TokenWordView::new(subject_tokens).word_refs(),
        ["creatures"]
    );
}

#[test]
fn try_parse_all_returns_some_on_full_match() {
    let tokens = lex_line("target creature", 0).unwrap();
    let result = try_parse_all(&tokens, phrase(&["target", "creature"]), "test");
    assert!(result.unwrap().is_some());
}

#[test]
fn try_parse_all_returns_none_on_backtrack() {
    let tokens = lex_line("target creature", 0).unwrap();
    let result = try_parse_all(&tokens, phrase(&["exile", "creature"]), "test");
    assert!(result.unwrap().is_none());
}

#[test]
fn try_parse_all_returns_err_on_trailing_tokens() {
    let tokens = lex_line("target creature gets", 0).unwrap();
    let result = try_parse_all(&tokens, phrase(&["target", "creature"]), "test");
    assert!(result.is_err());
}

#[test]
fn try_parse_all_returns_err_on_cut() {
    let tokens = lex_line("target creature", 0).unwrap();
    let parser = (kw("target").void(), cut_err(kw("opponent")).void()).void();
    let result = try_parse_all(&tokens, parser, "test");
    assert!(result.is_err());
}
