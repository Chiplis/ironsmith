//! Ordered finite adapters from legacy semantic leaves to canonical clauses.

use crate::cards::builders::CardTextError;
use crate::model::compiler_semantic::ParsedCardItem;
use crate::model::symbols::SymbolTable;

pub(crate) fn migrate_semantic_domains(
    _items: &mut [ParsedCardItem],
    _symbols: &mut SymbolTable,
) -> Result<(), CardTextError> {
    // Library clauses are not yet accepted by executable effect lowering.
    // Enabling the adapter before that dispatch exists turns otherwise valid
    // legacy library effects into an unlowerable `EffectAst::Clause`. Keep the
    // semantic boundary behavior-preserving until the clause consumer lands.
    Ok(())
}
