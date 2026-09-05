//! The readings of one "exile ..." clause: same-name hand-and-graveyard
//! exiles, a card from hand or a permanent, one per card type from a
//! graveyard, battlefield-and-graveyard pairs, "all/each" filters, a target
//! player's graveyard, damage-dealt histories, attached-object bundles,
//! same-name token bundles, ... Formerly a first-match ladder in
//! `exile_actions`; every reading runs, resolved by rank while the overlaps
//! are measured. The target-phrase exile is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct ExileClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) subject: Option<SubjectAst>,
    pub(super) until_source_leaves: bool,
    pub(super) face_down: bool,
    pub(super) clause_words: &'a [&'a str],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl ExileClause<'_> {
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
                RuleId::new("exile-clause-registry-reading"),
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
    admits: fn(&ExileClause<'_>) -> bool,
    read: fn(&ExileClause<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("exile-clause-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("same-name-exile-hand-and-graveyard"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_same_name_exile_hand_and_graveyard(input)),
    },
    Reading {
        id: RuleId::new("exile-card-from-their-hand-or-permanent-they-control"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_exile_card_from_their_hand_or_permanent_they_control(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("exile-one-per-card-type-from-graveyard"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_one_per_card_type_from_graveyard(input)),
    },
    Reading {
        id: RuleId::new("battlefield-graveyard-exile-all-pair"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_battlefield_graveyard_exile_all_pair(input)),
    },
    Reading {
        id: RuleId::new("exile-all-or-each-filter"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("battlefield-graveyard-exile-all-pair")
        },
        read: |input| input.outcome(read_exile_all_or_each_filter(input)),
    },
    Reading {
        id: RuleId::new("target-player-graveyard-filter"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_target_player_graveyard_filter(input)),
    },
    Reading {
        id: RuleId::new("mixed-target-and-all-exile-list"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("exile-all-or-each-filter")
        },
        read: |input| input.outcome(read_mixed_target_and_all_exile_list(input)),
    },
    Reading {
        id: RuleId::new("exile-bottom-library"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_bottom_library(input)),
    },
    Reading {
        id: RuleId::new("exile-dynamic-count-from-top-library"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_dynamic_count_from_top_library(input)),
    },
    Reading {
        id: RuleId::new("exile-top-library"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_exile_top_library(input)),
    },
    Reading {
        id: RuleId::new("dealt-damage-history"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_dealt_damage_history(input)),
    },
    Reading {
        id: RuleId::new("attached-object-exile-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_attached_object_exile_bundle(input)),
    },
    Reading {
        id: RuleId::new("same-name-token-bundle"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_same_name_token_bundle(input)),
    },
    Reading {
        id: RuleId::new("source-and-target-exile-pair"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_and_target_exile_pair(input)),
    },
    Reading {
        id: RuleId::new("independent-exile-pair"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_independent_exile_pair(input)),
    },
    Reading {
        id: RuleId::new("and-split-exile-pair"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_and_split_exile_pair(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &ExileClause<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
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

fn read_same_name_exile_hand_and_graveyard(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if let Some(effect) = parse_same_name_exile_hand_and_graveyard_clause(
        tokens,
        subject,
        until_source_leaves,
        face_down,
    )? {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_exile_card_from_their_hand_or_permanent_they_control(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if !face_down
        && !until_source_leaves
        && let Some(effect) =
            parse_exile_card_from_their_hand_or_permanent_they_control(tokens, subject)
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_exile_one_per_card_type_from_graveyard(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if let Some(shape) = effect_grammar::parse_exile_one_per_card_type_from_graveyard_shape(tokens)
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = controller_filter_for_token_player(shape.owner);
        filter.one_per_card_type = true;
        let target = TargetAst::WithCount(
            Box::new(TargetAst::Object(filter, None, None)),
            crate::effect::ChoiceCount::any_number(),
        );
        return Ok(Some(if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
        } else {
            EffectAst::subject_verb_exile(target, face_down)
        }));
    }
    Ok(None)
}
fn read_battlefield_graveyard_exile_all_pair(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if let Some(effect) =
        parse_battlefield_graveyard_exile_all_pair(tokens, subject, until_source_leaves, face_down)?
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_exile_all_or_each_filter(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    // A repeated `all` collection may give each arm its own domain, for
    // example battlefield permanents followed by every card in hands and
    // graveyards.  The ordinary shared-terminal path can otherwise lift
    // the first arm's card types onto the outer filter and incorrectly
    // constrain the card-only arms.  Prefer a proven branch-scoped union
    // before the general filter parser for this exhaustive shape.
    if let Some(filter_tokens) = effect_grammar::strip_exile_all_or_each_shape(tokens) {
        if let Some(effect) = parse_except_then_additional_exile_all_filter(
            filter_tokens,
            subject,
            until_source_leaves,
            face_down,
        )? {
            return Ok(Some(effect));
        }
        let scoped_union = crate::grammar::filters::parse_branch_scoped_object_filter_union_lexed(
            filter_tokens,
            false,
        )
        .or_else(|| {
            crate::grammar::filters::parse_domain_union_object_filter_lexed(filter_tokens, false)
        });
        let mut filter = match scoped_union {
            Some(filter) => filter,
            None => parse_object_filter_lexed(filter_tokens, false)?,
        };
        filter = scope_types_away_from_requantified_bare_card_domains(filter_tokens, filter);
        apply_exile_subject_owner_context(&mut filter, subject);
        return Ok(Some(if until_source_leaves {
            EffectAst::subject_verb_exile_all_until_source_leaves(
                TargetAst::Object(filter, None, None),
                face_down,
            )
        } else {
            EffectAst::subject_verb_exile_all(filter, face_down)
        }));
    }
    Ok(None)
}
fn read_target_player_graveyard_filter(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if let Some(filter) = parse_target_player_graveyard_filter(tokens) {
        return Ok(Some(if until_source_leaves {
            EffectAst::subject_verb_exile_until_source_leaves(
                TargetAst::Object(filter, None, None),
                face_down,
            )
        } else {
            EffectAst::subject_verb_exile_all(filter, face_down)
        }));
    }
    Ok(None)
}
fn read_mixed_target_and_all_exile_list(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if let Some(effect) =
        parse_mixed_target_and_all_exile_list(tokens, subject, until_source_leaves, face_down)?
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_exile_bottom_library(input: &ExileClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if !until_source_leaves
        && let Some(effect) = parse_exile_bottom_library_clause(tokens, subject, face_down)
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_exile_dynamic_count_from_top_library(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if !until_source_leaves
        && let Some(effect) =
            parse_exile_dynamic_count_from_top_library_clause(tokens, subject, face_down)
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_exile_top_library(input: &ExileClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if !until_source_leaves
        && let Some(effect) = parse_exile_top_library_clause(tokens, subject, face_down)
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_dealt_damage_history(input: &ExileClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let clause_words = input.clause_words;
    if grammar::contains_word(tokens, "dealt")
        && grammar::contains_word(tokens, "damage")
        && grammar::contains_word(tokens, "turn")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history exile clause (clause: '{}')",
            clause_words.join(" ")
        )))
        .map(Some);
    }
    Ok(None)
}
fn read_attached_object_exile_bundle(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let face_down = input.face_down;
    if let Some(effect) = parse_attached_object_exile_bundle(tokens, face_down)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_same_name_token_bundle(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let clause_words = input.clause_words;
    let has_same_name_token_bundle = grammar::contains_word(tokens, "and")
        && grammar::contains_word(tokens, "tokens")
        && grammar::contains_word(tokens, "same")
        && grammar::contains_word(tokens, "name");
    if has_same_name_token_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported same-name token exile bundle (clause: '{}')",
            clause_words.join(" ")
        )))
        .map(Some);
    }
    Ok(None)
}
fn read_source_and_target_exile_pair(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if let Some(effect) =
        parse_source_and_target_exile_pair(tokens, subject.clone(), until_source_leaves, face_down)?
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_independent_exile_pair(
    input: &ExileClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let subject = input.subject;
    let until_source_leaves = input.until_source_leaves;
    let face_down = input.face_down;
    if let Some(effect) =
        parse_independent_exile_pair(tokens, subject, until_source_leaves, face_down)?
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_and_split_exile_pair(input: &ExileClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let clause_words = input.clause_words;
    if let Some((before_and, after_and)) =
        crate::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            crate::grammar::primitives::kw("and").void()
        })
        && !before_and.is_empty()
    {
        let starts_multi_target = effect_grammar::starts_exile_multi_target_shape(after_and);
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target exile clause (clause: '{}')",
                clause_words.join(" ")
            )))
            .map(Some);
        }
    }
    Ok(None)
}
