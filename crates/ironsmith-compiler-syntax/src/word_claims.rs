//! Which authored words a parse actually read.
//!
//! The object-filter grammar scans words rather than consuming them, so an
//! unknown word ("blorp creatures you control") is silently ignored and the
//! filter means something other than what the card says. Readers record the
//! words they read here; a filter parse can then compare its input with what
//! was read (architecture/filter-word-consumption-plan.md).
//!
//! A claim is the source span of a word piece, not an index into a slice, so
//! it survives the copying, slicing, article removal and sub-phrase recursion
//! the filter code does to its input. Synthetic pieces have an empty span and
//! are never claimed: whoever inserted them already read the words they
//! replace.
//!
//! Most readers hold `&str` words rather than tokens. A word borrowed from a
//! token's word pieces is identified by its address for as long as the token
//! lives, so the functions that hand out word slices register each word's
//! span here while recording, and [`claim_word`] looks it up. If memory is
//! reused, the latest registration wins, and the registry is cleared at each
//! outermost scope.
//!
//! Like parse-loss recording, this is a side channel. Claims made inside a
//! [`scoped`] call are returned to its caller, which [`adopt`]s them into the
//! enclosing scope only for a reading it keeps, so an abandoned probe cannot
//! claim words for the reading that owns them. A memoized rule must
//! [`observe`] on a miss and [`adopt`] the stored claims on a hit.
//!
//! Outside any scope every function here is a no-op.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::panic::{self, AssertUnwindSafe};

use crate::diagnostics::TextSpan;
use crate::lexer::OwnedLexToken;

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

    pub fn extend(&mut self, other: &ClaimSet) {
        self.spans.extend(other.spans.iter().copied());
    }

    /// The word pieces of `tokens`.
    pub fn of_tokens<'a>(tokens: impl IntoIterator<Item = &'a OwnedLexToken>) -> Self {
        let mut claims = Self::default();
        for token in tokens.into_iter().filter(|token| !is_synthetic(token)) {
            for piece in token.parser_word_pieces() {
                claims.insert(piece.span);
            }
        }
        claims
    }
}

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static STACK: RefCell<Vec<ClaimSet>> = const { RefCell::new(Vec::new()) };
    static WORD_SPANS: RefCell<HashMap<(usize, usize), Vec<TextSpan>>> =
        RefCell::new(HashMap::new());
}

/// Whether claims are recorded for this process: when auditing
/// (`IRONSMITH_FILTER_CLAIM_AUDIT`) or enforcing
/// (`IRONSMITH_FILTER_CLAIM_ENFORCE`).
pub fn audit_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var_os("IRONSMITH_FILTER_CLAIM_AUDIT").is_some()
            || std::env::var_os("IRONSMITH_FILTER_CLAIM_ENFORCE").is_some()
    })
}

pub fn is_recording() -> bool {
    DEPTH.with(Cell::get) > 0
}

/// A token the compiler inserted has an empty span; its word pieces still
/// carry offsets from that empty base, so the token's own span decides.
fn is_synthetic(token: &OwnedLexToken) -> bool {
    token.span.end <= token.span.start
}

fn key(word: &str) -> (usize, usize) {
    (word.as_ptr() as usize, word.len())
}

/// Remember where the words of `tokens` come from, so a reader holding only
/// `&str` words can claim them. Called by the functions that hand out word
/// slices; a no-op unless recording.
pub fn register_tokens(tokens: &[OwnedLexToken]) {
    if !is_recording() {
        return;
    }
    WORD_SPANS.with(|spans| {
        let mut spans = spans.borrow_mut();
        for token in tokens.iter().filter(|token| !is_synthetic(token)) {
            let pieces = token.parser_word_pieces();
            for piece in pieces {
                spans.insert(key(&piece.text), vec![piece.span]);
            }
            if let Some(word) = token.as_word() {
                spans.insert(key(word), pieces.iter().map(|piece| piece.span).collect());
            }
        }
    });
}

/// Forget registered words. Called at the outermost scope.
pub fn reset_registry() {
    WORD_SPANS.with(|spans| spans.borrow_mut().clear());
}

/// Record that the current reading read the word piece at `span`.
pub fn claim(span: TextSpan) {
    if !is_recording() {
        return;
    }
    STACK.with(|stack| {
        if let Some(top) = stack.borrow_mut().last_mut() {
            top.insert(span);
        }
    });
}

/// Record that the current reading read `word`, a word handed out by one of
/// the registering functions. Words of unknown origin are ignored.
pub fn claim_word(word: &str) {
    if !is_recording() {
        return;
    }
    let spans = WORD_SPANS.with(|spans| spans.borrow().get(&key(word)).cloned());
    for span in spans.into_iter().flatten() {
        claim(span);
    }
}

pub fn claim_words(words: &[&str]) {
    if !is_recording() {
        return;
    }
    for word in words {
        claim_word(word);
    }
}

/// Record that the current reading read every word piece of `tokens`.
pub fn claim_tokens(tokens: &[OwnedLexToken]) {
    if !is_recording() {
        return;
    }
    for token in tokens.iter().filter(|token| !is_synthetic(token)) {
        for piece in token.parser_word_pieces() {
            claim(piece.span);
        }
    }
}

/// A word or token a reader can drain out of its working list.
pub trait Claimable {
    fn claim_item(&self);
}

impl Claimable for &str {
    fn claim_item(&self) {
        claim_word(self);
    }
}

impl Claimable for OwnedLexToken {
    fn claim_item(&self) {
        claim_tokens(std::slice::from_ref(self));
    }
}

/// Claim what a reader removes from its working list: removing words is how
/// the filter grammar marks a clause handled.
pub fn claim_drained<T: Claimable>(items: impl IntoIterator<Item = T>) {
    for item in items {
        item.claim_item();
    }
}

/// Run `f` in a fresh scope and return what it claimed. The claims are not
/// passed to the enclosing scope; call [`adopt`] for a reading that is kept.
pub fn scoped<T>(f: impl FnOnce() -> T) -> (T, ClaimSet) {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            DEPTH.with(|depth| depth.set(depth.get() - 1));
        }
    }
    STACK.with(|stack| stack.borrow_mut().push(ClaimSet::default()));
    DEPTH.with(|depth| depth.set(depth.get() + 1));
    let pop = Pop;
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    drop(pop);
    let claims = STACK.with(|stack| stack.borrow_mut().pop()).unwrap_or_default();
    match result {
        Ok(result) => (result, claims),
        Err(payload) => panic::resume_unwind(payload),
    }
}

/// Add claims a kept reading made (or a memoized one made earlier) to the
/// enclosing scope.
pub fn adopt(claims: &ClaimSet) {
    if claims.is_empty() || !is_recording() {
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

/// The authored words in `tokens` that `claims` does not cover.
pub fn unclaimed_words<'a>(tokens: &'a [OwnedLexToken], claims: &ClaimSet) -> Vec<&'a str> {
    tokens
        .iter()
        .filter(|token| token.kind == crate::lexer::TokenKind::Word && !is_synthetic(token))
        .flat_map(|token| token.parser_word_pieces())
        .filter(|piece| !claims.contains(piece.span))
        .map(|piece| piece.text.as_str())
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

    #[test]
    fn a_registered_word_is_claimed_by_its_text_address() {
        let tokens = crate::lexer::lex_line("creatures you control", 1).unwrap();
        let ((), claims) = scoped(|| {
            let words = crate::lexer::parser_token_word_refs(&tokens);
            claim_word(words[1]);
        });
        assert_eq!(unclaimed_words(&tokens, &claims), vec!["creatures", "control"]);
    }
}
