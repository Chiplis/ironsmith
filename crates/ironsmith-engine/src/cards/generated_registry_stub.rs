// The runtime catalog accepts versioned compiled-card artifacts.
// Generated Rust registries are data-maintenance output, never build-script output.

pub const GENERATED_PARSER_CARD_SOURCE_COUNT: usize = 0;

pub fn generated_parser_entry_count() -> usize {
    0
}

pub fn generated_parser_card_names() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
pub fn generated_parser_card_aliases() -> Vec<(String, String)> {
    Vec::new()
}

pub fn register_generated_parser_cards(_registry: &mut crate::cards::CardRegistry) {}

pub fn register_generated_parser_cards_chunk(
    _registry: &mut crate::cards::CardRegistry,
    cursor: usize,
    _chunk_size: usize,
) -> usize {
    cursor
}

pub fn register_generated_parser_cards_if_name<F>(
    _registry: &mut crate::cards::CardRegistry,
    _include_name: F,
) where
    F: FnMut(&str) -> bool,
{
}

pub fn generated_parser_semantic_score(_name: &str) -> Option<f32> {
    None
}

pub fn generated_parser_semantic_threshold_counts() -> [usize; 100] {
    [0; 100]
}

pub fn generated_parser_semantic_scored_count() -> usize {
    0
}

pub fn generated_parser_card_parse_source(_name: &str) -> Option<(String, String)> {
    None
}

pub fn try_compile_card_by_name(_name: &str) -> Result<crate::cards::CardDefinition, String> {
    Err("generated registry is not embedded in this product".to_string())
}
