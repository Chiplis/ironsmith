use crate::cards::builders::CardTextError;
use crate::model::CompilerStructuredAbilityAst;

pub(crate) trait StructuredAbilityMaterializer {
    type RuntimeAbility;

    fn materialize(
        &mut self,
        ability: &CompilerStructuredAbilityAst,
    ) -> Result<Self::RuntimeAbility, CardTextError>;
}

pub(crate) fn materialize_structured_ability<M: StructuredAbilityMaterializer>(
    materializer: &mut M,
    ability: &CompilerStructuredAbilityAst,
) -> Result<M::RuntimeAbility, CardTextError> {
    materializer.materialize(ability)
}
