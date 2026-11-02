//! Explicit operation-scoped token ability grants; labels are opaque data.
use super::*;
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let quotes = tokens.iter().enumerate().filter(|(_, token)| token.is_quote()).map(|(index, _)| index).collect::<Vec<_>>();
    let [open, close] = quotes.as_slice() else { return Ok(None); };
    let words = crate::lexer::token_word_refs(&tokens[..*open]);
    if !words.starts_with(&["as", "you", "perform", "the"])
        || !words.ends_with(&["action", "tokens", "you", "create", "gain"])
        || words.len() <= 9
        || tokens[*close + 1..].iter().any(|token| token.as_word().is_some())
    { return Ok(None); }
    let label = words[4..words.len()-5].join(" ");
    let (abilities, _) = parse_granted_abilities_for_gain_clause(&tokens[*open..=*close], &words, false)?;
    if abilities.is_empty() { return Err(CardTextError::ParseError("operation token grant has no ordinary ability".into())); }
    crate::word_claims::claim_tokens(tokens);
    Ok(Some(StaticAbilityAst::NamedActionTokenAbilityGrant {
        label, player: PlayerFilter::You, abilities,
        display: crate::lexer::render_token_slice(tokens),
    }))
}
