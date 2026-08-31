use super::*;

pub fn parse_emblem_action(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let shape = emblem_shapes::parse_emblem_payload_tokens(tokens)?;
    let subject = subject.or_else(|| {
        shape
            .explicit_you
            .then_some(SubjectAst::Player(PlayerAst::You))
    });
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    Some(EffectAst::subject_verb_create_emblem(
        player,
        parse_emblem_description_ast(shape),
    ))
}

/// Parse a quoted emblem followed by an ordinary effect in the same Oracle
/// sentence, such as `You get an emblem with "...", then create ...`.
/// Keeping this boundary at the whole-sentence level prevents commas and
/// sentence punctuation inside the quoted ability from being treated as
/// outer effect separators.
pub fn parse_quoted_emblem_then_action(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let open_quote =
        crate::slice_primitives::select_position(tokens, |token| token.kind == TokenKind::Quote)?;
    let close_quote = open_quote
        + 1
        + crate::slice_primitives::select_position(&tokens[open_quote + 1..], |token| {
            token.kind == TokenKind::Quote
        })?;
    let after_quote = tokens.get(close_quote + 1..).unwrap_or_default();
    let then_offset =
        crate::slice_primitives::select_position(after_quote, |token| token.is_word("then"))?;
    if after_quote[..then_offset]
        .iter()
        .any(|token| !matches!(token.kind, TokenKind::Comma | TokenKind::Period))
    {
        return None;
    }
    let emblem = parse_emblem_action(&tokens[..=close_quote], None)?;
    let trailing =
        crate::lexer::trim_lexed_commas(after_quote.get(then_offset + 1..).unwrap_or_default());
    if trailing.is_empty() {
        return None;
    }
    let mut effects = vec![emblem];
    effects.extend(crate::grammar::primitives::probe_shape(
        crate::effect_sentences::parse_effect_chain_lexed(trailing),
    )?);
    (effects.len() > 1).then_some(EffectAst::Sequence { effects })
}

/// Subject/verb dispatch normally passes only the action tail to `parse_get`.
/// That path has already consumed the quote tokens which delimit an emblem's
/// ability text, so retain a narrow fallback for the resulting `an emblem
/// with ...` shape. The whole-sentence parser remains authoritative whenever
/// the quotes are still present.
pub fn parse_unquoted_emblem_action(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Option<EffectAst> {
    let words = crate::lexer::token_word_refs(tokens);
    if !crate::word_primitives::parse_sequence_prefix(&words, &["an", "emblem", "with"])
        || tokens.len() <= 3
    {
        return None;
    }
    let payload_tokens = tokens.get(3..)?;
    let trailing_then = payload_tokens
        .iter()
        .enumerate()
        .find_map(|(index, token)| {
            if !token.is_word("then") {
                return None;
            }
            let previous = payload_tokens.get(index.saturating_sub(1))?;
            matches!(previous.kind, TokenKind::Comma | TokenKind::Period).then_some(index)
        });
    let (ability_tokens, trailing_tokens) = trailing_then
        .map(|index| {
            (
                &payload_tokens[..index.saturating_sub(1)],
                Some(crate::lexer::trim_lexed_commas(
                    payload_tokens.get(index + 1..).unwrap_or_default(),
                )),
            )
        })
        .unwrap_or((payload_tokens, None));
    let shape = emblem_shapes::EmblemPayloadShape {
        explicit_you: false,
        ability_groups: vec![ability_tokens],
        requires_whole_sentence_dispatch: false,
    };
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let emblem = EffectAst::subject_verb_create_emblem(player, parse_emblem_description_ast(shape));
    let Some(trailing_tokens) = trailing_tokens else {
        return Some(emblem);
    };
    let trailing = crate::grammar::primitives::probe_shape(
        clause_dispatch::parse_effect_clause_lexed(trailing_tokens),
    )?;
    Some(EffectAst::Sequence {
        effects: vec![emblem, trailing],
    })
}
