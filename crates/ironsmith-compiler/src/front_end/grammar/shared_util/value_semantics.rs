use crate::cards::builders::{CardTextError, IT_TAG, TagKey};
use crate::effect::{Value, ValueComparisonOperator};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::{ObjectFilter, Zone};
use ironsmith_core::EffectMetric;
use ironsmith_core::TurnHistoryCount;
use ironsmith_core::ValueSurfaceHint;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

use crate::object_filters::{
    parse_object_filter, parse_object_filter_lexed, parse_object_filter_words,
};
use crate::util::{
    parse_greater_than_or_equal_quantity_prefix, possessive_normalized_word_refs, trim_commas,
    trim_edge_punctuation, trim_edge_punctuation_tokens,
};

use super::super::super::lexer::{OwnedLexToken, render_token_slice, trim_lexed_commas};
use super::super::leaf;
use super::super::primitives::{self, TokenWordView, WordSliceInput};
use super::super::values::parse_value_comparison_words;
pub use super::super::values::{parse_number_prefix_lexed, parse_value_prefix_lexed};
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

/// Parse an authored mana-symbol payment total such as
/// `the amount of {S} spent to cast this spell`.
///
/// This must stay lexed: mana groups intentionally do not appear in
/// `TokenWordView`, so a word-only value parser cannot preserve which symbol
/// the count refers to.
pub fn parse_mana_symbol_spent_to_cast_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let mut mana_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| token.mana_group_inner().map(|_| index));
    let mana_index = mana_indices.next()?;
    if mana_indices.next().is_some() {
        return None;
    }

    let prefix = TokenWordView::new(&tokens[..mana_index]).to_word_refs();
    if !matches!(
        prefix.as_slice(),
        ["the", "amount", "of"]
            | ["amount", "of"]
            | ["where", "x", "is", "the", "amount", "of"]
            | ["where", "x", "is", "amount", "of"]
    ) {
        return None;
    }

    let suffix = TokenWordView::new(&tokens[mana_index + 1..]).to_word_refs();
    let reference = match suffix.as_slice() {
        ["spent", "to", "cast", "it"] => ironsmith_core::ManaSpentCastReferenceSurface::It,
        ["spent", "to", "cast", "this", "spell"] => {
            ironsmith_core::ManaSpentCastReferenceSurface::ThisSpell
        }
        ["spent", "to", "cast", "this", "creature"] => {
            ironsmith_core::ManaSpentCastReferenceSurface::ThisCreature
        }
        _ => return None,
    };
    // Reject hidden punctuation or other non-word tokens in an otherwise
    // matching surface instead of silently treating them as absent.
    if prefix.len() + suffix.len() + 1 != tokens.len() {
        return None;
    }

    let symbols =
        super::super::values::parse_mana_symbol_group(tokens[mana_index].parser_text()).ok()?;
    let [symbol] = symbols.as_slice() else {
        return None;
    };
    if !matches!(
        symbol,
        crate::mana::ManaSymbol::White
            | crate::mana::ManaSymbol::Blue
            | crate::mana::ManaSymbol::Black
            | crate::mana::ManaSymbol::Red
            | crate::mana::ManaSymbol::Green
            | crate::mana::ManaSymbol::Colorless
            | crate::mana::ManaSymbol::Snow
    ) {
        return None;
    }

    Some(Value::ManaSymbolSpentToCastThisSpell {
        symbol: *symbol,
        reference,
    })
}

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
pub fn parse_aggregate_scope_value_lexed(tokens: &[OwnedLexToken]) -> Option<Value> {
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
        AggregateValueMetric::CardTypes => Some(Value::CardTypesAmong(filter)),
        AggregateValueMetric::CreatureTypes => Some(Value::CreatureTypesAmong(filter)),
        AggregateValueMetric::Colors => Some(Value::ColorsAmong(filter)),
        AggregateValueMetric::ColorPairs => Some(Value::ColorPairsAmong(filter)),
        AggregateValueMetric::DistinctNames => Some(Value::DistinctNames(filter)),
        AggregateValueMetric::DistinctPowers => Some(Value::DistinctPowers(filter)),
        AggregateValueMetric::Counters => Some(
            Value::CountersOn(Box::new(crate::target::ChooseSpec::All(filter)), None)
                .with_surface_hint(ValueSurfaceHint::CountersAmong),
        ),
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
    let source = value_helper_shapes::parse_prior_effect_metric_source(object_words)?;
    let metric = aggregate_effect_metric(aggregate, value_kind);
    if let Some(value) = parse_prior_effect_aggregate_metric_value(metric, object_words) {
        return Some(value);
    }
    Some(Value::PendingEffectMetric { source, metric })
}

pub fn parse_prior_effect_aggregate_metric_value(
    metric: EffectMetric,
    object_words: &[&str],
) -> Option<Value> {
    let source = value_helper_shapes::parse_prior_effect_metric_source(object_words)?;
    let this_way_start =
        crate::word_primitives::parse_sequence_start(object_words, &["this", "way"]);
    if let Some(this_way_start) = this_way_start {
        let subject = &object_words[..this_way_start];
        if let Some((action, action_start)) =
            value_helper_shapes::parse_prior_effect_action(subject)
        {
            let mut query =
                ironsmith_core::PriorEffectMetricQuery::new(source, metric).with_action(action);
            let filter_words = &subject[..action_start];
            if !filter_words.is_empty() {
                let mut filter = parse_object_filter_words(filter_words, false).ok()?;
                if filter_words
                    .iter()
                    .any(|word| matches!(*word, "card" | "cards"))
                {
                    filter.set_explicit_card_noun(true);
                }
                query = query.with_filter(filter);
            }
            return Some(Value::PendingPriorEffectMetric(query));
        }
    }
    None
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

fn history_filter_from_word_prefix(
    tokens: &[OwnedLexToken],
    words: &TokenWordView<'_>,
    end_word: usize,
) -> Option<ObjectFilter> {
    let range = words.token_span_for_words(0, end_word)?;
    let mut filter = parse_object_filter(&trim_edge_punctuation(&tokens[range]), false).ok()?;
    // Historical values match the event snapshot, not the object's current
    // zone.  Zone transitions are carried by the query variant itself.
    filter.zone = None;
    // A bare `spell` noun is represented by the stack-kind discriminator.
    // The general object-filter parser also supplies `has_mana_cost`, but that
    // would incorrectly exclude spells without mana costs from cast history.
    // Keep the spell discriminator as structured surface/semantic metadata and
    // remove only the accidental mana-cost restriction.
    if filter.stack_kind == Some(crate::filter::StackObjectKind::Spell) {
        filter.has_mana_cost = false;
    }
    Some(filter)
}

fn suffix_start(words: &[&str], suffix: &[&str]) -> Option<usize> {
    crate::word_primitives::parse_sequence_suffix(words, suffix)
        .then_some(words.len().saturating_sub(suffix.len()))
}

fn parse_spell_cast_history_count(
    tokens: &[OwnedLexToken],
    word_view: &TokenWordView<'_>,
    words: &[&str],
) -> Option<Value> {
    let suffixes: &[(&[&str], PlayerFilter, bool)] = &[
        (
            &["youve", "cast", "before", "it", "this", "turn"],
            PlayerFilter::You,
            true,
        ),
        (
            &["you've", "cast", "before", "it", "this", "turn"],
            PlayerFilter::You,
            true,
        ),
        (
            &["you", "have", "cast", "before", "it", "this", "turn"],
            PlayerFilter::You,
            true,
        ),
        (
            &["cast", "before", "that", "spell", "this", "turn"],
            PlayerFilter::Any,
            true,
        ),
        (
            &["cast", "before", "this", "spell", "this", "turn"],
            PlayerFilter::Any,
            true,
        ),
        (
            &["cast", "before", "it", "this", "turn"],
            PlayerFilter::Any,
            true,
        ),
        (&["youve", "cast", "this", "turn"], PlayerFilter::You, false),
        (
            &["you've", "cast", "this", "turn"],
            PlayerFilter::You,
            false,
        ),
        (
            &["you", "have", "cast", "this", "turn"],
            PlayerFilter::You,
            false,
        ),
        (&["you", "cast", "this", "turn"], PlayerFilter::You, false),
        (&["cast", "this", "turn"], PlayerFilter::Any, false),
    ];

    for (suffix, player, before_triggering_spell) in suffixes {
        let Some(end) = suffix_start(words, suffix) else {
            continue;
        };
        if end == 0 {
            continue;
        }
        let prefix_words = &words[..end];
        if !prefix_words
            .iter()
            .any(|word| matches!(*word, "spell" | "spells"))
        {
            continue;
        }
        let mut filter = history_filter_from_word_prefix(tokens, word_view, end)?;
        let exclude_source =
            filter.other || crate::word_primitives::sequence_occurs(prefix_words, &["other"]);
        // `other` is relative to the cast being evaluated, not to the source
        // permanent of a triggered ability. Keep that relation in the query.
        filter.other = false;
        return Some(Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
            player: player.clone(),
            filter,
            from_zone: None,
            from_outside_hand: false,
            exclude_source,
            before_triggering_spell: *before_triggering_spell,
        }));
    }
    None
}

/// Build the historical spell count used by an ordinal triggering-spell gate.
///
/// Unlike an ordinary "spells you've cast this turn" value, this count stops
/// at the cast event which caused the current trigger. That event boundary is
/// important: spells cast while the trigger is waiting on the stack must not
/// change whether the triggering spell was first, second, and so on.
pub fn parse_triggering_spell_history_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation(tokens);
    let word_view = TokenWordView::new(&tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let mut filter = history_filter_from_word_prefix(&tokens, &word_view, words.len())?;
    let exclude_source =
        filter.other || crate::word_primitives::sequence_occurs(&words, &["other"]);
    filter.other = false;
    Some(Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
        player: PlayerFilter::You,
        filter,
        from_zone: None,
        from_outside_hand: false,
        exclude_source,
        before_triggering_spell: true,
    }))
}

#[cfg(test)]
#[path = "value_semantics_inline_tests.rs"]
mod tests;

#[path = "value_semantics/value_semantics_reference_programs.rs"]
mod value_semantics_reference_programs;
pub use value_semantics_reference_programs::{
    parse_commander_cast_count_player, parse_equal_to_aggregate_filter_value,
    parse_equal_to_number_of_filter_plus_or_minus_fixed_value,
    parse_equal_to_number_of_filter_value, parse_filter_comparison_tokens,
};
#[path = "value_semantics/value_semantics_core_programs.rs"]
mod value_semantics_core_programs;
use value_semantics_core_programs::parse_creatures_died_this_turn_count_value;
pub use value_semantics_core_programs::{
    parse_equal_to_number_of_opponents_you_have_value, parse_turn_history_count_value,
    parse_turn_history_value_binding, starts_explicit_ordered_comparison,
};
#[path = "value_semantics/value_semantics_permission_programs.rs"]
mod value_semantics_permission_programs;
use value_semantics_permission_programs::parse_spells_cast_this_turn_matching_count_value;
pub use value_semantics_permission_programs::parse_spells_cast_this_turn_matching_count_value_lexed;
#[path = "value_semantics/value_semantics_zone_programs.rs"]
mod value_semantics_zone_programs;
use value_semantics_zone_programs::commander_owner_from_battlefield_or_command_zone_words;
#[path = "value_semantics/value_semantics_resource_programs.rs"]
mod value_semantics_resource_programs;
pub use value_semantics_resource_programs::parse_where_x_greatest_commander_mana_value;
#[path = "value_semantics/value_semantics_counter_programs.rs"]
mod value_semantics_counter_programs;
pub use value_semantics_counter_programs::parse_equal_to_number_of_counters_on_reference_value;
#[path = "value_semantics/value_semantics_library_programs.rs"]
mod value_semantics_library_programs;
use value_semantics_library_programs::parse_cards_discarded_this_turn_count_value;
pub use value_semantics_library_programs::parse_players_with_cards_in_hand_at_least;
