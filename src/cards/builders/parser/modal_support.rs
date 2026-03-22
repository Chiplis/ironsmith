use crate::cards::builders::{
    ActivationTiming, CardTextError, EffectAst, EffectPredicate, IfResultPredicate, LineInfo,
    ParsedModalActivatedHeader, ParsedModalGate, ParsedModalHeader,
};
use crate::effect::Value;
use crate::target::PlayerFilter;

use super::LowercaseWordView;
use super::activation_and_restrictions::infer_activated_functional_zones_lexed;
use super::clause_support::{
    rewrite_parse_effect_sentences_lexed, rewrite_parse_trigger_clause_lexed,
};
use super::effect_ast_traversal::try_for_each_nested_effects_mut;
use super::keyword_static::parse_where_x_value_clause_lexed;
use super::leaf::{lower_activation_cost_cst, parse_activation_cost_rewrite};
use super::lexer::{OwnedLexToken, TokenKind, lex_line, split_lexed_sentences, trim_lexed_commas};
use super::modal_helpers::{
    parse_if_result_predicate_lexed, replace_unbound_x_with_value, value_contains_unbound_x,
};
use super::util::parse_number_word_u32;

type ModalHeader = ParsedModalHeader;
type ModalActivatedHeader = ParsedModalActivatedHeader;
type ModalGate = ParsedModalGate;

pub(crate) fn parse_modal_header(info: &LineInfo) -> Result<Option<ModalHeader>, CardTextError> {
    let tokens = lex_line(&info.normalized.normalized, info.line_index)?;
    let token_view = LowercaseWordView::new(&tokens);
    let token_words = token_view.to_word_refs();
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
        if let Some((value, _)) = parse_number_or_x_value_lexed(&choose_tokens[2..]) {
            min = Some(Value::Fixed(0));
            max = Some(value);
        }
    } else if let Some((value, _)) = parse_number_or_x_value_lexed(choose_tokens) {
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
        .position(|token| token.kind == TokenKind::Colon)
        .filter(|idx| *idx < choose_idx)
    {
        let cost_tokens = &tokens[..colon_idx];
        let cost_raw = slice_for_tokens(&info.normalized.normalized, cost_tokens)
            .unwrap_or_default()
            .trim();
        if !cost_raw.is_empty() {
            let cost_cst = parse_activation_cost_rewrite(cost_raw)?;
            let mana_cost = lower_activation_cost_cst(&cost_cst)?;
            let prechoose_tokens = trim_lexed_commas(&tokens[colon_idx + 1..choose_idx]);
            let effect_sentences = if prechoose_tokens.is_empty() {
                Vec::new()
            } else {
                split_lexed_sentences(prechoose_tokens)
            };
            let loyalty_shorthand = is_loyalty_shorthand_cost_text(cost_raw);
            let functional_zones =
                infer_activated_functional_zones_lexed(cost_tokens, &effect_sentences);

            activated = Some(ModalActivatedHeader {
                mana_cost,
                functional_zones,
                timing: if loyalty_shorthand {
                    ActivationTiming::SorcerySpeed
                } else {
                    ActivationTiming::AnyTime
                },
                additional_restrictions: if loyalty_shorthand {
                    vec!["Activate only once each turn.".to_string()]
                } else {
                    Vec::new()
                },
                activation_restrictions: Vec::new(),
            });
            effect_start_idx = colon_idx + 1;
        }
    }

    if activated.is_none()
        && let Some(comma_idx) = tokens
            .iter()
            .position(|token| token.kind == TokenKind::Comma)
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
                trigger = Some(rewrite_parse_trigger_clause_lexed(trigger_tokens)?);
            }
        }
        effect_start_idx = comma_idx + 1;
    }

    let prechoose_tokens = trim_lexed_commas(&tokens[effect_start_idx..choose_idx]);
    let (prefix_effects_ast, modal_gate) = parse_modal_header_prefix_effects(prechoose_tokens)?;

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

fn parse_modal_header_x_replacement(tokens: &[OwnedLexToken], choose_idx: usize) -> Option<Value> {
    let choose_tail = tokens.get(choose_idx + 1..)?;
    let choose_tail_words = LowercaseWordView::new(choose_tail);
    let choose_tail_word_refs = choose_tail_words.to_word_refs();
    let x_word_idx = choose_tail_word_refs.iter().position(|word| *word == "x")?;
    if choose_tail_word_refs.get(x_word_idx + 1).copied() != Some("is") {
        return None;
    }

    let x_token_idx = choose_tail_words.token_index_for_word_index(x_word_idx)?;
    let x_clause_tokens = trim_lexed_commas(&choose_tail[x_token_idx..]);
    parse_x_is_value_clause(x_clause_tokens)
}

fn parse_x_is_value_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = LowercaseWordView::new(tokens);
    let words = word_view.to_word_refs();
    if !words.starts_with(&["x", "is"]) {
        return None;
    }

    if (words.contains(&"spell") || words.contains(&"spells"))
        && (words.contains(&"cast") || words.contains(&"casts"))
        && words.contains(&"turn")
    {
        let player = if words
            .iter()
            .any(|word| matches!(*word, "you" | "your" | "youve" | "you've"))
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

    let mut where_prefixed = Vec::with_capacity(tokens.len() + 3);
    where_prefixed.push(OwnedLexToken {
        kind: TokenKind::Word,
        slice: "where".to_string(),
        span: tokens
            .first()
            .map(|token| token.span)
            .unwrap_or_else(crate::cards::builders::TextSpan::synthetic),
    });
    where_prefixed.extend_from_slice(tokens);
    parse_where_x_value_clause_lexed(&where_prefixed)
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
    tokens: &[OwnedLexToken],
) -> Result<(Vec<EffectAst>, Option<ModalGate>), CardTextError> {
    if tokens.is_empty() {
        return Ok((Vec::new(), None));
    }

    let (prefix_tokens, modal_gate) = strip_trailing_modal_gate_clause(tokens);
    if prefix_tokens.is_empty() {
        return Ok((Vec::new(), modal_gate));
    }

    let effects = rewrite_parse_effect_sentences_lexed(prefix_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(
            "modal header prefix produced no effects".to_string(),
        ));
    }

    Ok((effects, modal_gate))
}

fn strip_trailing_modal_gate_clause(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<ModalGate>) {
    let sentence_start = tokens
        .iter()
        .rposition(|token| token.kind == TokenKind::Period)
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let sentence_tokens = trim_lexed_commas(&tokens[sentence_start..]);
    if sentence_tokens.is_empty() {
        return (tokens, None);
    }
    if !sentence_tokens
        .first()
        .is_some_and(|token| token.is_word("if") || token.is_word("when"))
    {
        return (tokens, None);
    }

    let comma_idx = sentence_tokens
        .iter()
        .position(|token| token.kind == TokenKind::Comma)
        .unwrap_or(sentence_tokens.len());
    if comma_idx <= 1 {
        return (tokens, None);
    }

    let predicate_tokens = &sentence_tokens[1..comma_idx];
    let Some(predicate) = parse_if_result_predicate_lexed(predicate_tokens) else {
        return (tokens, None);
    };

    let trailing_tokens = if comma_idx < sentence_tokens.len() {
        trim_lexed_commas(&sentence_tokens[comma_idx + 1..])
    } else {
        &[]
    };
    if !trailing_tokens.is_empty() {
        return (tokens, None);
    }

    let mut prefix_end = sentence_start;
    while prefix_end > 0 && tokens[prefix_end - 1].kind == TokenKind::Comma {
        prefix_end -= 1;
    }

    let effect_predicate = match predicate {
        IfResultPredicate::Did => EffectPredicate::Happened,
        IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
        IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
    };
    let predicate_view = LowercaseWordView::new(predicate_tokens);
    let predicate_words = predicate_view.to_word_refs();
    let remove_mode_only = predicate_words.len() >= 2
        && matches!(predicate_words[0], "you" | "they")
        && matches!(predicate_words[1], "remove" | "removed");

    (
        &tokens[..prefix_end],
        Some(ModalGate {
            predicate: effect_predicate,
            remove_mode_only,
        }),
    )
}

fn parse_number_or_x_value_lexed(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let first = tokens.first()?.as_word()?.to_ascii_lowercase();
    if first == "x" {
        return Some((Value::X, 1));
    }
    if let Ok(value) = first.parse::<i32>() {
        return Some((Value::Fixed(value), 1));
    }
    parse_number_word_u32(&first).map(|value| (Value::Fixed(value as i32), 1))
}

fn is_loyalty_shorthand_cost_text(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == "0"
        || trimmed
            .strip_prefix(['+', '-'])
            .is_some_and(|tail| tail.eq_ignore_ascii_case("x") || tail.parse::<u32>().is_ok())
}

fn slice_for_tokens<'a>(text: &'a str, tokens: &[OwnedLexToken]) -> Option<&'a str> {
    let first = tokens.first()?;
    let last = tokens.last()?;
    text.get(first.span.start..last.span.end)
}
