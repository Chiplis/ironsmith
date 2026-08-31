use super::*;

pub fn parse_delayed_upkeep_payment_shape(
    upkeep_tokens: &[OwnedLexToken],
    lose_tokens: &[OwnedLexToken],
) -> Option<DelayedUpkeepPaymentShape> {
    let ((), mana_tokens) = primitives::parse_prefix(trimmed(upkeep_tokens), upkeep_pay_prefix)?;
    let mana_tokens = trimmed(mana_tokens);
    if mana_tokens.is_empty()
        || !super::super::delayed_step_shapes::is_delayed_lose_game_unless_paid_shape(lose_tokens)
    {
        return None;
    }
    let mana =
        crate::grammar::primitives::probe_shape(leaf::parse_leaf_mana_cost_tokens(mana_tokens))?;
    Some(DelayedUpkeepPaymentShape { mana })
}
