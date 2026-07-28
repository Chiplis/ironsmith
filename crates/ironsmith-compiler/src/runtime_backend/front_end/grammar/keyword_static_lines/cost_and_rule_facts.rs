use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::{leaf, primitives, static_keyword_cost_shapes};
use super::nearby_primitives::semantic_kw;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CostPrefixCondition<'a> {
    DuringTurnsOtherThanYours {
        subject_start: usize,
    },
    DuringYourTurn {
        subject_start: usize,
    },
    AsLongAs {
        condition_tokens: &'a [OwnedLexToken],
        subject_start: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EquipCostPayer {
    Unspecified,
    You,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EquipCostModifierHead {
    pub(crate) cost_token: usize,
    pub(crate) payer: EquipCostPayer,
    pub(crate) source_relative_equipment: bool,
}

pub(crate) fn parse_starting_life_bonus_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    primitives::parse_all(
        tokens,
        parse_starting_life_bonus_lexed,
        "starting life bonus",
    )
    .ok()
}

pub(crate) fn parse_buyback_cost_reduction_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    primitives::parse_all(
        tokens,
        parse_buyback_cost_reduction_lexed,
        "buyback cost reduction",
    )
    .ok()
}

pub(crate) fn parse_cost_increase_per_target_marker_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        primitives::phrase(&["target", "beyond", "the", "first"]).void()
    })
    .is_some()
        && primitives::find_prefix(tokens, || primitives::kw("more").void()).is_some()
}

pub(crate) fn parse_more_cost_tail_prefix_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::kw("more")).is_some()
}

pub(crate) fn parse_cost_prefix_condition_tokens(
    tokens: &[OwnedLexToken],
    spells_token_idx: usize,
) -> Option<CostPrefixCondition<'_>> {
    let head_tokens = tokens.get(..spells_token_idx.min(tokens.len()))?;
    let comma = static_keyword_cost_shapes::parse_cost_prefix_subject_comma(head_tokens)
        .map(|boundary| boundary.token + 1);
    if primitives::parse_prefix(
        tokens,
        primitives::phrase(&["during", "turns", "other", "than", "yours"]),
    )
    .is_some()
    {
        return Some(CostPrefixCondition::DuringTurnsOtherThanYours {
            subject_start: comma.unwrap_or(5),
        });
    }
    if primitives::parse_prefix(tokens, primitives::phrase(&["during", "your", "turn"])).is_some() {
        return Some(CostPrefixCondition::DuringYourTurn {
            subject_start: comma.unwrap_or(3),
        });
    }
    if primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"])).is_some() {
        let subject_start = comma?;
        let condition_tokens = trim_lexed_commas(tokens.get(3..subject_start)?);
        return Some(CostPrefixCondition::AsLongAs {
            condition_tokens,
            subject_start,
        });
    }
    None
}

pub(crate) fn parse_equip_cost_modifier_head_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EquipCostModifierHead> {
    let words = crate::runtime_backend::util::possessive_normalized_word_refs(
        &crate::runtime_backend::lexer::parser_token_word_refs(tokens),
    );
    let starts_with = |prefix: &[&str]| {
        words.len() >= prefix.len()
            && words
                .iter()
                .zip(prefix)
                .all(|(word, expected)| word.eq_ignore_ascii_case(expected))
    };
    let source_relative_equipment = words.len() >= 5
        && starts_with(&["this"])
        && matches!(
            words[1].to_ascii_lowercase().as_str(),
            "equipment" | "equipments"
        )
        && words[2].eq_ignore_ascii_case("equip")
        && words[3].eq_ignore_ascii_case("abilities")
        && matches!(words[4].to_ascii_lowercase().as_str(), "cost" | "costs");
    let has_equip_cost_head = starts_with(&["equip", "costs"])
        || starts_with(&["equip", "cost"])
        || source_relative_equipment;
    if !has_equip_cost_head {
        return None;
    }
    let cost_token = static_keyword_cost_shapes::parse_last_cost_verb(tokens)?.token;
    let payer = if primitives::find_prefix(tokens, || primitives::phrase(&["you", "pay"]).void())
        .is_some()
    {
        EquipCostPayer::You
    } else if primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["your", "opponents", "pay"]),
            primitives::phrase(&["opponents", "pay"]),
            primitives::phrase(&["opponent", "pays"]),
        ))
        .void()
    })
    .is_some()
    {
        EquipCostPayer::Opponent
    } else {
        EquipCostPayer::Unspecified
    };
    Some(EquipCostModifierHead {
        cost_token,
        payer,
        source_relative_equipment,
    })
}

pub(crate) fn parse_that_much_value_marker_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::phrase(&["that", "much"])).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegendRuleScopeShape {
    Global,
    Controller,
    ControllerTokens,
}

pub(crate) fn parse_legend_rule_doesnt_apply_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LegendRuleScopeShape> {
    let has_negative = primitives::find_prefix(tokens, || {
        alt((
            primitives::kw("doesnt").void(),
            primitives::kw("doesn't").void(),
            primitives::phrase(&["does", "not"]),
        ))
        .void()
    })
    .is_some();
    (has_negative
        && primitives::find_prefix(tokens, || primitives::phrase(&["legend", "rule"]).void())
            .is_some()
        && primitives::find_prefix(tokens, || primitives::kw("apply").void()).is_some())
    .then(|| {
        if primitives::find_prefix(tokens, || primitives::kw("tokens").void()).is_some()
            && primitives::find_prefix(tokens, || primitives::kw("you").void()).is_some()
        {
            LegendRuleScopeShape::ControllerTokens
        } else if primitives::find_prefix(tokens, || primitives::kw("you").void()).is_some() {
            LegendRuleScopeShape::Controller
        } else {
            LegendRuleScopeShape::Global
        }
    })
}

pub(crate) fn parse_all_cards_spells_permanents_colorless_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || semantic_kw("colorless")).is_some()
        && primitives::find_prefix(tokens, || semantic_kw("cards")).is_some()
        && primitives::find_prefix(tokens, || semantic_kw("spells")).is_some()
        && primitives::find_prefix(tokens, || semantic_kw("permanents")).is_some()
}

fn parse_starting_life_bonus_lexed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    primitives::phrase(&["you", "start", "the", "game", "with"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::kw("additional")))
        .void()
        .parse_next(input)?;
    primitives::kw("additional").parse_next(input)?;
    let amount = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::kw("life").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(amount)
}

fn parse_buyback_cost_reduction_lexed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    primitives::phrase(&["buyback", "costs", "cost"]).parse_next(input)?;
    let amount = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::kw("less").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(amount)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_cost_and_rule_facts() {
        let tokens = lex_line("You start the game with an additional 10 life.", 0).unwrap();
        assert_eq!(parse_starting_life_bonus_tokens(&tokens), Some(10));
        let tokens = lex_line("Buyback costs cost 2 less.", 0).unwrap();
        assert_eq!(parse_buyback_cost_reduction_tokens(&tokens), Some(2));
        let tokens = lex_line("The legend rule doesn't apply to you.", 0).unwrap();
        assert_eq!(
            parse_legend_rule_doesnt_apply_tokens(&tokens),
            Some(LegendRuleScopeShape::Controller)
        );
        let tokens = lex_line("The legend rule doesn't apply.", 0).unwrap();
        assert_eq!(
            parse_legend_rule_doesnt_apply_tokens(&tokens),
            Some(LegendRuleScopeShape::Global)
        );
        let tokens = lex_line(
            "The \"legend rule\" doesn't apply to tokens you control.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_legend_rule_doesnt_apply_tokens(&tokens),
            Some(LegendRuleScopeShape::ControllerTokens)
        );
    }

    #[test]
    fn parses_source_relative_equipment_equip_cost_modifier_head() {
        let tokens = lex_line(
            "This Equipment's equip abilities cost {2} less to activate.",
            0,
        )
        .unwrap();
        let head = parse_equip_cost_modifier_head_tokens(&tokens)
            .expect("source-relative equip cost modifier head");
        assert!(head.source_relative_equipment);
        assert_eq!(tokens[head.cost_token].as_word(), Some("cost"));
        assert_eq!(head.payer, EquipCostPayer::Unspecified);
    }
}
