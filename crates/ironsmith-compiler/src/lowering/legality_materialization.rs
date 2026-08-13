use crate::model::{
    CompilerActivationLegalityAst, CompilerCastingLegalityAst, CompilerPermissionAst,
    CompilerTriggerLegalityAst,
};

pub(crate) trait LegalityMaterializer {
    type RuntimeLegality;
    type Error;

    fn materialize_activation(
        &mut self,
        legality: &CompilerActivationLegalityAst,
    ) -> Result<Self::RuntimeLegality, Self::Error>;

    fn materialize_casting(
        &mut self,
        legality: &CompilerCastingLegalityAst,
    ) -> Result<Self::RuntimeLegality, Self::Error>;

    fn materialize_trigger(
        &mut self,
        legality: &CompilerTriggerLegalityAst,
    ) -> Result<Self::RuntimeLegality, Self::Error>;

    fn materialize_permission(
        &mut self,
        permission: &CompilerPermissionAst,
    ) -> Result<Self::RuntimeLegality, Self::Error>;
}
