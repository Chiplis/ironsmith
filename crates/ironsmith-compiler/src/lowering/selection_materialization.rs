use crate::model::CompilerSelectionAst;

pub(crate) trait SelectionMaterializer {
    type RuntimeSelection;
    type Error;

    fn materialize_selection(
        &mut self,
        selection: &CompilerSelectionAst,
    ) -> Result<Self::RuntimeSelection, Self::Error>;
}

pub(crate) fn lower_selection<M: SelectionMaterializer>(
    materializer: &mut M,
    selection: &CompilerSelectionAst,
) -> Result<M::RuntimeSelection, M::Error> {
    materializer.materialize_selection(selection)
}
