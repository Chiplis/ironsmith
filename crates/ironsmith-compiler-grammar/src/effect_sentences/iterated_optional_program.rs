//! Preserve an object's optional action and its result-dependent consequence
//! inside the same iteration. A numerical repeat loses the bound object and
//! an outer result test would instead inspect the aggregate loop outcome.
use crate::cards::builders::{CardTextError, EffectAst, ForEachEffectAst};
use crate::lexer::OwnedLexToken;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    for (index, pair) in sentences.windows(2).enumerate() {
        let Some(shape) = crate::word_claims::probe_only(|| {
            crate::grammar::effects::for_each_shapes::parse_for_each_object_effect_shape(&pair[0])
        }) else { continue; };
        let action = crate::lexer::parser_token_word_refs(shape.effect_tokens);
        let followup = crate::lexer::parser_token_word_refs(&pair[1]);
        if !action.starts_with(&["you", "may"])
            || !(followup.starts_with(&["if", "you", "do"])
                || followup.starts_with(&["if", "you", "don't"])
                || followup.starts_with(&["if", "you", "dont"]))
        { continue; }
        // Characteristic loops (colors/card types) have other owners. Probe
        // the object domain before committing this reader or its claims.
        if crate::word_claims::probe_only(|| {
            super::for_each_helpers::parse_for_each_object_filter(shape.filter_tokens)
        }).is_err() { continue; }
        let shape = crate::grammar::effects::for_each_shapes::parse_for_each_object_effect_shape(&pair[0])
            .expect("the object iteration shape was proven above");
        let filter = super::for_each_helpers::parse_for_each_object_filter(shape.filter_tokens)?;
        let body_tokens = crate::util::join_sentences_with_period(&[
            shape.effect_tokens.to_vec(), pair[1].to_vec(),
        ]);
        let body = super::parse_effect_sentences_lexed(&body_tokens)?;
        let mut result = if index == 0 { Vec::new() } else {
            super::parse_effect_sentences_lexed(&crate::util::join_sentences_with_period(
                &sentences[..index].iter().map(|sentence| sentence.to_vec()).collect::<Vec<_>>(),
            ))?
        };
        result.push(EffectAst::ForEach(ForEachEffectAst::ForEachObject { filter, effects: body }));
        if index + 2 < sentences.len() {
            result.extend(super::parse_effect_sentences_lexed(
                &crate::util::join_sentences_with_period(
                    &sentences[index + 2..].iter().map(|sentence| sentence.to_vec()).collect::<Vec<_>>(),
                ),
            )?);
        }
        return Ok(Some(result));
    }
    Ok(None)
}
