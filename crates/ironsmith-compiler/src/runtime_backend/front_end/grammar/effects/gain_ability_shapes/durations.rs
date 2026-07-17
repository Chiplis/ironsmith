use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::ConditionExpr;
use crate::effect::Until;
use crate::runtime_backend::front_end::grammar::primitives::WordSliceInput;
use crate::runtime_backend::front_end::grammar::{filters, primitives};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GainAbilityDurationShape {
    pub(crate) start: usize,
    pub(crate) len: usize,
    pub(crate) duration: Until,
    pub(crate) condition: Option<ConditionExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LeadingGainDurationShape {
    pub(crate) consumed_words: usize,
    pub(crate) duration: Until,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct QuotedGainDurationShape {
    pub(crate) close_quote_token: usize,
    pub(crate) duration: Until,
}

fn until_end_of_turn(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("end"),
        primitives::word_slice_exact("of"),
        primitives::word_slice_exact("turn"),
    )
        .value(Until::EndOfTurn)
        .parse_next(input)
}

fn until_next_turn(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        primitives::word_slice_exact("until"),
        primitives::word_slice_exact("your"),
        primitives::word_slice_exact("next"),
        alt((
            primitives::word_slice_exact("turn"),
            primitives::word_slice_exact("upkeep"),
        )),
    )
        .value(Until::YourNextTurn)
        .parse_next(input)
}

fn next_untap_step(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    (
        alt((
            primitives::word_slice_exact("until"),
            primitives::word_slice_exact("during"),
        )),
        primitives::word_slice_exact("your"),
        primitives::word_slice_exact("next"),
        primitives::word_slice_exact("untap"),
        primitives::word_slice_exact("step"),
    )
        .value(Until::YourNextTurn)
        .parse_next(input)
}

fn simple_turn_duration(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    alt((until_end_of_turn, until_next_turn, next_untap_step)).parse_next(input)
}

fn for_as_long_as(input: &mut WordSliceInput<'_>) -> WResult<()> {
    (
        primitives::word_slice_exact("for"),
        primitives::word_slice_exact("as"),
        primitives::word_slice_exact("long"),
        primitives::word_slice_exact("as"),
    )
        .void()
        .parse_next(input)
}

fn affected_object_counter_duration_lexed(input: &mut LexStream<'_>) -> WResult<Until> {
    primitives::phrase(&["for", "as", "long", "as"]).parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        (
            alt((primitives::kw("that"), primitives::kw("this"))),
            alt((
                primitives::kw("artifact"),
                primitives::kw("creature"),
                primitives::kw("enchantment"),
                primitives::kw("land"),
                primitives::kw("permanent"),
            )),
        )
            .void(),
    ))
    .parse_next(input)?;
    primitives::kw("has").parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let counter_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((primitives::kw("counter"), primitives::kw("counters")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("counter"), primitives::kw("counters"))).parse_next(input)?;
    primitives::phrase(&["on", "it"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;

    let counter_type =
        filters::parse_counter_type_from_tokens(counter_tokens).ok_or_else(|| {
            primitives::backtrack_err("counter-linked duration", "known counter type")
        })?;
    Ok(Until::ForAsLongAs(
        ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(counter_type),
    ))
}

/// Parse an authored leading duration such as
/// `for as long as that permanent has a charge counter on it, ...`.
///
/// The surrounding gain-ability parser uses the returned word count to start
/// verb recognition after the condition, so the condition's own `has` cannot
/// be mistaken for the grant verb.
pub(crate) fn parse_leading_affected_object_counter_duration_shape(
    tokens: &[OwnedLexToken],
) -> Option<LeadingGainDurationShape> {
    let (duration, rest) =
        primitives::parse_prefix(tokens, affected_object_counter_duration_lexed)?;
    if rest.is_empty() {
        return None;
    }
    let consumed_tokens = tokens.len().checked_sub(rest.len())?;
    Some(LeadingGainDurationShape {
        consumed_words: TokenWordView::new(&tokens[..consumed_tokens]).len(),
        duration,
    })
}

fn source_remains_on_battlefield(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    for_as_long_as.parse_next(input)?;
    alt((
        (
            alt((
                primitives::word_slice_exact("this"),
                primitives::word_slice_exact("thiss"),
            )),
            opt(alt((
                primitives::word_slice_exact("artifact"),
                primitives::word_slice_exact("creature"),
                primitives::word_slice_exact("enchantment"),
                primitives::word_slice_exact("permanent"),
                primitives::word_slice_exact("source"),
            ))),
        )
            .void(),
        primitives::word_slice_exact("source").void(),
    ))
    .parse_next(input)?;
    alt((
        primitives::word_slice_exact("remain"),
        primitives::word_slice_exact("remains"),
    ))
    .parse_next(input)?;
    primitives::word_slice_exact("on").parse_next(input)?;
    opt(primitives::word_slice_exact("the")).parse_next(input)?;
    primitives::word_slice_exact("battlefield").parse_next(input)?;
    Ok(Until::ThisLeavesTheBattlefield)
}

fn continuous_duration(input: &mut WordSliceInput<'_>) -> WResult<Until> {
    alt((simple_turn_duration, source_remains_on_battlefield)).parse_next(input)
}

fn you_control(input: &mut WordSliceInput<'_>) -> WResult<()> {
    (
        for_as_long_as,
        primitives::word_slice_exact("you"),
        primitives::word_slice_exact("control"),
    )
        .void()
        .parse_next(input)
}

fn source_tapped_duration<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "as", "long", "as"]).parse_next(input)?;
    primitives::kw("this").parse_next(input)?;
    opt(alt((
        primitives::kw("creature"),
        primitives::kw("permanent"),
        primitives::kw("source"),
    )))
    .parse_next(input)?;
    alt((primitives::kw("remains"), primitives::kw("is"))).parse_next(input)?;
    primitives::kw("tapped").parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

/// Parse the duration as a typed suffix over lexer tokens. The returned word
/// span is only a boundary for the surrounding gain-ability parser; semantic
/// recognition is wholly owned by the Winnow grammar above.
pub(crate) fn parse_source_tapped_gain_duration_shape(
    tokens: &[OwnedLexToken],
) -> Option<GainAbilityDurationShape> {
    let (start_token, (), rest) = primitives::find_prefix(tokens, || source_tapped_duration)?;
    if !rest.is_empty() {
        return None;
    }
    let start = TokenWordView::new(&tokens[..start_token]).len();
    let len = TokenWordView::new(&tokens[start_token..]).len();
    Some(GainAbilityDurationShape {
        start,
        len,
        duration: Until::SourceUntaps,
        condition: Some(ConditionExpr::SourceIsTapped),
    })
}

pub(crate) fn parse_simple_ability_duration_shape(
    words: &[&str],
) -> Option<GainAbilityDurationShape> {
    let mut start = 0usize;
    while start < words.len() {
        let mut input = &words[start..];
        if let Ok(duration) = continuous_duration.parse_next(&mut input) {
            return Some(GainAbilityDurationShape {
                start,
                len: words[start..].len().saturating_sub(input.len()),
                duration,
                condition: None,
            });
        }
        let mut input = &words[start..];
        if you_control.parse_next(&mut input).is_ok() {
            return Some(GainAbilityDurationShape {
                start,
                len: words.len().saturating_sub(start),
                duration: Until::YouStopControllingThis,
                condition: None,
            });
        }
        start += 1;
    }
    None
}

pub(crate) fn parse_gain_ability_duration_shape(
    words: &[&str],
) -> Option<GainAbilityDurationShape> {
    parse_simple_ability_duration_shape(words)
}

pub(crate) fn parse_leading_gain_duration_shape(
    words: &[&str],
) -> Option<LeadingGainDurationShape> {
    let mut input: WordSliceInput<'_> = words;
    let duration = continuous_duration.parse_next(&mut input).ok()?;
    Some(LeadingGainDurationShape {
        consumed_words: words.len().saturating_sub(input.len()),
        duration,
    })
}

pub(crate) fn parse_quoted_gain_duration_shape(
    tokens: &[OwnedLexToken],
    gain_token_idx: usize,
) -> Option<QuotedGainDurationShape> {
    let open_quote_idx = gain_token_idx.checked_add(1)?;
    primitives::parse_prefix(tokens.get(open_quote_idx..)?, primitives::quote())?;
    let after_open = tokens.get(open_quote_idx + 1..)?;
    let (relative_close, _, _) = primitives::find_prefix(after_open, primitives::quote)?;
    let close_quote_token = open_quote_idx + 1 + relative_close;
    let tail_tokens = trim_lexed_commas(tokens.get(close_quote_token + 1..)?);
    if tail_tokens.is_empty() {
        return None;
    }
    let tail_words = TokenWordView::new(tail_tokens).to_word_refs();
    let parsed = parse_simple_ability_duration_shape(&tail_words)?;
    if parsed.start != 0 || parsed.len != tail_words.len() {
        return None;
    }
    Some(QuotedGainDurationShape {
        close_quote_token,
        duration: parsed.duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_turn_conditional_and_quoted_durations() {
        let duration =
            parse_simple_ability_duration_shape(&["flying", "until", "your", "next", "upkeep"])
                .unwrap();
        assert_eq!(duration.start, 1);
        assert_eq!(duration.len, 4);
        assert_eq!(duration.duration, Until::YourNextTurn);

        let source_lifetime = parse_simple_ability_duration_shape(&[
            "all",
            "abilities",
            "for",
            "as",
            "long",
            "as",
            "this",
            "creature",
            "remains",
            "on",
            "the",
            "battlefield",
        ])
        .unwrap();
        assert_eq!(source_lifetime.start, 2);
        assert_eq!(source_lifetime.duration, Until::ThisLeavesTheBattlefield);

        let tapped_tokens =
            lex_line("flying for as long as this creature remains tapped.", 0).unwrap();
        let tapped = parse_source_tapped_gain_duration_shape(&tapped_tokens).unwrap();
        assert_eq!(tapped.start, 1);
        assert_eq!(tapped.duration, Until::SourceUntaps);
        assert_eq!(tapped.condition, Some(ConditionExpr::SourceIsTapped));

        let near_miss =
            lex_line("flying for as long as this creature remains untapped.", 0).unwrap();
        assert!(parse_source_tapped_gain_duration_shape(&near_miss).is_none());

        let tokens = lex_line(
            "Target creature gains \"Whenever it attacks, draw a card.\" until end of turn.",
            0,
        )
        .unwrap();
        let gain_token = primitives::find_prefix(&tokens, || primitives::kw("gains"))
            .map(|(offset, _, _)| offset)
            .unwrap();
        assert_eq!(
            parse_quoted_gain_duration_shape(&tokens, gain_token)
                .unwrap()
                .duration,
            Until::EndOfTurn
        );
    }

    #[test]
    fn parses_leading_affected_object_counter_duration_before_real_grant_verb() {
        let tokens = lex_line(
            "For as long as that creature has a bounty counter on it, it has \"When this creature dies, draw a card.\"",
            0,
        )
        .unwrap();
        let shape = parse_leading_affected_object_counter_duration_shape(&tokens)
            .expect("counter-linked grant duration should parse");

        assert_eq!(shape.consumed_words, 12);
        assert_eq!(
            shape.duration,
            Until::ForAsLongAs(
                ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                    crate::object::CounterType::Named("bounty")
                )
            )
        );

        let normalized_without_comma = lex_line(
            "For as long as that land has a blaze counter on it it has \"At the beginning of your upkeep, this land deals 1 damage to you.\"",
            0,
        )
        .unwrap();
        assert!(
            parse_leading_affected_object_counter_duration_shape(&normalized_without_comma)
                .is_some(),
            "multi-sentence normalization may omit the duration comma"
        );
    }
}
