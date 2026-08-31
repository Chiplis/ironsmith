use super::*;

use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TapThenReturnShape<'a> {
    pub tap_tokens: &'a [OwnedLexToken],
    pub return_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TapOrUntapAllShape<'a> {
    pub tap_filter_tokens: &'a [OwnedLexToken],
    pub untap_filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapControlRelation {
    TargetPlayer,
    ThatPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TapTypeChoiceShape<'a> {
    pub before_tokens: &'a [OwnedLexToken],
    pub after_tokens: &'a [OwnedLexToken],
}

fn tap_quantifier<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("all"), primitives::kw("each")))
        .void()
        .parse_next(input)
}

fn parse_tap_or_untap_all_lexed<'a>(input: &mut LexStream<'a>) -> WResult<TapOrUntapAllShape<'a>> {
    tap_quantifier.parse_next(input)?;
    let tap_filter_tokens = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["or", "untap", "all"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["or", "untap", "all"]).parse_next(input)?;
    let untap_filter_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(TapOrUntapAllShape {
        tap_filter_tokens: super::super::super::lexer::trim_lexed_commas(tap_filter_tokens),
        untap_filter_tokens: super::super::super::lexer::trim_lexed_commas(untap_filter_tokens),
    })
}

pub fn parse_tap_or_untap_all_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TapOrUntapAllShape<'_>> {
    crate::grammar::primitives::probe_all(tokens, parse_tap_or_untap_all_lexed, "tap or untap all")
}

fn parse_tap_quantified_filter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    tap_quantifier.parse_next(input)?;
    let filter_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(super::super::super::lexer::trim_lexed_commas(filter_tokens))
}

pub fn parse_tap_quantified_filter_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_tap_quantified_filter_lexed,
        "quantified tap filter",
    )
}

fn parse_tap_or_untap_target_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    primitives::phrase(&["or", "untap"]).parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(super::super::super::lexer::trim_lexed_commas(target_tokens))
}

pub fn parse_tap_or_untap_target_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_tap_or_untap_target_lexed,
        "tap or untap target",
    )
}

/// A coordinated tap followed by another effect, for example
/// "Tap this creature and all Goblins, then ...".
///
/// Keeping the two tap operands together here is important: a bare `and`
/// between object phrases is not an effect boundary.  The semantic layer can
/// lower the two captured operands to one non-target object union and preserve
/// the result as the antecedent of the `then` clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TapObjectUnionThenShape<'a> {
    pub first_target_tokens: &'a [OwnedLexToken],
    pub all_filter_tokens: &'a [OwnedLexToken],
    pub followup_tokens: &'a [OwnedLexToken],
}

fn parse_tap_object_union_then_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TapObjectUnionThenShape<'a>> {
    primitives::kw("tap").parse_next(input)?;
    let first_target_tokens = repeat_till(
        1..,
        any.void(),
        peek((primitives::kw("and"), primitives::kw("all"))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["and", "all"]).parse_next(input)?;
    let all_filter_tokens = repeat_till(
        1..,
        any.void(),
        peek((opt(primitives::comma()), primitives::kw("then"))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::kw("then").parse_next(input)?;
    let followup_tokens = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(TapObjectUnionThenShape {
        first_target_tokens: super::super::super::lexer::trim_lexed_commas(first_target_tokens),
        all_filter_tokens: super::super::super::lexer::trim_lexed_commas(all_filter_tokens),
        followup_tokens: super::super::super::lexer::trim_lexed_commas(followup_tokens),
    })
}

pub fn parse_tap_object_union_then_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TapObjectUnionThenShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_tap_object_union_then_lexed,
        "tap object union then followup",
    )
}

fn parse_tap_then_return_lexed<'a>(input: &mut LexStream<'a>) -> WResult<TapThenReturnShape<'a>> {
    let tap_tokens = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["then", "return"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::kw("then").parse_next(input)?;
    let return_tokens = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(TapThenReturnShape {
        tap_tokens,
        return_tokens,
    })
}

pub fn parse_tap_then_return_tokens(tokens: &[OwnedLexToken]) -> Option<TapThenReturnShape<'_>> {
    crate::grammar::primitives::probe_all(tokens, parse_tap_then_return_lexed, "tap-then-return")
}

#[path = "tap_shapes/control_and_type.rs"]
mod control_and_type;
pub use control_and_type::*;

#[cfg(test)]
#[path = "tap_shapes/tests.rs"]
mod tests;
