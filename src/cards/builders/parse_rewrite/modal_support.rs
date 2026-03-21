use crate::cards::builders::{
    ActivationTiming, CardTextError, EffectAst, EffectPredicate, IfResultPredicate, LineInfo,
    ParsedModalActivatedHeader, ParsedModalGate, ParsedModalHeader, TextSpan, Token,
};
use crate::effect::Value;
use crate::target::PlayerFilter;

use super::clause_support::{rewrite_parse_effect_sentences, rewrite_parse_trigger_clause};
use super::effect_ast_traversal::try_for_each_nested_effects_mut;
use super::modal_helpers::{
    find_activation_cost_start, parse_if_result_predicate, replace_unbound_x_with_value,
    starts_with_activation_cost, value_contains_unbound_x,
};
use super::activation_and_restrictions::{
    infer_activated_functional_zones, parse_activation_cost,
    parse_loyalty_shorthand_activation_cost,
};
use super::keyword_static::parse_where_x_value_clause;
use super::util::{
    parse_number_or_x_value, split_on_period, token_index_for_word_index, tokenize_line,
    trim_commas, words,
};

type ModalHeader = ParsedModalHeader;
type ModalActivatedHeader = ParsedModalActivatedHeader;
type ModalGate = ParsedModalGate;

pub(crate) fn parse_modal_header(info: &LineInfo) -> Result<Option<ModalHeader>, CardTextError> {
    let tokens = tokenize_line(&info.normalized.normalized, info.line_index);
    let token_words = words(&tokens);
    let Some(choose_idx) = tokens.iter().position(|token| token.is_word("choose")) else {
        return Ok(None);
    };

    let mut min: Option<Value> = None;
    let mut max: Option<Value> = None;
    let choose_tokens = &tokens[choose_idx + 1..];
    if choose_tokens.len() >= 3
        && choose_tokens[0].is_word("one")
        && choose_tokens[1].is_word("or")
        && choose_tokens[2].is_word("more")
    {
        min = Some(Value::Fixed(1));
        max = None;
    } else if choose_tokens.len() >= 3
        && choose_tokens[0].is_word("one")
        && choose_tokens[1].is_word("or")
        && choose_tokens[2].is_word("both")
    {
        min = Some(Value::Fixed(1));
        max = Some(Value::Fixed(2));
    } else if choose_tokens.len() >= 2
        && choose_tokens[0].is_word("up")
        && choose_tokens[1].is_word("to")
    {
        if let Some((value, _)) = parse_number_or_x_value(&choose_tokens[2..]) {
            min = Some(Value::Fixed(0));
            max = Some(value);
        }
    } else if let Some((value, _)) = parse_number_or_x_value(choose_tokens) {
        min = Some(value.clone());
        max = Some(value);
    } else if choose_tokens.iter().any(|token| token.is_word("or")) {
        min = Some(Value::Fixed(1));
        max = Some(Value::Fixed(1));
    }

    let Some(min) = min else {
        return Ok(None);
    };

    let commander_allows_both = token_words.contains(&"commander") && token_words.contains(&"both");
    let same_mode_more_than_once = token_words
        .windows(5)
        .any(|window| window == ["same", "mode", "more", "than", "once"]);
    let mode_must_be_unchosen_this_turn = token_words.windows(6).any(|window| {
        window == ["that", "hasnt", "been", "chosen", "this", "turn"]
            || window == ["that", "hasn't", "been", "chosen", "this", "turn"]
    }) || token_words
        .windows(7)
        .any(|window| window == ["that", "has", "not", "been", "chosen", "this", "turn"]);
    let mode_must_be_unchosen = mode_must_be_unchosen_this_turn
        || token_words.windows(4).any(|window| {
            window == ["that", "hasnt", "been", "chosen"]
                || window == ["that", "hasn't", "been", "chosen"]
        })
        || token_words
            .windows(5)
            .any(|window| window == ["that", "has", "not", "been", "chosen"]);

    let mut trigger = None;
    let mut activated = None;
    let x_replacement = parse_modal_header_x_replacement(&tokens, choose_idx);
    let mut effect_start_idx = 0usize;
    if let Some(colon_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
        .filter(|idx| *idx < choose_idx)
    {
        let cost_region = &tokens[..colon_idx];
        let loyalty_shorthand_cost =
            parse_loyalty_shorthand_activation_cost(cost_region, Some(info.raw_line.as_str()));
        if let Some(cost_start) = find_activation_cost_start(cost_region)
            .or_else(|| loyalty_shorthand_cost.as_ref().map(|_| 0))
        {
            let cost_tokens = &cost_region[cost_start..];
            if !cost_tokens.is_empty()
                && (starts_with_activation_cost(cost_tokens) || loyalty_shorthand_cost.is_some())
            {
                let mana_cost = if let Some(cost) = &loyalty_shorthand_cost {
                    cost.clone()
                } else {
                    parse_activation_cost(cost_tokens)?
                };

                let prechoose_tokens = trim_commas(&tokens[colon_idx + 1..choose_idx]).to_vec();
                let effect_sentences = if prechoose_tokens.is_empty() {
                    Vec::new()
                } else {
                    split_on_period(&prechoose_tokens)
                };
                let functional_zones =
                    infer_activated_functional_zones(cost_tokens, &effect_sentences);

                activated = Some(ModalActivatedHeader {
                    mana_cost,
                    functional_zones,
                    timing: if loyalty_shorthand_cost.is_some() {
                        ActivationTiming::SorcerySpeed
                    } else {
                        ActivationTiming::AnyTime
                    },
                    additional_restrictions: if loyalty_shorthand_cost.is_some() {
                        vec!["Activate only once each turn.".to_string()]
                    } else {
                        Vec::new()
                    },
                    activation_restrictions: Vec::new(),
                });
                effect_start_idx = colon_idx + 1;
            }
        }
    }

    if activated.is_none()
        && let Some(comma_idx) = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        && choose_idx > comma_idx
    {
        let start_idx = if tokens.first().is_some_and(|token| {
            token.is_word("whenever") || token.is_word("when") || token.is_word("at")
        }) {
            1
        } else {
            0
        };
        if comma_idx > start_idx {
            let trigger_tokens = &tokens[start_idx..comma_idx];
            if !trigger_tokens.is_empty() {
                trigger = Some(rewrite_parse_trigger_clause(trigger_tokens)?);
            }
        }
        effect_start_idx = comma_idx + 1;
    }

    let prechoose_tokens = trim_commas(&tokens[effect_start_idx..choose_idx]).to_vec();
    let (prefix_effects_ast, modal_gate) = parse_modal_header_prefix_effects(&prechoose_tokens)?;

    Ok(Some(ModalHeader {
        min,
        max,
        same_mode_more_than_once,
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
        commander_allows_both,
        trigger,
        activated,
        x_replacement,
        prefix_effects_ast,
        modal_gate,
        line_text: info.raw_line.clone(),
    }))
}

fn parse_modal_header_x_replacement(tokens: &[Token], choose_idx: usize) -> Option<Value> {
    let choose_tail = tokens.get(choose_idx + 1..)?;
    let choose_tail_words = words(choose_tail);
    let x_word_idx = choose_tail_words.iter().position(|word| *word == "x")?;
    if choose_tail_words.get(x_word_idx + 1).copied() != Some("is") {
        return None;
    }

    let x_token_idx = token_index_for_word_index(choose_tail, x_word_idx)?;
    let x_clause_tokens = trim_commas(&choose_tail[x_token_idx..]);
    parse_x_is_value_clause(&x_clause_tokens)
}

fn parse_x_is_value_clause(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["x", "is"]) {
        return None;
    }

    if (words.contains(&"spell") || words.contains(&"spells"))
        && (words.contains(&"cast") || words.contains(&"casts"))
        && words.contains(&"turn")
    {
        let player = if words
            .iter()
            .any(|word| matches!(*word, "you" | "your" | "youve"))
        {
            PlayerFilter::You
        } else if words
            .iter()
            .any(|word| matches!(*word, "opponent" | "opponents"))
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };
        return Some(Value::SpellsCastThisTurn(player));
    }

    let mut with_where = Vec::with_capacity(tokens.len() + 1);
    with_where.push(Token::Word("where".to_string(), TextSpan::synthetic()));
    with_where.extend_from_slice(tokens);
    parse_where_x_value_clause(&with_where)
}

pub(crate) fn replace_modal_header_x_in_effects_ast(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_modal_header_x_in_effect_ast(effect, replacement, clause)?;
    }
    Ok(())
}

fn replace_modal_header_x_in_value(
    value: &mut Value,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    if !value_contains_unbound_x(value) {
        return Ok(());
    }
    *value = replace_unbound_x_with_value(value.clone(), replacement, clause)?;
    Ok(())
}

fn replace_modal_header_x_in_effect_ast(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::Discard { count: amount, .. }
        | EffectAst::Mill { count: amount, .. }
        | EffectAst::PutCounters { count: amount, .. }
        | EffectAst::PutCountersAll { count: amount, .. }
        | EffectAst::RemoveUpToAnyCounters { amount, .. }
        | EffectAst::RemoveCountersAll { amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::PoisonCounters { count: amount, .. }
        | EffectAst::EnergyCounters { count: amount, .. }
        | EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. }
        | EffectAst::CreateTokenCopy { count: amount, .. }
        | EffectAst::CreateTokenCopyFromSource { count: amount, .. }
        | EffectAst::Monstrosity { amount, .. } => {
            replace_modal_header_x_in_value(amount, replacement, clause)?;
        }
        EffectAst::CreateTokenWithMods {
            count: amount,
            dynamic_power_toughness,
            ..
        } => {
            replace_modal_header_x_in_value(amount, replacement, clause)?;
            if let Some((power, toughness)) = dynamic_power_toughness {
                replace_modal_header_x_in_value(power, replacement, clause)?;
                replace_modal_header_x_in_value(toughness, replacement, clause)?;
            }
        }
        EffectAst::Pump {
            power, toughness, ..
        }
        | EffectAst::SetBasePowerToughness {
            power, toughness, ..
        }
        | EffectAst::PumpAll {
            power, toughness, ..
        } => {
            replace_modal_header_x_in_value(power, replacement, clause)?;
            replace_modal_header_x_in_value(toughness, replacement, clause)?;
        }
        EffectAst::SetBasePower { power, .. } => {
            replace_modal_header_x_in_value(power, replacement, clause)?;
        }
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_modal_header_x_in_effects_ast(nested, replacement, clause)
            })?;
        }
    }

    Ok(())
}

fn parse_modal_header_prefix_effects(
    tokens: &[Token],
) -> Result<(Vec<EffectAst>, Option<ModalGate>), CardTextError> {
    if tokens.is_empty() {
        return Ok((Vec::new(), None));
    }

    let (prefix_tokens, modal_gate) = strip_trailing_modal_gate_clause(tokens);
    if prefix_tokens.is_empty() {
        return Ok((Vec::new(), modal_gate));
    }

    let effects = rewrite_parse_effect_sentences(&prefix_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(
            "modal header prefix produced no effects".to_string(),
        ));
    }

    Ok((effects, modal_gate))
}

fn strip_trailing_modal_gate_clause(tokens: &[Token]) -> (Vec<Token>, Option<ModalGate>) {
    let sentence_start = tokens
        .iter()
        .rposition(|token| matches!(token, Token::Period(_)))
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let sentence_tokens = trim_commas(&tokens[sentence_start..]);
    if sentence_tokens.is_empty() {
        return (tokens.to_vec(), None);
    }
    if !sentence_tokens
        .first()
        .is_some_and(|token| token.is_word("if") || token.is_word("when"))
    {
        return (tokens.to_vec(), None);
    }

    let comma_idx = sentence_tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .unwrap_or(sentence_tokens.len());
    if comma_idx <= 1 {
        return (tokens.to_vec(), None);
    }

    let predicate_tokens = &sentence_tokens[1..comma_idx];
    let Some(predicate) = parse_if_result_predicate(predicate_tokens) else {
        return (tokens.to_vec(), None);
    };

    let trailing_tokens = if comma_idx < sentence_tokens.len() {
        trim_commas(&sentence_tokens[comma_idx + 1..])
    } else {
        Vec::new()
    };
    if !trailing_tokens.is_empty() {
        return (tokens.to_vec(), None);
    }

    let mut prefix_tokens = tokens[..sentence_start].to_vec();
    while matches!(prefix_tokens.last(), Some(Token::Comma(_))) {
        prefix_tokens.pop();
    }

    let effect_predicate = match predicate {
        IfResultPredicate::Did => EffectPredicate::Happened,
        IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
        IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
    };
    let predicate_words = words(predicate_tokens);
    let remove_mode_only = predicate_words.len() >= 2
        && matches!(predicate_words[0], "you" | "they")
        && matches!(predicate_words[1], "remove" | "removed");

    (
        prefix_tokens,
        Some(ModalGate {
            predicate: effect_predicate,
            remove_mode_only,
        }),
    )
}
