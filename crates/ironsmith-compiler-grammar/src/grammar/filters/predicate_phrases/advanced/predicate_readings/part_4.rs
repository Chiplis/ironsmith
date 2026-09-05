//! Readings shard 4 of 4, in rank order.

use super::super::*;
use super::{Predicate, Reading};
use crate::recognition::RuleId;
use crate::registry::HeadDiscriminator;

pub(super) fn read_player_life_change_this_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_life_change_this_turn_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_descended_this_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_descended_this_turn_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_object_death_this_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_object_death_this_turn_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_battlefield_change_this_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_battlefield_change_this_turn_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_battlefield_entry_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_battlefield_entry_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_combat_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_combat_turn_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_spell_lifecycle_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_spell_lifecycle_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_paid_cost_label_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_paid_cost_label_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_mana_spent_capture_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_mana_spent_capture_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_attached_tagged_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_attached_tagged_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_additional_cost_object_state_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_additional_cost_object_state_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_tagged_exiled_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_tagged_exiled_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_triggering_object_source_stat_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_triggering_object_source_stat_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_value_reference_comparison_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_value_reference_comparison_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_it_demonstrative_value(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    let demonstrative_reference = demonstrative_reference_kind(predicate_tokens);
    let is_it = demonstrative_reference == Some(DemonstrativeReferenceKind::It);
    if is_it {
        if let Some(predicate) = parse_demonstrative_mana_value_predicate(predicate_tokens)? {
            return Ok(Some(predicate));
        }
        if let Some(predicate) =
            parse_demonstrative_total_power_toughness_predicate(predicate_tokens)?
        {
            return Ok(Some(predicate));
        }
        if let Some(predicate) = parse_demonstrative_power_or_toughness_predicate(predicate_tokens)?
        {
            return Ok(Some(predicate));
        }
    }
    Ok(None)
}
pub(super) fn read_demonstrative_or_descriptor(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    let demonstrative_reference = demonstrative_reference_kind(predicate_tokens);
    // Let a single demonstrative copula own its complete coordinated
    // descriptor before the broad boolean splitter sees the conjunction.
    // In particular, negation in "it isn't A or B" scopes over A or B.
    if demonstrative_reference.is_some()
        && predicate_tokens
            .iter()
            .any(|token| token_word_is(token, OR_WORD))
        && !contains_most_common_color_among_all_permanents_clause(predicate_tokens)
    {
        if let Some(predicate) = parse_demonstrative_or_descriptor_predicate(predicate_tokens)? {
            return Ok(Some(predicate));
        }
        if let Some(predicate) = parse_or_predicate(predicate_tokens)? {
            return Ok(Some(predicate));
        }
    }
    Ok(None)
}
pub(super) fn read_demonstrative_descriptor(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    let demonstrative_reference = demonstrative_reference_kind(predicate_tokens);
    // "was blocked this turn" is a passive historical-event predicate,
    // not a copular last-known-characteristics predicate. It already
    // has dedicated turn-history semantics and surface rendering.
    if demonstrative_reference.is_some() {
        if let Some(predicate) = parse_demonstrative_power_or_toughness_predicate(predicate_tokens)?
        {
            return Ok(Some(predicate));
        }
        if let Some(predicate) = parse_demonstrative_shares_predicate(predicate_tokens) {
            return Ok(Some(predicate));
        }
        if let Some(predicate) = parse_demonstrative_or_descriptor_predicate(predicate_tokens)? {
            return Ok(Some(predicate));
        }
        if let Some(predicate) = parse_demonstrative_toxic_predicate(predicate_tokens) {
            return Ok(Some(predicate));
        }
        if let Some(predicate) = parse_demonstrative_keyword_predicate(predicate_tokens) {
            return Ok(Some(predicate));
        }
        if let Some((
            descriptor_tokens,
            negative,
            has_card,
            tagged_that_enchantment,
            mut match_time,
        )) = demonstrative_descriptor_filter_tokens(predicate_tokens)
        {
            let antecedent_surface = demonstrative_antecedent_surface(predicate_tokens);
            let descriptor_clause = LexedClause::new(&descriptor_tokens);
            if surface::exact(descriptor_clause, &["blocked", "this", "turn"]) {
                match_time = DemonstrativeMatchTime::Current;
            }
            if surface::exact(descriptor_clause, &["permanent", "spell"]) {
                let mut filter =
                    crate::grammar::permission_facts::subject_filters::permanent_spell_filter();
                filter.zone = Some(Zone::Stack);
                filter.stack_kind = Some(StackObjectKind::Spell);
                if antecedent_surface.is_some() {
                    filter.set_demonstrative_antecedent_surface(antecedent_surface);
                }
                let predicate = demonstrative_match_predicate(filter, match_time);
                return Ok(Some(if negative {
                    PredicateAst::Not(Box::new(predicate))
                } else {
                    predicate
                }));
            }
            if let Some(mut filter) =
                parse_single_card_type_card_descriptor_tokens(&descriptor_tokens)
            {
                if antecedent_surface.is_some() {
                    filter.set_demonstrative_antecedent_surface(antecedent_surface);
                }
                let predicate = if filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    if match_time == DemonstrativeMatchTime::LastKnown {
                        PredicateAst::ItMatchedLastKnown(filter)
                    } else {
                        PredicateAst::ItIsLandCard
                    }
                } else {
                    demonstrative_match_predicate(filter, match_time)
                };
                return Ok(Some(if negative {
                    PredicateAst::Not(Box::new(predicate))
                } else {
                    predicate
                }));
            }
            if let Ok(mut filter) = parse_object_filter_lexed(&descriptor_tokens, false)
                && filter != ObjectFilter::default()
            {
                if antecedent_surface.is_some() {
                    filter.set_demonstrative_antecedent_surface(antecedent_surface);
                }
                if has_card
                    && filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    let predicate = if match_time == DemonstrativeMatchTime::LastKnown {
                        PredicateAst::ItMatchedLastKnown(filter)
                    } else {
                        PredicateAst::ItIsLandCard
                    };
                    return Ok(Some(if negative {
                        PredicateAst::Not(Box::new(predicate))
                    } else {
                        predicate
                    }));
                }
                if tagged_that_enchantment && match_time == DemonstrativeMatchTime::Current {
                    return Ok(Some(PredicateAst::TaggedMatches(
                        crate::tag::CompilerReferenceTag::Triggering.key(),
                        filter,
                    )));
                }
                let predicate = demonstrative_match_predicate(filter, match_time);
                return Ok(Some(if negative {
                    PredicateAst::Not(Box::new(predicate))
                } else {
                    predicate
                }));
            }
        }
    }
    Ok(None)
}
pub(super) fn read_player_controls_no_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_controls_no_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_you_control_or_graveyard_predicate_2(
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
pub(super) fn read_you_control_or_player_controls(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if non_article_token_words_starts_with_any(predicate_tokens, YOU_CONTROL_PREFIXES) {
        if let Some(predicate) =
            parse_you_control_conjoined_predicate(predicate_tokens).transpose()?
        {
            return Ok(Some(predicate));
        }

        if let Some(predicate) = parse_player_controls_predicate(
            predicate_tokens,
            PlayerAst::You,
            Some(PlayerFilter::You),
            2,
            true,
            true,
        )? {
            return Ok(Some(predicate));
        }
    }
    Ok(None)
}
pub(super) fn read_rule_3(input: &Predicate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if non_article_token_words_starts_with_any(predicate_tokens, THAT_PLAYER_CONTROLS_PREFIXES) {
        let prefix_len = if predicate_tokens
            .first()
            .is_some_and(|token| token_word_is(token, "they"))
        {
            2
        } else {
            3
        };
        if let Some(predicate) = parse_player_controls_predicate(
            predicate_tokens,
            PlayerAst::That,
            None,
            prefix_len,
            false,
            false,
        )? {
            return Ok(Some(predicate));
        }
    }
    Ok(None)
}
pub(super) fn read_negative_put_tagged_object_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_negative_put_tagged_object_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_achievement_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_achievement_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_ring_bearer_temptation_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_ring_bearer_temptation_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_status_predicate_2(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_status_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_world_state_or_timing_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_world_state_or_timing_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_combat_damage_this_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_combat_damage_this_turn_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_spell_cast_this_turn_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_spell_cast_this_turn_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_x_value_comparison_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_x_value_comparison_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_or_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_or_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}

/// This shard's readings, in rank order.
pub(super) const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("player-life-change-this-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_life_change_this_turn_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-descended-this-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_descended_this_turn_predicate(input)),
    },
    Reading {
        id: RuleId::new("object-death-this-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_object_death_this_turn_predicate(input)),
    },
    Reading {
        id: RuleId::new("battlefield-change-this-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_battlefield_change_this_turn_predicate(input)),
    },
    Reading {
        id: RuleId::new("battlefield-entry-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("phase-step-gate-predicate")
                && !input.read_by("player-turn-event-predicate")
        },
        read: |input| input.outcome(read_battlefield_entry_predicate(input)),
    },
    Reading {
        id: RuleId::new("combat-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_combat_turn_predicate(input)),
    },
    Reading {
        id: RuleId::new("spell-lifecycle-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("turn-history-intervening-predicate")
        },
        read: |input| input.outcome(read_spell_lifecycle_predicate(input)),
    },
    Reading {
        id: RuleId::new("paid-cost-label-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_paid_cost_label_predicate(input)),
    },
    Reading {
        id: RuleId::new("mana-spent-capture-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("spell-context-predicate")
        },
        read: |input| input.outcome(read_mana_spent_capture_predicate(input)),
    },
    Reading {
        id: RuleId::new("attached-tagged-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_attached_tagged_predicate(input)),
    },
    Reading {
        id: RuleId::new("additional-cost-object-state-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_additional_cost_object_state_predicate(input)),
    },
    Reading {
        id: RuleId::new("tagged-exiled-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_tagged_exiled_predicate(input)),
    },
    Reading {
        id: RuleId::new("triggering-object-source-stat-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_triggering_object_source_stat_predicate(input)),
    },
    Reading {
        id: RuleId::new("value-reference-comparison-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("source-power-threshold-predicate")
        },
        read: |input| input.outcome(read_value_reference_comparison_predicate(input)),
    },
    Reading {
        id: RuleId::new("it-demonstrative-value"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("value-reference-comparison-predicate")
        },
        read: |input| input.outcome(read_it_demonstrative_value(input)),
    },
    Reading {
        id: RuleId::new("demonstrative-or-descriptor"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("rule")
                && !input.read_by("source-verbless-counted-counter-predicate")
                && !input.read_by("stack-object-targets-object-predicate")
                && !input.read_by("tagged-state-predicate")
        },
        read: |input| input.outcome(read_demonstrative_or_descriptor(input)),
    },
    Reading {
        id: RuleId::new("demonstrative-descriptor"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("exploited-triggering-object-predicate")
                && !input.read_by("implicit-subject-and-predicate")
                && !input.read_by("passive-this-way-tagged-object-predicate")
                && !input.read_by("repeated-and-predicate")
                && !input.read_by("repeated-if-or-predicate")
                && !input.read_by("rule")
                && !input.read_by("source-crewed-by-exactly-predicate")
                && !input.read_by("source-has-counted-counter-predicate")
                && !input.read_by("source-has-counter-predicate")
                && !input.read_by("source-simple-state-predicate")
                && !input.read_by("source-suspected")
                && !input.read_by("source-verbless-counted-counter-predicate")
                && !input.read_by("source-zone-predicate")
                && !input.read_by("stack-object-targets-object-predicate")
                && !input.read_by("stack-object-targets-only-source-predicate")
                && !input.read_by("tagged-exiled-predicate")
                && !input.read_by("tagged-state-predicate")
                && !input.read_by("triggering-object-first-counters-this-turn-predicate")
                && !input.read_by("triggering-object-first-tap-this-turn-predicate")
                && !input.read_by("turn-history-intervening-predicate")
                && !input.read_by("value-reference-comparison-predicate")
        },
        read: |input| input.outcome(read_demonstrative_descriptor(input)),
    },
    Reading {
        id: RuleId::new("player-controls-no-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_controls_no_predicate(input)),
    },
    Reading {
        id: RuleId::new("you-control-or-graveyard-predicate-2"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_you_control_or_graveyard_predicate_2(input)),
    },
    Reading {
        id: RuleId::new("you-control-or-player-controls"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("each-global-greatest-power-predicate")
                && !input.read_by("phase-step-gate-predicate")
                && !input.read_by("player-controls-more-than-each-other-player-predicate")
                && !input.read_by("player-controls-no-predicate")
                && !input.read_by("spell-context-predicate")
        },
        read: |input| input.outcome(read_you_control_or_player_controls(input)),
    },
    Reading {
        id: RuleId::new("rule-3"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("player-controls-fewer-than-you-predicate")
                && !input.read_by("player-controls-more-than-each-other-player-predicate")
                && !input.read_by("player-controls-more-than-you-predicate")
        },
        read: |input| input.outcome(read_rule_3(input)),
    },
    Reading {
        id: RuleId::new("negative-put-tagged-object-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_negative_put_tagged_object_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-achievement-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_achievement_predicate(input)),
    },
    Reading {
        id: RuleId::new("ring-bearer-temptation-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_ring_bearer_temptation_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-status-predicate-2"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_status_predicate_2(input)),
    },
    Reading {
        id: RuleId::new("world-state-or-timing-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_world_state_or_timing_predicate(input)),
    },
    Reading {
        id: RuleId::new("combat-damage-this-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_combat_damage_this_turn_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-spell-cast-this-turn-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("turn-history-intervening-predicate")
        },
        read: |input| input.outcome(read_player_spell_cast_this_turn_predicate(input)),
    },
    Reading {
        id: RuleId::new("x-value-comparison-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_x_value_comparison_predicate(input)),
    },
    Reading {
        id: RuleId::new("or-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("phase-step-gate-predicate")
                && !input.read_by("rule")
                && !input.read_by("some")
                && !input.read_by("some-2")
                && !input.read_by("stack-object-targets-object-predicate")
                && !input.read_by("tagged-state-predicate")
                && !input.read_by("you-control-or-graveyard-predicate")
        },
        read: |input| input.outcome(read_or_predicate(input)),
    },
];
