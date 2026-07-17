use winnow::Parser as _;
use winnow::combinator::{alt, cut_err, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::token::take_till;

use super::super::grammar::primitives as grammar;
use super::super::lexer::{LexStream, OwnedLexToken};
use super::super::util::{helper_tag_for_tokens, parse_subject, trim_commas};
use super::dispatch_entry::{
    ConsultCastClause, ConsultCastCost, ConsultCastManaValueCondition, ConsultCastTiming,
    ConsultSentenceParts, parse_looked_card_reveal_filter,
};
use super::search_library::normalize_search_library_filter;
use super::{find_verb, parse_effect_chain, parse_effect_sentence_lexed};
use crate::cards::builders::{
    CardTextError, EffectAst, LibraryBottomOrderAst, LibraryConsultModeAst, ObjectFilter,
    PlayerAst, PredicateAst, SubjectAst, TagKey, TargetAst,
};
use crate::effect::Value;
use crate::runtime_backend::front_end::grammar::effects as effect_grammar;
use crate::zone::Zone;

pub(crate) fn parse_exile_top_library_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let count = super::dispatch_entry::parse_top_of_your_library_count(
        tokens,
        effect_grammar::dispatch_entry_shapes::TopLibraryAction::Exile,
    )?;

    Some(vec![EffectAst::subject_verb_exile_top_of_library(
        PlayerAst::You,
        Value::Fixed(count as i32),
        Vec::new(),
        Vec::new(),
    )])
}

pub(crate) fn parse_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<ConsultSentenceParts>, CardTextError> {
    let Some(shape) = effect_grammar::parse_consult_traversal_shape(tokens) else {
        return Ok(None);
    };
    let prefix_tokens = shape.prefix.unwrap_or_default();
    let prefix_effects = if prefix_tokens.is_empty() {
        Vec::new()
    } else {
        let effects = parse_exile_top_library_prefix(&prefix_tokens)
            .or_else(|| parse_effect_sentence_lexed(&prefix_tokens).ok())
            .or_else(|| parse_effect_chain(&prefix_tokens).ok())
            .unwrap_or_default();
        if effects.is_empty() {
            return Ok(None);
        }
        effects
    };
    let player = match shape.player {
        effect_grammar::ConsultTraversalPlayerShape::ImpliedByPrefixOrYou => {
            infer_consult_player_from_prefix(&prefix_tokens).unwrap_or(PlayerAst::You)
        }
        effect_grammar::ConsultTraversalPlayerShape::ThatPlayer => PlayerAst::That,
        effect_grammar::ConsultTraversalPlayerShape::Subject(subject) => {
            match parse_subject(&subject) {
                SubjectAst::Player(player) => player,
                _ => return Ok(None),
            }
        }
    };
    let mode = shape.mode;
    let where_x = shape.where_x;
    let effect_grammar::ConsultTraversalStopShape {
        mut stop_rule,
        max_exposed,
        filter: filter_tokens,
        kind: stop_kind,
    } = shape.stop;
    if matches!(
        stop_rule,
        crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(Value::X)
    ) && let Some(value) = where_x
    {
        stop_rule = crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs),
        );
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else if matches!(stop_kind, effect_grammar::ConsultTraversalStopKind::Passive)
        && filter_tokens.is_empty()
    {
        ObjectFilter::default()
    } else {
        match super::super::object_filters::parse_object_filter(&filter_tokens, false) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        }
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let all_tag = helper_tag_for_tokens(
        tokens,
        match mode {
            LibraryConsultModeAst::Reveal => "revealed",
            LibraryConsultModeAst::Exile => "exiled",
        },
    );
    let match_tag = helper_tag_for_tokens(tokens, "consult_match");
    let mut effects = prefix_effects;
    effects.push(if let Some(max_exposed) = max_exposed {
        EffectAst::subject_verb_consult_top_of_library_with_max_exposed(
            player,
            mode,
            filter,
            stop_rule,
            max_exposed,
            all_tag.clone(),
            match_tag.clone(),
        )
    } else {
        EffectAst::subject_verb_consult_top_of_library(
            player,
            mode,
            filter,
            stop_rule,
            all_tag.clone(),
            match_tag.clone(),
        )
    });

    Ok(Some(ConsultSentenceParts {
        effects,
        player,
        all_tag,
        match_tag,
    }))
}

fn infer_consult_player_from_prefix(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let prefix_tokens = trim_commas(tokens);
    let (_, verb_idx) = find_verb(&prefix_tokens)?;
    match parse_subject(&prefix_tokens[..verb_idx]) {
        SubjectAst::Player(player) => Some(player),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn parse_consult_condition_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    effect_grammar::parse_consult_condition_value_shape(tokens)
}

fn take_remaining_clause_tokens<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    take_till(0.., |_token: &OwnedLexToken| false).parse_next(input)
}

fn parse_face_down_search_cast_mana_value_gate_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(crate::effect::ValueComparisonOperator, Value), ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "you" => (
            alt((
                grammar::phrase(&["you", "may", "cast", "the", "exiled", "card"]),
                grammar::phrase(&["you", "may", "cast", "that", "card"]),
                grammar::phrase(&["you", "may", "cast", "it"]),
            )),
            cut_err(grammar::phrase(&["without", "paying", "its", "mana", "cost"])),
            cut_err(|input: &mut LexStream<'a>| {
                let condition_tokens = take_remaining_clause_tokens(input)?;
                let condition = parse_consult_mana_value_condition_tokens(condition_tokens)
                    .ok_or_else(|| {
                        grammar::cut_err_ctx(
                            "mana value condition",
                            "supported mana value condition",
                        )
                    })?;
                Ok((condition.operator, condition.right))
            }),
        )
            .map(|(_, _, parsed)| parsed),
        _ => fail::<_, (crate::effect::ValueComparisonOperator, Value), _>,
    }
    .parse_next(input)
}

fn parse_bargained_face_down_cast_mana_value_gate_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(crate::effect::ValueComparisonOperator, Value), ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            grammar::phrase(&["if", "this", "spell", "was", "bargained"]),
            opt(grammar::comma()),
            cut_err(parse_face_down_search_cast_mana_value_gate_inner),
        )
            .map(|(_, _, parsed)| parsed),
        _ => fail::<_, (crate::effect::ValueComparisonOperator, Value), _>,
    }
    .parse_next(input)
}

pub(crate) fn parse_bargained_face_down_cast_mana_value_gate(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::ValueComparisonOperator, Value)>, CardTextError> {
    grammar::parse_all_or_none(
        tokens,
        parse_bargained_face_down_cast_mana_value_gate_inner,
        "bargained face-down cast clause",
    )
}

fn parse_if_you_dont_remainder_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            alt((
                grammar::phrase(&["if", "you", "dont"]),
                grammar::phrase(&["if", "you", "don't"]),
                grammar::phrase(&["if", "you", "do", "not"]),
            ))
            .context(StrContext::Label("if-you-don't prefix"))
            .context(StrContext::Expected(StrContextValue::Description(
                "if you don't",
            ))),
            cut_err(grammar::comma())
                .context(StrContext::Label("if-you-don't separator"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "comma after if-you-don't clause",
                ))),
            cut_err(take_remaining_clause_tokens),
        )
            .map(|(_, _, remainder)| remainder),
        _ => fail::<_, &'a [OwnedLexToken], _>,
    }
    .parse_next(input)
}

fn parse_if_you_cant_remainder_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            alt((
                grammar::phrase(&["if", "you", "cant"]),
                grammar::phrase(&["if", "you", "can't"]),
                grammar::phrase(&["if", "you", "cannot"]),
            ))
            .context(StrContext::Label("if-you-can't prefix"))
            .context(StrContext::Expected(StrContextValue::Description(
                "if you can't",
            ))),
            cut_err(grammar::comma())
                .context(StrContext::Label("if-you-can't separator"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "comma after if-you-can't clause",
                ))),
            cut_err(take_remaining_clause_tokens),
        )
            .map(|(_, _, remainder)| remainder),
        _ => fail::<_, &'a [OwnedLexToken], _>,
    }
    .parse_next(input)
}

pub(crate) fn parse_consult_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConsultCastManaValueCondition> {
    let shape = effect_grammar::parse_consult_mana_value_condition_shape(tokens)?;
    Some(ConsultCastManaValueCondition {
        operator: shape.operator,
        right: shape.right,
    })
}

pub(crate) fn parse_consult_cast_clause(tokens: &[OwnedLexToken]) -> Option<ConsultCastClause> {
    let shape = effect_grammar::parse_consult_cast_shape(tokens)?;
    let caster = match parse_subject(&shape.caster) {
        SubjectAst::Player(player) => player,
        _ => return None,
    };
    let timing = match shape.timing {
        effect_grammar::ConsultCastTimingShape::Immediate => ConsultCastTiming::Immediate,
        effect_grammar::ConsultCastTimingShape::UntilEndOfTurn => ConsultCastTiming::UntilEndOfTurn,
        effect_grammar::ConsultCastTimingShape::UntilYourNextTurnEnd => {
            ConsultCastTiming::UntilYourNextTurnEnd
        }
    };
    let cost = match shape.cost {
        effect_grammar::ConsultCastCostShape::Normal => ConsultCastCost::Normal,
        effect_grammar::ConsultCastCostShape::WithoutPayingManaCost => {
            ConsultCastCost::WithoutPayingManaCost
        }
        effect_grammar::ConsultCastCostShape::PayLifeEqualToManaValue => {
            ConsultCastCost::PayLifeEqualToManaValue
        }
    };
    Some(ConsultCastClause {
        caster,
        allow_land: shape.allow_land,
        timing,
        cost,
        mana_value_condition: shape.mana_value_condition.map(|condition| {
            ConsultCastManaValueCondition {
                operator: condition.operator,
                right: condition.right,
            }
        }),
    })
}

pub(crate) fn parse_consult_bottom_remainder_clause(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<LibraryBottomOrderAst> {
    effect_grammar::parse_consult_bottom_remainder_shape(tokens, mode)
}

pub(crate) fn parse_if_declined_put_match_into_hand(
    tokens: &[OwnedLexToken],
    match_tag: TagKey,
) -> Option<Vec<EffectAst>> {
    if !effect_grammar::is_if_declined_put_match_into_hand_shape(tokens) {
        return None;
    }

    Some(vec![EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(match_tag, None),
        Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    )])
}

pub(crate) fn consult_cast_effects(
    clause: &ConsultCastClause,
    match_tag: TagKey,
) -> Result<Vec<EffectAst>, CardTextError> {
    if clause.allow_land && !matches!(clause.cost, ConsultCastCost::Normal) {
        return Err(CardTextError::ParseError(
            "playing a land without paying its mana cost is unsupported".to_string(),
        ));
    }

    let mut cast_effects = match clause.cost {
        ConsultCastCost::Normal | ConsultCastCost::WithoutPayingManaCost => {
            let without_paying_mana_cost =
                matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost);
            if clause.allow_land
                || matches!(
                    clause.timing,
                    ConsultCastTiming::UntilEndOfTurn | ConsultCastTiming::UntilYourNextTurnEnd
                )
            {
                let grant = if matches!(clause.timing, ConsultCastTiming::UntilYourNextTurnEnd) {
                    EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                        match_tag.clone(),
                        clause.caster,
                        clause.allow_land,
                        false,
                    )
                } else {
                    EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                        match_tag.clone(),
                        clause.caster,
                        clause.allow_land,
                        without_paying_mana_cost,
                        false,
                    )
                };
                vec![grant]
            } else {
                vec![EffectAst::May {
                    effects: vec![EffectAst::subject_verb_cast_tagged(
                        match_tag.clone(),
                        clause.caster,
                        false,
                        false,
                        without_paying_mana_cost,
                        None,
                    )],
                }]
            }
        }
        ConsultCastCost::PayLifeEqualToManaValue => {
            if clause.allow_land {
                return Err(CardTextError::ParseError(
                    "pay-life consult cast clauses cannot allow lands".to_string(),
                ));
            }
            vec![
                EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(match_tag.clone(), clause.caster, false, false, false),
                EffectAst::subject_verb_grant_tagged_spell_alternative_cost_pay_life_by_mana_value_until_end_of_turn(match_tag.clone(), clause.caster),
            ]
        }
    };

    if let Some(condition) = &clause.mana_value_condition {
        cast_effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::ValueComparison {
                left: Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(match_tag))),
                operator: condition.operator,
                right: condition.right.clone(),
            },
            if_true: cast_effects,
            if_false: Vec::new(),
        }]
    }

    Ok(cast_effects)
}

pub(crate) fn parse_if_you_dont_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(after) = grammar::parse_all_or_none(
        tokens,
        parse_if_you_dont_remainder_inner,
        "if-you-don't clause",
    )?
    else {
        return Ok(None);
    };

    let effects = parse_effect_chain(after)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

pub(crate) fn parse_if_you_cant_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(after) = grammar::parse_all_or_none(
        tokens,
        parse_if_you_cant_remainder_inner,
        "if-you-can't clause",
    )?
    else {
        return Ok(None);
    };

    let effects = parse_effect_chain(after)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Subtype;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn active_consult_preserves_explicit_repeated_card_union() {
        let tokens = lex_line(
            "Reveal cards from the top of your library until you reveal a Doctor card, a card with doctor's companion, or a Vehicle card",
            0,
        )
        .expect("consult sentence should lex");
        let parsed = parse_consult_traversal_sentence(&tokens)
            .expect("consult sentence should parse")
            .expect("consult traversal shape");
        let EffectAst::SubjectVerb(subject_verb) = &parsed.effects[0] else {
            panic!(
                "expected consult subject-verb effect: {:#?}",
                parsed.effects
            );
        };
        let crate::cards::builders::SubjectVerbActionAst::ConsultTopOfLibrary { filter, .. } =
            &subject_verb.action
        else {
            panic!("expected consult action: {subject_verb:#?}");
        };

        assert!(filter.has_explicit_union_branch_articles(), "{filter:#?}");
        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        assert_eq!(filter.any_of[0].subtypes, [Subtype::Doctor]);
        assert!(filter.any_of[1].subtypes.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.any_of[1].ability_markers,
            ["doctor's companion".to_string()]
        );
        assert_eq!(filter.any_of[2].subtypes, [Subtype::Vehicle]);
        assert_eq!(
            filter.description(),
            "a Doctor card, a card with doctor's companion, or a Vehicle card"
        );
    }
}
