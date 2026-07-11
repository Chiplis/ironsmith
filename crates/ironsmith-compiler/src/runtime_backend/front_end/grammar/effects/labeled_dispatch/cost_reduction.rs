use crate::cards::builders::PlayerAst;
use crate::effect::{Until, Value};
use crate::mana::ManaCost;
use crate::runtime_backend::front_end::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed;
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, parser_token_word_refs};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
use winnow::prelude::*;

use super::super::super::{leaf, primitives};
use super::common;

#[derive(Debug, Clone)]
pub(crate) struct MatchingSpellCostReductionShape<'a> {
    pub(crate) player: PlayerAst,
    pub(crate) filter: ObjectFilter,
    pub(crate) reduction: Value,
    pub(crate) where_value_tokens: Option<&'a [OwnedLexToken]>,
    pub(crate) duration: Until,
    pub(crate) next_spell_mana_reduction: Option<ManaCost>,
}

pub(crate) fn parse_matching_spell_cost_reduction_shape(
    tokens: &[OwnedLexToken],
) -> Option<MatchingSpellCostReductionShape<'_>> {
    let words = parser_token_word_refs(tokens);
    let (spell_token_idx, _, _) =
        primitives::find_prefix(tokens, || primitives::kw("spells").void())
            .or_else(|| primitives::find_prefix(tokens, || primitives::kw("spell").void()))?;
    let (cost_token_idx, _, _) = primitives::find_prefix(tokens, || primitives::kw("cost").void())
        .or_else(|| primitives::find_prefix(tokens, || primitives::kw("costs").void()))?;
    let (less_token_idx, _, after_less) =
        primitives::find_prefix(tokens, || primitives::kw("less").void())?;

    let has_you_cast = common::present(&words, &["you", "cast"]);
    let has_that_player_casts = common::present(&words, &["that", "player", "casts"]);
    let has_chosen_name = common::present(&words, &["with", "chosen", "name"])
        || common::present(&words, &["with", "the", "chosen", "name"]);
    let has_this_turn_duration = common::present(&words, &["this", "turn"]);
    let has_until_your_next_turn_duration =
        common::prefix(&words, &["until", "your", "next", "turn"]);

    let after_to_cast = primitives::parse_prefix(
        crate::runtime_backend::front_end::shared::util::trim_edge_punctuation_tokens(after_less),
        primitives::phrase(&["to", "cast"]).void(),
    )
    .map(|(_, rest)| rest);
    if cost_token_idx <= spell_token_idx
        || less_token_idx <= cost_token_idx
        || (!has_you_cast && !has_that_player_casts && !has_chosen_name)
        || (!has_this_turn_duration && !has_until_your_next_turn_duration)
        || after_to_cast.is_none()
    {
        return None;
    }

    let subject_start_token_idx = if has_until_your_next_turn_duration {
        let (_, rest) = primitives::parse_prefix(
            tokens,
            primitives::phrase(&["until", "your", "next", "turn"]).void(),
        )?;
        tokens.len().checked_sub(rest.len())?
    } else {
        0
    };
    let subject_tokens =
        crate::runtime_backend::front_end::shared::util::trim_edge_punctuation_tokens(
            &tokens[subject_start_token_idx..=spell_token_idx],
        );
    let reduction_tokens =
        crate::runtime_backend::front_end::shared::util::trim_edge_punctuation_tokens(
            &tokens[cost_token_idx + 1..less_token_idx],
        );
    let (reduction, used) =
        crate::runtime_backend::front_end::shared::util::parse_value(reduction_tokens)?;
    if used != reduction_tokens.len() {
        return None;
    }

    let where_value_tokens = if matches!(reduction, Value::X) {
        let after_to_cast =
            crate::runtime_backend::front_end::shared::util::trim_edge_punctuation_tokens(
                after_to_cast?,
            );
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

    let next_spell_mana_reduction = if common::prefix(&words, &["the", "next"]) {
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
        next_spell_mana_reduction,
    })
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::front_end::lexer::lex_line;

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
}
