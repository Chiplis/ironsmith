//! "... or you gain control of a creature of your choice that player controls
//! until end of turn. If you gain control of a creature this way, tap it, and
//! it's attacking that player." (Midnight Crusader Shuttle)
//!
//! The follow-up instruction is a consequence of the one branch of the
//! preceding choice that gains control of an object: it is part of that
//! branch, reads the object that branch chose as "it", and never runs when
//! another branch was chosen (or when no object was chosen).

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst, ForEachEffectAst, ObjectChoiceEffectAst};
use crate::lexer::OwnedLexToken;

pub(super) struct GainedControlFollowupGroup {
    effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn trim(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    crate::util::trim_edge_punctuation_tokens(tokens)
}

/// The instruction after "If you gain control of <object> this way,".
fn followup_body(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let words = crate::lexer::token_word_refs(tokens);
    if !words.starts_with(&["if", "you", "gain", "control", "of"]) {
        return None;
    }
    let way = tokens
        .windows(2)
        .position(|window| window[0].is_word("this") && window[1].is_word("way"))?;
    let mut body = &tokens[way + 2..];
    if body
        .first()
        .is_some_and(|token| token.as_word().is_none() && token.slice == ",")
    {
        body = &body[1..];
    }
    (!body.is_empty()).then_some(body)
}

fn gains_control(effects: &[EffectAst]) -> bool {
    format!("{effects:?}").contains("GainControl")
}

/// Append `followup` to the single choice branch that gains control.
fn attach_to_gain_control_branch(effects: &mut [EffectAst], followup: &[EffectAst]) -> bool {
    for effect in effects {
        match effect {
            EffectAst::ForEach(
                ForEachEffectAst::ForEachPlayersFiltered { effects, .. }
                | ForEachEffectAst::ForEachOpponent { effects }
                | ForEachEffectAst::ForEachPlayer { effects }
                | ForEachEffectAst::ForEachTagged { effects, .. },
            ) => {
                if attach_to_gain_control_branch(effects, followup) {
                    return true;
                }
            }
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::VillainousChoice { modes, .. }) => {
                let mut branches = modes.iter_mut().filter(|mode| gains_control(&mode.effects));
                let (Some(branch), None) = (branches.next(), branches.next()) else {
                    return false;
                };
                branch.effects.extend(followup.iter().cloned());
                return true;
            }
            _ => {}
        }
    }
    false
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<GainedControlFollowupGroup>, CardTextError> {
    let (Some(choice), Some(followup)) =
        (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let Some(body) = followup_body(trim(followup.lowered())) else {
        return Ok(None);
    };
    let Ok(mut effects) = super::parse_effect_sentence_lexed(choice.lowered()) else {
        return Ok(None);
    };
    if !gains_control(&effects) {
        return Ok(None);
    }
    let Ok(followup_effects) = super::parse_effect_sentence_lexed(body) else {
        return Ok(None);
    };
    if !attach_to_gain_control_branch(&mut effects, &followup_effects) {
        return Ok(None);
    }
    Ok(Some(GainedControlFollowupGroup {
        effects,
        first_sentence: sentence_idx,
        consumed: 2,
    }))
}

pub(super) fn continue_with(
    _group: &mut GainedControlFollowupGroup,
    _sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    Ok(false)
}

pub(super) fn finish(group: GainedControlFollowupGroup) -> Vec<EffectAst> {
    group.effects
}
