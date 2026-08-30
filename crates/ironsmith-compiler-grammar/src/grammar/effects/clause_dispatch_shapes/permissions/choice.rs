use super::*;

pub fn parse_opponent_return_choice_shape(
    tokens: &[OwnedLexToken],
) -> Option<OpponentReturnChoiceShape<'_>> {
    let (_, choice_tail) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["for", "each", "opponent", "choose"]),
    )?;
    let (then_start, _, after_then_return) =
        primitives::find_prefix(choice_tail, || primitives::phrase(&["then", "return"]))?;
    let (_, _, after_unless) =
        primitives::find_prefix(after_then_return, || primitives::kw("unless"))?;
    primitives::parse_prefix(
        after_unless,
        primitives::phrase(&["its", "controller", "has", "you", "draw", "a", "card"]),
    )?;
    let target_tokens = trim_lexed_commas(choice_tail.get(..then_start)?);
    (!target_tokens.is_empty()).then_some(OpponentReturnChoiceShape { target_tokens })
}
