//! Readers 1 of 2 of the registry in the parent module.

use crate::cards::builders::ObjectChoiceEffectAst;
use super::*;

pub(super) fn read_revealed_remainder(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    if let Some(shape) = cca_shapes::parse_revealed_remainder_shape(tokens) {
        let order = if shape.random_order {
            crate::cards::builders::LibraryBottomOrderAst::Random
        } else {
            crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
        };
        return Ok(Some(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library_with_surface(
                crate::tag::CompilerReferenceTag::LastRevealed.bind(),
                shape
                    .exclude_current_reference
                    .then(|| crate::tag::CompilerReferenceTag::It.bind()),
                order,
                cca_shapes::parse_destination_player(tokens).unwrap_or(player),
                shape.surface,
            ),
        ));
    }
    Ok(None)
}
pub(super) fn read_reorder_tagged_cards(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    // "Put them/it back in any order." (typically after looking at the top cards of a library).
    if cca_shapes::is_reorder_tagged_cards(tokens) {
        let mut reorder = EffectAst::subject_verb_reorder_top_of_library(
            crate::tag::CompilerReferenceTag::It.bind(),
        );
        // "That player ..., then puts them back in any order": the authored
        // (or carried) subject orders the cards, not the ability's controller.
        if !matches!(input.player, PlayerAst::You | PlayerAst::Implicit | PlayerAst::Any)
            && let EffectAst::SubjectVerb(subject_verb) = &mut reorder
        {
            subject_verb.subject.player = input.player;
        }
        return Ok(Some(reorder));
    }
    Ok(None)
}
pub(super) fn read_tagged_battlefield_partition(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let clause_words = input.clause_words;
    if let Some(shape) = cca_shapes::parse_tagged_battlefield_partition_shape(tokens) {
        let collection_tag = crate::util::helper_tag_for_tokens(tokens, "partition_pool");
        let chosen_tag = crate::util::helper_tag_for_tokens(tokens, "partition_chosen");
        let owner = crate::activation_and_restrictions::controller_filter_for_token_player(player)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "battlefield collection partition has no resolvable player (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;

        let mut collection_filter =
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
        collection_filter.zone = Some(Zone::Library);
        collection_filter.owner = Some(owner.clone());
        let capture_collection = EffectAst::subject_verb_tag_matching_objects(
            collection_filter,
            vec![Zone::Library],
            crate::tag::TagRef::of(collection_tag.clone()),
        );

        let mut choose_filter = ObjectFilter::tagged(collection_tag.clone());
        choose_filter.zone = Some(Zone::Library);
        choose_filter.owner = Some(owner.clone());
        let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: choose_filter,
            count: shape.count,
            player,
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
            zone: Zone::Library,
        });

        let mut chosen_filter = ObjectFilter::tagged(chosen_tag.clone());
        chosen_filter.zone = Some(Zone::Library);
        chosen_filter.owner = Some(owner.clone());
        let chosen_controller = match shape.chosen_controller {
            cca_shapes::PartitionBattlefieldControllerShape::You => ReturnControllerAst::You,
            cca_shapes::PartitionBattlefieldControllerShape::SubjectPlayer => {
                ReturnControllerAst::Owner
            }
        };
        let move_chosen = EffectAst::subject_verb_put_all_onto_battlefield(
            chosen_filter,
            shape.chosen_tapped,
            false,
            chosen_controller,
        );

        let mut remainder_filter = ObjectFilter::tagged(collection_tag);
        remainder_filter.zone = Some(Zone::Library);
        remainder_filter.owner = Some(owner);
        remainder_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: chosen_tag.key.clone(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
        let remainder_controller = match shape.remainder_controller {
            cca_shapes::PartitionBattlefieldControllerShape::You => ReturnControllerAst::You,
            cca_shapes::PartitionBattlefieldControllerShape::SubjectPlayer => {
                ReturnControllerAst::Owner
            }
        };
        if shape.remainder_in_hand {
            let remainder_tag = crate::util::helper_tag_for_tokens(tokens, "partition_remainder");
            let capture_remainder = EffectAst::subject_verb_tag_matching_objects(
                remainder_filter, vec![Zone::Library], crate::tag::TagRef::of(remainder_tag.clone()));
            let move_remainder = EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::TagRef::of(remainder_tag), None), Zone::Hand,
                false, ReturnControllerAst::Preserve, false, None)
                .with_destination_player_surface(Some(player));
            return Ok(Some(EffectAst::Sequence { effects: vec![
                capture_collection, choose, capture_remainder, move_chosen, move_remainder,
            ] }));
        }
        let move_remainder = EffectAst::subject_verb_put_all_onto_battlefield(
            remainder_filter,
            shape.remainder_tapped,
            false,
            remainder_controller,
        );

        return Ok(Some(EffectAst::Sequence {
            effects: vec![capture_collection, choose, move_chosen, move_remainder],
        }));
    }
    Ok(None)
}
pub(super) fn read_from_among_them(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let clause_words = input.clause_words;
    let from_among_shape = cca_shapes::parse_from_among_them_shape(tokens);
    if let Some(shape) = from_among_shape
        && shape.destination == cca_shapes::FromAmongDestinationShape::Battlefield
    {
        if cca_shapes::has_relative_collection_entry_controller(tokens) {
            return Err(CardTextError::ParseError(
                "contextual-controller collection entry requires an actor-owned selection program".into(),
            ));
        }
        let looked_tag = crate::util::helper_tag_for_tokens(tokens, "looked");
        let chosen_tag = crate::util::helper_tag_for_tokens(tokens, "chosen");
        // "Put all land cards from among them onto the battlefield and the
        // rest into your hand" (Nissa, Nature's Artisan): every matching
        // card moves, with no choice.
        if shape.filter_tokens.first().is_some_and(|token| token.is_word("all"))
            && shape.filter_tokens.len() > 1
            && shape.rest_destination != Some(cca_shapes::RestDestinationShape::BottomOfLibrary)
            && let Some(filter) =
                crate::effect_sentences::parse_looked_card_choice_filter(&shape.filter_tokens[1..])
        {
            let mut effects = compose_put_all_filtered_looked_cards_to_zone(
                filter,
                looked_tag.clone().into(),
                chosen_tag.clone().into(),
                Zone::Battlefield,
            );
            let rest_zone = match shape.rest_destination {
                Some(cca_shapes::RestDestinationShape::Hand) => Some(Zone::Hand),
                Some(cca_shapes::RestDestinationShape::Graveyard) => Some(Zone::Graveyard),
                Some(cca_shapes::RestDestinationShape::BottomOfLibrary) | None => None,
            };
            if let Some(zone) = rest_zone {
                effects.push(EffectAst::subject_verb(
                    SubjectVerbRoleAst::Actor,
                    PlayerAst::Implicit,
                    SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
                        tag: crate::tag::TagRef::of(looked_tag),
                        keep_tagged: crate::tag::TagRef::of(chosen_tag),
                        zone,
                        surface: ironsmith_core::LibraryRemainderSurface::Rest,
                    }),
                ));
            }
            return Ok(Some(EffectAst::Sequence { effects }));
        }
        let filter = crate::effect_sentences::parse_looked_card_choice_filter(shape.filter_tokens)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to parse from-among hand filter (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let effects = if shape.rest_destination == Some(cca_shapes::RestDestinationShape::Hand) {
            compose_put_filtered_looked_cards_to_zone_rest_to_zone(
                player,
                filter,
                shape.count,
                (looked_tag).into(),
                (chosen_tag).into(),
                Zone::Battlefield,
                Zone::Hand,
            )
        } else {
            compose_put_filtered_looked_cards_to_zone(
                player,
                filter,
                shape.count,
                (looked_tag).into(),
                (chosen_tag).into(),
                Zone::Battlefield,
            )
        };
        return Ok(Some(EffectAst::Sequence { effects }));
    }
    Ok(None)
}
pub(super) fn read_from_among_hand_surface(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    let clause_words = input.clause_words;
    let from_among_shape = cca_shapes::parse_from_among_them_shape(tokens);
    if cca_shapes::has_from_among_hand_surface(tokens) {
        let looked_tag = crate::util::helper_tag_for_tokens(tokens, "looked");
        let chosen_tag = crate::util::helper_tag_for_tokens(tokens, "chosen");
        if let Some(shape) = from_among_shape {
            // "put all <filter> from among them into your hand": every
            // matching card, no choice. Without an authored "and the rest
            // into your graveyard", the rest stay for a following "then put
            // the rest ..." instruction.
            if shape.filter_tokens.first().is_some_and(|token| token.is_word("all"))
                && shape.filter_tokens.len() > 1
                && shape.rest_destination.is_none()
            {
                let filter = crate::effect_sentences::parse_looked_card_choice_filter(
                    &shape.filter_tokens[1..],
                )
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unable to parse from-among hand filter (clause: '{}')",
                        clause_words.join(" ")
                    ))
                })?;
                let mut effects = compose_put_all_filtered_looked_cards_to_zone(
                    filter,
                    (looked_tag.clone()).into(),
                    (chosen_tag.clone()).into(),
                    Zone::Hand,
                );
                // "..., then put the rest on the bottom of your library in a
                // random order" (Depala) travels in the same clause; the
                // remainder is every looked card that wasn't moved.
                if crate::word_primitives::sequence_occurs(
                    clause_words,
                    &["then", "put", "the", "rest", "on", "the", "bottom"],
                ) {
                    let order = if clause_words.iter().any(|word| *word == "random") {
                        crate::cards::builders::LibraryBottomOrderAst::Random
                    } else {
                        crate::cards::builders::LibraryBottomOrderAst::ChooserChooses
                    };
                    effects.push(
                        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                            crate::tag::TagRef::of(looked_tag),
                            Some(crate::tag::TagRef::of(chosen_tag)),
                            order,
                            player,
                        ),
                    );
                }
                return Ok(Some(EffectAst::Sequence { effects }));
            }
            // "Put all cards with the chosen name from among them into your
            // hand and the rest into your graveyard." (Tamiyo, Collector of
            // Tales): every matching card moves, no choice of "up to one".
            if shape.filter_tokens.first().is_some_and(|token| token.is_word("all"))
                && shape.filter_tokens.len() > 1
                && shape.rest_destination == Some(cca_shapes::RestDestinationShape::Graveyard)
                && let Some(filter) = crate::effect_sentences::parse_looked_card_choice_filter(
                    &shape.filter_tokens[1..],
                )
            {
                let effects = compose_put_all_filtered_looked_cards_into_hand_rest_into_graveyard(
                    filter,
                    (looked_tag).into(),
                    (chosen_tag).into(),
                );
                return Ok(Some(EffectAst::Sequence { effects }));
            }
            let filter =
                crate::effect_sentences::parse_looked_card_choice_filter(shape.filter_tokens)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unable to parse from-among hand filter (clause: '{}')",
                            clause_words.join(" ")
                        ))
                    })?;
            return Ok(Some(EffectAst::Sequence {
                effects: compose_put_filtered_looked_cards_into_hand_rest_into_graveyard(
                    player,
                    filter,
                    shape.count,
                    (looked_tag).into(),
                    (chosen_tag).into(),
                ),
            }));
        }
        return Ok(Some(EffectAst::Sequence {
            effects: EffectAst::compose_put_some_into_hand_rest_into_graveyard(
                player,
                crate::effect::ChoiceCount::exactly(1),
                crate::tag::TagRef::of(looked_tag),
                crate::tag::TagRef::of(chosen_tag),
            ),
        }));
    }
    Ok(None)
}
pub(super) fn read_all_exiled_into_hand_filter(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let exiled_with_source_surface = input.exiled_with_source_surface.clone();
    if let Some(filter_tokens) = cca_shapes::parse_all_exiled_into_hand_filter(tokens) {
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(Some(wrap_return_with_delayed_timing(
            EffectAst::subject_verb_return_all_to_hand(filter)
                .with_exiled_with_source_surface(exiled_with_source_surface.clone()),
            parse_put_into_hand_delayed_timing(tokens),
        )));
    }
    Ok(None)
}
pub(super) fn read_tagged_on_top_library(
    input: &PutClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let player = input.player;
    // "Put one of those cards on top of your library and the rest on the bottom of your library"
    if let Some(shape) = cca_shapes::parse_tagged_on_top_library_shape(tokens) {
        let library_owner = cca_shapes::parse_destination_player(tokens).unwrap_or(player);
        let looked_tag = crate::util::helper_tag_for_tokens(tokens, "looked");
        let chosen_tag = crate::util::helper_tag_for_tokens(tokens, "chosen");

        return Ok(Some(EffectAst::Sequence {
            effects: EffectAst::compose_put_some_on_top_rest_on_bottom_of_library(
                library_owner,
                shape.count,
                crate::tag::TagRef::of(looked_tag),
                crate::tag::TagRef::of(chosen_tag),
                shape.bottom_order,
            ),
        }));
    }
    Ok(None)
}
