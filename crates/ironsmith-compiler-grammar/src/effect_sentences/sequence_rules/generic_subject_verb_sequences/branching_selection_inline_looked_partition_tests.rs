use super::*;
use crate::lexer::lex_line;

#[test]
fn conditional_instead_count_keeps_two_exact_optional_looked_partitions() {
    let raw = [
        "Look at the top four cards of your library",
        "You may reveal a creature or land card from among them and put it into your hand",
        "If you gained life this turn, you may instead reveal two creature and/or land cards from among them and put them into your hand",
        "Put the rest on the bottom of your library in a random order",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    let effects = parse_look_reveal_one_or_instead_two_then_rest_bottom(&sentences, 0)
        .expect("count replacement parser should not error")
        .expect("count replacement partition should parse");
    let [
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            attach_to_previous_ability: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one complete self-replacement: {effects:#?}");
    };
    assert!(matches!(
        predicate,
        PredicateAst::PlayerGainedLifeThisTurnOrMore {
            player: PlayerAst::You,
            count: 1,
        }
    ));

    let assert_branch = |branch: &[EffectAst], expected_count| {
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::LookAtTopCards {
                        tag: looked_tag, ..
                    },
                ..
            }),
            EffectAst::May { effects: optional },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag,
                        keep_tagged: Some(keep_tagged),
                        order: LibraryBottomOrderAst::Random,
                        ..
                    },
                ..
            }),
        ] = branch
        else {
            panic!("expected look/may/exact-remainder branch: {branch:#?}");
        };
        let [
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count,
                tag: selected_tag,
                zone: Zone::Library,
                ..
            },
            EffectAst::ForEachTagged {
                tag: revealed_tag, ..
            },
            EffectAst::ForEachTagged { tag: moved_tag, .. },
        ] = optional.as_slice()
        else {
            panic!("expected exact choose/reveal/move optional body: {optional:#?}");
        };
        assert_eq!(*count, ChoiceCount::exactly(expected_count));
        assert_eq!(revealed_tag, selected_tag);
        assert_eq!(moved_tag, selected_tag);
        assert_eq!(tag, looked_tag);
        assert_eq!(keep_tagged, selected_tag);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *looked_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
    };
    assert_branch(if_false, 1);
    assert_branch(if_true, 2);
}

#[test]
fn discover_cast_condition_describes_the_exiled_card_not_a_stack_object() {
    let tokens = lex_line(
            "You may cast the exiled card without paying its mana cost if it's an instant spell with mana value 2 or less",
            0,
        )
        .expect("cast condition should lex");

    let filter = parse_exiled_card_cast_filter(&tokens)
        .expect("cast condition should not error")
        .expect("cast condition should parse");

    assert_eq!(filter.zone, None);
    assert_eq!(filter.stack_kind, None);
    assert_eq!(filter.card_types, vec![crate::types::CardType::Instant]);
    assert_eq!(
        filter.mana_value,
        Some(crate::filter::Comparison::LessThanOrEqual(2))
    );
}

#[test]
fn two_optional_selections_leave_an_exact_three_tag_graveyard_complement() {
    let raw = [
        "Reveal the top six cards of your library",
        "You may put a permanent card from among them onto the battlefield with an indestructible counter on it",
        "You may put a permanent card from among them into your hand",
        "Put the rest into your graveyard",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    let effects = parse_reveal_top_optional_battlefield_then_hand_rest_graveyard(&sentences, 0)
        .expect("two-stage partition parser should not error")
        .expect("two-stage looked-card partition should parse");
    let [
        look,
        battlefield_choice,
        battlefield_move,
        hand_choice,
        hand_move,
        tag_remainder,
        move_remainder,
    ] = effects.as_slice()
    else {
        panic!("expected one two-choice exact-complement program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::LookAtTopCards {
                tag: looked_tag,
                reveal: true,
                ..
            },
        ..
    }) = look
    else {
        panic!("expected public looked-card producer: {look:#?}");
    };
    let EffectAst::ChooseTaggedObjectsInZone {
        filter: battlefield_filter,
        tag: battlefield_tag,
        count: battlefield_count,
        ..
    } = battlefield_choice
    else {
        panic!("expected battlefield selection: {battlefield_choice:#?}");
    };
    assert_eq!(*battlefield_count, ChoiceCount::up_to(1));
    assert!(
        battlefield_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == *looked_tag
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            })
    );
    assert!(matches!(
        battlefield_move,
        EffectAst::ForEachTagged { tag, effects }
            if tag == battlefield_tag
                && effects.iter().any(|effect| matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::PutCounters {
                            counter_type: crate::object::CounterType::Indestructible,
                            ..
                        },
                        ..
                    })
                ))
    ));
    let EffectAst::ChooseTaggedObjectsInZone {
        filter: hand_filter,
        tag: hand_tag,
        count: hand_count,
        ..
    } = hand_choice
    else {
        panic!("expected hand selection: {hand_choice:#?}");
    };
    assert_eq!(*hand_count, ChoiceCount::up_to(1));
    assert!(hand_filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(hand_filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *battlefield_tag
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }));
    assert!(matches!(
        hand_move,
        EffectAst::ForEachTagged { tag, .. } if tag == hand_tag
    ));
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TagMatchingObjects {
                filter,
                tag: remainder_tag,
                ..
            },
        ..
    }) = tag_remainder
    else {
        panic!("expected exact remainder tag: {tag_remainder:#?}");
    };
    for (tag, relation) in [
        (looked_tag, TaggedOpbjectRelation::IsTaggedObject),
        (battlefield_tag, TaggedOpbjectRelation::IsNotTaggedObject),
        (hand_tag, TaggedOpbjectRelation::IsNotTaggedObject),
    ] {
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| { constraint.tag == *tag && constraint.relation == relation })
        );
    }
    assert!(matches!(
        move_remainder,
        EffectAst::MoveTaggedGroupToZone {
            tag,
            zone: Zone::Graveyard,
        } if tag == remainder_tag
    ));
}

#[test]
fn your_turn_destination_branch_keeps_one_selected_card_and_one_remainder() {
    let raw = [
        "Look at the top five cards of your library",
        "You may reveal a creature card with mana value 3 or less from among them",
        "You may put it onto the battlefield if it's your turn",
        "If you don't put it onto the battlefield, put it into your hand",
        "Put the rest on the bottom of your library in a random order",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    assert!(
        effect_sentences::parse_top_cards_view_sentence(sentences[0].lowered()).is_some(),
        "look sentence must retain the producer"
    );
    assert!(
        parse_may_reveal_up_to_from_looked_cards(sentences[1].lowered())
            .expect("reveal choice parser")
            .is_some(),
        "reveal sentence must retain a filtered singleton choice"
    );
    assert!(
        is_may_put_selected_onto_battlefield_on_your_turn(sentences[2].lowered()),
        "battlefield sentence must preserve its your-turn gate"
    );
    assert!(
        is_if_selected_not_put_onto_battlefield_put_into_hand(sentences[3].lowered()),
        "fallback sentence must preserve the selected-card reference"
    );
    assert!(
        triple_grammar::parse_looked_remainder_shape(sentences[4].lowered()).is_some(),
        "remainder sentence must retain the looked-card complement"
    );
    let effects =
        crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
            .expect("your-turn destination parser should not error")
            .expect("your-turn destination partition should parse");
    let [look, choose, reveal, conditional, remainder] = effects.as_slice() else {
        panic!("expected one selected-card destination program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards {
            tag: looked_tag, ..
        },
        ..
    }) = look
    else {
        panic!("expected looked-card producer: {look:#?}");
    };
    let EffectAst::ChooseTaggedObjectsInZone {
        filter,
        tag: selected_tag,
        count,
        ..
    } = choose
    else {
        panic!("expected selected-card choice: {choose:#?}");
    };
    assert_eq!(*count, ChoiceCount::up_to(1));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(matches!(
        reveal,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RevealTagged { tag },
            ..
        }) if tag == selected_tag
    ));
    assert!(matches!(
        conditional,
        EffectAst::Conditional {
            predicate: PredicateAst::YourTurn,
            ..
        }
    ));
    assert!(matches!(
        remainder,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    order: LibraryBottomOrderAst::Random,
                    ..
                },
            ..
        }) if tag == looked_tag && keep_tagged == selected_tag
    ));
}

#[test]
fn unfiltered_optional_exile_uses_one_tag_for_exile_remainder_and_permission() {
    let raw = [
        "Look at the top X cards of your library, where X is the excess damage dealt this way",
        "You may exile one of those cards",
        "Put the rest on the bottom of your library in a random order",
        "You may play the exiled card this turn",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    let effects = crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("optional exile parser should not error")
        .expect("unfiltered optional exile partition should parse");
    let [look, choose, exile, remainder, permission] = effects.as_slice() else {
        panic!("expected looked/exiled/remainder/permission program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards {
            tag: looked_tag, ..
        },
        ..
    }) = look
    else {
        panic!("expected looked-card producer: {look:#?}");
    };
    let EffectAst::ChooseTaggedObjectsInZone {
        filter,
        tag: exiled_tag,
        ..
    } = choose
    else {
        panic!("expected optional exiled-card selection: {choose:#?}");
    };
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(matches!(
        exile,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Exile {
                target: TargetAst::Tagged(tag, _),
                ..
            },
            ..
        }) if tag == exiled_tag
    ));
    assert!(matches!(
        remainder,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    ..
                },
            ..
        }) if tag == looked_tag && keep_tagged == exiled_tag
    ));
    assert!(format!("{permission:#?}").contains(exiled_tag.as_str()));
}

#[test]
fn battlefield_grant_keeps_selected_tag_and_exact_looked_complement() {
    let raw = [
        "Reveal the top four cards of your library",
        "You may put a creature card from among them onto the battlefield",
        "It gains \"At the beginning of your end step, return this creature to its owner's hand.\"",
        "Then put the rest of the cards revealed this way on the bottom of your library in any order",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    let effects = parse_top_cards_move_then_grant_rest_bottom(&sentences, 0)
        .expect("grant partition parser should not error")
        .expect("looked battlefield/grant/remainder shape");
    let [look, choose, move_each, grant, remainder] = effects.as_slice() else {
        panic!("expected look/choose/move/grant/remainder program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::LookAtTopCards {
                reveal: true,
                tag: looked_tag,
                ..
            },
        ..
    }) = look
    else {
        panic!("expected one public reveal producer: {look:#?}");
    };
    let EffectAst::ChooseTaggedObjectsInZone {
        filter,
        count,
        tag: selected_tag,
        ..
    } = choose
    else {
        panic!("expected tagged looked-card choice: {choose:#?}");
    };
    assert_eq!(*count, ChoiceCount::up_to(1));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(matches!(
        move_each,
        EffectAst::ForEachTagged { tag, .. } if tag == selected_tag
    ));
    assert!(matches!(
        grant,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target: TargetAst::Tagged(tag, None),
                    ..
                },
            ..
        }) if tag == selected_tag
    ));
    assert!(matches!(
        remainder,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    order: LibraryBottomOrderAst::ChooserChooses,
                    surface:
                        ironsmith_core::LibraryRemainderSurface::
                            RestOfCardsRevealedThisWay,
                    ..
                },
            ..
        }) if tag == looked_tag && keep_tagged == selected_tag
    ));
}

#[test]
fn conditional_cardinality_branches_share_one_selected_tag_and_one_complement() {
    let raw = [
        "Look at the top five cards of your library",
        "If you control more creatures than each other player, put two of those cards into your hand",
        "Otherwise, put one of them into your hand",
        "Then put the rest on the bottom of your library in any order",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    let effects = crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("conditional partition parser should not error")
        .expect("Advice from the Fae shape should parse");
    assert_eq!(effects.len(), 3);

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards {
            tag: looked_tag, ..
        },
        ..
    }) = &effects[0]
    else {
        panic!("expected explicit looked-card producer: {:?}", effects[0]);
    };
    let EffectAst::Conditional {
        if_true, if_false, ..
    } = &effects[1]
    else {
        panic!("expected cardinality conditional: {:?}", effects[1]);
    };

    let branch = |effects: &[EffectAst]| {
        let [
            EffectAst::ChooseTaggedObjectsInZone {
                count, tag, filter, ..
            },
            EffectAst::MoveTaggedGroupToZone {
                tag: moved_tag,
                zone: Zone::Hand,
            },
        ] = effects
        else {
            panic!("expected choose-and-move branch: {effects:?}");
        };
        assert_eq!(tag, moved_tag);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *looked_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        (*count, tag.clone())
    };
    let (true_count, true_tag) = branch(if_true);
    let (false_count, false_tag) = branch(if_false);
    assert_eq!(true_count, ChoiceCount::exactly(2));
    assert_eq!(false_count, ChoiceCount::exactly(1));
    assert_eq!(true_tag, false_tag);

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged: Some(keep_tagged),
                order,
                ..
            },
        ..
    }) = &effects[2]
    else {
        panic!("expected exact bottom complement: {:?}", effects[2]);
    };
    assert_eq!(tag, looked_tag);
    assert_eq!(keep_tagged, &true_tag);
    assert_eq!(*order, LibraryBottomOrderAst::ChooserChooses);
}

#[test]
fn conditional_remainder_branches_share_the_looked_minus_selected_partition() {
    let raw = [
        "Look at the top nine cards of your library",
        "You may put a Gate card from among them onto the battlefield",
        "Then if you control nine or more Gates, put the rest into your hand",
        "Otherwise, put the rest on the bottom of your library in a random order",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    let effects = crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("conditional partition parser should not error")
        .expect("looked/selected/conditional-remainder shape should parse");
    let [look, choose, move_selected, conditional] = effects.as_slice() else {
        panic!("expected look/choose/move/conditional program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards {
            tag: looked_tag, ..
        },
        ..
    }) = look
    else {
        panic!("expected looked-card producer: {look:#?}");
    };
    let EffectAst::ChooseTaggedObjectsInZone {
        filter,
        count,
        tag: selected_tag,
        ..
    } = choose
    else {
        panic!("expected selected Gate subset: {choose:#?}");
    };
    assert_eq!(*count, ChoiceCount::up_to(1));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(matches!(
        move_selected,
        EffectAst::ForEachTagged { tag, .. } if tag == selected_tag
    ));
    let EffectAst::Conditional {
        if_true, if_false, ..
    } = conditional
    else {
        panic!("expected threshold disposition: {conditional:#?}");
    };
    assert!(matches!(
        if_true.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderInZone {
                    tag,
                    keep_tagged,
                    zone: Zone::Hand,
                    ..
                },
            ..
        })] if tag == looked_tag && keep_tagged == selected_tag
    ));
    assert!(matches!(
        if_false.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    order: LibraryBottomOrderAst::Random,
                    ..
                },
            ..
        })] if tag == looked_tag && keep_tagged == selected_tag
    ));
}

#[test]
fn conditional_entry_modifier_is_not_claimed_as_a_remainder_branch() {
    let raw = [
        "Look at the top seven cards of your library",
        "You may put a creature card from among them onto the battlefield",
        "If that card has mana value 3 or less, it enters with three additional +1/+1 counters on it",
        "Put the rest on the bottom of your library in a random order",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    let effects = crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .expect("ownership check should not error")
        .expect("the looked procedure reads the entry modifier as a statement")
        .effects;
    let debug = format!("{effects:#?}");
    assert!(
        !debug.contains("PutTaggedRemainderInZone"),
        "conditional entry modifiers must not be rewritten into a remainder branch: {debug}"
    );
    assert!(debug.contains("PutTaggedRemainderOnBottomOfLibrary"), "{debug}");
}

#[test]
fn conditional_entry_modifier_keeps_one_looked_partition_program() {
    let tokens = lex_line(
        "Look at the top seven cards of your library. You may put a creature card from among them onto the battlefield. If that card has mana value 3 or less, it enters with three additional +1/+1 counters on it. Put the rest on the bottom of your library in a random order.",
        0,
    )
    .expect("fixture should lex");
    let split = crate::split_lexed_sentences(&tokens);
    let sentences = split
        .iter()
        .map(|sentence| SentenceInput::from_lexed(sentence))
        .collect::<Vec<_>>();

    let effects =
        crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("typed program should not error")
        .expect("conditional entry-counter partition should match");
    assert_eq!(effects.len(), 5, "{effects:#?}");
    assert!(matches!(effects[2], EffectAst::ForEachTagged { .. }));
    assert!(matches!(effects[3], EffectAst::Conditional { .. }));
    assert!(matches!(
        effects[4],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
            ..
        })
    ));
}

#[test]
fn optional_source_payment_does_not_replace_plural_looked_card_branches() {
    let raw = [
        "Look at the top two cards of your library",
        "You may sacrifice this enchantment and pay {2}{G}{G}",
        "If you do, put one of those cards into your hand",
        "If you don't, put one of those cards on the bottom of your library",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    let effects = parse_look_then_may_action_if_did_or_did_not_move_looked_card(&sentences, 0)
        .expect("looked-card result parser should not error")
        .expect("optional action with two looked-card result branches should parse");
    let [look, optional, did, did_not] = effects.as_slice() else {
        panic!("expected look/optional/two-branch program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards {
            tag: looked_tag, ..
        },
        ..
    }) = look
    else {
        panic!("expected looked-card producer: {look:#?}");
    };
    assert!(matches!(
        optional,
        EffectAst::May { .. } | EffectAst::MayByPlayer { .. }
    ));

    let branch_target = |effect: &EffectAst, expected| {
        let EffectAst::IfResult {
            predicate,
            effects: branch,
        } = effect
        else {
            panic!("expected one move result branch: {effect:#?}");
        };
        assert_eq!(*predicate, expected);
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone { target, .. },
                ..
            }),
        ] = branch.as_slice()
        else {
            panic!("expected one move in result branch: {branch:#?}");
        };
        let TargetAst::WithCount(inner, count) = target else {
            panic!("expected one-card selection: {target:#?}");
        };
        assert!(count.is_single());
        let TargetAst::Tagged(tag, _) = inner.as_ref() else {
            panic!("expected looked-card tag: {inner:#?}");
        };
        tag.clone()
    };
    assert_eq!(
        branch_target(did, IfResultPredicate::Did),
        looked_tag.clone()
    );
    assert_eq!(
        branch_target(did_not, IfResultPredicate::ExplicitDidNot),
        looked_tag.clone()
    );
}

#[test]
fn intervening_sacrifice_does_not_replace_the_looked_candidate_pool() {
    let raw = [
        "Look at the top seven cards of your library",
        "Then you may sacrifice a creature",
        "If you do, you may put a creature card with mana value X or less from among those cards onto the battlefield, where X is 1 plus the sacrificed creature's mana value",
        "Put the rest on the bottom of your library in a random order",
    ];
    let lexed = raw
        .iter()
        .enumerate()
        .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
        .collect::<Vec<_>>();
    let sentences = lexed
        .iter()
        .map(|tokens| SentenceInput::from_lexed(tokens))
        .collect::<Vec<_>>();

    let effects =
        parse_look_then_may_sacrifice_if_did_select_battlefield_rest_bottom(&sentences, 0)
            .expect("intervening-action partition parser should not error")
            .expect("Birthing Ritual shape should parse");

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards {
            tag: looked_tag, ..
        },
        ..
    }) = &effects[0]
    else {
        panic!("expected explicit looked-card producer: {:?}", effects[0]);
    };
    let EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: selection,
    } = &effects[effects.len() - 2]
    else {
        panic!("expected sacrifice-result gate: {effects:?}");
    };
    let [
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            tag: selected_tag,
            count,
            zone,
            ..
        },
        EffectAst::ForEachTagged { tag: moved_tag, .. },
    ] = selection.as_slice()
    else {
        panic!("expected selected subset and battlefield move: {selection:?}");
    };
    assert_eq!(*count, ChoiceCount::up_to(1));
    assert_eq!(*zone, Zone::Library);
    assert_eq!(selected_tag, moved_tag);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged: Some(keep_tagged),
                order,
                ..
            },
        ..
    }) = effects.last().expect("bottom complement")
    else {
        panic!("expected exact bottom complement: {effects:?}");
    };
    assert_eq!(tag, looked_tag);
    assert_eq!(keep_tagged, selected_tag);
    assert_eq!(*order, LibraryBottomOrderAst::Random);
}
