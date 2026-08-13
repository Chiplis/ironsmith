//! Shared state for the finite PR-25 through PR-29 semantic adapters.
//!
//! Legacy tags are admitted only at this boundary. Canonical domain clauses
//! receive typed symbol references and never inspect tag spelling.

use std::collections::HashMap;

use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceRole, SymbolReference, SymbolResolutionError, SymbolTable,
};
use crate::tag::TagKey;

pub(crate) struct SemanticMigrationContext<'a> {
    symbols: &'a mut SymbolTable,
    tagged_objects: HashMap<TagKey, SymbolReference>,
}

impl<'a> SemanticMigrationContext<'a> {
    pub(crate) fn new(symbols: &'a mut SymbolTable) -> Self {
        Self {
            symbols,
            tagged_objects: HashMap::new(),
        }
    }

    pub(crate) fn bind_object(
        &mut self,
        tag: Option<TagKey>,
        role: ReferenceRole,
        cardinality: Cardinality,
    ) -> Result<SymbolReference, SymbolResolutionError> {
        if let Some(reference) = tag.as_ref().and_then(|tag| self.tagged_objects.get(tag)) {
            return Ok(*reference);
        }
        let reference = SymbolReference {
            symbol: self.symbols.bind(
                self.symbols.root_scope(),
                role,
                cardinality,
                ObjectDomain::Card,
                None,
            )?,
            role,
            domain: ObjectDomain::Card,
            cardinality,
        };
        if let Some(tag) = tag {
            self.tagged_objects.insert(tag, reference);
        }
        Ok(reference)
    }

    pub(crate) fn object_reference(&self, tag: &TagKey) -> Option<SymbolReference> {
        self.tagged_objects.get(tag).copied()
    }

    pub(crate) fn bind_selection(
        &mut self,
        role: ReferenceRole,
        domain: ObjectDomain,
        cardinality: Cardinality,
    ) -> Result<SymbolReference, SymbolResolutionError> {
        Ok(SymbolReference {
            symbol: self.symbols.bind(
                self.symbols.root_scope(),
                role,
                cardinality,
                domain,
                None,
            )?,
            role,
            domain,
            cardinality,
        })
    }
}
