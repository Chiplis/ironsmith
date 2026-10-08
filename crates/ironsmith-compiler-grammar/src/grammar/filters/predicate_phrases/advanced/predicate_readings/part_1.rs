//! Readings shard 1 of 4, in rank order.

use super::super::*;
use super::{Predicate, Reading};
use crate::cards::builders::SourcePredicateAst;
use crate::recognition::RuleId;
use crate::registry::HeadDiscriminator;

fn read_exact_revealed_color_count(input: &Predicate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let words = crate::lexer::token_word_refs(input.predicate_tokens);
    if words != ["that", "opponent", "reveals", "exactly", "the", "chosen", "number", "of", "cards", "of", "the", "chosen", "color"] {
        return Ok(None);
    }
    let mut revealed = ironsmith_core::PriorEffectMetricQuery::new(
        ironsmith_core::EffectMetricSource::AffectedObjects, ironsmith_core::EffectMetric::Count,
    ).with_action(ironsmith_core::PriorEffectAction::Revealed)
        .with_filter(ObjectFilter::default().of_chosen_color());
    revealed.color_choice = Some(ironsmith_core::ColorChoiceReference::Pending);
    let number = ironsmith_core::PriorEffectMetricQuery::new(
        ironsmith_core::EffectMetricSource::Outcome, ironsmith_core::EffectMetric::Count,
    ).with_action(ironsmith_core::PriorEffectAction::ChosenNumber);
    Ok(Some(PredicateAst::ValueComparison {
        left: Value::PendingPriorEffectMetric(revealed),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::PendingPriorEffectMetric(number),
    }))
}

pub(super) fn read_saddled(input: &Predicate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    // Trigger structure owns the subject in "attacks while saddled" and may
    // pass only the typed state tail to the predicate grammar.  The omitted
    // subject is therefore the source object, just as in the complete
    // "this creature is saddled" spelling handled below.
    if surface::exact(LexedClause::new(predicate_tokens), &["saddled"]) {
        return Ok(Some(PredicateAst::Source(
            SourcePredicateAst::SourceIsSaddled,
        )));
    }
    Ok(None)
}
pub(super) fn read_you_controlled_as_cast_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_you_controlled_as_cast_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_conjoined_cards_in_your_graveyard_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    // Repeated articles on both sides of "and" are independent existential
    // requirements. Preserve that relationship before the broad conjunction
    // parsers can merge both card types into one disjunctive filter.
    if let Some(predicate) = parse_conjoined_cards_in_your_graveyard_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_you_control_or_returned_to_hand_this_way_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    // Keep independently articulated control conjunctions ahead of the broad
    // phase-step control gate, whose generic object-filter parser would merge
    // them into one filter (for example, "an artifact and a creature").
    if let Some(predicate) =
        parse_you_control_or_returned_to_hand_this_way_predicate(predicate_tokens).transpose()?
    {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_you_control_or_graveyard_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) =
        parse_you_control_or_graveyard_predicate(predicate_tokens).transpose()?
    {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_implicit_subject_and_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    // Split independently articulated player predicates before the broad
    // control-object conjunction parser. The latter is intentionally for
    // phrases such as "you control an artifact and a creature"; if it sees
    // "you control no permanents ... and have no cards in hand" first, it
    // treats both authored negatives as object-filter text and inverts them.
    if let Some(predicate) = parse_implicit_subject_and_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_you_control_conjunction(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if non_article_token_words_starts_with_any(predicate_tokens, YOU_CONTROL_PREFIXES)
        && let Some(predicate) =
            parse_you_control_conjoined_predicate(predicate_tokens).transpose()?
    {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_each_global_greatest_power_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_each_global_greatest_power_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_a_global_greatest_power_control_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_a_global_greatest_power_control_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_phase_step_gate_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = phase_step_gates::parse_phase_step_gate_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_triggering_spell_ordinal_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_triggering_spell_ordinal_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    if let Some(predicate) = parse_triggering_land_play_ordinal_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_regenerated_this_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_regenerated_this_turn_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_only_creature_card_in_your_graveyard_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) =
        parse_source_only_creature_card_in_your_graveyard_predicate(predicate_tokens)
    {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_turn_history_intervening_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_turn_history_intervening_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_triggering_object_first_tap_this_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_triggering_object_first_tap_this_turn_predicate(predicate_tokens)
    {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_triggering_object_first_counters_this_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) =
        parse_triggering_object_first_counters_this_turn_predicate(predicate_tokens)
    {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_repeated_if_or_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_repeated_if_or_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_repeated_and_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_repeated_and_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_suspected(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    {
        let simple_words = non_article_token_word_refs(predicate_tokens);
        if [
            &["it", "is", "suspected"][..], &["its", "suspected"],
            &["any", "of", "them", "are", "suspected"],
            &["they", "are", "suspected"], &["theyre", "suspected"],
        ].iter().any(|expected| surface::exact_words(&simple_words, expected)) {
            return Ok(Some(PredicateAst::ItMatches(ObjectFilter::default().suspected())));
        }
        if [
            &["this", "creature", "is", "suspected"][..],
            &["this", "permanent", "is", "suspected"][..],

        ]
        .iter()
        .any(|expected| surface::exact_words(&simple_words, expected))
        {
            return Ok(Some(PredicateAst::Source(
                SourcePredicateAst::SourceSuspected,
            )));
        }
    }
    Ok(None)
}
pub(super) fn read_secret_choices_match_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_secret_choices_match_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_vote_result_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_vote_result_predicate(predicate_tokens, true)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_passive_this_way_tagged_object_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_passive_this_way_tagged_object_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_active_this_way_discard_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_active_this_way_discard_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_active_this_way_battlefield_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_active_this_way_battlefield_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_passive_this_way_battlefield_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_passive_this_way_battlefield_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_this_ability_resolution_count_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_this_ability_resolution_count_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_stack_object_targets_only_source_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_stack_object_targets_only_source_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_stack_object_targets_object_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_stack_object_targets_object_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_spell_context_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    // Spell-context comparisons are exact typed predicates.  Parse them
    // before broader control/object predicates can accept only the leading
    // "you control ..." portion and discard the relative spell controller.
    if let Some(predicate) = parse_spell_context_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_exploited_triggering_object_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_exploited_triggering_object_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_graveyard_cards_above_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_graveyard_cards_above_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_zone_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_zone_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}

/// This shard's readings, in rank order.
/// "it would destroy a land you control": the referenced stack object's
/// resolution would destroy a matching object (Equinox). The relation is
/// carried on the stack object's filter so the runtime can inspect its
/// destroy instructions and chosen targets.
pub(super) fn read_stack_object_would_destroy_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.predicate_tokens;
    let subject_len = match tokens {
        [it, would, destroy, ..]
            if it.is_word("it") && would.is_word("would") && destroy.is_word("destroy") =>
        {
            1
        }
        [that, spell, would, destroy, ..]
            if that.is_word("that")
                && spell.is_word("spell")
                && would.is_word("would")
                && destroy.is_word("destroy") =>
        {
            2
        }
        _ => return Ok(None),
    };
    let destroyed_tokens = &tokens[subject_len + 2..];
    if destroyed_tokens.is_empty() {
        return Ok(None);
    }
    let Ok(destroyed) = parse_object_filter_lexed(destroyed_tokens, false) else {
        return Ok(None);
    };
    let mut filter = ObjectFilter::default();
    filter.would_destroy_object = Some(Box::new(destroyed));
    Ok(Some(PredicateAst::ItMatches(filter)))
}

pub(super) const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("exact-revealed-color-count"),
        head: HeadDiscriminator::Words(&["if", "that"]),
        admits: |_| true,
        read: |input| input.outcome(read_exact_revealed_color_count(input)),
    },
    Reading {
        id: RuleId::new("stack-object-would-destroy-predicate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_stack_object_would_destroy_predicate(input)),
    },
    Reading {
        id: RuleId::new("saddled"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_saddled(input)),
    },
    Reading {
        id: RuleId::new("you-controlled-as-cast-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_you_controlled_as_cast_predicate(input)),
    },
    Reading {
        id: RuleId::new("conjoined-cards-in-your-graveyard-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_conjoined_cards_in_your_graveyard_predicate(input)),
    },
    Reading {
        id: RuleId::new("you-control-or-returned-to-hand-this-way-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| {
            input.outcome(read_you_control_or_returned_to_hand_this_way_predicate(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("you-control-or-graveyard-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_you_control_or_graveyard_predicate(input)),
    },
    Reading {
        id: RuleId::new("implicit-subject-and-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_implicit_subject_and_predicate(input)),
    },
    Reading {
        id: RuleId::new("you-control-conjunction"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("implicit-subject-and-predicate")
        },
        read: |input| input.outcome(read_you_control_conjunction(input)),
    },
    Reading {
        id: RuleId::new("each-global-greatest-power-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_each_global_greatest_power_predicate(input)),
    },
    Reading {
        id: RuleId::new("a-global-greatest-power-control-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_a_global_greatest_power_control_predicate(input)),
    },
    Reading {
        id: RuleId::new("phase-step-gate-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("a-global-greatest-power-control-predicate")
                && !input.read_by("implicit-subject-and-predicate")
                && !input.read_by("you-control-conjunction")
                && !input.read_by("you-control-or-graveyard-predicate")
                && !input.read_by("you-control-or-returned-to-hand-this-way-predicate")
        },
        read: |input| input.outcome(read_phase_step_gate_predicate(input)),
    },
    Reading {
        id: RuleId::new("triggering-spell-ordinal-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_triggering_spell_ordinal_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-regenerated-this-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_regenerated_this_turn_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-only-creature-card-in-your-graveyard-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| {
            input.outcome(read_source_only_creature_card_in_your_graveyard_predicate(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("turn-history-intervening-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_turn_history_intervening_predicate(input)),
    },
    Reading {
        id: RuleId::new("triggering-object-first-tap-this-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_triggering_object_first_tap_this_turn_predicate(input)),
    },
    Reading {
        id: RuleId::new("triggering-object-first-counters-this-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| {
            input.outcome(read_triggering_object_first_counters_this_turn_predicate(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("repeated-if-or-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("implicit-subject-and-predicate")
        },
        read: |input| input.outcome(read_repeated_if_or_predicate(input)),
    },
    Reading {
        id: RuleId::new("repeated-and-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("implicit-subject-and-predicate")
        },
        read: |input| input.outcome(read_repeated_and_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-suspected"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_suspected(input)),
    },
    Reading {
        id: RuleId::new("secret-choices-match-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_secret_choices_match_predicate(input)),
    },
    Reading {
        id: RuleId::new("vote-result-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_vote_result_predicate(input)),
    },
    Reading {
        id: RuleId::new("passive-this-way-tagged-object-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_passive_this_way_tagged_object_predicate(input)),
    },
    Reading {
        id: RuleId::new("active-this-way-discard-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_active_this_way_discard_predicate(input)),
    },
    Reading {
        id: RuleId::new("active-this-way-battlefield-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_active_this_way_battlefield_predicate(input)),
    },
    Reading {
        id: RuleId::new("passive-this-way-battlefield-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_passive_this_way_battlefield_predicate(input)),
    },
    Reading {
        id: RuleId::new("this-ability-activation-count-predicate"),
        head: HeadDiscriminator::Words(&["this"]),
        admits: |input| input.predicate_tokens.first().is_some_and(|token| token.is_word("this")),
        read: |input| input.outcome(Ok(parse_this_ability_activation_count_predicate(input.predicate_tokens))),
    },
    Reading {
        id: RuleId::new("this-ability-resolution-count-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_this_ability_resolution_count_predicate(input)),
    },
    Reading {
        id: RuleId::new("stack-object-targets-only-source-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_stack_object_targets_only_source_predicate(input)),
    },
    Reading {
        id: RuleId::new("stack-object-targets-object-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_stack_object_targets_object_predicate(input)),
    },
    Reading {
        id: RuleId::new("spell-context-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_spell_context_predicate(input)),
    },
    Reading {
        id: RuleId::new("exploited-triggering-object-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_exploited_triggering_object_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-graveyard-cards-above-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_graveyard_cards_above_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-zone-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_zone_predicate(input)),
    },
];
