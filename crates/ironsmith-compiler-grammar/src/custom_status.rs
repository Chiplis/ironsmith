//! Explicit named-status syntax used by custom mechanic expansion. Names are data.
use crate::cards::builders::{
    EffectAst, KeywordActionAst, PlayerAst, PredicateAst, SourcePredicateAst, SubjectVerbActionAst,
    SubjectVerbRoleAst,
};
use crate::lexer::{OwnedLexToken, TokenKind, render_token_slice, token_word_refs};
use crate::target::ObjectFilter;

fn words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    token_word_refs(tokens)
}
fn name(tokens: &[OwnedLexToken]) -> Option<String> {
    if tokens.is_empty() || tokens.iter().any(|token| token.kind != TokenKind::Word) {
        return None;
    }
    let name = render_token_slice(tokens).to_ascii_lowercase();
    (!name.is_empty()).then_some(name)
}
fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let end = tokens
        .iter()
        .rposition(|t| !matches!(t.kind, TokenKind::Period | TokenKind::Comma))
        .map_or(0, |i| i + 1);
    &tokens[..end]
}

/// Peel precisely `with[out] the <name> status`, retaining all other filter words.
pub(crate) fn filter_clause(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, String, bool)> {
    for start in 0..tokens.len() {
        let prefix = words(&tokens[start..tokens.len().min(start + 2)]);
        if !matches!(prefix.as_slice(), ["with" | "without", "the"]) {
            continue;
        }
        let end = (start + 2..tokens.len()).find(|i| words(&tokens[*i..*i + 1]) == ["status"])?;
        let name = name(&tokens[start + 2..end])?;
        let mut base = tokens[..start].to_vec();
        base.extend_from_slice(&tokens[end + 1..]);
        crate::word_claims::claim_tokens(&tokens[start..end + 1]);
        return Some((base, name, prefix[0] == "without"));
    }
    None
}

pub(crate) fn source_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let tokens = trimmed(tokens);
    let end = tokens.len().checked_sub(1)?;
    if words(&tokens[end..]) != ["status"] {
        return None;
    }
    let at = (0..end).find(|i| words(&tokens[*i..*i + 1]) == ["the"])?;
    if tokens[..at]
        .iter()
        .any(|token| token.kind != TokenKind::Word)
    {
        return None;
    }
    let prefix = words(&tokens[..at]);
    let prefix = prefix.strip_prefix(&["if"]).unwrap_or(&prefix);
    let (subject, negative) = if let Some(subject) = prefix.strip_suffix(&["has"]) {
        (subject, false)
    } else if let Some(subject) = prefix.strip_suffix(&["does", "not", "have"]) {
        (subject, true)
    } else if let Some(subject) = prefix.strip_suffix(&["doesn't", "have"]).or_else(|| prefix.strip_suffix(&["doesnt", "have"])) {
        (subject, true)
    } else {
        return None;
    };
    if !matches!(
        subject,
        ["this"] | ["it"]
            | [
                "this",
                "creature"
                    | "permanent"
                    | "artifact"
                    | "enchantment"
                    | "land"
                    | "planeswalker"
                    | "battle"
                    | "card"
            ]
    ) {
        return None;
    }
    let name = name(&tokens[at + 1..end])?;
    let mut filter = ObjectFilter::default();
    filter.custom_statuses.push(name);
    crate::word_claims::claim_tokens(tokens);
    let condition = if subject == ["it"] {
        PredicateAst::ItMatches(filter)
    } else {
        PredicateAst::Source(SourcePredicateAst::SourceMatches(filter))
    };
    Some(if negative {
        PredicateAst::Not(Box::new(condition))
    } else {
        condition
    })
}

pub(crate) fn source_effect(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let tokens = trimmed(tokens);
    // Do not treat an enclosing activation cost as part of this instruction.
    if !tokens.first().is_some_and(|token| token.is_word("give")) {
        return None;
    }
    let end = tokens
        .iter()
        .position(|t| words(std::slice::from_ref(t)) == ["status"])?;
    let at = tokens
        .iter()
        .position(|t| words(std::slice::from_ref(t)) == ["the"])?;
    // The ordinary target reader owns target/non-target/reference distinctions.
    // Never infer Source from an arbitrary recipient phrase.
    let target = crate::util::parse_target_phrase(&tokens[1..at]).ok()?;
    if tokens[..at]
        .iter()
        .chain(tokens[end + 1..].iter())
        .any(|token| token.kind != TokenKind::Word)
    {
        return None;
    }
    let tail = words(&tokens[end + 1..]);
    let until_end_of_turn = match tail.as_slice() {
        [] => false,
        ["until", "end", "of", "turn"] => true,
        _ => return None,
    };
    let name = name(&tokens[at + 1..end])?;
    crate::word_claims::claim_tokens(tokens);
    Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::You,
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::SetPermanentDesignation {
            target,
            designation: ironsmith_core::PermanentDesignation::CustomStatus {
                name,
                until_end_of_turn,
            },
        }),
    ))
}

pub(crate) fn gained_trigger(tokens: &[OwnedLexToken]) -> Option<(Vec<OwnedLexToken>, String)> {
    let tokens = trimmed(tokens);
    let end = tokens.len().checked_sub(1)?;
    if words(&tokens[end..]) != ["status"] {
        return None;
    }
    let at = (0..end).find(|i| {
        matches!(
            words(&tokens[*i..tokens.len().min(*i + 2)]).as_slice(),
            ["gains" | "gain", "the"]
        )
    })?;
    let name = name(&tokens[at + 2..end])?;
    crate::word_claims::claim_tokens(&tokens[at..]);
    Some((tokens[..at].to_vec(), name))
}
