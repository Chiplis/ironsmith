//! Which ranked registries saw more than one non-equivalent rule accept the
//! same input, and which rules.
//!
//! A ranked registry runs every viable rule and keeps registration order as
//! its tie-break — the semantics of the first-match ladder it replaced. Every
//! input on which two or more non-equivalent rules matched is an overlap: a
//! place where the order, not the grammar, decides the language. The repair
//! order's item 4 resolves each overlap (tighter heads, a declared
//! equivalence, or a shared typed clause) and then flips the registry to
//! strict resolution. This ledger records the overlaps; a tool tallies them
//! over the corpus.
//!
//! Off unless a caller enables it for the current thread; the enabled check is
//! one thread-local read.

use std::cell::RefCell;

use crate::recognition::RuleId;

/// One input that more than one rule of a ranked registry accepted.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Overlap {
    pub registry: &'static str,
    pub rules: Vec<&'static str>,
    pub text: String,
}

thread_local! {
    static LEDGER: RefCell<Option<Vec<Overlap>>> = const { RefCell::new(None) };
}

/// Start recording on this thread. Any previous recording is discarded.
pub fn begin() {
    LEDGER.with(|ledger| *ledger.borrow_mut() = Some(Vec::new()));
}

/// The overlaps recorded since [`begin`], in order. Stops recording.
pub fn end() -> Vec<Overlap> {
    LEDGER.with(|ledger| ledger.borrow_mut().take().unwrap_or_default())
}

pub(crate) fn note(registry: RuleId, rules: &[RuleId], text: impl FnOnce() -> String) {
    LEDGER.with(|ledger| {
        if let Some(overlaps) = ledger.borrow_mut().as_mut() {
            overlaps.push(Overlap {
                registry: registry.as_str(),
                rules: rules.iter().map(|rule| rule.as_str()).collect(),
                text: text(),
            });
        }
    });
}
