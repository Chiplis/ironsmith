use super::*;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

const ENTERED_SINCE_LAST_TURN_WITH_THAT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "entered", "since", "your", "last", "turn", "ended"]);
const ENTERED_SINCE_LAST_TURN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["entered", "since", "your", "last", "turn", "ended"]);
const COLOR_OR_COLORS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["color"], &["colors"]]);
const NOT_ALL_COLORS_WITH_THAT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "isnt", "all", "colors"]);
const NOT_ALL_COLORS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["isnt", "all", "colors"]);
const NOT_EXACTLY_TWO_COLORS_WITH_THAT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "isnt", "exactly", "two", "colors"]);
const NOT_EXACTLY_TWO_COLORS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["isnt", "exactly", "two", "colors"]);
const MANA_VALUE_EQUAL_COUNTERS_ON_SOURCE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["with", "mana", "value", "equal", "to", "number", "of"]);
const COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const ON_THIS_ARTIFACT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["on", "this", "artifact"]);
const OTHER_THAN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["other", "than"]);
const ONE_OF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["one", "of"]);
const DIFFERENT_ONE_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["different", "one", "of"]);
const OF_OR_FROM_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["of"], &["from"]]);
const POWER_OR_TOUGHNESS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["power", "or", "toughness"]);
const NAMED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["named"]);
const IT_OR_THEM_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);
const REVEALED_CARD_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["revealed", "card"], &["revealed", "cards"]]);
const EXILED_CARD_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["exiled", "card"], &["exiled", "cards"]]);
const ENCHANTED_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["enchanted", "player"]);
const ATTACHED_TO_TAGGED_OBJECT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["it"],
            &["that", "object"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "equipment"],
            &["that", "aura"],
        ]
);
const THAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that"]);
const BE_VERB_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["were"], &["was"], &["is"], &["are"]]);
const NOT_NAMED_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["not", "named"]);
const SINGLE_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["single", "graveyard"]);
const ITS_ATTACHED_TO_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["its", "attached", "to"]);
const EXILED_WITH_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["exiled", "with"]);
const USED_TO_CRAFT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["used", "to", "craft"]);
const REFERENCE_HEAD_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["that"], &["the"], &["it"], &["them"]]);
const EXILED_CARD_OWNER_TAIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you"],
            &["your"],
            &["they"],
            &["their"],
            &["own"],
            &["owns"]
        ]
);
const REFERENCE_OBJECT_NOUN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["artifact"],
            &["creature"],
            &["enchantment"],
            &["land"],
            &["permanent"],
            &["planeswalker"],
            &["card"],
            &["spell"],
            &["source"],
        ]
);
const SHARES_CARD_TYPE_WITH_TAGGED_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["type", "it"];
    contains_any_words & [&["share", "shares"], &["card", "permanent"]]
);
const SHARES_COLOR_WITH_TAGGED_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["shares", "color", "it"]);
const SHARES_CARD_TYPE_WORDS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["card", "type"]; contains_any_words & [&["share", "shares"]]);
const EXILED_CARD_REFERENCE_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
        &["with", "exiled", "card"],
        &["with", "exiled", "cards"],
        &["with", "the", "exiled", "card"],
        &["with", "the", "exiled", "cards"],
    ]]);
const NO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["no"]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const ABILITY_OR_ABILITIES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["ability"], &["abilities"]]);
const COLORLESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["colorless"]);
const MULTICOLORED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["multicolored"]);
const POWER_TOUGHNESS_STICKER_ON_IT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["a", "power", "and", "toughness", "sticker", "on", "it"],
            &["power", "and", "toughness", "sticker", "on", "it"],
        ]
);
const ODD_MANA_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["odd", "mana", "value"], &["odd", "mana", "values"]]]
);
const EVEN_MANA_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["even", "mana", "value"], &["even", "mana", "values"]]]
);
const ODD_POWER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["odd", "power"]]);
const EVEN_POWER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["even", "power"]]);
const MANA_VALUE_EQUAL_THE_COUNTERS_ON_SOURCE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "with", "mana", "value", "equal", "to", "the", "number", "of"
        ]
);
const SAME_MANA_VALUE_AS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["same", "mana", "value", "as"]]);
const EQUAL_OR_LESSER_MANA_VALUE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["equal", "or", "lesser", "mana", "value"]]);
const LTE_MANA_VALUE_THAN_THAT_SPELL_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
        &[
            "equal", "or", "lesser", "mana", "value", "than", "that", "spell",
        ],
        &[
            "less", "than", "or", "equal", "to", "that", "spells", "mana", "value",
        ],
    ]]);
const LTE_MANA_VALUE_AS_TAGGED_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
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
    ]]);
const LESSER_MANA_VALUE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["lesser", "mana", "value"]]);
const SACRIFICE_COST_OBJECT_REFERENCE_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
        &["the", "sacrificed", "creature"],
        &["the", "sacrificed", "artifact"],
        &["the", "sacrificed", "permanent"],
        &["a", "sacrificed", "creature"],
        &["a", "sacrificed", "artifact"],
        &["a", "sacrificed", "permanent"],
        &["sacrificed", "creature"],
        &["sacrificed", "artifact"],
        &["sacrificed", "permanent"],
    ]]);
const IT_OR_ITS_REFERENCE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["it", "its"]]);
const ATTACKING_THAT_PLAYER_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
        &["attacking", "that", "player"],
        &["attacking", "that", "players"],
    ]]);
const ATTACKING_DEFENDING_PLAYER_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
        &["attacking", "defending", "player"],
        &["attacking", "defending", "players"],
    ]]);
const ATTACKING_TARGET_PLAYER_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
        &["attacking", "target", "player"],
        &["attacking", "target", "players"],
    ]]);
const ATTACKING_TARGET_OPPONENT_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
        &["attacking", "target", "opponent"],
        &["attacking", "target", "opponents"],
    ]]);
const ATTACKING_YOU_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["attacking", "you"]]);
const ATTACKING_THEM_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["attacking", "them"]]);
const ATTACKING_OPPONENT_PATTERN: ClauseShape<'static> =
    ClauseShape::new().contains_any_phrases(&[&[
        &["attacking", "opponent"],
        &["attacking", "opponents"],
        &["attacking", "one", "of", "your", "opponents"],
    ]]);
const EQUIPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equipped"]);
const ENCHANTED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["enchanted"]);
const ATTACHED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["attached"]);
const TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const CONVOKED_THIS_SPELL_TAG_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
        &["that", "convoked", "this", "spell"],
        &["that", "convoked", "it"],
    ]]);
const CREWED_IT_THIS_TURN_TAG_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["that", "crewed", "it", "this", "turn"]]);
const SADDLED_IT_THIS_TURN_TAG_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["that", "saddled", "it", "this", "turn"]]);
const AMASSED_ARMY_TAG_PATTERN: ClauseShape<'static> =
    ClauseShape::new().contains_any_phrases(&[&[
        &["army", "you", "amassed"],
        &["amassed", "army"],
        &["amassed", "armys"],
    ]]);
const THIS_WAY_TAG_PATTERN: ClauseShape<'static> = ClauseShape::new().contains_any_phrases(&[&[
    &["exiled", "this", "way"],
    &["destroyed", "this", "way"],
    &["sacrificed", "this", "way"],
    &["revealed", "this", "way"],
    &["discarded", "this", "way"],
    &["milled", "this", "way"],
]]);
const TAGGED_OBJECT_REFERENCE_FOR_MANA_VALUE_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
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
    ]]);
const SAME_NAME_AS_TAGGED_OBJECT_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
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
    ]]);

fn find_any_phrase_start(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    phrases.iter().find_map(|phrase| {
        words
            .windows(phrase.len())
            .position(|window| window == *phrase)
    })
}

pub(super) fn remove_word_range(words: &mut Vec<&str>, start: usize, end: usize) {
    let mut remaining = Vec::with_capacity(words.len());
    remaining.extend_from_slice(&words[..start]);
    remaining.extend_from_slice(&words[end..]);
    *words = remaining;
}

pub(super) fn try_apply_not_named_clause<'a, F, G>(
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
    let Some(not_named_idx) = NOT_NAMED_PATTERN.find_exact_window(all_words.as_slice(), 2) else {
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
    filter.excluded_name = Some(name);
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
    let Some(named_idx) = lower_words_find_index(all_words.as_slice(), |word| {
        NAMED_WORD_PATTERN.matches_word(word)
    }) else {
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
    if ENTERED_SINCE_LAST_TURN_WITH_THAT_PREFIX_PATTERN.matches_words(words) {
        Some(7)
    } else if ENTERED_SINCE_LAST_TURN_PREFIX_PATTERN.matches_words(words) {
        Some(6)
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
    while let Some(idx) = SINGLE_GRAVEYARD_PATTERN.find_exact_window(all_words.as_slice(), 2) {
        filter.single_graveyard = true;
        all_words.remove(idx);
    }
}

pub(super) fn parse_color_count_phrase_words(words: &[&str]) -> Option<(u32, usize)> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let (count, used) =
        parse_greater_than_or_equal_quantity_prefix(&tokens, false, false, "color-count phrase")
            .ok()
            .flatten()?;
    words
        .get(used)
        .is_some_and(|word| COLOR_OR_COLORS_WORD_PATTERN.matches_word(word))
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
    let Some((power, toughness)) = all_words
        .first()
        .and_then(|word| parse_unsigned_pt_word(word))
    else {
        return false;
    };
    filter.power = Some(crate::filter::Comparison::Equal(power));
    filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
    all_words.remove(0);
    true
}

pub(super) fn parse_not_all_colors_words(words: &[&str]) -> Option<usize> {
    if NOT_ALL_COLORS_WITH_THAT_PREFIX_PATTERN.matches_words(words) {
        Some(4)
    } else if NOT_ALL_COLORS_PREFIX_PATTERN.matches_words(words) {
        Some(3)
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

pub(super) fn parse_not_exactly_two_colors_words(words: &[&str]) -> Option<usize> {
    if NOT_EXACTLY_TWO_COLORS_WITH_THAT_PREFIX_PATTERN.matches_words(words) {
        Some(5)
    } else if NOT_EXACTLY_TWO_COLORS_PREFIX_PATTERN.matches_words(words) {
        Some(4)
    } else {
        None
    }
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

pub(super) fn parse_mana_value_eq_counters_on_source_words(
    words: &[&str],
) -> Option<(crate::object::CounterType, usize)> {
    if !MANA_VALUE_EQUAL_COUNTERS_ON_SOURCE_PREFIX_PATTERN.matches_words(words)
        || !words
            .get(8)
            .is_some_and(|word| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_words(&[*word]))
        || !ON_THIS_ARTIFACT_TAIL_PATTERN.matches_words(&words[9..])
    {
        return None;
    }
    let counter_type = parse_counter_type_word(*words.get(7)?)?;
    Some((counter_type, 12))
}

pub(super) fn try_apply_mana_value_eq_counters_on_source_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((idx, (counter_type, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_mana_value_eq_counters_on_source_words(&all_words[idx..])
                .map(|matched| (idx, matched))
        })
    else {
        return false;
    };
    filter.mana_value_eq_counters_on_source = Some(counter_type);
    all_words.drain(idx..idx + consumed);

    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let segment_words = segment_words_view.to_word_refs();
    let segment_match = find_mana_value_equal_counter_phrase_bounds(&segment_words);
    if let Some((start_word_idx, end_word_idx)) = segment_match
        && let Some(start_token_idx) = segment_words_view.token_index_for_word_index(start_word_idx)
    {
        let end_token_idx = segment_words_view
            .token_index_after_words(end_word_idx)
            .unwrap_or(segment_tokens.len());
        if start_token_idx < end_token_idx && end_token_idx <= segment_tokens.len() {
            segment_tokens.drain(start_token_idx..end_token_idx);
        }
    }

    true
}

pub(super) fn try_apply_attached_exclusion_phrases(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) {
    let mut idx = 0usize;
    while idx + 2 < all_words.len() {
        if !OTHER_THAN_PREFIX_PATTERN.matches_words(&all_words[idx..]) {
            idx += 1;
            continue;
        }

        let Some(tag) = (match all_words.get(idx + 2).copied() {
            Some("enchanted") => Some(TagKey::from("enchanted")),
            Some("equipped") => Some(TagKey::from("equipped")),
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
    while ONE_OF_PREFIX_PATTERN.matches_words(all_words.as_slice()) {
        all_words.drain(0..2);
    }
    while DIFFERENT_ONE_OF_PREFIX_PATTERN.matches_words(all_words.as_slice()) {
        all_words.drain(0..3);
    }
    while all_words
        .first()
        .is_some_and(|word| OF_OR_FROM_WORD_PATTERN.matches_word(word))
    {
        all_words.remove(0);
    }
}

pub(super) fn parse_spell_filter_power_or_toughness_words(words: &[&str]) -> Option<usize> {
    POWER_OR_TOUGHNESS_PREFIX_PATTERN
        .matches_words(words)
        .then_some(3)
}

pub(super) fn apply_spell_filter_word_atoms(filter: &mut ObjectFilter, words: &[&str]) {
    let mut idx = 0usize;
    while idx < words.len() {
        if let Some((kind, consumed)) = parse_alternative_cast_words(&words[idx..]) {
            filter.alternative_cast = Some(kind);
            idx += consumed;
            continue;
        }

        let word = words[idx];
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
        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique_filter_value(&mut filter.subtypes, subtype);
        }
        if COLORLESS_WORD_PATTERN.matches_word(word) {
            filter.colorless = true;
        }
        if MULTICOLORED_WORD_PATTERN.matches_word(word) {
            filter.multicolored = true;
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
        idx += 1;
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
        let parsed = parse_filter_comparison_tokens(axis.as_str(), value_tokens, clause_words)
            .ok()
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
        let Some((cmp, _)) = parse_filter_comparison_tokens("power", value_tokens, clause_words)
            .ok()
            .flatten()
        else {
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
    apply_spell_filter_comparisons(&mut filter, words, words);
    apply_spell_filter_tagged_relations(&mut filter, words);
    apply_spell_filter_parity_phrases(words, &mut filter);

    build_spell_filter_power_or_toughness_disjunction(&filter, words, words).unwrap_or(filter)
}

fn apply_spell_filter_tagged_relations(filter: &mut ObjectFilter, words: &[&str]) {
    let shares_card_type = SHARES_CARD_TYPE_WORDS_PATTERN.matches_words(words);
    let references_exiled_card = EXILED_CARD_REFERENCE_PATTERN.matches_words(words);

    if shares_card_type && references_exiled_card {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            relation: TaggedOpbjectRelation::SharesCardType,
        });
    }
}

pub(super) fn parse_with_no_abilities_words(words: &[&str]) -> Option<usize> {
    (words.len() >= 2
        && NO_WORD_PATTERN.matches_word(words[0])
        && ABILITY_OR_ABILITIES_WORD_PATTERN.matches_word(words[1]))
    .then_some(2)
}

pub(super) fn try_apply_with_clause_tail(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    if POWER_TOUGHNESS_STICKER_ON_IT_PATTERN.matches_words(words) {
        *filter = filter
            .clone()
            .with_ability_marker("a power and toughness sticker on it");
        return Some(if ARTICLE_WORD_PATTERN.matches_word(words[0]) {
            7
        } else {
            6
        });
    }

    if let Some(consumed) = parse_with_no_abilities_words(words) {
        filter.no_abilities = true;
        return Some(consumed);
    }

    if words
        .first()
        .is_some_and(|word| NO_WORD_PATTERN.matches_word(word))
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
        if words
            .get(consumed)
            .is_some_and(|word| OR_WORD_PATTERN.matches_word(word))
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
    let mut consumed = 0usize;
    let mut applied_keyword = false;
    while consumed < words.len() {
        while consumed < words.len()
            && matches!(words[consumed], "," | "or" | "and")
        {
            consumed += 1;
        }
        if let Some((ability, used)) = parse_without_static_ability_keyword(&words[consumed..]) {
            if !slice_contains(filter.excluded_static_abilities.as_slice(), &ability) {
                filter.excluded_static_abilities.push(ability);
            }
            consumed += used;
            applied_keyword = true;
            continue;
        }
        let Some((constraint, used)) = parse_filter_keyword_constraint_words(&words[consumed..])
        else {
            break;
        };
        apply_filter_keyword_constraint(filter, constraint, true);
        consumed += used;
        applied_keyword = true;
    }
    if applied_keyword {
        return Some(consumed);
    }
    if let Some((counter_constraint, consumed)) = parse_filter_counter_constraint_words(words) {
        filter.without_counter = Some(counter_constraint);
        return Some(consumed);
    }

    None
}

fn parse_without_static_ability_keyword(
    words: &[&str],
) -> Option<(crate::static_abilities::StaticAbilityId, usize)> {
    use crate::static_abilities::StaticAbilityId;

    if words.get(0..2) == Some(&["first", "strike"]) {
        return Some((StaticAbilityId::FirstStrike, 2));
    }
    if words
        .first()
        .is_some_and(|word| matches!(*word, "first-strike" | "firststrike"))
    {
        return Some((StaticAbilityId::FirstStrike, 1));
    }
    if words.get(0..2) == Some(&["double", "strike"]) {
        return Some((StaticAbilityId::DoubleStrike, 2));
    }
    if words
        .first()
        .is_some_and(|word| matches!(*word, "double-strike" | "doublestrike"))
    {
        return Some((StaticAbilityId::DoubleStrike, 1));
    }

    None
}

pub(super) fn apply_spell_filter_parity_phrases(words: &[&str], filter: &mut ObjectFilter) {
    if ODD_MANA_VALUE_PATTERN.matches_words(words) {
        filter.mana_value_parity = Some(crate::filter::ParityRequirement::Odd);
    }
    if EVEN_MANA_VALUE_PATTERN.matches_words(words) {
        filter.mana_value_parity = Some(crate::filter::ParityRequirement::Even);
    }
    if ODD_POWER_PATTERN.matches_words(words) {
        filter.power_parity = Some(crate::filter::ParityRequirement::Odd);
    }
    if EVEN_POWER_PATTERN.matches_words(words) {
        filter.power_parity = Some(crate::filter::ParityRequirement::Even);
    }
}

pub(super) fn find_any_filter_phrase_start(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    find_any_phrase_start(words, phrases)
}

pub(super) fn find_mana_value_equal_counter_phrase_bounds(
    words: &[&str],
) -> Option<(usize, usize)> {
    (0..words.len()).find_map(|idx| {
        let tail = &words[idx..];
        if tail.len() >= 13
            && MANA_VALUE_EQUAL_THE_COUNTERS_ON_SOURCE_PREFIX_PATTERN.matches_words(tail)
            && parse_counter_type_word(tail[8]).is_some()
            && COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(tail[9])
            && ON_THIS_ARTIFACT_TAIL_PATTERN.matches_words(&tail[10..])
        {
            return Some((idx, idx + 13));
        }
        if tail.len() >= 12
            && MANA_VALUE_EQUAL_COUNTERS_ON_SOURCE_PREFIX_PATTERN.matches_words(tail)
            && parse_counter_type_word(tail[7]).is_some()
            && COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(tail[8])
            && ON_THIS_ARTIFACT_TAIL_PATTERN.matches_words(&tail[9..])
        {
            return Some((idx, idx + 12));
        }
        None
    })
}

pub(super) fn attacking_player_filter_from_words(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<PlayerFilter> {
    if ATTACKING_THAT_PLAYER_PATTERN.matches_words(words) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if ATTACKING_DEFENDING_PLAYER_PATTERN.matches_words(words) {
        return Some(PlayerFilter::Defending);
    }
    if ATTACKING_TARGET_PLAYER_PATTERN.matches_words(words) {
        return Some(PlayerFilter::target_player());
    }
    if ATTACKING_TARGET_OPPONENT_PATTERN.matches_words(words) {
        return Some(PlayerFilter::target_opponent());
    }
    if ATTACKING_YOU_PATTERN.matches_words(words) {
        return Some(PlayerFilter::You);
    }
    if ATTACKING_THEM_PATTERN.matches_words(words) {
        return Some(pronoun_player_filter.clone());
    }
    if ATTACKING_OPPONENT_PATTERN.matches_words(words) {
        return Some(PlayerFilter::Opponent);
    }

    None
}

pub(super) struct ReferenceTagStageResult {
    pub(super) source_linked_exile_reference: bool,
    pub(super) early_return: bool,
}

pub(super) fn apply_reference_and_tag_stage(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> ReferenceTagStageResult {
    if all_words
        .first()
        .is_some_and(|word| EQUIPPED_WORD_PATTERN.matches_word(word))
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("equipped"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    } else if all_words
        .first()
        .is_some_and(|word| ENCHANTED_WORD_PATTERN.matches_word(word))
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("enchanted"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    }

    if is_source_reference_words(all_words) {
        filter.source = true;
    }

    if let Some(its_attached_idx) = ITS_ATTACHED_TO_PATTERN.find_exact_window(all_words, 3) {
        let mut normalized = Vec::with_capacity(all_words.len() + 1);
        normalized.extend_from_slice(&all_words[..its_attached_idx]);
        normalized.extend(["attached", "to", "it"]);
        normalized.extend_from_slice(&all_words[its_attached_idx + 3..]);
        *all_words = normalized;
    }

    if let Some(attached_idx) =
        lower_words_find_index(all_words, |word| ATTACHED_WORD_PATTERN.matches_word(word))
        && all_words
            .get(attached_idx + 1)
            .is_some_and(|word| TO_WORD_PATTERN.matches_word(word))
    {
        let attached_to_words = &all_words[attached_idx + 2..];
        if ENCHANTED_PLAYER_PREFIX_PATTERN.matches_words(attached_to_words) {
            let trim_start = if attached_idx >= 2
                && THAT_WORD_PATTERN.matches_word(all_words[attached_idx - 2])
                && BE_VERB_WORD_PATTERN.matches_word(all_words[attached_idx - 1])
            {
                attached_idx - 2
            } else {
                attached_idx
            };
            all_words.truncate(trim_start);
            filter.attached_to_player = Some(PlayerFilter::TaggedPlayer(TagKey::from("enchanted")));
            return ReferenceTagStageResult {
                source_linked_exile_reference: false,
                early_return: false,
            };
        }
        let references_it =
            ATTACHED_TO_TAGGED_OBJECT_PREFIX_PATTERN.matches_words(attached_to_words);
        if references_it {
            let trim_start = if attached_idx >= 2
                && THAT_WORD_PATTERN.matches_word(all_words[attached_idx - 2])
                && BE_VERB_WORD_PATTERN.matches_word(all_words[attached_idx - 1])
            {
                attached_idx - 2
            } else {
                attached_idx
            };
            all_words.truncate(trim_start);
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: IT_TAG.into(),
                relation: TaggedOpbjectRelation::AttachedToTaggedObject,
            });
        }
    }

    if let Some(relation_idx) = find_any_filter_phrase_start(
        all_words,
        &[
            &["blocking", "or", "blocked", "by", "this", "creature"],
            &["blocking", "or", "blocked", "by", "this", "permanent"],
            &["blocking", "or", "blocked", "by", "this", "source"],
        ],
    ) {
        filter.in_combat_with_source = true;
        all_words.truncate(relation_idx);
    }

    let starts_with_exiled_card = EXILED_CARD_PREFIX_PATTERN.matches_words(all_words);
    if starts_with_exiled_card {
        filter.zone.get_or_insert(Zone::Exile);
    }
    let has_exiled_with_phrase = EXILED_WITH_PATTERN
        .find_exact_window(all_words, 2)
        .is_some();
    let has_used_to_craft_phrase = USED_TO_CRAFT_PATTERN
        .find_exact_window(all_words, 3)
        .is_some();
    let owner_only_tail_after_exiled_cards = starts_with_exiled_card
        && all_words
            .iter()
            .skip(2)
            .all(|word| EXILED_CARD_OWNER_TAIL_WORD_PATTERN.matches_word(word));
    let is_source_linked_exile_reference = has_exiled_with_phrase
        || (starts_with_exiled_card
            && (all_words.len() == 2
                || owner_only_tail_after_exiled_cards
                || has_used_to_craft_phrase));
    let mut source_linked_exile_reference = false;
    if is_source_linked_exile_reference {
        source_linked_exile_reference = true;
        filter.zone = Some(Zone::Exile);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        if let Some(exiled_with_idx) = EXILED_WITH_PATTERN.find_exact_window(all_words, 2) {
            let mut reference_end = exiled_with_idx + 2;
            if all_words
                .get(reference_end)
                .is_some_and(|word| REFERENCE_HEAD_WORD_PATTERN.matches_word(word))
            {
                reference_end += 1;
            }
            if all_words
                .get(reference_end)
                .is_some_and(|word| REFERENCE_OBJECT_NOUN_WORD_PATTERN.matches_word(word))
            {
                reference_end += 1;
            }
            if reference_end > exiled_with_idx + 1 {
                all_words.drain(exiled_with_idx + 1..reference_end);
            }
        }
        if let Some(used_to_craft_idx) = USED_TO_CRAFT_PATTERN.find_exact_window(all_words, 3) {
            let mut reference_end = used_to_craft_idx + 3;
            if all_words
                .get(reference_end)
                .is_some_and(|word| REFERENCE_HEAD_WORD_PATTERN.matches_word(word))
            {
                reference_end += 1;
            }
            if all_words
                .get(reference_end)
                .is_some_and(|word| REFERENCE_OBJECT_NOUN_WORD_PATTERN.matches_word(word))
            {
                reference_end += 1;
            }
            all_words.drain(used_to_craft_idx..reference_end);
        }
        let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
        let segment_words = segment_words_view.to_word_refs();
        if let Some(exiled_with_idx) = EXILED_WITH_PATTERN.find_exact_window(&segment_words, 2)
            && let Some(exiled_with_token_idx) =
                segment_words_view.token_index_for_word_index(exiled_with_idx)
        {
            let mut reference_end = exiled_with_token_idx + 2;
            if segment_tokens
                .get(reference_end)
                .is_some_and(|token| REFERENCE_HEAD_WORD_PATTERN.matches_token(token))
            {
                reference_end += 1;
            }
            if segment_tokens
                .get(reference_end)
                .is_some_and(|token| REFERENCE_OBJECT_NOUN_WORD_PATTERN.matches_token(token))
            {
                reference_end += 1;
            }
            if reference_end > exiled_with_idx + 1 {
                segment_tokens.drain(exiled_with_token_idx + 1..reference_end);
            }
        }
        let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
        let segment_words = segment_words_view.to_word_refs();
        if let Some(used_to_craft_idx) = USED_TO_CRAFT_PATTERN.find_exact_window(&segment_words, 3)
            && let Some(used_to_craft_token_idx) =
                segment_words_view.token_index_for_word_index(used_to_craft_idx)
        {
            let mut reference_end = used_to_craft_token_idx + 3;
            if segment_tokens
                .get(reference_end)
                .is_some_and(|token| REFERENCE_HEAD_WORD_PATTERN.matches_token(token))
            {
                reference_end += 1;
            }
            if segment_tokens
                .get(reference_end)
                .is_some_and(|token| REFERENCE_OBJECT_NOUN_WORD_PATTERN.matches_token(token))
            {
                reference_end += 1;
            }
            segment_tokens.drain(used_to_craft_token_idx..reference_end);
        }
    }

    if all_words
        .first()
        .is_some_and(|word| IT_OR_THEM_WORD_PATTERN.matches_word(word))
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        if all_words.len() == 1 {
            return ReferenceTagStageResult {
                source_linked_exile_reference,
                early_return: true,
            };
        }
        all_words.remove(0);
    }

    if REVEALED_CARD_PREFIX_PATTERN.matches_words(all_words) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.drain(..2);
    }

    let has_share_card_type = SHARES_CARD_TYPE_WITH_TAGGED_PATTERN.matches_words(all_words);
    let has_share_color = SHARES_COLOR_WITH_TAGGED_PATTERN.matches_words(all_words);
    let has_same_mana_value = SAME_MANA_VALUE_AS_PATTERN.matches_words(all_words);
    let has_equal_or_lesser_mana_value =
        EQUAL_OR_LESSER_MANA_VALUE_PATTERN.matches_words(all_words);
    let has_lte_mana_value_than_that_spell =
        LTE_MANA_VALUE_THAN_THAT_SPELL_PATTERN.matches_words(all_words);
    let has_lte_mana_value_as_tagged =
        LTE_MANA_VALUE_AS_TAGGED_PATTERN.matches_words(all_words) || has_equal_or_lesser_mana_value;
    let has_lt_mana_value_as_tagged =
        LESSER_MANA_VALUE_PATTERN.matches_words(all_words) && !has_equal_or_lesser_mana_value;
    let references_sacrifice_cost_object =
        SACRIFICE_COST_OBJECT_REFERENCE_PATTERN.matches_words(all_words);
    let references_it_for_mana_value = IT_OR_ITS_REFERENCE_PATTERN.matches_words(all_words)
        || TAGGED_OBJECT_REFERENCE_FOR_MANA_VALUE_PATTERN.matches_words(all_words);
    let has_same_name_as_tagged_object =
        SAME_NAME_AS_TAGGED_OBJECT_PATTERN.matches_words(all_words);

    if has_share_card_type {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesCardType,
        });
    }
    if has_share_color {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesColorWithTagged,
        });
    }
    if has_same_mana_value && references_sacrifice_cost_object {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("sacrifice_cost_0"),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    } else if has_same_mana_value && references_it_for_mana_value {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    }
    if has_lte_mana_value_as_tagged
        && (references_it_for_mana_value || has_equal_or_lesser_mana_value)
    {
        let tag = if has_lte_mana_value_than_that_spell {
            TagKey::from("triggering")
        } else {
            IT_TAG.into()
        };
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag,
            relation: TaggedOpbjectRelation::ManaValueLteTagged,
        });
    }
    if has_lt_mana_value_as_tagged {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::ManaValueLtTagged,
        });
    }
    if has_same_name_as_tagged_object {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
    }

    if CONVOKED_THIS_SPELL_TAG_PATTERN.matches_words(all_words) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("convoked_this_spell"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if CREWED_IT_THIS_TURN_TAG_PATTERN.matches_words(all_words) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("crewed_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if SADDLED_IT_THIS_TURN_TAG_PATTERN.matches_words(all_words) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("saddled_it_this_turn"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if AMASSED_ARMY_TAG_PATTERN.matches_words(all_words) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if THIS_WAY_TAG_PATTERN.matches_words(all_words) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    ReferenceTagStageResult {
        source_linked_exile_reference,
        early_return: false,
    }
}
