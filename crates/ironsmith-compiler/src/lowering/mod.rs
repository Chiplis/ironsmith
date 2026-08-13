use crate::model::ParsedCardAst;

pub(crate) mod selection_materialization;

pub(crate) trait CardAstMaterializer {
    type RuntimeDocument;
    type Error;

    fn materialize(&mut self, ast: ParsedCardAst) -> Result<Self::RuntimeDocument, Self::Error>;
}

/// The only compiler-AST-to-runtime card boundary. Parsing never calls a
/// concrete runtime builder directly; the runtime backend supplies the
/// materializer implementation.
pub(crate) fn lower_card_ast<M: CardAstMaterializer>(
    materializer: &mut M,
    ast: ParsedCardAst,
) -> Result<M::RuntimeDocument, M::Error> {
    materializer.materialize(ast)
}
