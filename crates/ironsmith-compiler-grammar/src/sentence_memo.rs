//! One parse per span: the sentence rules are memoized for the current card.
//!
//! Recognition is an ordered choice over many line and sentence shapes, and
//! most of those shapes need the effect clause parsed before they can decide
//! whether the line is theirs. Without memoization each candidate parsed that
//! clause again — measured at roughly fifteen sentence-rule calls per distinct
//! span across the corpus — and, worse, a shape that *probed* a span could see
//! a different result from the shape that eventually *kept* it if anything in
//! between was not a pure function of the tokens. Memoizing at the rule makes
//! both problems go away by construction: every distinct span is parsed once,
//! and every caller sees that one parse.
//!
//! The rules are pure functions of their tokens except for one side channel,
//! parse-loss recording, which is observed on a miss and replayed on a hit so
//! the report a caller captures does not depend on cache state.
//!
//! The cache is thread-local and cleared at the document boundary — a card is
//! the unit of recognition — and bounded so a long-lived test thread cannot
//! grow it without limit.

use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::Location;

use crate::lexer::OwnedLexToken;
use crate::model::ast::EffectAst;
use crate::parse_loss::ParseLossDiagnostic;
use ironsmith_compiler_api::CardTextError;

/// Which rule a cached entry belongs to; the same span parsed by the sentence
/// rule and by the sentence-list rule is two different results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rule {
    Sentence,
    Sentences,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SpanKey {
    rule: Rule,
    tokens: String,
}

#[derive(Debug, Clone)]
struct Entry {
    result: Result<Vec<EffectAst>, CardTextError>,
    loss: Vec<ParseLossDiagnostic>,    /// The reference keys the parse minted, replayed on every hit.
    minted: Vec<ironsmith_compiler_ast::reference_ledger::MintedReference>,
}

const MAX_ENTRIES: usize = 8_192;

thread_local! {
    static MEMO: RefCell<HashMap<SpanKey, Entry>> = RefCell::new(HashMap::new());
    // Keys whose parse is still running on this thread. A nested call for one
    // of them is a recognizer re-entering the rule for its own span — a guarded
    // recursion, not a repeat — and it cannot be answered from the cache.
    static IN_PROGRESS: RefCell<std::collections::HashSet<SpanKey>> =
        RefCell::new(std::collections::HashSet::new());
}

pub(crate) fn span_key(rule: Rule, tokens: &[OwnedLexToken]) -> SpanKey {
    use std::fmt::Write as _;
    let mut key = String::with_capacity(tokens.len() * 24);
    for token in tokens {
        // Every field a rule can read goes into the key. `slice` is the
        // authored text, `parser_text` the normalized one, and the span
        // distinguishes the same words at two positions.
        let _ = write!(
            key,
            "{:?}\u{1f}{}\u{1f}{}\u{1f}{}:{}:{}\u{1e}",
            token.kind,
            token.slice,
            token.parser_text,
            token.span.line,
            token.span.start,
            token.span.end
        );
    }
    SpanKey { rule, tokens: key }
}

/// Forget everything cached. Called where a new card begins.
pub fn reset() {
    crate::parse_ledger::note_reset();
    MEMO.with(|memo| memo.borrow_mut().clear());
    IN_PROGRESS.with(|keys| keys.borrow_mut().clear());
}

/// Parse `tokens` with `rule`, computing with `compute` only if this span has
/// not been parsed by that rule on this card already.
pub fn memoized(
    rule: Rule,
    tokens: &[OwnedLexToken],
    caller: &'static Location<'static>,
    compute: impl FnOnce() -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<Vec<EffectAst>, CardTextError> {
    // A trace wants to see the rules fire, not a cache answer for them.
    if crate::parse_trace::is_enabled() {
        crate::parse_ledger::note(rule, tokens, caller);
        return compute();
    }
    let key = span_key(rule, tokens);
    let hit = MEMO.with(|memo| memo.borrow().get(&key).cloned());
    if let Some(entry) = hit {
        crate::parse_ledger::note_hit();
        crate::parse_loss::replay(&entry.loss);
        // The keys the parse minted belong to this use of the span as well.
        ironsmith_compiler_ast::reference_ledger::replay(&entry.minted);
        return entry.result;
    }
    let reentrant = IN_PROGRESS.with(|keys| !keys.borrow_mut().insert(key.clone()));
    if reentrant {
        crate::parse_ledger::note_reentrant(rule, tokens, caller);
        return compute();
    }
    crate::parse_ledger::note(rule, tokens, caller);
    let ((result, loss), minted) =
        ironsmith_compiler_ast::reference_ledger::observe(|| crate::parse_loss::observe(compute));
    IN_PROGRESS.with(|keys| keys.borrow_mut().remove(&key));
    MEMO.with(|memo| {
        let mut memo = memo.borrow_mut();
        if memo.len() >= MAX_ENTRIES {
            memo.clear();
        }
        memo.insert(
            key,
            Entry {
                result: result.clone(),
                loss,
                minted,
            },
        );
    });
    result
}
