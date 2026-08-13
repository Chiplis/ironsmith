use std::collections::HashMap;

use crate::TagKey;
use crate::model::provenance::ProvenanceId;
use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceRole, SymbolId, SymbolResolutionError, SymbolScopeId,
    SymbolTable,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LegacyBindingKey {
    tag: TagKey,
    scope: SymbolScopeId,
    role: ReferenceRole,
    cardinality: Cardinality,
    domain: ObjectDomain,
}

/// The only semantic boundary between legacy runtime tag names and compiler
/// symbol identity. The adapter never infers a role from a string prefix: the
/// caller must provide the complete typed reference contract.
#[derive(Debug, Default)]
pub(crate) struct LegacyTagSymbolBridge {
    imported: HashMap<LegacyBindingKey, SymbolId>,
    exported: HashMap<SymbolId, TagKey>,
}

impl LegacyTagSymbolBridge {
    pub(crate) fn import(
        &mut self,
        symbols: &mut SymbolTable,
        tag: &TagKey,
        scope: SymbolScopeId,
        role: ReferenceRole,
        cardinality: Cardinality,
        domain: ObjectDomain,
        provenance: Option<ProvenanceId>,
    ) -> Result<SymbolId, SymbolResolutionError> {
        let key = LegacyBindingKey {
            tag: tag.clone(),
            scope,
            role,
            cardinality,
            domain,
        };
        if let Some(symbol) = self.imported.get(&key) {
            return Ok(*symbol);
        }
        let symbol = symbols.bind(scope, role, cardinality, domain, provenance)?;
        self.imported.insert(key, symbol);
        self.exported.insert(symbol, tag.clone());
        Ok(symbol)
    }

    pub(crate) fn export(&mut self, symbol: SymbolId) -> TagKey {
        self.exported
            .entry(symbol)
            .or_insert_with(|| TagKey::new(format!("__compiler_symbol_{}", symbol.0)))
            .clone()
    }
}

/// Compatibility constructor for runtime-only or external-serialization tags.
/// Compiler grammar must allocate a `SymbolId` instead.
pub(crate) fn legacy_tag(value: impl Into<String>) -> TagKey {
    TagKey::new(value)
}
