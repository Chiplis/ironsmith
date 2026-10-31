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
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::panic::{self, AssertUnwindSafe, Location};
use std::rc::Rc;

use crate::diagnostics::TextSpan;
use crate::lexer::OwnedLexToken;

type SpanKey = (usize, usize, usize);

fn filter_hash(filter: &str) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    filter.hash(&mut hasher);
    hasher.finish()
}

fn span_key(span: TextSpan) -> SpanKey {
    (span.line, span.start, span.end)
}

/// A word an object filter was given but did not read. It is a drop only if
/// no other reader in the same card parse claims it either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Obligation {
    pub word: String,
    /// The filter's whole input, for the report.
    pub input: Rc<str>,
    /// The filter the parse produced, so the card check can tell whether
    /// the reading that left the word unread is the one the card kept.
    pub filter: Rc<str>,
    pub caller: &'static Location<'static>,
    /// The ids (`ClaimOrigin`) stamped on the filters the parses that left
    /// the word unread returned, so the card check can find those exact
    /// filters in the document whatever lowering did to their text.
    pub origins: Vec<u32>,
}

/// Authored source positions read by a parse, and the words its object
/// filters left unread.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaimSet {
    spans: BTreeSet<SpanKey>,
    // Every distinct filter reading that left a span unread: each is judged
    // on its own, so a probe's obligation cannot mask a kept filter's.
    obligations: BTreeMap<SpanKey, Vec<Obligation>>,
    // Filters (by hash of their comparable debug text) some parse produced
    // while reading every word it was given.
    clean_filters: BTreeSet<u64>,
}

impl ClaimSet {
    fn insert(&mut self, span: TextSpan) {
        if span.end > span.start {
            self.spans.insert(span_key(span));
        }
    }

    pub fn contains(&self, span: TextSpan) -> bool {
        span.end <= span.start || self.spans.contains(&span_key(span))
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() && self.obligations.is_empty() && self.clean_filters.is_empty()
    }

    pub fn extend(&mut self, other: &ClaimSet) {
        self.spans.extend(other.spans.iter().copied());
        for (span, obligations) in &other.obligations {
            for obligation in obligations {
                self.add_obligation(*span, obligation);
            }
        }
        self.clean_filters.extend(other.clean_filters.iter().copied());
    }

    fn add_obligation(&mut self, span: SpanKey, obligation: &Obligation) {
        let existing = self.obligations.entry(span).or_default();
        // The same reading reached twice (a memo replay, a re-parse) is one
        // obligation that any of its filters can show was kept.
        match existing.iter_mut().find(|known| {
            known.word == obligation.word
                && known.input == obligation.input
                && known.filter == obligation.filter
                && known.caller == obligation.caller
        }) {
            Some(known) => {
                for origin in &obligation.origins {
                    if !known.origins.contains(origin) {
                        known.origins.push(*origin);
                    }
                }
            }
            None => existing.push(obligation.clone()),
        }
    }

    /// Whether `tokens` has an authored word outside `self`.
    pub fn leaves_unread(&self, tokens: &[OwnedLexToken]) -> bool {
        authored_pieces(tokens).any(|piece| !self.contains(piece.span))
    }

    /// Record that a parse produced `filter` while reading all its input.
    pub fn note_clean_filter(&mut self, filter: &str) {
        self.clean_filters.insert(filter_hash(filter));
    }

    /// Whether some parse produced `filter` while reading all its input, so
    /// a kept filter with that text need not be the one that dropped words.
    pub fn has_clean_filter(&self, filter: &str) -> bool {
        self.clean_filters.contains(&filter_hash(filter))
    }

    /// Record that a filter given `tokens` left their words outside `self`
    /// unread. `origin` is the id stamped on the filter the parse returned.
    pub fn oblige_unclaimed(
        &mut self,
        tokens: &[OwnedLexToken],
        filter: impl FnOnce() -> String,
        origin: Option<u32>,
        caller: &'static Location<'static>,
    ) {
        let mut filter = Some(filter);
        let mut filter_text: Option<Rc<str>> = None;
        let mut input: Option<Rc<str>> = None;
        for piece in authored_pieces(tokens) {
            if self.contains(piece.span) {
                continue;
            }
            let input = input
                .get_or_insert_with(|| Rc::from(crate::lexer::token_word_refs(tokens).join(" ")))
                .clone();
            let filter = filter_text
                .get_or_insert_with(|| Rc::from(filter.take().map(|f| f()).unwrap_or_default()))
                .clone();
            self.add_obligation(
                span_key(piece.span),
                &Obligation {
                    word: piece.text.clone(),
                    input,
                    filter,
                    caller,
                    origins: origin.into_iter().collect(),
                },
            );
        }
    }

    /// Report every obligation in this set against `input`.
    pub fn set_obligation_inputs(&mut self, input: String) {
        let input: Rc<str> = Rc::from(input);
        for obligation in self.obligations.values_mut().flatten() {
            obligation.input = input.clone();
        }
    }

    /// [`Self::oblige_unclaimed`] for a filter given registered `&str`
    /// words; words of unknown origin are skipped.
    pub fn oblige_unclaimed_words(
        &mut self,
        words: &[&str],
        filter: impl FnOnce() -> String,
        origin: Option<u32>,
        caller: &'static Location<'static>,
    ) {
        let mut filter = Some(filter);
        let mut filter_text: Option<Rc<str>> = None;
        let mut input: Option<Rc<str>> = None;
        for word in words {
            for span in word_spans(word) {
                if self.contains(span) {
                    continue;
                }
                let input = input.get_or_insert_with(|| Rc::from(words.join(" "))).clone();
                let filter = filter_text
                    .get_or_insert_with(|| Rc::from(filter.take().map(|f| f()).unwrap_or_default()))
                    .clone();
                self.add_obligation(
                    span_key(span),
                    &Obligation {
                        word: word.to_string(),
                        input,
                        filter,
                        caller,
                        origins: origin.into_iter().collect(),
                    },
                );
            }
        }
    }

    /// Whether a registered word of `words` has a span outside `self`.
    pub fn leaves_words_unread(&self, words: &[&str]) -> bool {
        words.iter().any(|word| word_spans(word).into_iter().any(|span| !self.contains(span)))
    }

    /// The claimed source positions as `(line, start, end)`, in source
    /// order (debugging and traces).
    pub fn claimed_spans(&self) -> Vec<(usize, usize, usize)> {
        self.spans.iter().copied().collect()
    }

    /// Every obligation with whether some claim covers it (debugging).
    pub fn obligations_debug(&self) -> Vec<(&Obligation, bool)> {
        self.obligations
            .iter()
            .flat_map(|(span, obligations)| {
                let claimed = self.spans.contains(span);
                obligations.iter().map(move |obligation| (obligation, claimed))
            })
            .collect()
    }

    /// Obligations no claim in this set covers, in source order; a span
    /// several filters left unread yields one per filter reading.
    pub fn violations(&self) -> Vec<&Obligation> {
        self.obligations
            .iter()
            .filter(|(span, _)| !self.spans.contains(span))
            .flat_map(|(_, obligations)| obligations)
            .collect()
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
    // Keyed by address and length; the text hash guards against a freed
    // word's address being reused by a different word of the same length,
    // and a token forgets its words when it drops (`forget_token_words`).
    static WORD_SPANS: RefCell<HashMap<(usize, usize), (u64, Vec<TextSpan>)>> =
        RefCell::new(HashMap::new());
    // Whether WORD_SPANS may hold entries, so a token drop outside the audit
    // costs one flag read.
    static REGISTERED: Cell<bool> = const { Cell::new(false) };
    // Set while a caller asked for enforcement on this thread
    // (`with_enforcement`), independent of the process environment.
    static ENFORCED: Cell<bool> = const { Cell::new(false) };
    // Set while a caller asked for whole-line enforcement on this thread
    // (`with_line_enforcement`).
    static LINE_ENFORCED: Cell<bool> = const { Cell::new(false) };
}

/// Whether claims are recorded: when auditing
/// (`IRONSMITH_FILTER_CLAIM_AUDIT`, `IRONSMITH_LINE_CLAIM_AUDIT`) or
/// enforcing, for the process (`IRONSMITH_FILTER_CLAIM_ENFORCE`,
/// `IRONSMITH_LINE_CLAIM_ENFORCE`) or inside [`with_enforcement`] /
/// [`with_line_enforcement`].
pub fn audit_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var_os("IRONSMITH_FILTER_CLAIM_AUDIT").is_some()
            || std::env::var_os("IRONSMITH_FILTER_CLAIM_ENFORCE").is_some()
            || line_audit_env()
    }) || enforcement_requested()
        || line_enforcement_requested()
}

/// Whether the process asked for the whole-line audit or its enforcement.
fn line_audit_env() -> bool {
    std::env::var_os("IRONSMITH_LINE_CLAIM_AUDIT").is_some()
        || std::env::var_os("IRONSMITH_LINE_CLAIM_ENFORCE").is_some()
}

/// Whether the card check covers every authored word of a card's lines, not
/// only its object filters' inputs: for the process
/// (`IRONSMITH_LINE_CLAIM_AUDIT`, `IRONSMITH_LINE_CLAIM_ENFORCE`) or inside
/// [`with_line_enforcement`].
pub fn line_audit_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(line_audit_env) || line_enforcement_requested()
}

/// Compile inside `f` with whole-line enforcement: a card with an authored
/// word no reader reads fails, as under `IRONSMITH_LINE_CLAIM_ENFORCE`, but
/// only for this call on this thread. Independent of [`with_enforcement`].
pub fn with_line_enforcement<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            LINE_ENFORCED.with(|enforced| enforced.set(self.0));
        }
    }
    let _restore = Restore(LINE_ENFORCED.with(|enforced| enforced.replace(true)));
    f()
}

/// Whether a caller on this thread asked for whole-line enforcement.
pub fn line_enforcement_requested() -> bool {
    LINE_ENFORCED.with(Cell::get)
}

/// Compile inside `f` in the strict mode: a card whose object filters leave
/// authored words no reader reads fails, as under
/// `IRONSMITH_FILTER_CLAIM_ENFORCE`, but only for this call on this thread
/// (the card builder's compiles).
pub fn with_enforcement<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            ENFORCED.with(|enforced| enforced.set(self.0));
        }
    }
    let _restore = Restore(ENFORCED.with(|enforced| enforced.replace(true)));
    f()
}

/// Whether a caller on this thread asked for enforcement.
pub fn enforcement_requested() -> bool {
    ENFORCED.with(Cell::get)
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
    REGISTERED.with(|registered| registered.set(true));
    WORD_SPANS.with(|spans| {
        let mut spans = spans.borrow_mut();
        for token in tokens.iter().filter(|token| !is_synthetic(token)) {
            let pieces = token.parser_word_pieces();
            for piece in pieces {
                spans.insert(key(&piece.text), (filter_hash(&piece.text), vec![piece.span]));
            }
            let whole: Vec<TextSpan> = pieces.iter().map(|piece| piece.span).collect();
            if let Some(word) = token.as_word() {
                spans.insert(key(word), (filter_hash(word), whole.clone()));
            }
            spans.insert(key(token.parser_text()), (filter_hash(token.parser_text()), whole));
        }
    });
}

/// The authored spans of a registered `word`; empty for words of unknown
/// origin.
pub fn word_spans(word: &str) -> Vec<TextSpan> {
    registered_spans(word).unwrap_or_default()
}

fn registered_spans(word: &str) -> Option<Vec<TextSpan>> {
    WORD_SPANS.with(|spans| {
        spans
            .borrow()
            .get(&key(word))
            .filter(|(text, _)| *text == filter_hash(word))
            .map(|(_, spans)| spans.clone())
    })
}

/// The single authored span of a registered `word` whose text spans it
/// exactly, while recording; `None` otherwise (normalized or multi-piece
/// words keep a synthetic span).
pub fn authored_word_span(word: &str) -> Option<TextSpan> {
    if !is_recording() {
        return None;
    }
    let spans = registered_spans(word)?;
    let first = spans.first()?;
    if spans.iter().any(|span| span.line != first.line) {
        return None;
    }
    let span = TextSpan {
        start: spans.iter().map(|span| span.start).min()?,
        end: spans.iter().map(|span| span.end).max()?,
        ..*first
    };
    // A possessive's apostrophe is dropped from its word ("players'").
    ((span.end - span.start).abs_diff(word.len()) <= 1).then_some(span)
}

/// Forget registered words. Called at the outermost scope.
pub fn reset_registry() {
    WORD_SPANS.with(|spans| spans.borrow_mut().clear());
    REGISTERED.with(|registered| registered.set(false));
}

/// Forget the registered words `token` owns, before its strings are freed or
/// replaced. A string later allocated at the same address with the same text
/// would otherwise inherit the dead word's span, so what a reader claims (and
/// which `&str` filter words leave obligations) would depend on heap history:
/// on which cards this thread compiled before. A tilde's `as_word` is its
/// own `parser_text` ("this"), so it is forgotten with the token.
pub(crate) fn forget_token_words(token: &OwnedLexToken) {
    // `try_with`: tokens can drop during thread-local teardown.
    if !REGISTERED.try_with(Cell::get).unwrap_or(false) {
        return;
    }
    let _ = WORD_SPANS.try_with(|spans| {
        let Ok(mut spans) = spans.try_borrow_mut() else {
            return;
        };
        let mut forget = |text: &str| {
            if !text.is_empty() {
                spans.remove(&key(text));
            }
        };
        for piece in token.parser_word_pieces() {
            forget(&piece.text);
        }
        forget(&token.slice);
        forget(&token.parser_text);
    });
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
    let spans = registered_spans(word);
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

/// A speculative reading's claims, obligations and clean filters, held
/// until the caller decides whether it keeps the reading.
#[must_use]
#[derive(Debug, Default)]
pub struct Speculative(ClaimSet);

impl Speculative {
    /// The caller kept the reading: the words it read count as read, and
    /// the words its filters left unread count as left unread.
    pub fn keep(self) {
        adopt(&self.0);
    }
}

/// Run a reading the caller may throw away (a probe whose result is used
/// only in some cases). Nothing it recorded reaches the enclosing scope
/// unless the caller calls [`Speculative::keep`]: an abandoned reading must
/// neither excuse a word for the reading the card keeps (by claiming it) nor
/// blame one (by leaving it unread).
pub fn speculative<T>(f: impl FnOnce() -> T) -> (T, Speculative) {
    if !is_recording() {
        return (f(), Speculative::default());
    }
    let (result, claims) = scoped(f);
    (result, Speculative(claims))
}

/// [`speculative`] for a reading kept exactly when `keep` accepts its
/// result (a recognizer whose failure means "not this shape").
pub fn speculative_keep_if<T>(f: impl FnOnce() -> T, keep: impl FnOnce(&T) -> bool) -> T {
    if !is_recording() {
        return f();
    }
    let (result, probe) = speculative(f);
    if keep(&result) {
        probe.keep();
    }
    result
}

fn authored_pieces(
    tokens: &[OwnedLexToken],
) -> impl Iterator<Item = &crate::lexer::TokenWordPiece> {
    tokens
        .iter()
        .filter(|token| token.kind == crate::lexer::TokenKind::Word && !is_synthetic(token))
        .flat_map(|token| token.parser_word_pieces())
}

/// The authored words in `tokens` that `claims` does not cover.
pub fn unclaimed_words<'a>(tokens: &'a [OwnedLexToken], claims: &ClaimSet) -> Vec<&'a str> {
    authored_pieces(tokens)
        .filter(|piece| !claims.contains(piece.span))
        .map(|piece| piece.text.as_str())
        .collect()
}

/// The authored word pieces of `tokens` (word tokens, not synthetic ones),
/// as source span and text: what the whole-line check expects some reader
/// to claim.
pub fn authored_word_pieces(tokens: &[OwnedLexToken]) -> Vec<(TextSpan, String)> {
    authored_pieces(tokens).map(|piece| (piece.span, piece.text.clone())).collect()
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
    fn an_unread_filter_word_is_a_violation_only_if_nothing_claims_it() {
        let tokens = crate::lexer::lex_line("if you do blorp creatures", 1).unwrap();
        let ((), card) = scoped(|| {
            let ((), mut filter) = scoped(|| claim_tokens(&tokens[4..]));
            filter.oblige_unclaimed(&tokens, String::new, None, Location::caller());
            adopt(&filter);
            claim_tokens(&tokens[..3]);
        });
        let words: Vec<_> = card.violations().iter().map(|o| o.word.as_str()).collect();
        assert_eq!(words, vec!["blorp"]);
    }

    #[test]
    fn a_discarded_speculative_reading_claims_nothing() {
        let ((), outer) = scoped(|| {
            let ((), abandoned) = speculative(|| claim(span(0, 4)));
            drop(abandoned);
            let ((), kept) = speculative(|| claim(span(5, 9)));
            kept.keep();
        });
        assert!(!outer.contains(span(0, 4)));
        assert!(outer.contains(span(5, 9)));
    }

    #[test]
    fn speculative_obligations_and_clean_filters_count_only_when_kept() {
        let tokens = crate::lexer::lex_line("blorp creatures", 1).unwrap();
        let probe = |keep: bool| {
            scoped(|| {
                let ((), probe) = speculative(|| {
                    let mut filter = ClaimSet::default();
                    filter.oblige_unclaimed(&tokens[..1], || "F".into(), Some(7), Location::caller());
                    filter.note_clean_filter("G");
                    adopt(&filter);
                });
                if keep {
                    probe.keep();
                }
            })
            .1
        };
        let dropped = probe(false);
        assert!(dropped.violations().is_empty());
        assert!(!dropped.has_clean_filter("G"));
        let kept = probe(true);
        assert_eq!(kept.violations().len(), 1);
        assert_eq!(kept.violations()[0].origins, vec![7]);
        assert!(kept.has_clean_filter("G"));
    }

    #[test]
    fn speculative_keep_if_keeps_by_result() {
        let ((), outer) = scoped(|| {
            let _ = speculative_keep_if(|| { claim(span(0, 4)); None::<()> }, Option::is_some);
            let _ = speculative_keep_if(|| { claim(span(5, 9)); Some(()) }, Option::is_some);
        });
        assert!(!outer.contains(span(0, 4)));
        assert!(outer.contains(span(5, 9)));
    }

    #[test]
    fn one_reading_reached_twice_is_one_obligation_with_both_origins() {
        let tokens = crate::lexer::lex_line("blorp creatures", 1).unwrap();
        let caller = Location::caller();
        let mut claims = ClaimSet::default();
        claims.oblige_unclaimed(&tokens[..1], || "F".into(), Some(1), caller);
        claims.oblige_unclaimed(&tokens[..1], || "F".into(), Some(2), caller);
        let violations = claims.violations();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].origins, vec![1, 2]);
    }

    #[test]
    fn line_enforcement_is_scoped_to_the_call() {
        assert!(!line_enforcement_requested());
        with_line_enforcement(|| {
            assert!(line_enforcement_requested());
            assert!(line_audit_enabled());
            assert!(audit_enabled());
        });
        assert!(!line_enforcement_requested());
    }

    #[test]
    fn authored_word_pieces_are_the_word_pieces_only() {
        let tokens = crate::lexer::lex_line("Draw a card.", 1).unwrap();
        let pieces = authored_word_pieces(&tokens);
        assert_eq!(pieces.len(), 3);
        assert!(pieces.iter().all(|(span, _)| span.end > span.start));
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
