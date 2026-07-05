#![allow(dead_code)]

use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::{Value, ValueComparisonOperator};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::{ObjectFilter, Zone};
use ironsmith_core::ValueSurfaceHint;
use ironsmith_core::{EffectMetric, EffectMetricSource};

use super::effect_sentences::trim_edge_punctuation;
use super::grammar::primitives::TokenWordView;
pub(crate) use super::grammar::values::{
    parse_number_from_lexed, parse_value_comparison_tokens, parse_value_comparison_words,
    parse_value_from_lexed,
};
use super::lex_patterns::{LexCaptureKind, LexPattern, LexPatternAtom};
use super::lexer::{LexedClause, OwnedLexToken, TokenKind, contains_token_word, trim_lexed_commas};
use super::object_filters::{
    parse_object_filter, parse_object_filter_lexed, parse_object_filter_words,
};
use super::util::{
    is_article, non_article_word_refs, parse_counter_type_word, parse_number,
    parse_number_word_i32, parse_value, parse_value_expr_words, source_reference_surface_for_words,
    this_source_surface_for_words, trim_commas, trim_edge_punctuation_tokens,
};

type ValueHelperCompatWords<'a> = TokenWordView<'a>;

const THIS_WAY_PHRASE: &[&str] = &["this", "way"];
const PRIOR_EFFECT_OBJECT_MARKER_WORDS: &[&str] = &[
    "chosen",
    "destroyed",
    "discarded",
    "exiled",
    "milled",
    "revealed",
    "sacrificed",
    "searched",
];
const SOURCE_LINKED_EXILED_CARD_PHRASES: &[&[&str]] = &[
    &["the", "exiled", "card"],
    &["the", "exiled", "cards"],
    &["exiled", "card"],
    &["exiled", "cards"],
];
const CREATURES_DIED_THIS_TURN_PHRASES: &[&[&str]] = &[
    &["creature", "that", "died", "this", "turn"],
    &["creatures", "that", "died", "this", "turn"],
];
const THAT_PLAYER_PHRASES: &[&[&str]] = &[&["that", "player"], &["that", "players"]];
const ITERATED_PLAYER_MARKER_WORDS: &[&str] = &["they", "their", "theyve", "each"];
const COMMAND_ZONE_CAST_PHRASE: &[&str] = &["from", "the", "command", "zone"];
const EQUAL_TO_PHRASE: &[&str] = &["equal", "to"];
const NUMBER_OF_PHRASE: &[&str] = &["number", "of"];
const AGGREGATE_SCOPE_METRIC_PHRASES: &[&[&str]] = &[
    &["basic", "land", "type", "among"],
    &["basic", "land", "types", "among"],
    &["creature", "type", "among"],
    &["creature", "types", "among"],
    &["color", "among"],
    &["colors", "among"],
    &["different", "powers", "among"],
    &["different", "power", "values", "among"],
    &["different", "power", "among"],
    &["counter", "among"],
    &["counters", "among"],
];
const AGGREGATE_SCOPE_OPTIONAL_THE_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
const AGGREGATE_SCOPE_VALUE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::amount(
        "metric",
        LexCaptureKind::OneOfPhrase(AGGREGATE_SCOPE_METRIC_PHRASES),
    ),
    LexPattern::optional(AGGREGATE_SCOPE_OPTIONAL_THE_ATOMS),
    LexPattern::object("scope", LexCaptureKind::OneOrMoreWords),
]);
const NUMBER_OF_PHRASES: &[&[&str]] = &[NUMBER_OF_PHRASE];
const EQUAL_TO_NUMBER_OF_OPTIONAL_THE_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::word("the")];
const EQUAL_TO_NUMBER_OF_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::optional(EQUAL_TO_NUMBER_OF_OPTIONAL_THE_ATOMS),
    LexPattern::amount("number_of", LexCaptureKind::OneOfPhrase(NUMBER_OF_PHRASES)),
]);
const EQUAL_TO_AGGREGATE_OPTIONAL_THE_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::word("the")];
const EQUAL_TO_AGGREGATE_VALUE_KIND_PHRASES: &[&[&str]] =
    &[&["power"], &["toughness"], &["mana", "value"]];
const EQUAL_TO_AGGREGATE_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::optional(EQUAL_TO_AGGREGATE_OPTIONAL_THE_ATOMS),
    LexPattern::amount("aggregate", LexCaptureKind::OneOf(&["total", "greatest"])),
    LexPattern::amount(
        "value_kind",
        LexCaptureKind::OneOfPhrase(EQUAL_TO_AGGREGATE_VALUE_KIND_PHRASES),
    ),
    LexPattern::modifier("connector", LexCaptureKind::OneOf(&["of", "among"])),
]);
const SPELL_CAST_THIS_TURN_SUFFIX_PHRASES: &[&[&str]] = &[
    &["theyve", "cast", "this", "turn"],
    &["they", "cast", "this", "turn"],
    &["that", "player", "cast", "this", "turn"],
    &["youve", "cast", "this", "turn"],
    &["you", "cast", "this", "turn"],
    &["an", "opponent", "has", "cast", "this", "turn"],
    &["opponent", "has", "cast", "this", "turn"],
    &["opponents", "have", "cast", "this", "turn"],
    &["cast", "this", "turn"],
];
const SPELL_CAST_THIS_TURN_VALUE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object(
        "filter",
        LexCaptureKind::UntilAnyPhrase(SPELL_CAST_THIS_TURN_SUFFIX_PHRASES),
    ),
    LexPattern::action(
        "cast_suffix",
        LexCaptureKind::OneOfPhrase(SPELL_CAST_THIS_TURN_SUFFIX_PHRASES),
    ),
]);
const EQUAL_TO_OPPONENTS_YOU_HAVE_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::any_phrase(&[
        &[
            "equal",
            "to",
            "the",
            "number",
            "of",
            "opponents",
            "you",
            "have",
        ],
        &["equal", "to", "number", "of", "opponents", "you", "have"],
    ])]);
const SOURCE_COUNTER_REFERENCE_PHRASES: &[&[&str]] = &[
    &["it"],
    &["this"],
    &["this", "artifact"],
    &["this", "creature"],
    &["this", "enchantment"],
    &["this", "equipment"],
    &["this", "land"],
    &["this", "permanent"],
    &["this", "source"],
];
const TAGGED_COUNTER_REFERENCE_PHRASES: &[&[&str]] = &[
    &["that"],
    &["that", "creature"],
    &["that", "permanent"],
    &["that", "object"],
    &["those"],
    &["those", "creatures"],
    &["those", "permanents"],
];

fn counters_on_source_reference_value(
    reference: &[&str],
    counter_type: Option<crate::CounterType>,
) -> Value {
    let surface = source_reference_surface_for_words(reference).or_else(|| {
        (reference.len() > 1)
            .then(|| this_source_surface_for_words(reference))
            .flatten()
    });
    Value::counters_on_source_reference(counter_type, surface)
}

fn is_source_counter_reference(reference: &[&str]) -> bool {
    value_helper_words_equal_any(reference, SOURCE_COUNTER_REFERENCE_PHRASES)
        || source_reference_surface_for_words(reference).is_some()
        || (reference.len() > 1 && this_source_surface_for_words(reference).is_some())
}
const COMMANDER_YOU_OWN_BATTLEFIELD_OR_COMMAND_ZONE_PHRASE: &[&str] = &[
    "commander",
    "you",
    "own",
    "on",
    "battlefield",
    "or",
    "in",
    "command",
    "zone",
];
const COMMANDER_ITERATED_PLAYER_OWNS_BATTLEFIELD_OR_COMMAND_ZONE_PHRASES: &[&[&str]] = &[
    &[
        "commander",
        "they",
        "own",
        "on",
        "battlefield",
        "or",
        "in",
        "command",
        "zone",
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
];
const OR_POWER_TOUGHNESS_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::any_phrase(&[
        &["or", "power"],
        &["or", "toughness"],
    ])]);

fn value_helper_words_match_pattern<'a>(words: &[&str], pattern: LexPattern<'a>) -> bool {
    use super::lex_patterns::LexPattern as _;
    pattern.match_word_refs(words).is_some()
}

fn value_helper_words_start_with_pattern<'a>(words: &[&str], pattern: LexPattern<'a>) -> bool {
    pattern.match_prefix_word_refs(words).is_some()
}

pub(crate) fn parse_aggregate_scope_value_lexed(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let word_view = ValueHelperCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    let matched = AGGREGATE_SCOPE_VALUE_PATTERN.match_word_refs(&words)?;
    let metric_range = matched.capture_word_range("metric")?;
    let scope_range = matched.capture_word_range("scope")?;
    let metric_words = words.get(metric_range)?;
    let scope_token_range =
        word_view.token_range_for_word_range(scope_range.start, scope_range.end)?;
    let scope_tokens = trim_edge_punctuation_tokens(&tokens[scope_token_range]);
    let filter = parse_object_filter_lexed(scope_tokens, false).ok()?;

    match metric_words {
        ["basic", "land", "type", "among"] | ["basic", "land", "types", "among"] => {
            Some(Value::BasicLandTypesAmong(filter))
        }
        ["creature", "type", "among"] | ["creature", "types", "among"] => {
            Some(Value::CreatureTypesAmong(filter))
        }
        ["color", "among"] | ["colors", "among"] => Some(Value::ColorsAmong(filter)),
        ["different", "powers", "among"]
        | ["different", "power", "values", "among"]
        | ["different", "power", "among"] => Some(Value::DistinctPowers(filter)),
        ["counter", "among"] | ["counters", "among"] => Some(Value::CountersOn(
            Box::new(crate::target::ChooseSpec::All(filter)),
            None,
        )),
        _ => None,
    }
}

fn value_helper_words_contain_any(words: &[&str], expected: &[&str]) -> bool {
    expected
        .iter()
        .any(|expected_word| words.iter().any(|word| word == expected_word))
}

fn value_helper_words_equal_any(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases.iter().any(|phrase| words == *phrase)
}

fn value_helper_find_exact_phrase(words: &[&str], phrase: &[&str]) -> Option<usize> {
    words
        .windows(phrase.len())
        .position(|window| window == phrase)
}

fn value_helper_find_any_phrase_start(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    phrases.iter().find_map(|phrase| {
        words
            .windows(phrase.len())
            .position(|window| window == *phrase)
    })
}

fn is_you_reference_word(word: &str) -> bool {
    matches!(word, "you" | "your" | "youve")
}

fn is_opponent_reference_word(word: &str) -> bool {
    matches!(word, "opponent" | "opponents")
}

fn is_mana_value_kind_word(word: &str) -> bool {
    word == "mana_value"
}

fn is_power_toughness_axis_word(word: &str) -> bool {
    matches!(word, "power" | "toughness")
}

fn is_plus_minus_word(word: &str) -> bool {
    matches!(word, "plus" | "minus")
}

fn is_and_or_word(word: &str) -> bool {
    matches!(word, "and" | "or" | "and/or")
}

fn is_comparison_tail_word(word: &str) -> bool {
    matches!(word, "less" | "fewer" | "greater" | "more")
}

fn is_less_or_fewer_word(word: &str) -> bool {
    matches!(word, "less" | "fewer")
}

fn word_refs_reference_prior_effect_objects(words: &[&str]) -> bool {
    value_helper_find_exact_phrase(words, THIS_WAY_PHRASE).is_some()
        || words.iter().enumerate().any(|(idx, word)| {
            if !PRIOR_EFFECT_OBJECT_MARKER_WORDS.contains(word) {
                return false;
            }
            if *word == "chosen"
                && words
                    .get(idx + 1)
                    .is_some_and(|next| matches!(*next, "type" | "color"))
            {
                return false;
            }
            true
        })
}

fn effect_metric_source_for_prior_effect_words(words: &[&str]) -> EffectMetricSource {
    if value_helper_words_contain_any(words, &["chosen"]) {
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
    if value_helper_words_equal_any(object_words, SOURCE_LINKED_EXILED_CARD_PHRASES) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        ))));
    }
    None
}

fn player_filter_for_spell_cast_this_turn_suffix(suffix: &[&str]) -> Option<PlayerFilter> {
    match suffix {
        ["theyve", "cast", "this", "turn"]
        | ["they", "cast", "this", "turn"]
        | ["that", "player", "cast", "this", "turn"] => Some(PlayerFilter::IteratedPlayer),
        ["youve", "cast", "this", "turn"] | ["you", "cast", "this", "turn"] => {
            Some(PlayerFilter::You)
        }
        ["an", "opponent", "has", "cast", "this", "turn"]
        | ["opponent", "has", "cast", "this", "turn"]
        | ["opponents", "have", "cast", "this", "turn"] => Some(PlayerFilter::Opponent),
        ["cast", "this", "turn"] => Some(PlayerFilter::Any),
        _ => None,
    }
}

pub(crate) fn parse_spells_cast_this_turn_matching_count_value_words(
    words: &[&str],
) -> Option<Value> {
    if !words.iter().any(|word| matches!(*word, "spell" | "spells"))
        || !words.iter().any(|word| matches!(*word, "cast" | "casts"))
        || !words.iter().any(|word| *word == "this")
        || !words.iter().any(|word| *word == "turn")
    {
        return None;
    }

    let matched = SPELL_CAST_THIS_TURN_VALUE_PATTERN.match_word_refs(words)?;
    let filter_range = matched.capture_word_range("filter")?;
    let suffix_range = matched.capture_word_range("cast_suffix")?;
    let filter_words = words.get(filter_range)?;
    let suffix_words = words.get(suffix_range)?;
    let filter = parse_object_filter_words(filter_words, false).ok()?;
    let player = player_filter_for_spell_cast_this_turn_suffix(suffix_words)?;
    let exclude_source = filter_words.iter().any(|word| *word == "other");
    Some(Value::SpellsCastThisTurnMatching {
        player,
        filter,
        exclude_source,
    })
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

    let matched = SPELL_CAST_THIS_TURN_VALUE_PATTERN.match_word_refs(&filter_words)?;
    let filter_range = matched.capture_word_range("filter")?;
    let suffix_range = matched.capture_word_range("cast_suffix")?;
    let filter_token_range =
        word_view.token_range_for_word_range(filter_range.start, filter_range.end)?;
    let filter_tokens = trim_commas(&tokens[filter_token_range]);
    let suffix_words = filter_words.get(suffix_range)?;
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    let player = player_filter_for_spell_cast_this_turn_suffix(suffix_words)?;
    let exclude_source = contains_token_word(&filter_tokens, "other");
    Some(Value::SpellsCastThisTurnMatching {
        player,
        filter,
        exclude_source,
    })
}

fn parse_creatures_died_this_turn_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = ValueHelperCompatWords::new(tokens);
    if value_helper_words_equal_any(&word_view.to_word_refs(), CREATURES_DIED_THIS_TURN_PHRASES) {
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
        .any(|word| is_you_reference_word(word))
    {
        return Some(Value::CardsDiscardedThisTurn(PlayerFilter::You));
    }
    if words
        .to_word_refs()
        .iter()
        .any(|word| is_opponent_reference_word(word))
    {
        return Some(Value::CardsDiscardedThisTurn(PlayerFilter::Opponent));
    }
    let word_refs = words.to_word_refs();
    if value_helper_find_any_phrase_start(&word_refs, THAT_PLAYER_PHRASES).is_some()
        || value_helper_words_contain_any(&word_refs, ITERATED_PLAYER_MARKER_WORDS)
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
        || value_helper_find_exact_phrase(&words, COMMAND_ZONE_CAST_PHRASE).is_none()
        || !word_view.contains_word("game")
    {
        return None;
    }

    if words.iter().any(|word| is_you_reference_word(word)) {
        return Some(PlayerFilter::You);
    }
    if words.iter().any(|word| is_opponent_reference_word(word)) {
        return Some(PlayerFilter::Opponent);
    }
    if value_helper_words_contain_any(&words, ITERATED_PLAYER_MARKER_WORDS)
        || value_helper_find_any_phrase_start(&words, THAT_PLAYER_PHRASES).is_some()
    {
        return Some(PlayerFilter::IteratedPlayer);
    }

    Some(PlayerFilter::Any)
}

pub(crate) fn parse_equal_to_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = ValueHelperCompatWords::new(tokens);
    let words_all = word_view.to_word_refs();
    let equal_idx = value_helper_find_exact_phrase(&words_all, EQUAL_TO_PHRASE)?;
    let prefix_start = equal_idx + EQUAL_TO_PHRASE.len();
    let suffix_refs = words_all.get(prefix_start..)?;
    let matched = EQUAL_TO_NUMBER_OF_PREFIX_PATTERN.match_prefix_word_refs(suffix_refs)?;
    let number_word_idx = prefix_start + matched.capture_word_range("number_of")?.start;

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
    if matches!(
        filter_words.as_slice(),
        ["creatures", "in", "your", "party"] | ["creature", "in", "your", "party"]
    ) {
        return Some(Value::PartySize(PlayerFilter::You));
    }
    if let Some(value) = parse_aggregate_scope_value_lexed(&filter_tokens) {
        return Some(value);
    }
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

pub(crate) fn parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let word_view = ValueHelperCompatWords::new(tokens);
    let clause_words = word_view.to_word_refs();
    if clause_words.as_slice() != EQUAL_TO_PHRASE {
        return None;
    }

    let suffix_refs = clause_words.get(EQUAL_TO_PHRASE.len()..)?;
    let matched = EQUAL_TO_NUMBER_OF_PREFIX_PATTERN.match_prefix_word_refs(suffix_refs)?;
    let filter_start_word_idx = EQUAL_TO_PHRASE.len() + matched.word_range.end;
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

    let signed_offset = if operator == "minus" {
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
    if value_helper_words_start_with_pattern(&clause_refs, EQUAL_TO_OPPONENTS_YOU_HAVE_PATTERN) {
        return Some(Value::CountPlayers(PlayerFilter::Opponent));
    }
    None
}

pub(crate) fn parse_equal_to_number_of_counters_on_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = ValueHelperCompatWords::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if !value_helper_find_exact_phrase(&clause_refs, EQUAL_TO_PHRASE).is_some_and(|idx| idx == 0) {
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
        .is_some_and(|word| is_article(word) || word == "one")
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

    if is_source_counter_reference(reference) {
        return Some(counters_on_source_reference_value(reference, counter_type));
    }

    if value_helper_words_equal_any(reference, TAGGED_COUNTER_REFERENCE_PHRASES) {
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
    let equal_idx = value_helper_find_exact_phrase(&clause_refs, EQUAL_TO_PHRASE)?;

    let prefix_start = equal_idx + EQUAL_TO_PHRASE.len();
    let suffix_refs = clause_refs.get(prefix_start..)?;
    let matched = EQUAL_TO_AGGREGATE_PREFIX_PATTERN.match_prefix_word_refs(suffix_refs)?;
    let aggregate = match suffix_refs.get(matched.capture_word_range("aggregate")?.start) {
        Some(&"total") => "total",
        Some(&"greatest") => "greatest",
        _ => return None,
    };
    let value_kind = match suffix_refs.get(matched.capture_word_range("value_kind")?) {
        Some(["power"]) => "power",
        Some(["toughness"]) => "toughness",
        Some(["mana", "value"]) => "mana_value",
        _ => return None,
    };
    let idx = prefix_start + matched.word_range.end;

    if aggregate == "greatest" && is_mana_value_kind_word(value_kind) {
        if let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx) {
            return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
        }
    }

    let filter_range = clause_words.token_range_for_word_range(idx, clause_words.len())?;
    let filter_tokens = &tokens[filter_range];
    let object_words = &clause_refs[idx..];
    if is_mana_value_kind_word(value_kind)
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
    if words == COMMANDER_YOU_OWN_BATTLEFIELD_OR_COMMAND_ZONE_PHRASE {
        return Some(PlayerFilter::You);
    }
    if value_helper_words_equal_any(
        words,
        COMMANDER_ITERATED_PLAYER_OWNS_BATTLEFIELD_OR_COMMAND_ZONE_PHRASES,
    ) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    None
}

pub(crate) fn parse_equal_to_number_of_filter_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words_all = ValueHelperCompatWords::new(tokens);
    let words_refs = words_all.to_word_refs();
    let equal_idx = value_helper_find_exact_phrase(&words_refs, EQUAL_TO_PHRASE)?;
    let prefix_start = equal_idx + EQUAL_TO_PHRASE.len();
    let suffix_refs = words_refs.get(prefix_start..)?;
    let matched = EQUAL_TO_NUMBER_OF_PREFIX_PATTERN.match_prefix_word_refs(suffix_refs)?;
    let number_word_idx = prefix_start + matched.capture_word_range("number_of")?.start;

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
    if matches!(
        filter_words.as_slice(),
        ["creatures", "in", "your", "party"] | ["creature", "in", "your", "party"]
    ) {
        return Some(Value::PartySize(PlayerFilter::You));
    }
    if let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) {
        return Some(value);
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
    if !value_helper_find_exact_phrase(&clause_refs, EQUAL_TO_PHRASE).is_some_and(|idx| idx == 0) {
        return None;
    }

    let suffix_refs = clause_refs.get(EQUAL_TO_PHRASE.len()..)?;
    let matched = EQUAL_TO_NUMBER_OF_PREFIX_PATTERN.match_prefix_word_refs(suffix_refs)?;
    let filter_start_word_idx = EQUAL_TO_PHRASE.len() + matched.word_range.end;
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

    let signed_offset = if operator == "minus" {
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
    if value_helper_words_start_with_pattern(
        &clause_words.to_word_refs(),
        EQUAL_TO_OPPONENTS_YOU_HAVE_PATTERN,
    ) {
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
    if !value_helper_find_exact_phrase(&clause_refs, EQUAL_TO_PHRASE).is_some_and(|idx| idx == 0) {
        return None;
    }

    let mut idx = 2usize;
    if clause_words.at_is(idx, "the") {
        idx += 1;
    }
    if !clause_refs
        .get(idx..idx + NUMBER_OF_PHRASE.len())
        .is_some_and(|words| words == NUMBER_OF_PHRASE)
    {
        return None;
    }
    idx += 2;

    if clause_words
        .get(idx)
        .is_some_and(|word| is_article(word) || word == "one")
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

    if is_source_counter_reference(reference) {
        return Some(
            counters_on_source_reference_value(reference, counter_type)
                .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }

    if value_helper_words_equal_any(reference, TAGGED_COUNTER_REFERENCE_PHRASES) {
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
    let equal_idx = value_helper_find_exact_phrase(&clause_refs, EQUAL_TO_PHRASE)?;

    let prefix_start = equal_idx + EQUAL_TO_PHRASE.len();
    let suffix_refs = clause_refs.get(prefix_start..)?;
    let matched = EQUAL_TO_AGGREGATE_PREFIX_PATTERN.match_prefix_word_refs(suffix_refs)?;
    let aggregate = match suffix_refs.get(matched.capture_word_range("aggregate")?.start) {
        Some(&"total") => "total",
        Some(&"greatest") => "greatest",
        _ => return None,
    };
    let value_kind = match suffix_refs.get(matched.capture_word_range("value_kind")?) {
        Some(["power"]) => "power",
        Some(["toughness"]) => "toughness",
        Some(["mana", "value"]) => "mana_value",
        _ => return None,
    };
    let idx = prefix_start + matched.word_range.end;

    if aggregate == "greatest" && is_mana_value_kind_word(value_kind) {
        if let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx) {
            return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
        }
    }

    let filter_range = clause_words.token_range_for_word_range(idx, clause_words.len())?;
    let filter_tokens = &tokens[filter_range];
    let object_words = &clause_refs[idx..];
    if is_mana_value_kind_word(value_kind)
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

    let word_refs = filter_words.to_word_refs();
    let matched = SPELL_CAST_THIS_TURN_VALUE_PATTERN.match_word_refs(&word_refs)?;
    let filter_range = matched.capture_word_range("filter")?;
    let suffix_range = matched.capture_word_range("cast_suffix")?;
    let filter_token_range =
        filter_words.token_range_for_word_range(filter_range.start, filter_range.end)?;
    let filter_tokens = trim_lexed_commas(&tokens[filter_token_range]);
    let suffix_words = word_refs.get(suffix_range)?;
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    let player = player_filter_for_spell_cast_this_turn_suffix(suffix_words)?;
    let exclude_source = contains_token_word(filter_tokens, "other");
    Some(Value::SpellsCastThisTurnMatching {
        player,
        filter,
        exclude_source,
    })
}

pub(crate) fn parse_filter_comparison_tokens(
    axis: &str,
    tokens: &[&str],
    clause_words: &[&str],
) -> Result<Option<(crate::filter::Comparison, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    if is_power_toughness_axis_word(axis)
        && value_helper_words_start_with_pattern(tokens, OR_POWER_TOUGHNESS_PATTERN)
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
        if tokens.get(1).is_some_and(|word| is_plus_minus_word(word)) {
            let (cmp, used) = parse_operand(tokens, "eq")?;
            return Ok(Some((cmp, used)));
        }
        let mut values = vec![value];
        let mut consumed = 1usize;
        while consumed < tokens.len() {
            let token = tokens[consumed];
            if is_and_or_word(token) {
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

    if let Some((operator, operand_words, consumed_base)) = parse_value_comparison_words(tokens) {
        if operand_words.is_empty() {
            let consumed_phrase = consumed_base;
            let phrase = tokens[..consumed_phrase].join(" ");
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after '{phrase}' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (operand, used) = parse_value_expr_words(operand_words).ok_or_else(|| {
            let quoted = operand_words.first().copied().unwrap_or_default();
            CardTextError::ParseError(format!(
                "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let consumed = consumed_base + used;
        return Ok(Some((map_operator(operator, operand), consumed)));
    }

    if let Some((value, used)) = parse_value_expr_words(tokens) {
        if tokens.get(used).copied() == Some("or")
            && let Some(next) = tokens.get(used + 1)
            && is_comparison_tail_word(next)
        {
            let kind = if is_less_or_fewer_word(next) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardType;

    fn lex_words(text: &str) -> Vec<OwnedLexToken> {
        let mut tokens =
            crate::runtime_backend::lexer::lex_line(text, 0).expect("test phrase should lex");
        for token in &mut tokens {
            token.lowercase_word();
        }
        tokens
    }

    #[test]
    fn parse_aggregate_scope_value_lexed_uses_captured_metric_and_scope() {
        let color_tokens = lex_words("colors among creatures you control");
        let color_value = parse_aggregate_scope_value_lexed(&color_tokens)
            .expect("colors-among aggregate should parse");
        let Value::ColorsAmong(color_filter) = color_value else {
            panic!("expected colors-among value, got {color_value:?}");
        };
        assert_eq!(color_filter.card_types, vec![CardType::Creature]);
        assert_eq!(color_filter.controller, Some(PlayerFilter::You));

        let power_tokens = lex_words("different powers among creatures you control");
        let power_value = parse_aggregate_scope_value_lexed(&power_tokens)
            .expect("distinct-powers aggregate should parse");
        let Value::DistinctPowers(power_filter) = power_value else {
            panic!("expected distinct-powers value, got {power_value:?}");
        };
        assert_eq!(power_filter.card_types, vec![CardType::Creature]);
        assert_eq!(power_filter.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn parse_spells_cast_this_turn_matching_count_value_lexed_uses_captured_suffix() {
        let tokens = lex_words("other creature spells an opponent has cast this turn");
        let value = parse_spells_cast_this_turn_matching_count_value_lexed(&tokens)
            .expect("spell-cast count should parse");
        let Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } = value
        else {
            panic!("expected spell-cast matching value, got {value:?}");
        };
        assert_eq!(player, PlayerFilter::Opponent);
        assert!(exclude_source);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        );
    }
}
