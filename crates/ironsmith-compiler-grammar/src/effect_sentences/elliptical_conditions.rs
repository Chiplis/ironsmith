//! Elliptical conditions: "Draw a card if that creature has a +1/+1 counter
//! on it. If it doesn't, put a +1/+1 counter on it." (Marcus, Mutant Mayor),
//! "Then draw a card if it has seven or more phyresis counters on it. If it
//! doesn't, scry 1." (Weatherlight Compleated).
//!
//! The second sentence repeats the first sentence's subject and elides its
//! predicate: "If it doesn't [have a +1/+1 counter on it]". It is that
//! conditional's false arm. Positive ellipses ("If it does, ...") are left to
//! the result readers, which own "if <someone> does" after an action.

use crate::cards::builders::{CardTextError, ConditionalEffectAst, EffectAst};
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};
use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

/// "If it doesn't," / "If it isn't,": the negated, elided predicate.
fn elliptical_negated_head<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("if").parse_next(input)?;
    alt((
        primitives::kw("it").void(),
        primitives::kw("they").void(),
        primitives::phrase(&["that", "creature"]),
        primitives::phrase(&["that", "permanent"]),
    ))
    .parse_next(input)?;
    alt((
        primitives::kw("doesnt"),
        primitives::kw("doesn't"),
        primitives::kw("dont"),
        primitives::kw("don't"),
        primitives::kw("isnt"),
        primitives::kw("isn't"),
        primitives::kw("arent"),
        primitives::kw("aren't"),
    ))
    .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    Ok(())
}

/// Bind "If it doesn't, ..." to the preceding conditional. Returns `false`
/// when the sentence is not elliptical or nothing precedes it to complete.
pub(super) fn try_merge_elliptical_condition(
    effects: &mut [EffectAst],
    sentence_tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    let Some(((), rest)) = primitives::parse_prefix(sentence_tokens, elliptical_negated_head)
    else {
        return Ok(false);
    };
    if rest.is_empty() {
        return Ok(false);
    }
    let Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate,
        if_false,
        ..
    })) = effects.last_mut()
    else {
        return Ok(false);
    };
    // A populated false arm would make the elision ambiguous.
    if !if_false.is_empty() {
        return Ok(false);
    }
    let mut otherwise = super::parse_effect_sentences_lexed(rest)?;
    if otherwise.is_empty() {
        return Ok(false);
    }
    // "put a +1/+1 counter on it": the false arm's "it" is the object the
    // condition tested, as for "Otherwise, ...".
    ironsmith_compiler_semantic::condition_antecedent::bind_fallback_it_to_condition_tag(
        &mut otherwise,
        predicate,
    );
    *if_false = otherwise;
    Ok(true)
}
