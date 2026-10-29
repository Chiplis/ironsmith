//! Which authored words a parse actually read.
//!
//! The object-filter grammar scans words rather than consuming them, so an
//! unknown word ("blorp creatures you control") is silently ignored and the
//! filter means something other than what the card says. Readers record the
//! words they read here; a filter parse can then compare its input with what
//! was read (architecture/filter-word-consumption-plan.md).
//!
//! A claim is the source span of a token, not an index into a slice, so it
//! survives the copying, slicing, article removal and sub-phrase recursion the
//! filter code does to its input. Synthetic tokens have an empty span and are
//! never claimed: whoever inserted them already read the words they replace.
//!
//! Like parse-loss recording, this is a side channel. Claims made inside a
//! [`scoped`] call are returned to its caller, which [`adopt`]s them into the
//! enclosing scope only for a reading it keeps, so an abandoned probe cannot
//! claim words for the reading that owns them. A memoized rule must
//! [`observe`] on a miss and [`adopt`] the stored claims on a hit.
//!
//! Outside any scope, claiming is a no-op.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::io::Write as _;
use std::panic::{self, AssertUnwindSafe, Location};
use std::sync::OnceLock;

use crate::lexer::{OwnedLexToken, TokenKind};
use ironsmith_compiler_api::TextSpan;

/// Authored source positions read by a parse.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaimSet {
    spans: BTreeSet<(usize, usize, usize)>,
}

impl ClaimSet {
    fn insert(&mut self, span: TextSpan) {
        if span.end > span.start {
            self.spans.insert((span.line, span.start, span.end));
        }
    }

    pub fn contains(&self, span: TextSpan) -> bool {
        span.end <= span.start || self.spans.contains(&(span.line, span.start, span.end))
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    fn extend(&mut self, other: &ClaimSet) {
        self.spans.extend(other.spans.iter().copied());
    }
}

thread_local! {
    static STACK: RefCell<Vec<ClaimSet>> = const { RefCell::new(Vec::new()) };
    static FILTER_DEPTH: Cell<usize> = const { Cell::new(0) };
    static CARD: RefCell<String> = const { RefCell::new(String::new()) };
    // Lines already reported for the current card; recognition probes the
    // same filter many times.
    static REPORTED: RefCell<std::collections::HashSet<String>> =
        RefCell::new(std::collections::HashSet::new());
}

/// Where the audit report goes: `IRONSMITH_FILTER_CLAIM_AUDIT=<path>`. Unset,
/// nothing is scoped or reported and claiming stays a no-op.
fn audit_path() -> Option<&'static std::path::Path> {
    static PATH: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| std::env::var_os("IRONSMITH_FILTER_CLAIM_AUDIT").map(Into::into))
        .as_deref()
}

pub fn audit_enabled() -> bool {
    audit_path().is_some()
}

/// Name the card whose filters the next report lines describe.
pub fn begin_card(name: &str) {
    if audit_enabled() {
        CARD.with(|card| card.replace(name.to_string()));
        REPORTED.with(|reported| reported.borrow_mut().clear());
    }
}

pub fn is_recording() -> bool {
    STACK.with(|stack| !stack.borrow().is_empty())
}

/// Record that the current reading read the token at `span`.
pub fn claim(span: TextSpan) {
    STACK.with(|stack| {
        if let Some(top) = stack.borrow_mut().last_mut() {
            top.insert(span);
        }
    });
}

/// Record that the current reading read every token in `tokens`.
pub fn claim_tokens(tokens: &[OwnedLexToken]) {
    if !is_recording() {
        return;
    }
    for token in tokens {
        claim(token.span);
    }
}

/// Run `f` in a fresh scope and return what it claimed. The claims are not
/// passed to the enclosing scope; call [`adopt`] for a reading that is kept.
pub fn scoped<T>(f: impl FnOnce() -> T) -> (T, ClaimSet) {
    STACK.with(|stack| stack.borrow_mut().push(ClaimSet::default()));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    let claims = STACK.with(|stack| stack.borrow_mut().pop()).unwrap_or_default();
    match result {
        Ok(result) => (result, claims),
        Err(payload) => panic::resume_unwind(payload),
    }
}

/// Add claims a kept reading made (or a memoized one made earlier) to the
/// enclosing scope.
pub fn adopt(claims: &ClaimSet) {
    if claims.is_empty() {
        return;
    }
    STACK.with(|stack| {
        if let Some(top) = stack.borrow_mut().last_mut() {
            top.extend(claims);
        }
    });
}

/// Run `f`, passing its claims to the enclosing scope as usual, and also
/// return a copy so a memoized rule can [`adopt`] them on a later cache hit,
/// which skips the code that claims.
pub fn observe<T>(f: impl FnOnce() -> T) -> (T, ClaimSet) {
    // In audit mode always record: a cache entry filled outside any scope
    // would otherwise replay no claims for a later hit inside one.
    if !is_recording() && !audit_enabled() {
        return (f(), ClaimSet::default());
    }
    let (result, claims) = scoped(f);
    adopt(&claims);
    (result, claims)
}

/// Run one object-filter parse of `tokens` and, in audit mode, report the
/// authored words it did not read. Only the outermost filter parse reports:
/// a nested sub-phrase's claims are adopted by its parent, so its unread
/// words appear in the parent's line.
pub fn audit_filter<T, E>(
    tokens: &[OwnedLexToken],
    caller: &'static Location<'static>,
    f: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    if !audit_enabled() {
        return f();
    }
    struct Depth;
    impl Drop for Depth {
        fn drop(&mut self) {
            FILTER_DEPTH.with(|depth| depth.set(depth.get() - 1));
        }
    }
    let outermost = FILTER_DEPTH.with(|depth| {
        depth.set(depth.get() + 1);
        depth.get() == 1
    });
    let guard = Depth;
    let (result, claims) = scoped(f);
    drop(guard);
    if result.is_ok() {
        if outermost {
            report(tokens, &claims, caller);
        }
        adopt(&claims);
    }
    result
}

fn report(tokens: &[OwnedLexToken], claims: &ClaimSet, caller: &'static Location<'static>) {
    let unclaimed = unclaimed_words(tokens, claims);
    let Some(path) = audit_path() else {
        return;
    };
    if unclaimed.is_empty() {
        return;
    }
    let card = CARD.with(|card| card.borrow().clone());
    let line = format!(
        "{card}\t{}\t{}\t{}:{}\n",
        unclaimed.join(" "),
        crate::lexer::token_word_refs(tokens).join(" "),
        caller.file(),
        caller.line(),
    );
    if !REPORTED.with(|reported| reported.borrow_mut().insert(line.clone())) {
        return;
    }
    // One append per line keeps lines whole across rayon worker threads.
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// The authored words in `tokens` that `claims` does not cover.
pub fn unclaimed_words<'a>(tokens: &'a [OwnedLexToken], claims: &ClaimSet) -> Vec<&'a str> {
    tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Word && !claims.contains(token.span))
        .filter_map(OwnedLexToken::as_word)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: usize, end: usize) -> TextSpan {
        TextSpan { line: 1, start, end }
    }

    #[test]
    fn claims_outside_a_scope_are_dropped() {
        claim(span(0, 4));
        let ((), claims) = scoped(|| {});
        assert!(claims.is_empty());
    }

    #[test]
    fn nested_scope_claims_reach_the_parent_only_when_adopted() {
        let ((), outer) = scoped(|| {
            let ((), abandoned) = scoped(|| claim(span(0, 4)));
            assert!(abandoned.contains(span(0, 4)));
            let ((), kept) = scoped(|| claim(span(5, 9)));
            adopt(&kept);
        });
        assert!(!outer.contains(span(0, 4)));
        assert!(outer.contains(span(5, 9)));
    }

    #[test]
    fn observe_passes_claims_up_and_keeps_a_copy_for_replay() {
        let ((), outer) = scoped(|| {
            let ((), copy) = observe(|| claim(span(0, 4)));
            assert!(copy.contains(span(0, 4)));
        });
        assert!(outer.contains(span(0, 4)));
    }

    #[test]
    fn synthetic_spans_count_as_claimed() {
        let claims = ClaimSet::default();
        assert!(claims.contains(TextSpan::synthetic()));
        assert!(!claims.contains(span(0, 4)));
    }
}
