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
/// An authored option name ("honor", "condemn"): a single plain word that is
/// not grammar, a number, a color, a zone, a type or a parity/direction word
/// owned by another choice reading. Names are data, never card text.
fn named_option(word: &str) -> Option<String> {
    if word.len() < 3 || !word.chars().all(|ch| ch.is_ascii_alphabetic()) { return None; }
    if matches!(word, "the" | "one" | "two" | "three" | "target" | "another" | "each" | "any" | "all"
        | "that" | "this" | "those" | "these" | "them" | "it" | "odd" | "even" | "left" | "right"
        | "heads" | "tails" | "more" | "less" | "fewer" | "both" | "neither" | "either" | "not"
        | "new" | "mode" | "modes" | "number" | "player" | "opponent" | "card" | "cards"
        | "spell" | "spells" | "permanent" | "permanents" | "token" | "tokens" | "color" | "colors")
    {
        return None;
    }
    if crate::util::parse_number_word_i32(word).is_some()
        || crate::util::parse_color(word).is_some()
        || crate::util::parse_zone_word(word).is_some()
        || crate::util::parse_card_type(word.strip_prefix("non").unwrap_or(word)).is_some()
    {
        return None;
    }
    Some(word.to_ascii_lowercase())
}
pub(crate) fn parse_choice(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = words(tokens)?;
    let words = words.strip_prefix(&["you"]).unwrap_or(&words);
    let options = match words {
        ["choose", first, "or", second] => match (quality(first), quality(second)) {
            (Some(first), Some(second)) => vec![first, second],
            _ => vec![named_option(first)?, named_option(second)?],
        },
        // "You choose actor, cleric or scout": a longer list of names.
        ["choose", list @ .., "or", last] if list.len() >= 2 => {
            let mut options = list.iter().map(|word| named_option(word)).collect::<Option<Vec<_>>>()?;
            options.push(named_option(last)?);
            options
        }
        _ => return None,
    };
    if (1..options.len()).any(|index| options[..index].contains(&options[index])) { return None; }
    crate::word_claims::claim_tokens(tokens);
    Some(EffectAst::subject_verb_choose_named_option(PlayerAst::You, options))
}
pub(crate) fn parse_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let words = words(tokens)?;
    let words = words.strip_prefix(&["if"]).unwrap_or(&words);
    let option = match words {
        [option, "was", "chosen"] => quality(option)?,
        // "If you chose honor": the same source-chosen named option.
        ["you", "chose", option] => quality(option).or_else(|| named_option(option))?,
        _ => return None,
    };
    crate::word_claims::claim_tokens(tokens);
    Some(PredicateAst::Source(SourcePredicateAst::SourceChosenOption(option)))
}
