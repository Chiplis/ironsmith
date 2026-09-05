//! Readings shard 2 of 4, in rank order.

use super::super::*;
use super::{Predicate, Reading};
use crate::recognition::RuleId;
use crate::registry::HeadDiscriminator;

pub(super) fn read_source_exiled_with_counter_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_source_exiled_with_counter_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_happily_style_conjoined_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_happily_style_conjoined_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_colors_among_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_colors_among_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_card_types_among_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_card_types_among_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_revealed_or_controlled_subtype_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_revealed_or_controlled_subtype_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_graveyard_threshold_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_graveyard_threshold_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_card_in_your_graveyard_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_card_in_your_graveyard_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_quantified_objects_in_graveyard_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_quantified_objects_in_graveyard_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_cards_in_graveyard_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_empty_battlefield_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_empty_battlefield_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_object_on_battlefield_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_object_on_battlefield_predicate(tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_life_total_at_least_starting_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_life_total_at_least_starting_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_life_total_at_least_last_noted_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_life_total_at_least_last_noted_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_cards_in_graveyard_predicate_2(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_controls_more_than_each_other_player_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_controls_more_than_each_other_player_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_controls_fewer_than_you_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_controls_fewer_than_you_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_controls_more_than_you_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_controls_more_than_you_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_status_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_status_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_counted_objects_have_counter_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_counted_objects_have_counter_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_counted_source_exiled_objects_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_counted_source_exiled_objects_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_controlled_creatures_total_power_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_controlled_creatures_total_power_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_half_starting_life_total_threshold_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_half_starting_life_total_threshold_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_you_life_total_at_most_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_you_life_total_at_most_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_life_relation_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_life_relation_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_player_object_keyword_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_player_object_keyword_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_opponent_controls_tagged_object_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_opponent_controls_tagged_object_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_opponent_controls_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_opponent_controls_predicate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_vote_result_predicate_2(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_vote_result_predicate(predicate_tokens, false)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_attacking_you_own_control_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_attacking_you_own_control_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_you_both_own_and_control_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_you_both_own_and_control_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_while_conjoined_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_while_conjoined_predicate(predicate_tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
pub(super) fn read_tagged_state_predicate(
    input: &Predicate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if let Some(predicate) = parse_tagged_state_predicate(predicate_tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}

/// This shard's readings, in rank order.
pub(super) const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("source-exiled-with-counter-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_source_exiled_with_counter_predicate(input)),
    },
    Reading {
        id: RuleId::new("happily-style-conjoined-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_happily_style_conjoined_predicate(input)),
    },
    Reading {
        id: RuleId::new("colors-among-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_colors_among_predicate(input)),
    },
    Reading {
        id: RuleId::new("card-types-among-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_card_types_among_predicate(input)),
    },
    Reading {
        id: RuleId::new("revealed-or-controlled-subtype-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_revealed_or_controlled_subtype_predicate(input)),
    },
    Reading {
        id: RuleId::new("graveyard-threshold-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_graveyard_threshold_predicate(input)),
    },
    Reading {
        id: RuleId::new("card-in-your-graveyard-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("card-types-among-predicate")
                && !input.read_by("conjoined-cards-in-your-graveyard-predicate")
                && !input.read_by("graveyard-threshold-predicate")
        },
        read: |input| input.outcome(read_card_in_your_graveyard_predicate(input)),
    },
    Reading {
        id: RuleId::new("quantified-objects-in-graveyard-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("conjoined-cards-in-your-graveyard-predicate")
        },
        read: |input| input.outcome(read_quantified_objects_in_graveyard_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-cards-in-graveyard-predicate"),
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
        read: |input| input.outcome(read_player_cards_in_graveyard_predicate(input)),
    },
    Reading {
        id: RuleId::new("empty-battlefield-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_empty_battlefield_predicate(input)),
    },
    Reading {
        id: RuleId::new("object-on-battlefield-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("empty-battlefield-predicate")
                && !input.read_by("source-zone-predicate")
        },
        read: |input| input.outcome(read_object_on_battlefield_predicate(input)),
    },
    Reading {
        id: RuleId::new("life-total-at-least-starting-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_life_total_at_least_starting_predicate(input)),
    },
    Reading {
        id: RuleId::new("life-total-at-least-last-noted-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_life_total_at_least_last_noted_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-cards-in-graveyard-predicate-2"),
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
        read: |input| input.outcome(read_player_cards_in_graveyard_predicate_2(input)),
    },
    Reading {
        id: RuleId::new("player-controls-more-than-each-other-player-predicate"),
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
            input.outcome(read_player_controls_more_than_each_other_player_predicate(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("player-controls-fewer-than-you-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_controls_fewer_than_you_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-controls-more-than-you-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_controls_more_than_you_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-status-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_status_predicate(input)),
    },
    Reading {
        id: RuleId::new("counted-objects-have-counter-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_counted_objects_have_counter_predicate(input)),
    },
    Reading {
        id: RuleId::new("counted-source-exiled-objects-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_counted_source_exiled_objects_predicate(input)),
    },
    Reading {
        id: RuleId::new("controlled-creatures-total-power-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_controlled_creatures_total_power_predicate(input)),
    },
    Reading {
        id: RuleId::new("half-starting-life-total-threshold-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_half_starting_life_total_threshold_predicate(input)),
    },
    Reading {
        id: RuleId::new("you-life-total-at-most-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_you_life_total_at_most_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-life-relation-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_life_relation_predicate(input)),
    },
    Reading {
        id: RuleId::new("player-object-keyword-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_player_object_keyword_predicate(input)),
    },
    Reading {
        id: RuleId::new("opponent-controls-tagged-object-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_opponent_controls_tagged_object_predicate(input)),
    },
    Reading {
        id: RuleId::new("opponent-controls-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
                // Readings ranked above this one that read the input read it.
                && !input.read_by("opponent-controls-tagged-object-predicate")
                && !input.read_by("phase-step-gate-predicate")
        },
        read: |input| input.outcome(read_opponent_controls_predicate(input)),
    },
    Reading {
        id: RuleId::new("vote-result-predicate-2"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_vote_result_predicate_2(input)),
    },
    Reading {
        id: RuleId::new("attacking-you-own-control-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_attacking_you_own_control_predicate(input)),
    },
    Reading {
        id: RuleId::new("you-both-own-and-control-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_you_both_own_and_control_predicate(input)),
    },
    Reading {
        id: RuleId::new("while-conjoined-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            let predicate_tokens = input.predicate_tokens;
            !(!predicate_tokens.iter().any(|token| {
                token
                    .as_word()
                    .is_some_and(|_| !is_article(token.parser_text()))
            }))
        },
        read: |input| input.outcome(read_while_conjoined_predicate(input)),
    },
    Reading {
        id: RuleId::new("tagged-state-predicate"),
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
        },
        read: |input| input.outcome(read_tagged_state_predicate(input)),
    },
];
