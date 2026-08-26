use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedCounterTargetShape<'a> {
    pub descriptors: Vec<CounterDescriptorShape>,
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterFollowupShape<'a> {
    pub counter_tokens: &'a [OwnedLexToken],
    pub followup_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterPairShape<'a> {
    pub first_tokens: &'a [OwnedLexToken],
    pub second_tokens: &'a [OwnedLexToken],
}

fn descriptor_separator<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (primitives::comma(), opt(primitives::kw("and"))).void(),
        primitives::kw("and").void(),
    ))
    .parse_next(input)
}

fn parse_shared_counter_target_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SharedCounterTargetShape<'a>> {
    primitives::kw("put").parse_next(input)?;
    let descriptors =
        separated(2.., parse_counter_descriptor_lexed, descriptor_separator).parse_next(input)?;
    primitives::kw("on").parse_next(input)?;
    let target_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(SharedCounterTargetShape {
        descriptors,
        target_tokens,
    })
}

pub fn parse_shared_counter_target_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SharedCounterTargetShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_shared_counter_target_lexed,
        "shared counter target",
    )
    .ok()
}

fn parse_counter_followup_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CounterFollowupShape<'a>> {
    primitives::kw("put").parse_next(input)?;
    let counter_tokens = repeat_till(1.., any.void(), peek(primitives::phrase(&["and", "it"])))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::phrase(&["and", "it"]).parse_next(input)?;
    let followup_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterFollowupShape {
        counter_tokens,
        followup_tokens,
    })
}

pub fn parse_counter_followup_tokens(tokens: &[OwnedLexToken]) -> Option<CounterFollowupShape<'_>> {
    primitives::parse_all(tokens, parse_counter_followup_lexed, "counter followup").ok()
}

fn parse_counter_pair_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CounterPairShape<'a>> {
    primitives::kw("put").parse_next(input)?;
    let first_tokens = repeat_till(1.., any.void(), peek(primitives::kw("and")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let second_tokens = repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(CounterPairShape {
        first_tokens,
        second_tokens,
    })
}

pub fn parse_counter_pair_tokens(tokens: &[OwnedLexToken]) -> Option<CounterPairShape<'_>> {
    primitives::parse_all(tokens, parse_counter_pair_lexed, "counter pair").ok()
}
