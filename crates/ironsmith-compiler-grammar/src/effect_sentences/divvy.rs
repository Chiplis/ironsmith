use crate::cards::builders::ForEachEffectAst;
use super::super::front_end::grammar::effects::divvy_shapes::{
    self, DivvyChooserShape, DivvyRestDestinationShape, DivvySequenceShape,
};
use super::super::lexer::{OwnedLexToken, split_lexed_sentences};
use super::dispatch_entry::SentenceInput;
use super::dispatch_inner::parse_effect_sentence_lexed;
use crate::cards::builders::{
    CardTextError, EffectAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbRoleAst, TagKey, TargetAst, LibraryActionAst, ObjectChoiceEffectAst, ConditionalEffectAst,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::zone::Zone;

fn membership_predicate_for_iterated_object(tag: crate::tag::CompilerReferenceTag) -> PredicateAst {
    PredicateAst::TaggedMatches(
        tag.bind(),
        ObjectFilter::default()
            .same_stable_id_as_tagged(crate::tag::CompilerReferenceTag::It.bind()),
    )
}

fn parse_effect_sentence_sequence(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let effects = parse_effect_sentence_lexed(tokens)?;
    if effects.is_empty() {
        Err(CardTextError::ParseError(
            "missing effect sentence".to_string(),
        ))
    } else {
        Ok(effects)
    }
}

pub(super) fn try_parse_divvy_sentence_sequence(
    sentences: &[SentenceInput],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_tokens = sentences
        .iter()
        .map(SentenceInput::lexed)
        .collect::<Vec<_>>();
    let Some(shape) = divvy_shapes::parse_divvy_sequence_shape(&sentence_tokens) else {
        return Ok(None);
    };

    if shape == DivvySequenceShape::SearchLibraryGraveyardExileRemainderToTop {
        let chosen_tag = crate::tag::CompilerReferenceTag::MultiZoneSearchChosen.bind();
        let mut search_filter = ObjectFilter::default();
        search_filter.owner = Some(PlayerFilter::You);

        let mut remainder_filter = search_filter.clone();
        remainder_filter.any_of = vec![
            ObjectFilter {
                zone: Some(Zone::Library),
                ..ObjectFilter::default()
            },
            ObjectFilter {
                zone: Some(Zone::Graveyard),
                ..ObjectFilter::default()
            },
        ];
        remainder_filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: chosen_tag.clone().into(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });

        let mut effects = vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                filter: search_filter,
                count: ChoiceCount::exactly(5),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
                zones: vec![Zone::Library, Zone::Graveyard],
                search_mode: Some(crate::effect::SearchSelectionMode::Exact),
            }),
            EffectAst::subject_verb_exile_all(remainder_filter, false),
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(chosen_tag, None),
                Zone::Library,
                true,
                ReturnControllerAst::Preserve,
                false,
                None,
            )
            .with_library_order(
                Some(crate::cards::builders::LibraryBottomOrderAst::ChooserChooses),
                PlayerAst::You,
            ),
        ];
        effects.extend(parse_effect_sentence_lexed(sentences[2].lowered())?);
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::SearchFourCreatureCards {
        let first_effect_tokens = split_lexed_sentences(sentences[0].lowered())
            .into_iter()
            .next()
            .unwrap_or_else(|| sentences[0].lowered());
        let mut effects = parse_effect_sentence_sequence(first_effect_tokens)?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                vec![Zone::Library, Zone::Graveyard],
                crate::tag::CompilerReferenceTag::DivvySource.bind(),
            ),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(crate::tag::CompilerReferenceTag::DivvySource.bind()),
                count: ChoiceCount::exactly(2),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
                zones: vec![Zone::Library, Zone::Graveyard],
                search_mode: None,
            }),
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::CompilerReferenceTag::DivvySource.bind(),
                effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object(
                        crate::tag::CompilerReferenceTag::DivvyChosen,
                    ),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                })],
            }),
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Library,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::You,
                SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
            ),
            EffectAst::subject_verb_exile(TargetAst::Source(None), false),
        ]);
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::ExchangeCreatureControl {
        let first_player_tag = crate::tag::CompilerReferenceTag::ExchangePlayerOne.bind();
        let second_player_tag = crate::tag::CompilerReferenceTag::ExchangePlayerTwo.bind();
        let first_creatures_tag = crate::tag::CompilerReferenceTag::ExchangeCreaturesOne.bind();
        let second_creatures_tag = crate::tag::CompilerReferenceTag::ExchangeCreaturesTwo.bind();

        return Ok(Some(vec![
            EffectAst::subject_verb_target_only(TargetAst::WithCount(
                Box::new(TargetAst::Player(PlayerFilter::Any, None)),
                ChoiceCount::exactly(2),
            )),
            EffectAst::subject_verb_choose_player(
                PlayerAst::You,
                PlayerFilter::target_player(),
                first_player_tag.clone(),
                false,
                0,
            ),
            EffectAst::subject_verb_choose_player(
                PlayerAst::You,
                PlayerFilter::target_player(),
                second_player_tag.clone(),
                false,
                1,
            ),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: ObjectFilter::creature()
                    .controlled_by(PlayerFilter::TaggedPlayer(first_player_tag.clone().into())),
                count: ChoiceCount::up_to_dynamic_x(),
                count_value: Some(Value::Count(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::TaggedPlayer(second_player_tag.clone().into())),
                )),
                player: PlayerAst::You,
                tag: first_creatures_tag.clone(),
            }),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: ObjectFilter::creature()
                    .controlled_by(PlayerFilter::TaggedPlayer(second_player_tag.clone().into())),
                count: ChoiceCount::dynamic_x(),
                count_value: Some(Value::Count(ObjectFilter::tagged(
                    first_creatures_tag.clone(),
                ))),
                player: PlayerAst::You,
                tag: second_creatures_tag.clone(),
            }),
            EffectAst::ForEach(ForEachEffectAst::ForEachTaggedPlayer {
                tag: second_player_tag,
                effects: vec![EffectAst::subject_verb_gain_control(
                    PlayerAst::That,
                    TargetAst::Tagged(first_creatures_tag, None),
                    Until::Forever,
                )],
            }),
            EffectAst::ForEach(ForEachEffectAst::ForEachTaggedPlayer {
                tag: first_player_tag,
                effects: vec![EffectAst::subject_verb_gain_control(
                    PlayerAst::That,
                    TargetAst::Tagged(second_creatures_tag, None),
                    Until::Forever,
                )],
            }),
        ]));
    }

    if shape == DivvySequenceShape::DestroyChosenCreaturePile {
        return Ok(Some(vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::target_player()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Target,
                tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
            }),
            EffectAst::subject_verb_destroy_no_regeneration(TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
                None,
            )),
        ]));
    }

    if shape == DivvySequenceShape::GraveyardCreaturePiles {
        let mut graveyard_creatures = ObjectFilter::creature();
        graveyard_creatures.zone = Some(Zone::Graveyard);
        graveyard_creatures.owner = Some(PlayerFilter::You);
        let rest_filter = graveyard_creatures
            .clone()
            .not_tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind());
        return Ok(Some(vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: graveyard_creatures,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
            }),
            EffectAst::subject_verb_exile(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind(), None),
                false,
            ),
            EffectAst::subject_verb_return_all_to_battlefield(
                rest_filter,
                false,
                false,
                ReturnControllerAst::You,
            ),
        ]));
    }

    if shape == DivvySequenceShape::OpponentCreaturePilesSacrifice {
        let chosen_pile_filter = ObjectFilter::creature()
            .controlled_by(PlayerFilter::IteratedPlayer)
            .match_tagged(
                crate::tag::CompilerReferenceTag::DivvyPile.bind(),
                TaggedOpbjectRelation::IsTaggedObject,
            );
        let other_pile_filter = ObjectFilter::creature()
            .controlled_by(PlayerFilter::IteratedPlayer)
            .not_tagged(crate::tag::CompilerReferenceTag::DivvyPile.bind());

        return Ok(Some(vec![
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::Opponent,
                effects: vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: crate::tag::CompilerReferenceTag::DivvyPile.bind(),
                })],
            }),
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::Opponent,
                effects: vec![EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
                    player: PlayerAst::You,
                    effects: vec![EffectAst::subject_verb_sacrifice_all(
                        PlayerAst::Implicit,
                        chosen_pile_filter,
                    )],
                    alternative: vec![EffectAst::subject_verb_sacrifice_all(
                        PlayerAst::Implicit,
                        other_pile_filter,
                    )],
                })],
            }),
        ]));
    }

    if shape == DivvySequenceShape::PermanentPilesSacrifice {
        return Ok(Some(vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: ObjectFilter::permanent().controlled_by(PlayerFilter::target_player()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Target,
                tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
            }),
            EffectAst::subject_verb_sacrifice_all(
                PlayerAst::Target,
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind()),
            ),
        ]));
    }

    if shape == DivvySequenceShape::DefendingCreaturePilesBlock {
        return Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::Defending,
            effects: vec![
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
                }),
                EffectAst::subject_verb_cant(
                    crate::effect::Restriction::block(
                        ObjectFilter::creature()
                            .controlled_by(PlayerFilter::IteratedPlayer)
                            .not_tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind()),
                    ),
                    Until::EndOfTurn,
                    None,
                ),
            ],
        })]));
    }

    if shape == DivvySequenceShape::CreaturePilesAttack {
        return Ok(Some(vec![
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::That,
                tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
            }),
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind()),
                ),
                Until::EndOfTurn,
                None,
            ),
        ]));
    }

    if shape == DivvySequenceShape::LandPiles {
        return Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::subject_verb_choose_player(
                    PlayerAst::Implicit,
                    PlayerFilter::Opponent,
                    crate::tag::CompilerReferenceTag::DivvyOpponent.bind(),
                    false,
                    0,
                ),
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter: ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
                }),
                EffectAst::subject_verb_destroy(TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
                    None,
                )),
                EffectAst::subject_verb_tap_all(
                    ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind()),
                ),
            ],
        })]));
    }

    if shape == DivvySequenceShape::ExilePermanentCardsPile {
        let first_sentence = split_lexed_sentences(sentences[0].lowered())
            .into_iter()
            .next()
            .unwrap_or_else(|| sentences[0].lowered());
        let first_effect_tokens =
            crate::slice_primitives::find_window_by(first_sentence, 3, |window| {
                window[0].is_word("and")
                    && window[1].is_word("separate")
                    && window[2].is_word("them")
            })
            .map_or(first_sentence, |split| &first_sentence[..split]);
        let mut effects = parse_effect_sentence_sequence(first_effect_tokens)?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                vec![Zone::Exile],
                crate::tag::CompilerReferenceTag::DivvySource.bind(),
            ),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(crate::tag::CompilerReferenceTag::DivvySource.bind()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: crate::tag::CompilerReferenceTag::DivvyPile.bind(),
                zones: vec![Zone::Exile],
                search_mode: None,
            }),
            EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
                player: PlayerAst::Opponent,
                effects: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyPile.bind(), None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                        tag: crate::tag::CompilerReferenceTag::DivvySource.bind(),
                        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(
                                crate::tag::CompilerReferenceTag::DivvyPile,
                            ),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                                Zone::Hand,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        })],
                    }),
                ],
                alternative: vec![
                    EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyPile.bind(), None),
                        false,
                    ),
                    EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                        tag: crate::tag::CompilerReferenceTag::DivvySource.bind(),
                        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(
                                crate::tag::CompilerReferenceTag::DivvyPile,
                            ),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                                Zone::Graveyard,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        })],
                    }),
                ],
            }),
        ]);
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::RevealTopPiles {
        let mut effects = parse_effect_sentence_sequence(sentences[0].lowered())?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                vec![Zone::Library],
                crate::tag::CompilerReferenceTag::DivvySource.bind(),
            ),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(crate::tag::CompilerReferenceTag::DivvySource.bind()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: crate::tag::CompilerReferenceTag::DivvyPile.bind(),
                zones: vec![Zone::Library],
                search_mode: None,
            }),
            EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
                player: PlayerAst::You,
                effects: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyPile.bind(), None),
                        Zone::Hand,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                        tag: crate::tag::CompilerReferenceTag::DivvySource.bind(),
                        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(
                                crate::tag::CompilerReferenceTag::DivvyPile,
                            ),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                                Zone::Graveyard,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        })],
                    }),
                ],
                alternative: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyPile.bind(), None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                        tag: crate::tag::CompilerReferenceTag::DivvySource.bind(),
                        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(
                                crate::tag::CompilerReferenceTag::DivvyPile,
                            ),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                                Zone::Hand,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        })],
                    }),
                ],
            }),
        ]);
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::ExileCreatureCardsFromGraveyards {
        let first_effect_tokens = split_lexed_sentences(sentences[0].lowered())
            .into_iter()
            .next()
            .unwrap_or_else(|| sentences[0].lowered());
        let mut effects = parse_effect_sentence_sequence(first_effect_tokens)?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
                vec![Zone::Exile],
                crate::tag::CompilerReferenceTag::DivvySource.bind(),
            ),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(crate::tag::CompilerReferenceTag::DivvySource.bind()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
                zones: vec![Zone::Exile],
                search_mode: None,
            }),
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind(), None),
                Zone::Battlefield,
                false,
                ReturnControllerAst::You,
                false,
                None,
            ),
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::CompilerReferenceTag::DivvySource.bind(),
                effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object(
                        crate::tag::CompilerReferenceTag::DivvyChosen,
                    ),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                })],
            }),
        ]);
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::ChooseOneOfThem {
        let mut prefix = Vec::new();
        prefix.extend(parse_effect_sentence_lexed(sentences[0].lowered())?);
        prefix.extend(parse_effect_sentence_lexed(sentences[1].lowered())?);
        let mut effects = prefix;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
            vec![Zone::Library],
            crate::tag::CompilerReferenceTag::DivvySource.bind(),
        ));
        effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(crate::tag::CompilerReferenceTag::DivvySource.bind()),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
            zones: vec![Zone::Library],
            search_mode: None,
        }));
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind(), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::DivvySource.bind(),
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object(
                    crate::tag::CompilerReferenceTag::DivvyChosen,
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            })],
        }));
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
        ));
        return Ok(Some(effects));
    }

    if let DivvySequenceShape::SearchFourDifferentNames { chooser, rest } = shape {
        let choose_player = match chooser {
            DivvyChooserShape::Opponent => PlayerAst::Opponent,
            DivvyChooserShape::TargetOpponent => PlayerAst::TargetOpponent,
        };
        let (rest_zone, rest_enters_tapped) = match rest {
            DivvyRestDestinationShape::Hand => (Zone::Hand, false),
            DivvyRestDestinationShape::BattlefieldTapped => (Zone::Battlefield, true),
        };

        let mut effects = parse_effect_sentence_lexed(sentences[0].lowered())?;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
            vec![Zone::Library],
            crate::tag::CompilerReferenceTag::DivvySource.bind(),
        ));
        effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(crate::tag::CompilerReferenceTag::DivvySource.bind()),
            count: ChoiceCount::exactly(2),
            count_value: None,
            player: choose_player,
            tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
            zones: vec![Zone::Library],
            search_mode: None,
        }));
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind(), None),
            Zone::Graveyard,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::DivvySource.bind(),
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object(
                    crate::tag::CompilerReferenceTag::DivvyChosen,
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    rest_zone,
                    false,
                    ReturnControllerAst::Preserve,
                    rest_enters_tapped,
                    None,
                )],
            })],
        }));
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
        ));
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::SearchFourDifferentPowers {
        let source_tag = crate::tag::CompilerReferenceTag::Searched.bind();
        let chosen_tag = crate::tag::CompilerReferenceTag::DivvyChosen.bind();
        let mut effects = parse_effect_sentence_lexed(sentences[0].lowered())?;
        effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(source_tag.clone()),
            count: ChoiceCount::exactly(2),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: chosen_tag.clone(),
            zones: vec![Zone::Library],
            search_mode: None,
        }));
        effects.push(EffectAst::Coordinated {
            effects: vec![
                EffectAst::subject_verb_shuffle_objects_into_library(
                    PlayerAst::You,
                    TargetAst::Tagged(chosen_tag.clone(), None),
                ),
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Object(
                        ObjectFilter::tagged(source_tag)
                            .match_tagged(chosen_tag, TaggedOpbjectRelation::IsNotTaggedObject),
                        None,
                        None,
                    ),
                    Zone::Hand,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
            ],
            leading_duration: false,
            result_conjunction: false,
        });
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::TargetOpponentChoosesOne {
        let mut effects = parse_effect_sentence_lexed(sentences[0].lowered())?;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
            vec![Zone::Library],
            crate::tag::CompilerReferenceTag::DivvySource.bind(),
        ));
        effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(crate::tag::CompilerReferenceTag::DivvySource.bind()),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::TargetOpponent,
            tag: crate::tag::CompilerReferenceTag::DivvyChosen.bind(),
            zones: vec![Zone::Library],
            search_mode: None,
        }));
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyChosen.bind(), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::DivvySource.bind(),
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object(
                    crate::tag::CompilerReferenceTag::DivvyChosen,
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            })],
        }));
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
        ));
        return Ok(Some(effects));
    }

    Ok(None)
}
