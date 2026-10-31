//! Object-filter claim auditing over the syntax crate's word-claim recorder.
//!
//! The recorder (scopes, claims, the word registry) lives in
//! `ironsmith_compiler_syntax::word_claims` beside the word primitives that
//! claim. This module adds the filter audit: each outermost object-filter
//! parse leaves an obligation for every authored word it did not read, and
//! the card parse reports (or, when enforcing, fails on) the obligations no
//! reader anywhere in the card claimed
//! (architecture/filter-word-consumption-plan.md, "The check is per card").
//!
//! An obligation is tied to the filter its parse returned by a
//! [`ClaimOrigin`] stamped on that filter, so the card check can tell the
//! filter survived into the document even when lowering changed its text.
//!
//! The whole-line check (`IRONSMITH_LINE_CLAIM_AUDIT`,
//! `IRONSMITH_LINE_CLAIM_ENFORCE`, [`with_line_enforcement`]) goes further:
//! every authored word of the card's parse stream must be claimed by some
//! reader of the committed parse, or consumed by preprocessing
//! ([`claim_for_line`]).

use std::cell::{Cell, RefCell};
use std::io::Write as _;
use std::panic::Location;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::ObjectFilter;
use crate::lexer::OwnedLexToken;
pub use ironsmith_compiler_syntax::word_claims::*;
use ironsmith_core::filter_model::ClaimOrigin;

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
    // The parse-stream lines of the card being parsed (whole-line check).
    static CARD_LINES: RefCell<Option<Vec<CardLine>>> = const { RefCell::new(None) };
    // Words preprocessing consumed for the card being parsed; read for the
    // whole-line check only.
    static LINE_CONSUMED: RefCell<ClaimSet> = RefCell::new(ClaimSet::default());
}

/// One rules line of the card being parsed: its authored text and the
/// authored word pieces of its parse stream.
struct CardLine {
    text: String,
    pieces: Vec<(crate::TextSpan, String)>,
    /// Each piece's span in the authored line, when normalization rewrote the
    /// line text: a reader of the authored tokens claims those positions.
    authored_spans: Vec<Option<crate::TextSpan>>,
}

/// A fresh [`ClaimOrigin`] id, unique in the process (a memoized result can
/// carry an id from an earlier parse, so ids are never reused).
fn next_origin() -> u32 {
    static NEXT: AtomicU32 = AtomicU32::new(1);
    loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        if id != 0 {
            return id;
        }
    }
}

/// Record the parse stream of each rules line of the card being parsed, so
/// the card check can ask whether every authored word was read. Whole-line
/// audit only; the first call per card wins, so a nested document parse
/// does not replace the card's lines.
pub fn note_card_lines<'a>(lines: impl IntoIterator<Item = (&'a str, &'a [OwnedLexToken])>) {
    if !line_audit_enabled() || CARD_DEPTH.with(Cell::get) == 0 {
        return;
    }
    CARD_LINES.with(|card_lines| {
        let mut card_lines = card_lines.borrow_mut();
        if card_lines.is_some() {
            return;
        }
        *card_lines = Some(
            lines
                .into_iter()
                .map(|(text, tokens)| CardLine {
                    text: text.to_string(),
                    pieces: authored_word_pieces(tokens),
                    authored_spans: Vec::new(),
                })
                .collect(),
        );
    });
}

/// The normalization of each rules line noted by [`note_card_lines`], in the
/// same order. A line whose normalized text differs from its authored text
/// lexes at shifted offsets; readers that read the authored tokens claim the
/// authored positions, which the whole-line check then maps back. Whole-line
/// audit only; ignored once set for the card.
pub fn note_card_line_normalizations<'a>(
    lines: impl IntoIterator<Item = &'a crate::cards::builders::NormalizedLine>,
) {
    if !line_audit_enabled() || CARD_DEPTH.with(Cell::get) == 0 {
        return;
    }
    CARD_LINES.with(|card_lines| {
        let mut card_lines = card_lines.borrow_mut();
        let Some(card_lines) = card_lines.as_mut() else {
            return;
        };
        for (card_line, normalized) in card_lines.iter_mut().zip(lines) {
            let authored = card_line.text.trim();
            if !card_line.authored_spans.is_empty()
                || (normalized.normalized == normalized.original && normalized.original == authored)
            {
                continue;
            }
            // `original` is the stripped text normalization started from
            // (reminder text, whitespace runs and a definition header removed);
            // the strip map carries its offsets on to the authored line the
            // authored tokens lex. Without one, the stripped offsets stand.
            let strip_map =
                crate::preprocess::stripped_line_authored_char_map(&card_line.text, &normalized.original);
            card_line.authored_spans = card_line
                .pieces
                .iter()
                .map(|(span, _)| {
                    let stripped = crate::util::map_span_to_original(
                        *span,
                        &normalized.normalized,
                        &normalized.original,
                        &normalized.char_map,
                    );
                    match &strip_map {
                        Some(strip_map) => {
                            stripped_span_to_authored(stripped, &normalized.original, authored, strip_map)
                        }
                        None => Some(stripped),
                    }
                })
                .collect();
        }
    });
}

/// A byte span of the stripped line `stripped`, carried through the per-char
/// `strip_map` onto the authored line `authored`.
fn stripped_span_to_authored(
    span: crate::TextSpan,
    stripped: &str,
    authored: &str,
    strip_map: &[usize],
) -> Option<crate::TextSpan> {
    let start_char = stripped.get(..span.start)?.chars().count();
    let end_char = stripped.get(..span.end)?.chars().count();
    if end_char <= start_char {
        return None;
    }
    let first = *strip_map.get(start_char)?;
    let last = *strip_map.get(end_char - 1)?;
    let byte_at = |index: usize| {
        authored
            .char_indices()
            .nth(index)
            .map_or(authored.len(), |(byte, _)| byte)
    };
    Some(crate::TextSpan {
        line: span.line,
        start: byte_at(first),
        end: byte_at(last + 1),
    })
}

/// Words preprocessing read in place of a reader (a card name normalized to
/// a source reference). They count as read for the whole-line check only,
/// never for an object filter's unread words: preprocessing did not read
/// them as part of any filter.
pub fn claim_for_line<'a>(tokens: impl IntoIterator<Item = &'a OwnedLexToken>) {
    if !line_audit_enabled() || !is_recording() || CARD_DEPTH.with(Cell::get) == 0 {
        return;
    }
    let consumed = ClaimSet::of_tokens(tokens);
    LINE_CONSUMED.with(|set| set.borrow_mut().extend(&consumed));
}

/// Whether `IRONSMITH_REGISTRY_TRACE` asks for per-registry resolution
/// traces on stderr.
pub fn registry_trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("IRONSMITH_REGISTRY_TRACE").is_some())
}

/// The words `claims` covers, as `text@line:start` for the card being
/// parsed (or bare spans outside a card's whole-line audit). Traces only.
pub fn describe_claims(claims: &ClaimSet) -> String {
    let spans = claims.claimed_spans();
    CARD_LINES.with(|lines| {
        let lines = lines.borrow();
        spans
            .iter()
            .map(|&(line, start, end)| {
                let text = lines.as_ref().and_then(|lines| {
                    lines.iter().find_map(|card_line| {
                        card_line.pieces.iter().enumerate().find_map(|(index, (span, text))| {
                            let at = |span: &crate::TextSpan| {
                                span.line == line && span.start == start && span.end == end
                            };
                            // A claim at the authored position of a rewritten
                            // line's piece names that piece too.
                            (at(span)
                                || card_line
                                    .authored_spans
                                    .get(index)
                                    .copied()
                                    .flatten()
                                    .is_some_and(|authored| at(&authored)))
                            .then(|| text.clone())
                        })
                    })
                });
                match text {
                    Some(text) => format!("{text}@{line}:{start}"),
                    None => format!("?@{line}:{start}-{end}"),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// Run a yes/no probe whose reading is never the one the card keeps (its
/// answer only steers which reader runs next, and that reader reads the
/// words itself): nothing it records counts.
pub fn probe_only<T>(f: impl FnOnce() -> T) -> T {
    speculative(f).0
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
/// fails: for the process (`IRONSMITH_FILTER_CLAIM_ENFORCE`) or inside
/// [`with_enforcement`] (the card builder). Off by default otherwise.
fn enforce_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("IRONSMITH_FILTER_CLAIM_ENFORCE").is_some())
        || enforcement_requested()
}

/// Where the whole-line report goes: `IRONSMITH_LINE_CLAIM_AUDIT=<path>`.
fn line_audit_path() -> Option<&'static std::path::Path> {
    static PATH: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| std::env::var_os("IRONSMITH_LINE_CLAIM_AUDIT").map(Into::into))
        .as_deref()
}

/// Whether a card with an authored word no reader reads fails: for the
/// process (`IRONSMITH_LINE_CLAIM_ENFORCE`) or inside
/// [`with_line_enforcement`].
fn line_enforce_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("IRONSMITH_LINE_CLAIM_ENFORCE").is_some())
        || line_enforcement_requested()
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
/// claimed; when enforcing, those words fail the card. The whole-line check
/// does the same for every authored word of the card's lines. A card parse
/// nested in another (a quoted ability) joins the enclosing scope.
pub fn audit_card<T: std::fmt::Debug>(
    name: &str,
    f: impl FnOnce() -> Result<T, crate::CardTextError>,
) -> Result<T, crate::CardTextError> {
    if !audit_enabled() || CARD_DEPTH.with(Cell::get) > 0 {
        return f();
    }
    CARD.with(|card| card.replace(name.to_string()));
    REPORTED.with(|reported| reported.borrow_mut().clear());
    CARD_LINES.with(|lines| lines.borrow_mut().take());
    LINE_CONSUMED.with(|set| set.take());
    reset_registry();
    let guard = depth_guard(&CARD_DEPTH);
    let (result, claims) = scoped(f);
    drop(guard);
    reset_registry();
    let lines = CARD_LINES.with(|lines| lines.borrow_mut().take());
    let consumed = LINE_CONSUMED.with(|set| set.take());
    result.and_then(|value| {
        check_filter_obligations(name, &claims, &value)?;
        if let Some(lines) = lines {
            check_line_coverage(name, &claims, &consumed, &lines)?;
        }
        Ok(value)
    })
}

/// The card check over object-filter obligations.
fn check_filter_obligations<T: std::fmt::Debug>(
    name: &str,
    claims: &ClaimSet,
    value: &T,
) -> Result<(), crate::CardTextError> {
    // Only a filter the card kept counts: a probe of the wrong span
    // produces a filter that is not in the parsed document.
    let mut violations = claims.violations();
    let debug = std::env::var_os("IRONSMITH_FILTER_CLAIM_DEBUG").is_some();
    if debug {
        for (obligation, claimed) in claims.obligations_debug() {
            eprintln!(
                "claim-debug obligation '{}' in '{}' at {} claimed={claimed}",
                obligation.word, obligation.input, obligation.caller
            );
        }
    }
    if violations.is_empty() {
        return Ok(());
    }
    let document = comparable_debug(&format!("{value:?}"));
    // The document with each stamped filter's origin shown, to find the
    // exact filters the obligations' parses returned.
    let traced_document = if violations.iter().any(|obligation| !obligation.origins.is_empty()) {
        ironsmith_core::filter_model::with_claim_origins_in_debug(|| format!("{value:?}"))
    } else {
        String::new()
    };
    let traced = |obligation: &Obligation| {
        obligation
            .origins
            .iter()
            .any(|id| traced_document.contains(&format!("{:?}", ClaimOrigin::new(*id))))
    };
    if debug {
        for obligation in &violations {
            eprintln!(
                "claim-debug {name}: '{}' in '{}' at {} kept={} traced={} clean={}",
                obligation.word,
                obligation.input,
                obligation.caller,
                document.contains(&*obligation.filter),
                traced(*obligation),
                claims.has_clean_filter(&obligation.filter),
            );
        }
    }
    // A filter the parse returned that is still in the document was kept,
    // whatever lowering did to its text. Failing that, a filter whose text
    // is in the document counts as kept unless some other parse produced
    // the same text while reading all its words (a probe of a whole clause
    // that came back as the clause's real "target creature").
    violations.retain(|obligation| {
        traced(*obligation)
            || (document.contains(&*obligation.filter)
                && !claims.has_clean_filter(&obligation.filter))
    });
    if violations.is_empty() {
        return Ok(());
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
    Ok(())
}

/// The whole-line check: every authored word of the card's lines must be
/// claimed by the committed parse or consumed by preprocessing.
fn check_line_coverage(
    name: &str,
    claims: &ClaimSet,
    consumed: &ClaimSet,
    lines: &[CardLine],
) -> Result<(), crate::CardTextError> {
    let rows: Vec<(&str, Vec<&str>)> = lines
        .iter()
        .filter_map(|line| {
            let unread: Vec<&str> = line
                .pieces
                .iter()
                .enumerate()
                .filter(|(index, (span, _))| {
                    let read = |span: crate::TextSpan| claims.contains(span) || consumed.contains(span);
                    !read(*span)
                        && !line
                            .authored_spans
                            .get(*index)
                            .copied()
                            .flatten()
                            .is_some_and(|authored| authored.end > authored.start && read(authored))
                })
                .map(|(_, (_, word))| word.as_str())
                .collect();
            (!unread.is_empty()).then_some((line.text.as_str(), unread))
        })
        .collect();
    if rows.is_empty() {
        return Ok(());
    }
    for (text, words) in &rows {
        write_line_report(name, &words.join(" "), text);
    }
    if line_enforce_enabled() {
        let (text, words) = &rows[0];
        return Err(crate::CardTextError::ParseError(format!(
            "no reader reads '{}' (line: '{}')",
            words.join(" "),
            text,
        )));
    }
    Ok(())
}

/// Run one object-filter parse of `tokens`. In audit mode its claims reach
/// the enclosing scope, and the outermost filter parse adds an obligation
/// for each authored word of its input it did not read. A nested
/// sub-phrase's claims are adopted by its parent, so only the outermost
/// parse decides what was left unread. Unread words never fail the filter
/// itself: callers probe filters speculatively and branch on the result.
pub fn audit_filter(
    tokens: &[OwnedLexToken],
    caller: &'static Location<'static>,
    f: impl FnOnce() -> Result<ObjectFilter, crate::CardTextError>,
) -> Result<ObjectFilter, crate::CardTextError> {
    if !audit_enabled() {
        return f();
    }
    audit_filter_input(FilterInput::Tokens(tokens), caller, f)
}

/// [`audit_filter`] for a filter given registered `&str` words. Only an
/// outermost word parse inside a card leaves obligations: outside a card the
/// words' origin is unknown.
pub fn audit_filter_words(
    words: &[&str],
    caller: &'static Location<'static>,
    f: impl FnOnce() -> Result<ObjectFilter, crate::CardTextError>,
) -> Result<ObjectFilter, crate::CardTextError> {
    if !audit_enabled() {
        return f();
    }
    audit_filter_input(FilterInput::Words(words), caller, f)
}

enum FilterInput<'a> {
    Tokens(&'a [OwnedLexToken]),
    Words(&'a [&'a str]),
}

fn audit_filter_input(
    input: FilterInput<'_>,
    caller: &'static Location<'static>,
    f: impl FnOnce() -> Result<ObjectFilter, crate::CardTextError>,
) -> Result<ObjectFilter, crate::CardTextError> {
    let in_card = CARD_DEPTH.with(Cell::get) > 0;
    let outermost = FILTER_DEPTH.with(|depth| depth.get() == 0);
    if outermost && !in_card && matches!(input, FilterInput::Tokens(_)) {
        reset_registry();
    }
    let guard = depth_guard(&FILTER_DEPTH);
    let (mut result, mut claims) = scoped(f);
    drop(guard);
    if let Ok(filter) = result.as_mut() {
        if outermost {
            CALLER_OWNED.with(|owned| owned.borrow().iter().for_each(|set| claims.extend(set)));
            if in_card {
                let unread = match input {
                    FilterInput::Tokens(tokens) => claims.leaves_unread(tokens),
                    FilterInput::Words(words) => claims.leaves_words_unread(words),
                };
                if std::env::var_os("IRONSMITH_FILTER_CLAIM_DEBUG").is_some() {
                    let text = match input {
                        FilterInput::Tokens(tokens) => crate::lexer::token_word_refs(tokens).join(" "),
                        FilterInput::Words(words) => words.join(" "),
                    };
                    eprintln!("claim-debug filter '{text}' at {caller} unread={unread}");
                }
                // Words of unknown origin cannot vouch for a clean reading.
                let traceable = match input {
                    FilterInput::Tokens(_) => true,
                    FilterInput::Words(words) => {
                        words.iter().all(|word| !word_spans(word).is_empty())
                    }
                };
                // The origin never shows in this text (see `ClaimOrigin`).
                let text = comparable_debug(&format!("{filter:?}"));
                if !unread {
                    if traceable {
                        claims.note_clean_filter(&text);
                    }
                } else {
                    // Stamp the filter so the card check can find it in the
                    // document by identity, not only by its text.
                    let origin = next_origin();
                    filter.claim_origin = ClaimOrigin::new(origin);
                    match input {
                        FilterInput::Tokens(tokens) => {
                            claims.oblige_unclaimed(tokens, || text, Some(origin), caller)
                        }
                        FilterInput::Words(words) => {
                            claims.oblige_unclaimed_words(words, || text, Some(origin), caller)
                        }
                    }
                }
            } else if let FilterInput::Tokens(tokens) = input {
                report(tokens, &claims, caller);
            }
        }
        adopt(&claims);
    }
    result
}

/// A filter reading of `input` that read only part of it (a recovery that
/// kept the best-parsing suffix): the `skipped` words count as left unread
/// by `filter`, as an outermost filter parse would leave them. The filter is
/// stamped (if its own parse did not stamp it) so the card check can trace
/// it.
pub fn oblige_skipped(
    input: &[OwnedLexToken],
    skipped: &[OwnedLexToken],
    filter: &mut ObjectFilter,
    caller: &'static Location<'static>,
) {
    if skipped.is_empty() || !is_recording() || CARD_DEPTH.with(Cell::get) == 0 {
        return;
    }
    let origin = filter.claim_origin.id().unwrap_or_else(|| {
        let origin = next_origin();
        filter.claim_origin = ClaimOrigin::new(origin);
        origin
    });
    let text = comparable_debug(&format!("{filter:?}"));
    let mut claims = ClaimSet::default();
    claims.oblige_unclaimed(skipped, || text, Some(origin), caller);
    claims.set_obligation_inputs(crate::lexer::token_word_refs(input).join(" "));
    adopt(&claims);
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

fn write_line_report(card: &str, words: &str, line: &str) {
    let Some(path) = line_audit_path() else {
        return;
    };
    let line = format!("{card}\t{words}\t{line}\n");
    if !REPORTED.with(|reported| reported.borrow_mut().insert(format!("line\t{line}"))) {
        return;
    }
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
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


#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<OwnedLexToken> {
        crate::lexer::lex_line(text, 0).unwrap()
    }

    /// A card parse whose one filter reading left `tokens` unread and
    /// returned a filter whose text no longer matches the obligation.
    fn changed_filter_card(tokens: &[OwnedLexToken], stamp: bool) -> Result<ObjectFilter, crate::CardTextError> {
        audit_card("Probe", || {
            let origin = next_origin();
            let mut claims = ClaimSet::default();
            claims.oblige_unclaimed(tokens, || "lowered away".into(), Some(origin), Location::caller());
            adopt(&claims);
            let mut filter = ObjectFilter::default();
            if stamp {
                filter.claim_origin = ClaimOrigin::new(origin);
            }
            Ok(filter)
        })
    }

    #[test]
    fn a_kept_filter_is_traced_by_origin_after_lowering_changes_its_text() {
        let tokens = tokens("blorp creatures");
        let traced = with_enforcement(|| changed_filter_card(&tokens, true));
        assert!(format!("{:?}", traced.unwrap_err()).contains("blorp"));
        // Neither the origin nor the text is in the document: not kept.
        assert!(with_enforcement(|| changed_filter_card(&tokens, false)).is_ok());
    }

    #[test]
    fn a_clean_outermost_filter_parse_is_not_stamped() {
        let tokens = tokens("creatures you control");
        let filter = with_enforcement(|| {
            audit_card("Probe", || crate::object_filters::parse_object_filter(&tokens, false))
        })
        .unwrap();
        assert!(filter.claim_origin.is_none());
    }

    #[test]
    fn a_discarded_probe_cannot_excuse_a_kept_filter_word() {
        let tokens = tokens("blorp creatures");
        let result = with_enforcement(|| {
            audit_card("Probe", || {
                // An abandoned reading read "blorp"...
                let ((), probe) = speculative(|| claim_tokens(&tokens[..1]));
                drop(probe);
                // ...but the kept filter left it unread.
                let origin = next_origin();
                let mut claims = ClaimSet::default();
                claims.oblige_unclaimed(&tokens, || "x".into(), Some(origin), Location::caller());
                adopt(&claims);
                claim_tokens(&tokens[1..]);
                let mut filter = ObjectFilter::default();
                filter.claim_origin = ClaimOrigin::new(origin);
                Ok(filter)
            })
        });
        assert!(format!("{:?}", result.unwrap_err()).contains("blorp"));
    }

    fn line_card(tokens: &[OwnedLexToken], consumed: bool) -> Result<(), crate::CardTextError> {
        audit_card("Probe", || {
            note_card_lines([("Blorp creatures", tokens)]);
            if consumed {
                claim_for_line(&tokens[..1]);
            }
            claim_tokens(&tokens[1..]);
            Ok(())
        })
    }

    #[test]
    fn an_unread_line_word_fails_the_card_under_line_enforcement() {
        let tokens = tokens("blorp creatures");
        let error = with_line_enforcement(|| line_card(&tokens, false)).unwrap_err();
        assert!(format!("{error:?}").contains("'blorp'"));
        // Words preprocessing consumed count as read.
        assert!(with_line_enforcement(|| line_card(&tokens, true)).is_ok());
    }

    #[test]
    fn line_words_consumed_by_preprocessing_never_excuse_a_filter_word() {
        let tokens = tokens("blorp creatures");
        let result = with_enforcement(|| {
            with_line_enforcement(|| {
                audit_card("Probe", || {
                    note_card_lines([("Blorp creatures", tokens.as_slice())]);
                    claim_for_line(&tokens[..1]);
                    claim_tokens(&tokens[1..]);
                    let origin = next_origin();
                    let mut claims = ClaimSet::default();
                    claims.oblige_unclaimed(&tokens, || "x".into(), Some(origin), Location::caller());
                    adopt(&claims);
                    let mut filter = ObjectFilter::default();
                    filter.claim_origin = ClaimOrigin::new(origin);
                    Ok(filter)
                })
            })
        });
        assert!(format!("{:?}", result.unwrap_err()).contains("object filter does not read 'blorp'"));
    }
}
