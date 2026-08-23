use super::*;
use winnow::combinator::{alt, eof, opt};
use winnow::token::take_till;

#[derive(Debug, Clone, PartialEq)]
pub struct SerialDamageFanout {
    pub source: Vec<OwnedLexToken>,
    pub parts: Vec<SerialDamagePart>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SerialDamagePart {
    pub amount: Value,
    pub target_tokens: Vec<OwnedLexToken>,
}

pub fn parse_serial_damage_fanout_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<SerialDamageFanout>, CardTextError> {
    if let Some(serial) = primitives::parse_all_or_none(
        tokens,
        parse_serial_damage_fanout_lexed,
        "serial-damage-fanout",
    )? {
        return Ok(Some(serial));
    }
    Ok(parse_paired_damage_fanout_tokens(tokens))
}

fn trim_damage_part_tokens(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Comma))
    {
        tokens = &tokens[1..];
    }
    while tokens
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period))
    {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

fn parse_postpositive_rounded_half_damage_head(
    tokens: &[OwnedLexToken],
) -> Option<(Value, &[OwnedLexToken])> {
    if !tokens.first().is_some_and(|token| token.is_word("half")) {
        return None;
    }
    let damage_idx = tokens
        .iter()
        .position(|token| token.is_word("damage"))
        .filter(|idx| *idx > 1)?;
    let base_tokens = &tokens[1..damage_idx];
    let (base, used) =
        super::super::shared_util::value_semantics::parse_value_prefix_lexed(base_tokens)?;
    if used != base_tokens.len() {
        return None;
    }

    let mut idx = damage_idx + 1;
    if tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| token.is_word("rounded"))
    {
        return None;
    }
    let rounded_up = match tokens.get(idx + 1).and_then(OwnedLexToken::as_word) {
        Some("up") => true,
        Some("down") => false,
        _ => return None,
    };
    idx += 2;
    if tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("to")) {
        idx += 1;
    }

    let amount = if rounded_up {
        Value::HalfRoundedDown(Box::new(Value::Add(
            Box::new(base),
            Box::new(Value::Fixed(1)),
        )))
    } else {
        Value::HalfRoundedDown(Box::new(base))
    };
    Some((amount, &tokens[idx..]))
}

fn parse_damage_amount_head(tokens: &[OwnedLexToken]) -> Option<(Value, &[OwnedLexToken])> {
    if let Some(parsed) = parse_postpositive_rounded_half_damage_head(tokens) {
        return Some(parsed);
    }
    let (amount, used) =
        super::super::shared_util::value_semantics::parse_value_prefix_lexed(tokens)?;
    let (_, rest) = primitives::parse_prefix(
        tokens.get(used..)?,
        (primitives::kw("damage"), opt(primitives::kw("to"))).void(),
    )?;
    Some((amount, rest))
}

/// Parses the common two-recipient surface
/// "SOURCE deals N damage to TARGET and M damage to TARGET". The separator
/// is recognized structurally only when its tail starts with a complete
/// damage amount/head, so `and` inside a target filter is never a split.
fn parse_paired_damage_fanout_tokens(tokens: &[OwnedLexToken]) -> Option<SerialDamageFanout> {
    let tokens = trim_damage_part_tokens(tokens);
    let (deal_idx, (), after_deal) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("deal"), primitives::kw("deals"))).void()
    })?;
    let source = trim_damage_part_tokens(&tokens[..deal_idx]).to_vec();
    let (first_amount, first_target_and_tail) = parse_damage_amount_head(after_deal)?;

    for and_idx in 0..first_target_and_tail.len() {
        if first_target_and_tail[and_idx].as_word() != Some("and") {
            continue;
        }
        let first_target = trim_damage_part_tokens(&first_target_and_tail[..and_idx]);
        let Some((second_amount, second_target)) =
            parse_damage_amount_head(&first_target_and_tail[and_idx + 1..])
        else {
            continue;
        };
        let second_target = trim_damage_part_tokens(second_target);
        let second_target = second_target
            .iter()
            .position(|token| token.is_word("where"))
            .map_or(second_target, |where_idx| {
                trim_damage_part_tokens(&second_target[..where_idx])
            });
        if first_target.is_empty() || second_target.is_empty() {
            continue;
        }
        return Some(SerialDamageFanout {
            source,
            parts: vec![
                SerialDamagePart {
                    amount: first_amount,
                    target_tokens: first_target.to_vec(),
                },
                SerialDamagePart {
                    amount: second_amount,
                    target_tokens: second_target.to_vec(),
                },
            ],
        });
    }
    None
}

fn parse_serial_damage_fanout_lexed<'a>(
    input: &mut LexStream<'a>,
) -> Result<SerialDamageFanout, ErrMode<ContextError>> {
    let source = take_till(0.., |token: &OwnedLexToken| {
        token.is_any_word(&["deal", "deals"])
    })
    .parse_next(input)?
    .to_vec();
    alt((primitives::kw("deal"), primitives::kw("deals")))
        .void()
        .parse_next(input)?;

    let first = parse_serial_damage_part_lexed.parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let second = parse_serial_damage_part_lexed.parse_next(input)?;
    primitives::comma().parse_next(input)?;
    opt(primitives::kw("and")).parse_next(input)?;
    let third = parse_serial_damage_part_lexed.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;

    Ok(SerialDamageFanout {
        source,
        parts: vec![first, second, third],
    })
}

fn parse_serial_damage_part_lexed<'a>(
    input: &mut LexStream<'a>,
) -> Result<SerialDamagePart, ErrMode<ContextError>> {
    let amount = super::super::leaf::parse_leaf_modal_value_token.parse_next(input)?;
    primitives::kw("damage").parse_next(input)?;
    opt(primitives::kw("to")).parse_next(input)?;
    let target_tokens = take_till(1.., |token: &OwnedLexToken| {
        matches!(token.kind, TokenKind::Comma | TokenKind::Period)
    })
    .parse_next(input)?
    .to_vec();

    Ok(SerialDamagePart {
        amount,
        target_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::TokenWordView;
    use crate::lexer::lex_line;

    #[test]
    fn paired_damage_split_requires_a_second_typed_damage_head() {
        let tokens = lex_line(
            "This spell deals 1 damage to target player or planeswalker and 1 damage to target creature that player or that planeswalker's controller controls.",
            0,
        )
        .unwrap();
        let shape = parse_serial_damage_fanout_tokens(&tokens)
            .unwrap()
            .expect("paired damage shape");
        assert_eq!(shape.parts.len(), 2);
        assert_eq!(shape.parts[0].amount, Value::Fixed(1));
        assert_eq!(shape.parts[1].amount, Value::Fixed(1));
        assert_eq!(
            TokenWordView::new(&shape.parts[1].target_tokens).to_word_refs(),
            [
                "target",
                "creature",
                "that",
                "player",
                "or",
                "that",
                "planeswalkers",
                "controller",
                "controls"
            ]
        );
    }

    #[test]
    fn paired_damage_accepts_postpositive_rounded_half_amount() {
        let tokens = lex_line(
            "Eternal Flame deals X damage to target opponent or planeswalker and half X damage, \
             rounded up, to you, where X is the number of Mountains you control.",
            0,
        )
        .unwrap();
        let shape = parse_serial_damage_fanout_tokens(&tokens)
            .unwrap()
            .expect("paired damage shape");

        assert_eq!(shape.parts.len(), 2);
        assert_eq!(shape.parts[0].amount, Value::X);
        assert!(matches!(
            &shape.parts[1].amount,
            Value::HalfRoundedDown(inner)
                if matches!(inner.as_ref(), Value::Add(left, right)
                    if matches!(left.as_ref(), Value::X)
                        && matches!(right.as_ref(), Value::Fixed(1)))
        ));
        assert_eq!(
            TokenWordView::new(&shape.parts[1].target_tokens).to_word_refs(),
            ["you"]
        );
    }
}
