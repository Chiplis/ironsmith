//! Conditional statements over a viewed group.
//!
//! "Then if you control nine or more Gates, put the rest into your hand.
//! Otherwise, put the rest on the bottom of your library in a random order."
//! decides where the remainder goes; "If you control more creatures than each
//! other player, put two of those cards into your hand. Otherwise, put one of
//! them into your hand." decides how many cards are selected; "You may reveal
//! up to two creature and/or land cards from among them" followed by "Put all
//! land cards revealed this way onto the battlefield tapped and all creature
//! cards revealed this way into your hand" splits the selection by type. Each
//! reads the sentence that follows it as part of the statement.

use super::super::dispatch_entry::SentenceInput;
use super::super::looked_cards_family::{
    is_put_rest_on_bottom_of_library_sentence, parse_counted_looked_cards_into_your_hand_tokens,
    parse_looked_card_choice_filter,
};
use super::{ViewedGroup, it};
use crate::cards::builders::{
    ChoiceCount, EffectAst, ObjectFilter, PlayerAst, PredicateAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbRoleAst,
};
use crate::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::grammar::sentence_markers;
use crate::lexer::OwnedLexToken;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

fn trimmed(sentence: &SentenceInput) -> &[OwnedLexToken] {
    crate::lexer::trim_lexed_commas(sentence.lowered())
}

fn none_pending(count: usize) -> std::collections::VecDeque<Vec<EffectAst>> {
    std::iter::repeat_with(Vec::new).take(count).collect()
}

fn single_conditional_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let parsed =
        crate::grammar::primitives::probe_shape(super::super::parse_effect_sentence_lexed(tokens))?;
    let [EffectAst::Conditional { predicate, .. }] = parsed.as_slice() else {
        return None;
    };
    Some(predicate.clone())
}

fn otherwise_tail(sentence: &SentenceInput) -> Option<&[OwnedLexToken]> {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lexed());
    tokens
        .first()
        .is_some_and(|token| token.is_word("otherwise"))
        .then(|| crate::util::strip_leading_token_words_any(tokens, &["otherwise"]))
}

/// "Then if you control nine or more Gates, put the rest into your hand."
/// followed by "Otherwise, put the rest on the bottom of your library in a
/// random order.": the remainder's destination decided by the condition.
pub(super) fn conditional_remainder_shape(
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> Option<(PredicateAst, crate::cards::builders::LibraryBottomOrderAst)> {
    let [otherwise, ..] = rest else {
        return None;
    };
    // Authored number and type words are kept: a card name can install a
    // source alias matching one of them (Nine-Fingers Keene).
    let conditional_tokens = crate::lexer::trim_lexed_commas(sentence.lexed());
    if !crate::grammar::effects::parse_remainder_to_hand_presence(conditional_tokens) {
        return None;
    }
    let predicate = single_conditional_predicate(conditional_tokens)?;
    let bottom_tokens = otherwise_tail(otherwise)?;
    let triple_grammar::LookedRemainderShape::LibraryBottom(order) =
        triple_grammar::parse_looked_remainder_shape(bottom_tokens)?
    else {
        return None;
    };
    Some((predicate, order))
}

pub(super) fn conditional_remainder(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> bool {
    let Some(selected) = group.selected.clone() else {
        return false;
    };
    let Some((predicate, order)) = conditional_remainder_shape(sentence, rest) else {
        return false;
    };
    let hand_remainder = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::PutTaggedRemainderInZone {
            tag: group.tag.clone(),
            keep_tagged: selected.clone(),
            zone: Zone::Hand,
            surface: ironsmith_core::LibraryRemainderSurface::Rest,
        },
    );
    let bottom_remainder = EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
        group.tag.clone(),
        Some(selected),
        order,
        group.remainder_player,
    );
    group.effects.push(EffectAst::Conditional {
        predicate,
        if_true: vec![hand_remainder],
        if_false: vec![bottom_remainder],
    });
    group.pending_statements = none_pending(1);
    true
}

/// "If you control more creatures than each other player, put two of those
/// cards into your hand." followed by "Otherwise, put one of them into your
/// hand." and the bottom remainder: how many are selected, decided by the
/// condition.
pub(super) fn conditional_hand_counts_shape(
    sentence: &SentenceInput,
    rest: &[SentenceInput],
    revealed: bool,
) -> Option<(PredicateAst, u32, u32, crate::cards::builders::LibraryBottomOrderAst)> {
    let [otherwise, remainder, ..] = rest else {
        return None;
    };
    if revealed {
        return None;
    }
    let conditional_tokens = trimmed(sentence);
    let predicate = single_conditional_predicate(conditional_tokens)?;
    // The ordinary conditional parser proved the sentence; the branch's own
    // leading verb locates its exact counted selection.
    let if_true_start =
        crate::slice_primitives::select_last_position(conditional_tokens, |token| {
            token.is_word("put")
        })?;
    let if_true_count = parse_counted_looked_cards_into_your_hand_tokens(
        crate::lexer::trim_lexed_commas(&conditional_tokens[if_true_start..]),
    )?;
    let if_false_tokens = crate::util::strip_leading_token_words_any(trimmed(otherwise), &["otherwise"]);
    let if_false_count = parse_counted_looked_cards_into_your_hand_tokens(if_false_tokens)?;
    let remainder_tokens =
        crate::util::strip_leading_token_words_any(trimmed(remainder), &["then", "and"]);
    if !is_put_rest_on_bottom_of_library_sentence(remainder_tokens) {
        return None;
    }
    let order = crate::grammar::effects::parse_bottom_order(remainder_tokens)?;
    Some((predicate, if_true_count, if_false_count, order))
}

pub(super) fn conditional_hand_counts(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> bool {
    let Some((predicate, if_true_count, if_false_count, order)) =
        conditional_hand_counts_shape(sentence, rest, group.revealed)
    else {
        return false;
    };
    // The program this replaces named the group "looked_conditional_partition".
    group.tag = helper_tag_for_tokens(
        crate::lexer::trim_lexed_commas(&group.view_tokens),
        "looked_conditional_partition",
    );
    let selected_tag = helper_tag_for_tokens(trimmed(sentence), "conditional_selected");
    let choice = |count: u32| {
        let mut filter = ObjectFilter::tagged(group.tag.clone());
        filter.zone = Some(Zone::Library);
        vec![
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: ChoiceCount::exactly(count as usize),
                player: PlayerAst::You,
                tag: selected_tag.clone(),
                zone: Zone::Library,
            },
            EffectAst::MoveTaggedGroupToZone {
                tag: selected_tag.clone(),
                zone: Zone::Hand,
            },
        ]
    };
    group.effects.push(EffectAst::Conditional {
        predicate,
        if_true: choice(if_true_count),
        if_false: choice(if_false_count),
    });
    group.pending_statements = std::collections::VecDeque::from([
        Vec::new(),
        vec![EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            group.tag.clone(),
            Some(selected_tag.clone()),
            order,
            group.owner,
        )],
    ]);
    group.selected = Some(selected_tag);
    true
}

fn filter_mentions_card_type(filter: &ObjectFilter, card_type: CardType) -> bool {
    filter.card_types.contains(&card_type)
        || filter.any_of.iter().any(|arm| arm.card_types.contains(&card_type))
}

fn filter_only_mentions_creature_or_land_types(filter: &ObjectFilter) -> bool {
    let allowed = |types: &[CardType]| {
        types
            .iter()
            .all(|card_type| matches!(card_type, CardType::Creature | CardType::Land))
    };
    allowed(&filter.card_types) && filter.any_of.iter().all(|arm| allowed(&arm.card_types))
}

/// "You may reveal up to two creature and/or land cards from among them and
/// put the rest on the bottom of your library in a random order." followed by
/// "Put all land cards revealed this way onto the battlefield tapped and all
/// creature cards revealed this way into your hand.", read together.
pub(super) fn reveal_selection_land_creature_split_shape(
    sentence: &SentenceInput,
    rest: &[SentenceInput],
    revealed: bool,
) -> bool {
    let [split, ..] = rest else {
        return false;
    };
    if revealed {
        return false;
    }
    let Some(reveal_action) =
        sentence_markers::parse_leading_may_action_tokens(trimmed(sentence), &["reveal"], false)
    else {
        return false;
    };
    let Some(shape) = triple_grammar::parse_looked_reveal_selection_shape(reveal_action.tail_tokens)
    else {
        return false;
    };
    if !triple_grammar::is_revealed_land_creature_split_shape(split.lowered()) {
        return false;
    }
    let Some(filter) = parse_looked_card_choice_filter(crate::lexer::trim_lexed_commas(
        &reveal_action.tail_tokens[shape.filter],
    )) else {
        return false;
    };
    filter_mentions_card_type(&filter, CardType::Creature)
        && filter_mentions_card_type(&filter, CardType::Land)
        && filter_only_mentions_creature_or_land_types(&filter)
}

pub(super) fn reveal_selection_land_creature_split(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> bool {
    if !reveal_selection_land_creature_split_shape(sentence, rest, group.revealed) {
        return false;
    }
    let second_tokens = trimmed(sentence);
    let reveal_action =
        sentence_markers::parse_leading_may_action_tokens(second_tokens, &["reveal"], false)
            .expect("checked");
    let shape =
        triple_grammar::parse_looked_reveal_selection_shape(reveal_action.tail_tokens).expect("checked");
    let mut selection_filter = parse_looked_card_choice_filter(crate::lexer::trim_lexed_commas(
        &reveal_action.tail_tokens[shape.filter],
    ))
    .expect("checked");
    super::super::search_library::normalize_search_library_filter(&mut selection_filter);
    let player = group.owner;
    let chooser = super::super::dispatch_entry::leading_may_actor_to_player(reveal_action.actor, player);
    let selected_tag = helper_tag_for_tokens(second_tokens, "revealed_selection");
    selection_filter.zone = Some(Zone::Library);
    selection_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let mut land_filter = ObjectFilter::default();
    land_filter.card_types.push(CardType::Land);
    group.effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: selection_filter,
        count: shape.count,
        player: chooser,
        tag: selected_tag.clone(),
        zone: Zone::Library,
    });
    group
        .effects
        .push(EffectAst::subject_verb_reveal_tagged(selected_tag.clone()));
    group.effects.push(EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
        group.tag.clone(),
        Some(selected_tag.clone()),
        shape.remainder_order,
        player,
    ));
    group.pending_statements = std::collections::VecDeque::from([vec![EffectAst::ForEachTagged {
        tag: selected_tag.clone(),
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                crate::tag::CompilerReferenceTag::It.key(),
                land_filter,
            ),
            if_true: vec![EffectAst::subject_verb_move_to_zone(
                it(),
                Zone::Battlefield,
                false,
                ReturnControllerAst::Preserve,
                true,
                None,
            )],
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                it(),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        }],
    }]]);
    group.selected = Some(selected_tag);
    true
}
