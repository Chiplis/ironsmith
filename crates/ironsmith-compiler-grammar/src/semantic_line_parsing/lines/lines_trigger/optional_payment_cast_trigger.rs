//! Optional mana payment and its cast-trigger consequence are independent clauses.
//! This reading has no keyword identity: the payment and paid-label trigger
//! reuse the ordinary optional-cost lowering shared with other casting costs.
use super::*;

pub(super) fn read(
    shape: &keyword_special_grammar::OptionalCostWithCastTriggerShape<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let Some((head, tail)) = shape.optional_cost_effect_tokens.split_first() else {
        return Ok(None);
    };
    if !head.is_word("pay") || tail.is_empty() {
        return Ok(None);
    }
    let cost = crate::activation_and_restrictions::parse_compiler_activation_cost(tail)?;
    // Other payment classes have their own readers and affordability rules.
    if cost.has_non_mana_costs() || cost.mana_cost().is_none() {
        return Ok(None);
    }
    let label = format!("As an additional cost to cast this spell, {}", render_token_slice(shape.label_tokens));
    let mut optional = OptionalCost::custom(label.clone(), cost);
    optional.reference = crate::cost::OptionalCostRef::with_discriminator(
        crate::cost::OptionalCostKind::Additional, label);
    optional.reference.custom_keyword_attributable = true;
    let effects = parse_effect_sentences_lexed(shape.followup_effect_tokens)?;
    crate::word_claims::claim_tokens(shape.label_tokens);
    crate::word_claims::claim_tokens(shape.trigger_frame_tokens);
    Ok(Some(LineAst::OptionalCostWithCastTrigger { cost: optional, effects }))
}
