//! "As many times as you choose, you may <instruction>." (Dance with
//! Calamity, Lim-Dûl's Vault)
//!
//! An optional instruction performed again each time its player chooses to:
//! one repeated process whose round is the optional instruction itself, and
//! which continues while that instruction is done (CR 608.2c). Zero rounds is
//! a legal choice, so the first round is optional too.
//!
//! When the round exiles the top card of a library, every card exiled across
//! the rounds accumulates in one group, and a following sentence's "the
//! cards exiled this way" / "those cards" names that whole group ("If the
//! total mana value of the cards exiled this way is 13 or less, you may cast
//! any number of spells from among those cards ...").

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, EffectAst, ForEachEffectAst, IfResultPredicate, LibraryActionAst,
    PermissionEffectAst, SubjectVerbActionAst, SubjectVerbEffectAst,
};
use ironsmith_core::tag::TagKeyWalk as _;

pub(super) struct RepeatChoiceGroup {
    effects: Vec<EffectAst>,
    /// The group every exiled card joins, when a round exiles cards.
    accumulated: Option<crate::tag::TagRef>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

const HEAD: &[&str] = &["as", "many", "times", "as", "you", "choose"];

/// Add the accumulating group to each top-of-library exile in the round.
fn accumulate_exiles(effects: &mut [EffectAst], tag: &crate::tag::TagRef) -> bool {
    let mut found = false;
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                        accumulated_tags,
                        ..
                    }),
                ..
            }) => {
                accumulated_tags.push(tag.clone());
                found = true;
            }
            EffectAst::Permissions(PermissionEffectAst::May { effects })
            | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
            | EffectAst::Sequence { effects }
            | EffectAst::SourceSentence { effects, .. } => {
                found |= accumulate_exiles(effects, tag);
            }
            _ => {}
        }
    }
    found
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<RepeatChoiceGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let tokens = sentence.lowered();
    let words = crate::lexer::token_word_refs(tokens);
    if !words.starts_with(HEAD)
        || !matches!(words.get(HEAD.len()..HEAD.len() + 2), Some(["you", "may"]))
    {
        return Ok(None);
    }
    // The body begins at "you may", after the head and its comma.
    let Some(body_start) = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.as_word().is_some())
        .nth(HEAD.len())
        .map(|(index, _)| index)
    else {
        return Ok(None);
    };
    let body_tokens = &tokens[body_start..];
    let mut body = super::parse_effect_sentence_lexed(body_tokens)?;
    // A sentence wrapper around the single optional instruction is presentation.
    let unwrapped = if let [EffectAst::SourceSentence { effects, .. }] = body.as_mut_slice()
        && effects.len() == 1
    {
        Some(std::mem::take(effects))
    } else {
        None
    };
    if let Some(unwrapped) = unwrapped {
        body = unwrapped;
    }
    if !matches!(
        body.as_slice(),
        [EffectAst::Permissions(
            PermissionEffectAst::May { .. } | PermissionEffectAst::MayByPlayer { .. }
        )]
    ) {
        return Ok(None);
    }
    let tag = crate::tag::declared_key("exiled_as_many_times_as_chosen");
    let accumulated = accumulate_exiles(&mut body, &tag).then_some(tag);
    let continue_effect_index = body.len() - 1;
    Ok(Some(RepeatChoiceGroup {
        effects: vec![EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
            effects: body,
            continue_effect_index,
            continue_predicate: IfResultPredicate::Did,
            max_iterations: None,
        })],
        accumulated,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

/// The sentence after an accumulating process may name the whole exiled
/// group ("the cards exiled this way", "those cards").
pub(super) fn continue_with(
    group: &mut RepeatChoiceGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    let Some(accumulated) = group.accumulated.clone() else {
        return Ok(false);
    };
    if group.consumed != 1 {
        return Ok(false);
    }
    let words = crate::lexer::token_word_refs(sentence.lowered());
    let names_group = words
        .windows(3)
        .any(|window| window == ["exiled", "this", "way"])
        || words.windows(2).any(|window| window == ["those", "cards"]);
    if !names_group {
        return Ok(false);
    }
    let Ok(mut effects) = super::parse_effect_sentence_lexed(sentence.lowered()) else {
        return Ok(false);
    };
    let it = crate::tag::CompilerReferenceTag::It.as_str();
    for effect in &mut effects {
        effect.map_tag_keys(&mut |key| {
            if key.as_str() == it || key.as_str() == "it" {
                *key = accumulated.clone().into();
            }
        });
    }
    group.effects.extend(effects);
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: RepeatChoiceGroup) -> Vec<EffectAst> {
    group.effects
}
