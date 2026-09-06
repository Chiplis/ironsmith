//! The readings of one single-sentence static ability line with a condition
//! or duration prefix: the quest-counter draw replacement, "during your end
//! step", "must be blocked if able", conditional enters-with-counters, a
//! leading "if" clause, a fixed prefix condition. Formerly a first-match
//! ladder in `keyword_static`; every reading runs, resolved by rank while the
//! overlaps are measured. The unconditioned line with an "as long as" prefix
//! is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct SingleStaticLine<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl SingleStaticLine<'_> {
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
    fn outcome(
        &self,
        read: Result<Option<Vec<StaticAbilityAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<StaticAbilityAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("single-static-line-registry-reading"),
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
    admits: fn(&SingleStaticLine<'_>) -> bool,
    read: fn(&SingleStaticLine<'_>) -> ParseOutcome<Vec<StaticAbilityAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("single-static-line-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("quest-counter-draw-replacement"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_quest_counter_draw_replacement(input)),
    },
    Reading {
        id: RuleId::new("during-your-end-step"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_during_your_end_step(input)),
    },
    Reading {
        id: RuleId::new("must-be-blocked-if-able"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_must_be_blocked_if_able(input)),
    },
    Reading {
        id: RuleId::new("conditional-enters-with-counters"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conditional_enters_with_counters(input)),
    },
    Reading {
        id: RuleId::new("leading-if-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("conditional-enters-with-counters")
        },
        read: |input| input.outcome(read_leading_if_clause(input)),
    },
    Reading {
        id: RuleId::new("fixed-prefix-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_fixed_prefix_condition(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &SingleStaticLine<'_>) -> ParseOutcome<RuleMatch<Vec<StaticAbilityAst>>> {
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
    let mut distinct: Vec<RegistryCandidate<Vec<StaticAbilityAst>>> = Vec::new();
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

fn read_quest_counter_draw_replacement(
    input: &SingleStaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(tokens),
        &[
            "as",
            "long",
            "as",
            "this",
            "enchantment",
            "has",
            "six",
            "or",
            "more",
            "quest",
            "counters",
            "on",
            "it",
            "if",
            "you",
            "would",
            "draw",
            "a",
            "card",
            "you",
            "may",
            "instead",
            "search",
            "your",
            "library",
            "for",
            "a",
            "card",
            "put",
            "that",
            "card",
            "into",
            "your",
            "hand",
            "then",
            "shuffle",
        ],
    ) {
        let mut card = ObjectFilter::default();
        card.set_explicit_card_noun(true);
        let ability = StaticAbility::conditional_draw_replacement_with_optional(
            PredicateAst::ValueComparison {
                left: Value::Fixed(1),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(1),
            },
            vec![EffectAst::subject_verb_search_library(
                card,
                Zone::Hand,
                PlayerAst::You,
                PlayerAst::You,
                crate::effect::SearchSelectionMode::Exact,
                false,
                None,
                true,
                ChoiceCount::exactly(1),
                None,
                None,
                crate::effect::SearchResultReferenceSurface::ThatCard,
                false,
                false,
                false,
            )],
            true,
            render_token_slice(tokens),
        );
        return Ok(Some(vec![
            StaticAbilityAst::LabeledConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition: PredicateAst::Source(SourcePredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::Quest,
                    count: 6,
                    surface: crate::SourceCounterThresholdSurface::SourceHas,
                }),
                label: render_token_slice(tokens),
            },
        ]));
    }
    Ok(None)
}
fn read_during_your_end_step(
    input: &SingleStaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some((_, rest)) = crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::phrase(&["during", "your", "end", "step"]),
    ) {
        let remainder = trim_lexed_commas(rest);
        if remainder.len() < rest.len()
            && !remainder.is_empty()
            && let Some(abilities) =
                parse_static_ability_ast_line_lexed_single_without_leading_condition(remainder)?
            && !abilities.is_empty()
        {
            let mut conditioned = Vec::with_capacity(abilities.len());
            for ability in abilities {
                conditioned.push(add_static_ability_ast_condition(
                    ability,
                    PredicateAst::Source(SourcePredicateAst::SourceControllersEndStep),
                )?);
            }
            return Ok(Some(conditioned));
        }
    }
    Ok(None)
}
fn read_must_be_blocked_if_able(
    input: &SingleStaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    let condition_prefix_tokens =
            crate::grammar::effects::labeled_dispatch::parse_leading_effect_label_tokens(tokens)
                .filter(|shape| {
                    shape.kind
                    == crate::grammar::effects::labeled_dispatch::LeadingEffectLabelKind::Conditional
                })
                .map_or(tokens, |shape| shape.body_tokens);
    if let Some(spec) = split_as_long_as_condition_prefix_lexed(condition_prefix_tokens)
        && crate::word_primitives::parse_sequence_complete(
            &crate::lexer::parser_token_word_refs(spec.remainder_tokens),
            &["it", "must", "be", "blocked", "if", "able"],
        )
    {
        let condition_words = crate::lexer::parser_token_word_refs(spec.condition_tokens);
        let condition =
            if crate::word_primitives::parse_sequence_suffix(&condition_words, &["is", "equipped"])
            {
                PredicateAst::Source(SourcePredicateAst::SourceIsEquipped)
            } else if crate::word_primitives::parse_sequence_suffix(
                &condition_words,
                &["is", "enchanted"],
            ) {
                PredicateAst::Source(SourcePredicateAst::SourceIsEnchanted)
            } else {
                parse_static_condition_clause(spec.condition_tokens)?
            };
        if matches!(
            condition,
            PredicateAst::Source(SourcePredicateAst::SourceIsEquipped)
                | PredicateAst::Source(SourcePredicateAst::SourceIsEnchanted)
                | PredicateAst::Source(SourcePredicateAst::SourceIsMonstrous)
        ) {
            return Ok(Some(vec![StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::restriction(
                    crate::effect::Restriction::must_be_blocked(ObjectFilter::source()),
                    "this creature must be blocked if able".to_string(),
                ))),
                condition,
            }]));
        }
    }
    Ok(None)
}
fn read_conditional_enters_with_counters(
    input: &SingleStaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    // Conditional self-entry counters may carry an additional quoted ability.
    // The self-ETB grammar owns both pieces as one replacement payload; give
    // it the intact condition before the generic conditional wrapper strips
    // the prefix and leaves an apparently unconditional ability grant.
    if tokens.first().is_some_and(|token| token.is_word("if"))
        && tokens.iter().any(|token| token.is_word("enters"))
        && let Some(abilities) = parse_enters_with_counters_line(tokens)?
    {
        return Ok(Some(
            abilities.into_iter().map(StaticAbilityAst::from).collect(),
        ));
    }
    Ok(None)
}
fn read_leading_if_clause(
    input: &SingleStaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(spec) = crate::grammar::static_line_support::parse_leading_if_clause(tokens)
        && let Ok(condition) = parse_static_condition_clause(spec.condition_tokens)
        && let Some(abilities) =
            parse_static_ability_ast_line_lexed_single_without_leading_condition(
                spec.remainder_tokens,
            )?
        && !abilities.is_empty()
    {
        if cast_this_spell_as_though_flash_tokens(spec.remainder_tokens) && abilities.len() == 1 {
            let ability = abilities.into_iter().next().expect("length checked");
            return Ok(Some(vec![
                StaticAbilityAst::LabeledConditionalStaticAbility {
                    ability: Box::new(ability),
                    condition,
                    label: render_token_slice(&trim_edge_punctuation(tokens)),
                },
            ]));
        }
        let mut conditioned = Vec::with_capacity(abilities.len());
        for ability in abilities {
            conditioned.push(add_static_ability_ast_condition(
                ability,
                condition.clone(),
            )?);
        }
        return Ok(Some(conditioned));
    }
    Ok(None)
}
fn read_fixed_prefix_condition(
    input: &SingleStaticLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(spec) = anthem_grant_grammar::parse_fixed_prefix_condition_shape(tokens)
        && spec.kind == anthem_grant_grammar::AnthemPrefixConditionKind::DuringTurnsOtherThanYours
        && let Some(abilities) =
            parse_static_ability_ast_line_lexed_single_without_leading_condition(
                spec.subject_tokens,
            )?
        && !abilities.is_empty()
    {
        let condition = PredicateAst::Not(Box::new(PredicateAst::YourTurn));
        let mut conditioned = Vec::with_capacity(abilities.len());
        for ability in abilities {
            conditioned.push(add_static_ability_ast_condition(
                ability,
                condition.clone(),
            )?);
        }
        return Ok(Some(conditioned));
    }
    Ok(None)
}
