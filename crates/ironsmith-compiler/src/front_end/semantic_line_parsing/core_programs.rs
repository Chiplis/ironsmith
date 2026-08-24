use super::*;

/// A cross-sentence semantic rewrite can make prefix equality fail even when
/// a later sentence's distributive subject survives unchanged. In that flat
/// fallback, retain the explicit authored `Then for each ...` connective on
/// the matching typed filter rather than losing it with the sentence wrapper.
pub(super) fn preserve_flat_leading_then_for_each_surface(
    sentences: &[Vec<OwnedLexToken>],
    mut effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    for sentence in sentences {
        let words = token_word_refs(sentence);
        if !words.get(..3).is_some_and(|prefix| {
            prefix[0].eq_ignore_ascii_case("then")
                && prefix[1].eq_ignore_ascii_case("for")
                && prefix[2].eq_ignore_ascii_case("each")
        }) {
            continue;
        }
        let sentence_effects = parse_effect_sentences_lexed(sentence).or_else(|_| {
            // Some isolated sentence parsers receive the connective only
            // from the multi-sentence dispatcher. The body after `Then`
            // carries the same typed distributive filter.
            parse_effect_sentences_lexed(&sentence[1..])
        });
        let Ok(sentence_effects) = sentence_effects else {
            continue;
        };
        let Some(filter) = first_for_each_object_filter(&sentence_effects) else {
            continue;
        };
        mark_matching_for_each_object_leading_then(&mut effects, &filter);
    }
    effects
}
