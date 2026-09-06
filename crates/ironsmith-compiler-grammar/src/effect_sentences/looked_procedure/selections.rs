//! Selecting from a viewed group, and disposing of the rest.
//!
//! "You may put a creature card from among them into your hand" was spelled by
//! three registry programs in three ways, chosen by what followed: a program
//! for the bottom-of-library remainder, one for the graveyard remainder, and
//! one for the "reveal ... and put it into your hand" verb. The registry ranked
//! them through negated predicates. Here the selection is read once and
//! spelled when the remainder statement says which spelling applies.

use crate::cards::builders::ForEachEffectAst;
use super::super::dispatch_entry::{SentenceInput, leading_may_actor_to_player};
use super::super::sequence_rules::generic_subject_verb_sequences::ordered_control_flow_programs::{
    compose_choose_from_looked_cards_into_hand_rest_into_graveyard,
    compose_choose_from_looked_cards_onto_battlefield_and_into_hand_rest_on_bottom,
    looked_card_choice_filter_branches, parse_cast_from_among_looked_cards_action,
    parse_counted_from_looked_cards_action,
};
use super::{ViewStyle, ViewedGroup, it, remainder_owner};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, ObjectFilter, PlayerAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, Value, LibraryActionAst, StackActionAst, ObjectChoiceEffectAst,
};
use crate::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::grammar::sentence_markers::{self, LeadingMayActor};
use crate::lexer::OwnedLexToken;
use crate::tag::TagKey;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;
use triple_grammar::LookedRemainderShape;

/// "[You may] reveal/put a <filter> card from among them [and put it] into
/// your hand", read but not yet spelled as effects.
pub(super) struct HandSelection {
    chooser: PlayerAst,
    count: ChoiceCount,
    filter: ObjectFilter,
    filter_uses_and_or: bool,
    reveal_chosen: bool,
    tag: TagKey,
}

/// "[You may] put [up to two] <filter> cards from among them onto the
/// battlefield tapped / into your hand", read but not yet spelled.
pub(super) struct PutFromAmong {
    actor: LeadingMayActor,
    tail: Vec<OwnedLexToken>,
    tag: TagKey,
}

/// "[You may] put up to one land card from among them onto the battlefield
/// tapped and up to one Elf card from among them into your hand": two
/// selections in one sentence, spelled with the remainder that follows.
pub(super) struct BothSelection {
    chooser: PlayerAst,
    battlefield_filter: ObjectFilter,
    hand_filter: ObjectFilter,
    tapped: bool,
    tokens: Vec<OwnedLexToken>,
}

/// A selection whose spelling depends on the remainder statement that follows.
pub(super) struct PendingSelection {
    hand: Option<HandSelection>,
    put: Option<PutFromAmong>,
    both: Option<BothSelection>,
}

pub(super) enum Selection {
    /// Spelled now: a put-from-among whose destination is not the hand.
    Immediate(PutFromAmong),
    /// Spelled with the remainder.
    Deferred(PendingSelection),
}

/// `Ok(None)` when the sentence is not a hand selection; `Err` when it is one
/// whose card filter the grammar does not understand. A recognized statement
/// with an unreadable filter is a committed failure, not a sentence for the
/// general grammar to guess at.
fn hand_selection(
    sentence: &SentenceInput,
    owner: PlayerAst,
) -> Result<Option<HandSelection>, CardTextError> {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let Some(action) =
        sentence_markers::parse_leading_may_action_tokens(tokens, &["reveal", "put"], true)
    else {
        return Ok(None);
    };
    let reveal_chosen = action.verb == "reveal";
    let action_tokens = crate::lexer::trim_lexed_commas(action.tail_tokens);
    let Some(shape) = triple_grammar::parse_looked_hand_action_shape(action_tokens, reveal_chosen)
    else {
        return Ok(None);
    };
    let mut count = shape.count;
    if action.actor != LeadingMayActor::Default && count.min > 0 {
        count = ChoiceCount::up_to(count.max.unwrap_or(count.min));
    }
    let filter_tokens = crate::lexer::trim_lexed_commas(&action_tokens[shape.filter.clone()]);
    let Some(mut filter) =
        super::super::looked_cards_family::parse_looked_card_choice_filter(filter_tokens)
    else {
        return Err(CardTextError::ParseError(format!(
            "unable to parse looked-card filter (clause: '{}')",
            crate::lexer::token_word_refs(filter_tokens).join(" ")
        )));
    };
    filter.zone = Some(Zone::Library);
    Ok(Some(HandSelection {
        chooser: leading_may_actor_to_player(action.actor, owner),
        count,
        filter,
        filter_uses_and_or: shape.filter_uses_and_or,
        reveal_chosen,
        tag: (helper_tag_for_tokens(
            sentence.lowered(),
            if reveal_chosen { "revealed" } else { "chosen" },
        )).into(),
    }))
}

fn put_from_among(sentence: &SentenceInput) -> Option<(PutFromAmong, Zone)> {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let action = sentence_markers::parse_leading_may_action_tokens(tokens, &["put"], true)?;
    let (_, _, _, zone, ..) = parse_counted_from_looked_cards_action(action.tail_tokens)?;
    Some((
        PutFromAmong {
            actor: action.actor,
            tail: action.tail_tokens.to_vec(),
            tag: (helper_tag_for_tokens(sentence.lowered(), "chosen")).into(),
        },
        zone,
    ))
}

/// A selection statement, if the sentence is one. A sentence that also
/// disposes of the rest is the same-sentence statement, or nothing here: a
/// plain selection must not drop a remainder it did not read.
pub(super) fn selection_shape(
    sentence: &SentenceInput,
    owner: PlayerAst,
) -> Result<Option<Selection>, CardTextError> {
    if sentence.lowered().iter().any(|token| token.is_word("rest")) {
        return Ok(None);
    }
    if let Some((chooser, battlefield_filter, tapped, hand_filter)) =
        super::super::looked_cards_family::parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
            sentence.lowered(),
        )?
    {
        return Ok(Some(Selection::Deferred(PendingSelection {
            hand: None,
            put: None,
            both: Some(BothSelection {
                chooser,
                battlefield_filter,
                hand_filter,
                tapped,
                tokens: sentence.lowered().to_vec(),
            }),
        })));
    }
    let put = put_from_among(sentence);
    let hand = match hand_selection(sentence, owner) {
        Ok(hand) => hand,
        // The put-from-among reading of the same sentence stands on its own.
        Err(error) if put.is_none() => return Err(error),
        Err(_) => None,
    };
    Ok(match (put, hand) {
        (Some((put, Zone::Hand)), hand) => Some(Selection::Deferred(PendingSelection {
            hand,
            put: Some(put),
            both: None,
        })),
        (Some((put, _)), _) => Some(Selection::Immediate(put)),
        (None, Some(hand)) => Some(Selection::Deferred(PendingSelection {
            hand: Some(hand),
            put: None,
            both: None,
        })),
        (None, None) => None,
    })
}

/// "[You may] cast a <filter> card from among them without paying its mana
/// cost": the chooser and the filter.
pub(super) fn cast_from_among_shape(
    sentence: &SentenceInput,
    owner: PlayerAst,
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    parse_cast_from_among_looked_cards_action(sentence.lowered(), owner)
}

/// The cast-from-among statement: a chosen card cast without paying its mana
/// cost; the chooser disposes of the rest. A reveal view shows the group as a
/// look followed by revealing it, as the program this replaces spelled it.
pub(super) fn cast_from_among(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    let Some((chooser, mut filter)) = cast_from_among_shape(sentence, group.owner)? else {
        return Ok(false);
    };
    let chosen_tag = helper_tag_for_tokens(sentence.lowered(), "chosen_cast");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: group.tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    if group.revealed {
        group.view_style = ViewStyle::LookThenRevealTagged;
    }
    group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: ChoiceCount::up_to(1),
        player: chooser,
        tag: crate::tag::TagRef::of(chosen_tag.clone()),
        zone: Zone::Library,
    }));
    group.effects.push(EffectAst::SubjectVerb(
        crate::cards::builders::SubjectVerbEffectAst {
            subject: crate::model::ast::SubjectVerbSubjectAst {
                role: SubjectVerbRoleAst::Actor,
                player: chooser,
            },
            action: SubjectVerbActionAst::Stack(StackActionAst::CastTagged {
                tag: crate::tag::TagRef::of(chosen_tag.clone()),
                player: chooser,
                allow_land: false,
                as_copy: false,
                copy_cast_reminder_surface: false,
                copy_instruction_surface: None,
                without_paying_mana_cost: true,
                additional_mana_cost: None,
                cost_reduction: None,
                mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
            }),
        },
    ));
    group.selected = Some(chosen_tag.key.clone());
    group.remainder_player = chooser;
    Ok(true)
}

/// "[You may] put a land card from among them onto the battlefield and the
/// rest on the bottom of your library in a random order": a counted selection
/// with its remainder in the same sentence.
pub(super) fn same_sentence_shape(
    sentence: &SentenceInput,
) -> Option<(
    sentence_markers::LeadingMayActionMatch<'_>,
    LookedRemainderShape,
)> {
    let tokens = crate::lexer::trim_lexed_commas(sentence.lowered());
    let action = sentence_markers::parse_leading_may_action_tokens(tokens, &["put"], true)?;
    let remainder = triple_grammar::parse_looked_remainder_shape(tokens)?;
    parse_counted_from_looked_cards_action(action.tail_tokens)?;
    Some((action, remainder))
}

/// Record a selection statement. Returns false when the sentence turned out
/// not to be one after all.
pub(super) fn select(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    selection: Selection,
) -> bool {
    let _ = sentence;
    match selection {
        Selection::Immediate(put) => spell_put_from_among(group, &put, false),
        Selection::Deferred(pending) => {
            if let Some(hand) = &pending.hand {
                group.selected = Some(hand.tag.clone());
                group.remainder_player = hand.chooser;
            }
            if let Some(put) = &pending.put {
                group.selected = Some(put.tag.clone());
                group.remainder_player = remainder_owner(group.owner);
            }
            if let Some(both) = &pending.both {
                group.selected = Some(helper_tag_for_tokens(&both.tokens, "kept").into());
                if group.revealed {
                    group.view_style = ViewStyle::LookThenRevealTagged;
                }
            }
            if group.revealed && pending.hand.is_some() {
                group.view_style = ViewStyle::LookThenRevealTagged;
            }
            group.pending = Some(pending);
            true
        }
    }
}

/// The put-from-among spelling: a tagged choice (or every matching card),
/// each chosen card moved, with entry counters when the sentence names them.
/// Returns false when the actor cannot take all matching cards.
fn spell_put_from_among(
    group: &mut ViewedGroup,
    put: &PutFromAmong,
    graveyard_follows: bool,
) -> bool {
    let chooser = leading_may_actor_to_player(put.actor, group.owner);
    let Some((
        mut choice_count,
        filter,
        aggregate_constraint,
        zone,
        controller,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = parse_counted_from_looked_cards_action(&put.tail)
    else {
        return false;
    };
    if all_matching && put.actor != LeadingMayActor::Default {
        return false;
    }
    if put.actor != LeadingMayActor::Default && choice_count == ChoiceCount::exactly(1) {
        choice_count = ChoiceCount::up_to(1);
    }
    let _ = graveyard_follows;
    let chosen_tag = put.tag.clone();
    let mut choose_filter = filter;
    choose_filter.zone = Some(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: group.tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    if all_matching {
        choose_filter.zone = None;
        group
            .effects
            .push(EffectAst::subject_verb_tag_matching_objects(
                choose_filter,
                vec![Zone::Library],
                crate::tag::TagRef::of(chosen_tag.clone()),
            ));
    } else {
        group
            .effects
            .push(if let Some(constraint) = aggregate_constraint {
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
                    filter: choose_filter,
                    count: choice_count,
                    player: chooser,
                    tag: crate::tag::TagRef::of(chosen_tag.clone()),
                    constraint,
                })
            } else {
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                    filter: choose_filter,
                    count: choice_count,
                    player: chooser,
                    tag: crate::tag::TagRef::of(chosen_tag.clone()),
                    zone: Zone::Library,
                })
            });
    }
    let mut chosen_effects = vec![EffectAst::subject_verb_move_to_zone_with_attack_target(
        it(),
        zone,
        false,
        controller,
        tapped,
        attacking,
        attack_target_player,
        false,
        None,
    )];
    if let Some((amount, counter_type)) = triple_grammar::parse_looked_move_action_shape(&put.tail)
        .and_then(|shape| shape.entry_counter)
    {
        chosen_effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(amount as i32),
            it(),
            None,
            false,
        ));
    }
    group.effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(chosen_tag.clone()),
        effects: chosen_effects,
    }));
    group.selected = Some(chosen_tag);
    group.remainder_player = remainder_owner(group.owner);
    true
}

/// Spell a pending selection the way the remainder that follows it needs: the
/// graveyard remainder takes the hand-selection program's spelling, which
/// folds the remainder into a per-card split; the bottom remainder takes the
/// put-from-among spelling for "put" and the reveal spelling for "reveal".
/// Returns true when the remainder itself was spelled here.
pub(super) fn spell_pending(
    group: &mut ViewedGroup,
    remainder: Option<&LookedRemainderShape>,
) -> bool {
    let Some(pending) = group.pending.take() else {
        return false;
    };
    if let Some(both) = pending.both {
        let order = match remainder {
            Some(LookedRemainderShape::LibraryBottom(order)) => *order,
            _ => crate::cards::builders::LibraryBottomOrderAst::Random,
        };
        group.effects.extend(
            compose_choose_from_looked_cards_onto_battlefield_and_into_hand_rest_on_bottom(
                &both.tokens,
                group.tag.clone(),
                both.chooser,
                both.battlefield_filter,
                both.hand_filter,
                both.tapped,
                order,
            ),
        );
        return matches!(remainder, Some(LookedRemainderShape::LibraryBottom(_)));
    }
    match (remainder, pending.hand, pending.put) {
        (Some(LookedRemainderShape::Graveyard), Some(hand), _) => {
            spell_hand_selection_into_graveyard(group, hand);
            true
        }
        (_, Some(hand), None) => {
            spell_hand_selection(group, hand);
            false
        }
        (_, _, Some(put)) => {
            spell_put_from_among(group, &put, false);
            false
        }
        (_, None, None) => false,
    }
}

fn spell_hand_selection(group: &mut ViewedGroup, selection: HandSelection) {
    let HandSelection {
        chooser,
        count,
        mut filter,
        filter_uses_and_or,
        reveal_chosen,
        tag: chosen_tag,
    } = selection;
    let looked_tag = group.tag.clone();
    if count == ChoiceCount::up_to(1)
        && filter_uses_and_or
        && let Some(choice_filters) = looked_card_choice_filter_branches(&filter)
    {
        for mut choice_filter in choice_filters {
            choice_filter.zone = Some(Zone::Library);
            choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: looked_tag.clone(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: chosen_tag.clone(),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                });
            group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: choice_filter,
                count: ChoiceCount::up_to(1),
                player: chooser,
                tag: crate::tag::TagRef::of(chosen_tag.clone()),
                zone: Zone::Library,
            }));
        }
    } else {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: looked_tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            player: chooser,
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
            zone: Zone::Library,
        }));
    }
    if reveal_chosen {
        group
            .effects
            .push(EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(chosen_tag.clone())));
    }
    group.effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(chosen_tag),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            it(),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    }));
}

fn spell_hand_selection_into_graveyard(group: &mut ViewedGroup, selection: HandSelection) {
    let HandSelection {
        chooser,
        count,
        filter,
        filter_uses_and_or,
        reveal_chosen,
        tag: chosen_tag,
    } = selection;
    let looked_tag = group.tag.clone();
    if count == ChoiceCount::up_to(1)
        && filter.card_types.len() > 1
        && filter_uses_and_or
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.static_abilities.is_empty()
        && filter.any_of.is_empty()
    {
        for card_type in &filter.card_types {
            let mut choice_filter = filter.clone();
            choice_filter.card_types = vec![*card_type];
            choice_filter.zone = Some(Zone::Library);
            choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: looked_tag.clone(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: chosen_tag.clone(),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                });
            group.effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: choice_filter,
                count: ChoiceCount::up_to(1),
                player: chooser,
                tag: crate::tag::TagRef::of(chosen_tag.clone()),
                zone: Zone::Library,
            }));
        }
        group.effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                it(),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        }));
        group.effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
                tag: crate::tag::TagRef::of(looked_tag),
                keep_tagged: crate::tag::TagRef::of(chosen_tag),
                zone: Zone::Graveyard,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            }),
        ));
    } else {
        group.effects.extend(
            compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
                chooser,
                filter,
                looked_tag,
                chosen_tag,
                Zone::Library,
                reveal_chosen,
                Vec::new(),
                count,
            ),
        );
    }
}

/// The remainder statement: spell what waits on it, then the remainder itself
/// unless that spelling already included it.
pub(super) fn spell_remainder(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    remainder: LookedRemainderShape,
) {
    if spell_pending(group, Some(&remainder)) {
        return;
    }
    dispose_remainder(group, sentence, remainder);
}

/// A counted selection whose remainder shares its sentence. Returns false when
/// the shape's actor cannot take all matching cards.
pub(super) fn select_with_remainder(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    action: sentence_markers::LeadingMayActionMatch<'_>,
    remainder: LookedRemainderShape,
) -> bool {
    let chooser = leading_may_actor_to_player(action.actor, group.owner);
    let Some((
        mut choice_count,
        filter,
        aggregate_constraint,
        zone,
        controller,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = parse_counted_from_looked_cards_action(action.tail_tokens)
    else {
        return false;
    };
    if all_matching && action.actor != LeadingMayActor::Default {
        return false;
    }
    if action.actor != LeadingMayActor::Default && choice_count == ChoiceCount::exactly(1) {
        choice_count = ChoiceCount::up_to(1);
    }
    if group.revealed {
        group.view_style = ViewStyle::LookThenRevealTagged;
    }
    let chosen_tag = helper_tag_for_tokens(sentence.lowered(), "chosen");
    let mut choose_filter = filter;
    choose_filter.zone = Some(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: group.tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    if all_matching {
        choose_filter.zone = None;
        group
            .effects
            .push(EffectAst::subject_verb_tag_matching_objects(
                choose_filter,
                vec![Zone::Library],
                crate::tag::TagRef::of(chosen_tag.clone()),
            ));
    } else {
        group
            .effects
            .push(if let Some(constraint) = aggregate_constraint {
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint {
                    filter: choose_filter,
                    count: choice_count,
                    player: chooser,
                    tag: crate::tag::TagRef::of(chosen_tag.clone()),
                    constraint,
                })
            } else {
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                    filter: choose_filter,
                    count: choice_count,
                    player: chooser,
                    tag: crate::tag::TagRef::of(chosen_tag.clone()),
                    zone: Zone::Library,
                })
            });
    }
    group.effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(chosen_tag.clone()),
        effects: vec![EffectAst::subject_verb_move_to_zone_with_attack_target(
            it(),
            zone,
            false,
            controller,
            tapped,
            attacking,
            attack_target_player,
            false,
            None,
        )],
    }));
    match remainder {
        LookedRemainderShape::LibraryBottom(order) => group.effects.push(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                crate::tag::TagRef::of(group.tag.clone()),
                Some(crate::tag::TagRef::of(chosen_tag.clone())),
                order,
                chooser,
            ),
        ),
        LookedRemainderShape::Graveyard => group.effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
                tag: crate::tag::TagRef::of(group.tag.clone()),
                keep_tagged: crate::tag::TagRef::of(chosen_tag.clone()),
                zone: Zone::Graveyard,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            }),
        )),
    }
    group.selected = Some(chosen_tag.key.clone());
    true
}

/// "Put the rest on the bottom of your library in a random order" / "Put the
/// rest into your graveyard": what was not selected.
pub(super) fn dispose_remainder(
    group: &mut ViewedGroup,
    sentence: &SentenceInput,
    remainder: LookedRemainderShape,
) {
    // The authored wording of the remainder: an explicit complement ("all
    // cards revealed this way that weren't put onto the battlefield"), or
    // "the rest", spelled with its sentence-leading "then" when it has one.
    let surface = match triple_grammar::looked_remainder_surface(sentence.lexed()) {
        ironsmith_core::LibraryRemainderSurface::Rest
            if crate::lexer::parser_token_word_refs(sentence.lexed())
                .first()
                .is_some_and(|word| *word == "then") =>
        {
            ironsmith_core::LibraryRemainderSurface::SentenceLeadingThenRest
        }
        surface => surface,
    };
    let keep = group.selected.clone();
    match remainder {
        LookedRemainderShape::LibraryBottom(order) => group.effects.push(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library_with_surface(
                crate::tag::TagRef::of(group.tag.clone()),
                keep.map(crate::tag::TagRef::of),
                order,
                group.remainder_player,
                surface,
            ),
        ),
        LookedRemainderShape::Graveyard => group.effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
                tag: crate::tag::TagRef::of(group.tag.clone()),
                keep_tagged: crate::tag::TagRef::of(keep.unwrap_or_else(|| group.tag.clone())),
                zone: Zone::Graveyard,
                surface,
            }),
        )),
    }
}
