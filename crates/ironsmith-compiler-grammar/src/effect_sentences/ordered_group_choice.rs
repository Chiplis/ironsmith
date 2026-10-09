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

/// "Starting with you, each player chooses one of the exiled cards and puts
/// it onto the battlefield [tapped] under their control."
fn round_robin_pick(input: &mut LexStream<'_>) -> winnow::error::ModalResult<bool> {
    primitives::phrase(&["starting", "with", "you"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "each", "player", "chooses", "one", "of", "the", "exiled", "cards", "and", "puts", "it",
        "onto", "the", "battlefield",
    ])
    .parse_next(input)?;
    let tapped = opt(primitives::kw("tapped")).parse_next(input)?.is_some();
    primitives::phrase(&["under", "their", "control"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(tapped)
}

/// "Exile <...>. Starting with you, each player chooses one of the exiled
/// cards and puts it onto the battlefield tapped under their control. Repeat
/// this process until all cards exiled this way have been chosen." (Thieves'
/// Auction): rounds in turn order (CR 101.4) over the exiled pool. Each pick
/// joins a shared chosen set and is never offered again, so a card that
/// can't enter (CR 303.4g) stays chosen in exile; the process repeats while
/// an unchosen exiled card remains.
pub(super) fn read_round_robin_exiled_pool(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(exile), Some(pick), Some(repeat)) = (
        sentences.get(sentence_idx),
        sentences.get(sentence_idx + 1),
        sentences.get(sentence_idx + 2),
    ) else {
        return Ok(None);
    };
    let Some(tapped) = primitives::probe_all(
        trim_lexed_commas(pick.lowered()),
        round_robin_pick,
        "round-robin-exiled-pick",
    ) else {
        return Ok(None);
    };
    if primitives::probe_all(
        trim_lexed_commas(repeat.lowered()),
        (
            primitives::phrase(&[
                "repeat", "this", "process", "until", "all", "cards", "exiled", "this", "way",
                "have", "been", "chosen",
            ]),
            primitives::sentence_end(),
        ),
        "round-robin-repeat-until-chosen",
    )
    .is_none()
    {
        return Ok(None);
    }
    let exile_effects = super::parse_effect_sentence_lexed(exile.lowered())?;
    let [exile_effect] = exile_effects.as_slice() else {
        return Ok(None);
    };

    let pool = helper_tag_for_tokens(exile.lowered(), "auction_pool");
    let chosen = helper_tag_for_tokens(pick.lowered(), "auction_chosen");
    let pick_tag = helper_tag_for_tokens(pick.lowered(), "auction_pick");
    let never_written = helper_tag_for_tokens(repeat.lowered(), "auction_empty");
    let remaining = helper_tag_for_tokens(repeat.lowered(), "auction_remaining");

    let unchosen_pool = || {
        let mut filter = crate::target::ObjectFilter::tagged(pool.key.clone());
        filter.zone = Some(Zone::Exile);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: chosen.key.clone(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        filter
    };
    let round = vec![
        // This player's pick starts empty (the union of a never-written tag).
        EffectAst::subject_verb_tagged_object_union(
            crate::target::ObjectFilter::default(),
            vec![Zone::Exile],
            crate::tag::TagRef::of(pick_tag.clone()),
            vec![crate::tag::TagRef::of(never_written)],
        ),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: unchosen_pool(),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::That,
            tag: crate::tag::TagRef::of(pick_tag.clone()),
            zone: Zone::Exile,
        }),
        EffectAst::subject_verb_put_onto_battlefield(
            PlayerAst::That,
            crate::cards::builders::TargetAst::Tagged(crate::tag::TagRef::of(pick_tag.clone()), None),
            tapped,
            crate::cards::builders::ReturnControllerAst::Preserve,
        ),
        // The pick joins the chosen set whether or not it could enter.
        EffectAst::subject_verb_tagged_object_union(
            crate::target::ObjectFilter::default(),
            vec![Zone::Exile, Zone::Battlefield],
            crate::tag::TagRef::of(chosen.clone()),
            vec![
                crate::tag::TagRef::of(chosen.clone()),
                crate::tag::TagRef::of(pick_tag),
            ],
        ),
    ];
    Ok(Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(exile_effect.clone()),
            tag: crate::tag::TagRef::of(pool.clone()),
        },
        EffectAst::ForEach(ForEachEffectAst::RepeatProcess {
            effects: vec![
                EffectAst::SourceSentence {
                    effects: vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                        effects: round,
                    })],
                    leading_then: false,
                    starting_with_controller: true,
                },
                // Continue while an exiled card has not been chosen.
                EffectAst::subject_verb_tag_matching_objects(
                    unchosen_pool(),
                    vec![Zone::Exile],
                    crate::tag::TagRef::of(remaining),
                ),
            ],
            continue_effect_index: 1,
            continue_predicate: crate::cards::builders::IfResultPredicate::Did,
        }),
    ]))
}
