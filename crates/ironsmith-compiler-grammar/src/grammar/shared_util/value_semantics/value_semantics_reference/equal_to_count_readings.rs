//! The readings of one "equal to the number of ..." counted phrase: the typed
//! count values (turn history, cards in hand, party size, distinct names, a
//! relative controller clause, ...) read before the counted phrase is an
//! object filter. Formerly a first-match ladder in `value_semantics_reference`;
//! every reading runs, resolved by rank while the overlaps are measured.

use super::*;
use crate::recognition::{ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct CountedPhrase<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) value_tokens: &'a [OwnedLexToken],
    pub(super) filter_tokens: &'a [OwnedLexToken],
    pub(super) filter_word_view: &'a TokenWordView<'a>,
    pub(super) filter_words: &'a [&'a str],
    pub(super) possessive_filter_words: &'a [&'a str],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl CountedPhrase<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
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
    admits: fn(&CountedPhrase<'_>) -> bool,
    read: fn(&CountedPhrase<'_>) -> ParseOutcome<Value>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("equal-to-count-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("coordinated-player-antecedent"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_coordinated_player_antecedent(input)),
    },
    Reading {
        id: RuleId::new("relative-controller-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("coordinated-player-antecedent")
        },
        read: |input| input.outcome(read_relative_controller_clause(input)),
    },
    Reading {
        id: RuleId::new("turn-history-count-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_turn_history_count_value(input)),
    },
    Reading {
        id: RuleId::new("creatures-died-this-turn-count-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_creatures_died_this_turn_count_value(input)),
    },
    Reading {
        id: RuleId::new("cards-discarded-this-turn-count-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cards_discarded_this_turn_count_value(input)),
    },
    Reading {
        id: RuleId::new("players-with-cards-in-hand-at-least"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_players_with_cards_in_hand_at_least(input)),
    },
    Reading {
        id: RuleId::new("cards-in-hand-player"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("players-with-cards-in-hand-at-least")
        },
        read: |input| input.outcome(read_cards_in_hand_player(input)),
    },
    Reading {
        id: RuleId::new("spells-cast-this-turn-matching-count-value"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("turn-history-count-value")
        },
        read: |input| input.outcome(read_spells_cast_this_turn_matching_count_value(input)),
    },
    Reading {
        id: RuleId::new("party-size-player"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_party_size_player(input)),
    },
    Reading {
        id: RuleId::new("aggregate-scope-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_aggregate_scope_value(input)),
    },
    Reading {
        id: RuleId::new("pending-prior-effect-metric"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_pending_prior_effect_metric(input)),
    },
    Reading {
        id: RuleId::new("differently-named"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_differently_named(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &CountedPhrase<'_>) -> ParseOutcome<RuleMatch<Value>> {
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

fn read_coordinated_player_antecedent(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_tokens = input.filter_tokens;
    let possessive_filter_words = input.possessive_filter_words;
    // This is a coordinated player antecedent, not the narrower
    // `that OBJECT's controller` relation below. The ordinary typed
    // object-filter grammar already owns the complete suffix and maps it
    // to TargetPlayerOrControllerOfTarget; let it retain both arms.
    if crate::word_primitives::parse_sequence_suffix(
        &possessive_filter_words,
        &[
            "that",
            "opponent",
            "or",
            "that",
            "planeswalkers",
            "controller",
            "controls",
        ],
    ) {
        let filter =
            crate::grammar::primitives::probe_shape(parse_object_filter(&filter_tokens, false))?;
        return Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
fn read_relative_controller_clause(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_tokens = input.filter_tokens;
    let filter_word_view = input.filter_word_view;
    let filter_words = input.filter_words;
    // A relative controller clause scopes the counted set to the object
    // targeted by this same effect. Parse the set independently from the
    // back-reference so characteristic words in `that creature's controller`
    // cannot leak into the counted filter as an additional Creature type.
    if let Some(that_idx) =
        crate::word_primitives::parse_last_sequence_start(&filter_words, &["that"])
    {
        let relative = possessive_normalized_word_refs(&filter_words[that_idx..]);
        let relative_noun = relative.get(1).map(|word| word.trim_end_matches('s'));
        if relative.len() == 4
            && relative[0] == "that"
            && matches!(
                relative_noun,
                Some("creature" | "permanent" | "object" | "planeswalker")
            )
            && relative[2] == "controller"
            && relative[3] == "controls"
            && that_idx > 0
        {
            let base_range = filter_word_view.token_span_for_words(0, that_idx)?;
            let mut filter = crate::grammar::primitives::probe_shape(parse_object_filter(
                &trim_edge_punctuation(&filter_tokens[base_range]),
                false,
            ))?;
            filter.controller = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
            return Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::EqualTo));
        }
    }
    None
}
fn read_turn_history_count_value(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_tokens = input.filter_tokens;
    if let Some(value) = parse_turn_history_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
fn read_creatures_died_this_turn_count_value(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_tokens = input.filter_tokens;
    if let Some(value) = parse_creatures_died_this_turn_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
fn read_cards_discarded_this_turn_count_value(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_tokens = input.filter_tokens;
    if let Some(value) = parse_cards_discarded_this_turn_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
fn read_players_with_cards_in_hand_at_least(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_tokens = input.filter_tokens;
    if let Some((players, minimum)) = parse_players_with_cards_in_hand_at_least(&filter_tokens) {
        return Some(
            Value::CountPlayersWithCardsInHandAtLeast(players, minimum)
                .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }
    None
}
fn read_cards_in_hand_player(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_words = input.filter_words;
    if let Some(player) = value_helper_shapes::parse_cards_in_hand_player(&filter_words) {
        let mut value = Value::CardsInHand(player).with_surface_hint(ValueSurfaceHint::EqualTo);
        if value_helper_shapes::has_that_player_possessive(&filter_words) {
            value = value.with_surface_hint(ValueSurfaceHint::ThatPlayerPossessive);
        }
        return Some(value);
    }
    None
}
fn read_spells_cast_this_turn_matching_count_value(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_tokens = input.filter_tokens;
    if let Some(value) = parse_spells_cast_this_turn_matching_count_value(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
fn read_party_size_player(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_words = input.filter_words;
    if let Some(player) = value_helper_shapes::parse_party_size_player(&filter_words) {
        return Some(Value::PartySize(player).with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
fn read_aggregate_scope_value(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_tokens = input.filter_tokens;
    if let Some(value) = parse_aggregate_scope_value_lexed(&filter_tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
fn read_pending_prior_effect_metric(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_words = input.filter_words;
    let mut for_each_words = vec!["for", "each"];
    for_each_words.extend(filter_words.iter().copied());
    if let Some((value @ Value::PendingPriorEffectMetric(_), used)) =
        super::super::super::count_shapes::parse_for_each_count_value_words(&for_each_words)
        && used == for_each_words.len()
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
fn read_differently_named(input: &CountedPhrase<'_>) -> Option<Value> {
    let filter_tokens = input.filter_tokens;
    let filter_word_view = input.filter_word_view;
    let filter_words = input.filter_words;
    if let Some(distinct_filter_tokens) =
        primitives::parse_word_sequence_prefix(&filter_words, &["differently", "named"]).and_then(
            |remaining| {
                let consumed = filter_words.len().saturating_sub(remaining.len());
                filter_word_view
                    .token_span_for_words(consumed, filter_word_view.len())
                    .map(|range| &filter_tokens[range])
            },
        )
    {
        let filter = crate::grammar::primitives::probe_shape(parse_object_filter(
            distinct_filter_tokens,
            false,
        ))?;
        return Some(Value::DistinctNames(filter).with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
pub(super) fn read_value_expression(input: &CountedPhrase<'_>) -> Option<Value> {
    let value_tokens = input.value_tokens;
    if let Some((value, used)) = value_expr::parse_value_expr_tokens(&value_tokens)
        && TokenWordView::new(&value_tokens[used..]).is_empty()
    {
        return Some(value.with_surface_hint(ValueSurfaceHint::EqualTo));
    }
    None
}
