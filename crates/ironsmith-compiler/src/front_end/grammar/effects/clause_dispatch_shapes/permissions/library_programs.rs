use super::*;

pub fn parse_for_each_card_payment_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachCardPaymentShape> {
    let (_, body) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["for", "each", "of", "those", "cards"]),
    )?;
    let (_, _, after_pay) = primitives::find_prefix(body, || primitives::kw("pay"))?;
    let (life_amount, after_amount) =
        primitives::parse_prefix(after_pay, leaf::parse_leaf_number_token_lexed)?;
    primitives::parse_all(
        trim_lexed_commas(after_amount),
        (
            primitives::phrase(&[
                "or", "put", "the", "card", "on", "top", "of", "your", "library",
            ]),
            primitives::sentence_end(),
        )
            .void(),
        "for each card payment tail",
    )
    .ok()?;
    Some(ForEachCardPaymentShape { life_amount })
}
