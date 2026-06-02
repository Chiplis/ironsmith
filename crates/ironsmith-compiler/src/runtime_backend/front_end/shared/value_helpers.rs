#![allow(dead_code)]

use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::{Value, ValueComparisonOperator};
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::{ObjectFilter, Zone};
use ironsmith_core::ValueSurfaceHint;
use ironsmith_core::{EffectMetric, EffectMetricSource};

use super::effect_sentences::trim_edge_punctuation;
use super::grammar::primitives::TokenWordView;
pub(crate) use super::grammar::values::{
    parse_number_from_lexed, parse_value_comparison_tokens, parse_value_from_lexed,
};
use super::lexer::{OwnedLexToken, TokenKind, contains_token_word, trim_lexed_commas};
use super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::util::{
    is_article, non_article_word_refs, parse_counter_type_word, parse_number,
    parse_number_word_i32, parse_value, parse_value_expr_words, token_index_for_word_index,
    trim_commas, trim_edge_punctuation_tokens,
};

type ValueHelperCompatWords<'a> = TokenWordView<'a>;

const THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "way"]);
const PRIOR_EFFECT_OBJECT_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_words
        & [&[
            "chosen",
            "destroyed",
            "discarded",
            "exiled",
            "milled",
            "revealed",
            "sacrificed",
            "searched",
        ]]
);
const CHOSEN_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["chosen"]);
const SOURCE_LINKED_EXILED_CARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["the", "exiled", "card"],
            &["the", "exiled", "cards"],
            &["exiled", "card"],
            &["exiled", "cards"],
        ]
);
const CREATURES_DIED_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creature", "that", "died", "this", "turn"],
            &["creatures", "that", "died", "this", "turn"],
        ]
);
const THAT_PLAYER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that", "player"], &["that", "players"]]);
const ITERATED_PLAYER_WORD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["they", "their", "theyve", "each"]]);
const COMMAND_ZONE_CAST_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "the", "command", "zone"]);
const EQUAL_TO_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equal", "to"]);
const EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["equal", "to"]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const ONE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["one"]);
const NUMBER_OF_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["number", "of"]);
const BASIC_LAND_TYPES_AMONG_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["basic", "land", "type", "among"],
            &["basic", "land", "types", "among"],
        ]
);
const COLORS_AMONG_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["color", "among"], &["colors", "among"]]);
const EQUAL_TO_OPPONENTS_YOU_HAVE_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "equal",
                "to",
                "the",
                "number",
                "of",
                "opponents",
                "you",
                "have"
            ],
            &["equal", "to", "number", "of", "opponents", "you", "have"],
        ]
);
const SOURCE_COUNTER_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["this"],
            &["this", "artifact"],
            &["this", "creature"],
            &["this", "enchantment"],
            &["this", "equipment"],
            &["this", "land"],
            &["this", "permanent"],
            &["this", "source"],
        ]
);
const TAGGED_COUNTER_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
            &["those"],
            &["those", "creatures"],
            &["those", "permanents"],
        ]
);
const COMMANDER_YOU_OWN_BATTLEFIELD_OR_COMMAND_ZONE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "commander",
            "you",
            "own",
            "on",
            "battlefield",
            "or",
            "in",
            "command",
            "zone"
        ]
);
const COMMANDER_ITERATED_PLAYER_OWNS_BATTLEFIELD_OR_COMMAND_ZONE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "commander",
                "they",
                "own",
                "on",
                "battlefield",
                "or",
                "in",
                "command",
                "zone"
            ],
            &[
                "commander",
                "that",
                "player",
                "owns",
                "on",
                "battlefield",
                "or",
                "in",
                "command",
                "zone",
            ],
        ]
);
const OR_POWER_TOUGHNESS_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["or", "power"], &["or", "toughness"]]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const PLUS_MINUS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["plus"], &["minus"]]);
const MINUS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["minus"]);
const GREATEST_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["greatest"]);
const MANA_VALUE_KIND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["mana_value"]);
const POWER_TOUGHNESS_AXIS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["power"], &["toughness"]]);
const YOU_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["you"], &["your"], &["youve"]]);
const OPPONENT_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["opponent"], &["opponents"]]);
const AND_OR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"], &["and/or"]]);
const COMPARISON_OR_TAIL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["less"], &["fewer"], &["greater"], &["more"]]);
const LESS_OR_FEWER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["less"], &["fewer"]]);

fn value_helper_find_any_phrase_start(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    phrases.iter().find_map(|phrase| {
        words
            .windows(phrase.len())
            .position(|window| window == *phrase)
    })
}

fn word_refs_reference_prior_effect_objects(words: &[&str]) -> bool {
    THIS_WAY_PATTERN
        .find_exact_window_range(words, 2, 2)
        .is_some()
        || PRIOR_EFFECT_OBJECT_MARKER_PATTERN.matches_words(words)
}

fn effect_metric_source_for_prior_effect_words(words: &[&str]) -> EffectMetricSource {
    if CHOSEN_MARKER_PATTERN.matches_words(words) {
        EffectMetricSource::ChosenObjects
    } else {
        EffectMetricSource::AffectedObjects
    }
}

fn aggregate_effect_metric(aggregate: &str, value_kind: &str) -> Option<EffectMetric> {
    match (aggregate, value_kind) {
        ("total", "power") => Some(EffectMetric::TotalPower),
        ("total", "toughness") => Some(EffectMetric::TotalToughness),
        ("total", "mana_value") => Some(EffectMetric::TotalManaValue),
        ("greatest", "power") => Some(EffectMetric::GreatestPower),
        ("greatest", "toughness") => Some(EffectMetric::GreatestToughness),
        ("greatest", "mana_value") => Some(EffectMetric::GreatestManaValue),
        _ => None,
    }
}

fn pending_aggregate_metric_value(
    aggregate: &str,
    value_kind: &str,
    object_words: &[&str],
) -> Option<Value> {
    if !word_refs_reference_prior_effect_objects(object_words) {
        return None;
    }
    Some(Value::PendingEffectMetric {
        source: effect_metric_source_for_prior_effect_words(object_words),
        metric: aggregate_effect_metric(aggregate, value_kind)?,
    })
}

fn pending_count_metric_value(object_words: &[&str]) -> Option<Value> {
    if !word_refs_reference_prior_effect_objects(object_words) {
        return None;
    }
    Some(Value::PendingEffectMetric {
        source: effect_metric_source_for_prior_effect_words(object_words),
        metric: EffectMetric::Count,
    })
}

fn source_linked_exiled_mana_value(object_words: &[&str]) -> Option<Value> {
    if SOURCE_LINKED_EXILED_CARD_PATTERN.matches_words(object_words) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        ))));
    }
    None
}

fn parse_spells_cast_this_turn_matching_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = ValueHelperCompatWords::new(tokens);
    let filter_words = word_view.to_word_refs();
    if !word_view.contains_any_word(&["spell", "spells"])
        || !word_view.contains_any_word(&["cast", "casts"])
        || !word_view.contains_word("this")
        || !word_view.contains_word("turn")
    {
        return None;
    }

    let suffix_patterns: &[(&[&str], PlayerFilter)] = &[
        (
            &["theyve", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["they", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["that", "player", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (&["youve", "cast", "this", "turn"], PlayerFilter::You),
        (&["you", "cast", "this", "turn"], PlayerFilter::You),
        (
            &["an", "opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponents", "have", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (&["cast", "this", "turn"], PlayerFilter::Any),
    ];

    for (suffix, player) in suffix_patterns {
        if !filter_words.ends_with(suffix) {
            continue;
        }
        let filter_word_len = filter_words.len().saturating_sub(suffix.len());
        let filter_token_end =
            token_index_for_word_index(tokens, filter_word_len).unwrap_or(tokens.len());
        let filter_tokens = trim_commas(&tokens[..filter_token_end]);
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        let exclude_source = contains_token_word(&filter_tokens, "other");
        return Some(Value::SpellsCastThisTurnMatching {
            player: player.clone(),
            filter,
            exclude_source,
        });
    }

    None
}

fn parse_creatures_died_this_turn_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = ValueHelperCompatWords::new(tokens);
    if CREATURES_DIED_THIS_TURN_PATTERN.matches_words(&word_view.to_word_refs()) {
        Some(Value::CreaturesDiedThisTurn)
    } else {
        None
    }
}

fn parse_cards_discarded_this_turn_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = ValueHelperCompatWords::new(tokens);
    if !words.contains_word("cards")
        || !words.contains_word("discarded")
        || !words.contains_word("this")
        || !words.contains_word("turn")
    {
        return None;
    }

    if words
        .to_word_refs()
        .iter()
        .any(|word| YOU_REFERENCE_WORD_PATTERN.matches_word(word))
    {
        return Some(Value::CardsDiscardedThisTurn(PlayerFilter::You));
    }
    if words
        .to_word_refs()
        .iter()
        .any(|word| OPPONENT_REFERENCE_WORD_PATTERN.matches_word(word))
    {
        return Some(Value::CardsDiscardedThisTurn(PlayerFilter::Opponent));
    }
    let word_refs = words.to_word_refs();
    if THAT_PLAYER_PATTERN
        .find_exact_window_range(&word_refs, 2, 2)
        .is_some()
        || ITERATED_PLAYER_WORD_MARKER_PATTERN.matches_words(&word_refs)
    {
        return Some(Value::CardsDiscardedThisTurn(PlayerFilter::IteratedPlayer));
    }

    Some(Value::CardsDiscardedThisTurn(PlayerFilter::Any))
}

pub(crate) fn parse_commander_cast_count_player(tokens: &[OwnedLexToken]) -> Option<PlayerFilter> {
    let word_view = ValueHelperCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    if !word_view.contains_word("cast")
        || !word_view.contains_any_word(&["commander", "commanders"])
        || COMMAND_ZONE_CAST_PATTERN
            .find_exact_window_range(&words, 4, 4)
            .is_none()
        || !word_view.contains_word("game")
    {
        return None;
    }

    if words
        .iter()
        .any(|word| YOU_REFERENCE_WORD_PATTERN.matches_word(word))
    {
        return Some(PlayerFilter::You);
    }
    if words
        .iter()
        .any(|word| OPPONENT_REFERENCE_WORD_PATTERN.matches_word(word))
    {
        return Some(PlayerFilter::Opponent);
    }
    if ITERATED_PLAYER_WORD_MARKER_PATTERN.matches_words(&words)
        || THAT_PLAYER_PATTERN
            .find_exact_window_range(&words, 2, 2)
            .is_some()
    {
        return Some(PlayerFilter::IteratedPlayer);
    }

    Some(PlayerFilter::Any)
}

pub(crate) fn parse_equal_to_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = ValueHelperCompatWords::new(tokens);
    let words_all = word_view.to_word_refs();
    let equal_idx = EQUAL_TO_PATTERN.find_exact_window_range(&words_all, 2, 2)?;
    let mut number_word_idx = equal_idx + 2;
    if THE_WORD_PATTERN.matches_word_at(&words_all, number_word_idx) {
        number_word_idx += 1;
    }
    if !words_all
        .get(number_word_idx..number_word_idx + 2)
        .is_some_and(|words| NUMBER_OF_PATTERN.matches_words(words))
    {
        return None;
    }

    let value_range = word_view.token_range_for_word_range(number_word_idx, word_view.len())?;
    let value_tokens = trim_edge_punctuation(&tokens[value_range]);
    if let Some((value, used)) = parse_value(&value_tokens)
        && ValueHelperCompatWords::new(&value_tokens[used..]).is_empty()
    {
        return Some(value);
    }

    let filter_start_word_idx = number_word_idx + 2;
    let filter_range =
        word_view.token_range_for_word_range(filter_start_word_idx, word_view.len())?;
    let filter_tokens = trim_edge_punctuation(&tokens[filter_range]);
    let filter_word_view = ValueHelperCompatWords::new(&filter_tokens);
    let filter_words = filter_word_view.to_word_refs();
    if let Some(value) = parse_creatures_died_this_turn_count_value(&filter_tokens) {
        return Some(value);
    }
    if let Some(value) = parse_cards_discarded_this_turn_count_value(&filter_tokens) {
        return Some(value);
    }
    if filter_word_view.contains_word("cards")
        && filter_word_view.contains_word("in")
        && filter_word_view.contains_any_word(&["hand", "hands"])
    {
        if filter_word_view.contains_word("your") {
            return Some(Value::CardsInHand(PlayerFilter::You));
        }
        if filter_word_view.contains_word("their")
            || value_helper_find_any_phrase_start(
                &filter_words,
                &[
                    &["that", "player"],
                    &["that", "players"],
                    &["the", "chosen"],
                ],
            )
            .is_some()
        {
            return Some(Value::CardsInHand(PlayerFilter::IteratedPlayer));
        }
        if filter_word_view.contains_any_word(&["opponent", "opponents"]) {
            return Some(Value::CardsInHand(PlayerFilter::Opponent));
        }
    }
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        return Some(value);
    }
    if BASIC_LAND_TYPES_AMONG_PATTERN.matches_words(&filter_words) {
        let scope_range = filter_word_view.token_range_for_word_range(4, filter_word_view.len())?;
        let scope_tokens = trim_edge_punctuation(&filter_tokens[scope_range]);
        let filter = parse_object_filter(&scope_tokens, false).ok()?;
        return Some(Value::BasicLandTypesAmong(filter));
    }
    if COLORS_AMONG_PATTERN.matches_words(&filter_words) {
        let mut scope_start_word_idx = 2usize;
        if THE_WORD_PATTERN.matches_word_at(&filter_words, scope_start_word_idx) {
            scope_start_word_idx += 1;
        }
        let scope_range = filter_word_view
            .token_range_for_word_range(scope_start_word_idx, filter_word_view.len())?;
        let scope_tokens = trim_edge_punctuation(&filter_tokens[scope_range]);
        let filter = parse_object_filter(&scope_tokens, false).ok()?;
        return Some(Value::ColorsAmong(filter));
    }
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let word_view = ValueHelperCompatWords::new(tokens);
    let clause_words = word_view.to_word_refs();
    if !EQUAL_TO_PATTERN.matches_words(&clause_words) {
        return None;
    }

    let mut number_word_idx = 2usize;
    if THE_WORD_PATTERN.matches_word_at(&clause_words, number_word_idx) {
        number_word_idx += 1;
    }
    if !clause_words
        .get(number_word_idx..number_word_idx + 2)
        .is_some_and(|words| NUMBER_OF_PATTERN.matches_words(words))
    {
        return None;
    }

    let filter_start_word_idx = number_word_idx + 2;
    let operator_word_idx =
        word_view.find_any_word_from(&["plus", "minus"], filter_start_word_idx + 1)?;
    let operator = clause_words[operator_word_idx];

    let filter_range =
        word_view.token_range_for_word_range(filter_start_word_idx, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_range]);
    let base_value = if let Some(value) = parse_creatures_died_this_turn_count_value(&filter_tokens)
    {
        value
    } else if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        value
    } else {
        Value::Count(parse_object_filter(&filter_tokens, false).ok()?)
    };

    let offset_range =
        word_view.token_range_for_word_range(operator_word_idx + 1, word_view.len())?;
    let offset_tokens = trim_commas(&tokens[offset_range]);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    if !ValueHelperCompatWords::new(&offset_tokens[used..]).is_empty() {
        return None;
    }

    let signed_offset = if MINUS_WORD_PATTERN.matches_word(operator) {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(Value::Add(
        Box::new(base_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

pub(crate) fn parse_equal_to_number_of_opponents_you_have_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = ValueHelperCompatWords::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if EQUAL_TO_OPPONENTS_YOU_HAVE_PATTERN.matches_words(&clause_refs) {
        return Some(Value::CountPlayers(PlayerFilter::Opponent));
    }
    None
}

pub(crate) fn parse_equal_to_number_of_counters_on_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = ValueHelperCompatWords::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if !EQUAL_TO_PATTERN.matches_words(&clause_refs) {
        return None;
    }

    let mut idx = 2usize;
    if clause_words.at_is(idx, "the") {
        idx += 1;
    }
    if !clause_words.starts_with_at(idx, &["number", "of"]) {
        return None;
    }
    idx += 2;

    if clause_words
        .get(idx)
        .is_some_and(|word| is_article(word) || ONE_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }

    let mut counter_type = None;
    if let Some(word) = clause_words.get(idx)
        && let Some(parsed) = parse_counter_type_word(word)
    {
        counter_type = Some(parsed);
        idx += 1;
    }

    if !clause_words.at_is_any(idx, &["counter", "counters"]) {
        return None;
    }
    idx += 1;

    if !clause_words.at_is(idx, "on") {
        return None;
    }
    idx += 1;

    let reference = &clause_refs[idx..];
    if reference.is_empty() {
        return None;
    }

    if SOURCE_COUNTER_REFERENCE_PATTERN.matches_words(reference) {
        return Some(match counter_type {
            Some(counter_type) => Value::CountersOnSource(counter_type),
            None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
        });
    }

    if TAGGED_COUNTER_REFERENCE_PATTERN.matches_words(reference) {
        return Some(Value::CountersOn(
            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
            counter_type,
        ));
    }

    None
}

pub(crate) fn parse_equal_to_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = ValueHelperCompatWords::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    let equal_idx = EQUAL_TO_PATTERN.find_exact_window_range(&clause_refs, 2, 2)?;

    let mut idx = equal_idx + 2;
    if clause_words.at_is(idx, "the") {
        idx += 1;
    }

    let aggregate = match clause_words.get(idx) {
        Some("total") => "total",
        Some("greatest") => "greatest",
        _ => return None,
    };
    idx += 1;

    let value_kind = if clause_words.at_is(idx, "power") {
        idx += 1;
        "power"
    } else if clause_words.at_is(idx, "toughness") {
        idx += 1;
        "toughness"
    } else if clause_words.starts_with_at(idx, &["mana", "value"]) {
        idx += 2;
        "mana_value"
    } else {
        return None;
    };

    if !clause_words.at_is_any(idx, &["of", "among"]) {
        return None;
    }
    idx += 1;

    if GREATEST_WORD_PATTERN.matches_word(aggregate)
        && MANA_VALUE_KIND_WORD_PATTERN.matches_word(value_kind)
    {
        if let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx) {
            return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
        }
    }

    let filter_range = clause_words.token_range_for_word_range(idx, clause_words.len())?;
    let filter_tokens = &tokens[filter_range];
    let object_words = &clause_refs[idx..];
    if MANA_VALUE_KIND_WORD_PATTERN.matches_word(value_kind)
        && let Some(value) = source_linked_exiled_mana_value(object_words)
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    if let Some(value) = pending_aggregate_metric_value(aggregate, value_kind, object_words) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    let mut filter = parse_object_filter(filter_tokens, false).ok()?;
    if object_words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"))
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
    {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }

    match (aggregate, value_kind) {
        ("total", "power") => Some(Value::TotalPower(filter)),
        ("total", "toughness") => Some(Value::TotalToughness(filter)),
        ("total", "mana_value") => Some(Value::TotalManaValue(filter)),
        ("greatest", "power") => Some(Value::GreatestPower(filter)),
        ("greatest", "toughness") => Some(Value::GreatestToughness(filter)),
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_greatest_commander_mana_value(
    tokens: &[OwnedLexToken],
    commander_start_word_idx: usize,
) -> Option<Value> {
    let words = ValueHelperCompatWords::new(tokens);
    let commander_range =
        words.token_range_for_word_range(commander_start_word_idx, words.len())?;
    let commander_words = crate::runtime_backend::token_word_refs(&tokens[commander_range]);
    let normalized = non_article_word_refs(&commander_words);
    let owner = commander_owner_from_battlefield_or_command_zone_words(&normalized)?;

    let mut battlefield_commander = ObjectFilter::default();
    battlefield_commander.zone = Some(Zone::Battlefield);
    battlefield_commander.is_commander = true;
    battlefield_commander.owner = Some(owner);

    let mut command_zone_commander = battlefield_commander.clone();
    command_zone_commander.zone = Some(Zone::Command);

    let mut combined = ObjectFilter::default();
    combined.any_of = vec![battlefield_commander, command_zone_commander];

    Some(Value::GreatestManaValue(combined))
}

fn commander_owner_from_battlefield_or_command_zone_words(words: &[&str]) -> Option<PlayerFilter> {
    if COMMANDER_YOU_OWN_BATTLEFIELD_OR_COMMAND_ZONE_PATTERN.matches_words(words) {
        return Some(PlayerFilter::You);
    }
    if COMMANDER_ITERATED_PLAYER_OWNS_BATTLEFIELD_OR_COMMAND_ZONE_PATTERN.matches_words(words) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    None
}

pub(crate) fn parse_equal_to_number_of_filter_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words_all = ValueHelperCompatWords::new(tokens);
    let words_refs = words_all.to_word_refs();
    let equal_idx = EQUAL_TO_PATTERN.find_exact_window_range(&words_refs, 2, 2)?;
    let mut number_word_idx = equal_idx + 2;
    if words_all.at_is(number_word_idx, "the") {
        number_word_idx += 1;
    }
    if !words_all.starts_with_at(number_word_idx, &["number", "of"]) {
        return None;
    }

    let value_range = words_all.token_range_for_word_range(number_word_idx, words_all.len())?;
    let value_tokens = trim_edge_punctuation_tokens(&tokens[value_range]);
    if let Some((value, used)) = parse_value_from_lexed(value_tokens) {
        if ValueHelperCompatWords::new(&value_tokens[used..]).is_empty() {
            return Some(value);
        }
    }

    let filter_start_word_idx = number_word_idx + 2;
    let filter_range =
        words_all.token_range_for_word_range(filter_start_word_idx, words_all.len())?;
    let filter_tokens = trim_edge_punctuation_tokens(&tokens[filter_range]);
    let filter_words = ValueHelperCompatWords::new(filter_tokens).to_word_refs();
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value_lexed(filter_tokens) {
        return Some(value);
    }
    if let Some(value) = parse_cards_discarded_this_turn_count_value(filter_tokens) {
        return Some(value);
    }
    if BASIC_LAND_TYPES_AMONG_PATTERN.matches_words(&filter_words) {
        let filter_word_view = ValueHelperCompatWords::new(filter_tokens);
        let scope_range = filter_word_view.token_range_for_word_range(4, filter_word_view.len())?;
        let scope_tokens = trim_edge_punctuation_tokens(&filter_tokens[scope_range]);
        let filter = parse_object_filter_lexed(scope_tokens, false).ok()?;
        return Some(Value::BasicLandTypesAmong(filter));
    }
    if COLORS_AMONG_PATTERN.matches_words(&filter_words) {
        let mut scope_start_word_idx = 2usize;
        if THE_WORD_PATTERN.matches_word_at(&filter_words, scope_start_word_idx) {
            scope_start_word_idx += 1;
        }
        let filter_word_view = ValueHelperCompatWords::new(filter_tokens);
        let scope_range = filter_word_view
            .token_range_for_word_range(scope_start_word_idx, filter_word_view.len())?;
        let scope_tokens = trim_edge_punctuation_tokens(&filter_tokens[scope_range]);
        let filter = parse_object_filter_lexed(scope_tokens, false).ok()?;
        return Some(Value::ColorsAmong(filter));
    }
    let mut filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    if filter_words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"))
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
    {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }
    Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::EqualTo))
}

pub(crate) fn parse_equal_to_number_of_filter_plus_or_minus_fixed_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = ValueHelperCompatWords::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if !EQUAL_TO_PREFIX_PATTERN.matches_words(&clause_refs) {
        return None;
    }

    let mut number_word_idx = 2usize;
    if clause_words.at_is(number_word_idx, "the") {
        number_word_idx += 1;
    }
    if !clause_refs
        .get(number_word_idx..number_word_idx + 2)
        .is_some_and(|words| NUMBER_OF_PATTERN.matches_words(words))
    {
        return None;
    }

    let filter_start_word_idx = number_word_idx + 2;
    let operator_word_idx =
        clause_words.find_any_word_from(&["plus", "minus"], filter_start_word_idx + 1)?;
    let operator = clause_words.get(operator_word_idx)?;

    let filter_range =
        clause_words.token_range_for_word_range(filter_start_word_idx, operator_word_idx)?;
    let filter_tokens = trim_lexed_commas(&tokens[filter_range]);
    let base_value = if let Some(value) =
        parse_spells_cast_this_turn_matching_count_value_lexed(filter_tokens)
    {
        value
    } else {
        Value::Count(parse_object_filter_lexed(filter_tokens, false).ok()?)
    };

    let offset_range =
        clause_words.token_range_for_word_range(operator_word_idx + 1, clause_words.len())?;
    let offset_tokens = trim_lexed_commas(&tokens[offset_range]);
    let (offset_value, used) = parse_number_from_lexed(offset_tokens)?;
    if !ValueHelperCompatWords::new(&offset_tokens[used..]).is_empty() {
        return None;
    }

    let signed_offset = if MINUS_WORD_PATTERN.matches_word(operator) {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(Value::Add(
        Box::new(base_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

pub(crate) fn parse_equal_to_number_of_opponents_you_have_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = ValueHelperCompatWords::new(tokens);
    if EQUAL_TO_OPPONENTS_YOU_HAVE_PATTERN.matches_words(&clause_words.to_word_refs()) {
        return Some(
            Value::CountPlayers(PlayerFilter::Opponent)
                .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }
    None
}

pub(crate) fn parse_equal_to_number_of_counters_on_reference_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = ValueHelperCompatWords::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if !EQUAL_TO_PREFIX_PATTERN.matches_words(&clause_refs) {
        return None;
    }

    let mut idx = 2usize;
    if clause_words.at_is(idx, "the") {
        idx += 1;
    }
    if !NUMBER_OF_PATTERN.matches_words(&clause_refs[idx..]) {
        return None;
    }
    idx += 2;

    if clause_words
        .get(idx)
        .is_some_and(|word| is_article(word) || ONE_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }

    let mut counter_type = None;
    if let Some(word) = clause_words.get(idx)
        && let Some(parsed) = parse_counter_type_word(word)
    {
        counter_type = Some(parsed);
        idx += 1;
    }

    if !clause_words.at_is_any(idx, &["counter", "counters"]) {
        return None;
    }
    idx += 1;

    if !clause_words.at_is(idx, "on") {
        return None;
    }
    idx += 1;

    let reference = &clause_words.to_word_refs()[idx..];
    if reference.is_empty() {
        return None;
    }

    if SOURCE_COUNTER_REFERENCE_PATTERN.matches_words(reference) {
        return Some(
            match counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            }
            .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }

    if TAGGED_COUNTER_REFERENCE_PATTERN.matches_words(reference) {
        return Some(
            Value::CountersOn(
                Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                counter_type,
            )
            .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }

    None
}

pub(crate) fn parse_equal_to_aggregate_filter_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = ValueHelperCompatWords::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    let equal_idx = EQUAL_TO_PATTERN.find_exact_window_range(&clause_refs, 2, 2)?;

    let mut idx = equal_idx + 2;
    if clause_words.at_is(idx, "the") {
        idx += 1;
    }

    let aggregate = match clause_words.get(idx) {
        Some("total") => "total",
        Some("greatest") => "greatest",
        _ => return None,
    };
    idx += 1;

    let value_kind = if clause_words.at_is(idx, "power") {
        idx += 1;
        "power"
    } else if clause_words.at_is(idx, "toughness") {
        idx += 1;
        "toughness"
    } else if clause_words.starts_with_at(idx, &["mana", "value"]) {
        idx += 2;
        "mana_value"
    } else {
        return None;
    };

    if !clause_words.at_is_any(idx, &["of", "among"]) {
        return None;
    }
    idx += 1;

    if GREATEST_WORD_PATTERN.matches_word(aggregate)
        && MANA_VALUE_KIND_WORD_PATTERN.matches_word(value_kind)
    {
        if let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx) {
            return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
        }
    }

    let filter_range = clause_words.token_range_for_word_range(idx, clause_words.len())?;
    let filter_tokens = &tokens[filter_range];
    let object_words = &clause_refs[idx..];
    if MANA_VALUE_KIND_WORD_PATTERN.matches_word(value_kind)
        && let Some(value) = source_linked_exiled_mana_value(object_words)
    {
        return Some(value);
    }
    if let Some(value) = pending_aggregate_metric_value(aggregate, value_kind, object_words) {
        return Some(value);
    }
    let mut filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    if object_words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"))
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
    {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }

    match (aggregate, value_kind) {
        ("total", "power") => Some(Value::TotalPower(filter)),
        ("total", "toughness") => Some(Value::TotalToughness(filter)),
        ("total", "mana_value") => Some(Value::TotalManaValue(filter)),
        ("greatest", "power") => Some(Value::GreatestPower(filter)),
        ("greatest", "toughness") => Some(Value::GreatestToughness(filter)),
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_spells_cast_this_turn_matching_count_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_words = ValueHelperCompatWords::new(tokens);
    if !filter_words.contains_any_word(&["spell", "spells"])
        || !filter_words.contains_any_word(&["cast", "casts"])
        || !filter_words.contains_word("this")
        || !filter_words.contains_word("turn")
    {
        return None;
    }

    let suffix_patterns: &[(&[&str], PlayerFilter)] = &[
        (
            &["theyve", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["they", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (
            &["that", "player", "cast", "this", "turn"],
            PlayerFilter::IteratedPlayer,
        ),
        (&["youve", "cast", "this", "turn"], PlayerFilter::You),
        (&["you", "cast", "this", "turn"], PlayerFilter::You),
        (
            &["an", "opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponent", "has", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (
            &["opponents", "have", "cast", "this", "turn"],
            PlayerFilter::Opponent,
        ),
        (&["cast", "this", "turn"], PlayerFilter::Any),
    ];

    for (suffix, player) in suffix_patterns {
        if !filter_words.ends_with(suffix) {
            continue;
        }
        let filter_word_len = filter_words.len().saturating_sub(suffix.len());
        let filter_token_end = filter_words
            .token_index_for_word_index(filter_word_len)
            .unwrap_or(tokens.len());
        let filter_tokens = trim_lexed_commas(&tokens[..filter_token_end]);
        let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
        let exclude_source = contains_token_word(filter_tokens, "other");
        return Some(Value::SpellsCastThisTurnMatching {
            player: player.clone(),
            filter,
            exclude_source,
        });
    }

    None
}

pub(crate) fn parse_filter_comparison_tokens(
    axis: &str,
    tokens: &[&str],
    clause_words: &[&str],
) -> Result<Option<(crate::filter::Comparison, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    if POWER_TOUGHNESS_AXIS_WORD_PATTERN.matches_word(axis)
        && OR_POWER_TOUGHNESS_PATTERN.matches_words(tokens)
    {
        return Ok(None);
    }

    let to_comparison = |kind: &str, operand: Value| -> crate::filter::Comparison {
        use crate::filter::Comparison;

        match (kind, operand) {
            ("eq", Value::Fixed(value)) => Comparison::Equal(value),
            ("neq", Value::Fixed(value)) => Comparison::NotEqual(value),
            ("lt", Value::Fixed(value)) => Comparison::LessThan(value),
            ("lte", Value::Fixed(value)) => Comparison::LessThanOrEqual(value),
            ("gt", Value::Fixed(value)) => Comparison::GreaterThan(value),
            ("gte", Value::Fixed(value)) => Comparison::GreaterThanOrEqual(value),
            ("eq", operand) => Comparison::EqualExpr(Box::new(operand)),
            ("neq", operand) => Comparison::NotEqualExpr(Box::new(operand)),
            ("lt", operand) => Comparison::LessThanExpr(Box::new(operand)),
            ("lte", operand) => Comparison::LessThanOrEqualExpr(Box::new(operand)),
            ("gt", operand) => Comparison::GreaterThanExpr(Box::new(operand)),
            ("gte", operand) => Comparison::GreaterThanOrEqualExpr(Box::new(operand)),
            _ => unreachable!("unsupported comparison kind"),
        }
    };

    let parse_operand = |operand_tokens: &[&str],
                         comparison_kind: &str|
     -> Result<(crate::filter::Comparison, usize), CardTextError> {
        let Some((operand, used)) = parse_value_expr_words(operand_tokens) else {
            let quoted = operand_tokens
                .first()
                .copied()
                .unwrap_or_default()
                .to_string();
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        Ok((to_comparison(comparison_kind, operand), used))
    };

    let parse_numeric_token = |word: &str| -> Option<i32> {
        if let Ok(value) = word.parse::<i32>() {
            return Some(value);
        }
        parse_number_word_i32(word)
    };

    let map_operator =
        |operator: ValueComparisonOperator, operand: Value| -> crate::filter::Comparison {
            match operator {
                ValueComparisonOperator::Equal => to_comparison("eq", operand),
                ValueComparisonOperator::NotEqual => to_comparison("neq", operand),
                ValueComparisonOperator::LessThan => to_comparison("lt", operand),
                ValueComparisonOperator::LessThanOrEqual => to_comparison("lte", operand),
                ValueComparisonOperator::GreaterThan => to_comparison("gt", operand),
                ValueComparisonOperator::GreaterThanOrEqual => to_comparison("gte", operand),
            }
        };

    let first = tokens[0];
    if let Some(value) = parse_numeric_token(first) {
        if tokens
            .get(1)
            .is_some_and(|word| PLUS_MINUS_WORD_PATTERN.matches_word(word))
        {
            let (cmp, used) = parse_operand(tokens, "eq")?;
            return Ok(Some((cmp, used)));
        }
        let mut values = vec![value];
        let mut consumed = 1usize;
        while consumed < tokens.len() {
            let token = tokens[consumed];
            if AND_OR_WORD_PATTERN.matches_word(token) {
                consumed += 1;
                continue;
            }
            if let Some(next_value) = parse_numeric_token(token) {
                values.push(next_value);
                consumed += 1;
                continue;
            }
            break;
        }
        if values.len() > 1 {
            return Ok(Some((crate::filter::Comparison::OneOf(values), consumed)));
        }
        if tokens.len() == 1 {
            return Ok(Some((crate::filter::Comparison::Equal(value), 1)));
        }
    }

    let synthetic_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(tokens);
    if let Some((operator, operand_tokens)) = parse_value_comparison_tokens(&synthetic_tokens) {
        let operand_len = operand_tokens.len();
        let operand_start = if operand_len == 0
            || std::ptr::eq(operand_tokens.as_ptr(), synthetic_tokens.as_ptr())
        {
            0
        } else {
            synthetic_tokens.len().saturating_sub(operand_len)
        };
        let operand_words = operand_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>();
        if operand_words.is_empty() {
            let consumed_phrase = if operand_start == 0 {
                synthetic_tokens.len()
            } else {
                operand_start
            };
            let phrase = tokens[..consumed_phrase].join(" ");
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after '{phrase}' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (operand, used) = parse_value_expr_words(&operand_words).ok_or_else(|| {
            let quoted = operand_words.first().copied().unwrap_or_default();
            CardTextError::ParseError(format!(
                "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let consumed = if operand_start == 0 {
            used + (synthetic_tokens.len().saturating_sub(operand_len))
        } else {
            operand_start + used
        };
        return Ok(Some((map_operator(operator, operand), consumed)));
    }

    if let Some((value, used)) = parse_value_expr_words(tokens) {
        if OR_WORD_PATTERN.matches_word_at(tokens, used)
            && let Some(next) = tokens.get(used + 1)
            && COMPARISON_OR_TAIL_WORD_PATTERN.matches_word(next)
        {
            let kind = if LESS_OR_FEWER_WORD_PATTERN.matches_word(next) {
                "lte"
            } else {
                "gte"
            };
            return Ok(Some((to_comparison(kind, value), used + 2)));
        }
        if let Value::Fixed(fixed) = value
            && used == 1
        {
            return Ok(Some((crate::filter::Comparison::Equal(fixed), used)));
        }
        return Ok(Some((
            crate::filter::Comparison::EqualExpr(Box::new(value)),
            used,
        )));
    }

    Ok(None)
}
