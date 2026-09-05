//! Statements that partition a viewed group in one sentence.
//!
//! "Put one of them into your hand and the rest on the bottom of your library
//! in any order", "Put one of those cards into your hand, one on top of your
//! library, and one on the bottom", "Put all creature cards revealed this way
//! into your hand and the rest on the bottom": each selects and disposes of
//! the remainder in the same sentence, so each is one statement that closes
//! the selection. These come first after the view, before the plain selection
//! statements read the sentence.

use super::super::dispatch_entry::SentenceInput;
use super::super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::parse_may_exile_filtered_looked_card;
use super::super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::{
    looked_library_owner_filter, move_looked_partition_group, move_tagged_to_looked_destination,
    tagged_library_candidate_filter,
};
use crate::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::grammar::sentence_markers::{self, ConditionalFollowupActor};
use super::{ViewStyle, ViewedGroup, it};
use crate::cards::builders::{
    ChoiceCount, EffectAst, ObjectFilter, PlayerAst, PredicateAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TargetAst, Value,
};
use crate::grammar::effects::{
    LookExileFaceDownShape, LookedCardDestinationShape, parse_exact_looked_card_move_shape,
    parse_look_exile_face_down_shape,
};
use crate::cards::builders::CardTextError;
use crate::tag::TagKey;
use crate::grammar::effects::looked_card_shapes::CountedLookedHandRemainderShape;
use crate::grammar::effects::{
    LookedCardDisposition, LookedCardPartitionShape, LookedPartitionDestination,
    RevealTopMatchingFollowupShape, RevealTopRemainder, ThreeWayLookedCardDispositionShape,
    parse_counted_looked_hand_remainder_shape, parse_looked_card_disposition,
    parse_looked_card_partition_shape, parse_reveal_top_matching_followup_shape,
    parse_three_way_looked_card_disposition_shape,
};
use crate::lexer::OwnedLexToken;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

/// A view that carries a counted face-down exile in the same clause, or a view
/// and its face-down follow-up read together.
pub(super) fn face_down_exile_clause(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    super::super::dispatch_inner::parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(
        tokens,
    )
}

/// "You may play that card for as long as it remains exiled" / "You may cast
/// that card this turn" / "You may cast the exiled card without paying its
/// mana cost": the permission over the card the group exiled, whichever of the
/// three its sentence grants. A permission for as long as the card remains
/// exiled, one until end of turn, and an immediate cast stay distinct.
pub(super) fn exiled_permission(sentence: &SentenceInput, exiled: TagKey) -> Option<EffectAst> {
    let permission =
        crate::permission_helpers::parse_cast_or_play_tagged_clause(sentence.lowered()).ok()??;
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = permission else {
        return None;
    };
    match action {
        SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
            ..
        } => Some(EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            exiled,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
        )),
        SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            surface,
            ..
        } => Some(
            EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
                exiled,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                surface,
            ),
        ),
        SubjectVerbActionAst::CastTagged {
            player,
            allow_land,
            as_copy: false,
            without_paying_mana_cost,
            additional_mana_cost,
            cost_reduction,
            mana_spend_mode,
            ..
        } => Some(
            EffectAst::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
                exiled,
                player,
                allow_land,
                false,
                without_paying_mana_cost,
                additional_mana_cost,
                cost_reduction,
                mana_spend_mode,
            ),
        ),
        _ => None,
    }
}

/// "You may exile a <filter> card from among them" / "You may exile one of
/// them": the filter of the card to exile.
pub(super) fn exile_selection_shape(sentence: &SentenceInput) -> Option<ObjectFilter> {
    crate::grammar::primitives::probe_shape(parse_may_exile_filtered_looked_card(
        sentence.lowered(),
    ))
    .flatten()
}

/// The exile selection statement: one card chosen from the group and exiled,
/// awaiting the permission that says what may be done with it. The library's
/// owner disposes of the rest.
pub(super) fn exile_selection(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.revealed {
        return Ok(false);
    }
    let Some(mut filter) = parse_may_exile_filtered_looked_card(sentence.lowered())? else {
        return Ok(false);
    };
    let exiled_tag = helper_tag_for_tokens(sentence.lowered(), "exiled");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: ChoiceCount::up_to(1),
        player: group.owner,
        tag: exiled_tag.clone(),
        zone: Zone::Library,
    });
    group.effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(exiled_tag.clone(), None),
        false,
    ));
    group.selected = Some(exiled_tag.clone());
    group.remainder_player = group.owner;
    group.awaiting_permission = Some(exiled_tag);
    Ok(true)
}

/// "Then shuffle." / "Shuffle." after a selection: the library's owner
/// shuffles what remains.
pub(super) fn shuffle_statement(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let tokens = crate::util::strip_leading_token_words_any(tokens, &["then", "and"]);
    let Some(effects) =
        crate::grammar::primitives::probe_shape(super::super::parse_effect_sentence_lexed(tokens))
    else {
        return false;
    };
    if !matches!(
        effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ShuffleLibrary,
            ..
        })]
    ) {
        return false;
    }
    super::selections::spell_pending(group, None);
    group.effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        super::remainder_owner(group.owner),
        SubjectVerbActionAst::ShuffleLibrary,
    ));
    true
}

/// "If you do, [you may] reveal a land card from among them and put it on top
/// of your library, then put the rest on the bottom": the statement over an
/// optional view.
pub(super) fn optional_reveal_top_shape(
    sentence: &SentenceInput,
) -> Option<(
    sentence_markers::LeadingMayActor,
    Vec<OwnedLexToken>,
    triple_grammar::LookedTopAndRemainderActionShape,
)> {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let followup = sentence_markers::parse_conditional_followup_tokens(tokens)?;
    if followup.actor != ConditionalFollowupActor::You {
        return None;
    }
    let followup_tokens = crate::lexer::trim_lexed_commas(followup.tail_tokens);
    let reveal =
        sentence_markers::parse_leading_may_action_tokens(followup_tokens, &["reveal"], true)?;
    let shape = triple_grammar::parse_looked_top_and_remainder_action_shape(reveal.tail_tokens)?;
    Some((reveal.actor, reveal.tail_tokens.to_vec(), shape))
}

fn optional_reveal_top(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    if !group.optional {
        return false;
    }
    let Some((actor, tail, shape)) = optional_reveal_top_shape(sentence) else {
        return false;
    };
    let filter_tokens = crate::lexer::trim_lexed_commas(&tail[shape.filter.clone()]);
    let Some(mut filter) =
        super::super::looked_cards_family::parse_looked_card_reveal_filter(filter_tokens)
    else {
        return false;
    };
    super::super::search_library::normalize_search_library_filter(&mut filter);
    let chooser = super::super::dispatch_entry::leading_may_actor_to_player(actor, group.owner);
    let chosen_tag = helper_tag_for_tokens(
        crate::lexer::trim_lexed_commas(sentence.lowered()),
        "revealed",
    );
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: shape.count,
        player: chooser,
        tag: chosen_tag.clone(),
        zone: Zone::Library,
    });
    group.effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_reveal_tagged(chosen_tag.clone())],
    });
    group.effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            it(),
            Zone::Library,
            true,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    group.effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            group.tag.clone(),
            Some(chosen_tag.clone()),
            shape.remainder_order,
            group.owner,
        ),
    );
    group.selected = Some(chosen_tag);
    true
}

/// "Put one of them into your graveyard": exactly one looked card to the
/// graveyard, from a library whose owner the filter can name.
pub(super) fn exact_one_to_graveyard_shape(
    sentence: &SentenceInput,
    revealed: bool,
    owner: PlayerAst,
) -> Option<crate::target::PlayerFilter> {
    if revealed {
        return None;
    }
    let shape = parse_exact_looked_card_move_shape(sentence.lowered())?;
    if shape.destination != LookedCardDestinationShape::Graveyard {
        return None;
    }
    looked_library_owner_filter(owner)
}

fn exact_one_to_graveyard(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    let Some(owner_filter) = exact_one_to_graveyard_shape(sentence, group.revealed, group.owner)
    else {
        return false;
    };
    let selected_tag = helper_tag_for_tokens(sentence.lowered(), "looked_selected");
    let selected_filter =
        tagged_library_candidate_filter(&group.tag, &[]).owned_by(owner_filter);
    let mut move_selected = move_tagged_to_looked_destination(
        selected_tag.clone(),
        LookedCardDestinationShape::Graveyard,
    );
    move_selected = if group.owner == PlayerAst::You {
        move_selected.with_destination_player_surface(Some(PlayerAst::You))
    } else {
        move_selected.with_destination_player_reference_surface(Some(
            ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer,
        ))
    };
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: selected_filter,
        count: ChoiceCount::exactly(1),
        player: PlayerAst::You,
        tag: selected_tag.clone(),
        zone: Zone::Library,
    });
    group.effects.push(move_selected);
    group.selected = Some(selected_tag);
    true
}

/// "Look at the top four cards of target opponent's library, exile one of
/// them face down, then put the rest on the bottom of that library in a random
/// order." (or "look at the top card of that player's library, then exile it
/// face down"), followed by the permission to play the exiled card: the view
/// and the exile are one sentence, so the group opens already selected and
/// awaits the permission.
pub(super) fn open_exiled_face_down(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Option<ViewedGroup> {
    let sentence = sentences.get(sentence_idx)?;
    let next = sentences.get(sentence_idx + 1)?;
    let first_tokens = sentence.lowered();
    let shape = parse_look_exile_face_down_shape(first_tokens)?;
    let (look, counted) = match &shape {
        LookExileFaceDownShape::Counted {
            look,
            exile,
            count,
            bottom_order,
        } => (look.clone(), Some((exile.clone(), *count, Some(*bottom_order)))),
        LookExileFaceDownShape::CountedGraveyardRemainder { look, exile, count } => {
            (look.clone(), Some((exile.clone(), *count, None)))
        }
        LookExileFaceDownShape::Single { look } => (look.clone(), None),
    };
    let look_tokens = crate::lexer::trim_lexed_commas(&first_tokens[look]);
    let look_effects = super::super::parse_effect_sentence_lexed(look_tokens).ok()?;
    let [EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player: owner, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
    })] = look_effects.as_slice()
    else {
        return None;
    };
    let (owner, count) = (*owner, count.clone());
    let looked_tag = helper_tag_for_tokens(first_tokens, "looked");
    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        owner,
        count.clone(),
        looked_tag.clone(),
    )];
    let exiled_tag = match counted {
        Some((exile, exile_count, bottom_order)) => {
            let exile_tokens = crate::lexer::trim_lexed_commas(&first_tokens[exile]);
            let exiled_tag = helper_tag_for_tokens(exile_tokens, "exiled");
            let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
            choice_filter.zone = Some(Zone::Library);
            effects.push(EffectAst::ChooseTaggedObjectsInZone {
                filter: choice_filter,
                count: exile_count,
                player: PlayerAst::You,
                tag: exiled_tag.clone(),
                zone: Zone::Library,
            });
            effects.push(EffectAst::subject_verb_exile(
                TargetAst::Tagged(exiled_tag.clone(), None),
                true,
            ));
            effects.push(match bottom_order {
                Some(bottom_order) => EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    looked_tag.clone(),
                    Some(exiled_tag.clone()),
                    bottom_order,
                    owner,
                ),
                None => EffectAst::subject_verb(
                    SubjectVerbRoleAst::Actor,
                    PlayerAst::Implicit,
                    SubjectVerbActionAst::PutTaggedRemainderInZone {
                        tag: looked_tag.clone(),
                        keep_tagged: exiled_tag.clone(),
                        zone: Zone::Graveyard,
                        surface: ironsmith_core::LibraryRemainderSurface::Rest,
                    },
                ),
            });
            exiled_tag
        }
        None => {
            effects.push(EffectAst::subject_verb_exile(
                TargetAst::Tagged(looked_tag.clone(), None),
                true,
            ));
            looked_tag.clone()
        }
    };
    // The view claims its sentence only when the permission follows.
    exiled_permission(next, exiled_tag.clone())?;
    Some(ViewedGroup {
        tag: looked_tag,
        owner,
        count,
        revealed: false,
        view_style: ViewStyle::Absorbed,
        view_tokens: first_tokens.to_vec(),
        selected: Some(exiled_tag.clone()),
        pending: None,
        remainder_player: super::remainder_owner(owner),
        gated: false,
        optional: false,
        awaiting_permission: Some(exiled_tag),
        pending_statements: std::collections::VecDeque::new(),
        effects,
        first_sentence: sentence_idx,
        consumed: 1,
    })
}

/// "Put one of them into your hand and the rest on the bottom of your library
/// in any order": selection and remainder in one sentence, from your own
/// library.
pub(super) fn counted_hand_remainder_shape(
    sentence: &SentenceInput,
    revealed: bool,
    owner: PlayerAst,
) -> Option<CountedLookedHandRemainderShape> {
    if revealed || owner != PlayerAst::You {
        return None;
    }
    parse_counted_looked_hand_remainder_shape(sentence.lowered())
}

/// "Put one of them on top of your library and the rest into your graveyard":
/// a selection and a remainder, each with its own destination.
pub(super) fn partition_shape(
    sentence: &SentenceInput,
    revealed: bool,
) -> Option<LookedCardPartitionShape> {
    if revealed {
        return None;
    }
    parse_looked_card_partition_shape(crate::lexer::trim_lexed_commas(sentence.lowered()))
}

/// "Put one of them into your hand and the other on the bottom of your
/// library": one card to hand, the rest to a second destination.
pub(super) fn singleton_hand_disposition(
    sentence: &SentenceInput,
    revealed: bool,
) -> Option<LookedCardDisposition> {
    if revealed {
        return None;
    }
    parse_looked_card_disposition(crate::lexer::trim_lexed_commas(sentence.lowered()))
}

/// "Put one into your hand, one on top of your library, and one on the
/// bottom": three looked cards, each to its own destination.
pub(super) fn three_way_disposition(
    sentence: &SentenceInput,
    revealed: bool,
    count: &Value,
) -> Option<ThreeWayLookedCardDispositionShape> {
    if revealed || *count != Value::Fixed(3) {
        return None;
    }
    parse_three_way_looked_card_disposition_shape(crate::lexer::trim_lexed_commas(
        sentence.lowered(),
    ))
}

/// "Put all creature cards revealed this way into your hand and the rest on
/// the bottom of your library in a random order."
pub(super) fn reveal_top_followup(
    sentence: &SentenceInput,
    revealed: bool,
    owner: PlayerAst,
) -> Option<RevealTopMatchingFollowupShape> {
    if !revealed || owner != PlayerAst::You {
        return None;
    }
    parse_reveal_top_matching_followup_shape(crate::lexer::trim_lexed_commas(sentence.lowered()))
}

/// The first statement after the view, when it partitions the group in one
/// sentence. Returns false when the sentence is none of these.
pub(super) fn first_statement(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    // The counted face-down exile recognizer reads the view and its
    // follow-up as one clause and spells the view itself.
    let mut combined = group.view_tokens.clone();
    combined.extend_from_slice(sentence.lowered());
    if let Some(effects) = face_down_exile_clause(&combined) {
        group.view_style = ViewStyle::Absorbed;
        group.effects = effects;
        group.selected = Some(group.tag.clone());
        return true;
    }
    // Exactly one card to the graveyard is read before the partitions: the
    // program it replaces ranked ahead of them.
    if exact_one_to_graveyard(group, sentence) {
        return true;
    }
    if let Some(shape) = counted_hand_remainder_shape(sentence, group.revealed, group.owner) {
        counted_into_hand_with_remainder(group, sentence, shape);
        return true;
    }
    if let Some(shape) = partition_shape(sentence, group.revealed) {
        partition(group, sentence, shape);
        return true;
    }
    if let Some(disposition) = singleton_hand_disposition(sentence, group.revealed) {
        singleton_hand_partition(group, sentence, disposition);
        return true;
    }
    if let Some(shape) = three_way_disposition(sentence, group.revealed, &group.count) {
        three_way(group, sentence, shape);
        return true;
    }
    if let Some(shape) = reveal_top_followup(sentence, group.revealed, group.owner) {
        return reveal_top_matching(group, sentence, shape);
    }
    optional_reveal_top(group, sentence)
}

fn counted_into_hand_with_remainder(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    shape: CountedLookedHandRemainderShape,
) {
    group.tag = helper_tag_for_tokens(&group.view_tokens, "looked_partition");
    let selected_tag = helper_tag_for_tokens(sentence.lowered(), "partition_selected");
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: tagged_library_candidate_filter(&group.tag, &[]),
        count: shape.count,
        player: PlayerAst::You,
        tag: selected_tag.clone(),
        zone: Zone::Library,
    });
    group.effects.push(EffectAst::ForEachTagged {
        tag: selected_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            it(),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    group.effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            group.tag.clone(),
            Some(selected_tag.clone()),
            shape.remainder_order,
            PlayerAst::You,
        ),
    );
    group.selected = Some(selected_tag);
}

fn partition(group: &mut ViewedGroup, sentence: &SentenceInput, shape: LookedCardPartitionShape) {
    group.tag = helper_tag_for_tokens(&group.view_tokens, "looked_partition");
    let selected_tag = helper_tag_for_tokens(sentence.lowered(), "partition_selected");
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: tagged_library_candidate_filter(&group.tag, &[]),
        count: shape.selected_count,
        player: PlayerAst::You,
        tag: selected_tag.clone(),
        zone: Zone::Library,
    });
    if let (
        LookedPartitionDestination::LibraryTop(selected_order),
        LookedPartitionDestination::LibraryBottom(remainder_order),
    ) = (shape.selected_destination, shape.remainder_destination)
    {
        group.effects.push(EffectAst::ForEachTagged {
            tag: selected_tag.clone(),
            effects: vec![
                EffectAst::subject_verb_move_to_zone(
                    it(),
                    Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
                .with_library_order(Some(selected_order), PlayerAst::You),
            ],
        });
        group.effects.push(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                group.tag.clone(),
                Some(selected_tag.clone()),
                remainder_order,
                group.owner,
            ),
        );
    } else {
        let remainder_tag = helper_tag_for_tokens(sentence.lowered(), "partition_remainder");
        group
            .effects
            .push(EffectAst::subject_verb_tag_matching_objects(
                tagged_library_candidate_filter(&group.tag, std::slice::from_ref(&selected_tag)),
                vec![Zone::Library],
                remainder_tag.clone(),
            ));
        group.effects.push(move_looked_partition_group(
            selected_tag.clone(),
            shape.selected_destination,
            group.owner,
        ));
        group.effects.push(move_looked_partition_group(
            remainder_tag,
            shape.remainder_destination,
            group.owner,
        ));
    }
    group.selected = Some(selected_tag);
}

fn singleton_hand_partition(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    disposition: LookedCardDisposition,
) {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let hand_tag = helper_tag_for_tokens(tokens, "hand");
    let remainder_tag = helper_tag_for_tokens(tokens, "remainder");
    let remainder_destination = match disposition {
        LookedCardDisposition::HandAndLibraryBottom(order) => {
            LookedPartitionDestination::LibraryBottom(order)
        }
        LookedCardDisposition::HandAndGraveyard => LookedPartitionDestination::Graveyard,
    };
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: tagged_library_candidate_filter(&group.tag, &[]),
        count: ChoiceCount::exactly(1),
        player: group.owner,
        tag: hand_tag.clone(),
        zone: Zone::Library,
    });
    group
        .effects
        .push(EffectAst::subject_verb_tag_matching_objects(
            tagged_library_candidate_filter(&group.tag, std::slice::from_ref(&hand_tag)),
            vec![Zone::Library],
            remainder_tag.clone(),
        ));
    group.effects.push(move_looked_partition_group(
        hand_tag.clone(),
        LookedPartitionDestination::Hand,
        group.owner,
    ));
    group.effects.push(move_looked_partition_group(
        remainder_tag,
        remainder_destination,
        group.owner,
    ));
    group.selected = Some(hand_tag);
}

fn three_way(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    shape: ThreeWayLookedCardDispositionShape,
) {
    group.tag = helper_tag_for_tokens(&group.view_tokens, "looked_candidates");
    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let chosen_tags = [
        helper_tag_for_tokens(tokens, "looked_choice_0"),
        helper_tag_for_tokens(tokens, "looked_choice_1"),
        helper_tag_for_tokens(tokens, "looked_choice_2"),
    ];
    for (index, tag) in chosen_tags.iter().enumerate() {
        group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
            filter: tagged_library_candidate_filter(&group.tag, &chosen_tags[..index]),
            count: ChoiceCount::exactly(1),
            player: group.owner,
            tag: tag.clone(),
            zone: Zone::Library,
        });
    }
    for (tag, destination) in chosen_tags.iter().cloned().zip(shape.destinations()) {
        group
            .effects
            .push(move_tagged_to_looked_destination(tag, destination));
    }
    group.selected = chosen_tags.last().cloned();
}

/// Returns false when the matched-card filter does not parse; the sentence is
/// then not this statement.
fn reveal_top_matching(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    shape: RevealTopMatchingFollowupShape,
) -> bool {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let filter_tokens = crate::lexer::trim_lexed_commas(&tokens[shape.filter.clone()]);
    let Some(mut filter) =
        super::super::looked_cards_family::parse_looked_card_reveal_filter(filter_tokens)
    else {
        return false;
    };
    if shape.chosen_type_reference {
        filter.chosen_creature_type = true;
    }
    super::super::search_library::normalize_search_library_filter(&mut filter);
    filter.zone = None;
    group.view_style = ViewStyle::LookThenRevealTagged;
    match shape.remainder {
        RevealTopRemainder::LibraryBottom(order) => {
            let matched_tag = helper_tag_for_tokens(tokens, "matched");
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: group.tag.clone(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
            group
                .effects
                .push(EffectAst::subject_verb_tag_matching_objects(
                    filter,
                    vec![Zone::Library],
                    matched_tag.clone(),
                ));
            group.effects.push(EffectAst::ForEachTagged {
                tag: matched_tag.clone(),
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    it(),
                    Zone::Hand,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            });
            group.effects.push(
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    group.tag.clone(),
                    Some(matched_tag.clone()),
                    order,
                    PlayerAst::You,
                ),
            );
            group.selected = Some(matched_tag);
        }
        RevealTopRemainder::Graveyard => {
            group.effects.push(EffectAst::ForEachTagged {
                tag: group.tag.clone(),
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::TaggedMatches(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        filter,
                    ),
                    if_true: vec![EffectAst::subject_verb_move_to_zone(
                        it(),
                        Zone::Hand,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        it(),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            });
            group.selected = Some(group.tag.clone());
        }
    }
    true
}
