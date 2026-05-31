#![allow(dead_code)]

use crate::cards::builders::{CardTextError, ChoiceCount};
use crate::color::Color;
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::types::{CardType, Subtype};

use super::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::effect_sentences::parse_subtype_word;
use super::grammar::primitives::TokenWordView;
use super::grammar::values::{
    parse_count_word_tokens, parse_mana_cost_tokens, parse_mana_symbol, parse_mana_symbol_group,
};
use super::lexer::{
    OwnedLexToken, TokenKind, lex_line, render_token_slice, token_slice_at_is,
    token_slice_first_is, word_slice_at_is_any, word_slice_contains_word, word_slice_ends_with,
    word_slice_eq, word_slice_eq_any, word_slice_find_window_by, word_slice_first_is,
    word_slice_first_is_any, word_slice_last_is_any, word_slice_starts_with,
};
use super::object_filters::parse_object_filter_lexed;
use super::token_primitives::{
    find_index as find_token_index, str_ends_with, str_starts_with, str_strip_prefix,
    str_strip_suffix,
};
use super::util::{parse_card_type, parse_counter_type_from_tokens, parse_number};

const NAMED_ARTIFACTS_YOU_CONTROL_MARKERS: &[&[&str]] = &[
    &["and", "artifacts", "you", "control", "named"],
    &["and", "artifact", "you", "control", "named"],
];
const THE_TOP_SOURCE_WORDS: &[&str] = &["the", "top"];
const RETURN_TO_OWNER_HAND_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["to", "its", "owners", "hand"],
            &["to", "their", "owners", "hand"],
        ]
);
const AMONG_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["among"]);
const X_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["x"]);
const ALL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["all"]);
const ANY_NUMBER_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["any", "number", "of"]);
const ONE_OR_MORE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["one", "or", "more"]);
const REVEAL_THIS_CARD_FROM_HAND_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["reveal", "this", "card", "from", "your", "hand"]);
const REVEAL_THIS_SOURCE_TYPE_FROM_HAND_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["reveal", "this"]; suffix & ["from", "your", "hand"]);
const GENERIC_COUNTER_DESCRIPTOR_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const SPELL_OR_SPELLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["spell"], &["spells"]]);
const COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const OTHER_OR_ANOTHER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["another"], &["other"]]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActivationCostCst {
    pub(crate) raw: String,
    pub(crate) segments: Vec<ActivationCostSegmentCst>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ActivationCostSegmentCst {
    Mana(ManaCost),
    Tap,
    TapChosen {
        count: u32,
        filter_text: String,
        other: bool,
    },
    Untap,
    Life(u32),
    Energy(u32),
    DiscardSource,
    DiscardHand,
    DiscardCard(u32),
    DiscardFiltered {
        count: u32,
        card_types: Vec<CardType>,
        random: bool,
        name: Option<String>,
        other: bool,
    },
    Mill(u32),
    SacrificeSelf,
    SacrificeCreature,
    SacrificeChosen {
        count: u32,
        up_to: bool,
        filter_text: String,
        other: bool,
    },
    ExileSelf,
    ExileSelfFromGraveyard,
    ExileFromHand {
        count: u32,
        color_filter: Option<ColorSet>,
    },
    ExileFromGraveyard {
        count: u32,
        card_type: Option<CardType>,
    },
    ExileChosen {
        choice_count: ChoiceCount,
        filter_text: String,
    },
    ExileSelfAndNamedArtifacts {
        names: Vec<String>,
    },
    ExileTopLibrary {
        count: u32,
    },
    RevealSourceFromHand,
    ReturnSelfToHand,
    ReturnChosenToHand {
        count: u32,
        filter_text: String,
    },
    MoveOpponentOwnedExiledCardToGraveyard,
    ExertSelf {
        display_text: String,
    },
    PutCounters {
        counter_type: CounterType,
        count: u32,
    },
    PutCountersChosen {
        counter_type: CounterType,
        count: u32,
        filter_text: String,
    },
    Blight {
        count: u32,
    },
    RemoveCounters {
        counter_type: CounterType,
        count: u32,
    },
    RemoveCountersAmong {
        counter_type: Option<CounterType>,
        count: u32,
        filter_text: String,
        display_x: bool,
        dynamic: bool,
    },
    RemoveCountersDynamic {
        counter_type: Option<CounterType>,
        display_x: bool,
        remove_all: bool,
    },
    Behold {
        subtype: Subtype,
        count: u32,
    },
}

type LeafCompatWords<'a> = TokenWordView<'a>;

const LEAF_ONE_OR_MORE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["one", "or", "more"]);
const LEAF_ANY_NUMBER_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["any", "number", "of"]);
const LEAF_X_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["x"]);
const LEAF_ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["the"]]);
const LEAF_A_OR_AN_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"]]);
const LEAF_YOUR_HAND_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["your", "hand"]);
const LEAF_THIS_CARD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "card"]);
const LEAF_OTHER_OR_ANOTHER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["another"], &["other"]]);
const LEAF_CARD_NAMED_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["card", "named"]);
const LEAF_CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const LEAF_AND_OR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"]]);
const LEAF_AT_RANDOM_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["at", "random"]);
const LEAF_A_CREATURE_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["a", "creature"]);
const LEAF_UP_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["up", "to"]);
const LEAF_ON_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["on"]);
const LEAF_FROM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["from"]);
const LEAF_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const LEAF_ZERO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["0"]);
const LEAF_PAY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["pay"]);
const LEAF_LIFE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["life"], &["lives"]]);
const LEAF_EXERT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["exert"]);
const LEAF_MILL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["mill"]);
const LEAF_BEHOLD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["behold"]);
const LEAF_BLIGHT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["blight"]);
const LEAF_UNTAPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["untapped"]);
const LEAF_TARGET_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["target"]);
const LEAF_MOVE_OPPONENT_OWNED_EXILED_CARD_TO_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "put",
                "a",
                "card",
                "an",
                "opponent",
                "owns",
                "from",
                "exile",
                "into",
                "that",
                "players",
                "graveyard",
            ],
            &[
                "put",
                "a",
                "card",
                "an",
                "opponent",
                "owns",
                "from",
                "exile",
                "into",
                "that",
                "player's",
                "graveyard",
            ],
        ]
);
const LEAF_FROM_YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["from", "your", "graveyard"]);
const LEAF_TOP_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["the", "top"];
    suffix_any & [&["cards", "of", "your", "library"], &["card", "of", "your", "library"]]
);
const LEAF_FROM_YOUR_HAND_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["from", "your", "hand"]);
const LEAF_FROM_YOUR_GRAVEYARD_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["from", "your", "graveyard"]);
const LEAF_SOURCE_SELF_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["this"],
            &["this", "creature"],
            &["this", "artifact"],
            &["this", "aura"],
            &["this", "enchantment"],
            &["this", "equipment"],
            &["this", "fortification"],
            &["this", "land"],
            &["this", "permanent"],
            &["this", "card"],
        ]
);
const LEAF_EXILE_SELF_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this"],
            &["this", "card"],
            &["this", "spell"],
            &["this", "permanent"],
            &["this", "creature"],
            &["this", "artifact"],
            &["this", "enchantment"],
            &["this", "land"],
            &["this", "aura"],
            &["this", "vehicle"],
        ]
);
const LEAF_EXILE_SELF_FROM_YOUR_ZONE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "card", "from", "your"],
            &["this", "spell", "from", "your"],
            &["this", "creature", "from", "your"],
            &["this", "artifact", "from", "your"],
            &["this", "enchantment", "from", "your"],
            &["this", "land", "from", "your"],
            &["this", "aura", "from", "your"],
            &["this", "vehicle", "from", "your"],
        ]
);
const LEAF_RETURN_SELF_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["this"],
            &["this", "card"],
            &["this", "permanent"],
            &["this", "creature"],
            &["this", "artifact"],
            &["this", "enchantment"],
            &["this", "land"],
        ]
);
const LEAF_COUNTER_SELF_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "artifact"],
            &["this", "aura"],
            &["this", "card"],
            &["this", "land"],
        ]
);
const LEAF_COUNTER_REMOVAL_SELF_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "artifact"],
            &["this", "enchantment"],
            &["this", "card"],
            &["this", "land"],
            &["it"],
        ]
);

fn token_slice_is_symbol(token: &OwnedLexToken, symbol: &str) -> bool {
    token.slice.eq_ignore_ascii_case(symbol)
}

fn token_word_is_symbol(token: &OwnedLexToken, symbol: &str) -> bool {
    token
        .as_word()
        .is_some_and(|word| word.eq_ignore_ascii_case(symbol))
}

fn is_energy_symbol_token(token: &OwnedLexToken) -> bool {
    match token.kind {
        TokenKind::ManaGroup => token_slice_is_symbol(token, "{e}"),
        TokenKind::Word | TokenKind::Number => token_word_is_symbol(token, "e"),
        _ => false,
    }
}

fn is_tap_symbol_token(token: &OwnedLexToken) -> bool {
    token_word_is_symbol(token, "t") || token_slice_is_symbol(token, "{t}")
}

fn is_untap_symbol_token(token: &OwnedLexToken) -> bool {
    token_word_is_symbol(token, "q") || token_slice_is_symbol(token, "{q}")
}

fn is_reserved_activation_symbol_token(token: &OwnedLexToken) -> bool {
    is_energy_symbol_token(token) || is_tap_symbol_token(token) || is_untap_symbol_token(token)
}

fn parse_filter_text(text: &str, other: bool) -> Result<ObjectFilter, CardTextError> {
    let tokens = lex_line(text, 0)?;
    parse_object_filter_lexed(&tokens, other)
}

fn filter_text_mentions_spell(text: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_alphabetic())
        .any(|word| SPELL_OR_SPELLS_WORD_PATTERN.matches_word(word))
}

fn parse_card_type_word(word: &str) -> Option<CardType> {
    parse_card_type(&word.to_ascii_lowercase())
}

fn parse_color_word(word: &str) -> Option<ColorSet> {
    Color::from_name(word).map(ColorSet::from_color)
}

fn first_non_comma_token(tokens: &[OwnedLexToken]) -> Option<&OwnedLexToken> {
    for token in tokens {
        if !token.is_comma() {
            return Some(token);
        }
    }
    None
}

fn first_non_comma_token_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    crate::slice_primitives::find_index(tokens, |token| !token.is_comma())
}

fn trim_activation_cost_segment_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = first_non_comma_token_index(tokens).unwrap_or(tokens.len());
    let mut end = tokens.len();

    if token_slice_at_is(tokens, start, "and") {
        start += 1;
        while start < end && tokens[start].is_comma() {
            start += 1;
        }
    }

    if token_slice_at_is(tokens, start, "waterbend") {
        start += 1;
        while start < end && tokens[start].is_comma() {
            start += 1;
        }
    }

    while end > start && (tokens[end - 1].is_period() || tokens[end - 1].is_comma()) {
        end -= 1;
    }

    &tokens[start..end]
}

fn render_trimmed_lexed_tokens(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens).trim().to_string()
}

fn render_lower_lexed_tokens(tokens: &[OwnedLexToken]) -> String {
    render_trimmed_lexed_tokens(tokens).to_ascii_lowercase()
}

fn parse_count_prefix_words(words: &[&str]) -> Option<(u32, usize)> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_number(&tokens)
}

fn skip_articles(words: &[&str], mut idx: usize) -> usize {
    while word_slice_at_is_any(words, idx, &["a", "an", "the"]) {
        idx += 1;
    }
    idx
}

fn token_slice_from_word_index<'a>(
    tokens: &'a [OwnedLexToken],
    words: &LeafCompatWords,
    word_idx: usize,
) -> Option<&'a [OwnedLexToken]> {
    let token_start = if word_idx == 0 {
        0
    } else {
        words.token_index_for_word_index(word_idx)?
    };
    Some(&tokens[token_start..])
}

fn token_slice_for_word_range<'a>(
    tokens: &'a [OwnedLexToken],
    words: &LeafCompatWords,
    word_start: usize,
    word_end: usize,
) -> Option<&'a [OwnedLexToken]> {
    let token_start = if word_start == 0 {
        0
    } else {
        words.token_index_for_word_index(word_start)?
    };
    let token_end = if word_end == word_start {
        token_start
    } else {
        words.token_index_after_words(word_end)?
    };
    Some(&tokens[token_start..token_end])
}

fn parse_counter_type_descriptor(raw: &str) -> Result<CounterType, CardTextError> {
    let descriptor_words = raw
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| ch == ',' || ch == '.'))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(descriptor_words);
    parse_counter_type_from_tokens(&tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite counter parser could not determine counter type from '{raw}'"
        ))
    })
}

fn parse_optional_counter_type_descriptor(raw: &str) -> Result<Option<CounterType>, CardTextError> {
    let words = raw.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() || GENERIC_COUNTER_DESCRIPTOR_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    parse_counter_type_descriptor(raw).map(Some)
}

fn activation_cost_prefix_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if let Some(colon_idx) = find_token_index(tokens, OwnedLexToken::is_colon) {
        &tokens[..colon_idx]
    } else {
        tokens
    }
}

fn parse_loyalty_shorthand_activation_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<ActivationCostSegmentCst>> {
    let tokens = trim_activation_cost_segment_tokens(activation_cost_prefix_tokens(tokens));
    let parse_single = |text: &str| {
        let bytes = text.as_bytes();
        if let Some((&sign, rest)) = bytes.split_first() {
            let rest = std::str::from_utf8(rest).ok()?;
            if sign == b'+'
                && let Ok(amount) = rest.parse::<u32>()
            {
                return Some(if amount == 0 {
                    Vec::new()
                } else {
                    vec![ActivationCostSegmentCst::PutCounters {
                        counter_type: CounterType::Loyalty,
                        count: amount,
                    }]
                });
            }

            if sign == b'-' {
                if LEAF_X_WORD_PATTERN.matches_word(rest) {
                    return Some(vec![ActivationCostSegmentCst::RemoveCountersDynamic {
                        counter_type: Some(CounterType::Loyalty),
                        display_x: true,
                        remove_all: false,
                    }]);
                }
                if let Ok(amount) = rest.parse::<u32>() {
                    return Some(vec![ActivationCostSegmentCst::RemoveCounters {
                        counter_type: CounterType::Loyalty,
                        count: amount,
                    }]);
                }
            }
        }

        LEAF_ZERO_WORD_PATTERN.matches_word(text).then(Vec::new)
    };

    match tokens {
        [token] => parse_single(token.parser_text()),
        [sign, value] if sign.kind == TokenKind::Plus => {
            value.parser_text().parse::<u32>().ok().map(|amount| {
                if amount == 0 {
                    Vec::new()
                } else {
                    vec![ActivationCostSegmentCst::PutCounters {
                        counter_type: CounterType::Loyalty,
                        count: amount,
                    }]
                }
            })
        }
        [sign, value] if sign.kind == TokenKind::Dash => {
            let value = value.parser_text();
            if LEAF_X_WORD_PATTERN.matches_word(value) {
                Some(vec![ActivationCostSegmentCst::RemoveCountersDynamic {
                    counter_type: Some(CounterType::Loyalty),
                    display_x: true,
                    remove_all: false,
                }])
            } else {
                value.parse::<u32>().ok().map(|amount| {
                    vec![ActivationCostSegmentCst::RemoveCounters {
                        counter_type: CounterType::Loyalty,
                        count: amount,
                    }]
                })
            }
        }
        _ => None,
    }
}

fn parse_generic_choice_prefix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(ChoiceCount, &'a [OwnedLexToken])> {
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    if lowered.is_empty() {
        return None;
    }

    let (choice_count, consumed_words) =
        if LEAF_ONE_OR_MORE_PREFIX_PATTERN.matches_words(lowered.as_slice()) {
            (ChoiceCount::at_least(1), 3)
        } else if LEAF_ANY_NUMBER_OF_PREFIX_PATTERN.matches_words(lowered.as_slice()) {
            (ChoiceCount::any_number(), 3)
        } else if lowered
            .first()
            .is_some_and(|word| LEAF_X_WORD_PATTERN.matches_word(word))
        {
            (ChoiceCount::dynamic_x(), 1)
        } else if let Some((count, consumed_words)) = parse_count_prefix_words(lowered.as_slice()) {
            (ChoiceCount::exactly(count as usize), consumed_words)
        } else if lowered
            .first()
            .is_some_and(|word| LEAF_ARTICLE_WORD_PATTERN.matches_word(word))
        {
            (ChoiceCount::exactly(1), 1)
        } else {
            (ChoiceCount::exactly(1), 0)
        };

    let remainder = if consumed_words == 0 {
        tokens
    } else {
        let token_start = words.token_index_after_words(consumed_words)?;
        &tokens[token_start..]
    };
    Some((choice_count, remainder))
}

fn parse_discard_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    let tail = lowered.get(1..).unwrap_or_default();

    if LEAF_YOUR_HAND_PATTERN.matches_words(tail) {
        return Ok(ActivationCostSegmentCst::DiscardHand);
    }
    if LEAF_THIS_CARD_PATTERN.matches_words(tail) {
        return Ok(ActivationCostSegmentCst::DiscardSource);
    }
    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite discard parser expected selector in '{raw}'"
        )));
    }

    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some((parsed, consumed_words)) = parse_count_prefix_words(tail) {
        count = parsed;
        idx = consumed_words;
    }

    let mut other = false;
    if tail
        .get(idx)
        .is_some_and(|word| LEAF_OTHER_OR_ANOTHER_WORD_PATTERN.matches_word(word))
    {
        other = true;
        idx += 1;
    }

    while tail
        .get(idx)
        .is_some_and(|word| LEAF_A_OR_AN_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }

    if LEAF_CARD_NAMED_PREFIX_PATTERN.matches_words(&tail[idx..]) {
        let Some(name_tokens) = token_slice_from_word_index(tokens, &words, idx + 3) else {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser expected card name in '{raw}'"
            )));
        };
        let name = render_lower_lexed_tokens(name_tokens);
        if name.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser expected card name in '{raw}'"
            )));
        }
        return Ok(ActivationCostSegmentCst::DiscardFiltered {
            count,
            card_types: Vec::new(),
            random: false,
            name: Some(name),
            other,
        });
    }

    let mut card_types = Vec::new();
    while let Some(word) = tail.get(idx).copied() {
        if LEAF_CARD_OR_CARDS_WORD_PATTERN.matches_word(word) {
            break;
        }
        if LEAF_AND_OR_WORD_PATTERN.matches_word(word)
            || LEAF_A_OR_AN_WORD_PATTERN.matches_word(word)
        {
            idx += 1;
            continue;
        }
        let Some(card_type) = parse_card_type_word(word) else {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser does not yet support selector '{raw}'"
            )));
        };
        crate::slice_primitives::push_unique(&mut card_types, card_type);
        idx += 1;
    }

    if !tail
        .get(idx)
        .is_some_and(|word| LEAF_CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite discard parser expected card selector in '{raw}'"
        )));
    }
    idx += 1;

    let random = match tail.get(idx..) {
        None | Some([]) => false,
        Some(words) if LEAF_AT_RANDOM_PATTERN.matches_words(words) => true,
        _ => {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser does not yet support trailing clause in '{raw}'"
            )));
        }
    };

    if card_types.is_empty() && !random {
        return Ok(ActivationCostSegmentCst::DiscardCard(count));
    }

    Ok(ActivationCostSegmentCst::DiscardFiltered {
        count,
        card_types,
        random,
        name: None,
        other,
    })
}

fn parse_sacrifice_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    let tail = lowered.get(1..).unwrap_or_default();

    if LEAF_SOURCE_SELF_PATTERN.matches_words(tail) {
        return Ok(ActivationCostSegmentCst::SacrificeSelf);
    }
    if LEAF_A_CREATURE_PATTERN.matches_words(tail) {
        return Ok(ActivationCostSegmentCst::SacrificeCreature);
    }

    let mut idx = 0usize;
    let mut count = 1u32;
    let mut up_to = false;
    if LEAF_UP_TO_PREFIX_PATTERN.matches_words(tail)
        && let Some((parsed, consumed_words)) = parse_count_prefix_words(&tail[2..])
    {
        count = parsed;
        idx = 2 + consumed_words;
        up_to = true;
    } else if let Some((parsed, consumed_words)) = parse_count_prefix_words(tail) {
        count = parsed;
        idx = consumed_words;
    } else if LEAF_A_OR_AN_WORD_PATTERN.matches_first_word(tail) {
        idx = 1;
    }

    let mut other = false;
    if tail
        .get(idx)
        .is_some_and(|word| OTHER_OR_ANOTHER_WORD_PATTERN.matches_word(word))
    {
        other = true;
        idx += 1;
    }

    let Some(filter_tokens) = token_slice_from_word_index(tokens, &words, idx + 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite sacrifice parser missing filter in '{raw}'"
        )));
    };
    let filter_text = render_lower_lexed_tokens(filter_tokens);
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite sacrifice parser missing filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::SacrificeChosen {
        count,
        up_to,
        filter_text,
        other,
    })
}

fn parse_tap_chosen_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    let tail = lowered.get(1..).unwrap_or_default();

    let mut idx = 0usize;
    let mut count = 1u32;
    let mut other = false;

    if let Some((parsed, consumed_words)) = parse_count_prefix_words(tail) {
        count = parsed;
        idx = consumed_words;
    } else if LEAF_A_OR_AN_WORD_PATTERN.matches_first_word(tail) {
        idx = 1;
    }

    if tail
        .get(idx)
        .is_some_and(|word| OTHER_OR_ANOTHER_WORD_PATTERN.matches_word(word))
    {
        other = true;
        idx += 1;
    }

    if !tail
        .get(idx)
        .is_some_and(|word| LEAF_UNTAPPED_WORD_PATTERN.matches_word(word))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser expected untapped selector in '{raw}'"
        )));
    }
    idx += 1;

    let Some(filter_tokens) = token_slice_from_word_index(tokens, &words, idx + 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser missing tap filter in '{raw}'"
        )));
    };
    let filter_text = render_lower_lexed_tokens(filter_tokens);
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser missing tap filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::TapChosen {
        count,
        filter_text,
        other,
    })
}

fn parse_exile_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    let tail = lowered.get(1..).unwrap_or_default();

    if LEAF_TARGET_PREFIX_PATTERN.matches_words(tail) {
        return Err(CardTextError::ParseError(
            "unsupported targeted exile cost segment".to_string(),
        ));
    }

    if LEAF_EXILE_SELF_TARGET_PATTERN.matches_words(tail)
        || LEAF_EXILE_SELF_FROM_YOUR_ZONE_PREFIX_PATTERN.matches_words(tail)
    {
        let mut idx = 0usize;
        while idx + 2 < tail.len() {
            if LEAF_FROM_YOUR_GRAVEYARD_PATTERN.matches_words(&tail[idx..]) {
                return Ok(ActivationCostSegmentCst::ExileSelfFromGraveyard);
            }
            idx += 1;
        }
        return Ok(ActivationCostSegmentCst::ExileSelf);
    }

    if LEAF_TOP_LIBRARY_PATTERN.matches_words(tail) {
        let count_start = 3usize;
        let count_end = lowered.len().saturating_sub(4);
        let count = if count_start >= count_end {
            1
        } else {
            let count_tokens = token_slice_for_word_range(tokens, &words, count_start, count_end)
                .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite exile-top parser missing count in '{raw}'"
                ))
            })?;
            parse_count_word_tokens(count_tokens)?
        };
        return Ok(ActivationCostSegmentCst::ExileTopLibrary { count });
    }

    if LEAF_FROM_YOUR_HAND_SUFFIX_PATTERN.matches_words(tail) {
        let subject = &tail[..tail.len() - 3];
        if subject.is_empty() {
            return Err(CardTextError::ParseError(
                "rewrite exile-from-hand parser found empty selector".to_string(),
            ));
        }

        let mut idx = 0usize;
        let mut count = 1u32;
        if let Some((parsed, consumed_words)) = parse_count_prefix_words(subject) {
            count = parsed;
            idx = consumed_words;
        }
        idx = skip_articles(subject, idx);

        let mut color_filter = None;
        if let Some(word) = subject.get(idx).copied()
            && let Some(color) = parse_color_word(word)
        {
            color_filter = Some(color);
            idx += 1;
        }

        if !subject
            .get(idx)
            .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
        {
            return Err(CardTextError::ParseError(format!(
                "rewrite exile-from-hand parser expected card selector in '{raw}'"
            )));
        }

        return Ok(ActivationCostSegmentCst::ExileFromHand {
            count,
            color_filter,
        });
    }

    if LEAF_FROM_YOUR_GRAVEYARD_SUFFIX_PATTERN.matches_words(tail) {
        let Some(subject_tokens) =
            token_slice_for_word_range(tokens, &words, 1, lowered.len().saturating_sub(3))
        else {
            return Err(CardTextError::ParseError(
                "rewrite exile-from-graveyard parser found empty selector".to_string(),
            ));
        };
        let (choice_count, filter_tokens) = parse_generic_choice_prefix_tokens(subject_tokens)
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "rewrite exile-from-graveyard parser found empty selector".to_string(),
                )
            })?;
        let filter_text = render_lower_lexed_tokens(filter_tokens);
        return Ok(ActivationCostSegmentCst::ExileChosen {
            choice_count,
            filter_text: format!("{filter_text} from your graveyard"),
        });
    }

    if let Some(segment) = parse_exile_self_and_named_artifacts_cost(tail) {
        return Ok(segment);
    }

    let Some(subject_tokens) = token_slice_from_word_index(tokens, &words, 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite exile parser does not yet support '{raw}'"
        )));
    };
    let (choice_count, filter_tokens) = parse_generic_choice_prefix_tokens(subject_tokens)
        .ok_or_else(|| {
            CardTextError::ParseError(format!("rewrite exile parser does not yet support '{raw}'"))
        })?;
    let mut filter_text = render_lower_lexed_tokens(filter_tokens);
    if str_ends_with(filter_text.as_str(), " from a single graveyard") {
        filter_text = filter_text.replace(" from a single graveyard", " from a graveyard");
    }
    Ok(ActivationCostSegmentCst::ExileChosen {
        choice_count,
        filter_text,
    })
}

fn parse_exile_self_and_named_artifacts_cost(tail: &[&str]) -> Option<ActivationCostSegmentCst> {
    let marker_idx = word_slice_find_window_by(tail, 5, |window| {
        word_slice_eq_any(window, NAMED_ARTIFACTS_YOU_CONTROL_MARKERS)
    })?;
    if marker_idx == 0 {
        return None;
    }
    let source_words = &tail[..marker_idx];
    if source_words.is_empty() || word_slice_eq(source_words, THE_TOP_SOURCE_WORDS) {
        return None;
    }
    let name_words = &tail[marker_idx + 5..];
    if name_words.is_empty() {
        return None;
    }

    let mut names = Vec::new();
    let mut start = 0usize;
    for (idx, word) in name_words.iter().enumerate() {
        if LEAF_AND_WORD_PATTERN.matches_word(word) {
            if start < idx {
                names.push(name_words[start..idx].join(" "));
            }
            start = idx + 1;
        }
    }
    if start < name_words.len() {
        names.push(name_words[start..].join(" "));
    }
    names.retain(|name| !name.trim().is_empty());
    (names.len() >= 2).then_some(ActivationCostSegmentCst::ExileSelfAndNamedArtifacts { names })
}

fn parse_return_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    let suffix_len = if RETURN_TO_OWNER_HAND_SUFFIX_PATTERN.matches_words(lowered.as_slice()) {
        4
    } else {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser expected owner-hand suffix in '{raw}'"
        )));
    };

    let target = &lowered[1..lowered.len() - suffix_len];
    if LEAF_RETURN_SELF_TARGET_PATTERN.matches_words(target) {
        return Ok(ActivationCostSegmentCst::ReturnSelfToHand);
    }

    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some((parsed, consumed_words)) = parse_count_prefix_words(target) {
        count = parsed;
        idx = consumed_words;
    }
    idx = skip_articles(target, idx);

    let Some(filter_tokens) =
        token_slice_for_word_range(tokens, &words, idx + 1, lowered.len() - suffix_len)
    else {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser missing target filter in '{raw}'"
        )));
    };
    let filter_text = render_lower_lexed_tokens(filter_tokens);
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser missing target filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::ReturnChosenToHand { count, filter_text })
}

fn parse_put_counter_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    if LEAF_MOVE_OPPONENT_OWNED_EXILED_CARD_TO_GRAVEYARD_PATTERN.matches_words(&lowered) {
        return Ok(ActivationCostSegmentCst::MoveOpponentOwnedExiledCardToGraveyard);
    }

    let Some(on_word_idx) = LEAF_ON_WORD_PATTERN.find_word(lowered.as_slice()) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite put-counter parser missing 'on' in '{raw}'"
        )));
    };

    let descriptor = &lowered[1..on_word_idx];
    let target = &lowered[on_word_idx + 1..];
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some((parsed, consumed_words)) = parse_count_prefix_words(descriptor) {
        count = parsed;
        idx = consumed_words;
    }
    idx = skip_articles(descriptor, idx);

    let Some(counter_tokens) = token_slice_for_word_range(tokens, &words, idx + 1, on_word_idx)
    else {
        return Err(CardTextError::ParseError(format!(
            "rewrite put-counter parser missing counter description in '{raw}'"
        )));
    };
    let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
    let counter_type = parse_counter_type_descriptor(counter_descriptor.as_str())?;

    if LEAF_COUNTER_SELF_TARGET_PATTERN.matches_words(target) {
        return Ok(ActivationCostSegmentCst::PutCounters {
            counter_type,
            count,
        });
    }

    let Some(filter_tokens) = token_slice_from_word_index(tokens, &words, on_word_idx + 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite put-counter parser missing target filter in '{raw}'"
        )));
    };
    Ok(ActivationCostSegmentCst::PutCountersChosen {
        counter_type,
        count,
        filter_text: render_lower_lexed_tokens(filter_tokens),
    })
}

fn parse_remove_counter_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    let Some(from_word_idx) = LEAF_FROM_WORD_PATTERN.find_word(lowered.as_slice()) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite remove-counter parser missing 'from' in '{raw}'"
        )));
    };

    let descriptor = &lowered[1..from_word_idx];
    let target = &lowered[from_word_idx + 1..];
    let target_among = AMONG_PREFIX_PATTERN.matches_words(target);
    let target_filter_tokens = if target_among {
        token_slice_from_word_index(tokens, &words, from_word_idx + 2)
    } else {
        token_slice_from_word_index(tokens, &words, from_word_idx + 1)
    };
    let target_filter_text = target_filter_tokens
        .map(render_lower_lexed_tokens)
        .unwrap_or_default();

    if X_PREFIX_PATTERN.matches_words(descriptor) {
        let counter_type = if descriptor.len() <= 1 {
            None
        } else {
            let counter_tokens =
                token_slice_for_word_range(tokens, &words, 2, from_word_idx).unwrap_or(&[]);
            let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
            parse_optional_counter_type_descriptor(counter_descriptor.as_str())?
        };
        return if target_among {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 0,
                filter_text: target_filter_text,
                display_x: true,
                dynamic: true,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: true,
                remove_all: false,
            })
        };
    }

    if ALL_PREFIX_PATTERN.matches_words(descriptor) {
        let counter_type = if descriptor.len() <= 1 {
            None
        } else {
            let counter_tokens =
                token_slice_for_word_range(tokens, &words, 2, from_word_idx).unwrap_or(&[]);
            let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
            parse_optional_counter_type_descriptor(counter_descriptor.as_str())?
        };
        return if target_among {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 0,
                filter_text: target_filter_text,
                display_x: false,
                dynamic: true,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: false,
                remove_all: true,
            })
        };
    }

    if ANY_NUMBER_OF_PREFIX_PATTERN.matches_words(descriptor) {
        let counter_type = if descriptor.len() <= 3 {
            None
        } else {
            let counter_tokens =
                token_slice_for_word_range(tokens, &words, 4, from_word_idx).unwrap_or(&[]);
            let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
            parse_optional_counter_type_descriptor(counter_descriptor.as_str())?
        };
        return if target_among {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 0,
                filter_text: target_filter_text,
                display_x: false,
                dynamic: true,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: false,
                remove_all: false,
            })
        };
    }

    if ONE_OR_MORE_PREFIX_PATTERN.matches_words(descriptor) {
        let counter_type = if descriptor.len() <= 3 {
            None
        } else {
            let counter_tokens =
                token_slice_for_word_range(tokens, &words, 4, from_word_idx).unwrap_or(&[]);
            let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
            parse_optional_counter_type_descriptor(counter_descriptor.as_str())?
        };
        return if target_among {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 1,
                filter_text: target_filter_text,
                display_x: false,
                dynamic: true,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: false,
                remove_all: false,
            })
        };
    }

    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some((parsed, consumed_words)) = parse_count_prefix_words(descriptor) {
        count = parsed;
        idx = consumed_words;
    }
    idx = skip_articles(descriptor, idx);

    let counter_type = if idx >= descriptor.len() {
        None
    } else {
        let counter_tokens =
            token_slice_for_word_range(tokens, &words, idx + 1, from_word_idx).unwrap_or(&[]);
        let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
        parse_optional_counter_type_descriptor(counter_descriptor.as_str())?
    };

    if target_among {
        return Ok(ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type,
            count,
            filter_text: target_filter_text,
            display_x: false,
            dynamic: false,
        });
    }

    if !LEAF_COUNTER_REMOVAL_SELF_TARGET_PATTERN.matches_words(target) {
        return Ok(ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type,
            count,
            filter_text: target_filter_text,
            display_x: false,
            dynamic: false,
        });
    }

    if let Some(counter_type) = counter_type {
        return Ok(ActivationCostSegmentCst::RemoveCounters {
            counter_type,
            count,
        });
    }

    Ok(ActivationCostSegmentCst::RemoveCountersAmong {
        counter_type: None,
        count,
        filter_text: target_filter_text,
        display_x: false,
        dynamic: false,
    })
}

fn parse_activation_cost_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Result<ActivationCostSegmentCst, CardTextError>> {
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    let first = lowered.first().copied()?;

    match first {
        "pay" => Some(parse_pay_segment_tokens(tokens)),
        "discard" => Some(parse_discard_segment_tokens(tokens)),
        "mill" => Some(parse_mill_segment_tokens(tokens)),
        "sacrifice" => Some(parse_sacrifice_segment_tokens(tokens)),
        "tap" if word_slice_contains_word(&lowered, "untapped") => {
            Some(parse_tap_chosen_segment_tokens(tokens))
        }
        "behold" => Some(parse_behold_segment_tokens(tokens)),
        "blight" => Some(parse_blight_segment_tokens(tokens)),
        "exile" => Some(parse_exile_segment_tokens(tokens)),
        "reveal" => Some(parse_reveal_segment_tokens(tokens)),
        "return" => Some(parse_return_segment_tokens(tokens)),
        "exert" => Some(parse_exert_segment_tokens(tokens)),
        "put" => Some(parse_put_counter_segment_tokens(tokens)),
        "remove" => Some(parse_remove_counter_segment_tokens(tokens)),
        _ => parse_bare_symbol_segment_tokens(tokens).map(Ok),
    }
}

fn parse_energy_symbol_count_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    let mut count = 0u32;
    for token in tokens {
        if is_energy_symbol_token(token) {
            count += 1;
        } else {
            return None;
        }
    }

    (count > 0).then_some(count)
}

fn parse_bare_symbol_segment_tokens(tokens: &[OwnedLexToken]) -> Option<ActivationCostSegmentCst> {
    if tokens.is_empty() {
        return None;
    }

    if tokens.len() == 1 {
        let token = &tokens[0];
        if is_tap_symbol_token(token) {
            return Some(ActivationCostSegmentCst::Tap);
        }
        if is_untap_symbol_token(token) {
            return Some(ActivationCostSegmentCst::Untap);
        }
    }

    if let Some(count) = parse_energy_symbol_count_tokens(tokens) {
        return Some(ActivationCostSegmentCst::Energy(count));
    }

    let mut pips = Vec::new();
    for token in tokens {
        match token.kind {
            TokenKind::ManaGroup => {
                let slice = token.slice.as_str();
                if is_reserved_activation_symbol_token(token) {
                    return None;
                }
                let group = parse_mana_symbol_group(slice).ok()?;
                pips.push(group);
            }
            TokenKind::Word | TokenKind::Number => {
                let word = token.as_word()?;
                if is_reserved_activation_symbol_token(token) {
                    return None;
                }
                if let Ok(group) = parse_mana_symbol_group(word) {
                    pips.push(group);
                    continue;
                }
                let symbol = parse_mana_symbol(word).ok()?;
                pips.push(vec![symbol]);
            }
            _ => return None,
        }
    }

    (!pips.is_empty()).then(|| ActivationCostSegmentCst::Mana(ManaCost::from_pips(pips)))
}

fn parse_pay_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_trimmed_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    if !LEAF_PAY_WORD_PATTERN.matches_first_word(&lowered) {
        return Err(CardTextError::ParseError(
            "rewrite pay-cost parser expected leading 'pay'".to_string(),
        ));
    }

    let Some(rest_tokens) = token_slice_from_word_index(tokens, &words, 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite pay-cost parser missing payment in '{raw}'"
        )));
    };
    let rest_words = LeafCompatWords::new(rest_tokens);
    let lowered_rest = rest_words.to_word_refs();
    if lowered_rest.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite pay-cost parser missing payment in '{raw}'"
        )));
    }

    if LEAF_LIFE_WORD_PATTERN.matches_last_word(&lowered_rest) {
        let count_words = &lowered_rest[..lowered_rest.len() - 1];
        if let Some((amount, consumed_words)) = parse_count_prefix_words(count_words)
            && consumed_words == count_words.len()
        {
            return Ok(ActivationCostSegmentCst::Life(amount));
        }
    }

    if let Some(count) = parse_energy_symbol_count_tokens(rest_tokens) {
        return Ok(ActivationCostSegmentCst::Energy(count));
    }

    if let Some((count, consumed_words)) = parse_count_prefix_words(lowered_rest.as_slice())
        && let Some(energy_tokens) =
            token_slice_from_word_index(rest_tokens, &rest_words, consumed_words)
        && parse_energy_symbol_count_tokens(energy_tokens) == Some(1)
    {
        return Ok(ActivationCostSegmentCst::Energy(count));
    }

    if let Some(ActivationCostSegmentCst::Mana(cost)) =
        parse_bare_symbol_segment_tokens(rest_tokens)
    {
        return Ok(ActivationCostSegmentCst::Mana(cost));
    }

    Err(CardTextError::ParseError(format!(
        "rewrite pay-cost parser does not yet support '{raw}'"
    )))
}

fn parse_exert_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_trimmed_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    if !LEAF_EXERT_WORD_PATTERN.matches_first_word(&lowered) {
        return Err(CardTextError::ParseError(
            "rewrite exert-cost parser expected leading 'exert'".to_string(),
        ));
    }
    let missing_object = match token_slice_from_word_index(tokens, &words, 1) {
        None => true,
        Some(rest) => LeafCompatWords::new(rest).is_empty(),
    };
    if missing_object {
        return Err(CardTextError::ParseError(format!(
            "rewrite exert-cost parser missing exerted object in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::ExertSelf { display_text: raw })
}

fn parse_mill_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    if !LEAF_MILL_WORD_PATTERN.matches_first_word(&lowered) {
        return Err(CardTextError::ParseError(
            "rewrite mill parser expected leading 'mill'".to_string(),
        ));
    }

    let tail = lowered.get(1..).unwrap_or_default();
    let (count, consumed_words) =
        if let Some((count, consumed_words)) = parse_count_prefix_words(tail) {
            (count, consumed_words)
        } else if LEAF_A_OR_AN_WORD_PATTERN.matches_first_word(tail) {
            (1, 1)
        } else {
            return Err(CardTextError::ParseError(format!(
                "rewrite mill parser expected card count in '{raw}'"
            )));
        };

    let has_card_word = tail
        .get(consumed_words)
        .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word));
    if !has_card_word || consumed_words + 1 != tail.len() {
        return Err(CardTextError::ParseError(format!(
            "rewrite mill parser expected trailing card selector in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::Mill(count))
}

fn parse_behold_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    if !LEAF_BEHOLD_WORD_PATTERN.matches_first_word(&lowered) {
        return Err(CardTextError::ParseError(
            "rewrite behold parser expected leading 'behold'".to_string(),
        ));
    }

    let tail = lowered.get(1..).unwrap_or_default();
    let (count, consumed_words) =
        if let Some((count, consumed_words)) = parse_count_prefix_words(tail) {
            (count, consumed_words)
        } else if LEAF_A_OR_AN_WORD_PATTERN.matches_first_word(tail) {
            (1, 1)
        } else {
            return Err(CardTextError::ParseError(format!(
                "rewrite behold parser expected subtype count in '{raw}'"
            )));
        };

    let Some(subtype_word) = tail.get(consumed_words).copied() else {
        return Err(CardTextError::ParseError(format!(
            "rewrite behold parser expected subtype in '{raw}'"
        )));
    };
    let Some(subtype) = parse_subtype_word(subtype_word).or_else(|| {
        crate::string_primitives::strip_suffix_char(subtype_word, 's').and_then(parse_subtype_word)
    }) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite behold parser expected subtype in '{raw}'"
        )));
    };
    if consumed_words + 1 != tail.len() {
        return Err(CardTextError::ParseError(format!(
            "rewrite behold parser does not yet support trailing clause in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::Behold { subtype, count })
}

fn parse_blight_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    if !LEAF_BLIGHT_WORD_PATTERN.matches_first_word(&lowered) {
        return Err(CardTextError::ParseError(
            "rewrite blight parser expected leading 'blight'".to_string(),
        ));
    }

    let tail = lowered.get(1..).unwrap_or_default();
    let (count, consumed_words) =
        if let Some((count, consumed_words)) = parse_count_prefix_words(tail) {
            (count, consumed_words)
        } else {
            return Err(CardTextError::ParseError(format!(
                "rewrite blight parser expected amount in '{raw}'"
            )));
        };

    if consumed_words != tail.len() {
        return Err(CardTextError::ParseError(format!(
            "rewrite blight parser does not yet support trailing clause in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::Blight { count })
}

fn parse_reveal_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LeafCompatWords::new(tokens);
    let lowered = words.to_word_refs();
    if REVEAL_THIS_CARD_FROM_HAND_PATTERN.matches_words(&lowered) {
        return Ok(ActivationCostSegmentCst::RevealSourceFromHand);
    }

    if reveal_this_source_type_from_hand(&lowered) {
        return Ok(ActivationCostSegmentCst::RevealSourceFromHand);
    }

    Err(CardTextError::ParseError(format!(
        "rewrite reveal-cost parser does not yet support '{raw}'"
    )))
}

fn reveal_this_source_type_from_hand(words: &[&str]) -> bool {
    words.len() == 6
        && REVEAL_THIS_SOURCE_TYPE_FROM_HAND_PATTERN.matches_words(words)
        && words
            .get(2)
            .and_then(|word| parse_card_type_word(word))
            .is_some()
}

fn parse_shard_style_branch_tokens(tokens: &[OwnedLexToken]) -> Option<ManaSymbol> {
    let tokens = trim_activation_cost_segment_tokens(tokens);
    let comma_idx = find_token_index(tokens, OwnedLexToken::is_comma)?;
    let mana_tokens = trim_activation_cost_segment_tokens(&tokens[..comma_idx]);
    let tap_tokens = trim_activation_cost_segment_tokens(&tokens[comma_idx + 1..]);
    if tap_tokens.len() != 1 || tap_tokens[0].kind != TokenKind::ManaGroup {
        return None;
    }
    if !is_tap_symbol_token(&tap_tokens[0]) {
        return None;
    }

    let mana_cost = parse_mana_cost_tokens(mana_tokens).ok()?;
    let [pip] = mana_cost.pips() else {
        return None;
    };
    let [symbol] = pip.as_slice() else {
        return None;
    };
    Some(*symbol)
}

fn parse_shard_style_mana_or_tap_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(ManaSymbol, ManaSymbol)> {
    let tokens = trim_activation_cost_segment_tokens(activation_cost_prefix_tokens(tokens));
    let or_idx = find_token_index(tokens, |token| token.is_word("or"))?;
    let left = parse_shard_style_branch_tokens(&tokens[..or_idx])?;
    let right = parse_shard_style_branch_tokens(&tokens[or_idx + 1..])?;
    Some((left, right))
}

fn starts_new_activation_cost_segment_tokens(tokens: &[OwnedLexToken]) -> bool {
    let Some(first) = first_non_comma_token(tokens) else {
        return false;
    };

    match first.kind {
        TokenKind::ManaGroup | TokenKind::Number | TokenKind::Plus | TokenKind::Dash => true,
        TokenKind::Word => matches!(
            first.slice.to_ascii_lowercase().as_str(),
            "tap"
                | "t"
                | "untap"
                | "q"
                | "pay"
                | "discard"
                | "mill"
                | "sacrifice"
                | "exile"
                | "return"
                | "put"
                | "remove"
                | "behold"
                | "exert"
                | "reveal"
                | "waterbend"
                | "e"
                | "and"
                | "0"
        ),
        _ => false,
    }
}

fn split_activation_cost_segments_tokens(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut inside_named_card = false;
    let mut idx = 0usize;

    while idx < tokens.len() {
        if !inside_named_card
            && tokens[idx].is_word("card")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("named"))
        {
            inside_named_card = true;
        }

        let split_here = if tokens[idx].is_comma() {
            let remainder = &tokens[idx + 1..];
            let remainder = if token_slice_first_is(remainder, "and") {
                &remainder[1..]
            } else {
                remainder
            };
            starts_new_activation_cost_segment_tokens(remainder)
        } else if tokens[idx].is_word("and") && idx > start {
            let remainder = &tokens[idx + 1..];
            !inside_named_card && starts_new_activation_cost_segment_tokens(remainder)
        } else {
            false
        };

        if split_here {
            let segment = tokens[start..idx].to_vec();
            if !segment.is_empty() {
                segments.push(segment);
            }
            start = idx + 1;
            inside_named_card = false;
        }

        idx += 1;
    }

    let tail = tokens[start..].to_vec();
    if !tail.is_empty() {
        segments.push(tail);
    }

    segments
}

fn parse_activation_cost_cst_tokens(
    tokens: &[OwnedLexToken],
    raw: &str,
) -> Result<ActivationCostCst, CardTextError> {
    let trimmed_raw = raw.trim();
    if let Some(segments) = parse_loyalty_shorthand_activation_cost_tokens(tokens) {
        return Ok(ActivationCostCst {
            raw: trimmed_raw.to_string(),
            segments,
        });
    }

    if let Some((left, right)) = parse_shard_style_mana_or_tap_cost_tokens(tokens) {
        return Ok(ActivationCostCst {
            raw: trimmed_raw.to_string(),
            segments: vec![
                ActivationCostSegmentCst::Mana(ManaCost::from_pips(vec![vec![left, right]])),
                ActivationCostSegmentCst::Tap,
            ],
        });
    }

    let mut segments = Vec::new();
    for segment_tokens in split_activation_cost_segments_tokens(tokens) {
        let segment_tokens = trim_activation_cost_segment_tokens(&segment_tokens);
        if segment_tokens.is_empty() {
            continue;
        }

        let segment = render_trimmed_lexed_tokens(segment_tokens);
        let parsed = parse_activation_cost_segment_tokens(segment_tokens)
            .unwrap_or_else(|| {
                Err(CardTextError::ParseError(format!(
                    "rewrite activation-cost segment parser does not yet support '{segment}'",
                )))
            })
            .map_err(|err| {
                CardTextError::ParseError(format!(
                    "unsupported activation cost segment (clause: '{}'): {err}",
                    segment,
                ))
            })?;
        segments.push(parsed);
    }

    if segments.is_empty() {
        return Err(CardTextError::ParseError(
            "rewrite activation-cost parser found no segments".to_string(),
        ));
    }

    Ok(ActivationCostCst {
        raw: trimmed_raw.to_string(),
        segments,
    })
}

pub(crate) fn parse_activation_cost_tokens_rewrite(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostCst, CardTextError> {
    parse_activation_cost_cst_tokens(tokens, &render_token_slice(tokens))
}

pub(crate) fn parse_activation_cost_rewrite(raw: &str) -> Result<ActivationCostCst, CardTextError> {
    let tokens = lex_line(raw.trim(), 0)?;
    parse_activation_cost_cst_tokens(&tokens, raw)
}

pub(crate) fn lower_activation_cost_cst(
    cst: &ActivationCostCst,
) -> Result<TotalCost, CardTextError> {
    fn flush_pending_mana(costs: &mut Vec<Cost>, pending: &mut Vec<Vec<ManaSymbol>>) {
        if pending.is_empty() {
            return;
        }
        costs.push(Cost::mana(ManaCost::from_pips(std::mem::take(pending))));
    }

    let mut costs = Vec::new();
    let mut pending_mana_pips = Vec::new();
    let mut tap_tag_id = 0usize;
    let mut sacrifice_tag_id = 0usize;
    let mut exile_tag_id = 0usize;
    let mut return_tag_id = 0usize;
    for segment in &cst.segments {
        match segment {
            ActivationCostSegmentCst::Mana(cost) => {
                pending_mana_pips.extend(cost.pips().to_vec());
            }
            ActivationCostSegmentCst::Tap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::tap());
            }
            ActivationCostSegmentCst::TapChosen {
                count,
                filter_text,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = parse_filter_text(filter_text, *other)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                filter.untapped = true;
                let tag = format!("tap_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::tap(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::Untap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::untap());
            }
            ActivationCostSegmentCst::Life(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::life(*amount));
            }
            ActivationCostSegmentCst::Energy(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::energy(*amount));
            }
            ActivationCostSegmentCst::DiscardSource => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_source());
            }
            ActivationCostSegmentCst::DiscardHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_hand());
            }
            ActivationCostSegmentCst::DiscardCard(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard(*count, None));
            }
            ActivationCostSegmentCst::DiscardFiltered {
                count,
                card_types,
                random,
                name,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if *random || name.is_some() || *other {
                    let card_filter = if card_types.is_empty() && name.is_none() && !*other {
                        None
                    } else {
                        let mut filter = ObjectFilter {
                            zone: Some(crate::zone::Zone::Hand),
                            card_types: card_types.clone(),
                            ..Default::default()
                        };
                        if let Some(name) = name {
                            filter = filter.named(name.clone());
                        }
                        if *other {
                            filter.other = true;
                        }
                        Some(filter)
                    };
                    costs.push(Cost::validated_effect(Effect::discard_player_filtered(
                        *count as i32,
                        PlayerFilter::You,
                        *random,
                        card_filter,
                    )));
                } else if card_types.len() > 1 {
                    costs.push(Cost::discard_types(*count, card_types.clone()));
                } else if let Some(card_type) = card_types.first().copied() {
                    costs.push(Cost::discard(*count, Some(card_type)));
                } else {
                    costs.push(Cost::discard(*count, None));
                }
            }
            ActivationCostSegmentCst::Mill(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::mill(*count));
            }
            ActivationCostSegmentCst::Behold { subtype, count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::behold(*subtype, *count)));
            }
            ActivationCostSegmentCst::Blight { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("blight_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::put_counters(
                    CounterType::MinusOneMinusOne,
                    *count as i32,
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::SacrificeSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::sacrifice_self());
            }
            ActivationCostSegmentCst::SacrificeCreature => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::sacrifice(
                    ObjectFilter::tagged(tag),
                    1,
                )));
            }
            ActivationCostSegmentCst::SacrificeChosen {
                count,
                up_to,
                filter_text,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let normalized_filter_text = if *count == 1 {
                    str_strip_prefix(filter_text.trim(), "a ")
                        .or_else(|| str_strip_prefix(filter_text.trim(), "an "))
                        .unwrap_or(filter_text.trim())
                } else {
                    filter_text.trim()
                };
                let mut filter = parse_filter_text(normalized_filter_text, *other)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                let choice_count = if *up_to {
                    ChoiceCount::up_to(*count as usize)
                } else {
                    ChoiceCount::exactly(*count as usize)
                };
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    choice_count,
                    PlayerFilter::You,
                    tag.clone(),
                )));
                let sacrifice = if *up_to {
                    Effect::sacrifice_player(
                        ObjectFilter::tagged(tag.clone()),
                        crate::effect::Value::Count(ObjectFilter::tagged(tag)),
                        PlayerFilter::You,
                    )
                } else {
                    Effect::sacrifice(ObjectFilter::tagged(tag), *count)
                };
                costs.push(Cost::validated_effect(sacrifice));
            }
            ActivationCostSegmentCst::ExileSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileSelfFromGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileFromHand {
                count,
                color_filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_from_hand(*count, *color_filter));
            }
            ActivationCostSegmentCst::ExileFromGraveyard { count, card_type } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = ObjectFilter::default()
                    .owned_by(PlayerFilter::You)
                    .in_zone(crate::zone::Zone::Graveyard);
                if let Some(card_type) = card_type {
                    filter = filter.with_type(*card_type);
                }
                let tag = format!("exile_cost_{exile_tag_id}");
                exile_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::exile(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileChosen {
                choice_count,
                filter_text,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = parse_filter_text(filter_text, false)?;
                if filter_text_mentions_spell(filter_text) {
                    filter.zone = Some(crate::zone::Zone::Stack);
                    filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
                    filter.has_mana_cost = true;
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                if filter.zone == Some(crate::zone::Zone::Battlefield)
                    && filter.controller.is_none()
                {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("exile_cost_{exile_tag_id}");
                exile_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    *choice_count,
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::exile(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileSelfAndNamedArtifacts { names } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
                for name in names {
                    let tag = format!("exile_cost_{exile_tag_id}");
                    exile_tag_id += 1;
                    let mut filter = ObjectFilter {
                        zone: Some(crate::zone::Zone::Battlefield),
                        controller: Some(PlayerFilter::You),
                        card_types: vec![CardType::Artifact],
                        ..Default::default()
                    };
                    filter.name = Some(name.clone());
                    costs.push(Cost::validated_effect(Effect::choose_objects(
                        filter,
                        ChoiceCount::exactly(1),
                        PlayerFilter::You,
                        tag.clone(),
                    )));
                    costs.push(Cost::validated_effect(Effect::exile(
                        crate::target::ChooseSpec::tagged(tag),
                    )));
                }
            }
            ActivationCostSegmentCst::ExileTopLibrary { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                #[cfg(not(feature = "serialization"))]
                costs.push(Cost::validated_effect(Effect::exile_top_of_library_player(
                    *count as i32,
                    PlayerFilter::You,
                    crate::tag::TagKey::from("__cost_exiled_top__"),
                    None,
                )));
                #[cfg(feature = "serialization")]
                costs.push(Cost::validated_effect(Effect::exile_top_of_library_player(
                    *count as i32,
                    PlayerFilter::You,
                )));
            }
            ActivationCostSegmentCst::RevealSourceFromHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(Effect::reveal_source_from_hand()));
            }
            ActivationCostSegmentCst::ReturnSelfToHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::return_self_to_hand());
            }
            ActivationCostSegmentCst::ReturnChosenToHand { count, filter_text } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = parse_filter_text(filter_text, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = format!("return_cost_{return_tag_id}");
                return_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::return_to_hand(
                    ObjectFilter::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::MoveOpponentOwnedExiledCardToGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("graveyard_cost_{return_tag_id}");
                return_tag_id += 1;
                let filter = ObjectFilter {
                    zone: Some(crate::zone::Zone::Exile),
                    owner: Some(PlayerFilter::Opponent),
                    ..Default::default()
                };
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::move_to_zone(
                    crate::target::ChooseSpec::tagged(tag),
                    crate::zone::Zone::Graveyard,
                    false,
                )));
            }
            ActivationCostSegmentCst::ExertSelf { display_text } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(crate::effects::ExertCostEffect::new(
                    display_text.clone(),
                )));
            }
            ActivationCostSegmentCst::PutCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::add_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::PutCountersChosen {
                counter_type,
                count,
                filter_text,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let normalized_filter = filter_text.trim().to_ascii_lowercase();
                if matches!(
                    normalized_filter.as_str(),
                    "a creature you control" | "creature you control"
                ) {
                    costs.push(Cost::add_counters(*counter_type, *count));
                    continue;
                }
                let mut filter = parse_filter_text(filter_text, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                if filter.source {
                    costs.push(Cost::add_counters(*counter_type, *count));
                    continue;
                }
                let tag = format!("put_counter_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::put_counters(
                    *counter_type,
                    *count as i32,
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::RemoveCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::remove_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count,
                filter_text,
                display_x,
                dynamic,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = parse_filter_text(filter_text, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let effect = if *dynamic {
                    Effect::remove_dynamic_counters_among(
                        *count,
                        u32::MAX / 4,
                        filter,
                        *counter_type,
                        *display_x,
                    )
                } else {
                    Effect::remove_any_counters_among(*count, filter, *counter_type)
                };
                costs.push(Cost::validated_effect(effect));
            }
            ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x,
                remove_all,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let cost = if *remove_all {
                    Cost::remove_all_counters_from_source(*counter_type)
                } else {
                    Cost::remove_any_counters_from_source(*counter_type, *display_x)
                };
                costs.push(cost);
            }
        }
    }
    flush_pending_mana(&mut costs, &mut pending_mana_pips);
    Ok(TotalCost::from_costs(costs))
}
