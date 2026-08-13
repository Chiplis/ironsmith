use crate::cards::builders::CardTextError;
use crate::model::{CompilerGrantedAbilityAst, CompilerStaticAbilityAst, StaticOperationAst};

/// Runtime construction is injected here so recognition and semantic
/// validation never need runtime ability constructors.
pub(crate) trait StaticAbilityMaterializer {
    type RuntimeStatic;
    type RuntimeAbility;

    fn materialize_operation(
        &mut self,
        ability: &CompilerStaticAbilityAst,
        operation: &StaticOperationAst,
    ) -> Result<Self::RuntimeStatic, CardTextError>;

    fn materialize_granted(
        &mut self,
        ability: &CompilerGrantedAbilityAst,
    ) -> Result<Self::RuntimeAbility, CardTextError>;
}

pub(crate) fn materialize_static_ability<M: StaticAbilityMaterializer>(
    materializer: &mut M,
    ability: &CompilerStaticAbilityAst,
) -> Result<M::RuntimeStatic, CardTextError> {
    materializer.materialize_operation(ability, &ability.operation)
}

pub(crate) fn materialize_nested_grants<M: StaticAbilityMaterializer>(
    materializer: &mut M,
    ability: &CompilerStaticAbilityAst,
) -> Result<Vec<M::RuntimeAbility>, CardTextError> {
    ability
        .granted_abilities()
        .iter()
        .map(|granted| materializer.materialize_granted(granted))
        .collect()
}
