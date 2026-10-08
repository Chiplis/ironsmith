//! Compose binary card piles from finite captures and ordinary choice/move effects.
use super::*;
use divvy_shapes::{BinaryPileCount, BinaryPileDestination, BinaryPilePartitioner, BinaryPileProducer, BinaryPileProgramShape};

fn capture_filter(tag: &crate::tag::TagRef, source: Zone) -> ObjectFilter {
    // Keep the tagged-pool relation for hidden-zone choice discovery, and
    // constrain it to the captured incarnation for later replacement safety.
    ObjectFilter::tagged(tag.clone()).in_zone(source)
        .match_tagged(tag.clone(), TaggedOpbjectRelation::SameObjectId)
}

fn move_capture(tag: &crate::tag::TagRef, source: Zone, destination: Zone) -> EffectAst {
    EffectAst::subject_verb_move_all_to_zone(
        TargetAst::Object(capture_filter(tag, source), None, None), destination, false,
        ReturnControllerAst::Preserve, false, None,
    )
}

/// A counter continuation after this captured movement consumes actual
/// arrivals. The ordinary action-qualified resolver binds its producer ID.
fn bind_destination_counter_results(effects: &mut [EffectAst], preceding_pile: &mut bool) {
    fn bind(value: &mut Value) {
        match value {
            Value::PendingPriorEffectMetric(query)
                if query.action == Some(ironsmith_core::PriorEffectAction::PutIntoGraveyard) => {
                query.original_destination = Some(Zone::Graveyard);
            }
            Value::SurfaceHinted { value, .. } | Value::Scaled(value, _)
            | Value::DividedRoundedDown(value, _) | Value::HalfRoundedDown(value) => bind(value),
            Value::Add(left, right) | Value::Min(left, right) => { bind(left); bind(right); }
            _ => {}
        }
    }
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(subject) => {
                match &mut subject.action {
                    SubjectVerbActionAst::Counters(crate::cards::builders::CounterActionAst::PutCounters { count, .. }) => {
                        if *preceding_pile { bind(count); }
                    }
                    SubjectVerbActionAst::Tokens(_) => {},
                    // A later independent instruction may own its own result.
                    // Do not impose the pile's receipt on that new producer.
                    _ => *preceding_pile = false,
                }
            }
            EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, .. }) if *preceding_pile => bind(count),
            _ => {}
        }
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested|
            bind_destination_counter_results(nested, preceding_pile));
    }
}

pub(super) fn lower(program: BinaryPileProgramShape, sentences: &[SentenceInput])
    -> Result<Vec<EffectAst>, CardTextError>
{
    let tag = |suffix| crate::util::helper_tag_for_tokens(sentences[0].lexed(), suffix);
    let pool = tag("binary_pool");
    let first = tag("binary_first_pile");
    let second = tag("binary_second_pile");
    let opponent = tag("binary_choosing_opponent");
    let selected = tag("binary_selected_card");
    let remainder = tag("binary_remainder");
    let (source_zone, mut effects) = match program.producer {
        BinaryPileProducer::TopLibrary(count) => {
    let count = match count {
        BinaryPileCount::Fixed(count) => Value::Fixed(count),
        BinaryPileCount::XPlus(extra) => Value::Add(Box::new(Value::X), Box::new(Value::Fixed(extra))),
    };
    let producer = if program.reveal_pool {
        EffectAst::subject_verb_reveal_top_cards(PlayerAst::You, count, pool.clone())
    } else {
        EffectAst::PlayerLooksAtTopCardsOfLibrary {
            viewer: match program.partitioner {
                BinaryPilePartitioner::You => PlayerAst::You,
                BinaryPilePartitioner::TargetOpponent => PlayerAst::TargetOpponent,
            },
            library_owner: PlayerAst::You,
            count,
            tag: pool.clone(),
        }
    };
    let mut produced = vec![
        producer,
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            filter: capture_filter(&pool, Zone::Library),
            count: ChoiceCount::any_number(), count_value: None,
            // The private viewer already declared the target, so this is a
            // reference to that player, not a second target declaration.
            player: match program.partitioner {
                BinaryPilePartitioner::You => PlayerAst::You,
                BinaryPilePartitioner::TargetOpponent => PlayerAst::That,
            },
            tag: first.clone(), zones: vec![Zone::Library], search_mode: None,
        }),
        // Freeze the complementary incarnation set before any move/replacement.
        EffectAst::subject_verb_tag_matching_objects(
            capture_filter(&pool, Zone::Library).not_tagged(first.clone()),
            vec![Zone::Library], second.clone(),
        ),
    ];
    if !program.reveal_pool {
        produced.push(EffectAst::subject_verb_reveal_tagged(second.clone()));
    }
            (Zone::Library, produced)
        }
        BinaryPileProducer::SequentialFaceDownExile { first: first_count, second: second_count } => {
            let mut produced = Vec::new();
            for (count, tag) in [(first_count, &first), (second_count, &second)] {
                produced.push(EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner, PlayerAst::You,
                    SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                        count: Value::Fixed(count), surface: None, tags: vec![tag.clone()],
                        accumulated_tags: vec![], face_down: true,
                    }),
                ));
            }
            for tag in [&first, &second] {
                // Each producer names its actual exiled arrivals. Retain only
                // those same incarnations after replacement-added instructions.
                produced.push(EffectAst::subject_verb_tag_matching_objects(
                    capture_filter(tag, Zone::Exile), vec![Zone::Exile], tag.clone(),
                ));
                produced.push(EffectAst::subject_verb_look_at_objects(
                    PlayerAst::You, capture_filter(tag, Zone::Exile),
                ));
            }
            produced.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
                chooser: PlayerFilter::You,
                modes: [&first, &second].into_iter().enumerate().map(|(index, tag)| {
                    crate::cards::builders::ChooseOneModeAst {
                        description: format!("Turn pile {} face up", index + 1),
                        effects: vec![
                            EffectAst::subject_verb(
                                SubjectVerbRoleAst::Actor, PlayerAst::You,
                                SubjectVerbActionAst::PermanentState(crate::cards::builders::PermanentStateActionAst::TurnFaceUp {
                                    target: TargetAst::Tagged(tag.clone(), None),
                                }),
                            ),
                        ],
                    }
                }).collect(),
            }));
            (Zone::Exile, produced)
        }
        BinaryPileProducer::FaceDownThenFaceUpExile { first: first_count, second: second_count } => {
            let mut produced = Vec::new();
            for (count, tag, face_down) in [(first_count, &first, true), (second_count, &second, false)] {
                produced.push(EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner, PlayerAst::You,
                    SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                        count: Value::Fixed(count), surface: None, tags: vec![tag.clone()],
                        accumulated_tags: vec![], face_down,
                    }),
                ));
            }
            for tag in [&first, &second] {
                produced.push(EffectAst::subject_verb_tag_matching_objects(
                    capture_filter(tag, Zone::Exile), vec![Zone::Exile], tag.clone(),
                ));
            }
            (Zone::Exile, produced)
        }
        BinaryPileProducer::GraveyardCards(card_type) => {
            let pool_filter = ObjectFilter::default()
                .with_type(card_type)
                .owned_by(PlayerFilter::You)
                .in_zone(Zone::Graveyard);
            let produced = vec![
                EffectAst::subject_verb_tag_matching_objects(
                    pool_filter,
                    vec![Zone::Graveyard],
                    pool.clone(),
                ),
                // You separate the pool: the first pile is any subset of it.
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                    filter: capture_filter(&pool, Zone::Graveyard),
                    count: ChoiceCount::any_number(), count_value: None,
                    player: PlayerAst::You,
                    tag: first.clone(), zones: vec![Zone::Graveyard], search_mode: None,
                }),
                EffectAst::subject_verb_tag_matching_objects(
                    capture_filter(&pool, Zone::Graveyard).not_tagged(first.clone()),
                    vec![Zone::Graveyard], second.clone(),
                ),
            ];
            (Zone::Graveyard, produced)
        }
    };
    let chooser = match program.partitioner {
        BinaryPilePartitioner::TargetOpponent => PlayerFilter::You,
        BinaryPilePartitioner::You => {
            effects.push(EffectAst::subject_verb_choose_player(
                PlayerAst::You, PlayerFilter::Opponent, opponent.clone(), false, 0,
            ));
            PlayerFilter::TaggedPlayer(opponent.into())
        }
    };
    let modes = [(&first, &second), (&second, &first)].into_iter().enumerate()
        .map(|(index, (chosen, other))| {
            let effects = match program.destination {
                BinaryPileDestination::HandAndGraveyard => {
                    let mut union = ObjectFilter::default().in_zone(source_zone);
                    union.any_of = vec![capture_filter(chosen, source_zone), capture_filter(other, source_zone)];
                    vec![EffectAst::subject_verb_move_all_to_zone(
                        TargetAst::Object(union, None, None), Zone::Graveyard, false,
                        ReturnControllerAst::Preserve, false, None,
                    ).with_tagged_destinations(vec![(chosen.clone(), Zone::Hand), (other.clone(), Zone::Graveyard)])]
                }
                BinaryPileDestination::ChosenToGraveyardCastFromOtherRestToHand => vec![
                    move_capture(chosen, source_zone, Zone::Graveyard),
                    EffectAst::subject_verb_look_at_objects(
                        PlayerAst::You, capture_filter(other, source_zone),
                    ),
                    // "You may cast a spell from among them without paying
                    // its mana cost": an optional single cast during
                    // resolution (CR 608.2g, 118.9).
                    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                        filter: capture_filter(other, source_zone).without_type(crate::types::CardType::Land),
                        count: ChoiceCount::up_to(1),
                        player: PlayerAst::You,
                        tag: selected.clone(),
                        zone: source_zone,
                    }),
                    EffectAst::subject_verb_cast_tagged(
                        selected.clone(), PlayerAst::You, false, false, true, None,
                    ),
                    // The cast spell has left exile; the rest are the pile's
                    // remaining exiled incarnations.
                    move_capture(other, source_zone, Zone::Hand),
                ],
                BinaryPileDestination::ChosenExiledOtherToBattlefield => vec![
                    move_capture(chosen, source_zone, Zone::Exile),
                    move_capture(other, source_zone, Zone::Battlefield),
                ],
                BinaryPileDestination::OneToHandAndPoolToBottom => vec![
                    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                        filter: capture_filter(chosen, source_zone), count: ChoiceCount::exactly(1),
                        count_value: None, player: PlayerAst::You, tag: selected.clone(),
                        zones: vec![Zone::Library], search_mode: None,
                    }),
                    // "All other cards revealed" includes the unchosen pile.
                    EffectAst::subject_verb_tag_matching_objects(
                        capture_filter(&pool, source_zone).not_tagged(selected.clone()),
                        vec![Zone::Library], remainder.clone(),
                    ),
                    move_capture(&selected, source_zone, Zone::Hand),
                    move_capture(&remainder, source_zone, Zone::Library).with_library_order(
                        Some(crate::cards::builders::LibraryBottomOrderAst::ChooserChooses),
                        PlayerAst::You,
                    ),
                ],
            };
            crate::cards::builders::ChooseOneModeAst {
                description: if matches!(
                    program.producer,
                    BinaryPileProducer::FaceDownThenFaceUpExile { .. }
                ) {
                    if index == 0 { "Choose the face-down pile".to_string() } else { "Choose the face-up pile".to_string() }
                } else if program.reveal_pool || source_zone == Zone::Exile {
                    format!("Choose pile {}", index + 1)
                } else if index == 0 {
                    "Choose the face-down pile".to_string()
                } else {
                    "Choose the face-up pile".to_string()
                },
                effects,
            }
        }).collect();
    effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { chooser, modes }));
    // Every remaining sentence must compile; never claim only a pile prefix.
    let mut preceding_pile = program.destination == BinaryPileDestination::HandAndGraveyard;
    for sentence in &sentences[program.consumed_sentences..] {
        let mut continuation = parse_effect_sentence_sequence(sentence.lowered())?;
        bind_destination_counter_results(&mut continuation, &mut preceding_pile);
        effects.extend(continuation);
    }
    Ok(effects)
}
