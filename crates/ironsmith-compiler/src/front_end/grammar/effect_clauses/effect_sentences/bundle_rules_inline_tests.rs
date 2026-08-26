use super::*;
use crate::lexer::lex_line;

#[test]
fn each_opponent_hand_exile_keeps_permission_tax_and_land_entry_linked() {
    let tokens = lex_line(
            "Each opponent exiles a card from their hand and may play that card for as long as it remains exiled. Each spell cast this way costs {1} more to cast. Each land played this way enters tapped.",
            0,
        )
        .unwrap();
    let effects =
        parse_typed_effect_bundle_lexed(&tokens).expect("linked each-opponent hand exile bundle");
    let [
        EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::Opponent,
            effects: per_player,
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag: grant_tag,
                    player: PlayerAst::ItsOwner,
                    spell_cost_increase: Some(cost),
                    lands_enter_tapped: true,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected correlated exile and constrained play grant: {effects:#?}");
    };
    let [
        EffectAst::ChooseObjects {
            tag: chosen_tag,
            filter,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target: TargetAst::Tagged(exile_tag, None),
                    ..
                },
            ..
        }),
    ] = per_player.as_slice()
    else {
        panic!("expected per-player choose/exile pair: {per_player:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
    assert_eq!(chosen_tag, exile_tag);
    assert_eq!(chosen_tag, grant_tag);
    assert_eq!(cost.to_oracle(), "{1}");
}

fn conditional_mana_value_limit(effect: &EffectAst) -> Option<i32> {
    let EffectAst::Conditional {
        predicate: PredicateAst::ItMatches(filter),
        if_true,
        if_false,
    } = effect
    else {
        return None;
    };
    if !if_false.is_empty()
        || !matches!(
            if_true.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Counter {
                    target: TargetAst::Spell(_),
                },
                ..
            })]
        )
    {
        return None;
    }
    match filter.mana_value.as_ref() {
        Some(crate::target::Comparison::LessThanOrEqual(limit)) => Some(*limit),
        _ => None,
    }
}

#[test]
fn per_player_type_choice_phase_out_keeps_one_shared_card_type() {
    let tokens = lex_line(
            "That player chooses artifact, creature, land, or non-Aura enchantment. All nontoken permanents of that type phase out.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).expect("typed choice/phase-out bundle");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                crate::cards::builders::SubjectVerbSubjectAst {
                    player: PlayerAst::That,
                    ..
                },
            action: SubjectVerbActionAst::ChooseCardType { options },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PhaseOutAll { filter, .. },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected typed card-type choice and phase-out pair: {effects:#?}");
    };

    assert_eq!(
        options,
        &[
            CardType::Artifact,
            CardType::Creature,
            CardType::Land,
            CardType::Enchantment,
        ]
    );
    assert!(filter.nontoken);
    assert!(filter.chosen_card_type);
    assert!(!filter.chosen_creature_type);
    assert!(filter.card_types.is_empty());
    assert_eq!(filter.excluded_subtypes, [crate::types::Subtype::Aura]);
    assert!(filter.tagged_constraints.is_empty());
    assert!(filter.controller.is_none());
}

#[test]
fn mixed_target_collection_reuses_one_complete_consult_procedure_per_target() {
    let tokens = lex_line(
            "Choose any number of target players or planeswalkers. For each of them, reveal cards from the top of your library until you reveal a nonland card, this spell deals damage equal to that card's mana value to that player or planeswalker, then you put the revealed cards on the bottom of your library in any order.",
            0,
        )
        .unwrap();
    let sentences = split_lexed_sentences(&tokens);
    assert_eq!(
        sentences.len(),
        2,
        "mixed target bundle sentences: {sentences:#?}"
    );
    parse_choose_mixed_targets_then_for_each_bundle(sentences[0], sentences[1], None)
        .expect("mixed target consult grammar should parse")
        .expect("mixed target consult grammar should match");
    let effects = parse_typed_effect_bundle_lexed(&tokens).expect("mixed target consult bundle");
    let [
        EffectAst::TagAffected {
            effect: declaration,
            tag: object_targets,
        },
        EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::AliasedTarget(player_filter),
            effects: player_body,
        },
        EffectAst::ForEachTagged {
            tag: tagged_targets,
            effects: object_body,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one declaration and disjoint player/object loops: {effects:#?}");
    };
    assert_eq!(player_filter.as_ref(), &PlayerFilter::Any);
    assert_eq!(object_targets, tagged_targets);
    assert!(matches!(
        declaration.as_ref(),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TargetOnly {
                    target: TargetAst::WithCount(inner, count),
                    explicit_declaration: true,
                },
            ..
        }) if matches!(
            inner.as_ref(),
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, _)
        ) && count == &ChoiceCount::any_number()
    ));
    fn consult_procedure<'a>(
        body: &'a [EffectAst],
        label: &str,
    ) -> (&'a EffectAst, &'a EffectAst, &'a EffectAst) {
        let [EffectAst::Coordination(procedure)] = body else {
            panic!("expected one authored consult procedure per {label}: {body:#?}");
        };
        let [consult_member, tail_member] = procedure.members.as_slice() else {
            panic!("expected consult and result members: {procedure:#?}");
        };
        assert!(matches!(
            procedure.boundaries.as_slice(),
            [crate::model::CoordinationBoundaryAst {
                operator: crate::model::CoordinationOperatorAst::CommaThen,
                ordering: crate::model::EffectOrderingAst::Ordered,
                ..
            }]
        ));
        let [consult] = consult_member.effects.as_slice() else {
            panic!("expected one consult effect: {consult_member:#?}");
        };
        let [EffectAst::Coordination(tail)] = tail_member.effects.as_slice() else {
            panic!("expected ordered damage/disposition tail: {tail_member:#?}");
        };
        let [damage_member, disposition_member] = tail.members.as_slice() else {
            panic!("expected damage and disposition members: {tail:#?}");
        };
        let [damage] = damage_member.effects.as_slice() else {
            panic!("expected one damage effect: {damage_member:#?}");
        };
        let [disposition] = disposition_member.effects.as_slice() else {
            panic!("expected one disposition effect: {disposition_member:#?}");
        };
        (consult, damage, disposition)
    }

    let (player_consult, player_damage, player_disposition) =
        consult_procedure(player_body, "player");
    assert!(matches!(
        player_consult,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary { .. },
            ..
        })
    ));
    assert!(matches!(
        player_damage,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamage {
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, _),
                ..
            },
            ..
        })
    ));
    assert!(matches!(
        player_disposition,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                keep_tagged: None,
                ..
            },
            ..
        })
    ));

    let (object_consult, object_damage, object_disposition) =
        consult_procedure(object_body, "planeswalker");
    assert!(matches!(
        object_consult,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary { .. },
            ..
        })
    ));
    assert!(matches!(
        object_damage,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamage {
                target: TargetAst::Tagged(tag, _),
                ..
            },
            ..
        }) if tag.as_str() == IT_TAG
    ));
    assert!(matches!(
        object_disposition,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                keep_tagged: None,
                ..
            },
            ..
        })
    ));
}

#[test]
fn conditional_controller_sacrifice_consult_keeps_result_and_object_provenance() {
    let tokens = lex_line(
            "Target artifact's controller sacrifices it. If the player does, they reveal cards from the top of their library until they reveal an artifact card that shares a card type with the sacrificed artifact, put that card onto the battlefield, then shuffle.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).expect("conditional consult bundle");
    let [
        EffectAst::TagAffected {
            effect: sacrifice,
            tag: sacrificed_tag,
        },
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: followups,
        },
    ] = effects.as_slice()
    else {
        panic!("expected tagged sacrifice and result-gated consult, got {effects:#?}");
    };
    assert!(matches!(
        sacrifice.as_ref(),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Sacrifice {
                target: Some(_),
                ..
            },
            ..
        })
    ));
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
                    filter: match_filter,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::MoveToZone { .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ShuffleLibrary,
            ..
        }),
    ] = followups.as_slice()
    else {
        panic!("expected consult, move, and shuffle followups, got {followups:#?}");
    };
    assert_eq!(
        match_filter
            .tagged_constraints
            .iter()
            .filter(|constraint| {
                constraint.tag == *sacrificed_tag
                    && constraint.relation == TaggedOpbjectRelation::SharesCardType
            })
            .count(),
        1
    );
}

#[test]
fn inline_look_face_down_exile_permission_uses_one_collection_tag() {
    let tokens = lex_line(
            "Look at the top card of that player's library, exile it face down, then you may play that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).unwrap_or_else(|| {
            let segments = crate::grammar::primitives::split_lexed_slices_on_comma(&tokens);
            let look = effect_sentences::parse_effect_sentence_lexed(&trim_commas(segments[0]));
            let exile = effect_sentences::parse_effect_sentence_lexed(&trim_commas(segments[1]));
            let mut permission_tokens = Vec::new();
            for segment in &segments[2..] {
                permission_tokens.extend_from_slice(&trim_commas(segment));
            }
            let permission = parse_cast_or_play_tagged_clause(&permission_tokens);
            panic!(
                "inline bundle did not match; segments={segments:#?}\nlook={look:#?}\nexile={exile:#?}\npermission={permission:#?}"
            )
        });
    let debug = format!("{effects:#?}");
    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("face_down: true"), "{debug}");
    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(
        debug.contains("AnyColor") || debug.contains("AnyType"),
        "{debug}"
    );
}

#[test]
fn kicked_counter_bundle_builds_self_replacement_ast_before_lowering() {
    let tokens = lex_line(
            "Counter target spell if its mana value is 3 or less. If this spell was kicked, counter that spell if its mana value is 7 or less instead.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).unwrap();
    let [
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            attach_to_previous_ability,
        },
    ] = effects.as_slice()
    else {
        panic!("expected a typed self-replacement AST, got {effects:#?}");
    };

    assert_eq!(predicate, &PredicateAst::ThisSpellWasKicked);
    assert!(!*attach_to_previous_ability);
    assert_eq!(
        if_false.first().and_then(conditional_mana_value_limit),
        Some(3)
    );
    assert_eq!(
        if_true.first().and_then(conditional_mana_value_limit),
        Some(7)
    );
}

#[test]
fn inline_exile_top_then_put_binds_the_exact_exiled_collection() {
    let tokens = lex_line(
            "Exile the top seven cards of that player's library, then put a creature card from among them onto the battlefield under your control.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens)
        .expect("inline exile-top collection bundle should parse");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ExileTopOfLibrary { count, tags, .. },
            ..
        }),
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: choice_count,
            tag: chosen_tag,
            zone,
            ..
        },
        EffectAst::ForEachTagged {
            tag: loop_tag,
            effects: put_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected exile/choose/put typed bundle, got {effects:#?}");
    };
    assert_eq!(count, &Value::Fixed(7));
    assert_eq!(choice_count, &ChoiceCount::exactly(1));
    assert_eq!(zone, &Zone::Exile);
    assert_eq!(chosen_tag, loop_tag);
    assert!(matches!(
        put_effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutOntoBattlefield {
                controller: ReturnControllerAst::You,
                ..
            },
            ..
        })]
    ));
    assert!(filter.card_types.contains(&CardType::Creature));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        tags.first() == Some(&constraint.tag)
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
}

#[test]
fn exile_top_bundle_preserves_source_exile_permission_duration() {
    let tokens = lex_line(
            "Exile the top card of your library. You may play that card until you exile another card with this enchantment.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens)
        .expect("source-exile-bounded permission bundle should parse");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ExileTopOfLibrary { tags, .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    tag,
                    until_source_exiles_another: true,
                    surface: Some(surface),
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected linked exile/grant bundle, got {effects:#?}");
    };
    assert_eq!(tags.first(), Some(tag));
    assert_eq!(
        surface
            .until_source_exiles_another
            .as_ref()
            .map(ironsmith_core::SourceReferenceSurface::display_text)
            .as_deref(),
        Some("this enchantment")
    );
}

#[test]
fn inline_exile_top_choose_one_rebinds_the_play_permission() {
    let tokens = lex_line(
            "Exile the top two cards of your library, then choose one of them. You may play that card this turn.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens)
        .expect("inline choose-one exile permission should parse");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ExileTopOfLibrary { count, tags, .. },
            ..
        }),
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: choice_count,
            tag: chosen_tag,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: permission_tag,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected exile/choose/permission typed bundle, got {effects:#?}");
    };
    assert_eq!(count, &Value::Fixed(2));
    assert_eq!(choice_count, &ChoiceCount::exactly(1));
    assert_eq!(chosen_tag, permission_tag);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        tags.first() == Some(&constraint.tag)
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
}

#[test]
fn optional_result_exile_choice_rebinds_the_trailing_play_permission() {
    let tokens = lex_line(
            "You may discard a card. If you do, exile the top two cards of your library, then choose one of them. You may play that card this turn.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens)
        .expect("optional result-gated exile choice should parse as one linked bundle");
    let [
        EffectAst::May { .. } | EffectAst::MayByPlayer { .. },
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: linked,
        },
    ] = effects.as_slice()
    else {
        panic!("expected optional action plus result-gated linked program, got {effects:#?}");
    };
    assert!(
        matches!(
            linked.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ExileTopOfLibrary { count, .. },
                    ..
                }),
                EffectAst::ChooseTaggedObjectsInZone { tag: chosen, .. },
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                            tag: permission,
                            surface: Some(surface),
                            ..
                        },
                    ..
                }),
            ] if count == &Value::Fixed(2)
                && chosen == permission
                && !surface.leading_duration
        ),
        "the choice and trailing permission must share one exact exiled-card tag and surface: {linked:#?}"
    );
}

#[test]
fn shuffle_prefix_stays_in_the_exile_top_free_play_bundle() {
    let tokens = lex_line(
            "Shuffle your library, then exile the top card. Until end of turn, you may play that card without paying its mana cost.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens)
        .expect("shuffle/exile/free-play bundle should parse");
    assert!(
        matches!(
            effects.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ShuffleLibrary,
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ExileTopOfLibrary { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                        without_paying_mana_cost: true,
                        ..
                    },
                    ..
                }),
            ]
        ),
        "expected typed shuffle/exile/free-play sequence, got {effects:#?}"
    );
}

#[test]
fn selected_hand_double_choice_builds_distinct_filters_with_one_accumulating_tag() {
    let tokens = lex_line(
            "Target opponent reveals their hand. You choose from it a nonland card with mana value 3 or less and a card with mana value 4 or greater. That player discards those cards.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).expect("selected-hand bundle");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RevealHand,
            ..
        }),
        EffectAst::ChooseObjects {
            filter: first_filter,
            count: first_count,
            tag: first_tag,
            ..
        },
        EffectAst::ChooseObjects {
            filter: second_filter,
            count: second_count,
            tag: second_tag,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Discard {
                    count: Value::Count(discard_filter),
                    filter: Some(card_filter),
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected reveal, two choices, and one tagged discard, got {effects:#?}");
    };

    assert_eq!(first_count, &ChoiceCount::exactly(1));
    assert_eq!(second_count, &ChoiceCount::exactly(1));
    assert_eq!(first_tag, second_tag);
    assert!(first_filter.excluded_card_types.contains(&CardType::Land));
    assert!(matches!(
        first_filter.mana_value.as_ref(),
        Some(crate::target::Comparison::LessThanOrEqual(3))
    ));
    assert!(second_filter.excluded_card_types.is_empty());
    assert!(matches!(
        second_filter.mana_value.as_ref(),
        Some(crate::target::Comparison::GreaterThanOrEqual(4))
    ));
    for filter in [discard_filter, card_filter] {
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && &constraint.tag == first_tag
        }));
    }
}

#[test]
fn each_opponent_top_card_permission_preserves_the_accumulated_collection() {
    let tokens = lex_line(
            "Exile the top card of each opponent's library face down. You may look at and play those cards for as long as they remain exiled.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).expect("exile/permission bundle");
    let [
        EffectAst::ForEachOpponent {
            effects: exile_each,
        },
        permission,
    ] = effects.as_slice()
    else {
        panic!("expected each-opponent exile plus shared permission, got {effects:#?}");
    };
    let [
        EffectAst::ChooseObjectsTopOfLibrary {
            player: PlayerAst::You,
            ..
        },
        EffectAst::TagAffected {
            tag: collection_tag,
            ..
        },
    ] = exile_each.as_slice()
    else {
        panic!("expected typed top-library exile, got {exile_each:#?}");
    };
    assert!(matches!(
        permission,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { tag, .. },
            ..
        }) if tag == collection_tag
    ));
}

#[test]
fn hidden_exile_partition_uses_one_tag_for_choice_remainder_and_permission() {
    let tokens = lex_line(
            "Look at the top two cards of target opponent's library. Exile one of them face down and put the other on the bottom of that library. You may play the exiled card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).expect("hidden-exile bundle");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LookAtTopCards { .. },
            ..
        }),
        EffectAst::ChooseObjects {
            tag: selected_tag, ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target: TargetAst::Tagged(exile_tag, None),
                    face_down: true,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    keep_tagged: Some(kept_tag),
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag: permission_tag,
                    allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::AnyColor,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected linked hidden-exile partition, got {effects:#?}");
    };

    assert_eq!(selected_tag, exile_tag);
    assert_eq!(selected_tag, kept_tag);
    assert_eq!(selected_tag, permission_tag);
}

#[test]
fn looked_hand_exile_permission_tax_stays_in_one_linked_program() {
    let tokens = lex_line(
            "Look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {2} more to cast.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).expect("linked hand-exile bundle");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LookAtHand { .. },
            ..
        }),
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects: optional_exile,
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag: permission_tag,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantToTarget {
                    target: TargetAst::Tagged(tax_tag, None),
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one linked hand-exile program, got {effects:#?}");
    };
    let [
        EffectAst::ChooseObjects {
            filter,
            player: PlayerAst::You,
            tag: choice_tag,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target: TargetAst::Tagged(exile_tag, None),
                    face_down: false,
                    ..
                },
            ..
        }),
    ] = optional_exile.as_slice()
    else {
        panic!("expected an optional typed choose/exile pair, got {optional_exile:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert!(matches!(&filter.owner, Some(PlayerFilter::Target(_))));
    assert_eq!(choice_tag, exile_tag);
    assert_eq!(choice_tag, permission_tag);
    assert_eq!(choice_tag, tax_tag);
}

#[test]
fn inline_mill_then_optional_filtered_return_keeps_one_milled_collection() {
    let tokens = lex_line(
            "Mill three cards, then you may put an artifact or land card from among the milled cards into your hand.",
            0,
        )
        .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).expect("inline mill bundle");
    let [
        EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected an authored inline sequence boundary, got {effects:#?}");
    };
    let [
        EffectAst::TagAffected {
            tag: milled_tag,
            effect: mill,
        },
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            tag: chosen_tag,
            ..
        },
        EffectAst::ForEachTagged {
            tag: moved_tag,
            effects: move_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected linked mill, choice, and move program, got {effects:#?}");
    };

    assert!(matches!(
        mill.as_ref(),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mill {
                count: Value::Fixed(3),
            },
            ..
        })
    ));
    assert_eq!(count, &ChoiceCount::up_to(1));
    assert_eq!(chosen_tag, moved_tag);
    assert_eq!(
        filter.prior_effect_action_surface(),
        Some(ironsmith_core::PriorEffectAction::Milled)
    );
    for card_type in [CardType::Artifact, CardType::Land] {
        assert!(
            filter.card_types.contains(&card_type)
                || filter
                    .any_of
                    .iter()
                    .any(|branch| branch.card_types.contains(&card_type)),
            "{filter:#?}"
        );
    }
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *milled_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(matches!(
        move_effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::MoveToZone {
                zone: Zone::Hand,
                ..
            },
            ..
        })]
    ));
}

#[test]
fn inline_mill_then_return_from_among_them_keeps_one_milled_collection() {
    let tokens = lex_line(
        "Mill four cards, then you may return a permanent card from among them to your hand.",
        0,
    )
    .unwrap();
    let effects = parse_typed_effect_bundle_lexed(&tokens).expect("inline mill bundle");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("TagAffected"), "{debug}");
    assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
    assert!(debug.contains("zone: Graveyard"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
    assert!(debug.contains("zone: Hand"), "{debug}");
}
