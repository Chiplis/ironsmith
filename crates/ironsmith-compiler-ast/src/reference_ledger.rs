//! Where the grammar's reference keys become symbols.
//!
//! A reference scope is entered for the symbol scope a phase is working in
//! (a line, the document). While it lives, every key the grammar mints binds
//! immediately in that scope through `SymbolTable::bind_keyed`, which returns
//! the existing symbol for a key already bound there; the mint's result is a
//! `TagRef` carrying the symbol and the key. Outside any scope (unit tests,
//! detached probes) mints bind in a thread-local default table, so two mints
//! of one key in one scope-less run are the same symbol.

use std::cell::{Cell, RefCell};

use ironsmith_core::TagKey;

use crate::model::symbols::{Cardinality, ObjectDomain, ReferenceRole, SymbolId, SymbolScopeId, SymbolTable};
use crate::tag_ref::TagRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintedReference {
    pub key: TagKey,
    pub role: ReferenceRole,
    pub domain: ObjectDomain,
    pub cardinality: Cardinality,
}

/// One entered reference scope: the table it binds into and the scope id.
/// The pointer is valid while the guard that pushed the frame lives.
#[derive(Clone, Copy)]
struct Frame {
    symbols: *const RefCell<SymbolTable>,
    scope: SymbolScopeId,
}

thread_local! {
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
    /// Mints outside any scope bind here (root scope), so scope-less parses
    /// and hand-built fixtures agree on symbols.
    static DEFAULT: RefCell<SymbolTable> = RefCell::new(SymbolTable::default());
    /// Open `observe` frames: every mint noted while a frame is open is copied
    /// into it, so a memoized parse can replay its mints.
    static OBSERVERS: RefCell<Vec<Vec<MintedReference>>> = const { RefCell::new(Vec::new()) };
    static ACTIVE_SCOPES: Cell<usize> = const { Cell::new(0) };
}

/// Binds `key` for a reference of `role` over `domain` in the active scope
/// (or the default table) and returns the symbol, as a `TagRef`.
pub fn note_minted(key: TagKey, role: ReferenceRole, domain: ObjectDomain, cardinality: Cardinality) -> TagRef {
    OBSERVERS.with(|observers| {
        for frame in observers.borrow_mut().iter_mut() {
            frame.push(MintedReference { key: key.clone(), role, domain, cardinality });
        }
    });
    let frame = FRAMES.with(|frames| frames.borrow().last().copied());
    let symbol = match frame {
        Some(frame) => {
            // SAFETY: the frame was pushed by a live `ReferenceScopeGuard` that
            // borrows the table for at least as long as the frame is on the stack.
            let symbols = unsafe { &*frame.symbols };
            symbols
                .borrow_mut()
                .bind_keyed(frame.scope, key.clone(), role, cardinality, domain)
                .unwrap_or(SymbolId(u32::MAX))
        }
        None => DEFAULT.with(|table| {
            let mut table = table.borrow_mut();
            let root = table.root_scope();
            table
                .bind_keyed(root, key.clone(), role, cardinality, domain)
                .unwrap_or(SymbolId(u32::MAX))
        }),
    };
    TagRef { symbol, key }
}

/// Whether a reference scope is open on this thread.
pub fn in_scope() -> bool {
    ACTIVE_SCOPES.with(|active| active.get()) > 0
}

/// Runs `compute` and returns, with its result, every mint it noted, so a
/// cached result can `replay` them where it is reused.
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

/// A reference scope: while it lives, minted keys bind in `scope` of `symbols`.
pub struct ReferenceScopeGuard<'a> {
    _symbols: &'a RefCell<SymbolTable>,
}

impl<'a> ReferenceScopeGuard<'a> {
    pub fn enter(symbols: &'a RefCell<SymbolTable>, scope: SymbolScopeId) -> Self {
        FRAMES.with(|frames| frames.borrow_mut().push(Frame { symbols: symbols as *const _, scope }));
        ACTIVE_SCOPES.with(|active| active.set(active.get() + 1));
        Self { _symbols: symbols }
    }
}

impl Drop for ReferenceScopeGuard<'_> {
    fn drop(&mut self) {
        FRAMES.with(|frames| {
            frames.borrow_mut().pop();
        });
        ACTIVE_SCOPES.with(|active| active.set(active.get().saturating_sub(1)));
    }
}

#[cfg(test)]#[cfg(test)]
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
            let (_, observed) = observe(|| note_minted(key, role, domain, cardinality));
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
    fn mints_outside_any_scope_share_the_default_table() {
        let (key, role, domain, cardinality) = minted("__stray__");
        let first = note_minted(key.clone(), role, domain, cardinality);
        let second = note_minted(key, role, domain, cardinality);
        assert_eq!(first, second);
        assert_eq!(first.key.as_str(), "__stray__");
    }
}
