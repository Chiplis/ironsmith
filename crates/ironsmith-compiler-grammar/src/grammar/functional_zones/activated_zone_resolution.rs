use super::*;

pub fn parse_activated_functional_zones_tokens(
    cost_tokens: &[OwnedLexToken],
    effect_sentences: &[&[OwnedLexToken]],
) -> Vec<Zone> {
    if effect_sentences.iter().any(|sentence| {
        abilities::is_any_player_may_activate_sentence_lexed(sentence)
            && primitives::find_prefix(sentence, || primitives::phrase(&["on", "the", "stack"]))
                .is_some()
    }) {
        return vec![Zone::Stack];
    }

    let cost_words = normalized_activated_zone_words(cost_tokens);
    let effect_words = effect_sentences
        .iter()
        .map(|sentence| normalized_activated_zone_words(sentence))
        .collect::<Vec<_>>();
    let any_effect = |predicate: fn(&[&str]) -> bool| {
        effect_words.iter().any(|words| predicate(words.as_slice()))
    };

    let returns_source_from_graveyard_or_exile = effect_words.iter().any(|words| {
        reference_shapes::contains_source_from_your_graveyard(words)
            && crate::word_primitives::any_sequence_occurs(
                words,
                &[
                    &["graveyard", "or", "from", "exile"],
                    &["graveyard", "or", "exile"],
                ],
            )
    });

    if returns_source_from_graveyard_or_exile {
        vec![Zone::Graveyard, Zone::Exile]
    } else if reference_shapes::contains_source_from_your_graveyard(&cost_words)
        || any_effect(reference_shapes::contains_source_from_your_graveyard)
    {
        vec![Zone::Graveyard]
    } else if reference_shapes::contains_source_from_command_zone(&cost_words)
        || any_effect(reference_shapes::contains_source_from_command_zone)
        || effect_sentences
            .iter()
            .any(|sentence| contains_named_source_command_zone_move(sentence))
    {
        vec![Zone::Command]
    } else if reference_shapes::contains_source_from_your_hand(&cost_words)
        || reference_shapes::contains_discard_source(&cost_words)
        || any_effect(reference_shapes::contains_source_from_your_hand)
    {
        vec![Zone::Hand]
    } else {
        vec![Zone::Battlefield]
    }
}
