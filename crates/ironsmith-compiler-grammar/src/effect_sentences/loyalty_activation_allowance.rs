//! One-turn relaxations of the loyalty-ability rule (CR 606.3: one loyalty
//! activation per permanent each turn, at sorcery speed):
//!
//! - "You may activate the loyalty abilities of planeswalkers you control
//!   twice this turn rather than only once." (Urza Assembles the Titans)
//! - "you may activate loyalty abilities of Kaito twice this turn rather than
//!   only once" (Kaito, Dancing Shadow)
//! - "Until end of turn, you may activate loyalty abilities of Jace
//!   planeswalkers you control on any player's turn any time you could cast
//!   an instant." (Jace's Machinations)
//! - "For each planeswalker you control, you may activate one of its loyalty
//!   abilities once this turn as though none of its loyalty abilities have
//!   been activated this turn." (The Chain Veil)
use crate::cards::builders::{CardTextError, EffectAst};
use crate::grammar::{leaf, primitives};
use crate::lexer::{OwnedLexToken, TokenKind, token_word_refs};
use ironsmith_core::{LoyaltyActivationAllowance, LoyaltyActivationScope};
use winnow::Parser;
use winnow::combinator::{alt, opt};

fn is_authored_name(tokens: &[OwnedLexToken]) -> bool {
    !tokens.is_empty()
        && tokens.iter().all(|token| {
            matches!(token.kind, TokenKind::Word)
                && token.slice.chars().next().is_some_and(char::is_uppercase)
        })
}

fn parse_scope(tokens: &[OwnedLexToken]) -> Option<LoyaltyActivationScope> {
    let words = token_word_refs(tokens);
    if crate::util::is_source_reference_words(&words) || is_authored_name(tokens) {
        return Some(LoyaltyActivationScope::Source);
    }
    match words.as_slice() {
        ["planeswalkers", "you", "control"] => {
            Some(LoyaltyActivationScope::ControlledPlaneswalkers { subtype: None })
        }
        [subtype, "planeswalkers", "you", "control"] => {
            let subtype = leaf::parse_leaf_subtype_flexible_complete(subtype).ok()?;
            Some(LoyaltyActivationScope::ControlledPlaneswalkers {
                subtype: Some(subtype),
            })
        }
        _ => None,
    }
}

fn effect(scope: LoyaltyActivationScope, allowance: LoyaltyActivationAllowance) -> EffectAst {
    EffectAst::GrantLoyaltyActivationAllowance { scope, allowance }
}

/// "you may activate Comet's loyalty ability two more times this turn"
/// (Comet, Stellar Pup): that many extra activations of the source's loyalty
/// abilities this turn, one allowance per additional time.
fn parse_more_times_this_turn(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = token_word_refs(tokens);
    let rest = words.strip_prefix(&["you", "may", "activate"])?;
    let loyalty = rest.iter().position(|word| *word == "loyalty")?;
    let owner = &rest[..loyalty];
    let tail = &rest[loyalty + 1..];
    let (count, tail) = match tail {
        ["ability" | "abilities", "an", "additional", "time", tail @ ..] => (1, tail),
        ["ability" | "abilities", count, "more", "time" | "times", tail @ ..] => {
            (crate::util::parse_number_word_u32(count)?, tail)
        }
        _ => return None,
    };
    if tail != ["this", "turn"] || count == 0 || count > 10 {
        return None;
    }
    // The owner is the source, named by its possessive ("Comet's") or as
    // "this planeswalker's".
    let owner = owner.strip_suffix(&["s"]).unwrap_or(owner);
    let owner = owner
        .iter()
        .map(|word| word.trim_end_matches("'s").trim_end_matches("’s"))
        .filter(|word| !word.is_empty() && *word != "'" && *word != "’")
        .collect::<Vec<_>>();
    let first_owner_token = tokens.iter().find(|token| token.kind == TokenKind::Word
        && !matches!(token.slice.to_ascii_lowercase().as_str(), "you" | "may" | "activate"));
    let named_source = first_owner_token
        .is_some_and(|token| token.slice.chars().next().is_some_and(char::is_uppercase));
    if owner.is_empty()
        || !(named_source || crate::util::is_source_reference_words(&owner))
    {
        return None;
    }
    let grant = effect(
        LoyaltyActivationScope::Source,
        LoyaltyActivationAllowance::ExtraActivation,
    );
    Some(if count == 1 {
        grant
    } else {
        EffectAst::Sequence {
            effects: vec![grant; count as usize],
        }
    })
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if let Some(((), _)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["for", "each", "planeswalker", "you", "control"]),
            opt(primitives::comma()),
            primitives::phrase(&[
                "you", "may", "activate", "one", "of", "its", "loyalty", "abilities", "once",
                "this", "turn", "as", "though", "none", "of", "its", "loyalty", "abilities",
                "have", "been", "activated", "this", "turn",
            ]),
            primitives::sentence_end(),
        )
            .void(),
    ) {
        return Ok(Some(effect(
            LoyaltyActivationScope::EachControlledPlaneswalkerNow,
            LoyaltyActivationAllowance::ExtraActivation,
        )));
    }
    if let Some(effect) = parse_more_times_this_turn(tokens) {
        return Ok(Some(effect));
    }
    let (instant_prefix, rest) = match primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["until", "end", "of", "turn"]),
            opt(primitives::comma()),
        )
            .void(),
    ) {
        Some(((), rest)) => (true, rest),
        None => (false, tokens),
    };
    let Some(((), rest)) = primitives::parse_prefix(
        rest,
        (
            opt(primitives::phrase(&["you", "may"])),
            primitives::kw("activate"),
            opt(primitives::kw("the")),
            primitives::phrase(&["loyalty", "abilities", "of"]),
        )
            .void(),
    ) else {
        return Ok(None);
    };
    if let Some((index, (), tail)) = primitives::find_prefix(rest, || {
        primitives::phrase(&["twice", "this", "turn", "rather", "than", "only", "once"])
    }) {
        if instant_prefix || primitives::probe_all(tail, primitives::sentence_end(), "loyalty twice").is_none() {
            return Ok(None);
        }
        let Some(scope) = parse_scope(&rest[..index]) else {
            return Ok(None);
        };
        return Ok(Some(effect(scope, LoyaltyActivationAllowance::ExtraActivation)));
    }
    if let Some((index, (), tail)) = primitives::find_prefix(rest, || {
        (
            primitives::phrase(&["on", "any"]),
            alt((primitives::kw("players"), primitives::kw("player's"), primitives::kw("player’s"))),
            primitives::phrase(&["turn", "any", "time", "you", "could", "cast", "an", "instant"]),
        ).void()
    }) {
        if !instant_prefix
            || primitives::probe_all(tail, primitives::sentence_end(), "loyalty instant").is_none()
        {
            return Ok(None);
        }
        let Some(scope) = parse_scope(&rest[..index]) else {
            return Ok(None);
        };
        return Ok(Some(effect(scope, LoyaltyActivationAllowance::InstantSpeed)));
    }
    Ok(None)
}
