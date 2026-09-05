//! Readings shard 3 of 4, in rank order.

use super::super::*;
use super::{Predicate, Reading};
use crate::recognition::RuleId;
use crate::registry::HeadDiscriminator;

pub(super) fn read_source_simple_state_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_simple_state_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_crewed_by_exactly_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_crewed_by_exactly_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_attachment_count_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_attachment_count_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_identity_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_identity_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_keyword_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_keyword_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_triggering_object_keyword_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_triggering_object_keyword_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_did_not_attack_or_enter_control_this_turn(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) =
        parse_source_did_not_attack_or_enter_control_this_turn_shape(predicate_tokens)
    {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_there_are_no_counters_on_source_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_there_are_no_counters_on_source_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_doesnt_have_counter_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_source_doesnt_have_counter_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_has_counter_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_source_has_counter_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_has_counted_counter_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_source_has_counted_counter_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_verbless_counted_counter_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_verbless_counted_counter_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_triggering_object_had_counter_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_triggering_object_had_counter_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_there_are_source_counters_at_least_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_there_are_source_counters_at_least_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_source_power_threshold_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_source_power_threshold_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_basic_land_types_among_lands_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_basic_land_types_among_lands_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_there_are_objects_on_battlefield_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_there_are_objects_on_battlefield_predicate(tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_card_types_in_graveyard_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_card_types_in_graveyard_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_half_starting_life_total_threshold_predicate_2(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_half_starting_life_total_threshold_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_cards_in_graveyard_predicate_3(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_controls_more_than_each_other_player_predicate_2(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_controls_more_than_each_other_player_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_controls_fewer_than_you_predicate_2(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_controls_fewer_than_you_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_controls_more_than_you_predicate_2(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_controls_more_than_you_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_life_relation_predicate_2(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_life_relation_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_life_tie_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_life_tie_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_count_parity_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_count_parity_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_life_total_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_life_total_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_cards_in_hand_relation_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_cards_in_hand_relation_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_cards_in_hand_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_cards_in_hand_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_turn_event_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_turn_event_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_would_action_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_would_action_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_turn_timing_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_turn_timing_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}

/// This shard's readings, in rank order.
pub(super) const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("source-simple-state-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("tagged-state-predicate")
        },
        read: |input| input.outcome(read_source_simple_state_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-crewed-by-exactly-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_crewed_by_exactly_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-attachment-count-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_attachment_count_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-identity-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // An ordered-graveyard qualifier ("with N cards above/below it") is the ordered-graveyard reading's, or unsupported.
                && { let words = crate::lexer::parser_token_word_refs(input.predicate_tokens); !(words.contains(&"graveyard") && (words.contains(&"above") || words.contains(&"below"))) }
                // Readings ranked above this one that read the input read it.
                && !input.read_by("implicit-subject-and-predicate")
                && !input.read_by("repeated-if-or-predicate")
                && !input.read_by("source-attachment-count-predicate")
                && !input.read_by("source-graveyard-cards-above-predicate")
                && !input.read_by("source-only-creature-card-in-your-graveyard-predicate")
        },
        read: |input| input.outcome(read_source_identity_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-keyword-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_keyword_predicate(input)),
    },
    Reading {
        id: RuleId::new("triggering-object-keyword-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("source-keyword-predicate")
        },
        read: |input| input.outcome(read_triggering_object_keyword_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-did-not-attack-or-enter-control-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_did_not_attack_or_enter_control_this_turn(input)),
    },
    Reading {
        id: RuleId::new("there-are-no-counters-on-source-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_there_are_no_counters_on_source_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-doesnt-have-counter-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_doesnt_have_counter_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-has-counter-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_has_counter_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-has-counted-counter-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("source-has-counter-predicate")
        },
        read: |input| input.outcome(read_source_has_counted_counter_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-verbless-counted-counter-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("source-has-counted-counter-predicate")
        },
        read: |input| input.outcome(read_source_verbless_counted_counter_predicate(input)),
    },
    Reading {
        id: RuleId::new("triggering-object-had-counter-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_triggering_object_had_counter_predicate(input)),
    },
    Reading {
        id: RuleId::new("there-are-source-counters-at-least-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_there_are_source_counters_at_least_predicate(input)),
    },
    Reading {
        id: RuleId::new("source-power-threshold-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_power_threshold_predicate(input)),
    },
    Reading {
        id: RuleId::new("basic-land-types-among-lands-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_basic_land_types_among_lands_predicate(input)),
    },
    Reading {
        id: RuleId::new("there-are-objects-on-battlefield-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_there_are_objects_on_battlefield_predicate(input)),
    },
    Reading {
        id: RuleId::new("card-types-in-graveyard-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_card_types_in_graveyard_predicate(input)),
    },
    Reading {
        id: RuleId::new("half-starting-life-total-threshold-predicate-2"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_half_starting_life_total_threshold_predicate_2(input)),
    },
    Reading {
        id: RuleId::new("player-cards-in-graveyard-predicate-3"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("graveyard-threshold-predicate")
        },
        read: |input| input.outcome(read_player_cards_in_graveyard_predicate_3(input)),
    },
    Reading {
        id: RuleId::new("player-controls-more-than-each-other-player-predicate-2"),
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
            input.outcome(read_player_controls_more_than_each_other_player_predicate_2(input))
        },
    },
    Reading {
        id: RuleId::new("player-controls-fewer-than-you-predicate-2"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_controls_fewer_than_you_predicate_2(input)),
    },
    Reading {
        id: RuleId::new("player-controls-more-than-you-predicate-2"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_controls_more_than_you_predicate_2(input)),
    },
    Reading {
        id: RuleId::new("player-life-relation-predicate-2"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_life_relation_predicate_2(input)),
    },
    Reading {
        id: RuleId::new("player-life-tie-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_life_tie_predicate(input)),
    },
    Reading {
        id: RuleId::new("count-parity-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_count_parity_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-life-total-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_life_total_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-cards-in-hand-relation-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_cards_in_hand_relation_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-cards-in-hand-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_cards_in_hand_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-turn-event-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_turn_event_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-would-action-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_would_action_predicate(input)),
    },
    Reading {
        id: RuleId::new("turn-timing-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_turn_timing_predicate(input)),
    },
];
