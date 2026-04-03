use crate::cards::builders::{
    ActivationTiming, CardTextError, EffectAst, EffectPredicate, IfResultPredicate, LineInfo,
    ParsedModalActivatedHeader, ParsedModalGate, ParsedModalHeader,
};
use crate::effect::Value;
use crate::target::PlayerFilter;

use super::activation_and_restrictions::infer_activated_functional_zones_lexed;
use super::clause_support::{parse_effect_sentences_lexed, parse_trigger_clause_lexed};
use super::effect_ast_traversal::try_for_each_nested_effects_mut;
use super::grammar::primitives as grammar;
use super::grammar::structure::{
    parse_modal_header_choose_spec, scan_modal_header_flags, split_lexed_sentences,
    split_trailing_modal_gate_clause,
};
use super::keyword_static::parse_where_x_value_clause_lexed;
use super::leaf::{lower_activation_cost_cst, parse_activation_cost_tokens_rewrite};
use super::lexer::{OwnedLexToken, TokenKind, lex_line, render_token_slice, trim_lexed_commas};
use super::modal_helpers::{replace_unbound_x_with_value, value_contains_unbound_x};

type ModalHeader = ParsedModalHeader;
type ModalActivatedHeader = ParsedModalActivatedHeader;
type ModalGate = ParsedModalGate;

fn token_slice_has_word(tokens: &[OwnedLexToken], expected: &str) -> bool {
    tokens.iter().any(|token| token.is_word(expected))
}

fn token_slice_has_any_word(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    expected
        .iter()
        .any(|candidate| token_slice_has_word(tokens, candidate))
}

fn find_token_index(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if predicate(&tokens[idx]) {
            return Some(idx);
        }
        idx += 1;
    }

    None
}

fn strip_leading_sign(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    if bytes
        .first()
        .is_some_and(|byte| matches!(*byte, b'+' | b'-'))
    {
        return text.get(1..);
    }

    None
}

pub(crate) fn parse_modal_header(info: &LineInfo) -> Result<Option<ModalHeader>, CardTextError> {
    let tokens = lex_line(&info.normalized.normalized, info.line_index)?;
    let Some(choose_spec) = grammar::parse_all_with_display_line(
        &tokens,
        parse_modal_header_choose_spec,
        "modal-header",
        info.display_line_index,
    )?
    else {
        return Ok(None);
    };
    let modal_flags = scan_modal_header_flags(&tokens);
    let choose_idx = choose_spec.choose_idx;
    let min = choose_spec.min;
    let max = choose_spec.max;

    let mut trigger = None;
    let mut activated = None;
    let x_replacement = choose_spec.x_clause_start.and_then(|x_clause_start| {
        parse_x_is_value_clause(trim_lexed_commas(&tokens[x_clause_start..]))
    });
    let mut effect_start_idx = 0usize;
    if let Some(colon_idx) = find_token_index(&tokens, |token| token.kind == TokenKind::Colon)
        .filter(|idx| *idx < choose_idx)
    {
        let cost_tokens = &tokens[..colon_idx];
        let cost_raw = render_token_slice(cost_tokens);
        let cost_raw = cost_raw.trim();
        if !cost_raw.is_empty() {
            let cost_cst = parse_activation_cost_tokens_rewrite(cost_tokens)?;
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
        && let Some(comma_idx) = find_token_index(&tokens, |token| token.kind == TokenKind::Comma)
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
                trigger = Some(parse_trigger_clause_lexed(trigger_tokens)?);
            }
        }
        effect_start_idx = comma_idx + 1;
    }

    let prechoose_tokens = trim_lexed_commas(&tokens[effect_start_idx..choose_idx]);
    let (prefix_effects_ast, modal_gate) = parse_modal_header_prefix_effects(prechoose_tokens)?;

    Ok(Some(ModalHeader {
        min,
        max,
        same_mode_more_than_once: modal_flags.same_mode_more_than_once,
        mode_must_be_unchosen: modal_flags.mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn: modal_flags.mode_must_be_unchosen_this_turn,
        commander_allows_both: modal_flags.commander_allows_both,
        trigger,
        activated,
        x_replacement,
        prefix_effects_ast,
        modal_gate,
        line_text: info.raw_line.clone(),
    }))
}

fn parse_x_is_value_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    if tokens.len() < 2 || !tokens[0].is_word("x") || !tokens[1].is_word("is") {
        return None;
    }

    if token_slice_has_any_word(tokens, &["spell", "spells"])
        && token_slice_has_any_word(tokens, &["cast", "casts"])
        && token_slice_has_word(tokens, "turn")
    {
        let player = if token_slice_has_any_word(tokens, &["you", "your", "youve", "you've"]) {
            PlayerFilter::You
        } else if token_slice_has_any_word(tokens, &["opponent", "opponents"]) {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };
        return Some(Value::SpellsCastThisTurn(player));
    }

    let mut where_prefixed = Vec::with_capacity(tokens.len() + 3);
    where_prefixed.push(OwnedLexToken::word(
        "where",
        tokens
            .first()
            .map(|token| token.span)
            .unwrap_or_else(crate::cards::builders::TextSpan::synthetic),
    ));
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
        | EffectAst::Populate { count: amount, .. }
        | EffectAst::CreateTokenCopy { count: amount, .. }
        | EffectAst::CreateTokenCopyFromSource { count: amount, .. }
        | EffectAst::Monstrosity { amount, .. } => {
            replace_modal_header_x_in_value(amount, replacement, clause)?;
        }
        EffectAst::PreventDamageToTargetPutCounters {
            amount: Some(amount),
            ..
        } => {
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

    let (prefix_tokens, modal_gate) =
        if let Some(gate_spec) = split_trailing_modal_gate_clause(tokens) {
            let effect_predicate = match gate_spec.predicate {
                IfResultPredicate::Did => EffectPredicate::Happened,
                IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
                IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
                IfResultPredicate::WasDeclined => EffectPredicate::WasDeclined,
            };
            (
                gate_spec.prefix_tokens,
                Some(ModalGate {
                    predicate: effect_predicate,
                    remove_mode_only: gate_spec.remove_mode_only,
                }),
            )
        } else {
            (tokens, None)
        };
    if prefix_tokens.is_empty() {
        return Ok((Vec::new(), modal_gate));
    }

    let effects = parse_effect_sentences_lexed(prefix_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(
            "modal header prefix produced no effects".to_string(),
        ));
    }

    Ok((effects, modal_gate))
}

fn is_loyalty_shorthand_cost_text(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed == "0"
        || strip_leading_sign(trimmed)
            .is_some_and(|tail| tail.eq_ignore_ascii_case("x") || tail.parse::<u32>().is_ok())
}
