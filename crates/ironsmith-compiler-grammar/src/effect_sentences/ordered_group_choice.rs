//! A revealed group chosen from player by player, in turn order.
//!
//! "Reveal the top ten cards of your library. Starting with the next opponent
//! in turn order, each opponent chooses a different nonland card from among
//! them. Put the chosen cards into your hand and the rest on the bottom of
//! your library in a random order." (Manifold Insights)
//!
//! The players choose one at a time in the stated order, each seeing the
//! earlier choices (CR 101.4); "a different" card is one no earlier player
//! chose. Every choice joins one chosen set; the chosen set and the rest of
//! the revealed group are then disposed of as stated.

use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, ForEachEffectAst, ObjectChoiceEffectAst, PlayerAst,
};
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

/// Who chooses, and in which order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderedParticipants {
    /// "Starting with you, each player"
    PlayersStartingWithYou,
    /// "Starting with the next opponent in turn order, each opponent"
    OpponentsInTurnOrder,
}

struct OrderedGroupChoice<'a> {
    participants: OrderedParticipants,
    different: bool,
    filter_tokens: &'a [OwnedLexToken],
}

fn ordered_group_choice<'a>(
    input: &mut LexStream<'a>,
) -> winnow::error::ModalResult<OrderedGroupChoice<'a>> {
    opt(primitives::kw("then")).parse_next(input)?;
    let participants = alt((
        (
            primitives::phrase(&["starting", "with", "you"]),
            opt(primitives::comma()),
            primitives::phrase(&["each", "player"]),
        )
            .value(OrderedParticipants::PlayersStartingWithYou),
        (
            primitives::phrase(&[
                "starting", "with", "the", "next", "opponent", "in", "turn", "order",
            ]),
            opt(primitives::comma()),
            primitives::phrase(&["each", "opponent"]),
        )
            .value(OrderedParticipants::OpponentsInTurnOrder),
    ))
    .parse_next(input)?;
    primitives::kw("chooses").parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    let different = opt(primitives::kw("different")).parse_next(input)?.is_some();
    let filter_tokens = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["from", "among", "them"])),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["from", "among", "them"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(OrderedGroupChoice {
        participants,
        different,
        filter_tokens,
    })
}

/// "Reveal the top N cards of your library." + the ordered choice from among
/// them + "Put the chosen cards into your hand and the rest on the bottom of
/// your library in <order>."
pub(super) fn read_revealed_group_ordered_choice(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(view), Some(choice), Some(disposition)) = (
        sentences.get(sentence_idx),
        sentences.get(sentence_idx + 1),
        sentences.get(sentence_idx + 2),
    ) else {
        return Ok(None);
    };
    let Some((player, count, true)) = super::parse_top_cards_view_sentence(view.lowered()) else {
        return Ok(None);
    };
    let Some(shape) = primitives::probe_all(
        trim_lexed_commas(choice.lowered()),
        ordered_group_choice,
        "ordered-group-choice",
    ) else {
        return Ok(None);
    };
    let Some(disposition) = crate::grammar::effects::sequence_quad_shapes::parse_chosen_cards_hand_remainder_shape(
        disposition.lowered(),
    ) else {
        return Ok(None);
    };
    let Some(mut filter) = super::parse_looked_card_choice_filter(shape.filter_tokens) else {
        return Ok(None);
    };

    let revealed_tag = helper_tag_for_tokens(view.lowered(), "revealed");
    let chosen_tag = helper_tag_for_tokens(choice.lowered(), "chosen");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: revealed_tag.key.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    if shape.different {
        // Every choice joins the chosen set, so excluding it rules out a
        // card an earlier player already chose.
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: chosen_tag.key.clone(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
    }
    let pick = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: ChoiceCount::exactly(1),
        player: PlayerAst::That,
        tag: crate::tag::TagRef::of(chosen_tag.clone()),
        zone: Zone::Library,
    });
    let process = match shape.participants {
        OrderedParticipants::PlayersStartingWithYou => {
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects: vec![pick] })
        }
        OrderedParticipants::OpponentsInTurnOrder => {
            EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects: vec![pick] })
        }
    };
    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_cards(
            player,
            count,
            crate::tag::TagRef::of(revealed_tag.clone()),
        ),
        // Both orders begin with the controller's seat: "starting with you"
        // includes the controller, while the next opponent in turn order is
        // the first opponent after them.
        EffectAst::SourceSentence {
            effects: vec![process],
            leading_then: false,
            starting_with_controller: true,
        },
        EffectAst::MoveTaggedGroupToZone {
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
            zone: Zone::Hand,
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            crate::tag::TagRef::of(revealed_tag),
            Some(crate::tag::TagRef::of(chosen_tag)),
            disposition.order,
            player,
        ),
    ]))
}
