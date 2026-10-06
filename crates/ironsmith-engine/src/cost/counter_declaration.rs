//! A prospective cost choice is not a paid outcome. Only announcement target
//! views read this quantity; resolution reads the actual retained cost result.
use crate::ability::ActivatedAbilityRuntimeExt as _;
use crate::cost::{CostPaymentError, TotalCost};
use crate::costs::Cost;
use crate::effect::{Effect, EffectId, Value};
use crate::effects::{RemoveAnyCountersAmongEffect, RemoveAnyCountersFromSourceEffect, WithIdEffect};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::{CounterType, ObjectId, PlayerId, Zone};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterRemovalDeclaration { pub source: ObjectId, pub counter_type: CounterType, pub amount: u32 }
#[derive(Debug, Clone, Copy)]
pub(crate) struct CounterRemovalBounds { pub counter_type: CounterType, pub minimum: u32, pub maximum: u32 }
fn invalid(message: &str) -> CostPaymentError { CostPaymentError::Other(message.into()) }
fn source_counter_descriptor(effect: &Effect) -> Option<(CounterType, u32, u32)> {
    if let Some(remove) = effect.downcast_ref::<RemoveAnyCountersFromSourceEffect>() {
        return (!remove.display_x && !remove.remove_all).then_some((remove.counter_type?, 0, u32::MAX));
    }
    let remove = effect.downcast_ref::<RemoveAnyCountersAmongEffect>()?;
    let mut source = ObjectFilter::source(); source.zone = remove.filter.zone;
    (remove.dynamic_count && !remove.display_x && remove.single_object
        && matches!(remove.filter.zone, None | Some(Zone::Battlefield)) && remove.filter == source)
        .then_some((remove.counter_type?, remove.min_count, remove.count))
}
fn producer_component(cost: &TotalCost) -> Result<(usize, CounterType, u32, u32), CostPaymentError> {
    let components = cost.as_all().ok_or_else(|| invalid("counter declaration requires a selected ordinary cost branch"))?;
    let mut found = None;
    for (index, component) in components.iter().enumerate() {
        let Some(observed) = component.effect_ref().and_then(|effect| effect.downcast_ref::<WithIdEffect>())
            .filter(|observed| observed.id == EffectId::ACTIVATION_COUNTER_COST) else { continue; };
        let (kind, minimum, maximum) = source_counter_descriptor(&observed.effect)
            .ok_or_else(|| invalid("counter declaration requires one variable removal from the exact ability source"))?;
        if found.replace((index, kind, minimum, maximum)).is_some() { return Err(invalid("counter declaration has more than one producer")); }
    }
    found.ok_or_else(|| invalid("counter declaration has no retained payment producer"))
}
pub(crate) fn bounds(game: &GameState, source: ObjectId, cost: &TotalCost) -> Result<CounterRemovalBounds, CostPaymentError> {
    let (_, counter_type, minimum, limit) = producer_component(cost)?;
    if !game.battlefield.contains(&source) || !game.object(source).is_some_and(|object| object.zone == Zone::Battlefield)
        || game.is_phased_out(source) { return Err(invalid("counter declaration source is unavailable")); }
    let maximum = game.counter_count(source, counter_type).min(limit);
    if maximum < minimum { return Err(invalid("not enough source counters to declare this payment")); }
    Ok(CounterRemovalBounds { counter_type, minimum, maximum })
}
pub(crate) fn declare(game: &GameState, source: ObjectId, cost: &TotalCost, amount: u32) -> Result<CounterRemovalDeclaration, CostPaymentError> {
    let limits = bounds(game, source, cost)?;
    if amount < limits.minimum || amount > limits.maximum { return Err(invalid("declared counter removal is outside the payable range")); }
    Ok(CounterRemovalDeclaration { source, counter_type: limits.counter_type, amount })
}
pub(crate) fn lock(game: &GameState, source: ObjectId, cost: &TotalCost, declared: CounterRemovalDeclaration) -> Result<TotalCost, CostPaymentError> {
    if declared.source != source { return Err(invalid("counter declaration belongs to another source incarnation")); }
    if declare(game, source, cost, declared.amount)? != declared { return Err(invalid("counter declaration no longer names the selected payment")); }
    let (index, kind, _, _) = producer_component(cost)?;
    let mut components = cost.as_all().expect("producer checked ordinary branch").to_vec();
    components[index] = Cost::effect(WithIdEffect::new(EffectId::ACTIVATION_COUNTER_COST,
        Effect::remove_counters(kind, declared.amount, ChooseSpec::Source)));
    Ok(TotalCost::from_costs(components))
}
fn is_counter_quantity(value: &Value) -> bool {
    match value.unhinted() {
        Value::EffectValue(id) => *id == EffectId::ACTIVATION_COUNTER_COST,
        Value::PriorEffectMetric { effect_id, query } => *effect_id == EffectId::ACTIVATION_COUNTER_COST
            && query.source == crate::effect::EffectMetricSource::Outcome && query.metric == crate::effect::EffectMetric::Count
            && query.action == Some(crate::effect::PriorEffectAction::Removed) && query.filter.is_none() && query.player.is_none(),
        _ => false,
    }
}
pub(crate) fn target_spec(effects: &[Effect]) -> Option<ChooseSpec> {
    fn visit(effect: &Effect, targets: &mut Vec<ChooseSpec>) {
        if let Some(spec) = effect.0.get_target_spec().filter(|spec| spec.is_target()) && !targets.contains(spec) { targets.push(spec.clone()); }
        effect.visit_child_effects(&mut |child| visit(child, targets));
    }
    let mut targets = Vec::new(); for effect in effects { visit(effect, &mut targets); }
    match targets.as_slice() { [spec] if spec.is_activation_counter_power_bound() => Some(spec.clone()), _ => None }
}
impl CounterRemovalDeclaration {
    pub(crate) fn value(self, source: Option<ObjectId>, value: &Value) -> Option<i64> {
        if source != Some(self.source) || !is_counter_quantity(value) { return None; }
        if let Value::PriorEffectMetric { query, .. } = value.unhinted()
            && query.counter_type.is_some_and(|kind| kind != self.counter_type) { return None; }
        Some(i64::from(self.amount))
    }
}
/// For one power-at-most target, the maximum payable declaration admits every
/// smaller declaration's targets. Each finite candidate is priced separately.
pub(crate) fn preflight(game: &GameState, source: ObjectId, ability_index: usize, payer: PlayerId,
    activated: &crate::ability::ActivatedAbility) -> Option<bool>
{
    target_spec(activated.effects.flattened_default_effects())?;
    let limits = match bounds(game, source, &activated.mana_cost) { Ok(limits) => limits, Err(_) => return Some(false) };
    let declared = CounterRemovalDeclaration { source, counter_type: limits.counter_type, amount: limits.maximum };
    let locked = match lock(game, source, &activated.mana_cost, declared) { Ok(cost) => cost, Err(_) => return Some(false) };
    let x = activated.activation_x_minimum();
    let locked = TotalCost::from_costs(locked.costs().iter().map(|component| {
        if let Some(mana) = component.mana_cost_ref().filter(|mana| mana.has_x()) {
            Cost::mana(crate::decision::mana_cost_with_locked_x_and_generic_reduction(mana, x, 0))
        } else { component.clone() }
    }).collect());
    let references = super::prospective_references::activation_reference_context(game, source, ability_index);
    let view = crate::derived_view::DerivedGameView::new(game).with_target_reference_bindings(references.clone())
        .with_counter_removal_declaration(Some(declared));
    let requirements = crate::game_loop::extract_target_requirements_with_modes_and_announcements(
        game, activated.effects.flattened_default_effects(), payer, Some(source), None, Some(&references), Some(declared));
    let [requirement] = requirements.as_slice() else { return Some(false); };
    if requirement.min_targets != 1 || requirement.max_targets != Some(1) { return Some(false); }
    Some(requirement.legal_targets.iter().copied().any(|target| {
        let priced = crate::decision::calculate_effective_activation_total_cost_for_ability(game, payer, source, &locked, &[target],
            Some(crate::decision::ActivationCostAbility::of(game, payer, source, activated)));
        if validate_locked(&priced, declared).is_err() { return false; }
        let mut mana = crate::mana::ManaCost::new(); let mut nonmana = Vec::new();
        for component in priced.costs() {
            if let Some(part) = component.mana_cost_ref() { mana = crate::decision::add_mana_cost(&mana, part); }
            else { nonmana.push(component.clone()); }
        }
        let reason = activated.payment_reason(game, source, payer);
        if !view.can_potentially_pay_with_reason(payer, Some(source), &mana, 0, reason) { return false; }
        let mut execution = crate::effects::ExecutionContext::new_default(source, payer).with_tagged_objects(references.clone());
        execution.x_value = Some(x);
        execution.announced_targets = Some(vec![match target { crate::Target::Object(id) => crate::effects::ResolvedTarget::Object(id), crate::Target::Player(id) => crate::effects::ResolvedTarget::Player(id) }]);
        crate::special_actions::can_pay_total_cost_with_reason_in_context(game, payer, source, &TotalCost::from_costs(nonmana), reason, &mut execution).is_ok()
    }))
}
pub(crate) fn validate_locked(cost: &TotalCost, declared: CounterRemovalDeclaration) -> Result<(), CostPaymentError> {
    let components = cost.as_all().ok_or_else(|| invalid("declared payment lost its selected branch"))?;
    let mut producers = components.iter().filter_map(|c| c.effect_ref()).filter_map(|e| e.downcast_ref::<WithIdEffect>())
        .filter(|observed| observed.id == EffectId::ACTIVATION_COUNTER_COST);
    let observed = producers.next().ok_or_else(|| invalid("priced cost discarded its declared counter producer"))?;
    if producers.next().is_some() { return Err(invalid("priced cost duplicated its declared counter producer")); }
    let remove = observed.effect.downcast_ref::<crate::effects::RemoveCountersEffect>().ok_or_else(|| invalid("priced cost replaced its declared counter removal"))?;
    if remove.counter_type != declared.counter_type || !matches!(remove.target.base(), ChooseSpec::Source)
        || remove.count.constant_integer() != Some(i64::from(declared.amount)) { return Err(invalid("priced cost changed the declared counter quantity or scope")); }
    Ok(())
}
/// CR 118.11: replacements may alter the paid action. Check its completed
/// original instruction receipt, preserving actual counts for resolution.
pub(crate) fn validate_paid(outcomes: &std::collections::HashMap<EffectId, crate::effect::EffectOutcome>) -> Result<(), CostPaymentError> {
    let outcome = outcomes.get(&EffectId::ACTIVATION_COUNTER_COST).ok_or_else(|| invalid("declared counter payment has no completed receipt"))?.instruction_result();
    if !matches!(outcome.status, crate::effect::OutcomeStatus::Succeeded | crate::effect::OutcomeStatus::Prevented | crate::effect::OutcomeStatus::Replaced)
        || !matches!(outcome.value, crate::effect::OutcomeValue::Count(amount) if amount >= 0) {
        return Err(invalid("counter producer has no completed removal outcome"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::costs::CostContext;
    fn cost() -> TotalCost {
        let removal = RemoveAnyCountersAmongEffect::dynamic(1, u32::MAX, ObjectFilter::source().in_zone(Zone::Battlefield), false)
            .with_counter_type(Some(CounterType::Charge)).from_single_object();
        TotalCost::from_costs(vec![Cost::effect(WithIdEffect::new(EffectId::ACTIVATION_COUNTER_COST, Effect::new(removal)))])
    }
    #[test]
    fn declared_payment_is_exact_wide_and_never_asks_a_second_quantity_or_changes_x() {
        struct NoSecondNumber;
        impl crate::decision::DecisionMaker for NoSecondNumber {
            fn decide_number(&mut self, _: &GameState, _: &crate::decisions::context::NumberContext) -> u32 { panic!("fixed payment cannot redeclare a quantity"); }
        }
        for amount in [1, 2, i32::MAX as u32 + 1, u32::MAX] {
            let payer = PlayerId::from_index(0); let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let card = crate::card::CardBuilder::new(crate::CardId::new(), "Declaration source").card_types(vec![crate::CardType::Artifact]).build();
            let source = game.create_object_from_card(&card, payer, Zone::Battlefield);
            game.object_mut(source).unwrap().counters.insert(CounterType::Charge, amount);
            let cost = cost(); let declared = declare(&game, source, &cost, amount).unwrap();
            let locked = lock(&game, source, &cost, declared).unwrap(); validate_locked(&locked, declared).unwrap();
            let mut dm = NoSecondNumber; let mut context = CostContext::new(source, payer, &mut dm).with_x(17);
            assert!(validate_paid(&context.effect_outcomes).is_err()); locked.costs()[0].pay(&mut game, &mut context).unwrap();
            validate_paid(&context.effect_outcomes).unwrap(); assert_eq!(context.x_value, Some(17));
            assert_eq!(context.effect_outcomes[&EffectId::ACTIVATION_COUNTER_COST].instruction_result().count_or_zero(), i64::from(amount));
            assert_eq!(game.counter_count(source, CounterType::Charge), 0);
        }
    }
    #[test]
    fn declaration_checks_kind_source_range_current_counters_and_price_identity() {
        let payer=PlayerId::from_index(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let card=crate::card::CardBuilder::new(crate::CardId::new(),"Counter scope").card_types(vec![crate::CardType::Artifact]).build();
        let source=game.create_object_from_card(&card,payer,Zone::Battlefield);game.add_counters(source,CounterType::Charge,4);game.add_counters(source,CounterType::Dream,20);
        let cost=cost();assert!(declare(&game,source,&cost,0).is_err());assert!(declare(&game,source,&cost,5).is_err());
        let declared=declare(&game,source,&cost,3).unwrap();assert!(lock(&game,source,&cost,CounterRemovalDeclaration {counter_type:CounterType::Dream,..declared}).is_err());
        assert!(lock(&game,ObjectId::from_raw(999),&cost,declared).is_err());let locked=lock(&game,source,&cost,declared).unwrap();
        assert!(validate_locked(&TotalCost::free(),declared).is_err());assert!(validate_locked(&TotalCost::from_costs(vec![locked.costs()[0].clone(),locked.costs()[0].clone()]),declared).is_err());
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge,2);assert!(lock(&game,source,&cost,declared).is_err());
        let mut dm=crate::decision::SelectFirstDecisionMaker;let mut context=CostContext::new(source,payer,&mut dm);assert!(locked.costs()[0].pay(&mut game,&mut context).is_err());assert!(context.effect_outcomes.is_empty());
    }
}
