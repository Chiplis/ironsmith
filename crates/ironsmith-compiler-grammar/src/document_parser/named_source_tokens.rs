//! The card's own name, normalized to a self-reference — on tokens.
//!
//! These are the token-level twins of the string normalizers beside them in
//! `document_parser`: the same alias matching over word pieces, the same
//! surface-preservation guards, the same trigger head/body split. What changes
//! is the medium. A recognizer that already holds a line's tokens no longer
//! renders them to text, edits the text, and lexes the result; the name's
//! tokens are replaced in place by the typed subject as word tokens carrying
//! the span of the name they stand in for. Nothing is tokenized here except
//! the card name itself, which is metadata and is tokenized once per alias.

use super::*;

/// The string normalizers returned lowercase text — a normalized line is
/// lowercase, and recognizers downstream rely on it. Their token twins return
/// lowercase tokens for the same reason.
fn lowercased(tokens: Vec<OwnedLexToken>) -> Vec<OwnedLexToken> {
    tokens
        .into_iter()
        .map(|token| OwnedLexToken::new(token.kind, token.slice.to_ascii_lowercase(), token.span))
        .collect()
}

/// One alias of the card's name, with the word pieces the alias replacer
/// matches on and the tokens the effect-verb guard reads.
pub(super) struct Alias {
    pub(super) text: String,
    pub(super) words: Vec<String>,
    pub(super) tokens: Vec<OwnedLexToken>,
}

pub(super) fn aliases_for_builder(card: &crate::card::CardBuilder) -> Vec<Alias> {
    let mut surfaces = Vec::new();
    for full_name in source_full_names_for_builder(card) {
        // Tokenizing the card name: metadata, not rules text, and the one
        // place this module turns a string into tokens. The name's short
        // alias ("Brago" for "Brago, King Eternal") is read from its tokens.
        let short_name = crate::util::lex_fragment(full_name.trim(), 0)
            .map(|tokens| {
                crate::grammar::preprocess::parse_short_self_reference_name_tokens(
                    full_name.as_str(),
                    &tokens,
                )
            })
            .unwrap_or_else(|| full_name.trim().to_string());
        push_source_name_alias_surfaces(&mut surfaces, full_name.as_str(), short_name.as_str());
    }
    surfaces.sort_by_key(|alias| std::cmp::Reverse(alias.len()));
    surfaces
        .into_iter()
        .filter_map(|text| {
            let tokens = crate::util::lex_fragment(text.trim(), 0)?;
            let words = TokenWordView::new(&tokens).owned_words();
            Some(Alias {
                text,
                words,
                tokens,
            })
        })
        .collect()
}

/// The alias that is the card's full name, if it lexed.
fn full_name_alias<'a>(card: &crate::card::CardBuilder, aliases: &'a [Alias]) -> Option<&'a Alias> {
    let name = card.name_ref().trim();
    if name.is_empty() {
        return None;
    }
    let mut full = None;
    for alias in aliases {
        if alias.text.eq_ignore_ascii_case(name) {
            full = Some(alias);
            break;
        }
    }
    full
}

/// Every alias's word pieces, for the longest-alias precedence check.
pub(super) fn alias_word_lists(aliases: &[Alias]) -> Vec<Vec<String>> {
    aliases.iter().map(|alias| alias.words.clone()).collect()
}

/// Tokens for the typed subject ("this creature"), each carrying `span`.
fn subject_tokens(subject: &str, span: TextSpan) -> Vec<OwnedLexToken> {
    generic_subject_words(subject)
        .iter()
        .map(|word| OwnedLexToken::word(*word, span))
        .collect()
}

fn span_over(tokens: &[OwnedLexToken]) -> TextSpan {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => TextSpan {
            line: first.span.line,
            start: first.span.start,
            end: last.span.end,
        },
        _ => TextSpan::synthetic(),
    }
}

/// Word pieces of `tokens` with the token each piece came from.
fn pieces_with_tokens(tokens: &[OwnedLexToken]) -> (Vec<SourceAliasWordPiece<'_>>, Vec<usize>) {
    let pieces = source_alias_word_pieces(tokens);
    let mut piece_tokens = Vec::with_capacity(pieces.len());
    for (token_index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Period {
            continue;
        }
        piece_tokens.extend(std::iter::repeat_n(
            token_index,
            token.parser_word_pieces().len(),
        ));
    }
    (pieces, piece_tokens)
}

/// If `tokens` open with the alias's words, the tokens after it.
fn strip_alias_prefix<'a>(
    tokens: &'a [OwnedLexToken],
    alias: &Alias,
) -> Option<&'a [OwnedLexToken]> {
    if alias.words.is_empty() {
        return None;
    }
    let (pieces, piece_tokens) = pieces_with_tokens(tokens);
    if pieces.len() < alias.words.len() {
        return None;
    }
    let matches = pieces[..alias.words.len()]
        .iter()
        .zip(alias.words.iter())
        .all(|(piece, word)| piece.text == word.as_str());
    if !matches {
        return None;
    }
    let last_token = piece_tokens[alias.words.len() - 1];
    // The alias must end on a token boundary.
    if piece_tokens.get(alias.words.len()) == Some(&last_token) {
        return None;
    }
    let tail = &tokens[last_token + 1..];
    (!tail.is_empty()).then_some(tail)
}

/// The string form's `source_alias_prefix_looks_like_effect_verb`: an alias
/// that is itself an effect verb ("Exile", "Return") followed by an ordinary
/// object rather than a predicate.
fn alias_prefix_looks_like_effect_verb(alias: &Alias, remainder: &[OwnedLexToken]) -> bool {
    document_grammar::source_alias_effect_verb_surface_tokens(&alias.tokens, remainder).is_some()
}

fn first_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    tokens
        .iter()
        .flat_map(|token| token.parser_word_pieces())
        .map(|piece| piece.text.as_str())
        .next()
}

fn mentions_named_reference_tokens(tokens: &[OwnedLexToken]) -> bool {
    tokens.iter().any(|token| token.is_word("named"))
}

/// The string form's `named_source_enters_tail_lexed`: what follows "enters",
/// unless a comma follows it directly.
fn enters_tail(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (enters, _, _) = grammar::find_prefix(tokens, || grammar::kw("enters"))?;
    let tail = &tokens[enters + 1..];
    if tail
        .first()
        .is_some_and(|token| token.kind == TokenKind::Comma)
    {
        return None;
    }
    (!tail.is_empty()).then_some(tail)
}

/// Replace every alias in `tokens` with `subject`, in the aliases' order.
/// `None` when nothing matched.
fn replace_all_aliases(
    tokens: &[OwnedLexToken],
    aliases: &[Alias],
    all_words: &[Vec<String>],
    subject: &str,
    preserve_surface_hints: impl Fn(&Alias) -> bool,
) -> Option<Vec<OwnedLexToken>> {
    let mut rewritten = tokens.to_vec();
    let mut changed = false;
    for alias in aliases {
        if let Some(next) = replace_named_source_alias_tokens(
            &rewritten,
            &alias.words,
            subject,
            all_words,
            preserve_surface_hints(alias),
        ) {
            rewritten = next;
            changed = true;
        }
    }
    changed.then_some(rewritten)
}

/// Token twin of `normalized_line_mentions_source_alias`.
pub(super) fn tokens_mention_source_alias(
    card: &crate::card::CardBuilder,
    tokens: &[OwnedLexToken],
) -> bool {
    let subject = named_source_subject_for_builder(card);
    let aliases = aliases_for_builder(card);
    let all_words = alias_word_lists(&aliases);
    aliases.iter().any(|alias| {
        replace_named_source_alias_tokens(
            tokens,
            &alias.words,
            subject,
            &all_words,
            document_grammar::parse_alias_face_separator(&alias.text).is_some(),
        )
        .is_some()
    })
}

/// Token twin of `normalize_named_source_sentence_for_builder`.
pub(super) fn normalize_named_source_sentence_tokens(
    card: &crate::card::CardBuilder,
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    normalize_named_source_sentence_tokens_cased(card, tokens).map(lowercased)
}

fn normalize_named_source_sentence_tokens_cased(
    card: &crate::card::CardBuilder,
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let subject = named_source_subject_for_builder(card);
    let aliases = aliases_for_builder(card);
    let all_words = alias_word_lists(&aliases);

    // A leading full name: "<Name> gets +1/+1" → "<subject> gets +1/+1".
    if let Some(full) = full_name_alias(card, &aliases)
        && let Some(remainder) = strip_alias_prefix(tokens, full)
    {
        if alias_prefix_looks_like_effect_verb(full, remainder) {
            return None;
        }
        // A characteristic-defining P/T line keeps its proper-name subject
        // so the static parser can attach its `SourceNameSubject` hint.
        if matches!(first_word(remainder), Some("power" | "toughness")) {
            return None;
        }
        let span = span_over(&tokens[..tokens.len() - remainder.len()]);
        let mut rewritten = subject_tokens(subject, span);
        rewritten.extend_from_slice(remainder);
        return Some(rewritten);
    }

    if !aliases.is_empty() && !mentions_named_reference_tokens(tokens) {
        let mut rewritten = replace_all_aliases(tokens, &aliases, &all_words, subject, |_| true)
            .unwrap_or_else(|| tokens.to_vec());
        // "As this creature enters, ... . <Name> enters with ..." — once the
        // leading source is normalized, normalize the follow-up sentence too,
        // or the two sentences stop reading as one as-enters program.
        if let Some(as_enters_subject) = as_enters_subject(&rewritten)
            && let Some((period, _, _)) =
                grammar::find_prefix(&rewritten, || grammar::token_kind(TokenKind::Period))
            && period + 1 < rewritten.len()
        {
            let tail = &rewritten[period + 1..];
            if let Some(normalized_tail) =
                replace_all_aliases(tail, &aliases, &all_words, &as_enters_subject, |alias| {
                    document_grammar::parse_alias_face_separator(&alias.text).is_some()
                })
            {
                rewritten.truncate(period + 1);
                rewritten.extend(normalized_tail);
            }
        }
        normalize_named_source_enter_agreement_tokens(&mut rewritten, subject);
        if rewritten != tokens {
            return Some(rewritten);
        }
    }

    // "<alias> enters with two counters" when a possessive elsewhere blocked
    // the broad rewrite: only a leading alias counts as a source-entry line.
    for alias in &aliases {
        let Some(tail) = strip_alias_prefix(tokens, alias) else {
            continue;
        };
        if alias_prefix_looks_like_effect_verb(alias, tail) {
            continue;
        }
        if first_word(tail) != Some("enters") {
            continue;
        }
        let Some(rest) = enters_tail(tail) else {
            continue;
        };
        let span = span_over(&tokens[..tokens.len() - tail.len()]);
        let mut rewritten = subject_tokens(subject, span);
        let enters_span = grammar::find_prefix(tail, || grammar::kw("enters"))
            .map_or(span, |(_, token, _)| token.span);
        rewritten.push(OwnedLexToken::word("enters", enters_span));
        rewritten.extend_from_slice(rest);
        return Some(rewritten);
    }

    None
}

/// "as this creature enters, ..." → the subject between "as" and "enters,",
/// when it is the typed self-reference.
fn as_enters_subject(tokens: &[OwnedLexToken]) -> Option<String> {
    if !tokens.first().is_some_and(|token| token.is_word("as")) {
        return None;
    }
    let (enters, _, _) = grammar::find_prefix(tokens, || grammar::kw("enters"))?;
    if !tokens
        .get(enters + 1)
        .is_some_and(|token| token.kind == TokenKind::Comma)
    {
        return None;
    }
    let subject = &tokens[1..enters];
    if !subject.first().is_some_and(|token| token.is_word("this")) {
        return None;
    }
    Some(
        subject
            .iter()
            .flat_map(|token| token.parser_word_pieces())
            .map(|piece| piece.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Token twin of `normalize_comma_bearing_leading_source_trigger`: a legendary
/// name with a comma in it, right after "when"/"whenever", becomes the subject
/// before the trigger is split on its first comma.
fn normalize_comma_bearing_leading_trigger(
    card: &crate::card::CardBuilder,
    aliases: &[Alias],
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let intro = tokens.first()?;
    if !(intro.is_word("when") || intro.is_word("whenever")) {
        return None;
    }
    let full = full_name_alias(card, aliases)?;
    let after_intro = &tokens[1..];
    let (pieces, piece_tokens) = pieces_with_tokens(after_intro);
    if pieces.len() < full.words.len() || full.words.is_empty() {
        return None;
    }
    let matches = pieces[..full.words.len()]
        .iter()
        .zip(full.words.iter())
        .all(|(piece, word)| piece.text == word.as_str());
    if !matches || pieces[full.words.len() - 1].possessive {
        return None;
    }
    let last_token = piece_tokens[full.words.len() - 1];
    if piece_tokens.get(full.words.len()) == Some(&last_token) {
        return None;
    }
    let name_tokens = &after_intro[..=last_token];
    let mut rewritten = vec![intro.clone()];
    rewritten.extend(subject_tokens(
        named_source_subject_for_builder(card),
        span_over(name_tokens),
    ));
    rewritten.extend_from_slice(&after_intro[last_token + 1..]);
    Some(rewritten)
}

/// Token twin of `normalize_named_source_trigger_head_for_builder`.
fn normalize_trigger_head(
    card: &crate::card::CardBuilder,
    aliases: &[Alias],
    all_words: &[Vec<String>],
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let subject = named_source_subject_for_builder(card);
    if let Some(full) = full_name_alias(card, aliases)
        && let Some(remainder) = strip_alias_prefix(tokens, full)
    {
        let span = span_over(&tokens[..tokens.len() - remainder.len()]);
        let mut rewritten = subject_tokens(subject, span);
        rewritten.extend_from_slice(remainder);
        return Some(rewritten);
    }
    let mut rewritten = replace_all_aliases(tokens, aliases, all_words, subject, |_| false)?;
    normalize_named_source_enter_agreement_tokens(&mut rewritten, subject);
    Some(rewritten)
}

/// Token twin of `normalize_named_source_trigger_for_builder`.
pub(super) fn normalize_named_source_trigger_tokens(
    card: &crate::card::CardBuilder,
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    normalize_named_source_trigger_tokens_cased(card, tokens).map(lowercased)
}

fn normalize_named_source_trigger_tokens_cased(
    card: &crate::card::CardBuilder,
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let aliases = aliases_for_builder(card);
    let all_words = alias_word_lists(&aliases);
    let subject = named_source_subject_for_builder(card);

    let (tokens, mut changed) =
        match normalize_comma_bearing_leading_trigger(card, &aliases, tokens) {
            Some(rewritten) => (rewritten, true),
            None => (tokens.to_vec(), false),
        };

    if let Some((comma, _, _)) = grammar::find_prefix(&tokens, grammar::comma) {
        let head = &tokens[..comma];
        let body = &tokens[comma + 1..];
        if head.is_empty() || body.is_empty() {
            return None;
        }
        let head = match normalize_trigger_head(card, &aliases, &all_words, head) {
            Some(rewritten) => {
                changed = true;
                rewritten
            }
            None => head.to_vec(),
        };
        let body = match replace_all_aliases(body, &aliases, &all_words, subject, |_| false) {
            Some(rewritten) => {
                changed = true;
                rewritten
            }
            None => body.to_vec(),
        };
        if !changed {
            return None;
        }
        let mut rewritten = head;
        rewritten.push(tokens[comma].clone());
        rewritten.extend(body);
        return Some(rewritten);
    }

    match normalize_trigger_head(card, &aliases, &all_words, &tokens) {
        Some(rewritten) => Some(rewritten),
        None => changed.then_some(tokens),
    }
}

/// Whether a trigger line names its source: a comma-bearing legendary name
/// right after the intro, or any alias the trigger normalizer would rewrite.
pub(super) fn trigger_names_source(
    card: &crate::card::CardBuilder,
    tokens: &[OwnedLexToken],
) -> bool {
    let aliases = aliases_for_builder(card);
    normalize_comma_bearing_leading_trigger(card, &aliases, tokens).is_some()
        || normalize_named_source_trigger_tokens_cased(card, tokens).is_some()
}

/// Tokens with parenthetical reminder text removed.
pub(super) fn strip_parenthetical_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    crate::util::strip_parenthetical_tokens(tokens)
}

/// The rewritten trigger plus, when it says "this permanent", one variant per
/// typed subject — the string form's candidate list.
pub(super) fn this_permanent_candidates(tokens: Vec<OwnedLexToken>) -> Vec<Vec<OwnedLexToken>> {
    let mut candidates = vec![tokens];
    if document_grammar::parse_this_permanent_surface(&candidates[0]).is_none() {
        return candidates;
    }
    for subject in [
        "this creature",
        "this artifact",
        "this enchantment",
        "this land",
        "this planeswalker",
        "this battle",
    ] {
        let base = &candidates[0];
        let mut candidate = Vec::with_capacity(base.len());
        let mut index = 0;
        let mut replaced = false;
        while index < base.len() {
            if base[index].is_word("this")
                && base
                    .get(index + 1)
                    .is_some_and(|next| next.is_word("permanent"))
            {
                candidate.extend(subject_tokens(subject, span_over(&base[index..=index + 1])));
                index += 2;
                replaced = true;
            } else {
                candidate.push(base[index].clone());
                index += 1;
            }
        }
        if replaced {
            candidates.push(candidate);
        }
    }
    candidates
}

/// The authored name right after "when"/"whenever" in `authored`, as tokens —
/// the token twin of `leading_named_source_trigger_subject_for_builder`.
pub(super) fn leading_authored_trigger_subject(
    card: &crate::card::CardBuilder,
    authored: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let intro = authored.first()?;
    if !(intro.is_word("when") || intro.is_word("whenever")) {
        return None;
    }
    let after_intro = &authored[1..];
    let (pieces, piece_tokens) = pieces_with_tokens(after_intro);
    for alias in aliases_for_builder(card) {
        if alias.words.is_empty() || pieces.len() < alias.words.len() {
            continue;
        }
        let matches = pieces[..alias.words.len()]
            .iter()
            .zip(alias.words.iter())
            .all(|(piece, word)| piece.text == word.as_str());
        if !matches || pieces[alias.words.len() - 1].possessive {
            continue;
        }
        let last_token = piece_tokens[alias.words.len() - 1];
        if piece_tokens.get(alias.words.len()) == Some(&last_token) {
            continue;
        }
        return Some(after_intro[..=last_token].to_vec());
    }
    None
}
