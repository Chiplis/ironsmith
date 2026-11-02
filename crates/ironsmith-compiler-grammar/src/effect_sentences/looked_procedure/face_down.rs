//! A face-down selection out of the viewed group.
//!
//! "Look at the top two cards of your library. Manifest one of those cards,
//! then put the other on the top or bottom of your library." The selected
//! cards enter face down (CR 701.40a manifest: a 2/2 creature; CR 701.58a
//! cloak: also with ward {2}), and the cards left in the library are then
//! disposed of: each on the top or bottom of the library at its owner's
//! choice, or all on the bottom in the stated order.

use super::super::dispatch_entry::SentenceInput;
use super::{ViewedGroup, it};
use crate::cards::builders::{
    EffectAst, ForEachEffectAst, ObjectChoiceEffectAst, ObjectFilter, PlayerAst,
    ReturnControllerAst, TargetAst,
};
use crate::grammar::effects::{
    LookedFaceDownRemainder, LookedFaceDownSelectionShape, parse_looked_face_down_selection_shape,
};
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

pub(super) fn face_down_selection_shape(
    sentence: &SentenceInput,
) -> Option<LookedFaceDownSelectionShape> {
    parse_looked_face_down_selection_shape(crate::lexer::trim_lexed_commas(sentence.lowered()))
}

pub(super) fn face_down_selection(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    // A revealed group would need its reveal spelled first; only a looked
    // group is read here.
    if group.revealed {
        return false;
    }
    let Some(shape) = face_down_selection_shape(sentence) else {
        return false;
    };
    let selected_tag = helper_tag_for_tokens(sentence.lowered(), "face_down_selected");
    let mut selected_filter = ObjectFilter::tagged(group.tag.clone());
    selected_filter.zone = Some(Zone::Library);
    group.effects.push(EffectAst::ObjectChoices(
        ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: selected_filter,
            count: shape.count,
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(selected_tag.clone()),
            zone: Zone::Library,
        },
    ));
    let selected = TargetAst::Tagged(crate::tag::TagRef::of(selected_tag.clone()), None);
    group.effects.push(if shape.manifest {
        EffectAst::subject_verb_manifest_onto_battlefield(
            PlayerAst::You,
            selected,
            false,
            ReturnControllerAst::You,
            false,
        )
    } else {
        EffectAst::subject_verb_cloak_onto_battlefield(
            PlayerAst::You,
            selected,
            false,
            ReturnControllerAst::You,
            false,
        )
    });
    match shape.remainder {
        LookedFaceDownRemainder::Graveyard => {
            // Re-query the viewed group in the library after moving the
            // selection, so the chosen card is never also sent to the graveyard.
            let rest_tag = helper_tag_for_tokens(sentence.lowered(), "face_down_rest");
            let mut rest_filter = ObjectFilter::tagged(group.tag.clone());
            rest_filter.zone = Some(Zone::Library);
            rest_filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
                tag: selected_tag.key.clone(),
                relation: crate::target::TaggedOpbjectRelation::IsNotTaggedObject,
            });
            group
                .effects
                .push(EffectAst::subject_verb_tag_matching_objects(
                    rest_filter,
                    vec![Zone::Library],
                    crate::tag::TagRef::of(rest_tag.clone()),
                ));
            group.effects.push(EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::TagRef::of(rest_tag), None),
                Zone::Graveyard,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ));
        }
        LookedFaceDownRemainder::Bottom(order) => {
            group.effects.push(
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    crate::tag::TagRef::of(group.tag.clone()),
                    Some(crate::tag::TagRef::of(selected_tag.clone())),
                    order,
                    group.remainder_player,
                ),
            );
        }
        LookedFaceDownRemainder::TopOrBottom => {
            // The looked-at cards still in the library after the face-down
            // entry are the rest; each goes on the top or the bottom.
            let rest_tag = helper_tag_for_tokens(sentence.lowered(), "face_down_rest");
            let mut rest_filter = ObjectFilter::tagged(group.tag.clone());
            rest_filter.zone = Some(Zone::Library);
            group
                .effects
                .push(EffectAst::subject_verb_tag_matching_objects(
                    rest_filter,
                    vec![Zone::Library],
                    crate::tag::TagRef::of(rest_tag.clone()),
                ));
            group
                .effects
                .push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                    tag: crate::tag::TagRef::of(rest_tag),
                    effects: vec![
                        EffectAst::subject_verb_move_to_library_top_or_bottom_choice(it()),
                    ],
                }));
        }
    }
    group.selected = Some(selected_tag.key.clone());
    true
}

/// Read a viewed-card entry and its attached optional action, preserving the
/// original viewed set for the independent remainder reader.
pub(super) fn described_selection_shape<'a>(sentence: &SentenceInput, rest: &'a [SentenceInput]) -> Result<Option<Vec<crate::lexer::OwnedLexToken>>, crate::cards::builders::CardTextError> {
    if rest.len() < 2 { return Ok(None); }
    let mut tokens = sentence.lowered().to_vec();
    tokens.push(crate::lexer::OwnedLexToken::period(crate::cards::builders::TextSpan::synthetic()));
    tokens.extend_from_slice(rest[0].lowered());
    tokens.push(crate::lexer::OwnedLexToken::period(crate::cards::builders::TextSpan::synthetic()));
    tokens.extend_from_slice(rest[1].lowered());
    tokens.push(crate::lexer::OwnedLexToken::period(crate::cards::builders::TextSpan::synthetic()));
    let Some(entry) = super::super::dispatch_entry::described_face_down_entry::read(&tokens)? else { return Ok(None); };
    if entry.permission.is_none() { return Ok(None); }
    if !crate::lexer::TokenWordView::new(entry.target_tokens).word_refs().windows(3).any(|words| words == ["from", "among", "them"])
        && !crate::lexer::TokenWordView::new(entry.target_tokens).word_refs().windows(4).any(|words| words == ["from", "among", "those", "cards"]) { return Ok(None); }
    Ok(Some(tokens))
}

pub(super) fn described_selection(group: &mut ViewedGroup, sentence: &SentenceInput, rest: &[SentenceInput]) -> Result<bool, crate::cards::builders::CardTextError> {
    if group.revealed { return Ok(false); }
    let Some(tokens) = described_selection_shape(sentence, rest)? else { return Ok(false); };
    let entry = super::super::dispatch_entry::described_face_down_entry::read(&tokens)?.unwrap();
    let view = crate::lexer::TokenWordView::new(entry.target_tokens);
    let words = view.word_refs();
    if !matches!(words.first(), Some(&"a" | &"an" | &"one")) && !words.starts_with(&["up", "to", "one"]) { return Ok(false); }
    let Some(from) = words.windows(2).position(|pair| pair == ["from", "among"]) else { return Ok(false); };
    let Some(from_token) = view.token_start_indices().get(from).copied() else { return Ok(false); };
    let Some(mut filter) = super::super::looked_cards_family::parse_looked_card_choice_filter(&entry.target_tokens[..from_token]) else { return Ok(false); };
    crate::word_claims::claim_tokens(&entry.target_tokens[from_token..]);
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
        tag: group.tag.clone(), relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
    });
    let selected = helper_tag_for_tokens(sentence.lowered(), "described_face_down_selected");
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter, count: if entry.optional || words.starts_with(&["up", "to", "one"]) { crate::effect::ChoiceCount::up_to(1) } else { crate::effect::ChoiceCount::exactly(1) },
        player: PlayerAst::You, tag: crate::tag::TagRef::of(selected.clone()), zone: Zone::Library,
    }));
    group.effects.push(super::super::dispatch_entry::described_face_down_entry::build_entry(
        TargetAst::Tagged(crate::tag::TagRef::of(selected.clone()), None), entry.owner, entry.permission,
    ));
    group.selected = Some(selected.into());
    // The permission's two statements are owned by this entry, rather than
    // becoming separately executable cast instructions.
    group.pending_statements.push_back(Vec::new());
    group.pending_statements.push_back(Vec::new());
    Ok(true)
}
