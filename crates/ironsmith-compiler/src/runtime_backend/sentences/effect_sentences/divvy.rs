use super::super::front_end::grammar::effects::divvy_shapes::{
    self, DivvyChooserShape, DivvyRestDestinationShape, DivvySequenceShape,
};
use super::super::lexer::{OwnedLexToken, split_lexed_sentences};
use super::dispatch_entry::SentenceInput;
use super::dispatch_inner::parse_effect_sentence_lexed;
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::zone::Zone;

fn membership_predicate_for_iterated_object(tag: &str) -> PredicateAst {
    PredicateAst::TaggedMatches(
        TagKey::from(tag),
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG)),
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
        .map(SentenceInput::lowered)
        .collect::<Vec<_>>();
    let Some(shape) = divvy_shapes::parse_divvy_sequence_shape(&sentence_tokens) else {
        return Ok(None);
    };

    if shape == DivvySequenceShape::SearchFourCreatureCards {
        let first_effect_tokens = split_lexed_sentences(sentences[0].lowered())
            .into_iter()
            .next()
            .unwrap_or_else(|| sentences[0].lowered());
        let mut effects = parse_effect_sentence_sequence(first_effect_tokens)?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                vec![Zone::Library, Zone::Graveyard],
                TagKey::from("divvy_source"),
            ),
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::exactly(2),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Library, Zone::Graveyard],
                search_mode: None,
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_chosen"),
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Library,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            },
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::You,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
            EffectAst::subject_verb_exile(TargetAst::Source(None), false),
        ]);
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::ExchangeCreatureControl {
        let first_player_tag = TagKey::from("exchange_player_one");
        let second_player_tag = TagKey::from("exchange_player_two");
        let first_creatures_tag = TagKey::from("exchange_creatures_one");
        let second_creatures_tag = TagKey::from("exchange_creatures_two");

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
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature()
                    .controlled_by(PlayerFilter::TaggedPlayer(first_player_tag.clone())),
                count: ChoiceCount::up_to_dynamic_x(),
                count_value: Some(Value::Count(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::TaggedPlayer(second_player_tag.clone())),
                )),
                player: PlayerAst::You,
                tag: first_creatures_tag.clone(),
            },
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature()
                    .controlled_by(PlayerFilter::TaggedPlayer(second_player_tag.clone())),
                count: ChoiceCount::dynamic_x(),
                count_value: Some(Value::Count(ObjectFilter::tagged(
                    first_creatures_tag.clone(),
                ))),
                player: PlayerAst::You,
                tag: second_creatures_tag.clone(),
            },
            EffectAst::ForEachTaggedPlayer {
                tag: second_player_tag,
                effects: vec![EffectAst::subject_verb_gain_control(
                    PlayerAst::That,
                    TargetAst::Tagged(first_creatures_tag, None),
                    Until::Forever,
                )],
            },
            EffectAst::ForEachTaggedPlayer {
                tag: first_player_tag,
                effects: vec![EffectAst::subject_verb_gain_control(
                    PlayerAst::That,
                    TargetAst::Tagged(second_creatures_tag, None),
                    Until::Forever,
                )],
            },
        ]));
    }

    if shape == DivvySequenceShape::DestroyChosenCreaturePile {
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::target_player()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Target,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_destroy_no_regeneration(TargetAst::Tagged(
                TagKey::from("divvy_chosen"),
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
            .not_tagged(TagKey::from("divvy_chosen"));
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: graveyard_creatures,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_exile(
                TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
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
                TagKey::from("divvy_pile"),
                TaggedOpbjectRelation::IsTaggedObject,
            );
        let other_pile_filter = ObjectFilter::creature()
            .controlled_by(PlayerFilter::IteratedPlayer)
            .not_tagged(TagKey::from("divvy_pile"));

        return Ok(Some(vec![
            EffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::Opponent,
                effects: vec![EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: TagKey::from("divvy_pile"),
                }],
            },
            EffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::Opponent,
                effects: vec![EffectAst::UnlessAction {
                    player: PlayerAst::You,
                    effects: vec![EffectAst::subject_verb_sacrifice_all(
                        PlayerAst::Implicit,
                        chosen_pile_filter,
                    )],
                    alternative: vec![EffectAst::subject_verb_sacrifice_all(
                        PlayerAst::Implicit,
                        other_pile_filter,
                    )],
                }],
            },
        ]));
    }

    if shape == DivvySequenceShape::PermanentPilesSacrifice {
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::permanent().controlled_by(PlayerFilter::target_player()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Target,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_sacrifice_all(
                PlayerAst::Target,
                ObjectFilter::tagged(TagKey::from("divvy_chosen")),
            ),
        ]));
    }

    if shape == DivvySequenceShape::DefendingCreaturePilesBlock {
        return Ok(Some(vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::Defending,
            effects: vec![
                EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: TagKey::from("divvy_chosen"),
                },
                EffectAst::subject_verb_cant(
                    crate::effect::Restriction::block(
                        ObjectFilter::creature()
                            .controlled_by(PlayerFilter::IteratedPlayer)
                            .not_tagged(TagKey::from("divvy_chosen")),
                    ),
                    Until::EndOfTurn,
                    None,
                ),
            ],
        }]));
    }

    if shape == DivvySequenceShape::CreaturePilesAttack {
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::That,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
                Until::EndOfTurn,
                None,
            ),
        ]));
    }

    if shape == DivvySequenceShape::LandPiles {
        return Ok(Some(vec![EffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::subject_verb_choose_player(
                    PlayerAst::Implicit,
                    PlayerFilter::Opponent,
                    TagKey::from("divvy_opponent"),
                    false,
                    0,
                ),
                EffectAst::ChooseObjects {
                    filter: ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: TagKey::from("divvy_chosen"),
                },
                EffectAst::subject_verb_destroy(TargetAst::Tagged(
                    TagKey::from("divvy_chosen"),
                    None,
                )),
                EffectAst::subject_verb_tap_all(
                    ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
            ],
        }]));
    }

    if shape == DivvySequenceShape::ExilePermanentCardsPile {
        let first_effect_tokens = split_lexed_sentences(sentences[0].lowered())
            .into_iter()
            .next()
            .unwrap_or_else(|| sentences[0].lowered());
        let mut effects = parse_effect_sentence_sequence(first_effect_tokens)?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                vec![Zone::Exile],
                TagKey::from("divvy_source"),
            ),
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: TagKey::from("divvy_pile"),
                zones: vec![Zone::Exile],
                search_mode: None,
            },
            EffectAst::UnlessAction {
                player: PlayerAst::Opponent,
                effects: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from("divvy_pile"), None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: TagKey::from("divvy_source"),
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object("divvy_pile"),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Hand,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
                alternative: vec![
                    EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(TagKey::from("divvy_pile"), None),
                        false,
                    ),
                    EffectAst::ForEachTagged {
                        tag: TagKey::from("divvy_source"),
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object("divvy_pile"),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Graveyard,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
            },
        ]);
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::RevealTopPiles {
        let mut effects = parse_effect_sentence_sequence(sentences[0].lowered())?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                vec![Zone::Library],
                TagKey::from("divvy_source"),
            ),
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_pile"),
                zones: vec![Zone::Library],
                search_mode: None,
            },
            EffectAst::UnlessAction {
                player: PlayerAst::You,
                effects: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from("divvy_pile"), None),
                        Zone::Hand,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: TagKey::from("divvy_source"),
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object("divvy_pile"),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Graveyard,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
                alternative: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from("divvy_pile"), None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: TagKey::from("divvy_source"),
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object("divvy_pile"),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Hand,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
            },
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
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                vec![Zone::Exile],
                TagKey::from("divvy_source"),
            ),
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Exile],
                search_mode: None,
            },
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                Zone::Battlefield,
                false,
                ReturnControllerAst::You,
                false,
                None,
            ),
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            },
        ]);
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::ChooseOneOfThem {
        let mut prefix = Vec::new();
        prefix.extend(parse_effect_sentence_lexed(sentences[0].lowered())?);
        prefix.extend(parse_effect_sentence_lexed(sentences[1].lowered())?);
        let mut effects = prefix;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
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
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(2),
            count_value: None,
            player: choose_player,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Graveyard,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    rest_zone,
                    false,
                    ReturnControllerAst::Preserve,
                    rest_enters_tapped,
                    None,
                )],
            }],
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(effects));
    }

    if shape == DivvySequenceShape::TargetOpponentChoosesOne {
        let mut effects = parse_effect_sentence_lexed(sentences[0].lowered())?;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::TargetOpponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(effects));
    }

    Ok(None)
}
