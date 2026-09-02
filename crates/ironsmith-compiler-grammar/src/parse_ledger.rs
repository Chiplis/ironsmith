//! A ledger of every span handed to the sentence rule, so redundant parses can
//! be counted rather than estimated.
//!
//! Recognition is allowed to *compose*: a multi-sentence recognizer hands each
//! sentence to the sentence rule once. What it is not allowed to do is hand the
//! same span to the rule twice — probe it, throw the result away, and parse it
//! again later; or let two sibling recognizers each parse the same tail. This
//! tracker tells those two apart precisely, which a grep over call sites cannot.
//!
//! The tracker is off unless a caller enables it for the current thread, and
//! the enabled check is a single thread-local read, so recognition pays nothing
//! for it in production.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::panic::Location;

use crate::lexer::OwnedLexToken;

/// One span parsed more than once: where it was first parsed and where the
/// repeat came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Repeat {
    pub first_caller: String,
    pub repeat_caller: String,
    pub text: String,
}

#[derive(Debug, Default)]
struct Tracker {
    enabled: bool,
    calls: usize,
    hits: usize,
    resets: usize,
    reentrant: Vec<Repeat>,
    first_seen: HashMap<u64, (&'static Location<'static>, String)>,
    repeats: Vec<Repeat>,
}

thread_local! {
    static TRACKER: RefCell<Tracker> = RefCell::new(Tracker::default());
}

/// Start recording on this thread. Any previous recording is discarded.
pub fn begin() {
    TRACKER.with(|t| {
        *t.borrow_mut() = Tracker {
            enabled: true,
            ..Tracker::default()
        }
    });
}

/// What was recorded since [`begin`]: sentence-rule parses actually run, cache
/// hits that stood in for a parse, and every span that was parsed more than
/// once. Stops recording.
pub fn end() -> Report {
    TRACKER.with(|t| {
        let mut tracker = t.borrow_mut();
        let taken = std::mem::take(&mut *tracker);
        Report {
            parses: taken.calls,
            hits: taken.hits,
            resets: taken.resets,
            reentrant: taken.reentrant,
            repeats: taken.repeats,
        }
    })
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    pub parses: usize,
    pub hits: usize,
    /// Times the memo was cleared while recording — a card whose text is
    /// recognized more than once will show its spans parsed once per pass.
    pub resets: usize,
    /// Calls for a span whose own parse was still running: a recognizer
    /// re-entering the rule for the span it was handed. Listed so the
    /// recursion is visible; not counted as a repeat.
    pub reentrant: Vec<Repeat>,
    pub repeats: Vec<Repeat>,
}

/// Record a call for a span whose parse is in progress on this thread.
pub fn note_reentrant(
    rule: crate::sentence_memo::Rule,
    tokens: &[OwnedLexToken],
    caller: &'static Location<'static>,
) {
    TRACKER.with(|t| {
        let mut tracker = t.borrow_mut();
        if !tracker.enabled {
            return;
        }
        let key = span_key(rule, tokens);
        let first = tracker
            .first_seen
            .get(&key)
            .map(|(first, _)| short_location(first))
            .unwrap_or_else(|| "?".to_string());
        let text = crate::lexer::token_word_refs(tokens).join(" ");
        tracker.reentrant.push(Repeat {
            first_caller: first,
            repeat_caller: short_location(caller),
            text,
        });
    });
}

/// Record that the memo was cleared: a new recognition pass begins, so what
/// the previous pass parsed is not a baseline for repeats. (A card whose
/// strict compile fails is compiled again from its oracle text; each pass
/// parses its spans once.)
pub fn note_reset() {
    TRACKER.with(|t| {
        let mut tracker = t.borrow_mut();
        if tracker.enabled {
            tracker.resets += 1;
            tracker.first_seen.clear();
        }
    });
}

/// Record a call the memo answered without parsing.
pub fn note_hit() {
    TRACKER.with(|t| {
        let mut tracker = t.borrow_mut();
        if tracker.enabled {
            tracker.hits += 1;
        }
    });
}

fn span_key(rule: crate::sentence_memo::Rule, tokens: &[OwnedLexToken]) -> u64 {
    // The same fields the memo keys on, so a repeat here is a span the memo
    // itself would have recognized as already parsed.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    crate::sentence_memo::span_key(rule, tokens).hash(&mut hasher);
    hasher.finish()
}

fn short_location(location: &Location<'_>) -> String {
    let file = location.file();
    let trimmed = file
        .rsplit_once("/src/")
        .map(|(_, tail)| tail)
        .unwrap_or(file);
    format!("{trimmed}:{}", location.line())
}

/// Record one parse actually run by `rule` for `caller`.
pub fn note(
    rule: crate::sentence_memo::Rule,
    tokens: &[OwnedLexToken],
    caller: &'static Location<'static>,
) {
    TRACKER.with(|t| {
        let mut tracker = t.borrow_mut();
        if !tracker.enabled {
            return;
        }
        tracker.calls += 1;
        let key = span_key(rule, tokens);
        if let Some((first, text)) = tracker.first_seen.get(&key) {
            let repeat = Repeat {
                first_caller: short_location(first),
                repeat_caller: short_location(caller),
                text: text.clone(),
            };
            tracker.repeats.push(repeat);
        } else {
            let text = crate::lexer::token_word_refs(tokens).join(" ");
            tracker.first_seen.insert(key, (caller, text));
        }
    });
}
