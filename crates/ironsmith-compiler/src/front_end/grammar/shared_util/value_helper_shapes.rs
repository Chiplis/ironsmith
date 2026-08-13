use ironsmith_core::{EffectMetricSource, PriorEffectAction};

use crate::effect::Value;
use crate::object::CounterType;
use crate::grammar::filters::parse_counter_type_words;
use crate::grammar::permission_shapes;
use crate::object_filters::parse_object_filter_words;
use crate::util::{
    is_article, source_reference_surface_for_words, this_source_surface_for_words,
};
use crate::target::{ChooseSpec, PlayerFilter, SourceReferenceSurface};

use super::value_shapes::{self, AggregateValueMetric};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpellCastThisTurnSurface {
    pub(crate) filter_end: usize,
    pub(crate) player: PlayerFilter,
    pub(crate) exclude_source: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NumberOfPrefix {
    pub(crate) number_of_start: usize,
    pub(crate) consumed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CounterValueReference {
    Source(Option<SourceReferenceSurface>),
    Tagged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CounterReferenceValueShape {
    pub(crate) counter_type: Option<CounterType>,
    pub(crate) reference: CounterValueReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AggregateKind {
    Total,
    Greatest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AggregateValueKind {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AggregatePrefix {
    pub(crate) aggregate: AggregateKind,
    pub(crate) value_kind: AggregateValueKind,
    pub(crate) consumed: usize,
}

const SPELL_CAST_SUFFIXES: &[(&[&str], PlayerFilter)] = &[
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
#[path = "value_helper_shapes/reference_values.rs"]
mod reference_values;
pub(crate) use reference_values::*;

pub(crate) fn parse_spell_cast_this_turn_surface(
    words: &[&str],
) -> Option<SpellCastThisTurnSurface> {
    if !has_any(words, &["spell", "spells"])
        || !has_any(words, &["cast", "casts"])
        || !has_word(words, "this")
        || !has_word(words, "turn")
    {
        return None;
    }
    for (suffix, player) in SPELL_CAST_SUFFIXES {
        if permission_shapes::suffix_words(words, suffix) {
            let filter_end = words.len().checked_sub(suffix.len())?;
            if filter_end == 0 {
                return None;
            }
            return Some(SpellCastThisTurnSurface {
                filter_end,
                player: player.clone(),
                exclude_source: has_word(&words[..filter_end], "other"),
            });
        }
    }
    None
}

pub(crate) fn parse_spells_cast_this_turn_value_words(words: &[&str]) -> Option<Value> {
    let surface = parse_spell_cast_this_turn_surface(words)?;
    let filter = parse_object_filter_words(&words[..surface.filter_end], false).ok()?;
    Some(Value::SpellsCastThisTurnMatching {
        player: surface.player,
        filter,
        exclude_source: surface.exclude_source,
    })
}

pub(crate) fn parse_aggregate_scope_value_words(words: &[&str]) -> Option<Value> {
    let surface = value_shapes::parse_aggregate_value_surface(words)?;
    let filter = parse_object_filter_words(surface.scope_words, false).ok()?;
    match surface.metric {
        AggregateValueMetric::BasicLandTypes => Some(Value::BasicLandTypesAmong(filter)),
        AggregateValueMetric::CreatureTypes => Some(Value::CreatureTypesAmong(filter)),
        AggregateValueMetric::Colors => Some(Value::ColorsAmong(filter)),
        AggregateValueMetric::ColorPairs => Some(Value::ColorPairsAmong(filter)),
        AggregateValueMetric::DistinctNames => Some(Value::DistinctNames(filter)),
        AggregateValueMetric::DistinctPowers => Some(Value::DistinctPowers(filter)),
        AggregateValueMetric::Counters => Some(
            Value::CountersOn(Box::new(ChooseSpec::All(filter)), None)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersAmong),
        ),
    }
}

pub(crate) fn parse_prior_effect_metric_source(words: &[&str]) -> Option<EffectMetricSource> {
    let references_prior_effect = permission_shapes::find_words(words, &["this", "way"]).is_some()
        || words.iter().enumerate().any(|(index, word)| {
            matches!(
                *word,
                "chosen"
                    | "destroyed"
                    | "discarded"
                    | "exiled"
                    | "milled"
                    | "revealed"
                    | "sacrificed"
                    | "searched"
            ) && !(*word == "chosen"
                && words
                    .get(index + 1)
                    .is_some_and(|next| matches!(*next, "type" | "color")))
        });
    if !references_prior_effect {
        return None;
    }
    Some(if has_word(words, "chosen") {
        EffectMetricSource::ChosenObjects
    } else {
        EffectMetricSource::AffectedObjects
    })
}

/// Parse the passive action at the end of a `... this way` object phrase and
/// return the word index where that action starts.
///
/// Keeping the action typed lets reference resolution bind numeric queries to
/// an exact producer while rendering preserves the authored relationship
/// without inferring semantics from a generated tag name.
pub(crate) fn parse_prior_effect_action(words: &[&str]) -> Option<(PriorEffectAction, usize)> {
    const PATTERNS: &[(&[&str], PriorEffectAction)] = &[
        (&["phased", "out"], PriorEffectAction::PhasedOut),
        (
            &["put", "onto", "the", "battlefield"],
            PriorEffectAction::PutOntoBattlefield,
        ),
        (
            &["put", "onto", "battlefield"],
            PriorEffectAction::PutOntoBattlefield,
        ),
        (&["put", "into", "exile"], PriorEffectAction::Exiled),
        (&["dealt", "damage"], PriorEffectAction::DealtDamage),
        (
            &["counters", "put", "on", "it"],
            PriorEffectAction::CountersPut,
        ),
        (
            &["counter", "put", "on", "it"],
            PriorEffectAction::CountersPut,
        ),
        (
            &["counters", "put", "on", "them"],
            PriorEffectAction::CountersPut,
        ),
        (
            &["counter", "put", "on", "them"],
            PriorEffectAction::CountersPut,
        ),
        (&["counters", "put"], PriorEffectAction::CountersPut),
        (&["counter", "put"], PriorEffectAction::CountersPut),
        (&["searched", "for"], PriorEffectAction::Searched),
        (&["cast"], PriorEffectAction::Cast),
        (&["chosen"], PriorEffectAction::Chosen),
        (&["connived"], PriorEffectAction::Connived),
        (&["countered"], PriorEffectAction::Countered),
        (&["destroyed"], PriorEffectAction::Destroyed),
        (&["discarded"], PriorEffectAction::Discarded),
        (&["drawn"], PriorEffectAction::Drawn),
        (&["exiled"], PriorEffectAction::Exiled),
        (&["goaded"], PriorEffectAction::Goaded),
        (&["milled"], PriorEffectAction::Milled),
        (&["prevented"], PriorEffectAction::Prevented),
        (&["removed"], PriorEffectAction::Removed),
        (
            &["returned", "to", "your", "hand"],
            PriorEffectAction::Returned,
        ),
        (
            &["returned", "to", "their", "hand"],
            PriorEffectAction::Returned,
        ),
        (&["returned"], PriorEffectAction::Returned),
        (&["revealed"], PriorEffectAction::Revealed),
        (&["sacrificed"], PriorEffectAction::Sacrificed),
        (&["searched"], PriorEffectAction::Searched),
        (&["shuffled"], PriorEffectAction::Shuffled),
        (&["tapped"], PriorEffectAction::Tapped),
    ];

    PATTERNS.iter().find_map(|(suffix, action)| {
        if !permission_shapes::suffix_words(words, suffix) {
            return None;
        }
        let mut action_start = words.len().saturating_sub(suffix.len());
        if action_start > 0 && matches!(words[action_start - 1], "is" | "are" | "was" | "were") {
            action_start -= 1;
        }
        if action_start > 0 && words[action_start - 1] == "that" {
            action_start -= 1;
        }
        Some((*action, action_start))
    })
}

pub(crate) fn parse_number_of_prefix(words: &[&str]) -> Option<NumberOfPrefix> {
    let number_of_start = usize::from(permission_shapes::prefix_words(words, &["the"]));
    permission_shapes::starts_at_words(words, number_of_start, &["number", "of"]).then_some(
        NumberOfPrefix {
            number_of_start,
            consumed: number_of_start + 2,
        },
    )
}

pub(crate) fn parse_aggregate_prefix(words: &[&str]) -> Option<AggregatePrefix> {
    let mut index = usize::from(permission_shapes::prefix_words(words, &["the"]));
    let aggregate = match words.get(index).copied()? {
        "total" => AggregateKind::Total,
        "greatest" => AggregateKind::Greatest,
        _ => return None,
    };
    index += 1;
    let value_kind = if permission_shapes::starts_at_words(words, index, &["mana", "value"]) {
        index += 2;
        AggregateValueKind::ManaValue
    } else {
        let kind = match words.get(index).copied()? {
            "power" => AggregateValueKind::Power,
            "toughness" => AggregateValueKind::Toughness,
            _ => return None,
        };
        index += 1;
        kind
    };
    if words
        .get(index)
        .is_none_or(|word| !matches!(*word, "of" | "among"))
    {
        return None;
    }
    Some(AggregatePrefix {
        aggregate,
        value_kind,
        consumed: index + 1,
    })
}

pub(crate) fn starts_equal_to_opponents_you_have(words: &[&str]) -> bool {
    permission_shapes::prefix_words(
        words,
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
    ) || permission_shapes::prefix_words(
        words,
        &["equal", "to", "number", "of", "opponents", "you", "have"],
    )
}

pub(crate) fn starts_or_power_toughness(words: &[&str]) -> bool {
    permission_shapes::prefix_words(words, &["or", "power"])
        || permission_shapes::prefix_words(words, &["or", "toughness"])
}

fn has_word(words: &[&str], expected: &str) -> bool {
    permission_shapes::find_words(words, &[expected]).is_some()
}

fn has_any(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().any(|word| has_word(words, word))
}

#[cfg(test)]
#[path = "value_helper_shapes/tests.rs"]
mod tests;
