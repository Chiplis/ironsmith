//! The readings of one "put ..." clause: revealed remainders, reordered
//! tagged cards, battlefield partitions, "from among them", all exiled cards
//! into hand, tagged cards on top or into hand, destination-first battlefield
//! wording, library choices and placements, "into" and "onto" destinations.
//! Formerly a first-match ladder in `control_copy_attach_verbs_zone`; every
//! reading runs, resolved by rank while the overlaps are measured. The
//! sticker and unsupported-put diagnoses follow in the wrapper.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

#[path = "put_clause_readings/part_1.rs"]
mod part_1;
#[path = "put_clause_readings/part_2.rs"]
mod part_2;

/// The input the readings read.
pub(super) struct PutClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) player: PlayerAst,
    pub(super) subject: Option<SubjectAst>,
    pub(super) clause_words: &'a [&'a str],
    pub(super) exiled_with_source_surface: &'a Option<ironsmith_core::ExiledWithSourceMoveSurface>,
    pub(super) authored_tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl PutClause<'_> {
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
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("put-clause-registry-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&PutClause<'_>) -> bool,
    read: fn(&PutClause<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("put-clause-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("revealed-remainder"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_revealed_remainder(input)),
    },
    Reading {
        id: RuleId::new("reorder-tagged-cards"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_reorder_tagged_cards(input)),
    },
    Reading {
        id: RuleId::new("tagged-battlefield-partition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_tagged_battlefield_partition(input)),
    },
    Reading {
        id: RuleId::new("from-among-them"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_from_among_them(input)),
    },
    Reading {
        id: RuleId::new("from-among-hand-surface"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("from-among-them")
        },
        read: |input| input.outcome(part_1::read_from_among_hand_surface(input)),
    },
    Reading {
        id: RuleId::new("all-exiled-into-hand-filter"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_all_exiled_into_hand_filter(input)),
    },
    Reading {
        id: RuleId::new("tagged-on-top-library"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_1::read_tagged_on_top_library(input)),
    },
    Reading {
        id: RuleId::new("tagged-into-hand"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_tagged_into_hand(input)),
    },
    Reading {
        id: RuleId::new("destination-first-battlefield"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_destination_first_battlefield(input)),
    },
    Reading {
        id: RuleId::new("library-choice-destination"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(part_2::read_library_choice_destination(input)),
    },
    Reading {
        id: RuleId::new("library-placement-destination"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("from-among-hand-surface")
                && !input.read_by("from-among-them")
                && !input.read_by("reorder-tagged-cards")
                && !input.read_by("revealed-remainder")
                && !input.read_by("tagged-into-hand")
                && !input.read_by("tagged-on-top-library")
        },
        read: |input| input.outcome(part_2::read_library_placement_destination(input)),
    },
    Reading {
        id: RuleId::new("into-destination"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("from-among-hand-surface")
                && !input.read_by("library-choice-destination")
                && !input.read_by("library-placement-destination")
                && !input.read_by("reorder-tagged-cards")
                && !input.read_by("revealed-remainder")
                && !input.read_by("tagged-into-hand")
        },
        read: |input| input.outcome(part_2::read_into_destination(input)),
    },
    Reading {
        id: RuleId::new("onto-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("from-among-them")
                && !input.read_by("into-destination")
                && !input.read_by("library-placement-destination")
        },
        read: |input| input.outcome(part_2::read_onto_clause(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &PutClause<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
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
    let mut distinct: Vec<RegistryCandidate<EffectAst>> = Vec::new();
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
    let outcome = resolve_ranked_candidates(REGISTRY, distinct, diagnostics, || {
        crate::lexer::parser_token_word_refs(input.tokens).join(" ")
    });
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn parse_put_into_hand_delayed_timing(tokens: &[OwnedLexToken]) -> Option<DelayedReturnTimingAst> {
    let tail_tokens = cca_shapes::parse_delayed_hand_tail(tokens)?;
    let tail_words = crate::lexer::token_word_refs(tail_tokens);
    parse_delayed_return_timing_words(&tail_words)
}
fn force_object_targeting(target: TargetAst, span: TextSpan) -> TargetAst {
    match target {
        TargetAst::Object(filter, explicit_span, fixed_span) => {
            TargetAst::Object(filter, explicit_span.or(Some(span)), fixed_span)
        }
        TargetAst::WithCount(inner, count) => {
            TargetAst::WithCount(Box::new(force_object_targeting(*inner, span)), count)
        }
        other => other,
    }
}
fn expand_graveyard_or_hand_disjunction(
    mut target: TargetAst,
    target_tokens: &[OwnedLexToken],
) -> TargetAst {
    if !cca_shapes::contains_graveyard_and_hand(target_tokens) {
        return target;
    }

    // Parse the characteristic prefix independently from the zone
    // disjunction.  Otherwise the generic filter parser can put the
    // Aura/Equipment (or other type) union inside `any_of`, and clearing
    // that union while expanding the two zones silently drops it.
    if let Some(from_index) = crate::slice_primitives::select_position(target_tokens, |token| {
        token
            .as_word()
            .is_some_and(|word| word.eq_ignore_ascii_case("from"))
    }) && from_index > 0
        && let Ok(base) = parse_target_phrase(&target_tokens[..from_index])
    {
        target = base;
    }

    let target_words = crate::lexer::token_word_refs(target_tokens);
    let owner = crate::slice_primitives::find_window_by(&target_words, 2, |pair| {
        pair[0].eq_ignore_ascii_case("your")
            && (pair[1].eq_ignore_ascii_case("hand") || pair[1].eq_ignore_ascii_case("graveyard"))
    })
    .is_some();

    fn apply(filter: &ObjectFilter, owner: bool) -> ObjectFilter {
        let mut hand = filter.clone();
        hand.any_of.clear();
        hand.zone = Some(Zone::Hand);
        if owner {
            hand.owner = Some(PlayerFilter::You);
        }

        let mut graveyard = filter.clone();
        graveyard.any_of.clear();
        graveyard.zone = Some(Zone::Graveyard);
        if owner {
            graveyard.owner = Some(PlayerFilter::You);
        }

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![hand, graveyard];
        disjunction
    }

    match &mut target {
        TargetAst::Object(filter, _, _) => {
            *filter = apply(filter, owner);
        }
        TargetAst::WithCount(inner, _) => {
            if let TargetAst::Object(filter, _, _) = inner.as_mut() {
                *filter = apply(filter, owner);
            }
        }
        _ => {}
    }

    target
}
fn apply_source_zone_constraint(target: &mut TargetAst, zone: Zone) {
    match target {
        TargetAst::Source(span) => {
            *target = TargetAst::Object(ObjectFilter::source().in_zone(zone), *span, None);
        }
        TargetAst::Object(filter, _, _) => {
            filter.zone = Some(zone);
        }
        TargetAst::WithCount(inner, _) => apply_source_zone_constraint(inner, zone),
        _ => {}
    }
}
fn apply_explicit_source_location(target: &mut TargetAst, tokens: &[OwnedLexToken]) {
    let words = crate::lexer::token_word_refs(tokens);
    let location = if crate::word_primitives::sequence_occurs(&words, &["from", "your", "hand"]) {
        Some((Zone::Hand, Some(PlayerFilter::You)))
    } else if crate::word_primitives::sequence_occurs(&words, &["from", "your", "graveyard"]) {
        Some((Zone::Graveyard, Some(PlayerFilter::You)))
    } else if crate::word_primitives::sequence_occurs(&words, &["from", "your", "library"]) {
        Some((Zone::Library, Some(PlayerFilter::You)))
    } else if crate::word_primitives::sequence_occurs(&words, &["from", "the", "command", "zone"]) {
        Some((Zone::Command, Some(PlayerFilter::You)))
    } else {
        None
    };
    let Some((zone, owner)) = location else {
        return;
    };

    apply_source_zone_constraint(target, zone);
    if let Some(owner) = owner
        && let Some(filter) =
            crate::effect_sentences::zone_counter_helpers::target_object_filter_mut(target)
    {
        filter.owner = Some(owner);
    }
}
fn strip_source_top_only_prefix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], bool) {
    use winnow::Parser as _;

    crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::phrase(&["the", "top"]).void(),
    )
    .map(|(_, rest)| (rest, true))
    .unwrap_or((tokens, false))
}
