//! Structural definition proof over typed native payloads. No rendering or wire
//! codec participates in equality. Unsupported opaque executors fail closed.
#[path = "../native_effect_types.rs"]
mod native_effect_types;

use crate::effects::cards::definition_scope::CardDefinitionScopeEffect;
use crate::{CardDefinition, Effect};

#[derive(Clone, Debug)]
struct EffectProof(Effect);
impl PartialEq for EffectProof {
    fn eq(&self, other: &Self) -> bool {
        effect_equal(&self.0, &other.0)
    }
}
type ProofCost = ironsmith_core::Cost<EffectProof>;
type ProofStatic = ironsmith_core::StaticAbility<
    ironsmith_core::trigger_model::Trigger,
    EffectProof,
    ProofCost,
    crate::static_abilities::ThisSpellCostCondition,
>;

fn cost_proof(cost: crate::costs::Cost) -> Result<ProofCost, String> {
    cost.compiled_model()
        .ok_or_else(|| "native cost has no structural definition proof".to_string())?
        .clone()
        .try_map_effect(|effect| Ok::<_, String>(EffectProof(effect)))
}
fn trigger_proof(
    trigger: crate::triggers::Trigger,
) -> Result<ironsmith_core::trigger_model::Trigger, String> {
    trigger
        .compiled_model()
        .cloned()
        .ok_or_else(|| "native trigger has no structural definition proof".to_string())
}
fn static_proof(ability: crate::static_abilities::StaticAbility) -> Result<ProofStatic, String> {
    ability
        .canonical_model()
        .ok_or_else(|| "native static ability has no structural definition proof".to_string())?
        .try_map(
            trigger_proof,
            |effect| Ok::<_, String>(EffectProof(effect)),
            cost_proof,
            Ok::<_, String>,
        )
}

pub(crate) fn definitions_equal(
    left: &CardDefinition,
    right: &CardDefinition,
) -> Result<bool, String> {
    // Replacing only executable wrappers preserves every non-executable field
    // of the definition, ability, resolution segment, cost and cast route.
    let normalize = |definition: CardDefinition| {
        definition.try_map(
            |ability| {
                ability.try_map(
                    static_proof,
                    trigger_proof,
                    |effect| Ok::<_, String>(EffectProof(effect)),
                    cost_proof,
                    Ok::<_, String>,
                )
            },
            |effect| Ok::<_, String>(EffectProof(effect)),
            cost_proof,
            |method| method.try_map(|effect| Ok::<_, String>(EffectProof(effect)), cost_proof),
            |cost| cost.try_map(cost_proof),
        )
    };
    Ok(normalize(left.clone())? == normalize(right.clone())?)
}
fn effects_equal(left: &[Effect], right: &[Effect]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(a, b)| effect_equal(a, b))
}

macro_rules! with_native_scalar_types {
    ($consumer:ident) => {
        native_effect_types::with_shared_native_effect_types!(
            $consumer,
            crate::effects::AdditionalLandPlaysEffect,
            crate::effects::ShuffleHandAndGraveyardIntoLibraryEffect,
        );
    };
}

/// Exact native scalar admission: visitors must confirm that no executable
/// children or owned definitions are hidden behind a scalar registration.
pub(crate) fn trusted_native_scalar_effect(effect: &Effect) -> bool {
    let mut nested = false;
    effect.visit_child_effects(&mut |_| nested = true);
    effect.visit_card_definitions(&mut |_| nested = true);
    if nested {
        return false;
    }
    macro_rules! recognized { ($($ty:path),* $(,)?) => { $(
        if effect.downcast_ref::<$ty>().is_some() { return true; }
    )* }; }
    with_native_scalar_types!(recognized);
    false
}

fn effect_equal(left: &Effect, right: &Effect) -> bool {
    // Shared immutable executor identity is a complete proof, unlike a card
    // name, display string, or a reused CardId.
    if std::sync::Arc::ptr_eq(&left.0, &right.0) {
        return true;
    }
    if let Some(a) = left.downcast_ref::<CardDefinitionScopeEffect>() {
        let Some(b) = right.downcast_ref::<CardDefinitionScopeEffect>() else {
            return false;
        };
        return a.scope == b.scope
            && a.definitions.len() == b.definitions.len()
            && a.definitions
                .iter()
                .zip(&b.definitions)
                .all(|(a, b)| definitions_equal(a, b) == Ok(true))
            && effect_equal(&a.effect, &b.effect);
    }
    if let Some(a) = left.downcast_ref::<super::CreateCardsEffect>() {
        let Some(b) = right.downcast_ref::<super::CreateCardsEffect>() else {
            return false;
        };
        return a.definition_scope == b.definition_scope
            && a.definition_refs == b.definition_refs
            && a.operation_label == b.operation_label
            && a.selection == b.selection
            && a.count == b.count
            && a.player == b.player
            && a.zone == b.zone
            && a.library_position == b.library_position
            && a.definitions.len() == b.definitions.len()
            && a.definitions
                .iter()
                .zip(&b.definitions)
                .all(|(a, b)| definitions_equal(a, b) == Ok(true));
    }
    if let Some(a) = left.downcast_ref::<crate::effects::SequenceEffect>() {
        let Some(b) = right.downcast_ref::<crate::effects::SequenceEffect>() else {
            return false;
        };
        return a.surface == b.surface
            && a.result_label == b.result_label
            && effects_equal(&a.effects, &b.effects);
    }
    if let Some(a) = left.downcast_ref::<crate::effects::MayEffect>() {
        let Some(b) = right.downcast_ref::<crate::effects::MayEffect>() else {
            return false;
        };
        return a.payment_receipt == b.payment_receipt
            && a.decider == b.decider
            && a.fallback == b.fallback
            && a.pay_as_cost == b.pay_as_cost
            && effects_equal(&a.effects, &b.effects);
    }
    if let Some(a) = left.downcast_ref::<crate::effects::TaggedEffect>() {
        let Some(b) = right.downcast_ref::<crate::effects::TaggedEffect>() else {
            return false;
        };
        return a.tag == b.tag
            && a.outcome_only == b.outcome_only
            && a.action == b.action
            && effect_equal(&a.effect, &b.effect);
    }
    if let Some(a) = left.downcast_ref::<crate::effects::WithIdEffect>() {
        let Some(b) = right.downcast_ref::<crate::effects::WithIdEffect>() else {
            return false;
        };
        return a.id == b.id && effect_equal(&a.effect, &b.effect);
    }
    let mut owned_children = false;
    left.visit_child_effects(&mut |_| owned_children = true);
    left.visit_card_definitions(&mut |_| owned_children = true);
    right.visit_child_effects(&mut |_| owned_children = true);
    right.visit_card_definitions(&mut |_| owned_children = true);
    if owned_children {
        return false;
    }
    macro_rules! compare_scalar {
        ($($ty:path),* $(,)?) => { $(
            if let Some(a) = left.downcast_ref::<$ty>() {
                return right.downcast_ref::<$ty>().is_some_and(|b| a == b);
            }
        )* };
    }
    with_native_scalar_types!(compare_scalar);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sequence(effects: Vec<Effect>) -> Effect {
        Effect::new(crate::effects::SequenceEffect::new(effects))
    }
    fn definition(effect: Effect) -> CardDefinition {
        crate::CardDefinitionBuilder::new(ironsmith_core::CardId::new(), "Native identity")
            .with_spell_effect(vec![effect])
            .build()
    }
    #[test]
    fn independent_native_leaf_payloads_compare_structurally_and_all_definition_metadata_matters() {
        let a = definition(Effect::draw(1));
        let mut b = a.clone();
        b.spell_effect = Some(vec![Effect::draw(1)].into());
        assert_eq!(definitions_equal(&a, &b), Ok(true));
        b.spell_effect = Some(vec![Effect::draw(2)].into());
        assert_eq!(definitions_equal(&a, &b), Ok(false));
        b = a.clone();
        b.card.name = "Different name".into();
        assert_eq!(definitions_equal(&a, &b), Ok(false));
        b = a.clone();
        b.ability_labels.push("unmatched label".into());
        assert_eq!(definitions_equal(&a, &b), Ok(false));
    }
    #[test]
    fn native_wrapper_receipts_and_sequential_child_order_are_not_erased() {
        let a = definition(Effect::new(crate::effects::TaggedEffect::new(
            "result",
            sequence(vec![Effect::draw(1), Effect::gain_life(2)]),
        )));
        let mut b = a.clone();
        b.spell_effect = Some(
            vec![Effect::new(crate::effects::TaggedEffect::new(
                "result",
                sequence(vec![Effect::draw(1), Effect::gain_life(2)]),
            ))]
            .into(),
        );
        assert_eq!(definitions_equal(&a, &b), Ok(true));
        b.spell_effect = Some(
            vec![Effect::new(crate::effects::TaggedEffect::new(
                "result",
                sequence(vec![Effect::gain_life(2), Effect::draw(1)]),
            ))]
            .into(),
        );
        assert_eq!(definitions_equal(&a, &b), Ok(false));
        b.spell_effect = Some(
            vec![Effect::new(crate::effects::TaggedEffect::new(
                "other-result",
                sequence(vec![Effect::draw(1), Effect::gain_life(2)]),
            ))]
            .into(),
        );
        assert_eq!(definitions_equal(&a, &b), Ok(false));
    }
}
