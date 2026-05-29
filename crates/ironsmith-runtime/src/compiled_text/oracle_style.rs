use super::*;

pub fn canonical_compiled_lines(def: &CardDefinition) -> Vec<String> {
    super::normalize_ast_surface_lines(super::debug_compiled_lines(def))
        .into_iter()
        .map(|line| super::substitute_legendary_source_reference(&line, &def.card, ""))
        .collect()
}
