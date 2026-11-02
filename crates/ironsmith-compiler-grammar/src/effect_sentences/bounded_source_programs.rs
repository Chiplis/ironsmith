//! Explicit scope boundaries for existing conditional and local-X programs.
use super::*;
use crate::cards::builders::{CardTextError, EffectAst};
pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(colon) = tokens.iter().position(OwnedLexToken::is_colon) else { return Ok(None); };
    let header = &tokens[..colon];
    let words = header.iter().filter_map(OwnedLexToken::as_word).collect::<Vec<_>>();
    if words == ["with", "x", "equal", "to", "the", "number", "of", "objects", "chosen", "this", "way", "do", "the", "following"]
        && header.iter().filter(|token| token.kind == crate::lexer::TokenKind::Comma).count() == 1
        && header.iter().all(|token| token.as_word().is_some() || token.kind == crate::lexer::TokenKind::Comma)
    {
        let effects = super::parse_bounded_program(&tokens[colon + 1..])?;
        if effects.is_empty() { return Ok(None); }
        crate::word_claims::claim_tokens(&tokens[..=colon]);
        // Reference resolution pins the selected-set alias to the exact local
        // choice's published tag. Count its historical packet, never the last
        // tap/exile action's affected count or a pre-existing outer X.
        let value = crate::effect::Value::Count(crate::target::ObjectFilter::tagged(
            crate::tag::CompilerReferenceTag::ChosenObjects.bind()));
        return Ok(Some(vec![EffectAst::BindX { value, effects }]));
    }
    if header.first().is_some_and(|token| token.is_word("unless")) {
        let Some(comma) = header.iter().position(|token| token.kind == crate::lexer::TokenKind::Comma) else { return Ok(None); };
        let tail = header[comma + 1..].iter().map(OwnedLexToken::as_word).collect::<Option<Vec<_>>>();
        if tail.as_deref() != Some(&["do", "the", "following"]) { return Ok(None); }
        let (player, cost) = super::clause_primitives::parse_unless_pays_clause(&header[1..comma])?;
        let effects = super::parse_bounded_program(&tokens[colon + 1..])?;
        if effects.is_empty() { return Ok(None); }
        crate::word_claims::claim_tokens(&tokens[..1]);
        crate::word_claims::claim_tokens(&header[comma..]);
        crate::word_claims::claim_tokens(&tokens[colon..=colon]);
        return Ok(Some(vec![EffectAst::Conditionals(crate::cards::builders::ConditionalEffectAst::UnlessPays {
            effects, player, cost, before_delayed_step: false,
        })]));
    }
    Ok(None)
}

/// Instruction colons cannot become activation-price delimiters. Inspect only
/// the first unquoted colon; an actual earlier activation price keeps ownership.
pub(crate) fn has_bounded_header(tokens: &[OwnedLexToken]) -> bool {
    if tokens.first().is_some_and(|token| token.is_any_word(&["when", "whenever", "at"])) { return false; }
    let mut quoted = false;
    let Some(colon) = tokens.iter().enumerate().find_map(|(index,token)| {
        if token.kind == crate::lexer::TokenKind::Quote { quoted = !quoted; }
        (!quoted && token.is_colon()).then_some(index)
    }) else { return false; };
    let prefix = &tokens[..colon];
    let words = prefix.iter().filter_map(OwnedLexToken::as_word).collect::<Vec<_>>();
    if tokens.get(colon+1).is_some_and(|token| token.kind == crate::lexer::TokenKind::LBracket)
        && (words == ["do", "the", "following"]
            || words.first() == Some(&"if") && words.ends_with(&["do", "the", "following"])
                && prefix.len() >= 4 && prefix[prefix.len()-4].kind == crate::lexer::TokenKind::Comma)
    { return true; }
    let binding = ["with", "x", "equal", "to", "the", "number", "of", "objects", "chosen", "this", "way", "do", "the", "following"];
    let sentence_start = prefix.iter().rposition(|token| token.kind == crate::lexer::TokenKind::Period)
        .map_or(0, |index| index + 1);
    let instruction = &prefix[sentence_start..];
    let quantified = crate::lexer::parser_token_word_refs(instruction)
        .starts_with(&["for", "each"]);
    words.starts_with(&["with", "x", "equal", "to"]) && words.ends_with(&["do", "the", "following"])
        || words.ends_with(&binding) || words.ends_with(&["do", "the", "following"])
        && (prefix.iter().any(|token| token.is_word("unless")) || quantified)
}

/// An explicit quoted payment admits a resolving statement, independently of
/// whether its opaque body contains a colon or a conventional effect verb.
pub(crate) fn has_optional_cost_program_head(tokens:&[OwnedLexToken])->bool{
    if !tokens.first().is_some_and(|token|token.is_any_word(&["you","they","that","its"])){return false;}
    let Some(may)=tokens.iter().position(|token|token.is_word("may")) else{return false;};
    may>0 && may<=3 && tokens.len()>may+4
        && tokens[may+1..may+4].iter().zip(["pay","the","cost"]).all(|(token,word)|token.is_word(word))
        && tokens[may+4].kind==crate::lexer::TokenKind::Quote
}

#[cfg(test)]
mod quantified_colon_regressions {
    use super::*;
    #[test]
    fn quantified_instruction_after_previous_sentence_keeps_statement_ownership() {
        let tokens = crate::lexer::lex_line("Exile all cards from your graveyard. For each card exiled this way, you may do the following: draw a card.", 0).unwrap();
        assert!(has_bounded_header(&tokens));
        let activation = crate::lexer::lex_line("{2}: For each card exiled this way, you may do the following: draw a card.", 0).unwrap();
        assert!(!has_bounded_header(&activation));
    }
}
