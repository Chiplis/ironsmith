//! The readings of the counted filter of one "where X is the number of ..."
//! clause: the typed counts (party size, players with cards in hand, counters
//! on the source, abilities and types among a scope, the special participant
//! filters, ...) read before the filter is an object count. Formerly a
//! first-match ladder in `etb_static_lines`; every reading runs; two different
//! readings of one input are an ambiguity error.

use super::*;
use crate::recognition::{ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct CountedFilter<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) filter_words: &'a [&'a str],
    pub(super) multiplier: i32,
}

impl CountedFilter<'_> {
    /// A reading's outcome.
    fn outcome(&self, read: Option<Value>) -> ParseOutcome<Value> {
        match read {
            Some(value) => ParseOutcome::matched(value, crate::util::span_from_tokens(self.tokens)),
            None => ParseOutcome::NoMatch,
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&CountedFilter<'_>) -> bool,
    read: fn(&CountedFilter<'_>) -> ParseOutcome<Value>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("where-x-count-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("party-size-player"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_party_size_player(input)),
    },
    Reading {
        id: RuleId::new("players-with-cards-in-hand-at-least"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_players_with_cards_in_hand_at_least(input)),
    },
    Reading {
        id: RuleId::new("number-of-counters-on-source-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_number_of_counters_on_source_value(input)),
    },
    Reading {
        id: RuleId::new("static-abilities-among-scope-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_static_abilities_among_scope_value(input)),
    },
    Reading {
        id: RuleId::new("among-types-scope-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_among_types_scope_value(input)),
    },
    Reading {
        id: RuleId::new("card-types-among-scope"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_card_types_among_scope(input)),
    },
    Reading {
        id: RuleId::new("aggregate-scope-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_aggregate_scope_value(input)),
    },
    Reading {
        id: RuleId::new("shared-domain-relative-selector-filter"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_shared_domain_relative_selector_filter(input)),
    },
    Reading {
        id: RuleId::new("special-number-filter"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_special_number_filter(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &CountedFilter<'_>) -> ParseOutcome<RuleMatch<Value>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<Value>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_party_size_player(input: &CountedFilter<'_>) -> Option<Value> {
    let filter_words = input.filter_words;
    let multiplier = input.multiplier;
    if let Some(player) =
        crate::grammar::shared_util::value_helper_shapes::parse_party_size_player(&filter_words)
    {
        return Some(scale_where_x_number_value(
            Value::PartySize(player),
            multiplier,
        ));
    }
    None
}
fn read_players_with_cards_in_hand_at_least(input: &CountedFilter<'_>) -> Option<Value> {
    let multiplier = input.multiplier;
    let filter_tokens = input.tokens;
    if let Some((players, minimum)) =
        crate::grammar::shared_util::value_semantics::parse_players_with_cards_in_hand_at_least(
            filter_tokens,
        )
    {
        return Some(scale_where_x_number_value(
            Value::CountPlayersWithCardsInHandAtLeast(players, minimum),
            multiplier,
        ));
    }
    None
}
fn read_number_of_counters_on_source_value(input: &CountedFilter<'_>) -> Option<Value> {
    let filter_tokens = input.tokens;
    if let Some(value) = etb_grammar::parse_number_of_counters_on_source_value_tokens(filter_tokens)
    {
        return Some(value);
    }
    None
}
fn read_static_abilities_among_scope_value(input: &CountedFilter<'_>) -> Option<Value> {
    let multiplier = input.multiplier;
    let filter_tokens = input.tokens;
    if let Some(value) = parse_static_abilities_among_scope_value(filter_tokens) {
        return Some(scale_where_x_number_value(value, multiplier));
    }
    None
}
fn read_among_types_scope_value(input: &CountedFilter<'_>) -> Option<Value> {
    let multiplier = input.multiplier;
    let filter_tokens = input.tokens;
    if let Some(value) = parse_among_types_scope_value(filter_tokens) {
        return Some(scale_where_x_number_value(value, multiplier));
    }
    None
}
fn read_card_types_among_scope(input: &CountedFilter<'_>) -> Option<Value> {
    let multiplier = input.multiplier;
    let filter_tokens = input.tokens;
    if let Some(among) = etb_grammar::parse_etb_among_scope_tokens(filter_tokens) {
        let card_types = match among.metric {
            EtbAmongMetric::CardTypesAmongCards
                if etb_grammar::etb_tokens_have_graveyard_marker(among.scope_tokens) =>
            {
                let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
                filter.owner =
                    match etb_grammar::parse_etb_graveyard_owner_tokens(among.scope_tokens) {
                        Some(EtbGraveyardOwner::Opponent) => Some(PlayerFilter::Opponent),
                        Some(EtbGraveyardOwner::You) => Some(PlayerFilter::You),
                        None => None,
                    };
                Some(Value::CardTypesAmong(filter))
            }
            EtbAmongMetric::CardTypesAmong => {
                let filter = crate::grammar::primitives::probe_shape(parse_object_filter_lexed(
                    among.scope_tokens,
                    false,
                ))?;
                Some(Value::CardTypesAmong(filter))
            }
            _ => None,
        };
        if let Some(card_types) = card_types {
            return Some(scale_where_x_number_value(card_types, multiplier));
        }
    }
    None
}
fn read_aggregate_scope_value(input: &CountedFilter<'_>) -> Option<Value> {
    let multiplier = input.multiplier;
    let filter_tokens = input.tokens;
    if let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) {
        return Some(scale_where_x_number_value(value, multiplier));
    }
    None
}
fn read_shared_domain_relative_selector_filter(input: &CountedFilter<'_>) -> Option<Value> {
    let multiplier = input.multiplier;
    let filter_tokens = input.tokens;
    // Normalize a selector list with one authored object domain before the
    // broad for-each count grammar can split that domain across union arms.
    if let Some(filter) = parse_shared_domain_relative_selector_filter(filter_tokens) {
        return Some(scale_where_x_number_value(Value::Count(filter), multiplier));
    }
    None
}
fn read_special_number_filter(input: &CountedFilter<'_>) -> Option<Value> {
    let multiplier = input.multiplier;
    let filter_tokens = input.tokens;
    // Preserve semantic participant references before the broad for-each
    // count parser gets a chance to accept only the leading object noun. In
    // particular, `creatures those players control` is a count over the
    // spell's selected player set, not every creature on the battlefield.
    if let Some(kind) = etb_grammar::parse_where_x_special_number_filter_tokens(filter_tokens) {
        let value = match kind {
            etb_grammar::WhereXSpecialNumberFilterKind::CreaturesDiedThisTurn => {
                Value::CreaturesDiedThisTurn
            }
            etb_grammar::WhereXSpecialNumberFilterKind::CommanderCastCount => {
                Value::CommanderCastCount(PlayerFilter::You)
            }
            etb_grammar::WhereXSpecialNumberFilterKind::CreaturesControlledByThosePlayers => {
                let mut filter = ObjectFilter::creature();
                filter.controller = Some(PlayerFilter::target_player());
                Value::Count(filter)
            }
        };
        return Some(scale_where_x_number_value(value, multiplier));
    }
    None
}
