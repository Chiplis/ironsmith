use crate::cards::builders::PlayerAst;
use crate::effect::{Until, Value};
use crate::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed;
use crate::lexer::{OwnedLexToken, parser_token_word_refs};
use crate::mana::ManaCost;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
use winnow::prelude::*;

use super::super::super::{leaf, primitives};
use super::common;

#[derive(Debug, Clone)]
pub struct MatchingSpellCostReductionShape<'a> {
    pub player: PlayerAst,
    pub filter: ObjectFilter,
    pub reduction: Value,
    pub where_value_tokens: Option<&'a [OwnedLexToken]>,
    pub duration: Until,
    pub next_spell: bool,
    pub next_spell_mana_reduction: Option<ManaCost>,
    /// "... cost {N} more to cast": the amount increases matching spells'
    /// costs instead of reducing them.
    pub increase: bool,
}

pub fn parse_matching_spell_cost_reduction_shape(
    tokens: &[OwnedLexToken],
) -> Option<MatchingSpellCostReductionShape<'_>> {
    let words = parser_token_word_refs(tokens);
    let (spell_token_idx, _, _) =
        primitives::find_prefix(tokens, || primitives::kw("spells").void())
            .or_else(|| primitives::find_prefix(tokens, || primitives::kw("spell").void()))?;
    let (cost_token_idx, _, _) = primitives::find_prefix(tokens, || primitives::kw("cost").void())
        .or_else(|| primitives::find_prefix(tokens, || primitives::kw("costs").void()))?;
    let (less_token_idx, _, after_less) =
        primitives::find_prefix(tokens, || primitives::kw("less").void())
            .or_else(|| primitives::find_prefix(tokens, || primitives::kw("more").void()))?;
    let increase = tokens[less_token_idx].is_word("more");

    let has_you_cast = common::present(&words, &["you", "cast"]);
    // "If they do, spells they cast this turn ..." names the same relative
    // player as "spells that player casts this turn".
    let has_that_player_casts = common::present(&words, &["that", "player", "casts"])
        || common::present(&words, &["spells", "they", "cast"]);
    let has_chosen_name = common::present(&words, &["with", "chosen", "name"])
        || common::present(&words, &["with", "the", "chosen", "name"]);
    // "Noncreature spells your opponents cast cost {2} more to cast until
    // your next turn" (Elspeth Conquers Death): the opponents' spells.
    let has_opponents_cast = !has_you_cast
        && !has_that_player_casts
        && (common::present(&words, &["your", "opponents", "cast"])
            || common::present(&words, &["spells", "opponents", "cast"]));
    let has_this_turn_duration = common::present(&words, &["this", "turn"]);
    let has_trailing_until_your_next_turn_duration = !common::prefix(
        &words,
        &["until", "your", "next", "turn"],
    ) && words.ends_with(&["to", "cast", "until", "your", "next", "turn"]);
    let has_until_your_next_turn_duration =
        common::prefix(&words, &["until", "your", "next", "turn"])
            || has_trailing_until_your_next_turn_duration;
    // "Noncreature spells cost {1} less to cast until end of turn": the
    // explicit duration marks a one-shot effect even without a caster
    // phrase, so it reduces every player's matching spells this turn.
    let has_trailing_until_end_of_turn_duration =
        words.ends_with(&["to", "cast", "until", "end", "of", "turn"]);

    let after_to_cast = primitives::parse_prefix(
        crate::util::trim_edge_punctuation_tokens(after_less),
        primitives::phrase(&["to", "cast"]).void(),
    )
    .map(|(_, rest)| rest);
    // "Creature spells you cast cost {1} less to cast for each 1 life your
    // opponents have lost this turn" (Rakdos, Lord of Riots): the "this turn"
    // belongs to the for-each count, so this is a permanent static
    // modifier, not a this-turn reduction that would drop the multiplier.
    if let Some(after_to_cast) = after_to_cast
        && parser_token_word_refs(after_to_cast).starts_with(&["for", "each"])
        && !common::present(&parser_token_word_refs(&tokens[..less_token_idx]), &["this", "turn"])
    {
        return None;
    }
    if cost_token_idx <= spell_token_idx
        || less_token_idx <= cost_token_idx
        || (!has_you_cast
            && !has_that_player_casts
            && !has_chosen_name
            && !has_opponents_cast
            && !has_trailing_until_end_of_turn_duration)
        || (!has_this_turn_duration
            && !has_until_your_next_turn_duration
            && !has_trailing_until_end_of_turn_duration)
        || after_to_cast.is_none()
    {
        return None;
    }

    let subject_start_token_idx = if has_until_your_next_turn_duration
        && !has_trailing_until_your_next_turn_duration
    {
        let (_, rest) = primitives::parse_prefix(
            tokens,
            primitives::phrase(&["until", "your", "next", "turn"]).void(),
        )?;
        tokens.len().checked_sub(rest.len())?
    } else {
        0
    };
    // The spell noun does not necessarily end the authored restriction. In
    // `spells you cast this turn that are black and/or red`, the relative
    // color clause is still part of the candidate-spell filter. Keep the
    // complete subject up to `cost(s)`; the spell-filter grammar deliberately
    // ignores duration words while retaining typed characteristics.
    let subject_tokens =
        crate::util::trim_edge_punctuation_tokens(&tokens[subject_start_token_idx..cost_token_idx]);
    // "... deals 3 damage, then noncreature spells cost ...": the subject
    // must not swallow a preceding sequenced clause; the chain splitter
    // reads each clause on its own.
    if parser_token_word_refs(subject_tokens).contains(&"then") {
        return None;
    }
    let reduction_tokens =
        crate::util::trim_edge_punctuation_tokens(&tokens[cost_token_idx + 1..less_token_idx]);
    let (mut reduction, used) = crate::util::parse_value(reduction_tokens)?;
    if used != reduction_tokens.len() {
        return None;
    }
    // "costs {1} less to cast for each counter removed this way" (Mutated
    // Cultist): the per-unit amount scales by the counted basis; dropping
    // the tail would leave a flat {1} reduction.
    let mut scaled_by_for_each = false;
    if let Some(after_to_cast) = after_to_cast {
        let tail = parser_token_word_refs(crate::util::trim_edge_punctuation_tokens(after_to_cast));
        if tail.starts_with(&["for", "each"])
            && let Some((per_unit, consumed)) = crate::util::parse_for_each_count_value_words(&tail)
            && consumed == tail.len()
            && let Value::Fixed(amount) = reduction
        {
            reduction = if amount == 1 {
                per_unit
            } else {
                Value::Scaled(Box::new(per_unit), amount)
            };
            scaled_by_for_each = true;
        }
    }

    let where_value_tokens = if matches!(reduction, Value::X) {
        let after_to_cast = crate::util::trim_edge_punctuation_tokens(after_to_cast?);
        primitives::find_prefix(after_to_cast, || primitives::kw("where").void())
            .map(|(where_idx, _, _)| &after_to_cast[where_idx..])
    } else {
        None
    };

    let mut filter = parse_spell_filter_with_grammar_entrypoint_lexed(subject_tokens);
    let player = if has_you_cast {
        filter.cast_by = Some(PlayerFilter::You);
        PlayerAst::You
    } else if has_that_player_casts {
        filter.cast_by = Some(PlayerFilter::IteratedPlayer);
        PlayerAst::That
    } else if has_opponents_cast {
        filter.cast_by = Some(PlayerFilter::Opponent);
        PlayerAst::Opponent
    } else {
        PlayerAst::Any
    };

    let between_words = parser_token_word_refs(&tokens[spell_token_idx + 1..cost_token_idx]);
    if has_chosen_name {
        filter.name = Some("{chosen name}".to_string());
    }
    if common::present(&between_words, &["from", "exile"]) {
        filter.zone = Some(Zone::Exile);
    } else if common::present(&between_words, &["from", "your", "graveyard"]) {
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
    }

    let next_spell = common::prefix(&words, &["the", "next"]);
    let next_spell_mana_reduction = if next_spell && !increase && !scaled_by_for_each {
        leaf::parse_leaf_fixed_mana_cost_prefix_tokens(reduction_tokens)
            .filter(|parsed| parsed.consumed == reduction_tokens.len())
            .map(|parsed| parsed.cost)
    } else {
        None
    };
    Some(MatchingSpellCostReductionShape {
        player,
        filter,
        reduction,
        where_value_tokens,
        duration: if has_until_your_next_turn_duration {
            Until::YourNextTurn
        } else {
            Until::EndOfTurn
        },
        next_spell,
        next_spell_mana_reduction,
        increase,
    })
}

#[cfg(test)]
mod tests {
    use crate::color::ColorSet;
    use crate::lexer::lex_line;

    use super::*;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn parses_matching_reduction_player_zone_value_and_duration() {
        let tokens = lex(
            "Until your next turn, instant spells you cast from your graveyard cost 2 less to cast.",
        );
        let shape = parse_matching_spell_cost_reduction_shape(&tokens).expect("reduction shape");
        assert_eq!(shape.player, PlayerAst::You);
        assert_eq!(shape.reduction, Value::Fixed(2));
        assert_eq!(shape.duration, Until::YourNextTurn);
        assert_eq!(shape.filter.zone, Some(Zone::Graveyard));
        assert_eq!(shape.filter.owner, Some(PlayerFilter::You));
    }

    #[test]
    fn preserves_typed_where_value_tail_for_x_reduction() {
        let tokens = lex(
            "Creature spells you cast cost X less to cast this turn, where X is your life total.",
        );
        let shape = parse_matching_spell_cost_reduction_shape(&tokens).expect("reduction shape");
        assert_eq!(shape.reduction, Value::X);
        assert!(shape.where_value_tokens.is_some());
    }

    #[test]
    fn preserves_relative_color_restriction_after_spell_noun() {
        let tokens = lex(
            "Spells you cast this turn that are black and/or red cost {X} less to cast, where X is the amount of life you lost this turn.",
        );
        let shape = parse_matching_spell_cost_reduction_shape(&tokens).expect("reduction shape");
        assert_eq!(shape.player, PlayerAst::You);
        assert_eq!(
            shape.filter.colors,
            Some(ColorSet::BLACK.union(ColorSet::RED))
        );
        assert_eq!(shape.filter.cast_by, Some(PlayerFilter::You));
        assert!(shape.where_value_tokens.is_some());
    }

    #[test]
    fn preserves_dynamic_next_spell_and_resolution_time_value_tail() {
        let tokens = lex(
            "The next instant or sorcery spell you cast this turn costs {X} less to cast, where X is this creature's power as this ability resolves.",
        );
        let shape = parse_matching_spell_cost_reduction_shape(&tokens).expect("reduction shape");
        assert!(shape.next_spell);
        assert!(shape.next_spell_mana_reduction.is_none());
        assert!(shape.where_value_tokens.is_some());
    }
}
