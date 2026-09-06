//! A statement and the riders that bind back to it, as one procedure.
//!
//! "Prevent the next 3 damage that would be dealt to any target this turn. You
//! gain life equal to the damage prevented this way." is a statement followed
//! by a sentence that refers back to it — here to the prevention event's
//! amount, which only the shield knows. The sentence loop binds such riders to
//! the preceding effect ([`super::chain_carry`]); this module lets every place
//! that asks whether a program covers a run of sentences see the statement and
//! its riders as one, so the pair is neither split into independently parsed
//! sentences nor kept from the segment structure a program gives.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst};

pub(super) struct RiddenStatement {
    effects: Vec<EffectAst>,
    /// Sentences the opener already read — ordinary sentences between the
    /// statement and its rider, and the rider itself; the continuations
    /// consume them.
    read_ahead: usize,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn bind(effects: &mut Vec<EffectAst>, sentence: &SentenceInput) -> bool {
    super::chain_carry::bind_population_counter_followup(effects, sentence.lowered())
        || super::chain_carry::bind_counted_object_grant_followup(effects, sentence.lowered())
        || super::chain_carry::bind_prevention_followup(effects, sentence.lowered())
        || super::chain_carry::bind_tap_lock(effects, sentence.lowered())
        || super::chain_carry::bind_self_animate_after_life_gain(effects, sentence.lowered())
        || super::chain_carry::bind_destroy_typed_subset(effects, sentence.lowered())
        || super::chain_carry::bind_return_exiled_to_owners_hands(effects, sentence.lowered())
}

/// Open at a statement whose next sentence binds back to it.
pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<RiddenStatement>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let Some(mut effects) =
        crate::grammar::primitives::probe_shape(super::parse_effect_sentence_lexed(sentence.lowered()))
    else {
        return Ok(None);
    };
    if effects.is_empty() {
        return Ok(None);
    }
    // The rider may follow the statement directly or after ordinary sentences
    // ("Exile all creatures. Each player may put … onto the battlefield. Then
    // put all cards exiled this way into their owners' hands."); the sentences
    // between are read as they are.
    let mut read_ahead = 0;
    let mut candidates = std::iter::once(next).chain(
        sentences.get(sentence_idx + 2..).unwrap_or(&[]).iter(),
    );
    loop {
        let Some(candidate) = candidates.next() else {
            return Ok(None);
        };
        read_ahead += 1;
        if bind(&mut effects, candidate) {
            break;
        }
        let Some(between) = crate::grammar::primitives::probe_shape(
            super::parse_effect_sentence_lexed(candidate.lowered()),
        ) else {
            return Ok(None);
        };
        if between.is_empty() || read_ahead > 2 {
            return Ok(None);
        }
        effects.extend(between);
    }
    Ok(Some(RiddenStatement {
        effects,
        read_ahead,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

/// Continue with another rider; false for anything else.
pub(super) fn continue_with(
    group: &mut RiddenStatement,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.read_ahead > 0 {
        group.read_ahead -= 1;
    } else if !bind(&mut group.effects, sentence) {
        return Ok(false);
    }
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: RiddenStatement) -> Vec<EffectAst> {
    group.effects
}
