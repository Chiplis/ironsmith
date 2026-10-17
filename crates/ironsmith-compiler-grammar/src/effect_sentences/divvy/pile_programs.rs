//! Lower pile programs into ordinary choice, tag, restriction and move
//! effects. A pile is a tagged object set: a player separates objects by
//! choosing the members of every pile but the last (which is the remainder),
//! and a pile is then chosen by a player (`ChooseOneOf`) or at random
//! (`ChooseOneOfAtRandom`).
use super::*;
use crate::cards::builders::{ChooseOneModeAst, GrantActionAst};
use crate::tag::TagRef;
use divvy_shapes::{
    ExilePlayDuration, PileObjectKind, PileOp, PilePickActor, PilePickDestination,
    PileProgramShape,
};

fn kind_filter(kind: PileObjectKind) -> ObjectFilter {
    match kind {
        PileObjectKind::Permanents => ObjectFilter::permanent(),
        PileObjectKind::Creatures => ObjectFilter::creature(),
        PileObjectKind::Lands => ObjectFilter::land(),
    }
}

/// The captured incarnations of a tagged pool in `zone`.
fn capture(tag: &TagRef, zone: Zone) -> ObjectFilter {
    ObjectFilter::tagged(tag.clone())
        .in_zone(zone)
        .match_tagged(tag.clone(), TaggedOpbjectRelation::SameObjectId)
}

fn move_all(filter: ObjectFilter, zone: Zone) -> EffectAst {
    EffectAst::subject_verb_move_all_to_zone(
        TargetAst::Object(filter, None, None),
        zone,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    )
}

fn move_tagged(tag: &TagRef, zone: Zone) -> EffectAst {
    EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(tag.clone(), None),
        zone,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    )
}

fn choose_from(filter: ObjectFilter, count: ChoiceCount, player: PlayerAst, tag: &TagRef, zone: Zone) -> EffectAst {
    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
        optional_search_zones: false,
        filter,
        count,
        count_value: None,
        player,
        tag: tag.clone(),
        zones: vec![zone],
        search_mode: None,
    })
}

fn shuffle_your_library() -> EffectAst {
    EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::You,
        SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
    )
}

pub(super) fn lower(
    shape: PileProgramShape,
    sentences: &[SentenceInput],
) -> Result<Vec<EffectAst>, CardTextError> {
    let tag = |suffix: &str| crate::util::helper_tag_for_tokens(sentences[0].lexed(), suffix);
    let (mut effects, consumed) = match shape {
        PileProgramShape::EachPlayerRandomPileSacrifice {
            piles,
            kind,
            starting_with_you,
            consumed_sentences,
        } => {
            let piles = usize::from(piles.max(2));
            let base = kind_filter(kind).controlled_by(PlayerFilter::IteratedPlayer);
            // The player chooses the members of each pile but the last; the
            // last pile is everything left. Piles may be empty (CR 700.3).
            let pile_tags = (1..piles)
                .map(|index| tag(format!("pile_program_pile_{index}").as_str()))
                .collect::<Vec<_>>();
            let mut separate = Vec::with_capacity(pile_tags.len());
            for (index, pile) in pile_tags.iter().enumerate() {
                let filter = pile_tags[..index]
                    .iter()
                    .fold(base.clone(), |filter, earlier| filter.not_tagged(earlier.clone()));
                separate.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: pile.clone(),
                }));
            }
            let separation = EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects: separate });
            let separation = if starting_with_you {
                EffectAst::SourceSentence {
                    effects: vec![separation],
                    leading_then: false,
                    starting_with_controller: true,
                }
            } else {
                separation
            };
            let modes = (0..piles)
                .map(|index| {
                    let filter = match pile_tags.get(index) {
                        Some(pile) => base
                            .clone()
                            .match_tagged(pile.clone(), TaggedOpbjectRelation::IsTaggedObject),
                        None => pile_tags
                            .iter()
                            .fold(base.clone(), |filter, pile| filter.not_tagged(pile.clone())),
                    };
                    ChooseOneModeAst {
                        description: format!("Pile {}", index + 1),
                        effects: vec![EffectAst::subject_verb_sacrifice_all(PlayerAst::That, filter)],
                    }
                })
                .collect();
            let sacrifice = EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                effects: vec![EffectAst::ObjectChoices(
                    ObjectChoiceEffectAst::ChooseOneOfAtRandom { modes },
                )],
            });
            (vec![separation, sacrifice], consumed_sentences)
        }
        PileProgramShape::AlternatingRevealedPicks {
            count,
            ops,
            rest,
            consumed_sentences,
        } => {
            let pool = tag("pile_program_revealed");
            let opponent = tag("pile_program_opponent");
            let mut effects = vec![EffectAst::subject_verb_reveal_top_cards(
                PlayerAst::You,
                Value::Fixed(i32::try_from(count).unwrap_or(i32::MAX)),
                pool.clone(),
            )];
            // Cards already put on the bottom stay in the library (and in the
            // revealed pool), so every later pick excludes them explicitly.
            let mut bottomed: Vec<TagRef> = Vec::new();
            for (index, op) in ops.iter().flatten().enumerate() {
                match *op {
                    PileOp::ChooseOpponent => effects.push(EffectAst::subject_verb_choose_player(
                        PlayerAst::You,
                        PlayerFilter::Opponent,
                        opponent.clone(),
                        false,
                        0,
                    )),
                    PileOp::Pick { actor, destination } => {
                        let picked = tag(format!("pile_program_pick_{index}").as_str());
                        let remaining = bottomed
                            .iter()
                            .fold(capture(&pool, Zone::Library), |filter, placed| {
                                filter.not_tagged(placed.clone())
                            });
                        let zone = match destination {
                            PilePickDestination::Hand => Zone::Hand,
                            PilePickDestination::LibraryBottom => Zone::Library,
                        };
                        match actor {
                            PilePickActor::You => {
                                effects.push(choose_from(
                                    remaining,
                                    ChoiceCount::exactly(1),
                                    PlayerAst::You,
                                    &picked,
                                    Zone::Library,
                                ));
                                effects.push(move_tagged(&picked, zone));
                            }
                            // The chosen opponent picks; iterating that one
                            // player keeps "they" bound to them even after
                            // your own interleaved picks.
                            PilePickActor::ChosenOpponent => {
                                effects.push(EffectAst::ForEach(
                                    ForEachEffectAst::ForEachPlayersFiltered {
                                        sequential: false,
                                        filter: PlayerFilter::TaggedPlayer(opponent.clone().into()),
                                        effects: vec![
                                            choose_from(
                                                remaining,
                                                ChoiceCount::exactly(1),
                                                PlayerAst::That,
                                                &picked,
                                                Zone::Library,
                                            ),
                                            move_tagged(&picked, zone),
                                        ],
                                    },
                                ));
                            }
                        }
                        if destination == PilePickDestination::LibraryBottom {
                            bottomed.push(picked);
                        }
                    }
                }
            }
            let remaining = bottomed
                .iter()
                .fold(capture(&pool, Zone::Library), |filter, placed| {
                    filter.not_tagged(placed.clone())
                });
            effects.push(move_all(
                remaining,
                match rest {
                    PilePickDestination::Hand => Zone::Hand,
                    PilePickDestination::LibraryBottom => Zone::Library,
                },
            ));
            (effects, consumed_sentences)
        }
        PileProgramShape::OpponentFaceDownPilesExileOneSearchOther {
            count,
            minimum_library,
            consumed_sentences,
        } => {
            let pool = tag("pile_program_pool");
            let first = tag("pile_program_first_pile");
            let second = tag("pile_program_second_pile");
            let selected = tag("pile_program_selected");
            let count = Value::Fixed(i32::try_from(count).unwrap_or(i32::MAX));
            let mut program = vec![
                // Only the target opponent sees the cards while separating.
                EffectAst::PlayerLooksAtTopCardsOfLibrary {
                    viewer: PlayerAst::TargetOpponent,
                    library_owner: PlayerAst::You,
                    count,
                    tag: pool.clone(),
                },
                choose_from(
                    capture(&pool, Zone::Library),
                    ChoiceCount::any_number(),
                    PlayerAst::That,
                    &first,
                    Zone::Library,
                ),
                EffectAst::subject_verb_tag_matching_objects(
                    capture(&pool, Zone::Library).not_tagged(first.clone()),
                    vec![Zone::Library],
                    second.clone(),
                ),
            ];
            let modes = [(&first, &second), (&second, &first)]
                .into_iter()
                .enumerate()
                .map(|(index, (exiled, searched))| ChooseOneModeAst {
                    description: format!("Exile face-down pile {}", index + 1),
                    effects: vec![
                        move_all(capture(exiled, Zone::Library), Zone::Exile),
                        // The pile is still in the library; searching it
                        // reveals it to you, and its rest is shuffled back.
                        choose_from(
                            capture(searched, Zone::Library),
                            ChoiceCount::exactly(1),
                            PlayerAst::You,
                            &selected,
                            Zone::Library,
                        ),
                        move_tagged(&selected, Zone::Hand),
                        shuffle_your_library(),
                    ],
                })
                .collect();
            program.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
                chooser: PlayerFilter::You,
                modes,
            }));
            let effects = match minimum_library {
                Some(minimum) => vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate: PredicateAst::ValueComparison {
                        left: Value::CardsInLibrary(PlayerFilter::You),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(i32::try_from(minimum).unwrap_or(i32::MAX)),
                    },
                    if_true: program,
                    if_false: Vec::new(),
                })],
                None => program,
            };
            (effects, consumed_sentences)
        }
        PileProgramShape::DefendingLabeledPilesBlockRestriction { consumed_sentences } => {
            use crate::static_abilities::StaticAbilityId;
            let left = tag("pile_program_left_pile");
            let right = tag("pile_program_right_pile");
            let right_here = tag("pile_program_right_pile_of_player");
            let divisible = ObjectFilter::creature()
                .without_static_ability(StaticAbilityId::Flying)
                .controlled_by(PlayerFilter::IteratedPlayer);
            let divide = EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                sequential: false,
                filter: PlayerFilter::Defending,
                effects: vec![
                    // Each defender's "left" choices accumulate in one pile set.
                    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                        filter: divisible.clone(),
                        count: ChoiceCount::any_number(),
                        count_value: None,
                        player: PlayerAst::That,
                        tag: left.clone(),
                    }),
                    EffectAst::subject_verb_tag_matching_objects(
                        divisible.not_tagged(left.clone()),
                        vec![Zone::Battlefield],
                        right_here.clone(),
                    ),
                    // Union this defender's "right" pile into the shared set.
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        PlayerAst::Implicit,
                        SubjectVerbActionAst::TagMatchingObjects {
                            filter: ObjectFilter::default(),
                            zones: vec![Zone::Battlefield],
                            tag: right.clone(),
                            source_tags: vec![right.clone(), right_here.clone()],
                            filter_source_snapshots: false,
                        },
                    ),
                ],
            });
            let mut attackers = ObjectFilter::creature().controlled_by(PlayerFilter::You);
            attackers.attacking = true;
            let attacker = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
            let modes = [("left", &left), ("right", &right)]
                .into_iter()
                .map(|(label, pile)| ChooseOneModeAst {
                    description: format!("\"{label}\""),
                    effects: vec![EffectAst::subject_verb_cant(
                        // Only flyers and creatures in the labeled pile may
                        // block this attacker.
                        crate::effect::Restriction::block_specific_attacker(
                            ObjectFilter::creature()
                                .without_static_ability(StaticAbilityId::Flying)
                                .not_tagged(pile.clone()),
                            attacker.clone(),
                        ),
                        Until::EndOfCombat,
                        None,
                    )],
                })
                .collect();
            let label = EffectAst::ForEach(ForEachEffectAst::ForEachObject {
                filter: attackers,
                effects: vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
                    chooser: PlayerFilter::You,
                    modes,
                })],
            });
            (vec![divide, label], consumed_sentences)
        }
        PileProgramShape::FaceDownFaceUpExilePilesDefenderChooses {
            count,
            consumed_sentences,
        } => {
            let pool = tag("pile_program_looked");
            let face_down = tag("pile_program_face_down_pile");
            let face_up = tag("pile_program_face_up_pile");
            let effects = vec![
                EffectAst::PlayerLooksAtTopCardsOfLibrary {
                    viewer: PlayerAst::You,
                    library_owner: PlayerAst::You,
                    count: Value::Fixed(i32::try_from(count).unwrap_or(i32::MAX)),
                    tag: pool.clone(),
                },
                choose_from(
                    capture(&pool, Zone::Library),
                    ChoiceCount::any_number(),
                    PlayerAst::You,
                    &face_down,
                    Zone::Library,
                ),
                EffectAst::subject_verb_tag_matching_objects(
                    capture(&pool, Zone::Library).not_tagged(face_down.clone()),
                    vec![Zone::Library],
                    face_up.clone(),
                ),
                EffectAst::subject_verb_exile(TargetAst::Tagged(face_down.clone(), None), true),
                EffectAst::subject_verb_exile(TargetAst::Tagged(face_up.clone(), None), false),
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
                    chooser: PlayerFilter::Defending,
                    modes: [("face-down", &face_down), ("face-up", &face_up)]
                        .into_iter()
                        .map(|(label, pile)| ChooseOneModeAst {
                            description: format!("Choose the {label} pile"),
                            effects: vec![move_all(
                                ObjectFilter::tagged(pile.clone()).in_zone(Zone::Exile),
                                Zone::Hand,
                            )],
                        })
                        .collect(),
                }),
            ];
            (effects, consumed_sentences)
        }
        PileProgramShape::EachPlayerExilesTopMayPlay {
            count,
            duration,
            cant_play_from_hand,
            consumed_sentences,
        } => {
            let exiled = tag("pile_program_exiled_by_player");
            let per_player = vec![
                // `tags` restarts per iteration: each grant names only the
                // cards that player exiled.
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::That,
                    SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                        count: Value::Fixed(i32::try_from(count).unwrap_or(i32::MAX)),
                        surface: None,
                        tags: vec![exiled.clone()],
                        accumulated_tags: vec![],
                        face_down: false,
                    }),
                ),
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::Actor,
                    PlayerAst::Implicit,
                    SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn {
                        tag: exiled.clone(),
                        player: PlayerAst::That,
                        allow_land: true,
                        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::Normal,
                        until_next_end_step: duration == ExilePlayDuration::UntilYourNextEndStep,
                        until_next_turn_start: duration == ExilePlayDuration::UntilYourNextTurn,
                        during_next_turn: false,
                        until_end_of_combat_next_turn: false,
                        spell_filter: None,
                        surface: None,
                        max_plays: None,
                        without_paying_mana_cost: false,
                    }),
                ),
            ];
            let mut effects = vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                effects: per_player,
            })];
            if cant_play_from_hand {
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::PlayLandsMatching(
                        PlayerFilter::Any,
                        ObjectFilter::land().in_zone(Zone::Hand),
                    ),
                    Until::YourNextTurn,
                    None,
                ));
                effects.push(EffectAst::subject_verb_cant(
                    crate::effect::Restriction::cast_spells_matching(
                        PlayerFilter::Any,
                        ObjectFilter::default().in_zone(Zone::Hand),
                    ),
                    Until::YourNextTurn,
                    None,
                ));
            }
            (effects, consumed_sentences)
        }
    };
    for sentence in sentences.get(consumed..).unwrap_or_default() {
        effects.extend(parse_effect_sentence_sequence(sentence.lowered())?);
    }
    Ok(effects)
}
