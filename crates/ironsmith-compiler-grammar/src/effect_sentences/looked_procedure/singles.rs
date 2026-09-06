//! Further statements over a viewed group, each the statement of one card.
//!
//! "You may put one of those cards onto the battlefield if it has the same
//! name as a permanent", "You may put one of those cards on top of your
//! library", "You may reveal a creature or land card from among them and put it
//! on top of your library", "You may reveal up to two creature cards with mana
//! value X or less from among them" followed by "Put the revealed cards into
//! your hand, then shuffle", and "Put one of them into your hand, put one of
//! them on the bottom of your library, and exile one of them" followed by the
//! permission to play the exiled card this turn. Each selects from the group;
//! what disposes of the rest is the remainder statement or the sentence the
//! statement read together with its own.

use crate::cards::builders::ForEachEffectAst;
use super::super::dispatch_entry::{SentenceInput, leading_may_actor_to_player};
use super::super::looked_cards_family::{parse_looked_card_choice_filter, parse_looked_card_reveal_filter};
use super::{ViewStyle, ViewedGroup, it};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, ObjectFilter, PlayerAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TargetAst, CounterActionAst, GrantActionAst, LibraryActionAst, ObjectChoiceEffectAst, ConditionalEffectAst, PermissionEffectAst,
};
use crate::grammar::effects::looked_card_shapes::parse_optional_looked_top_remainder_shape;
use crate::grammar::effects::sequence_quad_shapes as quad_grammar;
use crate::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::grammar::sentence_markers;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

fn trimmed(sentence: &SentenceInput) -> &[crate::lexer::OwnedLexToken] {
    crate::lexer::trim_lexed_commas(sentence.lowered())
}

/// "You may put one of those cards onto the battlefield if it has the same
/// name as a permanent."
pub(super) fn same_name_battlefield_shape(sentence: &SentenceInput, owner: PlayerAst, revealed: bool) -> bool {
    owner == PlayerAst::You
        && !revealed
        && triple_grammar::is_looked_same_name_permanent_battlefield_action(sentence.lowered())
}

pub(super) fn same_name_battlefield(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    if !same_name_battlefield_shape(sentence, group.owner, group.revealed) {
        return false;
    }
    let comparison_tag = helper_tag_for_tokens(sentence.lowered(), "same_name_permanents");
    let chosen_tag = helper_tag_for_tokens(sentence.lowered(), "chosen");
    let mut selection_filter = ObjectFilter::default();
    selection_filter.zone = Some(Zone::Library);
    selection_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    selection_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: comparison_tag.clone().into(),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    group.effects.push(EffectAst::subject_verb_tag_matching_objects(
        ObjectFilter::permanent(),
        vec![Zone::Battlefield],
        crate::tag::TagRef::of(comparison_tag),
    ));
    group.effects.push(EffectAst::Permissions(PermissionEffectAst::May {
        effects: vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: selection_filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: crate::tag::TagRef::of(chosen_tag.clone()),
                zone: Zone::Library,
            }),
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::TagRef::of(chosen_tag.clone()),
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    it(),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }),
        ],
    }));
    group.selected = Some(chosen_tag.key.clone());
    group.remainder_player = PlayerAst::You;
    true
}

/// "You may put one of those cards on top of your library." followed by the
/// bottom remainder, read together.
pub(super) fn optional_top_shape(
    sentence: &SentenceInput,
    following: Option<&SentenceInput>,
    owner: PlayerAst,
    revealed: bool,
) -> bool {
    owner == PlayerAst::You
        && !revealed
        && following.is_some_and(|third| {
            parse_optional_looked_top_remainder_shape(sentence.lowered(), third.lowered()).is_some()
        })
}

pub(super) fn optional_top(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    following: Option<&SentenceInput>,
) -> bool {
    if !optional_top_shape(sentence, following, group.owner, group.revealed) {
        return false;
    }
    let shape = parse_optional_looked_top_remainder_shape(
        sentence.lowered(),
        following.expect("checked").lowered(),
    )
    .expect("checked");
    // The program this replaces named the group "looked_partition".
    group.tag = helper_tag_for_tokens(&group.view_tokens, "looked_partition").into();
    let selected_tag = helper_tag_for_tokens(sentence.lowered(), "partition_selected");
    let mut selected_filter = ObjectFilter::tagged(group.tag.clone());
    selected_filter.zone = Some(Zone::Library);
    group.effects.push(EffectAst::Permissions(PermissionEffectAst::May {
        effects: vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: selected_filter,
                count: shape.count,
                player: PlayerAst::You,
                tag: crate::tag::TagRef::of(selected_tag.clone()),
                zone: Zone::Library,
            }),
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::TagRef::of(selected_tag.clone()),
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    it(),
                    Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }),
        ],
    }));
    group.pending_statements = std::collections::VecDeque::from([vec![
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            crate::tag::TagRef::of(group.tag.clone()),
            Some(crate::tag::TagRef::of(selected_tag.clone())),
            shape.remainder_order,
            PlayerAst::You,
        ),
    ]]);
    group.selected = Some(selected_tag.key.clone());
    true
}

/// "[You may] reveal a creature or land card from among them and put it on
/// top of your library."
pub(super) fn reveal_put_top_shape(sentence: &SentenceInput) -> bool {
    let tokens = trimmed(sentence);
    let Some(action) = sentence_markers::parse_leading_may_action_tokens(tokens, &["reveal"], true)
    else {
        return false;
    };
    let reveal_tokens = crate::lexer::trim_lexed_commas(action.tail_tokens);
    let Some(shape) = triple_grammar::parse_looked_top_action_shape(reveal_tokens) else {
        return false;
    };
    parse_looked_card_reveal_filter(&reveal_tokens[shape.filter]).is_some()
}

pub(super) fn reveal_put_top(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    if !reveal_put_top_shape(sentence) {
        return false;
    }
    let tokens = trimmed(sentence);
    let action = sentence_markers::parse_leading_may_action_tokens(tokens, &["reveal"], true)
        .expect("checked");
    let chooser = leading_may_actor_to_player(action.actor, group.owner);
    let reveal_tokens = crate::lexer::trim_lexed_commas(action.tail_tokens);
    let shape = triple_grammar::parse_looked_top_action_shape(reveal_tokens).expect("checked");
    let mut filter = parse_looked_card_reveal_filter(&reveal_tokens[shape.filter]).expect("checked");
    super::super::search_library::normalize_search_library_filter(&mut filter);
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    if group.revealed {
        group.view_style = ViewStyle::LookThenRevealTagged;
    }
    let chosen_tag = helper_tag_for_tokens(sentence.lowered(), "chosen");
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: ChoiceCount::up_to(1),
        player: chooser,
        tag: crate::tag::TagRef::of(chosen_tag.clone()),
        zone: Zone::Library,
    }));
    group.effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(chosen_tag.clone()),
        effects: vec![EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(chosen_tag.clone()))],
    }));
    group.effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(chosen_tag.clone()),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            it(),
            Zone::Library,
            true,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    }));
    group.selected = Some(chosen_tag.key.clone());
    group.remainder_player = chooser;
    true
}

/// "You may reveal up to two creature cards with mana value X or less from
/// among them." followed by "Put the revealed cards into your hand, then
/// shuffle.", read together.
pub(super) fn reveal_to_hand_then_shuffle_shape(
    sentence: &SentenceInput,
    following: Option<&SentenceInput>,
    revealed: bool,
) -> bool {
    !revealed
        && quad_grammar::parse_may_reveal_looked_card_shape(sentence.lowered()).is_some()
        && following.is_some_and(|third| {
            quad_grammar::parse_put_revealed_into_hand_then_shuffle_shape(third.lowered())
        })
}

pub(super) fn reveal_to_hand_then_shuffle(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    following: Option<&SentenceInput>,
) -> Result<bool, CardTextError> {
    if !reveal_to_hand_then_shuffle_shape(sentence, following, group.revealed) {
        return Ok(false);
    }
    let shape = quad_grammar::parse_may_reveal_looked_card_shape(sentence.lowered()).expect("checked");
    let mut filter = parse_looked_card_choice_filter(shape.filter_tokens).ok_or_else(|| {
        CardTextError::ParseError(
            "unable to parse revealed looked-card selection filter".to_string(),
        )
    })?;
    if let Some(x_value) = shape.x_value {
        let Some(crate::filter::Comparison::LessThanOrEqualExpr(maximum)) =
            filter.mana_value.as_mut()
        else {
            return Ok(false);
        };
        **maximum = crate::util::replace_unbound_x_with_value(
            (**maximum).clone(),
            &x_value,
            "looked-card mana-value selection",
        )?;
    }
    super::super::search_library::normalize_search_library_filter(&mut filter);
    let revealed_tag = helper_tag_for_tokens(sentence.lowered(), "revealed");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: shape.count,
        player: group.owner,
        tag: crate::tag::TagRef::of(revealed_tag.clone()),
        zone: Zone::Library,
    }));
    group
        .effects
        .push(EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(revealed_tag.clone())));
    group.pending_statements = std::collections::VecDeque::from([vec![
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::TagRef::of(revealed_tag.clone()), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            group.owner,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
        ),
    ]]);
    group.selected = Some(revealed_tag.key.clone());
    Ok(true)
}

/// "Put one of them into your hand, put one of them on the bottom of your
/// library, and exile one of them." followed by "You may play the exiled card
/// this turn.", read together.
pub(super) fn hand_bottom_exile_split_shape(
    sentence: &SentenceInput,
    following: Option<&SentenceInput>,
) -> bool {
    triple_grammar::is_hand_bottom_exile_split_shape(sentence.lowered())
        && following.is_some_and(|third| {
            matches!(
                crate::grammar::primitives::probe_shape(
                    crate::permission_helpers::parse_cast_or_play_tagged_clause(third.lowered()),
                )
                .flatten(),
                Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn { .. }),
                    ..
                }))
            )
        })
}

pub(super) fn hand_bottom_exile_split(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    following: Option<&SentenceInput>,
) -> bool {
    if !hand_bottom_exile_split_shape(sentence, following) {
        return false;
    }
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                player: permission_player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                surface,
                ..
            }),
        ..
    })) = crate::grammar::primitives::probe_shape(
        crate::permission_helpers::parse_cast_or_play_tagged_clause(
            following.expect("checked").lowered(),
        ),
    )
    .flatten()
    else {
        return false;
    };
    let player = group.owner;
    let hand_tag = helper_tag_for_tokens(sentence.lowered(), "hand");
    let bottom_tag = helper_tag_for_tokens(sentence.lowered(), "bottom");
    let exiled_tag = helper_tag_for_tokens(sentence.lowered(), "exiled");
    if group.revealed {
        group.view_style = ViewStyle::LookThenRevealTagged;
    }
    let mut hand_filter = ObjectFilter::tagged(group.tag.clone());
    hand_filter.zone = Some(Zone::Library);
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter: hand_filter,
        count: ChoiceCount::exactly(1),
        player,
        tag: crate::tag::TagRef::of(hand_tag.clone()),
        zone: Zone::Library,
    }));
    let mut bottom_filter = ObjectFilter::tagged(group.tag.clone()).not_tagged(hand_tag.clone());
    bottom_filter.zone = Some(Zone::Library);
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter: bottom_filter,
        count: ChoiceCount::exactly(1),
        player,
        tag: crate::tag::TagRef::of(bottom_tag.clone()),
        zone: Zone::Library,
    }));
    let mut exile_filter = ObjectFilter::tagged(group.tag.clone())
        .not_tagged(hand_tag.clone())
        .not_tagged(bottom_tag.clone());
    exile_filter.zone = Some(Zone::Library);
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter: exile_filter,
        count: ChoiceCount::exactly(1),
        player,
        tag: crate::tag::TagRef::of(exiled_tag.clone()),
        zone: Zone::Library,
    }));
    group.effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(crate::tag::TagRef::of(hand_tag), None),
        Zone::Hand,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    group.effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(crate::tag::TagRef::of(bottom_tag), None),
        Zone::Library,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    group.effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(crate::tag::TagRef::of(exiled_tag.clone()), None),
        false,
    ));
    group.pending_statements = std::collections::VecDeque::from([vec![
        EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
            crate::tag::TagRef::of(exiled_tag.clone()),
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            surface,
        ),
    ]]);
    group.selected = Some(exiled_tag.key.clone());
    true
}

/// "If that card has mana value 3 or less, it enters with three additional
/// +1/+1 counters on it.": a condition on the selected card's entry, before
/// the remainder.
pub(super) fn entry_counter_condition(group: &mut ViewedGroup, sentence: &SentenceInput) -> bool {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lexed());
    let Some(parsed) =
        crate::grammar::primitives::probe_shape(super::super::parse_effect_sentence_lexed(tokens))
    else {
        return false;
    };
    let [
        conditional @ EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        }),
    ] = parsed.as_slice()
    else {
        return false;
    };
    if !if_false.is_empty()
        || !matches!(
            if_true.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { count, .. }),
                ..
            })] if count.has_surface_hint(
                ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter
            )
        )
    {
        return false;
    }
    group.effects.push(conditional.clone());
    true
}

/// "You may put a land card from among them onto the battlefield tapped."
/// followed by "If you don't, put a card from among them into your hand." and
/// "Put the rest on the bottom of your library in a random order.": the three
/// read together, spelled as one optional choice with its fallback.
pub(super) fn battlefield_or_hand_split_shape(
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> Result<Option<(PlayerAst, ObjectFilter, bool)>, CardTextError> {
    let [if_not, remainder, ..] = rest else {
        return Ok(None);
    };
    if !super::super::looked_cards_family::parse_if_you_dont_put_card_from_among_them_into_your_hand(
        if_not.lowered(),
    ) || !super::super::looked_cards_family::is_put_rest_on_bottom_of_library_sentence(
        remainder.lowered(),
    ) {
        return Ok(None);
    }
    super::super::looked_cards_family::parse_may_put_filtered_looked_card_onto_battlefield(
        sentence.lowered(),
    )
}

pub(super) fn battlefield_or_hand_split(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> Result<bool, CardTextError> {
    let Some((chooser, battlefield_filter, tapped)) = battlefield_or_hand_split_shape(sentence, rest)?
    else {
        return Ok(false);
    };
    // The composition spells the view itself.
    group.view_style = ViewStyle::Absorbed;
    group.effects =
        super::super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::compose_look_at_top_may_put_onto_battlefield_or_into_hand_rest_bottom(
            &group.view_tokens,
            sentence.lowered(),
            group.owner,
            group.count.clone(),
            group.revealed,
            chooser,
            battlefield_filter,
            tapped,
        );
    group.pending_statements = std::collections::VecDeque::from([Vec::new(), Vec::new()]);
    group.selected = Some(group.tag.clone());
    Ok(true)
}

fn none_pending(count: usize) -> std::collections::VecDeque<Vec<EffectAst>> {
    std::iter::repeat_with(Vec::new).take(count).collect()
}

/// "Exile one of them face down and put the rest on the bottom of your library
/// in a random order." followed by "You may cast the exiled card without paying
/// its mana cost if it's an instant spell with mana value 2 or less." and "If
/// you don't, put that card into your hand.", read together.
pub(super) fn exile_one_cast_else_hand_shape(
    sentence: &SentenceInput,
    rest: &[SentenceInput],
    revealed: bool,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let [cast, if_not, ..] = rest else {
        return Ok(None);
    };
    if revealed
        || !quad_grammar::parse_exile_one_and_bottom_remainder_shape(sentence.lowered())
        || !quad_grammar::parse_exiled_card_hand_followup_shape(if_not.lowered())
    {
        return Ok(None);
    }
    let Some(shape) = quad_grammar::parse_exiled_card_cast_filter_shape(cast.lowered()) else {
        return Ok(None);
    };
    let mut filter = crate::object_filters::parse_object_filter_lexed(shape.filter_tokens, false)?;
    if filter.zone == Some(Zone::Stack) {
        filter.zone = None;
        filter.stack_kind = None;
    }
    Ok(Some(filter))
}

pub(super) fn exile_one_cast_else_hand(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> Result<bool, CardTextError> {
    let Some(cast_filter) = exile_one_cast_else_hand_shape(sentence, rest, group.revealed)? else {
        return Ok(false);
    };
    let player = group.owner;
    let exiled_tag = helper_tag_for_tokens(sentence.lowered(), "exiled");
    let mut choice_filter = ObjectFilter::tagged(group.tag.clone());
    choice_filter.zone = Some(Zone::Library);
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter: choice_filter,
        count: ChoiceCount::exactly(1),
        player,
        tag: crate::tag::TagRef::of(exiled_tag.clone()),
        zone: Zone::Library,
    }));
    group.effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(crate::tag::TagRef::of(exiled_tag.clone()), None),
        true,
    ));
    group.effects.push(EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
        crate::tag::TagRef::of(group.tag.clone()),
        Some(crate::tag::TagRef::of(exiled_tag.clone())),
        crate::cards::builders::LibraryBottomOrderAst::Random,
        player,
    ));
    group.pending_statements = std::collections::VecDeque::from([
        vec![EffectAst::Permissions(PermissionEffectAst::May {
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::TaggedMatches(
                    crate::tag::TagRef::of(exiled_tag.clone()),
                    cast_filter,
                ),
                if_true: vec![EffectAst::subject_verb_cast_tagged(
                    crate::tag::TagRef::of(exiled_tag.clone()),
                    player,
                    false,
                    false,
                    true,
                    None,
                )],
                if_false: Vec::new(),
            })],
        })],
        vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::DidNot,
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::TagRef::of(exiled_tag.clone()), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        })],
    ]);
    group.selected = Some(exiled_tag.key.clone());
    Ok(true)
}

/// "Put one of those cards into your hand." followed by "If this spell was
/// kicked, put two of those cards into your hand instead." and the bottom
/// remainder, read together: one choice, its count decided by the kick.
pub(super) fn kicked_hand_count_shape(sentence: &SentenceInput, rest: &[SentenceInput]) -> Option<(u32, u32, crate::cards::builders::LibraryBottomOrderAst)> {
    let [kicked, remainder, ..] = rest else {
        return None;
    };
    let base = super::super::looked_cards_family::parse_counted_looked_cards_into_your_hand_tokens(
        sentence.lowered(),
    )?;
    let kicked_count =
        super::super::looked_cards_family::parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
            kicked.lowered(),
        )?;
    if !super::super::looked_cards_family::is_put_rest_on_bottom_of_library_sentence(
        remainder.lowered(),
    ) {
        return None;
    }
    let order = crate::grammar::effects::parse_bottom_order(remainder.lowered())?;
    Some((base, kicked_count, order))
}

pub(super) fn kicked_hand_count(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> bool {
    let Some((base, kicked_count, order)) = kicked_hand_count_shape(sentence, rest) else {
        return false;
    };
    let kicked_tokens = rest[0].lowered();
    let player = group.owner;
    group.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: crate::cards::builders::PredicateAst::ThisSpellWasKicked,
        if_true: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
            player,
            ChoiceCount::exactly(kicked_count as usize),
            crate::tag::TagRef::of(helper_tag_for_tokens(kicked_tokens, "looked")),
            crate::tag::TagRef::of(helper_tag_for_tokens(kicked_tokens, "chosen")),
            order,
        ),
        if_false: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
            player,
            ChoiceCount::exactly(base as usize),
            crate::tag::TagRef::of(helper_tag_for_tokens(sentence.lowered(), "looked")),
            crate::tag::TagRef::of(helper_tag_for_tokens(sentence.lowered(), "chosen")),
            order,
        ),
    }));
    group.pending_statements = none_pending(2);
    group.selected = Some(group.tag.clone());
    true
}

fn may_reveal_up_to(
    sentence: &SentenceInput,
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let Some(shape) = quad_grammar::parse_may_reveal_looked_card_shape(sentence.lowered()) else {
        return Ok(None);
    };
    let mut filter = parse_looked_card_choice_filter(shape.filter_tokens).ok_or_else(|| {
        CardTextError::ParseError("unable to parse reveal filter from looked cards".to_string())
    })?;
    filter.zone = Some(Zone::Library);
    Ok(Some((filter, shape.count)))
}

/// "You may reveal a creature card with mana value 3 or less from among them."
/// followed by "You may put it onto the battlefield if it's your turn." and "If
/// you don't put it onto the battlefield, put it into your hand.", read
/// together; the remainder statement follows on its own.
pub(super) fn reveal_then_your_turn_shape(
    sentence: &SentenceInput,
    rest: &[SentenceInput],
    revealed: bool,
) -> bool {
    let [your_turn, if_not, ..] = rest else {
        return false;
    };
    !revealed
        && quad_grammar::parse_may_reveal_looked_card_shape(sentence.lowered()).is_some()
        && super::super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::is_may_put_selected_onto_battlefield_on_your_turn(your_turn.lowered())
        && super::super::sequence_rules::generic_subject_verb_sequences::branching_selection_programs::is_if_selected_not_put_onto_battlefield_put_into_hand(if_not.lowered())
}

pub(super) fn reveal_then_your_turn(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> Result<bool, CardTextError> {
    if !reveal_then_your_turn_shape(sentence, rest, group.revealed) {
        return Ok(false);
    }
    let Some((mut filter, mut reveal_count)) = may_reveal_up_to(sentence)? else {
        return Ok(false);
    };
    if reveal_count.min > 0 {
        reveal_count = ChoiceCount::up_to(reveal_count.max.unwrap_or(reveal_count.min));
    }
    if reveal_count.min != 0 || reveal_count.max != Some(1) || reveal_count.random {
        return Ok(false);
    }
    let player = group.owner;
    let selected_tag = helper_tag_for_tokens(sentence.lowered(), "revealed");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let battlefield_move = EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(selected_tag.clone()),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            it(),
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    let hand_move = EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(selected_tag.clone()),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            it(),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: reveal_count,
        player,
        tag: crate::tag::TagRef::of(selected_tag.clone()),
        zone: Zone::Library,
    }));
    group
        .effects
        .push(EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(selected_tag.clone())));
    group.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: crate::cards::builders::PredicateAst::YourTurn,
        if_true: vec![
            EffectAst::Permissions(PermissionEffectAst::May {
                effects: vec![battlefield_move],
            }),
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::DidNot,
                effects: vec![hand_move.clone()],
            }),
        ],
        if_false: vec![hand_move],
    }));
    group.pending_statements = none_pending(2);
    group.selected = Some(selected_tag.key.clone());
    group.remainder_player = player;
    Ok(true)
}

/// "You may reveal up to two creature cards from among them." followed by
/// "If this spell was bargained, put the revealed cards onto the battlefield.",
/// "Otherwise, put the revealed cards into your hand." and "Shuffle your
/// library.", read together.
pub(super) fn reveal_then_bargain_shape(
    sentence: &SentenceInput,
    rest: &[SentenceInput],
    revealed: bool,
) -> bool {
    let [bargained, otherwise, shuffle, ..] = rest else {
        return false;
    };
    !revealed
        && quad_grammar::parse_may_reveal_looked_card_shape(sentence.lowered()).is_some()
        && quad_grammar::parse_bargained_revealed_battlefield_shape(bargained.lowered())
        && quad_grammar::parse_otherwise_revealed_hand_shape(otherwise.lowered())
        && quad_grammar::parse_then_shuffle_shape(shuffle.lowered())
}

pub(super) fn reveal_then_bargain(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> Result<bool, CardTextError> {
    if !reveal_then_bargain_shape(sentence, rest, group.revealed) {
        return Ok(false);
    }
    let Some((mut filter, reveal_count)) = may_reveal_up_to(sentence)? else {
        return Ok(false);
    };
    let player = group.owner;
    let revealed_tag = helper_tag_for_tokens(sentence.lowered(), "revealed");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: reveal_count,
        player,
        tag: crate::tag::TagRef::of(revealed_tag.clone()),
        zone: Zone::Library,
    }));
    group
        .effects
        .push(EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(revealed_tag.clone())));
    group.effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: crate::cards::builders::PredicateAst::ThisSpellPaidLabel("Bargain".into()),
        if_true: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::TagRef::of(revealed_tag.clone()), None),
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
        if_false: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::TagRef::of(revealed_tag.clone()), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    }));
    group.pending_statements = std::collections::VecDeque::from([
        Vec::new(),
        Vec::new(),
        vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
        )],
    ]);
    group.selected = Some(revealed_tag.key.clone());
    Ok(true)
}

/// "Put one of those cards into your hand and the rest on the bottom of your
/// library in any order." followed by "If this spell was cast from anywhere
/// other than your hand, put each of those cards into your hand instead.",
/// read together as a replacement of the whole procedure.
pub(super) fn nonhand_replacement_shape(
    sentence: &SentenceInput,
    rest: &[SentenceInput],
    revealed: bool,
) -> bool {
    let [replacement, ..] = rest else {
        return false;
    };
    !revealed
        && triple_grammar::is_nonhand_replacement_looked_split_shape(
            trimmed(sentence),
            replacement.lowered(),
        )
}

pub(super) fn nonhand_replacement(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> bool {
    if !nonhand_replacement_shape(sentence, rest, group.revealed) {
        return false;
    }
    let player = group.owner;
    let hand_tag = helper_tag_for_tokens(sentence.lowered(), "hand");
    let mut hand_filter = ObjectFilter::tagged(group.tag.clone());
    hand_filter.zone = Some(Zone::Library);
    let look_effect =
        EffectAst::subject_verb_look_at_top_cards(player, group.count.clone(), crate::tag::TagRef::of(group.tag.clone()));
    let default_effects = vec![
        look_effect.clone(),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            player,
            tag: crate::tag::TagRef::of(hand_tag.clone()),
            zone: Zone::Library,
        }),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::TagRef::of(hand_tag.clone()), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            crate::tag::TagRef::of(group.tag.clone()),
            Some(crate::tag::TagRef::of(hand_tag)),
            crate::cards::builders::LibraryBottomOrderAst::ChooserChooses,
            player,
        ),
    ];
    let replacement_effects = vec![
        look_effect,
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::TagRef::of(group.tag.clone()), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];
    // The replacement spells the view itself, in both branches.
    group.view_style = ViewStyle::Absorbed;
    group.effects = vec![EffectAst::SelfReplacement {
        predicate: crate::cards::builders::PredicateAst::ThisSpellWasCastFromNonHand,
        if_true: replacement_effects,
        if_false: default_effects,
        attach_to_previous_ability: false,
    }];
    group.pending_statements = none_pending(1);
    group.selected = Some(group.tag.clone());
    true
}

/// "Choose any number of artifact and/or land cards revealed this way."
/// followed by "Put all nonland cards chosen this way onto the battlefield,
/// then put all land cards chosen this way onto the battlefield tapped …", read
/// together; the rest goes on the bottom in a random order.
pub(super) fn any_number_revealed_land_split_shape(
    sentence: &SentenceInput,
    rest: &[SentenceInput],
    revealed: bool,
) -> bool {
    let [split, ..] = rest else {
        return false;
    };
    revealed
        && super::super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_any_number_revealed_this_way_choice(sentence.lowered())
            .is_some_and(|(_, filter)| {
                super::super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::looked_choice_filter_can_include_card_type(
                    &filter,
                    crate::types::CardType::Land,
                )
            })
        && triple_grammar::is_land_nonland_split_bottom_shape(split.lowered())
}

pub(super) fn any_number_revealed_land_split(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> bool {
    if !any_number_revealed_land_split_shape(sentence, rest, group.revealed) {
        return false;
    }
    let (choice_count, mut filter) =
        super::super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::parse_any_number_revealed_this_way_choice(sentence.lowered())
            .expect("checked");
    let player = group.owner;
    let chosen_tag = helper_tag_for_tokens(sentence.lowered(), "chosen");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let land_filter = ObjectFilter {
        card_types: vec![crate::types::CardType::Land],
        ..Default::default()
    };
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: choice_count,
        player,
        tag: crate::tag::TagRef::of(chosen_tag.clone()),
        zone: Zone::Library,
    }));
    group.pending_statements = std::collections::VecDeque::from([vec![
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::ItMatches(land_filter),
                if_true: vec![EffectAst::subject_verb_put_onto_battlefield(
                    player,
                    it(),
                    true,
                    ReturnControllerAst::Preserve,
                )],
                if_false: vec![EffectAst::subject_verb_put_onto_battlefield(
                    player,
                    it(),
                    false,
                    ReturnControllerAst::Preserve,
                )],
            })],
        }),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            crate::tag::TagRef::of(group.tag.clone()),
            Some(crate::tag::TagRef::of(chosen_tag.clone())),
            crate::cards::builders::LibraryBottomOrderAst::Random,
            player,
        ),
    ]]);
    group.selected = Some(chosen_tag.key.clone());
    true
}
