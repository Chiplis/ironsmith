//! "Until end of turn, double target creature's power X times." (Exponential
//! Growth): the doubling instruction performed X times in a row, each
//! doubling the power the previous one produced (CR 701.10e, 608.2c). The
//! target is chosen once; the repeated body refers to it.
use crate::cards::builders::{CardTextError, EffectAst, ForEachEffectAst, OwnedLexToken};
use crate::effect::Value;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::parser_token_word_refs(tokens);
    if !words.contains(&"double") || words.len() < 4 || words.last() != Some(&"times") {
        return Ok(None);
    }
    let count_token = &tokens[tokens.len() - 2];
    let count = if count_token.is_word("x") {
        Value::X
    } else if let Some(count) = count_token
        .as_word()
        .and_then(crate::util::parse_number_word_u32)
        .filter(|count| *count >= 2)
    {
        Value::Fixed(count as i32)
    } else {
        return Ok(None);
    };
    let body = &tokens[..tokens.len() - 2];
    let effects = super::parse_effect_chain_lexed(body)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
        count,
        effects,
    })]))
}
