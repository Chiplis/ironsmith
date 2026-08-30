use super::*;

/// Preserve an ordered token-creation/spell-copy pair after document
/// sentence normalization has consumed the comma before `then`.
///
/// Both arms must independently lower to the typed actions, so an unrelated
/// descriptive `then` tail cannot be mistaken for a second effect.
pub(super) fn parse_create_token_then_copy_spell_chain(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(then_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("then"))
    else {
        return Ok(None);
    };
    let create_tokens = trim_edge_punctuation(&tokens[..then_idx]);
    let copy_tokens = trim_edge_punctuation(&tokens[then_idx + 1..]);
    if !create_tokens
        .first()
        .is_some_and(|token| token.is_word("create"))
        || !copy_tokens
            .first()
            .is_some_and(|token| token.is_word("copy"))
    {
        return Ok(None);
    }

    // Parse the two grammar-proven arms directly. The general chain parser is
    // allowed to normalize and wrap an isolated arm for carry semantics, but
    // this specialist needs one exact producer followed by one exact copy.
    // The leading-token guards above make the dedicated action parsers the
    // narrowest reusable boundary for this ordered pair.
    let create_effect = super::super::creation_handlers::parse_create(&create_tokens[1..], None)?;
    if !matches!(
        &create_effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CreateTokenWithMods { .. },
            ..
        })
    ) {
        return Ok(None);
    }
    let Some(copy_effect) =
        super::super::clause_pattern_helpers::parse_copy_spell_clause(&copy_tokens)?
    else {
        return Ok(None);
    };
    if !matches!(
        copy_effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell { .. },
            ..
        })
    ) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::CommaThen {
        effects: vec![create_effect, copy_effect],
    }]))
}
