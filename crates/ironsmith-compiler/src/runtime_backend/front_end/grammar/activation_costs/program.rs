use winnow::combinator::{alt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::CardTextError;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;

#[cfg(test)]
use super::super::super::lexer::lex_line;
use super::super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, render_token_slice, token_slice_at_is,
    token_slice_first_is,
};
use super::super::super::token_primitives::locate_index as locate_token_index;
use super::super::super::util::is_source_reference_words;
use super::super::activated_lines::{
    ActivatedLoyaltyShorthand, parse_loyalty_shorthand_activation_tokens,
};
use super::super::keyword_action_costs::parse_payment_alternative_split_tokens;
use super::super::primitives;
use super::super::values::parse_mana_cost_tokens;
use super::{
    ActivationCostCst, ActivationCostSegmentCst, ActivationCostSegmentKind,
    is_tap_activation_symbol_token, parse_activation_cost_segment_kind_tokens,
    parse_bare_symbol_segment_tokens, parse_behold_segment_tokens, parse_blight_segment_tokens,
    parse_discard_segment_tokens, parse_exert_segment_tokens,
    parse_exile_segment_tokens as parse_typed_exile_segment_tokens, parse_mill_segment_tokens,
    parse_pay_segment_tokens, parse_put_counter_segment_tokens,
    parse_remove_counter_segment_tokens, parse_return_segment_tokens, parse_reveal_segment_tokens,
    parse_sacrifice_segment_tokens as parse_typed_sacrifice_segment_tokens,
    parse_tap_chosen_segment_tokens, parse_unattach_segment_tokens,
};

fn first_non_comma_token_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_comma() {
            return Some(idx);
        }
    }
    None
}

fn trim_activation_cost_segment_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = first_non_comma_token_index(tokens).unwrap_or(tokens.len());
    let mut end = tokens.len();

    if token_slice_at_is(tokens, start, "and") {
        start += 1;
        while start < end && tokens[start].is_comma() {
            start += 1;
        }
    }

    if token_slice_at_is(tokens, start, "waterbend") {
        start += 1;
        while start < end && tokens[start].is_comma() {
            start += 1;
        }
    }

    while end > start && (tokens[end - 1].is_period() || tokens[end - 1].is_comma()) {
        end -= 1;
    }

    &tokens[start..end]
}

fn render_trimmed_lexed_tokens(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens).trim().to_string()
}

fn activation_cost_prefix_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if let Some(colon_idx) = locate_token_index(tokens, OwnedLexToken::is_colon) {
        &tokens[..colon_idx]
    } else {
        tokens
    }
}

fn parse_loyalty_shorthand_activation_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<ActivationCostSegmentCst>> {
    let tokens = trim_activation_cost_segment_tokens(activation_cost_prefix_tokens(tokens));
    match parse_loyalty_shorthand_activation_tokens(tokens)? {
        ActivatedLoyaltyShorthand::Add(0) => Some(Vec::new()),
        ActivatedLoyaltyShorthand::Add(count) => {
            Some(vec![ActivationCostSegmentCst::PutCounters {
                counter_type: CounterType::Loyalty,
                count,
            }])
        }
        ActivatedLoyaltyShorthand::Remove(count) => {
            Some(vec![ActivationCostSegmentCst::RemoveCounters {
                counter_type: CounterType::Loyalty,
                count,
            }])
        }
        ActivatedLoyaltyShorthand::RemoveX => {
            Some(vec![ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type: Some(CounterType::Loyalty),
                display_x: true,
                remove_all: false,
            }])
        }
    }
}

fn parse_activation_cost_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Result<ActivationCostSegmentCst, CardTextError>> {
    match parse_activation_cost_segment_kind_tokens(tokens) {
        ActivationCostSegmentKind::Pay => Some(parse_pay_segment_tokens(tokens)),
        ActivationCostSegmentKind::Discard => Some(parse_discard_segment_tokens(tokens)),
        ActivationCostSegmentKind::Mill => Some(parse_mill_segment_tokens(tokens)),
        ActivationCostSegmentKind::Sacrifice => Some(parse_typed_sacrifice_segment_tokens(
            tokens,
            is_source_reference_words,
        )),
        ActivationCostSegmentKind::Unattach => Some(parse_unattach_segment_tokens(
            tokens,
            is_source_reference_words,
        )),
        ActivationCostSegmentKind::TapChosen => Some(parse_tap_chosen_segment_tokens(tokens)),
        ActivationCostSegmentKind::Behold => Some(parse_behold_segment_tokens(tokens)),
        ActivationCostSegmentKind::Blight => Some(parse_blight_segment_tokens(tokens)),
        ActivationCostSegmentKind::Exile => Some(parse_typed_exile_segment_tokens(
            tokens,
            is_source_reference_words,
        )),
        ActivationCostSegmentKind::Reveal => Some(parse_reveal_segment_tokens(tokens)),
        ActivationCostSegmentKind::Return => Some(parse_return_segment_tokens(tokens)),
        ActivationCostSegmentKind::Exert => Some(parse_exert_segment_tokens(tokens)),
        ActivationCostSegmentKind::PutCounter => Some(parse_put_counter_segment_tokens(tokens)),
        ActivationCostSegmentKind::RemoveCounter => {
            Some(parse_remove_counter_segment_tokens(tokens))
        }
        ActivationCostSegmentKind::BareSymbol => parse_bare_symbol_segment_tokens(tokens).map(Ok),
    }
}

fn parse_shard_style_branch_tokens(tokens: &[OwnedLexToken]) -> Option<ManaSymbol> {
    let tokens = trim_activation_cost_segment_tokens(tokens);
    let comma_idx = locate_token_index(tokens, OwnedLexToken::is_comma)?;
    let mana_tokens = trim_activation_cost_segment_tokens(&tokens[..comma_idx]);
    let tap_tokens = trim_activation_cost_segment_tokens(&tokens[comma_idx + 1..]);
    if tap_tokens.len() != 1 || tap_tokens[0].kind != TokenKind::ManaGroup {
        return None;
    }
    if !is_tap_activation_symbol_token(&tap_tokens[0]) {
        return None;
    }

    let mana_cost = parse_mana_cost_tokens(mana_tokens).ok()?;
    let [pip] = mana_cost.pips() else {
        return None;
    };
    let [symbol] = pip.as_slice() else {
        return None;
    };
    Some(*symbol)
}

fn parse_shard_style_mana_or_tap_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(ManaSymbol, ManaSymbol)> {
    let tokens = trim_activation_cost_segment_tokens(activation_cost_prefix_tokens(tokens));
    let or_idx = locate_token_index(tokens, |token| token.is_word("or"))?;
    let left = parse_shard_style_branch_tokens(&tokens[..or_idx])?;
    let right = parse_shard_style_branch_tokens(&tokens[or_idx + 1..])?;
    Some((left, right))
}

fn starts_new_activation_cost_segment_tokens(tokens: &[OwnedLexToken]) -> bool {
    let mut input = LexStream::new(tokens);
    parse_activation_cost_segment_head_lexed
        .parse_next(&mut input)
        .is_ok()
}

fn parse_activation_cost_segment_head_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., primitives::comma().void()).parse_next(input)?;
    alt((
        alt((
            primitives::token_kind(TokenKind::ManaGroup),
            primitives::token_kind(TokenKind::Number),
            primitives::token_kind(TokenKind::Plus),
            primitives::token_kind(TokenKind::Dash),
        ))
        .void(),
        alt((
            alt((
                primitives::kw("tap"),
                primitives::kw("t"),
                primitives::kw("untap"),
                primitives::kw("q"),
                primitives::kw("pay"),
                primitives::kw("discard"),
                primitives::kw("mill"),
                primitives::kw("sacrifice"),
                primitives::kw("unattach"),
            ))
            .void(),
            alt((
                alt((
                    primitives::kw("exile"),
                    primitives::kw("return"),
                    primitives::kw("put"),
                    primitives::kw("remove"),
                    primitives::kw("behold"),
                ))
                .void(),
                alt((
                    primitives::kw("exert"),
                    primitives::kw("reveal"),
                    primitives::kw("waterbend"),
                    primitives::kw("e"),
                    primitives::kw("and"),
                ))
                .void(),
            ))
            .void(),
        ))
        .void(),
    ))
    .parse_next(input)
}

fn split_activation_cost_segments_tokens(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut inside_named_card = false;
    let mut idx = 0usize;

    while idx < tokens.len() {
        if !inside_named_card
            && tokens[idx].is_word("card")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("named"))
        {
            inside_named_card = true;
        }

        let split_here = if tokens[idx].is_comma() {
            let remainder = &tokens[idx + 1..];
            let remainder = if token_slice_first_is(remainder, "and") {
                &remainder[1..]
            } else {
                remainder
            };
            starts_new_activation_cost_segment_tokens(remainder)
        } else if tokens[idx].is_word("and") && idx > start {
            let remainder = &tokens[idx + 1..];
            !inside_named_card && starts_new_activation_cost_segment_tokens(remainder)
        } else {
            false
        };

        if split_here {
            let segment = tokens[start..idx].to_vec();
            if !segment.is_empty() {
                segments.push(segment);
            }
            start = idx + 1;
            inside_named_card = false;
        }

        idx += 1;
    }

    let tail = tokens[start..].to_vec();
    if !tail.is_empty() {
        segments.push(tail);
    }

    segments
}

fn parse_activation_cost_cst_tokens(
    tokens: &[OwnedLexToken],
    raw: &str,
) -> Result<ActivationCostCst, CardTextError> {
    let trimmed_raw = raw.trim();
    if let Some(segments) = parse_loyalty_shorthand_activation_cost_tokens(tokens) {
        return Ok(ActivationCostCst {
            raw: trimmed_raw.to_string(),
            segments,
            alternative_branches: Vec::new(),
            is_loyalty_shorthand: true,
        });
    }

    if let Some((left, right)) = parse_shard_style_mana_or_tap_cost_tokens(tokens) {
        return Ok(ActivationCostCst {
            raw: trimmed_raw.to_string(),
            segments: vec![
                ActivationCostSegmentCst::Mana(ManaCost::from_pips(vec![vec![left, right]])),
                ActivationCostSegmentCst::Tap,
            ],
            alternative_branches: Vec::new(),
            is_loyalty_shorthand: false,
        });
    }

    if let Some(split) = parse_payment_alternative_split_tokens(tokens) {
        let left_tokens = trim_activation_cost_segment_tokens(&tokens[..split.delimiter]);
        let right_tokens = trim_activation_cost_segment_tokens(&tokens[split.delimiter + 1..]);
        if !left_tokens.is_empty() && !right_tokens.is_empty() {
            let left_raw = render_trimmed_lexed_tokens(left_tokens);
            let right_raw = render_trimmed_lexed_tokens(right_tokens);
            if let (Ok(left), Ok(right)) = (
                parse_activation_cost_cst_tokens(left_tokens, &left_raw),
                parse_activation_cost_cst_tokens(right_tokens, &right_raw),
            ) {
                return Ok(ActivationCostCst {
                    raw: trimmed_raw.to_string(),
                    segments: Vec::new(),
                    alternative_branches: vec![left, right],
                    is_loyalty_shorthand: false,
                });
            }
        }
    }

    let mut segments = Vec::new();
    for segment_tokens in split_activation_cost_segments_tokens(tokens) {
        let segment_tokens = trim_activation_cost_segment_tokens(&segment_tokens);
        if segment_tokens.is_empty() {
            continue;
        }

        let segment = render_trimmed_lexed_tokens(segment_tokens);
        let parsed = parse_activation_cost_segment_tokens(segment_tokens)
            .unwrap_or_else(|| {
                Err(CardTextError::ParseError(format!(
                    "rewrite activation-cost segment parser does not yet support '{segment}'",
                )))
            })
            .map_err(|err| {
                CardTextError::ParseError(format!(
                    "unsupported activation cost segment (clause: '{}'): {err}",
                    segment,
                ))
            })?;
        segments.push(parsed);
    }

    if segments.is_empty() {
        return Err(CardTextError::ParseError(
            "rewrite activation-cost parser found no segments".to_string(),
        ));
    }

    Ok(ActivationCostCst {
        raw: trimmed_raw.to_string(),
        segments,
        alternative_branches: Vec::new(),
        is_loyalty_shorthand: false,
    })
}

pub(crate) fn parse_activation_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostCst, CardTextError> {
    parse_activation_cost_cst_tokens(tokens, &render_token_slice(tokens))
}

#[cfg(test)]
pub(crate) fn parse_activation_cost_tokens_rewrite(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostCst, CardTextError> {
    parse_activation_cost_tokens(tokens)
}

#[cfg(test)]
pub(crate) fn parse_activation_cost_rewrite(raw: &str) -> Result<ActivationCostCst, CardTextError> {
    let tokens = lex_line(raw.trim(), 0)?;
    parse_activation_cost_cst_tokens(&tokens, raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> ActivationCostCst {
        let tokens = lex_line(raw, 0).expect("activation-cost test surface should lex");
        parse_activation_cost_tokens(&tokens)
            .expect("activation-cost grammar should own the test surface")
    }

    #[test]
    fn program_owns_loyalty_shorthand_and_shard_style_costs() {
        let loyalty = parse("[-X]");
        assert!(loyalty.is_loyalty_shorthand);
        assert!(matches!(
            loyalty.segments.as_slice(),
            [ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type: Some(CounterType::Loyalty),
                display_x: true,
                remove_all: false,
            }]
        ));

        let shard = parse("{W}, {T} or {U}, {T}");
        assert!(matches!(
            shard.segments.as_slice(),
            [
                ActivationCostSegmentCst::Mana(_),
                ActivationCostSegmentCst::Tap
            ]
        ));
    }

    #[test]
    fn program_preserves_named_card_commas_and_payment_alternatives() {
        let composite =
            parse("Discard a card named Mishra, Lost to Phyrexia, sacrifice a creature");
        assert_eq!(composite.segments.len(), 2);
        assert!(composite.alternative_branches.is_empty());

        let alternative = parse("Pay {3} or discard a card");
        assert!(alternative.segments.is_empty());
        assert_eq!(alternative.alternative_branches.len(), 2);
    }

    #[test]
    fn token_and_raw_test_entrypoints_share_the_typed_program() {
        let raw = "{2}, {T}, sacrifice another creature";
        let tokens = lex_line(raw, 0).unwrap();
        assert_eq!(
            parse_activation_cost_tokens_rewrite(&tokens).unwrap(),
            parse_activation_cost_rewrite(raw).unwrap()
        );
    }
}
