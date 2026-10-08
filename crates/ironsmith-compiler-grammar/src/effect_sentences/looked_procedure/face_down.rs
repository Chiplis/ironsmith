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
        LookedFaceDownRemainder::Bottom(order) => {
            group
                .effects
                .push(EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    crate::tag::TagRef::of(group.tag.clone()),
                    Some(crate::tag::TagRef::of(selected_tag.clone())),
                    order,
                    group.remainder_player,
                ));
        }
        LookedFaceDownRemainder::TopOrBottom => {
            // The looked-at cards still in the library after the face-down
            // entry are the rest; each goes on the top or the bottom.
            let rest_tag = helper_tag_for_tokens(sentence.lowered(), "face_down_rest");
            let mut rest_filter = ObjectFilter::tagged(group.tag.clone());
            rest_filter.zone = Some(Zone::Library);
            group.effects.push(EffectAst::subject_verb_tag_matching_objects(
                rest_filter,
                vec![Zone::Library],
                crate::tag::TagRef::of(rest_tag.clone()),
            ));
            group.effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::TagRef::of(rest_tag),
                effects: vec![EffectAst::subject_verb_move_to_library_top_or_bottom_choice(it())],
            }));
        }
    }
    group.selected = Some(selected_tag.key.clone());
    true
}
