use crate::model::ParsedCardAst;

pub mod activated_ability_materialization;
pub mod cost_materialization;
pub mod legality_materialization;
pub mod selection_materialization;
pub mod static_ability_materialization;
pub mod structured_ability_materialization;
pub mod triggered_ability_materialization;

pub trait CardAstMaterializer {
    type RuntimeDocument;
    type Error;

    fn materialize(&mut self, ast: ParsedCardAst) -> Result<Self::RuntimeDocument, Self::Error>;
}

/// The only compiler-AST-to-runtime card boundary. Parsing never calls a
/// concrete runtime builder directly; the runtime backend supplies the
/// materializer implementation.
pub fn lower_card_ast<M: CardAstMaterializer>(
    materializer: &mut M,
    ast: ParsedCardAst,
) -> Result<M::RuntimeDocument, M::Error> {
    materializer.materialize(ast)
}
