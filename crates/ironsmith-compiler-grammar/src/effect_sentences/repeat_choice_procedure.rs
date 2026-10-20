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
    LibraryBottomOrderAst, PermissionEffectAst, PlayerAst, ReturnControllerAst,
    RevealLookActionAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst,
    TargetAst, ObjectChoiceEffectAst,
};
use ironsmith_core::tag::TagKeyWalk as _;

pub(super) struct RepeatChoiceGroup {
    effects: Vec<EffectAst>,
    /// The group every exiled card joins, when a round exiles cards.
    accumulated: Option<crate::tag::TagRef>,
    /// "Look at the top five cards ... As many times as you choose, ...
    /// then look at the top five cards" (Lim-Dûl's Vault): every look
    /// re-tags the same group, so "those cards" in a round and "the last
    /// cards you looked at this way" afterwards name the latest view.
    looked: Option<crate::tag::TagRef>,
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
            | EffectAst::CommaThen { effects }
            | EffectAst::SourceSentence { effects, .. } => {
                found |= accumulate_exiles(effects, tag);
            }
            _ => {}
        }
    }
    found
}

fn look_tag(effects: &[EffectAst]) -> Option<crate::tag::TagRef> {
    effects.iter().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { tag, .. }),
            ..
        }) => Some(tag.clone()),
        EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
        | EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. } => look_tag(effects),
        _ => None,
    })
}

/// Point every look at the top of a library in the round at `tag`.
fn retag_looks(effects: &mut [EffectAst], tag: &crate::tag::TagRef) -> bool {
    let mut found = false;
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
                        tag: looked,
                        ..
                    }),
                ..
            }) => {
                *looked = tag.clone();
                found = true;
            }
            EffectAst::Permissions(PermissionEffectAst::May { effects })
            | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
            | EffectAst::Sequence { effects }
            | EffectAst::CommaThen { effects }
            | EffectAst::SourceSentence { effects, .. }
            | EffectAst::ForEach(ForEachEffectAst::RepeatProcess { effects, .. }) => {
                found |= retag_looks(effects, tag);
            }
            _ => crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                found |= retag_looks(nested, tag);
            }),
        }
    }
    found
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<RepeatChoiceGroup>, CardTextError> {
    if let Some(group) = open_after_look(sentences, sentence_idx)? {
        return Ok(Some(group));
    }
    open_round(sentences, sentence_idx)
}

/// "Look at the top five cards of your library. As many times as you
/// choose, you may pay 1 life, put those cards on the bottom of your library
/// in any order, then look at the top five cards of your library."
fn open_after_look(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<RepeatChoiceGroup>, CardTextError> {
    let Some(look_sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let words = crate::lexer::token_word_refs(look_sentence.lowered());
    if !words.starts_with(&["look", "at", "the", "top"]) {
        return Ok(None);
    }
    let Some(mut group) = open_round(sentences, sentence_idx + 1)? else {
        return Ok(None);
    };
    let mut effects = super::parse_effect_sentence_lexed(look_sentence.lowered())?;
    let Some(_) = look_tag(&effects) else {
        return Ok(None);
    };
    // A shared explicit binding survives reference resolution. The generic
    // `it` placeholder would allocate a different result tag for each look.
    let tag: crate::tag::TagRef = crate::util::helper_tag_for_tokens(
        look_sentence.lowered(), "repeated_look",
    ).into();
    retag_looks(&mut effects, &tag);
    if !retag_looks(&mut group.effects, &tag) {
        return Ok(None);
    }
    effects.append(&mut group.effects);
    group.effects = effects;
    group.looked = Some(tag);
    group.first_sentence = sentence_idx;
    group.consumed = 2;
    Ok(Some(group))
}

fn open_round(
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
    // Source-sentence/sequence wrappers group executable siblings; they are
    // not permission boundaries. Normalize all such wrappers, including one
    // around the first instruction of a comma chain, before finding its May.
    fn flatten_groupings(effects: Vec<EffectAst>, out: &mut Vec<EffectAst>) {
        for effect in effects {
            match effect {
                EffectAst::SourceSentence { effects, .. } | EffectAst::Sequence { effects } => {
                    flatten_groupings(effects, out);
                }
                effect => out.push(effect),
            }
        }
    }
    let mut flat = Vec::new();
    flatten_groupings(body, &mut flat);
    body = flat;
    // The permission applies to the complete repeated instruction sequence,
    // including the bottom move and subsequent look after its life payment.
    if body.len() > 1
        && let Some((first, rest)) = body.split_first_mut()
        && let EffectAst::Permissions(
            PermissionEffectAst::May { effects } | PermissionEffectAst::MayByPlayer { effects, .. }
        ) = first
    {
        effects.extend(rest.iter().cloned());
        body.truncate(1);
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
        looked: None,
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
    if let Some(looked) = group.looked.clone() {
        return continue_after_looks(group, sentence, looked);
    }
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
    // "you may cast any number of spells from among those cards" lowers to a
    // choice plus a per-chosen-card loop whose body casts the iterated card
    // (`it`). That per-iteration `it` is not the group reference: keep loop
    // bodies out of the group rebinding so each chosen card is cast.
    let mut loop_bodies = Vec::new();
    detach_for_each_tagged_bodies(&mut effects, &mut loop_bodies);
    for effect in &mut effects {
        effect.map_tag_keys(&mut |key| {
            if key.as_str() == it || key.as_str() == "it" {
                *key = accumulated.clone().into();
            }
        });
    }
    let mut loop_bodies = loop_bodies.into_iter();
    reattach_for_each_tagged_bodies(&mut effects, &mut loop_bodies);
    // The accumulated tag records the actual exile objects produced by the
    // repeated instruction. A generic tagged choice otherwise searches every
    // zone and IsTaggedObject also accepts a later object with the same
    // stable card id. Casting "from among those cards" needs the original
    // exile object to still be in exile.
    bind_accumulated_exile_choices(&mut effects, &accumulated);
    group.effects.extend(effects);
    group.consumed += 1;
    Ok(true)
}

fn bind_accumulated_exile_choices(effects: &mut [EffectAst], accumulated: &crate::tag::TagRef) {
    let tag: ironsmith_core::tag::TagKey = accumulated.clone().into();
    for effect in effects {
        if let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, .. }) = effect
            && filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == tag
                    && constraint.relation
                        == ironsmith_core::TaggedOpbjectRelation::IsTaggedObject
            })
            && filter.zone.is_none()
        {
            filter.zone = Some(crate::zone::Zone::Exile);
            *filter = filter.clone().match_tagged(
                tag.clone(), ironsmith_core::TaggedOpbjectRelation::SameObjectId,
            );
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            bind_accumulated_exile_choices(nested, accumulated);
        });
    }
}

/// Take the body of every `ForEachTagged` loop (at any depth) out of the
/// tree, in a fixed traversal order, so a tag rewrite skips the loops'
/// per-iteration references.
fn detach_for_each_tagged_bodies(effects: &mut [EffectAst], saved: &mut Vec<Vec<EffectAst>>) {
    for effect in effects.iter_mut() {
        if let EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects: body, .. }) = effect {
            saved.push(std::mem::take(body));
            continue;
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            detach_for_each_tagged_bodies(nested, saved);
        });
    }
}

/// Restore the bodies taken by [`detach_for_each_tagged_bodies`], in the same
/// traversal order.
fn reattach_for_each_tagged_bodies(
    effects: &mut [EffectAst],
    saved: &mut std::vec::IntoIter<Vec<EffectAst>>,
) {
    for effect in effects.iter_mut() {
        if let EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects: body, .. }) = effect {
            if let Some(restored) = saved.next() {
                *body = restored;
            }
            continue;
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            reattach_for_each_tagged_bodies(nested, saved);
        });
    }
}

/// "Then shuffle and put the last cards you looked at this way on top in any
/// order." The cards are found in the shuffled library and placed on top in
/// the controller's chosen order.
fn continue_after_looks(
    group: &mut RepeatChoiceGroup,
    sentence: &SentenceInput,
    looked: crate::tag::TagRef,
) -> Result<bool, CardTextError> {
    if group.consumed != 2 {
        return Ok(false);
    }
    let words = crate::lexer::token_word_refs(sentence.lowered());
    let words = words.strip_prefix(&["then"][..]).unwrap_or(&words);
    const PUT_LAST_LOOKED_ON_TOP: &[&str] = &[
        "shuffle", "and", "put", "the", "last", "cards", "you", "looked", "at", "this", "way",
        "on", "top", "in", "any", "order",
    ];
    if words != PUT_LAST_LOOKED_ON_TOP {
        return Ok(false);
    }
    group.effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::You,
        SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
    ));
    group.effects.push(
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(looked, None),
            crate::zone::Zone::Library,
            true,
            ReturnControllerAst::Preserve,
            false,
            None,
        )
        .with_library_order(Some(LibraryBottomOrderAst::ChooserChooses), PlayerAst::You),
    );
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: RepeatChoiceGroup) -> Vec<EffectAst> {
    group.effects
}
