#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "At the beginning of combat on your turn, you may discard a card. When you do, this creature deals X damage to any target, where X is the number of card types the discarded card has.";

fn find_damage(effect: &crate::effect::Effect) -> Option<&crate::effects::DealDamageEffect> {
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return Some(damage);
    }
    if let Some(execute) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
        return find_damage(&execute.effect);
    }
    if let Some(reflexive) = effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>() {
        return reflexive.effects.iter().find_map(find_damage);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return find_damage(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return find_damage(&with_id.effect);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence.effects.iter().find_map(find_damage);
    }
    None
}

#[test]
fn mount_velus_manticore_counts_card_types_on_the_discard_result() {
    let definition = parse_oracle_card_definition("Mount Velus Manticore");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Mount Velus Manticore should have a combat trigger");
    let damage = triggered
        .effects
        .flattened_default_effects()
        .into_iter()
        .find_map(find_damage)
        .expect("the reflexive trigger should deal damage");
    let crate::effect::Value::PriorEffectMetric { effect_id, query } = damage.amount.unhinted()
    else {
        panic!("X must be a typed metric over the discarded object: {damage:#?}");
    };
    assert_eq!(*effect_id, crate::effect::EffectId::from(0));
    assert_eq!(query.metric, crate::effect::EffectMetric::CardTypesAmong);
    assert_eq!(
        query.action,
        Some(crate::effect::PriorEffectAction::Discarded)
    );
}
