use super::*;
use crate::lexer::{TokenKind, parser_token_word_positions, render_token_slice};
use winnow::error::{ContextError, ErrMode};

const ENTERED_SINCE_LAST_TURN_WITH_THAT_PREFIX: &[&str] =
    &["that", "entered", "since", "your", "last", "turn", "ended"];
const ENTERED_SINCE_LAST_TURN_PREFIX: &[&str] =
    &["entered", "since", "your", "last", "turn", "ended"];
const COLOR_OR_COLORS_WORDS: &[&str] = &["color", "colors"];
const NOT_ALL_COLORS_WITH_THAT_PREFIX: &[&str] = &["that", "isnt", "all", "colors"];
const NOT_ALL_COLORS_PREFIX: &[&str] = &["isnt", "all", "colors"];
const NOT_EXACTLY_TWO_COLORS_WITH_THAT_PREFIXES: &[&[&str]] = &[
    &["that", "isnt", "exactly", "two", "colors"],
    &["that", "arent", "exactly", "two", "colors"],
];
const NOT_EXACTLY_TWO_COLORS_PREFIXES: &[&[&str]] = &[
    &["isnt", "exactly", "two", "colors"],
    &["arent", "exactly", "two", "colors"],
];
const EXACTLY_TWO_COLORS_WITH_THAT_PREFIXES: &[&[&str]] = &[
    &["that", "is", "exactly", "two", "colors"],
    &["that", "are", "exactly", "two", "colors"],
];
const EXACTLY_TWO_COLORS_PREFIXES: &[&[&str]] = &[
    &["is", "exactly", "two", "colors"],
    &["are", "exactly", "two", "colors"],
];
const MANA_VALUE_COUNTERS_ON_SOURCE_PREFIX: &[&str] = &["with", "mana", "value"];
const MANA_VALUE_EQUAL_WORDS: &[&str] = &["equal", "to"];
const MANA_VALUE_LT_WORDS: &[&str] = &["less", "than"];
const MANA_VALUE_LTE_WORDS: &[&str] = &["less", "than", "or", "equal", "to"];
const MANA_VALUE_GT_WORDS: &[&str] = &["greater", "than"];
const MANA_VALUE_GTE_WORDS: &[&str] = &["greater", "than", "or", "equal", "to"];
const NUMBER_OF_WORDS: &[&str] = &["number", "of"];
const COUNTER_OR_COUNTERS_WORDS: &[&str] = &["counter", "counters"];
const ON_THIS_ARTIFACT_TAIL: &[&str] = &["on", "this", "artifact"];
const ON_IT_TAIL: &[&str] = &["on", "it"];
const OTHER_THAN_PREFIX: &[&str] = &["other", "than"];
const ONE_OF_PREFIX: &[&str] = &["one", "of"];
const DIFFERENT_ONE_OF_PREFIX: &[&str] = &["different", "one", "of"];
const OF_OR_FROM_WORDS: &[&str] = &["of", "from"];
const POWER_OR_TOUGHNESS_PREFIX: &[&str] = &["power", "or", "toughness"];
const NAMED_WORD: &str = "named";
const IT_OR_THEM_WORDS: &[&str] = &["it", "them"];
const REVEALED_CARD_PREFIXES: &[&[&str]] = &[&["revealed", "card"], &["revealed", "cards"]];
const EXILED_CARD_PREFIXES: &[&[&str]] = &[&["exiled", "card"], &["exiled", "cards"]];
const ENCHANTED_PLAYER_PREFIX: &[&str] = &["enchanted", "player"];
const ATTACHED_TO_TAGGED_OBJECT_PREFIXES: &[&[&str]] = &[
    &["it"],
    &["that", "object"],
    &["that", "creature"],
    &["that", "permanent"],
    &["that", "equipment"],
    &["that", "aura"],
];
const THAT_WORD: &str = "that";
const BE_VERB_WORDS: &[&str] = &["were", "was", "is", "are"];
const NOT_NAMED_PHRASE: &[&str] = &["not", "named"];
const SINGLE_GRAVEYARD_PHRASE: &[&str] = &["single", "graveyard"];
const ITS_ATTACHED_TO_PHRASE: &[&str] = &["its", "attached", "to"];
const EXILED_WITH_PHRASE: &[&str] = &["exiled", "with"];
const USED_TO_CRAFT_PHRASE: &[&str] = &["used", "to", "craft"];
const REFERENCE_HEAD_WORDS: &[&str] = &["this", "that", "the", "it", "them"];
const REFERENCE_OBJECT_NOUN_WORDS: &[&str] = &[
    "artifact",
    "creature",
    "enchantment",
    "land",
    "permanent",
    "planeswalker",
    "card",
    "spell",
    "source",
];
const SHARE_WORDS: &[&str] = &["share", "shares"];
const SHARED_CARD_TYPE_PHRASES: &[&[&str]] = &[
    &["card", "type"],
    &["card", "types"],
    &["permanent", "type"],
    &["permanent", "types"],
];

fn shared_type_relation(words: &[&str]) -> TaggedOpbjectRelation {
    if find_any_phrase_start(words, &[&["permanent", "type"], &["permanent", "types"]]).is_some() {
        TaggedOpbjectRelation::SharesPermanentType
    } else {
        TaggedOpbjectRelation::SharesCardType
    }
}
const CREATURE_TYPE_PHRASES: &[&[&str]] = &[&["creature", "type"], &["creature", "types"]];
const TAPPED_THIS_WAY_PHRASE: &[&str] = &["tapped", "this", "way"];
const EACH_CREATURE_TAPPED_THIS_WAY_PHRASE: &[&str] =
    &["each", "creature", "tapped", "this", "way"];
const EXILED_CARD_REFERENCE_PHRASES: &[&[&str]] = &[
    &["with", "exiled", "card"],
    &["with", "exiled", "cards"],
    &["with", "the", "exiled", "card"],
    &["with", "the", "exiled", "cards"],
];
const NO_WORD: &str = "no";
const OR_WORD: &str = "or";
const ABILITY_OR_ABILITIES_WORDS: &[&str] = &["ability", "abilities"];
const COLORLESS_WORD: &str = "colorless";
const MULTICOLORED_WORD: &str = "multicolored";
const ODD_MANA_VALUE_PHRASES: &[&[&str]] = &[&["odd", "mana", "value"], &["odd", "mana", "values"]];
const EVEN_MANA_VALUE_PHRASES: &[&[&str]] =
    &[&["even", "mana", "value"], &["even", "mana", "values"]];
const ODD_POWER_PHRASE: &[&str] = &["odd", "power"];
const EVEN_POWER_PHRASE: &[&str] = &["even", "power"];
const SAME_MANA_VALUE_AS_PHRASE: &[&str] = &["same", "mana", "value", "as"];
const EQUAL_OR_LESSER_MANA_VALUE_PHRASE: &[&str] = &["equal", "or", "lesser", "mana", "value"];
const LTE_MANA_VALUE_THAN_THAT_SPELL_PHRASES: &[&[&str]] = &[
    &[
        "equal", "or", "lesser", "mana", "value", "than", "that", "spell",
    ],
    &[
        "less", "than", "or", "equal", "to", "that", "spells", "mana", "value",
    ],
];
const LTE_MANA_VALUE_AS_TAGGED_PHRASES: &[&[&str]] = &[
    &[
        "equal", "or", "lesser", "mana", "value", "than", "that", "spell",
    ],
    &[
        "equal", "or", "lesser", "mana", "value", "than", "that", "card",
    ],
    &[
        "equal", "or", "lesser", "mana", "value", "than", "that", "object",
    ],
    &[
        "less", "than", "or", "equal", "to", "that", "spells", "mana", "value",
    ],
    &[
        "less", "than", "or", "equal", "to", "that", "cards", "mana", "value",
    ],
    &[
        "less", "than", "or", "equal", "to", "that", "objects", "mana", "value",
    ],
];
const LESSER_MANA_VALUE_PHRASE: &[&str] = &["lesser", "mana", "value"];
const ADDITIONAL_COST_OBJECT_REFERENCE_PHRASES: &[&[&str]] = &[
    &["the", "sacrificed", "creature"],
    &["the", "sacrificed", "artifact"],
    &["the", "sacrificed", "enchantment"],
    &["the", "sacrificed", "permanent"],
    &["a", "sacrificed", "creature"],
    &["a", "sacrificed", "artifact"],
    &["a", "sacrificed", "enchantment"],
    &["a", "sacrificed", "permanent"],
    &["sacrificed", "creature"],
    &["sacrificed", "artifact"],
    &["sacrificed", "enchantment"],
    &["sacrificed", "permanent"],
    &["the", "exiled", "creature"],
    &["the", "exiled", "artifact"],
    &["the", "exiled", "enchantment"],
    &["the", "exiled", "permanent"],
    &["an", "exiled", "creature"],
    &["an", "exiled", "artifact"],
    &["an", "exiled", "enchantment"],
    &["an", "exiled", "permanent"],
    &["exiled", "creature"],
    &["exiled", "artifact"],
    &["exiled", "enchantment"],
    &["exiled", "permanent"],
];
const IT_OR_ITS_REFERENCE_WORDS: &[&str] = &["it", "its"];
const ATTACKING_THAT_PLAYER_PHRASES: &[&[&str]] = &[
    &["attacking", "that", "player"],
    &["attacking", "that", "players"],
];
const ATTACKING_DEFENDING_PLAYER_PHRASES: &[&[&str]] = &[
    &["attacking", "defending", "player"],
    &["attacking", "defending", "players"],
];
const ATTACKING_TARGET_PLAYER_PHRASES: &[&[&str]] = &[
    &["attacking", "target", "player"],
    &["attacking", "target", "players"],
];
const ATTACKING_TARGET_OPPONENT_PHRASES: &[&[&str]] = &[
    &["attacking", "target", "opponent"],
    &["attacking", "target", "opponents"],
];
const ATTACKING_CHOSEN_PLAYER_PHRASES: &[&[&str]] = &[
    &["attacking", "the", "last", "chosen", "player"],
    &["attacking", "last", "chosen", "player"],
    &["attacking", "the", "chosen", "player"],
    &["attacking", "chosen", "player"],
];
const ATTACKING_YOU_PHRASE: &[&str] = &["attacking", "you"];
const ATTACKING_THEM_PHRASE: &[&str] = &["attacking", "them"];
const ATTACKING_OPPONENT_PHRASES: &[&[&str]] = &[
    &["attacking", "opponent"],
    &["attacking", "opponents"],
    &["attacking", "your", "opponent"],
    &["attacking", "your", "opponents"],
    &["attacking", "one", "of", "your", "opponents"],
];
const EQUIPPED_WORD: &str = "equipped";
const ENCHANTED_WORD: &str = "enchanted";
const ATTACHED_WORD: &str = "attached";
const TO_WORD: &str = "to";
const CONVOKED_THIS_SPELL_TAG_PHRASES: &[&[&str]] = &[
    &["that", "convoked", "this", "spell"],
    // A permanent spell becomes "this creature" once it is on the
    // battlefield, but the recorded convoke payment still belongs to the
    // spell that produced it.  Keep both oracle surfaces on the same tag.
    &["that", "convoked", "this", "creature"],
    &["that", "convoked", "it"],
];
const CREWED_IT_THIS_TURN_TAG_PHRASE: &[&str] = &["that", "crewed", "it", "this", "turn"];
const SADDLED_IT_THIS_TURN_TAG_PHRASE: &[&str] = &["that", "saddled", "it", "this", "turn"];
const AMASSED_ARMY_TAG_PHRASES: &[&[&str]] = &[
    &["army", "you", "amassed"],
    &["amassed", "army"],
    &["amassed", "armys"],
];
const TAGGED_OBJECT_REFERENCE_FOR_MANA_VALUE_PHRASES: &[&[&str]] = &[
    &["same", "mana", "value", "as", "object"],
    &["same", "mana", "value", "as", "creature"],
    &["same", "mana", "value", "as", "artifact"],
    &["same", "mana", "value", "as", "permanent"],
    &["same", "mana", "value", "as", "spell"],
    &["same", "mana", "value", "as", "card"],
    &["that", "object"],
    &["that", "creature"],
    &["that", "artifact"],
    &["that", "permanent"],
    &["that", "spell"],
    &["that", "card"],
    &["the", "object"],
    &["the", "creature"],
    &["the", "artifact"],
    &["the", "permanent"],
    &["the", "spell"],
    &["the", "card"],
];
const SAME_NAME_AS_TAGGED_OBJECT_PHRASES: &[&[&str]] = &[
    &["same", "name", "as", "spell"],
    &["same", "name", "as", "card"],
    &["same", "name", "as", "object"],
    &["same", "name", "as", "creature"],
    &["same", "name", "as", "permanent"],
    &["same", "name", "as", "the", "spell"],
    &["same", "name", "as", "the", "card"],
    &["same", "name", "as", "the", "object"],
    &["same", "name", "as", "the", "creature"],
    &["same", "name", "as", "the", "permanent"],
    &["same", "name", "as", "that", "spell"],
    &["same", "name", "as", "that", "card"],
    &["same", "name", "as", "that", "object"],
    &["same", "name", "as", "that", "creature"],
    &["same", "name", "as", "that", "permanent"],
    &["same", "name", "as", "this", "spell"],
    &["same", "name", "as", "this", "card"],
    &["same", "name", "as", "this", "object"],
    &["same", "name", "as", "this", "creature"],
    &["same", "name", "as", "this", "permanent"],
];

const SAME_NAME_AS_SOURCE_OBJECT_PHRASES: &[&[&str]] = &[
    &["same", "name", "as", "this", "spell"],
    &["same", "name", "as", "this", "card"],
    &["same", "name", "as", "this", "object"],
    &["same", "name", "as", "this", "creature"],
    &["same", "name", "as", "this", "permanent"],
];

fn same_name_antecedent_surface(
    words: &[&str],
) -> Option<ironsmith_core::SameNameAntecedentSurface> {
    let start = crate::word_primitives::parse_sequence_start(words, &["same", "name", "as"])?;
    words[start + 3..]
        .iter()
        .copied()
        .find_map(ironsmith_core::SameNameAntecedentSurface::from_noun)
}

fn find_any_phrase_start(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    let mut start = 0usize;
    while start < words.len() {
        let mut input: primitives::WordSliceInput<'_> = &words[start..];
        if parse_any_word_phrase(&mut input, phrases).is_ok() {
            return Some(start);
        }
        start += 1;
    }
    None
}

fn additional_cost_object_surface(
    words: &[&str],
) -> Option<ironsmith_core::AdditionalCostObjectSurface> {
    for (idx, word) in words.iter().enumerate() {
        let action = match *word {
            "sacrificed" => ironsmith_core::AdditionalCostObjectAction::Sacrificed,
            "exiled" => ironsmith_core::AdditionalCostObjectAction::Exiled,
            _ => continue,
        };
        let kind = match words.get(idx + 1).copied() {
            Some("creature" | "creatures") => ironsmith_core::SacrificedObjectKind::Creature,
            Some("artifact" | "artifacts") => ironsmith_core::SacrificedObjectKind::Artifact,
            Some("enchantment" | "enchantments") => {
                ironsmith_core::SacrificedObjectKind::Enchantment
            }
            Some("permanent" | "permanents") => ironsmith_core::SacrificedObjectKind::Permanent,
            _ => continue,
        };
        return Some(ironsmith_core::AdditionalCostObjectSurface::new(
            action, kind,
        ));
    }
    None
}

fn find_phrase_start(words: &[&str], phrase: &[&str]) -> Option<usize> {
    find_any_phrase_start(words, &[phrase])
}

fn words_start_with_phrase(words: &[&str], phrase: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_word_phrase(&mut input, phrase).is_ok()
}

fn words_start_with_any_phrase(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_any_word_phrase(input, phrases)
    })?;
    words.len().checked_sub(input.len())
}

fn parse_any_word_phrase(
    input: &mut primitives::WordSliceInput<'_>,
    phrases: &[&[&str]],
) -> Result<(), ErrMode<ContextError>> {
    for phrase in phrases {
        let mut probe = *input;
        if parse_word_phrase(&mut probe, phrase).is_ok() {
            *input = probe;
            return Ok(());
        }
    }
    Err(primitives::backtrack_err(
        "filter phrase",
        "one of the expected filter phrases",
    ))
}

fn parse_word_phrase(
    input: &mut primitives::WordSliceInput<'_>,
    phrase: &[&str],
) -> Result<(), ErrMode<ContextError>> {
    for expected in phrase {
        let Some((actual, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err(
                "filter phrase",
                "expected filter word",
            ));
        };
        if actual != expected {
            return Err(primitives::backtrack_err(
                "filter phrase",
                "expected filter word",
            ));
        }
        *input = rest;
    }
    Ok(())
}

fn word_is_any(word: &str, expected: &[&str]) -> bool {
    let mut idx = 0usize;
    while idx < expected.len() {
        if expected[idx] == word {
            return true;
        }
        idx += 1;
    }
    false
}

fn words_contain_word(words: &[&str], expected: &str) -> bool {
    words.contains(&expected)
}

fn words_contain_any_word(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().any(|word| words_contain_word(words, word))
}

fn words_contain_all(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().all(|word| words_contain_word(words, word))
}

fn word_index_for_exact(words: &[&str], expected: &str) -> Option<usize> {
    let mut idx = 0usize;
    while idx < words.len() {
        if words[idx] == expected {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

pub(super) fn remove_word_range(words: &mut Vec<&str>, start: usize, end: usize) {
    let mut remaining = Vec::with_capacity(words.len());
    remaining.extend_from_slice(&words[..start]);
    remaining.extend_from_slice(&words[end..]);
    *words = remaining;
}

fn normalized_literal_name_key(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn literal_name_surface_after_marker(tokens: &[OwnedLexToken], marker: &[&str]) -> Option<String> {
    let positions = parser_token_word_positions(tokens);
    let position_words = positions.iter().map(|(_, word)| *word).collect::<Vec<_>>();
    let marker_start = crate::word_primitives::parse_sequence_start(&position_words, marker)?;
    let name_start = marker_start + marker.len();
    let start_token = positions.get(name_start)?.0;
    let name_end =
        crate::slice_primitives::select_position(&positions[name_start + 1..], |(_, word)| {
            is_name_clause_boundary(word)
        })
        .map(|offset| name_start + 1 + offset)
        .unwrap_or(positions.len());
    let mut end_token = positions
        .get(name_end)
        .map(|(token_idx, _)| *token_idx)
        .unwrap_or(tokens.len());
    while end_token > start_token
        && matches!(
            tokens[end_token - 1].kind,
            TokenKind::Period | TokenKind::Semicolon
        )
    {
        end_token -= 1;
    }
    let surface = render_token_slice(&tokens[start_token..end_token])
        .trim()
        .to_string();
    (!surface.is_empty()).then_some(surface)
}

pub(super) fn try_apply_not_named_clause<'a, F, G>(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&'a str>,
    all_words_with_articles: &[&'a str],
    map_non_article_index: &F,
    map_non_article_end: &G,
    source_tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError>
where
    F: Fn(usize) -> Option<usize>,
    G: Fn(usize) -> Option<usize>,
{
    let Some(not_named_idx) = find_phrase_start(all_words.as_slice(), NOT_NAMED_PHRASE) else {
        return Ok(false);
    };
    let (name, name_end) = extract_name_clause_text(
        all_words.as_slice(),
        all_words_with_articles,
        not_named_idx,
        2,
        map_non_article_index,
        map_non_article_end,
        "not-named",
    )?;
    let parsed_surface = literal_name_surface_after_marker(source_tokens, NOT_NAMED_PHRASE);
    filter.excluded_name = Some(name);
    if let Some(surface) = parsed_surface {
        filter.set_excluded_name_surface(surface);
    }
    remove_word_range(all_words, not_named_idx, name_end);
    Ok(true)
}

pub(super) fn try_apply_named_clause<'a, F, G>(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&'a str>,
    all_words_with_articles: &[&'a str],
    map_non_article_index: &F,
    map_non_article_end: &G,
) -> Result<bool, CardTextError>
where
    F: Fn(usize) -> Option<usize>,
    G: Fn(usize) -> Option<usize>,
{
    let Some(named_idx) = word_index_for_exact(all_words.as_slice(), NAMED_WORD) else {
        return Ok(false);
    };
    let (name, name_end) = extract_name_clause_text(
        all_words.as_slice(),
        all_words_with_articles,
        named_idx,
        1,
        map_non_article_index,
        map_non_article_end,
        "named",
    )?;
    filter.name = Some(name);
    remove_word_range(all_words, named_idx, name_end);
    Ok(true)
}

pub(super) fn parse_entered_since_your_last_turn_ended_words(words: &[&str]) -> Option<usize> {
    if words_start_with_phrase(words, ENTERED_SINCE_LAST_TURN_WITH_THAT_PREFIX) {
        Some(ENTERED_SINCE_LAST_TURN_WITH_THAT_PREFIX.len())
    } else if words_start_with_phrase(words, ENTERED_SINCE_LAST_TURN_PREFIX) {
        Some(ENTERED_SINCE_LAST_TURN_PREFIX.len())
    } else {
        None
    }
}

pub(super) fn try_apply_entered_since_your_last_turn_ended_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((idx, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_entered_since_your_last_turn_ended_words,
    ) else {
        return false;
    };
    filter.entered_since_your_last_turn_ended = true;
    all_words.drain(idx..idx + consumed);
    true
}

pub(super) fn strip_object_filter_face_state_words(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) {
    let mut idx = 0usize;
    while idx < all_words.len() {
        let Some((face_down, consumed)) = parse_filter_face_state_words(&all_words[idx..]) else {
            idx += 1;
            continue;
        };
        filter.face_down = Some(face_down);
        all_words.drain(idx..idx + consumed);
    }
}

pub(super) fn strip_single_graveyard_phrase(filter: &mut ObjectFilter, all_words: &mut Vec<&str>) {
    while let Some(idx) = find_phrase_start(all_words.as_slice(), SINGLE_GRAVEYARD_PHRASE) {
        filter.single_graveyard = true;
        all_words.remove(idx);
    }
}

fn parse_color_count_number_words(words: &[&str]) -> Option<(u32, usize)> {
    let word = words.first().copied()?;
    parse_number_word_u32(word)
        .or_else(|| crate::util::decimal_count(word))
        .map(|count| (count, 1))
}

fn parse_min_color_count_quantity_prefix(words: &[&str]) -> Option<(u32, usize)> {
    if words_start_with_phrase(words, &["at", "least"]) {
        let (count, used) = parse_color_count_number_words(words.get(2..).unwrap_or_default())?;
        return Some((count, used + 2));
    }

    if word_is_any(words.first().copied()?, &["more", "greater"])
        && words.get(1).copied() == Some("than")
    {
        let (count, used) = parse_color_count_number_words(words.get(2..).unwrap_or_default())?;
        return Some((count.saturating_add(1), used + 2));
    }

    let (count, used) = parse_color_count_number_words(words)?;
    if words.get(used).copied() == Some("or")
        && words
            .get(used + 1)
            .is_some_and(|word| word_is_any(word, &["more", "greater"]))
    {
        return Some((count, used + 2));
    }

    None
}

pub(super) fn parse_color_count_phrase_words(words: &[&str]) -> Option<(u32, usize)> {
    let (count, used) = parse_min_color_count_quantity_prefix(words)?;
    words
        .get(used)
        .is_some_and(|word| word_is_any(word, COLOR_OR_COLORS_WORDS))
        .then_some((count, used + 1))
}

pub(super) fn try_apply_color_count_phrase(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> Result<bool, CardTextError> {
    let Some((color_count_idx, (count, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_color_count_phrase_words(&all_words[idx..]).map(|matched| (idx, matched))
        })
    else {
        return Ok(false);
    };

    if count >= 3 {
        return Err(CardTextError::ParseError(format!(
            "unsupported color-count object filter '{}'",
            all_words[color_count_idx..color_count_idx + consumed].join(" ")
        )));
    }

    if count == 1 {
        let any_color: ColorSet = Color::ALL.into_iter().collect();
        filter.colors = Some(any_color);
    } else {
        filter.color_count = Some(crate::filter::Comparison::GreaterThanOrEqual(count as i32));
    }

    all_words.drain(color_count_idx..color_count_idx + consumed);
    Ok(true)
}

pub(super) fn try_apply_pt_literal_prefix(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let pt_index = usize::from(
        all_words
            .first()
            .is_some_and(|word| matches!(*word, "all" | "each")),
    );
    let Some((power, toughness)) = all_words
        .get(pt_index)
        .and_then(|word| parse_unsigned_pt_word(word))
    else {
        return false;
    };
    filter.power = Some(crate::filter::Comparison::Equal(power));
    filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
    all_words.remove(pt_index);
    true
}

pub(super) fn parse_not_all_colors_words(words: &[&str]) -> Option<usize> {
    if words_start_with_phrase(words, NOT_ALL_COLORS_WITH_THAT_PREFIX) {
        Some(NOT_ALL_COLORS_WITH_THAT_PREFIX.len())
    } else if words_start_with_phrase(words, NOT_ALL_COLORS_PREFIX) {
        Some(NOT_ALL_COLORS_PREFIX.len())
    } else {
        None
    }
}

pub(super) fn try_apply_not_all_colors_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((idx, consumed)) =
        find_filter_prefix_consumed(all_words.as_slice(), parse_not_all_colors_words)
    else {
        return false;
    };
    filter.all_colors = Some(false);
    all_words.drain(idx..idx + consumed);
    true
}

fn parse_phrase_choice_len(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    phrases
        .iter()
        .find(|phrase| words_start_with_phrase(words, phrase))
        .map(|phrase| phrase.len())
}

pub(super) fn parse_not_exactly_two_colors_words(words: &[&str]) -> Option<usize> {
    parse_phrase_choice_len(words, NOT_EXACTLY_TWO_COLORS_WITH_THAT_PREFIXES)
        .or_else(|| parse_phrase_choice_len(words, NOT_EXACTLY_TWO_COLORS_PREFIXES))
}

pub(super) fn parse_exactly_two_colors_words(words: &[&str]) -> Option<usize> {
    parse_phrase_choice_len(words, EXACTLY_TWO_COLORS_WITH_THAT_PREFIXES)
        .or_else(|| parse_phrase_choice_len(words, EXACTLY_TWO_COLORS_PREFIXES))
}

pub(super) fn try_apply_not_exactly_two_colors_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((idx, consumed)) =
        find_filter_prefix_consumed(all_words.as_slice(), parse_not_exactly_two_colors_words)
    else {
        return false;
    };
    filter.exactly_two_colors = Some(false);
    all_words.drain(idx..idx + consumed);
    true
}

pub(super) fn try_apply_exactly_two_colors_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let Some((idx, consumed)) =
        find_filter_prefix_consumed(all_words.as_slice(), parse_exactly_two_colors_words)
    else {
        return false;
    };
    filter.exactly_two_colors = Some(true);
    all_words.drain(idx..idx + consumed);
    true
}

fn source_counter_tail_consumed(words: &[&str]) -> Option<usize> {
    if words_start_with_phrase(words, ON_THIS_ARTIFACT_TAIL) {
        Some(ON_THIS_ARTIFACT_TAIL.len())
    } else if words_start_with_phrase(words, ON_IT_TAIL) {
        Some(ON_IT_TAIL.len())
    } else {
        None
    }
}

fn comparison_from_mana_value_counter_operator(
    operator: crate::effect::ValueComparisonOperator,
    counter_type: crate::object::CounterType,
) -> crate::filter::Comparison {
    let value = Box::new(crate::effect::Value::CountersOnSource(counter_type));
    match operator {
        crate::effect::ValueComparisonOperator::Equal => {
            crate::filter::Comparison::EqualExpr(value)
        }
        crate::effect::ValueComparisonOperator::NotEqual => {
            crate::filter::Comparison::NotEqualExpr(value)
        }
        crate::effect::ValueComparisonOperator::LessThan => {
            crate::filter::Comparison::LessThanExpr(value)
        }
        crate::effect::ValueComparisonOperator::LessThanOrEqual => {
            crate::filter::Comparison::LessThanOrEqualExpr(value)
        }
        crate::effect::ValueComparisonOperator::GreaterThan => {
            crate::filter::Comparison::GreaterThanExpr(value)
        }
        crate::effect::ValueComparisonOperator::GreaterThanOrEqual => {
            crate::filter::Comparison::GreaterThanOrEqualExpr(value)
        }
    }
}

pub(super) fn parse_mana_value_counters_on_source_words(
    words: &[&str],
) -> Option<(
    crate::filter::Comparison,
    Option<crate::object::CounterType>,
    usize,
)> {
    if !words_start_with_phrase(words, MANA_VALUE_COUNTERS_ON_SOURCE_PREFIX) {
        return None;
    }
    let after_axis = &words[MANA_VALUE_COUNTERS_ON_SOURCE_PREFIX.len()..];
    let (operator, operator_len) = if words_start_with_phrase(after_axis, MANA_VALUE_LTE_WORDS) {
        (
            crate::effect::ValueComparisonOperator::LessThanOrEqual,
            MANA_VALUE_LTE_WORDS.len(),
        )
    } else if words_start_with_phrase(after_axis, MANA_VALUE_GTE_WORDS) {
        (
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            MANA_VALUE_GTE_WORDS.len(),
        )
    } else if words_start_with_phrase(after_axis, MANA_VALUE_EQUAL_WORDS) {
        (
            crate::effect::ValueComparisonOperator::Equal,
            MANA_VALUE_EQUAL_WORDS.len(),
        )
    } else if words_start_with_phrase(after_axis, MANA_VALUE_LT_WORDS) {
        (
            crate::effect::ValueComparisonOperator::LessThan,
            MANA_VALUE_LT_WORDS.len(),
        )
    } else if words_start_with_phrase(after_axis, MANA_VALUE_GT_WORDS) {
        (
            crate::effect::ValueComparisonOperator::GreaterThan,
            MANA_VALUE_GT_WORDS.len(),
        )
    } else {
        return None;
    };
    let mut after_operator = &after_axis[operator_len..];
    let mut optional_article_len = 0usize;
    if after_operator.first().copied() == Some("the") {
        after_operator = &after_operator[1..];
        optional_article_len = 1;
    }
    if !words_start_with_phrase(after_operator, NUMBER_OF_WORDS) {
        return None;
    }
    let counter_idx = NUMBER_OF_WORDS.len();
    let counter_type = parse_counter_type_word(after_operator.get(counter_idx)?)?;
    if !after_operator
        .get(counter_idx + 1)
        .is_some_and(|word| word_is_any(word, COUNTER_OR_COUNTERS_WORDS))
    {
        return None;
    }
    let tail = &after_operator[counter_idx + 2..];
    let tail_len = source_counter_tail_consumed(tail)?;
    let consumed = MANA_VALUE_COUNTERS_ON_SOURCE_PREFIX.len()
        + operator_len
        + optional_article_len
        + NUMBER_OF_WORDS.len()
        + 2
        + tail_len;
    let equality_counter_type =
        (operator == crate::effect::ValueComparisonOperator::Equal).then_some(counter_type);
    Some((
        comparison_from_mana_value_counter_operator(operator, counter_type),
        equality_counter_type,
        consumed,
    ))
}

pub(super) fn try_apply_mana_value_counters_on_source_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((idx, (comparison, equality_counter_type, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_mana_value_counters_on_source_words(&all_words[idx..])
                .map(|matched| (idx, matched))
        })
    else {
        return false;
    };
    if let Some(counter_type) = equality_counter_type {
        filter.mana_value_eq_counters_on_source = Some(counter_type);
    } else {
        filter.mana_value = Some(comparison);
    }
    all_words.drain(idx..idx + consumed);

    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let segment_words = segment_words_view.to_word_refs();
    let segment_match = find_mana_value_counter_phrase_bounds(&segment_words);
    if let Some((start_word_idx, end_word_idx)) = segment_match
        && let Some(token_range) =
            segment_words_view.token_span_for_words(start_word_idx, end_word_idx)
        && token_range.start < token_range.end
        && token_range.end <= segment_tokens.len()
    {
        segment_tokens.drain(token_range);
    }

    true
}

pub(super) fn try_apply_attached_exclusion_phrases(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) {
    let mut idx = 0usize;
    while idx + 2 < all_words.len() {
        if !words_start_with_phrase(&all_words[idx..], OTHER_THAN_PREFIX) {
            idx += 1;
            continue;
        }

        let Some(tag) = (match all_words.get(idx + 2).copied() {
            Some("enchanted") => Some(crate::tag::CompilerReferenceTag::Enchanted.key()),
            Some("equipped") => Some(crate::tag::CompilerReferenceTag::Equipped.key()),
            _ => None,
        }) else {
            idx += 1;
            continue;
        };

        let mut drain_end = idx + 3;
        if all_words
            .get(drain_end)
            .is_some_and(|word| is_demonstrative_object_head(word))
        {
            drain_end += 1;
        }
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        all_words.drain(idx..drain_end);
    }
}

pub(super) fn strip_object_filter_leading_prefixes(all_words: &mut Vec<&str>) {
    while words_start_with_phrase(all_words.as_slice(), ONE_OF_PREFIX) {
        all_words.drain(0..ONE_OF_PREFIX.len());
    }
    while words_start_with_phrase(all_words.as_slice(), DIFFERENT_ONE_OF_PREFIX) {
        all_words.drain(0..DIFFERENT_ONE_OF_PREFIX.len());
    }
    while all_words
        .first()
        .is_some_and(|word| word_is_any(word, OF_OR_FROM_WORDS))
    {
        all_words.remove(0);
    }
}

pub(super) fn parse_spell_filter_power_or_toughness_words(words: &[&str]) -> Option<usize> {
    words_start_with_phrase(words, POWER_OR_TOUGHNESS_PREFIX)
        .then_some(POWER_OR_TOUGHNESS_PREFIX.len())
}

pub(super) fn apply_spell_filter_word_atoms(filter: &mut ObjectFilter, words: &[&str]) {
    if words.iter().any(|word| matches!(*word, "kicked" | "kick")) {
        filter.ability_markers.push("kicked".to_string());
    }
    let mut idx = 0usize;
    while idx < words.len() {
        if let Some((kind, consumed)) = parse_alternative_cast_words(&words[idx..]) {
            filter.alternative_cast = Some(kind);
            idx += consumed;
            continue;
        }

        let word = words[idx];
        // `permanent` is an aggregate spell characteristic rather than a
        // literal card type. Preserve its complete executable domain before
        // applying trailing relative qualifiers such as `that have an
        // Adventure`; otherwise the qualifier survives while the permanent
        // restriction is silently discarded.
        if matches!(word, "permanent" | "permanents") {
            for card_type in [
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ] {
                push_unique_filter_value(&mut filter.card_types, card_type);
            }
            idx += 1;
            continue;
        }
        if word == "non"
            && let Some(subtype) = words.get(idx + 1).copied().and_then(parse_subtype_flexible)
        {
            push_unique_filter_value(&mut filter.excluded_subtypes, subtype);
            idx += 2;
            continue;
        }
        if word == "with"
            && let Some((constraint, consumed)) =
                parse_filter_keyword_constraint_words(&words[idx + 1..])
        {
            apply_filter_keyword_constraint(filter, constraint, false);
            idx += consumed + 1;
            continue;
        }
        if let Some((face_down, consumed)) = parse_filter_face_state_words(&words[idx..]) {
            filter.face_down = Some(face_down);
            idx += consumed;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            push_unique_filter_value(&mut filter.card_types, card_type);
        }
        if let Some(card_type) = parse_non_type(word) {
            push_unique_filter_value(&mut filter.excluded_card_types, card_type);
        }
        if let Some(supertype) = parse_supertype_word(word) {
            push_unique_filter_value(&mut filter.supertypes, supertype);
        }
        if let Some(subtype) = word.strip_prefix("non-").and_then(parse_subtype_flexible) {
            push_unique_filter_value(&mut filter.excluded_subtypes, subtype);
        } else if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique_filter_value(&mut filter.subtypes, subtype);
        }
        if word == COLORLESS_WORD {
            filter.colorless = true;
        }
        if word == MULTICOLORED_WORD {
            filter.multicolored = true;
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or_default();
            filter.colors = Some(existing.union(color));
        }
        idx += 1;
    }
}

pub(super) fn apply_spell_filter_chosen_type_reference(filter: &mut ObjectFilter, words: &[&str]) {
    let has_phrase = |phrase: &[&str]| crate::word_primitives::sequence_occurs(words, phrase);

    if has_phrase(&["chosen", "card", "type"]) {
        filter.chosen_card_type = true;
    }
    if has_phrase(&["chosen", "creature", "type"])
        || has_phrase(&["chosen", "type"])
        || has_phrase(&["that", "type"])
    {
        // The generic "chosen type" surface is intentionally represented by
        // this predicate: runtime matching first consults the source's chosen
        // creature type and then falls back to its chosen card type. That is
        // what lets one spell-filter shape serve both Herald's Horn-style
        // creature choices and Cloud Key-style card-type choices.
        filter.chosen_creature_type = true;
    }
}

pub(super) fn apply_spell_filter_comparisons(
    filter: &mut ObjectFilter,
    words: &[&str],
    clause_words: &[&str],
) {
    let mut cmp_idx = 0usize;
    while cmp_idx < words.len() {
        let Some((axis, axis_word_count)) =
            parse_spell_filter_comparison_axis_words(&words[cmp_idx..])
        else {
            cmp_idx += 1;
            continue;
        };

        let value_tokens = if cmp_idx + axis_word_count < words.len() {
            &words[cmp_idx + axis_word_count..]
        } else {
            &[]
        };
        let parsed = crate::grammar::primitives::probe_shape(parse_filter_comparison_tokens(
            axis.as_str(),
            value_tokens,
            clause_words,
        ))
        .flatten();
        let Some((cmp, consumed)) = parsed else {
            cmp_idx += 1;
            continue;
        };

        axis.assign(filter, cmp);
        cmp_idx += axis_word_count + consumed;
    }
}

pub(super) fn build_spell_filter_power_or_toughness_disjunction(
    filter: &ObjectFilter,
    words: &[&str],
    clause_words: &[&str],
) -> Option<ObjectFilter> {
    for idx in 0..words.len() {
        let Some(consumed) = parse_spell_filter_power_or_toughness_words(&words[idx..]) else {
            continue;
        };
        let value_tokens = if idx + consumed < words.len() {
            &words[idx + consumed..]
        } else {
            &[]
        };
        let Some((cmp, _)) = crate::grammar::primitives::probe_shape(
            parse_filter_comparison_tokens("power", value_tokens, clause_words),
        )
        .flatten() else {
            continue;
        };

        let mut base = filter.clone();
        base.any_of.clear();
        base.power = None;
        base.toughness = None;

        let mut power_branch = base.clone();
        power_branch.power = Some(cmp.clone());

        let mut toughness_branch = base;
        toughness_branch.toughness = Some(cmp);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![power_branch, toughness_branch];
        return Some(disjunction);
    }

    None
}

pub(super) fn parse_spell_filter_from_words(words: &[&str]) -> ObjectFilter {
    let mut filter = ObjectFilter::default();

    apply_spell_filter_word_atoms(&mut filter, words);
    apply_spell_filter_chosen_type_reference(&mut filter, words);
    apply_spell_filter_comparisons(&mut filter, words, words);
    apply_spell_filter_tagged_relations(&mut filter, words);
    apply_spell_filter_source_creature_type_relation(&mut filter, words);
    apply_spell_filter_parity_phrases(words, &mut filter);

    build_spell_filter_power_or_toughness_disjunction(&filter, words, words).unwrap_or(filter)
}

fn apply_spell_filter_source_creature_type_relation(filter: &mut ObjectFilter, words: &[&str]) {
    let shares_creature_type = words_contain_all(words, &["creature", "type"])
        && words_contain_any_word(words, SHARE_WORDS);
    let references_source = find_any_phrase_start(
        words,
        &[
            &["with", "this", "creature"],
            &["with", "this", "permanent"],
        ],
    )
    .is_some();
    if shares_creature_type && references_source {
        filter.shares_creature_type_with_source = true;
    }
}

fn apply_spell_filter_tagged_relations(filter: &mut ObjectFilter, words: &[&str]) {
    let shares_card_type = find_any_phrase_start(words, SHARED_CARD_TYPE_PHRASES).is_some()
        && words_contain_any_word(words, SHARE_WORDS);
    let references_exiled_card =
        find_any_phrase_start(words, EXILED_CARD_REFERENCE_PHRASES).is_some();

    if shares_card_type && references_exiled_card {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::SourceExiled.key(),
            relation: shared_type_relation(words),
        });
    }
}

pub(super) fn parse_with_no_abilities_words(words: &[&str]) -> Option<usize> {
    (words.len() >= 2 && words[0] == NO_WORD && word_is_any(words[1], ABILITY_OR_ABILITIES_WORDS))
        .then_some(2)
}

pub(super) fn try_apply_with_clause_tail(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if let Some((sticker, consumed)) = parse_sticker_filter_words(words) {
        filter.sticker = Some(sticker);
        return Some(consumed);
    }

    if let Some(consumed) = parse_with_no_abilities_words(words) {
        filter.no_abilities = true;
        return Some(consumed);
    }

    if words.first().is_some_and(|word| *word == NO_WORD)
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&words[1..])
    {
        filter.without_counter = Some(counter_constraint);
        return Some(1 + consumed);
    }

    if let Some((kind, consumed)) = parse_alternative_cast_words(words) {
        filter.alternative_cast = Some(kind);
        return Some(consumed);
    }
    if let Some((counter_constraint, consumed)) = parse_filter_counter_constraint_words(words) {
        filter.with_counter = Some(counter_constraint);
        return Some(consumed);
    }

    if let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(words) {
        if words.get(consumed).is_some_and(|word| *word == OR_WORD)
            && let Some((rhs_constraint, rhs_consumed)) =
                parse_filter_keyword_constraint_words(&words[consumed + 1..])
        {
            let mut left = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut left, constraint, false);
            let mut right = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut right, rhs_constraint, false);
            filter.any_of = vec![left, right];
            return Some(consumed + 1 + rhs_consumed);
        }

        apply_filter_keyword_constraint(filter, constraint, false);
        return Some(consumed);
    }

    None
}

pub(super) fn try_apply_without_clause_tail(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(words) {
        apply_filter_keyword_constraint(filter, constraint, true);
        return Some(consumed);
    }
    if let Some((counter_constraint, consumed)) = parse_filter_counter_constraint_words(words) {
        filter.without_counter = Some(counter_constraint);
        return Some(consumed);
    }

    None
}

pub(super) fn apply_spell_filter_parity_phrases(words: &[&str], filter: &mut ObjectFilter) {
    if find_any_phrase_start(words, ODD_MANA_VALUE_PHRASES).is_some() {
        filter.mana_value_parity = Some(crate::filter::ParityRequirement::Odd);
    }
    if find_any_phrase_start(words, EVEN_MANA_VALUE_PHRASES).is_some() {
        filter.mana_value_parity = Some(crate::filter::ParityRequirement::Even);
    }
    if find_phrase_start(words, ODD_POWER_PHRASE).is_some() {
        filter.power_parity = Some(crate::filter::ParityRequirement::Odd);
    }
    if find_phrase_start(words, EVEN_POWER_PHRASE).is_some() {
        filter.power_parity = Some(crate::filter::ParityRequirement::Even);
    }
}

pub(super) fn find_any_filter_phrase_start(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    find_any_phrase_start(words, phrases)
}

pub(super) fn find_mana_value_counter_phrase_bounds(words: &[&str]) -> Option<(usize, usize)> {
    (0..words.len()).find_map(|idx| {
        let (_, _, consumed) = parse_mana_value_counters_on_source_words(&words[idx..])?;
        Some((idx, idx + consumed))
    })
}

pub(super) fn attacking_player_filter_from_words(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<PlayerFilter> {
    if find_any_phrase_start(words, ATTACKING_THAT_PLAYER_PHRASES).is_some() {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if find_any_phrase_start(words, ATTACKING_DEFENDING_PLAYER_PHRASES).is_some() {
        return Some(PlayerFilter::Defending);
    }
    if find_any_phrase_start(words, ATTACKING_TARGET_PLAYER_PHRASES).is_some() {
        return Some(PlayerFilter::target_player());
    }
    if find_any_phrase_start(words, ATTACKING_TARGET_OPPONENT_PHRASES).is_some() {
        return Some(PlayerFilter::target_opponent());
    }
    if find_any_phrase_start(words, ATTACKING_CHOSEN_PLAYER_PHRASES).is_some() {
        return Some(PlayerFilter::ChosenPlayer);
    }
    if find_phrase_start(words, ATTACKING_YOU_PHRASE).is_some() {
        return Some(PlayerFilter::You);
    }
    if find_phrase_start(words, ATTACKING_THEM_PHRASE).is_some() {
        return Some(pronoun_player_filter.clone());
    }
    if find_any_phrase_start(words, ATTACKING_OPPONENT_PHRASES).is_some() {
        return Some(PlayerFilter::Opponent);
    }

    None
}

pub(super) struct ReferenceTagStageResult {
    pub(super) source_linked_exile_reference: bool,
    pub(super) early_return: bool,
}

fn find_blocking_or_blocked_by_source_phrase(words: &[&str]) -> Option<usize> {
    find_any_filter_phrase_start(
        words,
        &[
            &["blocking", "or", "blocked", "by", "this", "creature"],
            &["blocking", "or", "blocked", "by", "this", "permanent"],
            &["blocking", "or", "blocked", "by", "this", "source"],
        ],
    )
    .or_else(|| {
        const BLOCKING_OR_BLOCKED_BY_PHRASE: &[&str] = &["blocking", "or", "blocked", "by"];
        let mut idx = 0usize;
        while idx + BLOCKING_OR_BLOCKED_BY_PHRASE.len() <= words.len() {
            if words_start_with_phrase(&words[idx..], BLOCKING_OR_BLOCKED_BY_PHRASE)
                && is_source_reference_words(&words[idx + BLOCKING_OR_BLOCKED_BY_PHRASE.len()..])
            {
                return Some(idx);
            }
            idx += 1;
        }
        None
    })
}

/// Find the directional combat relation in object phrases such as
/// "creature blocking it" and "creature blocking this creature".
///
/// `blocking` by itself only means that the candidate is blocking *some*
/// attacker.  The following source reference narrows it to the current
/// grammatical source.  Keep that direction separate from the existing
/// symmetric "blocking or blocked by" spelling: combining `blocking` with
/// `in_combat_with_source` represents exactly a blocker of the source.
fn find_blocking_source_phrase(words: &[&str]) -> Option<usize> {
    words.iter().enumerate().find_map(|(idx, word)| {
        if *word != "blocking" {
            return None;
        }
        let reference = &words[idx + 1..];
        (!reference.is_empty()
            && (crate::word_primitives::parse_sequence_complete(reference, &["it"])
                || is_source_reference_words(reference)))
        .then_some(idx)
    })
}

fn source_reference_prefix_surface(
    words: &[&str],
    segment_tokens: &[OwnedLexToken],
) -> Option<(usize, crate::target::SourceReferenceSurface)> {
    for prefix_len in (1..=words.len()).rev() {
        if prefix_len == 1 && words.len() > 1 {
            continue;
        }
        let prefix = &words[..prefix_len];
        let Some(surface) = source_reference_surface_for_words(prefix)
            .or_else(|| this_source_surface_for_words(prefix))
        else {
            continue;
        };
        // In object-filter grammar, a valid subtype noun outranks a coincident
        // short alias of the current source. This matters for names beginning
        // with a subtype (for example, "Time Lord Regeneration"): "target
        // Time Lord you control" and "a Time Lord creature card" describe a
        // typed object, not the named source. A complete longer source name is
        // unaffected because its prefix length is greater than the subtype.
        if super::reference_tag_stage::compound_filter_subtype_prefix_word_len(words)
            == Some(prefix_len)
        {
            continue;
        }
        if source_reference_prefix_is_unquoted(prefix, segment_tokens) {
            let surface = match surface {
                crate::target::SourceReferenceSurface::ThisPermanentType(_) => {
                    let token_len = super::reference_tag_stage::token_index_after_word_prefix(
                        segment_tokens,
                        prefix_len,
                    )?;
                    crate::target::SourceReferenceSurface::ThisPermanentType(
                        crate::lexer::render_token_slice(&segment_tokens[..token_len])
                            .trim()
                            .to_string(),
                    )
                }
                surface => surface,
            };
            return Some((prefix_len, surface));
        }
    }
    None
}

fn source_reference_prefix_is_unquoted(
    prefix_words: &[&str],
    segment_tokens: &[OwnedLexToken],
) -> bool {
    if !segment_tokens.iter().any(OwnedLexToken::is_quote) {
        return true;
    }

    let mut outside_tokens = Vec::new();
    let mut inside_quotes = false;
    for token in segment_tokens {
        if token.is_quote() {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes {
            outside_tokens.push(token.clone());
        }
    }

    let outside_view = GrammarFilterNormalizedWords::new(outside_tokens.as_slice());
    let outside_words = outside_view.to_word_refs();
    let outside_words = non_article_word_refs(&outside_words);
    outside_words
        .get(..prefix_words.len())
        .is_some_and(|head| head.iter().copied().eq(prefix_words.iter().copied()))
}

fn drain_source_reference_prefix_tokens(
    segment_tokens: &mut Vec<OwnedLexToken>,
    prefix_word_len: usize,
) {
    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let Some(end_token_idx) = segment_words_view.map_word_or_end_to_token_boundary(prefix_word_len)
    else {
        return;
    };
    segment_tokens.drain(..end_token_idx);
}

#[path = "naming_and_reference/naming_and_reference_reference.rs"]
mod naming_and_reference_reference_programs;
pub(super) use naming_and_reference_reference_programs::apply_reference_and_tag_stage;
