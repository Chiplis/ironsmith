use super::*;
use crate::lexer::{lex_line, split_lexed_sentences};

#[test]
fn opponent_revealed_choice_tags_the_filtered_selection_and_exact_remainder() {
    let tokens = lex_line(
            "Reveal the top five cards of your library. An opponent chooses a creature card from among them. Put that card onto the battlefield and the rest into your graveyard.",
            0,
        )
        .expect("opponent partition should lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    let effects = parse_reveal_top_opponent_chooses_then_partition(&sentences, 0)
        .expect("parse")
        .expect("typed opponent partition");

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtTopCards {
                    tag: revealed,
                    reveal: true,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ChoosePlayer {
                    filter: PlayerFilter::Opponent,
                    ..
                },
            ..
        }),
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            tag: selected,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TagMatchingObjects {
                    filter: remainder,
                    tag: remainder_tag,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Battlefield,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Graveyard,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected revealed pool/opponent/selection/complement/moves: {effects:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *revealed && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(remainder.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *revealed && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(remainder.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *selected
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }));
    assert_ne!(selected, remainder_tag);
}

#[test]
fn opponent_exile_partition_reuses_one_explicit_player_choice_for_cast_permission() {
    let tokens = lex_line(
        "Reveal the top six cards of your library. An opponent exiles a nonland card from among them, then you put the rest into your hand. That opponent may cast the exiled card without paying its mana cost.",
        0,
    )
    .expect("opponent exile partition should lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    let effects = parse_reveal_top_opponent_exiles_one_put_rest_hand_then_may_cast(&sentences, 0)
        .expect("opponent exile partition should parse")
        .expect("opponent exile partition should match");

    assert!(
        matches!(
            effects.as_slice(),
            [
                _,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ChoosePlayer {
                        filter: PlayerFilter::Opponent,
                        ..
                    },
                    ..
                }),
                EffectAst::ChooseTaggedObjectsInZone {
                    player: PlayerAst::That,
                    ..
                },
                _,
                _,
                EffectAst::MayByPlayer {
                    player: PlayerAst::That,
                    effects
                }
            ] if matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::CastTagged {
                        player: PlayerAst::That,
                        ..
                    },
                    ..
                })]
            )
        ),
        "the selection and permission must share one chosen opponent: {effects:#?}"
    );
}
use crate::types::Subtype;

#[test]
fn historical_block_reanimation_keeps_target_success_and_controller_provenance() {
    let tokens = lex_line(
            "Destroy all creatures that were blocked by target Wall this turn. They can't be regenerated. For each creature that died this way, put a creature card from the graveyard of the player who controlled that creature the last time it became blocked by that Wall onto the battlefield under its owner's control.",
            0,
        )
        .expect("lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    let effects =
        parse_destroy_historically_blocked_then_reanimate_from_historical_controller(&sentences, 0)
            .expect("parse")
            .expect("historical block reanimation");

    let [
        EffectAst::TagAffected {
            effect: target_effect,
            tag: blocker_tag,
        },
        EffectAst::TagAffected {
            effect: destroy_effect,
            tag: destroyed_tag,
        },
        EffectAst::ForEachTaggedWithControllerAtLastBlockedBy {
            tag: loop_tag,
            blocker_tag: historical_blocker_tag,
            effects: reanimate,
        },
    ] = effects.as_slice()
    else {
        panic!("expected target/destroy/historical-controller loop: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TargetOnly {
                target: TargetAst::Object(blocker_filter, Some(_), _),
                explicit_declaration: true,
            },
        ..
    }) = target_effect.as_ref()
    else {
        panic!("expected explicit target blocker: {target_effect:#?}");
    };
    assert!(blocker_filter.subtypes.contains(&Subtype::Wall));

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::DestroyAll {
                filter,
                no_regeneration: true,
                ..
            },
        ..
    }) = destroy_effect.as_ref()
    else {
        panic!("expected no-regeneration destroy: {destroy_effect:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert_eq!(
        filter.blocked_by,
        Some(ObjectRef::Tagged(blocker_tag.clone()))
    );
    assert!(
        !filter.blocked,
        "must use turn history, not current blocking"
    );
    assert_eq!(loop_tag, destroyed_tag);
    assert_eq!(historical_blocker_tag, blocker_tag);

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    target,
                    zone: Zone::Battlefield,
                    battlefield_controller: ReturnControllerAst::Owner,
                    ..
                },
            ..
        }),
    ] = reanimate.as_slice()
    else {
        panic!("expected owner-controlled reanimation: {reanimate:#?}");
    };
    let TargetAst::WithCount(inner, count) = target else {
        panic!("expected exactly one creature card: {target:#?}");
    };
    assert_eq!(*count, ChoiceCount::exactly(1));
    let TargetAst::Object(filter, _, _) = inner.as_ref() else {
        panic!("expected graveyard object filter: {inner:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
    assert!(filter.has_explicit_card_noun());

    let public_effects = effect_sentences::parse_effect_sentences_lexed(&tokens)
        .expect("public sentence dispatcher should select the historical provenance rule");
    assert!(
        matches!(
            public_effects.as_slice(),
            [
                EffectAst::TagAffected { .. },
                EffectAst::TagAffected { .. },
                EffectAst::ForEachTaggedWithControllerAtLastBlockedBy { .. }
            ]
        ),
        "public dispatch bypassed the exact three-sentence rule: {public_effects:#?}"
    );
}

#[test]
fn historical_block_reanimation_rejects_unlinked_controller_wording() {
    let tokens = lex_line(
            "Destroy all creatures that were blocked by target Wall this turn. They can't be regenerated. For each creature that died this way, put a creature card from its controller's graveyard onto the battlefield under its owner's control.",
            0,
        )
        .expect("lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    assert!(
            parse_destroy_historically_blocked_then_reanimate_from_historical_controller(
                &sentences, 0,
            )
            .expect("parse")
            .is_none()
        );
}

#[test]
fn looked_any_number_battlefield_then_shuffle_keeps_one_tagged_pool() {
    let tokens = lex_line(
            "Look at the top X cards of your library, where X is your life total. You may put any number of nonland permanent cards with mana value 3 or less from among them onto the battlefield. Then shuffle.",
            0,
        )
        .expect("lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    let effects = parse_look_at_top_put_matching_onto_battlefield_then_shuffle(&sentences, 0)
        .expect("parse")
        .expect("looked battlefield/shuffle program");

    let [look, choose, move_each, shuffle] = effects.as_slice() else {
        panic!("expected look/choose/move/shuffle program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards { tag: looked, .. },
        ..
    }) = look
    else {
        panic!("expected looked-card producer: {look:#?}");
    };
    let EffectAst::ChooseTaggedObjectsInZone {
        filter,
        count,
        tag: chosen,
        zone: Zone::Library,
        ..
    } = choose
    else {
        panic!("expected tagged looked-card choice: {choose:#?}");
    };
    assert!(count.is_any_number());
    assert!(filter.excluded_card_types.contains(&CardType::Land));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *looked && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(matches!(
        move_each,
        EffectAst::ForEachTagged { tag, effects }
            if tag == chosen
                && matches!(
                    effects.as_slice(),
                    [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::MoveToZone {
                            zone: Zone::Battlefield,
                            ..
                        },
                        ..
                    })]
                )
    ));
    assert!(matches!(
        shuffle,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ShuffleLibrary,
            ..
        })
    ));
}

#[test]
fn looked_reveal_to_hand_preserves_the_authored_singular_reference() {
    for (reference, expected) in [
        ("it", ironsmith_core::SearchResultReferenceSurface::It),
        (
            "that card",
            ironsmith_core::SearchResultReferenceSurface::ThatCard,
        ),
    ] {
        let text = format!(
            "Look at the top five cards of your library. You may reveal a creature card from among them and put {reference} into your hand. Put the rest on the bottom of your library in a random order."
        );
        let tokens = lex_line(&text, 0).expect("looked partition should lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();
        let effects = parse_look_at_top_reveal_match_put_rest_bottom(&sentences, 0)
            .expect("parse")
            .expect("typed looked partition");
        let EffectAst::ForEachTagged { effects: moved, .. } = &effects[3] else {
            panic!("expected tagged selected-card move: {effects:#?}");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::MoveToZone {
                        target_reference_surface,
                        ..
                    },
                ..
            }),
        ] = moved.as_slice()
        else {
            panic!("expected one selected-card move: {moved:#?}");
        };
        assert_eq!(*target_reference_surface, Some(expected));
    }
}

#[test]
fn optional_top_selection_and_separate_remainder_share_one_looked_pool() {
    let tokens = lex_line(
            "Look at the top X cards of your library, where X is the number of basic land types among lands you control. You may put one of those cards on top of your library. Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    let effects = parse_look_at_top_then_optional_one_top_then_remainder_bottom(&sentences, 0)
        .expect("parse")
        .expect("optional top/remainder partition");

    let [look_effect, may_effect, remainder_effect] = effects.as_slice() else {
        panic!("expected look/may/remainder program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::LookAtTopCards {
                count,
                tag: looked_tag,
                ..
            },
        ..
    }) = look_effect
    else {
        panic!("expected looked-card provenance: {look_effect:#?}");
    };
    assert!(matches!(count.unhinted(), Value::BasicLandTypesAmong(_)));
    let EffectAst::May { effects: optional } = may_effect else {
        panic!("expected explicit optional branch: {may_effect:#?}");
    };
    let [
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            player,
            tag: selected_tag,
            zone,
        },
        move_effect,
    ] = optional.as_slice()
    else {
        panic!("expected exact singleton selection and move: {optional:#?}");
    };
    assert_eq!(*count, ChoiceCount::exactly(1));
    assert_eq!(*player, PlayerAst::You);
    assert_eq!(*zone, Zone::Library);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(matches!(
        move_effect,
        EffectAst::ForEachTagged { tag, effects }
            if tag == selected_tag
                && matches!(
                    effects.as_slice(),
                    [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::MoveToZone {
                            zone: Zone::Library,
                            to_top: true,
                            ..
                        },
                        ..
                    })]
                )
    ));
    assert!(matches!(
        remainder_effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    order: crate::cards::builders::LibraryBottomOrderAst::Random,
                    player: PlayerAst::You,
                    ..
                },
            ..
        }) if tag == looked_tag && keep_tagged == selected_tag
    ));
}

#[test]
fn same_name_permanent_selection_has_two_explicit_candidate_domains() {
    let tokens = lex_line(
            "Look at the top seven cards of your library. You may put one of those cards onto the battlefield if it has the same name as a permanent. Put the rest on the bottom of your library in any order.",
            0,
        )
        .expect("lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    let effects = parse_look_at_top_may_put_same_name_as_permanent_rest_bottom(&sentences, 0)
        .expect("parse")
        .expect("same-name looked-card program");
    let debug = format!("{effects:#?}");
    let [_, tag_comparison, _, _] = effects.as_slice() else {
        panic!("expected look/comparison/optional move/remainder: {debug}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::TagMatchingObjects { filter, zones, .. },
        ..
    }) = tag_comparison
    else {
        panic!("expected permanent comparison-set tag: {debug}");
    };
    assert_eq!(zones, &[Zone::Battlefield]);
    assert_eq!(filter.zone, Some(Zone::Battlefield), "{debug}");
    assert!(debug.contains("SameNameAsTagged"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn composes_compound_looked_exile_remainder_and_cast_sequence() {
    let tokens = lex_line(
            "Look at that many cards from the top of your library. Exile up to one nonland card from among them and put the rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost.",
            0,
        )
        .expect("lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();
    let effects = parse_look_at_top_exile_match_and_rest_bottom_then_cast_exiled(&sentences, 0)
        .expect("parse")
        .expect("compound looked-card shape");

    assert_eq!(effects.len(), 5);
    assert!(matches!(
        &effects[0],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LookAtTopCards {
                count: Value::EventValue(crate::effect::EventValueSpec::Amount),
                ..
            },
            ..
        })
    ));
    let EffectAst::ChooseTaggedObjectsInZone {
        filter, count, tag, ..
    } = &effects[1]
    else {
        panic!("expected typed looked-card choice: {:#?}", effects[1]);
    };
    assert_eq!(*count, ChoiceCount::up_to(1));
    assert!(
        tag.as_str().starts_with("__sentence_helper_exiled_up_to_"),
        "the compound up-to surface must survive lowering: {tag:?}"
    );
    assert!(filter.excluded_card_types.contains(&CardType::Land));
    assert!(matches!(
        &effects[3],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
            ..
        })
    ));
    assert!(matches!(
        &effects[4],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CastTagged {
                without_paying_mana_cost: true,
                ..
            },
            ..
        })
    ));
}

#[test]
fn consult_cleanup_reflexive_keeps_variable_damage_and_full_set_cleanup() {
    let tokens = lex_line(
            "Reveal cards from the top of your library until you reveal a nonland card. Put the revealed cards on the bottom of your library in a random order. When you reveal a nonland card this way, this deals damage equal to that card's mana value to any target.",
            0,
        )
        .expect("lex");
    let split = split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();

    let effects = parse_consult_cleanup_then_typed_when_result(&sentences, 0)
        .expect("parse")
        .expect("consult/cleanup/reflexive shape");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
                    all_tag, match_tag, ..
                },
            ..
        }),
        EffectAst::WhenResult {
            effects: reflexive, ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag, keep_tagged, ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected consult, reflexive result, and cleanup: {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamage { amount, target, .. },
            ..
        }),
    ] = reflexive.as_slice()
    else {
        panic!("expected variable reflexive damage: {reflexive:#?}");
    };

    assert!(matches!(amount.unhinted(), Value::ManaValueOf(_)));
    assert!(matches!(target, TargetAst::AnyTarget(_)));
    assert_ne!(all_tag, match_tag);
    assert_eq!(tag.as_str(), "__last_revealed__");
    assert!(keep_tagged.is_none());
}
