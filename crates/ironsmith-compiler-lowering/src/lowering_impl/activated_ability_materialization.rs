use crate::cards::builders::CardTextError;
use crate::model::CompilerActivatedAbilityAst;

pub trait ActivatedAbilityMaterializer {
    type RuntimeAbility;

    fn materialize(
        &mut self,
        ability: &CompilerActivatedAbilityAst,
    ) -> Result<Self::RuntimeAbility, CardTextError>;
}

/// Runtime activation allocation begins only after the cost, effects,
/// restrictions, targets, and mana-ability facts form one resolved node.
pub fn materialize_activated_ability<M: ActivatedAbilityMaterializer>(
    materializer: &mut M,
    ability: &CompilerActivatedAbilityAst,
) -> Result<M::RuntimeAbility, CardTextError> {
    materializer.materialize(ability)
}
