//! Independent optional-payment and cost-modifier clause readers. Neither
//! clause carries a custom keyword name or an official keyword identity.
use super::*;

struct TapPayment {
    filter: ObjectFilter,
    count: crate::effect::ChoiceCount,
    label: String,
    name: Option<String>,
}

fn read_payment(tokens: &[OwnedLexToken]) -> Result<Option<TapPayment>, CardTextError> {
    use crate::grammar::primitives;
    let (tail,name)=if let Some((_,tail))=primitives::parse_prefix(tokens,primitives::phrase(&[
        "as","an","additional","cost","to","cast","this","spell",
    ])){(tail,None)}else{
        let view=crate::lexer::TokenWordView::new(tokens);let words=view.word_refs();
        if !words.starts_with(&["as","an","additional","cost","named"]){return Ok(None)}
        let Some(at)=words.windows(4).position(|part|part==["to","cast","this","spell"])else{return Ok(None)};
        if at<=5{return Ok(None)}
        let name=words[5..at].join(" ");
        if !name.chars().all(|c|c.is_alphabetic()||c==' '||c=='-'||c=='\''){return Ok(None)}
        let Some(end)=view.token_index_after_words(at+4) else {return Ok(None)};
        crate::word_claims::claim_tokens(&tokens[..end]);
        (&tokens[end..],Some(name))
    };
    let tail = crate::util::trim_commas(tail);
    let Some((_, cost_tokens)) = primitives::parse_prefix(&tail, primitives::phrase(&["you", "may"])) else { return Ok(None); };
    let cost = crate::activation_and_restrictions::parse_compiler_activation_cost(cost_tokens)?;
    let [crate::model::CompilerCost::TapChosen { count, filter }] = cost.costs() else { return Ok(None); };
    // This adapter links one object to one characteristic value. Other cost
    // counts retain their ordinary readers and require a distinct composition.
    if count.min != 1 || count.max != Some(1) { return Ok(None); }
    Ok(Some(TapPayment { filter: filter.clone(), count: *count,
        label: render_token_slice(tokens),name }))
}

fn read_reduction(tokens: &[OwnedLexToken]) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(ability) = crate::keyword_static::parse_spells_cost_modifier_line(tokens)? else { return Ok(None); };
    let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) = &ability.payload else { return Ok(None); };
    let Value::PowerOf(spec) = reduction.amount.unhinted() else { return Ok(None); };
    let crate::target::ChooseSpec::Tagged(tag) = spec.unhinted() else { return Ok(None); };
    // This tag is the existing value reader's typed tap-cost reference. An
    // arbitrary power expression cannot be rebound to this payment.
    if tag.as_str() != "tap_cost_0" { return Ok(None); }
    Ok(Some(ability))
}

pub(super) fn read(tokens: &[OwnedLexToken]) -> Result<Option<LineAst>, CardTextError> {
    let Some(period) = tokens.iter().position(|token| matches!(token.kind, crate::lexer::TokenKind::Period)) else { return Ok(None); };
    let Some(payment) = read_payment(&tokens[..period])? else { return Ok(None); };
    let Some(mut reduction) = read_reduction(&tokens[period + 1..])? else { return Ok(None); };
    let tag = crate::tag::declared_key(format!("optional_payment_{}", payment.label));
    let mut filter = payment.filter;
    filter.untapped = true;
    let choose = EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseObjects {
        filter: filter.clone(), count: payment.count, count_value: None, player: PlayerAst::You, tag: tag.clone(),
    });
    let tap = EffectAst::subject_verb_tap_all(ObjectFilter::tagged(tag.clone()));
    let cost = ironsmith_core::TotalCost::from_costs(vec![
        crate::model::CompilerCost::ValidatedEffect(Box::new(choose)),
        crate::model::CompilerCost::ValidatedEffect(Box::new(tap)),
    ]);
    let mut optional = crate::model::CompilerOptionalCost::custom(payment.label.clone(), cost);
    optional.reference = crate::cost::OptionalCostRef::with_discriminator(
        crate::cost::OptionalCostKind::Additional, payment.label);
    optional.reference.custom_keyword_attributable = true;
    optional.reference.custom_keyword = payment.name;
    let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(value) = &mut reduction.payload else { unreachable!() };
    value.amount = Value::PendingPriorEffectMetric(ironsmith_core::PriorEffectMetricQuery::new(
        ironsmith_core::EffectMetricSource::AffectedObjects, ironsmith_core::EffectMetric::TotalPower,
    ).with_filter(ObjectFilter::tagged(tag)).with_action(ironsmith_core::PriorEffectAction::Tapped));
    // Prefix grammar, full cost/filter grammar, and independent cost/value
    // grammar own their respective words. There is no whole-paragraph claim.
    Ok(Some(LineAst::Multiple(vec![LineAst::OptionalCost(optional),
        LineAst::StaticAbilities(vec![reduction.into()])])))
}

/// Adapt the already parsed optional payment and matching modifier as one
/// static acquisition recipe. It is used only by the quoted grant route.
pub(super) fn read_granted_policy(tokens:&[OwnedLexToken])->Result<Option<StaticAbility>,CardTextError>{
 let Some(LineAst::Multiple(lines))=read(tokens)? else{return Ok(None)};
 let mut payment=None;let mut reduction=None;
 for line in lines {match line{
  LineAst::OptionalCost(cost)=>payment=Some(cost),
  LineAst::StaticAbilities(abilities)=>for ability in abilities {if let crate::cards::builders::StaticAbilityAst::Static(ability)=ability {
   if let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(value)=ability.payload {reduction=Some(value.amount);}
  }},_=>return Ok(None),
 }}
 let (Some(payment),Some(reduction))=(payment,reduction)else{return Ok(None)};
 let locals=match reduction.unhinted(){Value::PendingPriorEffectMetric(query)=>query.filter.as_ref().map(|filter|filter.tagged_constraints.iter().map(|constraint|constraint.tag.clone()).collect()).unwrap_or_default(),_=>Vec::new()};
 Ok(Some(StaticAbility::grant_optional_spell_cost(ObjectFilter::source(),
  ironsmith_core::static_ability_model::GrantedOptionalCostPrice::Fixed(payment.cost),payment.reference,Vec::new(),
  ironsmith_core::static_ability_model::GrantedOptionalCostTiming::DuringCasting,render_token_slice(tokens))
  .with_optional_spell_cost_reduction(reduction).with_optional_spell_cost_local_references(locals)))
}
