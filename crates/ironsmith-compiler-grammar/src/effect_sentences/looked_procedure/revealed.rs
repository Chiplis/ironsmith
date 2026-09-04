//! Statements over a revealed group made by or about an opponent.
//!
//! "Reveal the top three cards of your library. An opponent chooses one of
//! them. Put that card into your graveyard and the rest into your hand." binds
//! the revealed group, lets a chosen opponent (or the targeted one, or you)
//! choose from it, and then moves the chosen card and the rest. "An opponent
//! exiles a nonland card from among them, then you put the rest into your
//! hand. That opponent may cast the exiled card without paying its mana cost."
//! is the same group with the opponent's choice spelled as an exile and a cast
//! permission for that same opponent.

use super::super::dispatch_entry::SentenceInput;
use super::super::looked_cards_family::parse_looked_card_choice_filter;
use super::super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::{
    move_tagged_to_looked_destination, tagged_library_candidate_filter,
};
use super::ViewedGroup;
use crate::cards::builders::{
    ChoiceCount, EffectAst, ObjectFilter, PlayerAst, ReturnControllerAst, TargetAst,
};
use crate::grammar::effects::looked_card_shapes::{
    RevealedCardChooserShape, parse_chosen_card_move_followup_shape,
    parse_chosen_card_partition_shape, parse_opponent_revealed_card_selection_shape,
    parse_revealed_card_choice_shape,
};
use crate::grammar::effects::triple_sequence_shapes::parse_opponent_exile_then_hand_shape;
use crate::target::{PlayerFilter, TaggedOpbjectRelation};
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

fn trimmed(sentence: &SentenceInput) -> &[crate::lexer::OwnedLexToken] {
    crate::lexer::trim_lexed_commas(sentence.lowered())
}

/// "You choose one of those cards [and put it into their graveyard]" /
/// "Target opponent chooses one of those cards": one revealed card chosen,
/// moved when the sentence says where.
pub(super) fn revealed_choice(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    if !group.revealed {
        return false;
    }
    let Some(shape) = parse_revealed_card_choice_shape(sentence.lowered()) else {
        return false;
    };
    let chooser = match shape.chooser {
        RevealedCardChooserShape::You => PlayerAst::You,
        RevealedCardChooserShape::TargetOpponent => PlayerAst::TargetOpponent,
    };
    // The program this replaces named the group "revealed_candidates".
    group.tag = helper_tag_for_tokens(&group.view_tokens, "revealed_candidates");
    let chosen_tag = helper_tag_for_tokens(sentence.lowered(), "revealed_choice");
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: tagged_library_candidate_filter(&group.tag, &[]),
        count: ChoiceCount::exactly(1),
        player: chooser,
        tag: chosen_tag.clone(),
        zone: Zone::Library,
    });
    if let Some(destination) = shape.destination {
        group
            .effects
            .push(move_tagged_to_looked_destination(chosen_tag.clone(), destination));
    }
    group.selected = Some(chosen_tag);
    true
}

/// "Put that card into your graveyard, then draw two cards.": the chosen
/// card moved, then whatever follows in the sentence.
pub(super) fn chosen_move_followup(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
) -> Result<bool, crate::cards::builders::CardTextError> {
    let Some(chosen) = group.selected.clone() else {
        return Ok(false);
    };
    let tokens = trimmed(sentence);
    let Some(shape) = parse_chosen_card_move_followup_shape(tokens) else {
        return Ok(false);
    };
    let followup_tokens = crate::lexer::trim_lexed_commas(&tokens[shape.followup]);
    let followups = super::super::parse_effect_sentence_lexed(followup_tokens)?;
    if followups.is_empty() {
        return Ok(false);
    }
    group
        .effects
        .push(move_tagged_to_looked_destination(chosen, shape.destination));
    group.effects.extend(followups);
    Ok(true)
}

/// "An opponent chooses a creature card from among them": you choose the
/// opponent, who then chooses from the group.
pub(super) fn opponent_selection(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
) -> Result<bool, crate::cards::builders::CardTextError> {
    if !group.revealed {
        return Ok(false);
    }
    let tokens = trimmed(sentence);
    let Some(selection) = parse_opponent_revealed_card_selection_shape(tokens) else {
        return Ok(false);
    };
    // The program this replaces named the group "revealed_pool".
    group.tag = helper_tag_for_tokens(&group.view_tokens, "revealed_pool");
    let opponent_tag = helper_tag_for_tokens(sentence.lowered(), "choosing_opponent");
    let selected_tag = helper_tag_for_tokens(sentence.lowered(), "revealed_choice");
    let mut selected_filter = if let Some(range) = selection.filter {
        parse_looked_card_choice_filter(&tokens[range]).ok_or_else(|| {
            crate::cards::builders::CardTextError::ParseError(
                "unable to parse opponent's revealed-card selection filter".to_string(),
            )
        })?
    } else {
        ObjectFilter::default()
    };
    selected_filter.zone = Some(Zone::Library);
    selected_filter =
        selected_filter.match_tagged(group.tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
    group.effects.push(EffectAst::subject_verb_choose_player(
        PlayerAst::You,
        PlayerFilter::Opponent,
        opponent_tag,
        false,
        0,
    ));
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: selected_filter,
        count: ChoiceCount::exactly(1),
        player: PlayerAst::That,
        tag: selected_tag.clone(),
        zone: Zone::Library,
    });
    group.selected = Some(selected_tag);
    group.remainder_player = PlayerAst::That;
    Ok(true)
}

/// "Put that card onto the battlefield and the rest into your graveyard":
/// the chosen card and the remainder each to their destination.
pub(super) fn chosen_partition(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    let Some(selected) = group.selected.clone() else {
        return false;
    };
    let Some(partition) = parse_chosen_card_partition_shape(sentence.lowered()) else {
        return false;
    };
    let remainder_tag = helper_tag_for_tokens(sentence.lowered(), "revealed_remainder");
    group.effects.push(EffectAst::subject_verb_tag_matching_objects(
        tagged_library_candidate_filter(&group.tag, std::slice::from_ref(&selected)),
        vec![Zone::Library],
        remainder_tag.clone(),
    ));
    group.effects.push(move_tagged_to_looked_destination(
        selected,
        partition.selected_destination,
    ));
    group.effects.push(move_tagged_to_looked_destination(
        remainder_tag,
        partition.remainder_destination,
    ));
    true
}

/// "An opponent exiles a nonland card from among them, then you put the rest
/// into your hand." followed by "That opponent may cast the exiled card
/// without paying its mana cost.": the two sentences read together; the
/// second is consumed as the pending cast statement.
pub(super) fn opponent_exile_then_hand_shape(
    group_owner: PlayerAst,
    revealed: bool,
    sentence: &SentenceInput,
    following: Option<&SentenceInput>,
) -> bool {
    revealed
        && group_owner == PlayerAst::You
        && following.is_some_and(|third| {
            parse_opponent_exile_then_hand_shape(trimmed(sentence), trimmed(third)).is_some()
        })
}

pub(super) fn opponent_exile_then_hand(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    following: Option<&SentenceInput>,
) -> bool {
    if !opponent_exile_then_hand_shape(group.owner, group.revealed, sentence, following) {
        return false;
    }
    let second = trimmed(sentence);
    let third = trimmed(following.expect("checked"));
    let Some(shape) = parse_opponent_exile_then_hand_shape(second, third) else {
        return false;
    };
    let Some(mut exile_filter) = parse_looked_card_choice_filter(&second[shape.exile_filter])
    else {
        return false;
    };
    let first = crate::lexer::trim_lexed_commas(&group.view_tokens);
    let opponent_tag = helper_tag_for_tokens(second, "choosing_opponent");
    let exiled_tag = helper_tag_for_tokens(first, "exiled");
    exile_filter.zone = Some(Zone::Library);
    exile_filter =
        exile_filter.match_tagged(group.tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
    let rest_filter = ObjectFilter::tagged(group.tag.clone())
        .not_tagged(exiled_tag.clone())
        .in_zone(Zone::Library);
    group.effects.push(EffectAst::subject_verb_choose_player(
        PlayerAst::You,
        PlayerFilter::Opponent,
        opponent_tag,
        false,
        0,
    ));
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: exile_filter,
        count: ChoiceCount::exactly(1),
        player: PlayerAst::That,
        tag: exiled_tag.clone(),
        zone: Zone::Library,
    });
    group.effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(exiled_tag.clone(), None),
        false,
    ));
    group.effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Object(rest_filter, None, None),
        Zone::Hand,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    // The first opponent reference above creates the concrete player choice;
    // "that opponent" casting the card is that same player.
    group.pending_statements = std::collections::VecDeque::from([vec![EffectAst::MayByPlayer {
        player: PlayerAst::That,
        effects: vec![EffectAst::subject_verb_cast_tagged(
            exiled_tag.clone(),
            PlayerAst::That,
            false,
            false,
            true,
            None,
        )],
    }]]);
    group.selected = Some(exiled_tag);
    true
}
