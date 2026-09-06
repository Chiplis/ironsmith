use crate::cards::builders::ZoneMoveActionAst;
use crate::cards::builders::ReplacementActionAst;
use crate::cards::builders::LibraryActionAst;
use super::*;
use crate::lexer::lex_line;

fn parse_pair(first: &str, second: &str) -> Option<Vec<EffectAst>> {
    let first = lex_line(first, 0).expect("first sentence should lex");
    let second = lex_line(second, 1).expect("second sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    parse_look_at_top_then_partition_selected_and_remainder(&sentences, 0)
        .expect("partition parser should not error")
}

fn parse_counted_hand_pair(first: &str, second: &str) -> Option<Vec<EffectAst>> {
    let first = lex_line(first, 0).expect("first sentence should lex");
    let second = lex_line(second, 1).expect("second sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    parse_look_at_top_then_put_counted_hand_rest_bottom(&sentences, 0)
        .expect("counted hand partition parser should not error")
}

fn parse_singleton_graveyard_pair(first: &str, second: &str) -> Option<Vec<EffectAst>> {
    let first = lex_line(first, 0).expect("first sentence should lex");
    let second = lex_line(second, 1).expect("second sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("singleton looked-card parser should not error")
}

fn parse_singleton_hand_partition(
    first: &str,
    second: &str,
    remainder_zone: Zone,
) -> Option<Vec<EffectAst>> {
    let first = lex_line(first, 0).expect("first sentence should lex");
    let second = lex_line(second, 1).expect("second sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    match remainder_zone {
        Zone::Graveyard => parse_look_at_top_then_put_one_hand_other_graveyard(&sentences, 0),
        Zone::Library => parse_look_at_top_then_put_one_hand_other_bottom(&sentences, 0),
        other => panic!("unsupported test remainder zone: {other:?}"),
    }
    .expect("singleton hand partition parser should not error")
}

#[test]
fn targeted_graveyard_cast_accepts_single_spell_type_and_unspecified_owner() {
    for (first, expected_owner) in [
        (
            "You may cast target instant card from your graveyard without paying its mana cost",
            Some(PlayerFilter::You),
        ),
        (
            "You may cast target instant or sorcery card from a graveyard without paying its mana cost",
            None,
        ),
        (
            "You may cast target red instant or sorcery card from your graveyard",
            Some(PlayerFilter::You),
        ),
    ] {
        let expects_red = first.contains("target red");
        let first = lex_line(first, 0).expect("cast sentence should lex");
        let second = lex_line(
            "If that spell would be put into a graveyard, exile it instead",
            1,
        )
        .expect("replacement sentence should lex");
        let sentences = [
            SentenceInput::from_lexed(&first),
            SentenceInput::from_lexed(&second),
        ];
        let effects = parse_may_cast_target_graveyard_spell_then_exile_replacement(&sentences, 0)
            .expect("graveyard cast parser should not error")
            .expect("single-type/any-graveyard cast pair should parse");
        let debug = format!("{effects:#?}");
        let Some(EffectAst::TagAffected { effect, .. }) = effects.first() else {
            panic!("expected tagged target declaration: {effects:#?}");
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TargetOnly {
                    target: TargetAst::Object(filter, _, _),
                    ..
                },
            ..
        }) = effect.as_ref()
        else {
            panic!("expected typed graveyard target: {effect:#?}");
        };

        assert!(debug.contains("TargetOnly"), "{debug}");
        assert!(debug.contains("CastTagged"), "{debug}");
        assert!(debug.contains("RegisterFutureZoneReplacement"), "{debug}");
        assert_eq!(filter.owner, expected_owner);
        if expects_red {
            assert_eq!(filter.colors, Some(crate::color::ColorSet::RED));
        }
    }
}

#[test]
fn reflexive_targeted_graveyard_cast_keeps_target_x_and_replacement_scope() {
    let first = lex_line(
            "When you do, you may cast target instant or sorcery card with mana value X from a graveyard without paying its mana cost",
            0,
        )
        .expect("reflexive cast sentence should lex");
    let second = lex_line(
        "If that spell would be put into a graveyard, exile it instead",
        1,
    )
    .expect("replacement sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];

    let effects =
        crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
            .expect("reflexive graveyard cast parser should not error")
            .expect("reflexive graveyard cast pair should parse");
    let [
        EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
            predicate: IfResultPredicate::Did,
            effects: body,
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one typed reflexive result wrapper: {effects:#?}");
    };
    let debug = format!("{body:#?}");
    assert!(debug.contains("TargetOnly"), "{debug}");
    let target_filter = body.iter().find_map(|effect| match effect {
        EffectAst::TagAffected { effect, .. } => match effect.as_ref() {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::TargetOnly {
                        target: TargetAst::Object(filter, ..),
                        ..
                    },
                ..
            }) => Some(filter),
            _ => None,
        },
        _ => None,
    });
    assert!(
        target_filter.is_some_and(|filter| filter.mana_value.is_some()),
        "{debug}"
    );
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("RegisterFutureZoneReplacement"), "{debug}");

    let nonreflexive = lex_line(
            "If you do, you may cast target instant or sorcery card with mana value X from a graveyard without paying its mana cost",
            0,
        )
        .unwrap();
    let near_miss = [
        SentenceInput::from_lexed(&nonreflexive),
        SentenceInput::from_lexed(&second),
    ];
    assert!(
        crate::effect_sentences::sequence_rules::try_parse_document_program(&near_miss, 0)
        .map(|matched| matched.map(|matched| matched.effects))
            .unwrap()
            .is_none()
    );
}

#[test]
fn targeted_graveyard_cast_preserves_instruction_additional_mana_cost() {
    let first = lex_line(
            "You may cast target instant or sorcery card from your graveyard by paying {R}{R} in addition to its other costs",
            0,
        )
        .expect("cast sentence should lex");
    let second = lex_line(
        "If that spell would be put into a graveyard, exile it instead",
        1,
    )
    .expect("replacement sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];

    let effects = parse_may_cast_target_graveyard_spell_then_exile_replacement(&sentences, 0)
        .expect("graveyard cast parser should not error")
        .expect("additional-cost graveyard cast should parse");
    let debug = format!("{effects:#?}");
    assert!(!debug.contains("other: true"), "{debug}");
    assert!(debug.contains("additional_mana_cost: Some"), "{debug}");
    assert!(debug.contains("Red"), "{debug}");
    assert!(debug.contains("stack_kind: Some(\n"), "{debug}");
    assert!(debug.contains("Spell"), "{debug}");
}

#[test]
fn targeted_graveyard_cast_keeps_dynamic_source_power_and_exact_spell_tag() {
    let first = lex_line(
            "You may cast target instant or sorcery card with mana value less than or equal to his power from your graveyard",
            0,
        )
        .expect("cast sentence should lex");
    let second = lex_line(
        "If that spell would be put into your graveyard, exile it instead",
        1,
    )
    .expect("replacement sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    let first_lowered = trim_commas(sentences[0].lowered());
    let second_lowered = trim_commas(sentences[1].lowered());
    assert!(
        effect_grammar::parse_graveyard_cast_replacement_shape(&first_lowered, &second_lowered)
            .is_some(),
        "dynamic cast/replacement surface should reach the typed pair shape: {:?} / {:?}",
        crate::lexer::token_word_refs(&first_lowered),
        crate::lexer::token_word_refs(&second_lowered)
    );
    let target_start = first_lowered
        .iter()
        .position(|token| token.is_word("target"))
        .expect("dynamic cast sentence should keep its target")
        + 1;
    let diagnostic_filter = parse_object_filter_lexed(&first_lowered[target_start..], false)
        .unwrap_or_else(|error| {
            panic!(
                "dynamic source-power target filter should parse ({error}): {:?}",
                crate::lexer::token_word_refs(&first_lowered[target_start..])
            )
        });
    assert_eq!(
        diagnostic_filter.zone,
        Some(Zone::Graveyard),
        "{diagnostic_filter:#?}"
    );
    assert_eq!(
        diagnostic_filter.owner,
        Some(PlayerFilter::You),
        "{diagnostic_filter:#?}"
    );
    assert!(
        diagnostic_filter.card_types.contains(&CardType::Instant)
            && diagnostic_filter.card_types.contains(&CardType::Sorcery),
        "{diagnostic_filter:#?}"
    );
    let effects = parse_may_cast_target_graveyard_spell_then_exile_replacement(&sentences, 0)
        .expect("graveyard cast parser should not error")
        .expect("graveyard cast/replacement pair should parse");

    let [
        EffectAst::TagAffected {
            effect: target_effect,
            tag: target_tag,
        },
        EffectAst::Permissions(PermissionEffectAst::May {
            effects: cast_effects,
        }),
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: replacement_effects,
        }),
    ] = effects.as_slice()
    else {
        panic!("expected target/cast/replacement program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TargetOnly {
                target: TargetAst::Object(filter, _, _),
                ..
            },
        ..
    }) = target_effect.as_ref()
    else {
        panic!("expected typed graveyard target: {target_effect:#?}");
    };
    assert!(matches!(
        filter.mana_value.as_ref(),
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value))
            if value.unhinted() == &Value::SourcePower
                && value.has_surface_hint(
                    ironsmith_core::ValueSurfaceHint::MasculineSourcePossessive
                )
    ));

    let [
        EffectAst::TagAffected {
            effect: cast_effect,
            tag: cast_spell_tag,
        },
    ] = cast_effects.as_slice()
    else {
        panic!("expected exact cast-result tag: {cast_effects:#?}");
    };
    assert!(matches!(
        cast_effect.as_ref(),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Stack(StackActionAst::CastTagged { tag, .. }),
            ..
        }) if tag == target_tag
    ));

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterFutureZoneReplacement {
                    filter: replacement_filter,
                    from_zone: Some(Zone::Stack),
                    to_zone: Some(Zone::Graveyard),
                    replacement_zone: Zone::Exile,
                    ..
                }),
            ..
        }),
    ] = replacement_effects.as_slice()
    else {
        panic!("expected tagged future replacement: {replacement_effects:#?}");
    };
    assert!(
        replacement_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == **cast_spell_tag
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            })
    );
}

#[test]
fn targeted_graveyard_cast_keeps_one_shot_any_type_mana_permission() {
    let first = lex_line(
            "You may cast target instant or sorcery card from a graveyard, and mana of any type can be spent to cast that spell",
            0,
        )
        .expect("cast sentence should lex");
    let second = lex_line(
        "If that spell would be put into a graveyard, exile it instead",
        1,
    )
    .expect("replacement sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    let effects = parse_may_cast_target_graveyard_spell_then_exile_replacement(&sentences, 0)
        .expect("graveyard cast parser should not error")
        .expect("typed any-type graveyard cast should parse");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("mana_spend_mode: AnyType"), "{debug}");
    let target_filter = effects.iter().find_map(|effect| match effect {
        EffectAst::TagAffected { effect, .. } => match effect.as_ref() {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::TargetOnly {
                        target: TargetAst::Object(filter, ..),
                        ..
                    },
                ..
            }) => Some(filter),
            _ => None,
        },
        _ => None,
    });
    assert_eq!(
        target_filter.and_then(|filter| filter.zone),
        Some(Zone::Graveyard)
    );
}

#[test]
fn duration_scoped_targeted_graveyard_cast_keeps_permission_and_replacement_lifetime() {
    let first = lex_line(
            "Until end of turn, you may cast target instant or sorcery card from your graveyard without paying its mana cost",
            0,
        )
        .expect("cast sentence should lex");
    let second = lex_line(
        "If that spell would be put into your graveyard, exile it instead",
        1,
    )
    .expect("replacement sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    let effects = parse_may_cast_target_graveyard_spell_then_exile_replacement(&sentences, 0)
        .expect("graveyard cast parser should not error")
        .expect("duration-scoped cast/replacement pair should parse");

    let [
        EffectAst::TagAffected {
            tag: target_tag, ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: permission_tag,
                    without_paying_mana_cost: true,
                    free_cast_from_current_zone: true,
                    surface: Some(surface),
                    ..
                }),
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterFutureZoneReplacement {
                    filter,
                    from_zone: Some(Zone::Stack),
                    to_zone: Some(Zone::Graveyard),
                    replacement_zone: Zone::Exile,
                    duration: ZoneReplacementDurationAst::UntilEndOfTurn,
                    ..
                }),
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected target/permission/replacement program: {effects:#?}");
    };
    assert_eq!(target_tag, permission_tag);
    assert!(surface.leading_duration);
    assert_eq!(
        surface.object,
        Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard)
    );
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == **target_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
}

fn move_parts(effect: &EffectAst) -> (Zone, bool, Option<LibraryBottomOrderAst>, PlayerAst) {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                zone,
                to_top,
                library_order,
                library_order_chooser,
                ..
            }),
        ..
    }) = effect
    else {
        panic!("expected a structured move-to-zone effect: {effect:?}");
    };
    (*zone, *to_top, *library_order, *library_order_chooser)
}

fn assert_singleton_hand_partition_has_exact_complement(
    effects: &[EffectAst],
    expected_remainder: (Zone, bool, Option<LibraryBottomOrderAst>, PlayerAst),
) {
    let [look, choose, complement, selected_move, remainder_move] = effects else {
        panic!("expected look/choose/complement/two-move program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
            tag: looked_tag, ..
        }),
        ..
    }) = look
    else {
        panic!("expected looked-card producer: {look:#?}");
    };
    let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        tag: selected_tag,
        count,
        ..
    }) = choose
    else {
        panic!("expected selected-card choice: {choose:#?}");
    };
    assert_eq!(*count, ChoiceCount::exactly(1));
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TagMatchingObjects {
                filter,
                zones,
                tag: remainder_tag,
                ..
            },
        ..
    }) = complement
    else {
        panic!("expected exact-complement tag: {complement:#?}");
    };
    assert_eq!(zones, &[Zone::Library]);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == **looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == **selected_tag
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }));
    assert!(matches!(
        selected_move,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    target: TargetAst::Tagged(tag, _),
                    zone: Zone::Hand,
                    ..
                }),
            ..
        }) if tag == selected_tag
    ));
    assert!(matches!(
        remainder_move,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    target: TargetAst::Tagged(tag, _),
                    ..
                }),
            ..
        }) if tag == remainder_tag
    ));
    assert_eq!(move_parts(remainder_move), expected_remainder);
}

#[test]
fn singleton_hand_graveyard_partition_tags_and_moves_the_exact_complement() {
    let effects = parse_singleton_hand_partition(
        "Look at the top three cards of your library",
        "Put one of them into your hand and the rest into your graveyard",
        Zone::Graveyard,
    )
    .expect("hand/graveyard partition should parse");
    assert_singleton_hand_partition_has_exact_complement(
        &effects,
        (Zone::Graveyard, false, None, PlayerAst::Implicit),
    );
}

#[test]
fn singleton_hand_bottom_partition_tags_and_moves_the_exact_complement() {
    let effects = parse_singleton_hand_partition(
            "Look at the top four cards of your library",
            "Put one of those cards into your hand and the rest on the bottom of your library in a random order",
            Zone::Library,
        )
        .expect("hand/bottom partition should parse");
    assert_singleton_hand_partition_has_exact_complement(
        &effects,
        (
            Zone::Library,
            false,
            Some(LibraryBottomOrderAst::Random),
            PlayerAst::You,
        ),
    );
}

#[test]
fn target_library_partition_keeps_you_as_chooser_and_tags_the_complement() {
    let effects = parse_pair(
            "Look at the top five cards of target opponent's library",
            "Put one of those cards into that player's graveyard and the rest on top of their library in any order",
        )
        .expect("Cruel Fate shape should parse");
    assert_eq!(effects.len(), 5);

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst {
            player: library_owner,
            ..
        },
        action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
            tag: looked_tag, ..
        }),
    }) = &effects[0]
    else {
        panic!("expected looked-card provenance: {:?}", effects[0]);
    };
    assert_eq!(*library_owner, PlayerAst::TargetOpponent);

    let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        player,
        tag: selected_tag,
        count,
        zone,
        ..
    }) = &effects[1]
    else {
        panic!("expected selected-subset choice: {:?}", effects[1]);
    };
    assert_eq!(*player, PlayerAst::You);
    assert_eq!(*count, ChoiceCount::exactly(1));
    assert_eq!(*zone, Zone::Library);

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::TagMatchingObjects { filter, zones, .. },
        ..
    }) = &effects[2]
    else {
        panic!("expected a structured complement tag: {:?}", effects[2]);
    };
    assert_eq!(zones, &[Zone::Library]);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == **looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == **selected_tag
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }));

    assert_eq!(move_parts(&effects[3]).0, Zone::Graveyard);
    assert_eq!(
        move_parts(&effects[4]),
        (
            Zone::Library,
            true,
            Some(LibraryBottomOrderAst::ChooserChooses),
            PlayerAst::You,
        )
    );
}

#[test]
fn selected_and_remainder_library_orders_are_independent() {
    let effects = parse_pair(
            "Look at the top five cards of target player's library",
            "Put any number of them on the bottom of that library in a random order and the rest on top of the library in any order",
        )
        .expect("Ransack shape should parse");
    let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { count, player, .. }) = &effects[1] else {
        panic!("expected selected-subset choice: {:?}", effects[1]);
    };
    assert_eq!(*count, ChoiceCount::any_number());
    assert_eq!(*player, PlayerAst::You);
    assert_eq!(
        move_parts(&effects[3]),
        (
            Zone::Library,
            false,
            Some(LibraryBottomOrderAst::Random),
            PlayerAst::You,
        )
    );
    assert_eq!(
        move_parts(&effects[4]),
        (
            Zone::Library,
            true,
            Some(LibraryBottomOrderAst::ChooserChooses),
            PlayerAst::You,
        )
    );
}

#[test]
fn optional_top_selection_uses_a_tagged_move_and_exact_bottom_remainder() {
    let effects = parse_pair(
            "Look at the top X cards of your library, where X is your devotion to blue",
            "Put up to one of them on top of your library and the rest on the bottom of your library in a random order",
        )
        .expect("optional top/bottom partition should parse");
    let [look, choose, move_each, remainder] = effects.as_slice() else {
        panic!("expected look/choose/move/remainder program: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { tag: looked, .. }),
        ..
    }) = look
    else {
        panic!("expected looked-card producer: {look:#?}");
    };
    let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        count,
        tag: selected,
        ..
    }) = choose
    else {
        panic!("expected selected tag: {choose:#?}");
    };
    assert_eq!(*count, ChoiceCount::up_to(1));
    assert!(matches!(
        move_each,
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged { tag, .. }) if tag == selected
    ));
    assert!(matches!(
        remainder,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep),
                    order: LibraryBottomOrderAst::Random,
                    ..
                }),
            ..
        }) if tag == looked && keep == selected
    ));
}

#[test]
fn counted_hand_selection_and_singular_graveyard_remainder_stay_disjoint() {
    let effects = parse_pair(
        "Look at the top three cards of your library",
        "Put two of them into your hand and the other into your graveyard",
    )
    .expect("counted hand/graveyard partition should parse");
    assert_eq!(effects.len(), 5);

    let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        count,
        player,
        tag: selected_tag,
        zone,
        ..
    }) = &effects[1]
    else {
        panic!("expected selected-subset choice: {:?}", effects[1]);
    };
    assert_eq!(*count, ChoiceCount::exactly(2));
    assert_eq!(*player, PlayerAst::You);
    assert_eq!(*zone, Zone::Library);

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::TagMatchingObjects { filter, zones, .. },
        ..
    }) = &effects[2]
    else {
        panic!(
            "expected the exact complement to be tagged: {:?}",
            effects[2]
        );
    };
    assert_eq!(zones, &[Zone::Library]);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == **selected_tag
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }));
    assert_eq!(move_parts(&effects[3]).0, Zone::Hand);
    assert_eq!(move_parts(&effects[4]).0, Zone::Graveyard);
}

#[test]
fn direct_counted_hand_selection_uses_one_looked_pool_and_exact_complement() {
    for (first, second, expected_count) in [
        (
            "Look at the top X cards of your library, where X is the number of Caves you control plus the number of Cave cards in your graveyard",
            "Put two of those cards into your hand and the rest on the bottom of your library in a random order",
            ChoiceCount::exactly(2),
        ),
        (
            "Look at the top X cards of your library, where X is three plus the number of creatures in your party",
            "Put three of those cards into your hand and the rest on the bottom of your library in a random order",
            ChoiceCount::exactly(3),
        ),
    ] {
        let effects = parse_counted_hand_pair(first, second)
            .expect("direct counted hand partition should parse");
        let [look_effect, choose_effect, move_effect, remainder_effect] = effects.as_slice() else {
            panic!("expected flat look/choose/move/remainder program: {effects:#?}");
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
                    tag: looked_tag, ..
                }),
            ..
        }) = look_effect
        else {
            panic!("expected looked-card provenance: {look_effect:#?}");
        };
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            player,
            tag: selected_tag,
            zone,
        }) = choose_effect
        else {
            panic!("expected selected looked-card subset: {choose_effect:#?}");
        };
        assert_eq!(*count, expected_count);
        assert_eq!(*player, PlayerAst::You);
        assert_eq!(*zone, Zone::Library);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == **looked_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(matches!(
            move_effect,
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged { tag, effects })
                if tag == selected_tag
                    && matches!(
                        effects.as_slice(),
                        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                                zone: Zone::Hand,
                                ..
                            }),
                            ..
                        })]
                    )
        ));
        assert!(matches!(
            remainder_effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag,
                        keep_tagged: Some(keep_tagged),
                        order: LibraryBottomOrderAst::Random,
                        player: PlayerAst::You,
                        ..
                    }),
                ..
            }) if tag == looked_tag && keep_tagged == selected_tag
        ));
    }
}

#[test]
fn exact_singleton_looked_selection_moves_only_the_selected_tag() {
    for (first, second, expected_owner, expected_filter_owner) in [
        (
            "Look at the top two cards of your library",
            "Put one of them into your graveyard",
            PlayerAst::You,
            PlayerFilter::You,
        ),
        (
            "Look at the top two cards of target player's library",
            "Put one of them into their graveyard",
            PlayerAst::Target,
            PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)),
        ),
    ] {
        let effects = parse_singleton_graveyard_pair(first, second)
            .expect("exact singleton looked-card pair should parse");
        let [look_effect, choose_effect, move_effect] = effects.as_slice() else {
            panic!("expected look/choose/move program without a remainder move: {effects:#?}");
        };

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action:
                SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
                    count,
                    tag: looked_tag,
                    reveal: false,
                }),
        }) = look_effect
        else {
            panic!("expected private looked-card producer: {look_effect:#?}");
        };
        assert_eq!(*player, expected_owner);
        assert_eq!(*count, Value::Fixed(2));

        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            player,
            tag: selected_tag,
            zone,
        }) = choose_effect
        else {
            panic!("expected exact selected-card tag: {choose_effect:#?}");
        };
        assert_eq!(*count, ChoiceCount::exactly(1));
        assert_eq!(*player, PlayerAst::You);
        assert_eq!(*zone, Zone::Library);
        assert_eq!(filter.owner.as_ref(), Some(&expected_filter_owner));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == **looked_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));

        assert!(matches!(
            move_effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                        target: TargetAst::Tagged(tag, _),
                        zone: Zone::Graveyard,
                        ..
                    }),
                ..
            }) if tag == selected_tag
        ));
    }
}

#[test]
fn existing_bottom_and_rearrange_controls_do_not_match_partition_pair() {
    for second in [
        "Put one of them into your hand and the rest on the bottom of your library in any order",
        "Put them back in any order",
    ] {
        assert!(
            parse_pair("Look at the top five cards of your library", second).is_none(),
            "control should remain on its existing parser path: {second}"
        );
    }
}

#[test]
fn face_down_exile_keeps_the_graveyard_complement_and_permission_tag() {
    let first = lex_line(
            "Look at the top three cards of that player's library, exile one of them face down, then put the rest into their graveyard",
            0,
        )
        .expect("first sentence should lex");
    let second = lex_line(
            "You may cast that card for as long as it remains exiled, and mana of any type can be spent to cast that spell",
            1,
        )
        .expect("second sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];
    let effects = crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
        .map(|matched| matched.map(|matched| matched.effects))
        .expect("face-down partition parser should not error")
        .expect("Thief of Sanity shape should parse");
    assert_eq!(effects.len(), 5);

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
            tag: looked_tag, ..
        }),
        ..
    }) = &effects[0]
    else {
        panic!("expected looked-card producer: {:?}", effects[0]);
    };
    let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        tag: exiled_tag,
        count,
        filter,
        zone,
        ..
    }) = &effects[1]
    else {
        panic!("expected face-down selected subset: {:?}", effects[1]);
    };
    assert_eq!(*count, ChoiceCount::exactly(1));
    assert_eq!(*zone, Zone::Library);
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == **looked_tag
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
                tag,
                keep_tagged,
                zone: Zone::Graveyard,
                ..
            }),
        ..
    }) = &effects[3]
    else {
        panic!("expected exact graveyard complement: {:?}", effects[3]);
    };
    assert_eq!(tag, looked_tag);
    assert_eq!(keep_tagged, exiled_tag);

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag: permission_tag,
                ..
            }),
        ..
    }) = &effects[4]
    else {
        panic!("expected tagged cast permission: {:?}", effects[4]);
    };
    assert_eq!(permission_tag, exiled_tag);
}

#[test]
fn complete_face_down_partition_does_not_steal_cast_permission_followup() {
    let first = lex_line(
            "Look at the top four cards of target opponent's library, exile one of them face down, then put the rest on the bottom of that library in a random order",
            0,
        )
        .expect("first sentence should lex");
    let second = lex_line(
            "You may cast that card for as long as it remains exiled, and mana of any type can be spent to cast that spell",
            1,
        )
        .expect("second sentence should lex");
    let sentences = [
        SentenceInput::from_lexed(&first),
        SentenceInput::from_lexed(&second),
    ];

    let matched =
        crate::effect_sentences::sequence_rules::try_parse_document_program(&sentences, 0)
            .expect("sequence registry should not error")
            .expect("look/exile/permission sequence should match");

    assert_eq!(matched.name, "looked-procedure");
    assert_eq!(matched.effects.len(), 5);
    let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        tag: selected_tag, ..
    }) = &matched.effects[1]
    else {
        panic!("expected selected looked card: {:#?}", matched.effects[1]);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag: permission_tag,
                player: PlayerAst::You,
                allow_land: false,
                without_paying_mana_cost: false,
                allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode::AnyType,
                filter: None,
                ..
            }),
        ..
    }) = &matched.effects[4]
    else {
        panic!(
            "expected exact tagged cast permission: {:#?}",
            matched.effects[4]
        );
    };
    assert_eq!(permission_tag, selected_tag);
}
