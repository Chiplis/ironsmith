//! The reference ledger: where the grammar mints a string reference key, it
//! also declares the symbol the key stands for. Minting happens deep in the
//! effect grammar, far from any parse context, so the declarations queue in a
//! thread-local ledger and the enclosing parse scope (a line, a nested
//! ability, a modal mode) drains them into the symbol table when it closes.
//! Item 6 of the repair order: `SymbolId` becomes the only semantic reference
//! identity; this ledger is the step that gives every key a symbol.

use std::cell::{Cell, RefCell};

use ironsmith_core::TagKey;

use crate::model::symbols::{Cardinality, ObjectDomain, ReferenceRole, SymbolScopeId, SymbolTable};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintedReference {
    pub key: TagKey,
    pub role: ReferenceRole,
    pub domain: ObjectDomain,
    pub cardinality: Cardinality,
}

thread_local! {
    static ACTIVE_SCOPES: Cell<usize> = const { Cell::new(0) };
    static PENDING: RefCell<Vec<MintedReference>> = const { RefCell::new(Vec::new()) };
    /// Open `observe` frames: every mint noted while a frame is open is copied
    /// into it, deduplicated or not, so a memoized parse can replay its mints.
    static OBSERVERS: RefCell<Vec<Vec<MintedReference>>> = const { RefCell::new(Vec::new()) };
}

/// Record that the grammar minted `key` for a reference of `role` over
/// `domain`. Outside any reference scope (unit tests, detached probes) the
/// mint is not recorded.
pub fn note_minted(key: TagKey, role: ReferenceRole, domain: ObjectDomain, cardinality: Cardinality) {
    OBSERVERS.with(|observers| {
        for frame in observers.borrow_mut().iter_mut() {
            frame.push(MintedReference { key: key.clone(), role, domain, cardinality });
        }
    });
    if ACTIVE_SCOPES.with(|active| active.get()) == 0 {
        return;
    }
    PENDING.with(|pending| {
        let mut pending = pending.borrow_mut();
        if !pending.iter().any(|minted| minted.key == key) {
            pending.push(MintedReference { key, role, domain, cardinality });
        }
    });
}

/// The pending mints, drained.
/// Runs `compute` and returns, with its result, every mint it noted (in or out
/// of a scope), so a cached result can `replay` them where it is reused.
pub fn observe<T>(compute: impl FnOnce() -> T) -> (T, Vec<MintedReference>) {
    OBSERVERS.with(|observers| observers.borrow_mut().push(Vec::new()));
    let result = compute();
    let minted = OBSERVERS.with(|observers| observers.borrow_mut().pop().unwrap_or_default());
    (result, minted)
}

/// Notes `minted` again, as if the parse that produced them ran here.
pub fn replay(minted: &[MintedReference]) {
    for mint in minted {
        note_minted(mint.key.clone(), mint.role, mint.domain, mint.cardinality);
    }
}

fn take_pending() -> Vec<MintedReference> {
    PENDING.with(|pending| std::mem::take(&mut *pending.borrow_mut()))
}

/// A reference scope: while it lives, minted keys queue for `scope`; when it
/// drops, they bind in `symbols` at `scope`.
pub struct ReferenceScopeGuard<'a> {
    symbols: &'a RefCell<SymbolTable>,
    scope: SymbolScopeId,
    outer_pending: Vec<MintedReference>,
}

impl<'a> ReferenceScopeGuard<'a> {
    pub fn enter(symbols: &'a RefCell<SymbolTable>, scope: SymbolScopeId) -> Self {
        ACTIVE_SCOPES.with(|active| active.set(active.get() + 1));
        // mints queued by an enclosing scope stay with that scope
        let outer_pending = take_pending();
        Self { symbols, scope, outer_pending }
    }
}

impl Drop for ReferenceScopeGuard<'_> {
    fn drop(&mut self) {
        let minted = take_pending();
        if let Ok(mut symbols) = self.symbols.try_borrow_mut() {
            for reference in minted {
                let _ = symbols.bind_keyed(
                    self.scope,
                    reference.key,
                    reference.role,
                    reference.cardinality,
                    reference.domain,
                );
            }
        }
        PENDING.with(|pending| pending.borrow_mut().extend(std::mem::take(&mut self.outer_pending)));
        ACTIVE_SCOPES.with(|active| active.set(active.get().saturating_sub(1)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::SymbolScopeKind;

    fn minted(key: &str) -> (TagKey, ReferenceRole, ObjectDomain, Cardinality) {
        (TagKey::new(key), ReferenceRole::Affected, ObjectDomain::Object, Cardinality::Any)
    }

    #[test]
    fn a_scope_binds_what_was_minted_while_it_lived_and_dedupes_keys() {
        let symbols = RefCell::new(SymbolTable::default());
        let root = symbols.borrow().root_scope();
        let line = symbols.borrow_mut().create_scope(root, SymbolScopeKind::Line { source_line: 0 }).unwrap();
        {
            let _scope = ReferenceScopeGuard::enter(&symbols, line);
            let (key, role, domain, cardinality) = minted("__it__");
            note_minted(key.clone(), role, domain, cardinality);
            note_minted(key, role, domain, cardinality);
        }
        let table = symbols.borrow();
        let bound = table.symbol_for_key(line, &TagKey::new("__it__")).expect("bound at the line");
        assert_eq!(table.binding(bound).and_then(|b| b.key.clone()), Some(TagKey::new("__it__")));
        assert_eq!(table.visible_bindings(line).len(), 1);
    }

    #[test]
    fn a_nested_scope_keeps_its_own_mints_and_returns_the_outer_ones() {
        let symbols = RefCell::new(SymbolTable::default());
        let root = symbols.borrow().root_scope();
        let line = symbols.borrow_mut().create_scope(root, SymbolScopeKind::Line { source_line: 0 }).unwrap();
        let nested = symbols.borrow_mut().create_scope(line, SymbolScopeKind::NestedAbility).unwrap();
        {
            let _line_scope = ReferenceScopeGuard::enter(&symbols, line);
            let (outer, role, domain, cardinality) = minted("__outer__");
            note_minted(outer, role, domain, cardinality);
            {
                let _nested_scope = ReferenceScopeGuard::enter(&symbols, nested);
                let (inner, role, domain, cardinality) = minted("__inner__");
                note_minted(inner, role, domain, cardinality);
            }
        }
        let table = symbols.borrow();
        assert!(table.symbol_for_key(nested, &TagKey::new("__inner__")).is_some());
        assert!(table.symbol_for_key(line, &TagKey::new("__inner__")).is_none());
        assert!(table.symbol_for_key(nested, &TagKey::new("__outer__")).is_some(), "outer keys resolve from the nested scope");
    }

    #[test]
    fn an_observed_parse_reports_every_mint_and_a_replay_binds_them_elsewhere() {
        let symbols = RefCell::new(SymbolTable::default());
        let root = symbols.borrow().root_scope();
        let first = symbols.borrow_mut().create_scope(root, SymbolScopeKind::Line { source_line: 1 }).unwrap();
        let second = symbols.borrow_mut().create_scope(root, SymbolScopeKind::Line { source_line: 2 }).unwrap();
        let minted_in_first = {
            let _scope = ReferenceScopeGuard::enter(&symbols, first);
            let (key, role, domain, cardinality) = minted("__it__");
            note_minted(key.clone(), role, domain, cardinality);
            // Already pending for this scope: deduplicated there, still observed.
            let ((), observed) = observe(|| note_minted(key, role, domain, cardinality));
            observed
        };
        assert_eq!(minted_in_first.len(), 1);
        {
            let _scope = ReferenceScopeGuard::enter(&symbols, second);
            replay(&minted_in_first);
        }
        let table = symbols.borrow();
        assert!(table.symbol_for_key(second, &TagKey::new("__it__")).is_some());
        assert_ne!(
            table.symbol_for_key(first, &TagKey::new("__it__")),
            table.symbol_for_key(second, &TagKey::new("__it__"))
        );
    }

    #[test]
    fn mints_outside_any_scope_are_not_recorded() {
        let (key, role, domain, cardinality) = minted("__stray__");
        note_minted(key, role, domain, cardinality);
        assert!(take_pending().is_empty());
    }
}
