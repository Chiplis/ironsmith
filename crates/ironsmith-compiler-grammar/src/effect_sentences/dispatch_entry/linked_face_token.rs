//! A conditional linked-face operand, independent of source name and token blueprint.
use super::*;
pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let view=crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    let prefix = ["if", "this", "permanents", "back", "face", "is", "a"];
    // Apostrophes are lexical punctuation; consume their role with the clause.
    let Some(rest) = words.strip_prefix(&prefix) else {
        return Ok(None);
    };
    if rest.len() < 14 {
        return Ok(None);
    }
    let Some(required_subtype) = crate::util::parse_subtype_flexible(rest[0]) else {
        return Ok(None);
    };
    let tail = [
        "create", "a", "token", "thats", "a", "copy", "of", "its", "back", "face", "attached",
        "to",
    ];
    if !rest[1..].starts_with(&tail) {
        return Ok(None);
    }
    // This tail's `attached to` owns the operand boundary. A comparison in
    // that target ("mana value equal to ...") may contain a later `to`.
    let Some(operand_start)=view.token_index_after_words(prefix.len()+1+tail.len()) else {return Ok(None)};
    let attached_to = parse_target_phrase(&tokens[operand_start..])?;
    crate::word_claims::claim_tokens(&tokens[..operand_start]);
    Ok(Some(EffectAst::CreateLinkedFaceToken {
        required_subtype,
        attached_to,
    }))
}
