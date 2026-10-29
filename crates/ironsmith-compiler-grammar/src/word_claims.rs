//! Object-filter claim auditing over the syntax crate's word-claim recorder.
//!
//! The recorder (scopes, claims, the word registry) lives in
//! `ironsmith_compiler_syntax::word_claims` beside the word primitives that
//! claim. This module adds the filter audit: each outermost object-filter
//! parse reports the authored words it did not read
//! (architecture/filter-word-consumption-plan.md).

use std::cell::{Cell, RefCell};
use std::io::Write as _;
use std::panic::Location;
use std::sync::OnceLock;

use crate::lexer::OwnedLexToken;
pub use ironsmith_compiler_syntax::word_claims::*;

thread_local! {
    static FILTER_DEPTH: Cell<usize> = const { Cell::new(0) };
    static CARD: RefCell<String> = const { RefCell::new(String::new()) };
    // Lines already reported for the current card; recognition probes the
    // same filter many times.
    static REPORTED: RefCell<std::collections::HashSet<String>> =
        RefCell::new(std::collections::HashSet::new());
    // Words callers read themselves around the filter they parse.
    static CALLER_OWNED: RefCell<Vec<ClaimSet>> = const { RefCell::new(Vec::new()) };
}

/// Parse a filter whose input carries words the caller reads itself (an
/// ordinal "your second spell each turn" around the spell filter). The
/// owned words count as read for the outermost filter parse inside `f`.
pub fn with_caller_owned<'a, T>(
    owned: impl IntoIterator<Item = &'a OwnedLexToken>,
    f: impl FnOnce() -> T,
) -> T {
    if !audit_enabled() {
        return f();
    }
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            CALLER_OWNED.with(|owned| owned.borrow_mut().pop());
        }
    }
    CALLER_OWNED.with(|stack| stack.borrow_mut().push(ClaimSet::of_tokens(owned)));
    let _pop = Pop;
    f()
}

/// Where the audit report goes: `IRONSMITH_FILTER_CLAIM_AUDIT=<path>`.
fn audit_path() -> Option<&'static std::path::Path> {
    static PATH: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| std::env::var_os("IRONSMITH_FILTER_CLAIM_AUDIT").map(Into::into))
        .as_deref()
}

/// Whether an object filter that leaves authored words unread fails
/// (`IRONSMITH_FILTER_CLAIM_ENFORCE`). Off by default until every caller
/// passes only the words its filter reads (plan stage 5).
fn enforce_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("IRONSMITH_FILTER_CLAIM_ENFORCE").is_some())
}

/// Name the card whose filters the next report lines describe.
pub fn begin_card(name: &str) {
    if audit_enabled() {
        CARD.with(|card| card.replace(name.to_string()));
        REPORTED.with(|reported| reported.borrow_mut().clear());
    }
}

/// Run one object-filter parse of `tokens` and, in audit mode, report the
/// authored words it did not read; when enforcing, those words fail the
/// parse. Only the outermost filter parse checks: a nested sub-phrase's
/// claims are adopted by its parent, so its unread words appear there.
pub fn audit_filter<T>(
    tokens: &[OwnedLexToken],
    caller: &'static Location<'static>,
    f: impl FnOnce() -> Result<T, crate::CardTextError>,
) -> Result<T, crate::CardTextError> {
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
    if outermost {
        reset_registry();
    }
    let guard = Depth;
    let (result, mut claims) = scoped(f);
    drop(guard);
    if outermost {
        CALLER_OWNED.with(|owned| owned.borrow().iter().for_each(|set| claims.extend(set)));
    }
    if result.is_ok() {
        if outermost {
            report(tokens, &claims, caller);
            let unread = unclaimed_words(tokens, &claims);
            if enforce_enabled() && !unread.is_empty() {
                return Err(crate::CardTextError::ParseError(format!(
                    "object filter does not read '{}' (full input: '{}')",
                    unread.join(" "),
                    crate::lexer::token_word_refs(tokens).join(" "),
                )));
            }
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

