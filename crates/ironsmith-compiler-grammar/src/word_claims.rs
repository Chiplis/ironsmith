//! Object-filter claim auditing over the syntax crate's word-claim recorder.
//!
//! The recorder (scopes, claims, the word registry) lives in
//! `ironsmith_compiler_syntax::word_claims` beside the word primitives that
//! claim. This module adds the filter audit: each outermost object-filter
//! parse leaves an obligation for every authored word it did not read, and
//! the card parse reports (or, when enforcing, fails on) the obligations no
//! reader anywhere in the card claimed
//! (architecture/filter-word-consumption-plan.md, "The check is per card").

use std::cell::{Cell, RefCell};
use std::io::Write as _;
use std::panic::Location;
use std::sync::OnceLock;

use crate::lexer::OwnedLexToken;
pub use ironsmith_compiler_syntax::word_claims::*;

thread_local! {
    static FILTER_DEPTH: Cell<usize> = const { Cell::new(0) };
    static CARD_DEPTH: Cell<usize> = const { Cell::new(0) };
    static CARD: RefCell<String> = const { RefCell::new(String::new()) };
    // Lines already reported for the current card; outside a card parse the
    // same filter is probed many times.
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

/// Whether a card whose filters leave authored words that nothing reads
/// fails (`IRONSMITH_FILTER_CLAIM_ENFORCE`). Off by default until the
/// readers those words reach all claim (plan stage 5).
fn enforce_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("IRONSMITH_FILTER_CLAIM_ENFORCE").is_some())
}

fn depth_guard(depth: &'static std::thread::LocalKey<Cell<usize>>) -> impl Drop {
    struct Guard(&'static std::thread::LocalKey<Cell<usize>>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.with(|depth| depth.set(depth.get() - 1));
        }
    }
    depth.with(|depth| depth.set(depth.get() + 1));
    Guard(depth)
}

/// Parse one card inside a single recording scope. In audit mode, report
/// the words its object filters left unread that no reader in the card
/// claimed; when enforcing, those words fail the card. A card parse nested
/// in another (a quoted ability) joins the enclosing scope.
pub fn audit_card<T: std::fmt::Debug>(
    name: &str,
    f: impl FnOnce() -> Result<T, crate::CardTextError>,
) -> Result<T, crate::CardTextError> {
    if !audit_enabled() || CARD_DEPTH.with(Cell::get) > 0 {
        return f();
    }
    CARD.with(|card| card.replace(name.to_string()));
    REPORTED.with(|reported| reported.borrow_mut().clear());
    reset_registry();
    let guard = depth_guard(&CARD_DEPTH);
    let (result, claims) = scoped(f);
    drop(guard);
    reset_registry();
    result.and_then(|value| {
        // Only a filter the card kept counts: a probe of the wrong span
        // produces a filter that is not in the parsed document.
        let mut violations = claims.violations();
        if violations.is_empty() {
            return Ok(value);
        }
        let document = comparable_debug(&format!("{value:?}"));
        violations.retain(|obligation| document.contains(&*obligation.filter));
        if violations.is_empty() {
            return Ok(value);
        }
        // One row per filter input and caller, words in source order.
        let mut rows: Vec<(&str, &'static Location<'static>, Vec<&str>)> = Vec::new();
        for obligation in violations {
            match rows.iter_mut().find(|(input, caller, _)| {
                *input == &*obligation.input && *caller == obligation.caller
            }) {
                Some(row) => row.2.push(&obligation.word),
                None => rows.push((&obligation.input, obligation.caller, vec![&obligation.word])),
            }
        }
        for (input, caller, words) in &rows {
            write_report(name, &words.join(" "), input, caller);
        }
        if enforce_enabled() {
            let (input, _, words) = &rows[0];
            return Err(crate::CardTextError::ParseError(format!(
                "object filter does not read '{}' (full input: '{}')",
                words.join(" "),
                input,
            )));
        }
        Ok(value)
    })
}

/// Run one object-filter parse of `tokens`. In audit mode its claims reach
/// the enclosing scope, and the outermost filter parse adds an obligation
/// for each authored word of its input it did not read. A nested
/// sub-phrase's claims are adopted by its parent, so only the outermost
/// parse decides what was left unread. Unread words never fail the filter
/// itself: callers probe filters speculatively and branch on the result.
pub fn audit_filter<T: std::fmt::Debug>(
    tokens: &[OwnedLexToken],
    caller: &'static Location<'static>,
    f: impl FnOnce() -> Result<T, crate::CardTextError>,
) -> Result<T, crate::CardTextError> {
    if !audit_enabled() {
        return f();
    }
    let in_card = CARD_DEPTH.with(Cell::get) > 0;
    let outermost = FILTER_DEPTH.with(|depth| depth.get() == 0);
    if outermost && !in_card {
        reset_registry();
    }
    let guard = depth_guard(&FILTER_DEPTH);
    let (result, mut claims) = scoped(f);
    drop(guard);
    if result.is_ok() {
        if outermost {
            CALLER_OWNED.with(|owned| owned.borrow().iter().for_each(|set| claims.extend(set)));
            if in_card {
                let filter = result.as_ref().ok();
                claims.oblige_unclaimed(
                    tokens,
                    || comparable_debug(&format!("{:?}", filter.expect("ok result"))),
                    caller,
                );
            } else {
                report(tokens, &claims, caller);
            }
        }
        adopt(&claims);
    }
    result
}

/// A filter's debug text with the fields its callers routinely finish
/// (the zone a static ability applies in) blanked, so a kept filter is
/// found in the document after its caller completed it.
fn comparable_debug(text: &str) -> String {
    text.replace("zone: Some(Battlefield)", "zone: _")
        .replace("zone: None", "zone: _")
}

/// Report a filter parse outside any card parse (lowering, tools), which
/// no card-level check covers.
fn report(tokens: &[OwnedLexToken], claims: &ClaimSet, caller: &'static Location<'static>) {
    if audit_path().is_none() {
        return;
    }
    let unclaimed = unclaimed_words(tokens, claims);
    if unclaimed.is_empty() {
        return;
    }
    let card = CARD.with(|card| format!("<outside> {}", card.borrow()));
    let input = crate::lexer::token_word_refs(tokens).join(" ");
    write_report(&card, &unclaimed.join(" "), &input, caller);
}

fn write_report(card: &str, words: &str, input: &str, caller: &'static Location<'static>) {
    let Some(path) = audit_path() else {
        return;
    };
    let line = format!(
        "{card}\t{words}\t{input}\t{}:{}\n",
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

