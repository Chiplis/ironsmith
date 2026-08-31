use super::*;

pub fn parse_counted_card_target_shape(
    tokens: &[OwnedLexToken],
) -> Option<CountedCardTargetShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(tokens)?;
    let after_count = trim_lexed_commas(tokens.get(parsed.consumed..)?);
    let (_, _) = primitives::parse_prefix(
        after_count,
        alt((primitives::kw("card"), primitives::kw("cards"))).void(),
    )?;
    Some(CountedCardTargetShape {
        count: parsed.count,
        target_tokens: after_count,
    })
}

pub fn parse_counted_those_cards(tokens: &[OwnedLexToken]) -> Option<u32> {
    let tokens = trim_lexed_commas(tokens);
    let (_, tail) = primitives::parse_prefix(tokens, primitives::kw("put").void())?;
    let parsed = leaf::parse_leaf_number_prefix_tokens(tail)?;
    let after_count = tail.get(parsed.consumed..)?;
    let mut input = LexStream::new(after_count);
    crate::grammar::primitives::take_leaf(&mut input, opt(primitives::kw("of")))?;
    crate::grammar::primitives::take_leaf(&mut input, primitives::kw("those"))?;
    crate::grammar::primitives::take_leaf(
        &mut input,
        alt((primitives::kw("card"), primitives::kw("cards"))).void(),
    )?;
    if !input.is_empty() {
        return None;
    }
    parsed.into_fixed().map(|(count, _)| count)
}
