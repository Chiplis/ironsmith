use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::{Value, ValueComparisonOperator};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::{ObjectFilter, Zone};
use ironsmith_core::EffectMetric;
use ironsmith_core::ValueSurfaceHint;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

use crate::runtime_backend::object_filters::{parse_object_filter, parse_object_filter_lexed};
use crate::runtime_backend::util::{
    trim_commas, trim_edge_punctuation, trim_edge_punctuation_tokens,
};

use super::super::super::lexer::{OwnedLexToken, trim_lexed_commas};
use super::super::leaf;
use super::super::primitives::{self, TokenWordView, WordSliceInput};
use super::super::values::parse_value_comparison_words;
pub(crate) use super::super::values::{parse_number_prefix_lexed, parse_value_prefix_lexed};
use super::value_expr;
use super::value_helper_shapes;
use super::value_shapes::{self, AggregateValueMetric};

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
const EQUAL_TO_PHRASE: &[&str] = &["equal", "to"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EqualToStart {
    start: usize,
    after: usize,
}

fn parse_equal_to_start(words: &[&str]) -> Option<EqualToStart> {
    let mut input: WordSliceInput<'_> = words;
    parse_equal_to_start_words.parse_next(&mut input).ok()
}

fn parse_equal_to_start_words(
    input: &mut WordSliceInput<'_>,
) -> Result<EqualToStart, ErrMode<ContextError>> {
    let initial_len = input.len();
    loop {
        let checkpoint = *input;
        if (
            primitives::word_slice_exact("equal"),
            primitives::word_slice_exact("to"),
        )
            .void()
            .parse_next(input)
            .is_ok()
        {
            let after = initial_len.saturating_sub(input.len());
            return Ok(EqualToStart {
                start: after.saturating_sub(EQUAL_TO_PHRASE.len()),
                after,
            });
        }
        *input = checkpoint;
        any.void().parse_next(input)?;
    }
}

fn words_match_any_phrase(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::parse_word_sequence_complete(words, phrase).is_some())
}

fn counter_reference_shape_value(shape: value_helper_shapes::CounterReferenceValueShape) -> Value {
    match shape.reference {
        value_helper_shapes::CounterValueReference::Source(surface) => {
            Value::counters_on_source_reference(shape.counter_type, surface)
        }
        value_helper_shapes::CounterValueReference::Tagged => Value::CountersOn(
            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
            shape.counter_type,
        ),
    }
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
pub(crate) fn parse_aggregate_scope_value_lexed(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let surface = value_shapes::parse_aggregate_value_surface(&words)?;
    let scope_start = words.len().checked_sub(surface.scope_words.len())?;
    let scope_token_range = word_view.token_span_for_words(scope_start, words.len())?;
    let scope_tokens = trim_edge_punctuation_tokens(&tokens[scope_token_range]);
    let filter = parse_object_filter_lexed(scope_tokens, false).ok()?;

    match surface.metric {
        AggregateValueMetric::BasicLandTypes => Some(Value::BasicLandTypesAmong(filter)),
        AggregateValueMetric::CreatureTypes => Some(Value::CreatureTypesAmong(filter)),
        AggregateValueMetric::Colors => Some(Value::ColorsAmong(filter)),
        AggregateValueMetric::DistinctPowers => Some(Value::DistinctPowers(filter)),
        AggregateValueMetric::Counters => Some(Value::CountersOn(
            Box::new(crate::target::ChooseSpec::All(filter)),
            None,
        )),
    }
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

fn aggregate_effect_metric(
    aggregate: value_helper_shapes::AggregateKind,
    value_kind: value_helper_shapes::AggregateValueKind,
) -> EffectMetric {
    use value_helper_shapes::{AggregateKind, AggregateValueKind};

    match (aggregate, value_kind) {
        (AggregateKind::Total, AggregateValueKind::Power) => EffectMetric::TotalPower,
        (AggregateKind::Total, AggregateValueKind::Toughness) => EffectMetric::TotalToughness,
        (AggregateKind::Total, AggregateValueKind::ManaValue) => EffectMetric::TotalManaValue,
        (AggregateKind::Greatest, AggregateValueKind::Power) => EffectMetric::GreatestPower,
        (AggregateKind::Greatest, AggregateValueKind::Toughness) => EffectMetric::GreatestToughness,
        (AggregateKind::Greatest, AggregateValueKind::ManaValue) => EffectMetric::GreatestManaValue,
    }
}

fn pending_aggregate_metric_value(
    aggregate: value_helper_shapes::AggregateKind,
    value_kind: value_helper_shapes::AggregateValueKind,
    object_words: &[&str],
) -> Option<Value> {
    Some(Value::PendingEffectMetric {
        source: value_helper_shapes::parse_prior_effect_metric_source(object_words)?,
        metric: aggregate_effect_metric(aggregate, value_kind),
    })
}

fn aggregate_filter_value(
    aggregate: value_helper_shapes::AggregateKind,
    value_kind: value_helper_shapes::AggregateValueKind,
    filter: ObjectFilter,
) -> Value {
    use value_helper_shapes::{AggregateKind, AggregateValueKind};

    match (aggregate, value_kind) {
        (AggregateKind::Total, AggregateValueKind::Power) => Value::TotalPower(filter),
        (AggregateKind::Total, AggregateValueKind::Toughness) => Value::TotalToughness(filter),
        (AggregateKind::Total, AggregateValueKind::ManaValue) => Value::TotalManaValue(filter),
        (AggregateKind::Greatest, AggregateValueKind::Power) => Value::GreatestPower(filter),
        (AggregateKind::Greatest, AggregateValueKind::Toughness) => {
            Value::GreatestToughness(filter)
        }
        (AggregateKind::Greatest, AggregateValueKind::ManaValue) => {
            Value::GreatestManaValue(filter)
        }
    }
}

fn source_linked_exiled_mana_value(object_words: &[&str]) -> Option<Value> {
    if words_match_any_phrase(object_words, SOURCE_LINKED_EXILED_CARD_PHRASES) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        ))));
    }
    None
}

fn parse_spells_cast_this_turn_matching_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let filter_words = word_view.to_word_refs();
    let surface = value_helper_shapes::parse_spell_cast_this_turn_surface(&filter_words)?;
    let filter_token_range = word_view.token_span_for_words(0, surface.filter_end)?;
    let filter_tokens = trim_commas(&tokens[filter_token_range]);
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::SpellsCastThisTurnMatching {
        player: surface.player,
        filter,
        exclude_source: surface.exclude_source,
    })
}

fn parse_creatures_died_this_turn_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    if words_match_any_phrase(&word_view.to_word_refs(), CREATURES_DIED_THIS_TURN_PHRASES) {
        Some(Value::CreaturesDiedThisTurn)
    } else {
        None
    }
}

fn parse_cards_discarded_this_turn_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = TokenWordView::new(tokens).to_word_refs();
    value_helper_shapes::parse_cards_discarded_this_turn_player(&words)
        .map(Value::CardsDiscardedThisTurn)
}

pub(crate) fn parse_commander_cast_count_player(tokens: &[OwnedLexToken]) -> Option<PlayerFilter> {
    let words = TokenWordView::new(tokens).to_word_refs();
    value_helper_shapes::parse_commander_cast_count_player(&words)
}

pub(crate) fn parse_equal_to_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let words_all = word_view.to_word_refs();
    let prefix_start = parse_equal_to_start(&words_all)?.after;
    let suffix_refs = words_all.get(prefix_start..)?;
    let matched = value_helper_shapes::parse_number_of_prefix(suffix_refs)?;
    let number_word_idx = prefix_start + matched.number_of_start;

    let value_range = word_view.token_span_for_words(number_word_idx, word_view.len())?;
    let value_tokens = trim_edge_punctuation(&tokens[value_range]);
    if let Some((value, used)) = value_expr::parse_value_expr_tokens(&value_tokens)
        && TokenWordView::new(&value_tokens[used..]).is_empty()
    {
        return Some(value);
    }

    let filter_start_word_idx = number_word_idx + 2;
    let filter_range = word_view.token_span_for_words(filter_start_word_idx, word_view.len())?;
    let filter_tokens = trim_edge_punctuation(&tokens[filter_range]);
    let filter_word_view = TokenWordView::new(&filter_tokens);
    let filter_words = filter_word_view.to_word_refs();
    if let Some(value) = parse_creatures_died_this_turn_count_value(&filter_tokens) {
        return Some(value);
    }
    if let Some(value) = parse_cards_discarded_this_turn_count_value(&filter_tokens) {
        return Some(value);
    }
    if let Some(player) = value_helper_shapes::parse_cards_in_hand_player(&filter_words) {
        return Some(Value::CardsInHand(player));
    }
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        return Some(value);
    }
    if let Some(player) = value_helper_shapes::parse_party_size_player(&filter_words) {
        return Some(Value::PartySize(player));
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
    let word_view = TokenWordView::new(tokens);
    let clause_words = word_view.to_word_refs();
    if primitives::parse_word_sequence_complete(&clause_words, EQUAL_TO_PHRASE).is_none() {
        return None;
    }

    let suffix_refs = clause_words.get(EQUAL_TO_PHRASE.len()..)?;
    let matched = value_helper_shapes::parse_number_of_prefix(suffix_refs)?;
    let filter_start_word_idx = EQUAL_TO_PHRASE.len() + matched.consumed;
    let operator_word_idx =
        word_view.find_any_word_from(&["plus", "minus"], filter_start_word_idx + 1)?;
    let operator = clause_words[operator_word_idx];

    let filter_range = word_view.token_span_for_words(filter_start_word_idx, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_range]);
    let base_value = if let Some(value) = parse_creatures_died_this_turn_count_value(&filter_tokens)
    {
        value
    } else if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        value
    } else {
        Value::Count(parse_object_filter(&filter_tokens, false).ok()?)
    };

    let offset_range = word_view.token_span_for_words(operator_word_idx + 1, word_view.len())?;
    let offset_tokens = trim_commas(&tokens[offset_range]);
    let (offset_value, used) =
        leaf::parse_leaf_number_prefix_tokens(&offset_tokens)?.into_fixed()?;
    if !TokenWordView::new(&offset_tokens[used..]).is_empty() {
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
    let clause_words = TokenWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if value_helper_shapes::starts_equal_to_opponents_you_have(&clause_refs) {
        return Some(Value::CountPlayers(PlayerFilter::Opponent));
    }
    None
}

pub(crate) fn parse_equal_to_number_of_counters_on_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let shape = value_helper_shapes::parse_counter_reference_value_shape(&words)?;
    Some(counter_reference_shape_value(shape))
}

pub(crate) fn parse_equal_to_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = TokenWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    let prefix_start = parse_equal_to_start(&clause_refs)?.after;
    let suffix_refs = clause_refs.get(prefix_start..)?;
    let matched = value_helper_shapes::parse_aggregate_prefix(suffix_refs)?;
    let aggregate = matched.aggregate;
    let value_kind = matched.value_kind;
    let idx = prefix_start + matched.consumed;

    if aggregate == value_helper_shapes::AggregateKind::Greatest
        && value_kind == value_helper_shapes::AggregateValueKind::ManaValue
    {
        if let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx) {
            return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
        }
    }

    let filter_range = clause_words.token_span_for_words(idx, clause_words.len())?;
    let filter_tokens = &tokens[filter_range];
    let object_words = &clause_refs[idx..];
    if value_kind == value_helper_shapes::AggregateValueKind::ManaValue
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

    Some(aggregate_filter_value(aggregate, value_kind, filter))
}

pub(crate) fn parse_where_x_greatest_commander_mana_value(
    tokens: &[OwnedLexToken],
    commander_start_word_idx: usize,
) -> Option<Value> {
    let words = TokenWordView::new(tokens);
    let commander_range = words.token_span_for_words(commander_start_word_idx, words.len())?;
    let commander_words = crate::runtime_backend::token_word_refs(&tokens[commander_range]);
    let normalized = commander_words
        .iter()
        .copied()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect::<Vec<_>>();
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
    if words_match_any_phrase(
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
    let words_all = TokenWordView::new(tokens);
    let words_refs = words_all.to_word_refs();
    let prefix_start = parse_equal_to_start(&words_refs)?.after;
    let suffix_refs = words_refs.get(prefix_start..)?;
    let matched = value_helper_shapes::parse_number_of_prefix(suffix_refs)?;
    let number_word_idx = prefix_start + matched.number_of_start;

    let value_range = words_all.token_span_for_words(number_word_idx, words_all.len())?;
    let value_tokens = trim_edge_punctuation_tokens(&tokens[value_range]);
    if let Some((value, used)) = parse_value_prefix_lexed(value_tokens) {
        if TokenWordView::new(&value_tokens[used..]).is_empty() {
            return Some(value);
        }
    }

    let filter_start_word_idx = number_word_idx + 2;
    let filter_range = words_all.token_span_for_words(filter_start_word_idx, words_all.len())?;
    let filter_tokens = trim_edge_punctuation_tokens(&tokens[filter_range]);
    let filter_words = TokenWordView::new(filter_tokens).to_word_refs();
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value_lexed(filter_tokens) {
        return Some(value);
    }
    if let Some(value) = parse_cards_discarded_this_turn_count_value(filter_tokens) {
        return Some(value);
    }
    if let Some(player) = value_helper_shapes::parse_party_size_player(&filter_words) {
        return Some(Value::PartySize(player));
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
    let clause_words = TokenWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if !parse_equal_to_start(&clause_refs).is_some_and(|parsed| parsed.start == 0) {
        return None;
    }

    let suffix_refs = clause_refs.get(EQUAL_TO_PHRASE.len()..)?;
    let matched = value_helper_shapes::parse_number_of_prefix(suffix_refs)?;
    let filter_start_word_idx = EQUAL_TO_PHRASE.len() + matched.consumed;
    let operator_word_idx =
        clause_words.find_any_word_from(&["plus", "minus"], filter_start_word_idx + 1)?;
    let operator = clause_words.get(operator_word_idx)?;

    let filter_range =
        clause_words.token_span_for_words(filter_start_word_idx, operator_word_idx)?;
    let filter_tokens = trim_lexed_commas(&tokens[filter_range]);
    let base_value = if let Some(value) =
        parse_spells_cast_this_turn_matching_count_value_lexed(filter_tokens)
    {
        value
    } else {
        Value::Count(parse_object_filter_lexed(filter_tokens, false).ok()?)
    };

    let offset_range =
        clause_words.token_span_for_words(operator_word_idx + 1, clause_words.len())?;
    let offset_tokens = trim_lexed_commas(&tokens[offset_range]);
    let (offset_value, used) = parse_number_prefix_lexed(offset_tokens)?;
    if !TokenWordView::new(&offset_tokens[used..]).is_empty() {
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
    let clause_words = TokenWordView::new(tokens);
    if value_helper_shapes::starts_equal_to_opponents_you_have(&clause_words.to_word_refs()) {
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
    let words = TokenWordView::new(tokens).to_word_refs();
    let shape = value_helper_shapes::parse_counter_reference_value_shape(&words)?;
    Some(counter_reference_shape_value(shape).with_surface_hint(ValueSurfaceHint::EqualTo))
}

pub(crate) fn parse_equal_to_aggregate_filter_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = TokenWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    let prefix_start = parse_equal_to_start(&clause_refs)?.after;
    let suffix_refs = clause_refs.get(prefix_start..)?;
    let matched = value_helper_shapes::parse_aggregate_prefix(suffix_refs)?;
    let aggregate = matched.aggregate;
    let value_kind = matched.value_kind;
    let idx = prefix_start + matched.consumed;

    if aggregate == value_helper_shapes::AggregateKind::Greatest
        && value_kind == value_helper_shapes::AggregateValueKind::ManaValue
    {
        if let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx) {
            return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
        }
    }

    let filter_range = clause_words.token_span_for_words(idx, clause_words.len())?;
    let filter_tokens = &tokens[filter_range];
    let object_words = &clause_refs[idx..];
    if value_kind == value_helper_shapes::AggregateValueKind::ManaValue
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

    Some(aggregate_filter_value(aggregate, value_kind, filter))
}

pub(crate) fn parse_spells_cast_this_turn_matching_count_value_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_words = TokenWordView::new(tokens);
    let word_refs = filter_words.to_word_refs();
    let surface = value_helper_shapes::parse_spell_cast_this_turn_surface(&word_refs)?;
    let filter_token_range = filter_words.token_span_for_words(0, surface.filter_end)?;
    let filter_tokens = trim_lexed_commas(&tokens[filter_token_range]);
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    Some(Value::SpellsCastThisTurnMatching {
        player: surface.player,
        filter,
        exclude_source: surface.exclude_source,
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

    if is_power_toughness_axis_word(axis) && value_helper_shapes::starts_or_power_toughness(tokens)
    {
        return Ok(None);
    }

    let to_comparison = |operator: ValueComparisonOperator,
                         operand: Value|
     -> crate::filter::Comparison {
        use crate::filter::Comparison;

        match (operator, operand) {
            (ValueComparisonOperator::Equal, Value::Fixed(value)) => Comparison::Equal(value),
            (ValueComparisonOperator::NotEqual, Value::Fixed(value)) => Comparison::NotEqual(value),
            (ValueComparisonOperator::LessThan, Value::Fixed(value)) => Comparison::LessThan(value),
            (ValueComparisonOperator::LessThanOrEqual, Value::Fixed(value)) => {
                Comparison::LessThanOrEqual(value)
            }
            (ValueComparisonOperator::GreaterThan, Value::Fixed(value)) => {
                Comparison::GreaterThan(value)
            }
            (ValueComparisonOperator::GreaterThanOrEqual, Value::Fixed(value)) => {
                Comparison::GreaterThanOrEqual(value)
            }
            (ValueComparisonOperator::Equal, operand) => Comparison::EqualExpr(Box::new(operand)),
            (ValueComparisonOperator::NotEqual, operand) => {
                Comparison::NotEqualExpr(Box::new(operand))
            }
            (ValueComparisonOperator::LessThan, operand) => {
                Comparison::LessThanExpr(Box::new(operand))
            }
            (ValueComparisonOperator::LessThanOrEqual, operand) => {
                Comparison::LessThanOrEqualExpr(Box::new(operand))
            }
            (ValueComparisonOperator::GreaterThan, operand) => {
                Comparison::GreaterThanExpr(Box::new(operand))
            }
            (ValueComparisonOperator::GreaterThanOrEqual, operand) => {
                Comparison::GreaterThanOrEqualExpr(Box::new(operand))
            }
        }
    };

    let parse_operand = |operand_tokens: &[&str],
                         operator: ValueComparisonOperator|
     -> Result<(crate::filter::Comparison, usize), CardTextError> {
        let Some((operand, used)) = value_expr::parse_value_expr_words(operand_tokens) else {
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
        Ok((to_comparison(operator, operand), used))
    };

    let parse_numeric_token = |word: &str| -> Option<i32> {
        if let Ok(value) = word.parse::<i32>() {
            return Some(value);
        }
        leaf::parse_number_i32_complete(word).ok()
    };

    let first = tokens[0];
    if let Some(value) = parse_numeric_token(first) {
        if tokens.get(1).is_some_and(|word| is_plus_minus_word(word)) {
            let (cmp, used) = parse_operand(tokens, ValueComparisonOperator::Equal)?;
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
        let (operand, used) =
            value_expr::parse_value_expr_words(operand_words).ok_or_else(|| {
                let quoted = operand_words.first().copied().unwrap_or_default();
                CardTextError::ParseError(format!(
                    "unsupported dynamic {axis} comparison operand '{quoted}' (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let consumed = consumed_base + used;
        return Ok(Some((to_comparison(operator, operand), consumed)));
    }

    if let Some((value, used)) = value_expr::parse_value_expr_words(tokens) {
        if tokens.get(used).copied() == Some("or")
            && let Some(next) = tokens.get(used + 1)
            && is_comparison_tail_word(next)
        {
            let operator = if is_less_or_fewer_word(next) {
                ValueComparisonOperator::LessThanOrEqual
            } else {
                ValueComparisonOperator::GreaterThanOrEqual
            };
            return Ok(Some((to_comparison(operator, value), used + 2)));
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
    fn equal_to_parser_returns_typed_word_boundaries() {
        assert_eq!(
            parse_equal_to_start(&["where", "x", "is", "equal", "to", "the"]),
            Some(EqualToStart { start: 3, after: 5 })
        );
        assert_eq!(parse_equal_to_start(&["not", "equal"]), None);
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
