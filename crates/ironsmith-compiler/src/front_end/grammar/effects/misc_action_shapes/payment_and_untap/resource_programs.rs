use super::*;

/// Parse a mana payment whose printed X is chosen subject to an authored
/// upper bound, rather than defined to equal another value.
pub fn parse_bounded_x_payment_tokens(tokens: &[OwnedLexToken]) -> Option<BoundedXPaymentShape> {
    let (_, after_pay) = primitives::parse_prefix(tokens, primitives::kw("pay").void())?;
    let parsed_cost = leaf::parse_leaf_mana_cost_prefix_tokens(after_pay)?;
    if !parsed_cost.cost.has_x() {
        return None;
    }

    let after_cost = trim_lexed_commas(after_pay.get(parsed_cost.consumed..)?);
    let (_, maximum_tokens) = primitives::parse_prefix(
        after_cost,
        primitives::phrase(&["where", "x", "is", "less", "than", "or", "equal", "to"]).void(),
    )?;
    let maximum = permission_shapes::exact_tokens(
        maximum_tokens,
        &["the", "amount", "of", "life", "you", "gained"],
    )
    .then_some(BoundedXMaximumShape::TriggeringLifeGained)?;

    Some(BoundedXPaymentShape {
        cost: parsed_cost.cost,
        maximum,
    })
}
