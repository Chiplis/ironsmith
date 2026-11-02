//! A universal object predicate is zero matching counterexamples, including
//! the empty set. Keyword identity is a bounded identity operand, never rules text.
use super::*;

pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if !tokens.first().is_some_and(|token| token.is_word("each")) { return Ok(None); }
    let Some(has) = tokens.iter().position(|token| token.is_word("has")) else { return Ok(None); };
    if has < 2 || !tokens.get(has + 1).is_some_and(|token| token.is_word("custom"))
        || !tokens.get(has + 2).is_some_and(|token| token.is_word("keyword")) { return Ok(None); }
    let name = &tokens[has + 3..];
    let name = if name.first().is_some_and(|t|t.kind == crate::lexer::TokenKind::Quote) {
        if name.len()<3 || name.last().is_none_or(|t|t.kind != crate::lexer::TokenKind::Quote) {
            return Err(CardTextError::ParseError("universal custom keyword condition requires a complete identity".into()));
        }
        &name[1..name.len()-1]
    } else {name};
    if name.is_empty() || name.iter().any(|t|t.kind != crate::lexer::TokenKind::Word) {
        return Err(CardTextError::ParseError("universal custom keyword condition requires one complete identity".into()));
    }
    let name = crate::lexer::render_token_slice(name).trim().to_ascii_lowercase();
    if name.is_empty() || name.len()>64 {return Err(CardTextError::ParseError("invalid custom keyword identity operand".into()));}
    let mut filter = crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens[1..has],false)?;
    if filter.zone.is_none() {filter.zone=Some(Zone::Battlefield);}
    filter.excluded_ability_markers.push(name);
    crate::word_claims::claim_tokens(&tokens[..1]);
    crate::word_claims::claim_tokens(&tokens[has..]);
    Ok(Some(PredicateAst::CountComparison {
        count: AnthemCountExpression::MatchingFilter(filter),
        comparison: crate::effect::Comparison::Equal(0),
        display: None,
    }))
}
