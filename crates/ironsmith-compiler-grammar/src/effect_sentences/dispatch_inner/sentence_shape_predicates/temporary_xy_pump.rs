//! Bind a complete two-variable P/T expression without adding global X/Y state.
use crate::cards::builders::{CardTextError, ConditionalEffectAst, EffectAst};
use crate::effect::Value;
use crate::lexer::OwnedLexToken;

pub(super) fn parse(
    body: &[OwnedLexToken],
    binding: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = crate::grammar::anthem_grants::parse_where_x_y_bindings_shape(binding) else {
        return Ok(None);
    };
    let value = |tokens| {
        crate::grammar::shared_util::value_expr::parse_value_expr_tokens(tokens)
            .filter(|(_, used)| *used == tokens.len())
            .map(|(value, _)| value)
    };
    let (Some(x), Some(mut y)) = (value(shape.x_tokens), value(shape.y_tokens)) else {
        return Ok(None);
    };
    // The pronoun in a coordinated characteristic binding refers to the
    // explicitly named object in the preceding binding, not the pump recipient.
    if crate::lexer::token_word_refs(shape.y_tokens) == ["its", "toughness"] {
        fn power_reference(value: &Value) -> Option<&crate::target::ChooseSpec> {
            match value {
                Value::SurfaceHinted { value, .. } => power_reference(value),
                Value::PowerOf(spec) => Some(spec),
                _ => None,
            }
        }
        if let Some(reference) = power_reference(&x) {
            y = Value::ToughnessOf(Box::new(reference.clone()));
        }
    }
    parse_body(crate::util::trim_edge_punctuation_tokens(body), &x, &y)
}

fn parse_body(
    body: &[OwnedLexToken],
    x: &Value,
    y: &Value,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if body.first().is_some_and(|token| token.is_word("if")) {
        let Some((head, tail)) = crate::grammar::primitives::split_lexed_once_on_comma(body) else {
            return Ok(None);
        };
        let Some(effects) = parse_body(tail, x, y)? else {
            return Ok(None);
        };
        let predicate =
            crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&head[1..])?;
        return Ok(Some(vec![EffectAst::Conditionals(
            ConditionalEffectAst::Conditional {
                predicate,
                if_true: effects,
                if_false: Vec::new(),
            },
        )]));
    }
    let Some(get) = body
        .iter()
        .position(|token| token.is_any_word(&["get", "gets"]))
    else {
        return Ok(None);
    };
    let Some(modifier) = body.get(get + 1) else {
        return Ok(None);
    };
    let Ok((power, toughness)) =
        crate::keyword_static::split_pt_modifier_components(modifier.parser_text())
    else {
        return Ok(None);
    };
    // This bounded reader requires an explicit duration and no discarded
    // ability grant/compound tail; the regular pump reader handles its scope.
    let tail = crate::lexer::token_word_refs(&body[get + 2..]);
    if tail != ["until", "end", "of", "turn"] {
        return Ok(None);
    }
    let component = |text: &str| {
        let text = text.trim();
        let (sign, variable) =
            if let Some(rest) = text.strip_prefix('-').or_else(|| text.strip_prefix('−')) {
                (-1, rest)
            } else {
                (1, text.strip_prefix('+').unwrap_or(text))
            };
        let value = match variable.to_ascii_lowercase().as_str() {
            "x" => x.clone(),
            "y" => y.clone(),
            _ => return None,
        };
        Some(if sign == 1 {
            value
        } else {
            Value::Scaled(Box::new(value), sign)
        })
    };
    let (Some(power), Some(toughness)) = (component(power), component(toughness)) else {
        return Ok(None);
    };
    Ok(
        crate::effect_sentences::clause_dispatch::parse_get_pump_clause_with_bound_values(
            &body[..get],
            &body[get + 1..],
            body,
            Some((power, toughness)),
        )?
        .map(|effect| vec![effect]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{StatChangeActionAst, SubjectVerbActionAst};
    #[test]
    fn two_independent_bindings_keep_both_signs_and_duration() {
        let body =
            crate::lexer::lex_line("Target creature gets -X/+Y until end of turn", 0).unwrap();
        let binding = crate::lexer::lex_line("where X is 3 and Y is 5", 0).unwrap();
        let effects = parse(&body, &binding).unwrap().unwrap();
        let EffectAst::SubjectVerb(effect) = &effects[0] else {
            panic!("{effects:?}");
        };
        let SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
            power,
            toughness,
            duration,
            ..
        }) = &effect.action
        else {
            panic!("{effect:?}");
        };
        assert_eq!(power, &Value::Scaled(Box::new(Value::Fixed(3)), -1));
        assert_eq!(toughness, &Value::Fixed(5));
        assert_eq!(*duration, crate::effect::Until::EndOfTurn);
        let incomplete = crate::lexer::lex_line("where X is 3 and Y is unknown", 0).unwrap();
        assert!(parse(&body, &incomplete).unwrap().is_none());
    }
}
