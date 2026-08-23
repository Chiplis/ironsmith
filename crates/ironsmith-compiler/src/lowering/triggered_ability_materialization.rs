use crate::cards::builders::CardTextError;
use crate::model::CompilerTriggeredAbilityAst;

pub trait TriggeredAbilityMaterializer {
    type RuntimeAbility;

    fn materialize(
        &mut self,
        ability: &CompilerTriggeredAbilityAst,
    ) -> Result<Self::RuntimeAbility, CardTextError>;
}

/// Runtime matcher and trigger allocation begins only after event semantics,
/// scoped triggering references, conditions, and linked effects are resolved.
pub fn materialize_triggered_ability<M: TriggeredAbilityMaterializer>(
    materializer: &mut M,
    ability: &CompilerTriggeredAbilityAst,
) -> Result<M::RuntimeAbility, CardTextError> {
    materializer.materialize(ability)
}
