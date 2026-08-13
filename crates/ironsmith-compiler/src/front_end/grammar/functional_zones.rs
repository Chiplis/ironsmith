use crate::zone::Zone;

use super::super::lexer::OwnedLexToken;
use super::shared_util::reference_shapes;
use super::{abilities, leaf, primitives};

const STATIC_LIBRARY_SEARCH_ZONE_PHRASES: &[&[&str]] = &[
    &["while", "youre", "searching", "your", "library"],
    &["while", "you're", "searching", "your", "library"],
];
const FROM_YOUR_LIBRARY_PHRASE: &[&str] = &["from", "your", "library"];
const CAST_OR_PLAY_SELF_FROM_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["cast", "this", "card", "from", "your", "graveyard"],
    &["cast", "this", "spell", "from", "your", "graveyard"],
    &["cast", "this", "permanent", "from", "your", "graveyard"],
    &["cast", "this", "creature", "from", "your", "graveyard"],
    &["cast", "this", "artifact", "from", "your", "graveyard"],
    &["cast", "this", "enchantment", "from", "your", "graveyard"],
    &["play", "this", "card", "from", "your", "graveyard"],
    &["play", "this", "permanent", "from", "your", "graveyard"],
];
const CAST_OR_PLAY_SELF_FROM_EXILE_PHRASES: &[&[&str]] = &[
    &["cast", "this", "card", "from", "exile"],
    &["play", "this", "card", "from", "exile"],
];
const CAUSES_YOU_TO_DISCARD_THIS_CARD_PHRASE: &[&str] =
    &["causes", "you", "to", "discard", "this", "card"];
const INSTEAD_OF_PUTTING_IT_INTO_YOUR_GRAVEYARD_PHRASE: &[&str] = &[
    "instead",
    "of",
    "putting",
    "it",
    "into",
    "your",
    "graveyard",
];
const SOURCE_NOT_ON_BATTLEFIELD_PHRASES: &[&[&str]] = &[
    &["this", "creature", "isn't", "on", "the", "battlefield"],
    &["this", "permanent", "isn't", "on", "the", "battlefield"],
    &["this", "card", "isn't", "on", "the", "battlefield"],
    &["this", "isn't", "on", "the", "battlefield"],
    &["it", "isn't", "on", "the", "battlefield"],
    &["this", "creature", "isnt", "on", "the", "battlefield"],
    &["this", "permanent", "isnt", "on", "the", "battlefield"],
    &["this", "card", "isnt", "on", "the", "battlefield"],
    &["this", "isnt", "on", "the", "battlefield"],
    &["it", "isnt", "on", "the", "battlefield"],
    &["this", "creature", "is", "not", "on", "the", "battlefield"],
    &["this", "permanent", "is", "not", "on", "the", "battlefield"],
    &["this", "card", "is", "not", "on", "the", "battlefield"],
    &["this", "is", "not", "on", "the", "battlefield"],
    &["it", "is", "not", "on", "the", "battlefield"],
];
const STATIC_ZONE_HINT_PHRASES: &[(&[&str], Zone)] = &[
    (&["this", "card", "is", "in", "your", "hand"], Zone::Hand),
    (
        &["there", "is", "this", "card", "in", "your", "hand"],
        Zone::Hand,
    ),
    (
        &["this", "card", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "card", "is", "in", "a", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "creature", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "permanent", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "object", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["there", "is", "this", "card", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "card", "is", "in", "your", "library"],
        Zone::Library,
    ),
    (
        &["there", "is", "this", "card", "in", "your", "library"],
        Zone::Library,
    ),
    (&["this", "card", "is", "in", "exile"], Zone::Exile),
    (&["there", "is", "this", "card", "in", "exile"], Zone::Exile),
    (
        &["this", "card", "is", "in", "the", "command", "zone"],
        Zone::Command,
    ),
    (
        &[
            "there", "is", "this", "card", "in", "the", "command", "zone",
        ],
        Zone::Command,
    ),
];
const TRIGGER_ZONE_HINT_PHRASES: &[(&[&str], Zone)] = &[
    (&["if", "this", "is", "in", "your", "hand"], Zone::Hand),
    (
        &["if", "this", "card", "is", "in", "your", "hand"],
        Zone::Hand,
    ),
    (
        &["if", "this", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "card", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &[
            "if",
            "this",
            "card",
            "is",
            "the",
            "only",
            "creature",
            "card",
            "in",
            "your",
            "graveyard",
        ],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "creature", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "permanent", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "object", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "is", "in", "your", "library"],
        Zone::Library,
    ),
    (
        &["if", "this", "card", "is", "in", "your", "library"],
        Zone::Library,
    ),
    (&["if", "this", "is", "in", "exile"], Zone::Exile),
    (&["if", "this", "card", "is", "in", "exile"], Zone::Exile),
    (&["if", "this", "card", "is", "exiled"], Zone::Exile),
    (
        &["if", "this", "is", "in", "the", "command", "zone"],
        Zone::Command,
    ),
    (
        &["if", "this", "card", "is", "in", "the", "command", "zone"],
        Zone::Command,
    ),
];
const RETURN_SELF_FROM_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["return", "this", "from", "your", "graveyard"],
    &["return", "this", "card", "from", "your", "graveyard"],
];
const DISCARD_THIS_CARD_PHRASE: &[&str] = &["discard", "this", "card"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TriggerFunctionalZoneFacts {
    pub(crate) explicit_zone: Option<Zone>,
    pub(crate) returns_self_from_graveyard: bool,
    pub(crate) discards_this_card: bool,
}

pub(crate) fn parse_activated_functional_zones_tokens(
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
        words.windows(8).any(|window| {
            window
                == [
                    "return",
                    "this",
                    "card",
                    "from",
                    "your",
                    "graveyard",
                    "or",
                    "from",
                ]
                && words.iter().any(|word| *word == "exile")
        })
    });

    if returns_source_from_graveyard_or_exile {
        vec![Zone::Graveyard, Zone::Exile]
    } else if reference_shapes::contains_source_from_your_graveyard(&cost_words)
        || any_effect(reference_shapes::contains_source_from_your_graveyard)
    {
        vec![Zone::Graveyard]
    } else if reference_shapes::contains_source_from_command_zone(&cost_words)
        || any_effect(reference_shapes::contains_source_from_command_zone)
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

fn normalized_activated_zone_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    primitives::TokenWordView::new(tokens)
        .word_refs()
        .into_iter()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect()
}

fn has_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn has_any_phrase(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    phrases.iter().any(|phrase| has_phrase(tokens, phrase))
}

fn parse_trigger_zone_hint_tokens(tokens: &[OwnedLexToken]) -> Option<Zone> {
    for (phrase, zone) in TRIGGER_ZONE_HINT_PHRASES {
        if has_phrase(tokens, phrase) {
            return Some(zone.clone());
        }
    }
    None
}

pub(crate) fn parse_static_functional_zones_tokens(tokens: &[OwnedLexToken]) -> Option<Vec<Zone>> {
    if has_any_phrase(tokens, SOURCE_NOT_ON_BATTLEFIELD_PHRASES) {
        return Some(vec![
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if has_any_phrase(tokens, STATIC_LIBRARY_SEARCH_ZONE_PHRASES)
        && has_phrase(tokens, FROM_YOUR_LIBRARY_PHRASE)
    {
        return Some(vec![Zone::Library]);
    }
    if has_any_phrase(tokens, CAST_OR_PLAY_SELF_FROM_GRAVEYARD_PHRASES) {
        return Some(vec![Zone::Graveyard]);
    }
    if has_any_phrase(tokens, CAST_OR_PLAY_SELF_FROM_EXILE_PHRASES) {
        return Some(vec![Zone::Exile]);
    }
    if has_phrase(tokens, CAUSES_YOU_TO_DISCARD_THIS_CARD_PHRASE)
        && has_phrase(tokens, INSTEAD_OF_PUTTING_IT_INTO_YOUR_GRAVEYARD_PHRASE)
    {
        return Some(vec![Zone::Hand]);
    }

    let zones = STATIC_ZONE_HINT_PHRASES
        .iter()
        .filter(|(phrase, _)| has_phrase(tokens, phrase))
        .map(|(_, zone)| zone.clone())
        .collect::<Vec<_>>();
    (!zones.is_empty()).then_some(zones)
}

pub(crate) fn parse_trigger_functional_zone_facts_tokens(
    tokens: &[OwnedLexToken],
) -> TriggerFunctionalZoneFacts {
    TriggerFunctionalZoneFacts {
        explicit_zone: parse_trigger_zone_hint_tokens(tokens),
        returns_self_from_graveyard: has_any_phrase(tokens, RETURN_SELF_FROM_GRAVEYARD_PHRASES),
        discards_this_card: has_phrase(tokens, DISCARD_THIS_CARD_PHRASE),
    }
}

#[cfg(test)]
mod tests;
