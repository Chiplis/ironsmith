//! Typed quality names chosen during resolution, independent of an action.
use crate::cards::builders::{EffectAst, PlayerAst, PredicateAst, SourcePredicateAst};
use crate::lexer::{OwnedLexToken, TokenKind, parser_token_word_refs};
fn quality(word: &str) -> Option<String> {
    let raw = word.strip_prefix("non").unwrap_or(word);
    crate::util::parse_card_type(raw)?;
    Some(word.to_ascii_lowercase())
}
fn words(tokens: &[OwnedLexToken]) -> Option<Vec<&str>> {
    if tokens.iter().any(|token| !matches!(token.kind, TokenKind::Word | TokenKind::Comma | TokenKind::Period)) { return None; }
    Some(parser_token_word_refs(tokens))
}
pub(crate) fn parse_choice(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = words(tokens)?;
    let words = words.strip_prefix(&["you"]).unwrap_or(&words);
    let ["choose", first, "or", second] = words else { return None; };
    let options = vec![quality(first)?, quality(second)?];
    if options[0] == options[1] { return None; }
    crate::word_claims::claim_tokens(tokens);
    Some(EffectAst::subject_verb_choose_named_option(PlayerAst::You, options))
}
pub(crate) fn parse_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let words = words(tokens)?;
    let words = words.strip_prefix(&["if"]).unwrap_or(&words);
    let [option, "was", "chosen"] = words else { return None; };
    let option = quality(option)?;
    crate::word_claims::claim_tokens(tokens);
    Some(PredicateAst::Source(SourcePredicateAst::SourceChosenOption(option)))
}
