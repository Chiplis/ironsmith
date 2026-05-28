use super::*;

pub fn canonical_compiled_lines(def: &CardDefinition) -> Vec<String> {
    super::normalize_ast_surface_lines(def, super::debug_compiled_lines(def))
}
