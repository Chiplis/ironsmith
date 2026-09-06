    use super::*;

    #[test]
    fn life_loss_handler_wraps_delayed_draw_step_unless_payment() {
        let tokens = crate::lexer::lex_line(
            "1 life at the beginning of their next draw step unless they pay {1} before that draw step.",
            0,
        )
        .expect("lex delayed life loss");
        let effect = parse_lose_life(&tokens, Some(SubjectAst::Player(PlayerAst::That)))
            .expect("delayed life loss should parse");

        let EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextDrawStep {
            player: PlayerAst::That,
            effects,
        }) = effect
        else {
            panic!("expected delayed draw-step wrapper: {effect:#?}");
        };
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
                player: PlayerAst::That,
                effects,
                ..
            })] if matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                        amount: Value::Fixed(1),
                    }),
                    ..
                })]
            )
        ));
    }

    #[test]
    fn explicit_that_source_controller_keeps_the_triggering_source_reference() {
        let tokens =
            crate::lexer::lex_line("control of this creature.", 0)
                .expect("lex triggering-source controller clause");
        let effect = parse_gain_control(&tokens, Some(SubjectAst::TriggeringSourceController))
            .expect("parse triggering-source controller clause");

        assert!(matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Control(ControlActionAst::GainControl {
                    controller_reference:
                        Some(crate::target::ObjectRef::Tagged(ref tag)),
                    ..
                }),
                ..
            }) if tag.as_str() == "triggering_source"
        ));
    }

    #[test]
    fn gain_control_target_keeps_distinct_combat_damage_controller_history() {
        let tokens = crate::lexer::lex_line(
            "control of target nonland permanent controlled by a player who was dealt combat damage by three or more Pirates this turn.",
            0,
        )
        .expect("lex historical gain-control target");
        let effect =
            parse_gain_control(&tokens, None).expect("parse historical gain-control target");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Control(ControlActionAst::GainControl { target, .. }),
            ..
        }) = effect
        else {
            panic!("expected gain-control effect: {effect:#?}");
        };
        let TargetAst::Object(filter, explicit_target, _) = target else {
            panic!("expected object target: {target:#?}");
        };

        assert!(explicit_target.is_some());
        assert_eq!(filter.excluded_card_types, [CardType::Land]);
        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        assert!(matches!(
            filter.controller,
            Some(PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
                minimum: 3,
                ref sources,
                ..
            }) if sources.subtypes == [Subtype::Pirate]
        ));
    }

    #[test]
    fn gain_control_of_opponents_choice_declares_a_delegated_target() {
        let tokens = crate::lexer::lex_line(
            "control of target creature of an opponent's choice they control.",
            0,
        )
        .expect("lex opponent-chosen control target");
        let effect =
            parse_gain_control(&tokens, None).expect("parse opponent-chosen control target");
        let debug = format!("{effect:#?}");
        assert!(debug.contains("player: Opponent"), "{debug}");
        assert!(debug.contains("explicit_declaration: true"), "{debug}");
        assert!(debug.contains("opponent_chosen_target"), "{debug}");
    }

    #[test]
    fn source_exiled_move_surface_preserves_typed_subjects_and_onto_marker() {
        let tokens = crate::lexer::lex_line(
            "Put target creature card with mana value X exiled with this creature onto the battlefield under your control.",
            0,
        )
        .expect("lex source-exiled move");
        let surface = parse_exiled_with_source_move_surface(&tokens)
            .expect("parse source-exiled move surface");

        assert_eq!(
            surface.subject,
            ironsmith_core::ExiledWithSourceSubjectSurface::Custom(
                "target creature card with mana value X".to_string()
            )
        );
        assert!(matches!(
            surface.source,
            ironsmith_core::ExiledWithSourceReferenceSurface::Source(
                crate::target::SourceReferenceSurface::ThisPermanentType(ref text)
            ) if text == "this creature"
        ));

        let effect = parse_put_into_hand(&tokens, None).expect("parse source-exiled move");
        assert!(matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    exiled_with_source_surface: Some(
                        ironsmith_core::ExiledWithSourceMoveSurface {
                            subject: ironsmith_core::ExiledWithSourceSubjectSurface::Custom(ref text),
                            ..
                        }
                    ),
                    zone: Zone::Battlefield,
                    ..
                }),
                ..
            }) if text == "target creature card with mana value X"
        ));

        let tokens = crate::lexer::lex_line(
            "Return all cards you own exiled with this artifact to your hand.",
            0,
        )
        .expect("lex source-exiled return");
        let surface = parse_exiled_with_source_move_surface(&tokens)
            .expect("parse source-exiled return surface");
        assert_eq!(
            surface.verb,
            ironsmith_core::ExiledWithSourceMoveVerbSurface::Return
        );
        assert_eq!(
            surface.subject,
            ironsmith_core::ExiledWithSourceSubjectSurface::Custom("all cards you own".to_string())
        );

        let tokens = crate::lexer::lex_line(
            "Put all cards exiled with this enchantment on the bottom of their library in a random order.",
            0,
        )
        .expect("lex source-exiled bottom-library move");
        let surface = parse_exiled_with_source_move_surface(&tokens)
            .expect("parse source-exiled bottom-library move surface");
        assert_eq!(
            surface.subject,
            ironsmith_core::ExiledWithSourceSubjectSurface::AllCards
        );
        assert!(matches!(
            surface.source,
            ironsmith_core::ExiledWithSourceReferenceSurface::Source(
                crate::target::SourceReferenceSurface::ThisPermanentType(ref text)
            ) if text == "this enchantment"
        ));
        let effect =
            parse_put_into_hand(&tokens, None).expect("parse source-exiled bottom-library move");
        assert!(matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    exiled_with_source_surface: Some(ironsmith_core::ExiledWithSourceMoveSurface {
                        subject: ironsmith_core::ExiledWithSourceSubjectSurface::AllCards,
                        ..
                    }),
                    zone: Zone::Library,
                    ..
                }),
                ..
            })
        ));

        let tokens = crate::lexer::lex_line(
            "They put all cards exiled with this enchantment on the bottom of their library in a random order.",
            0,
        )
        .expect("lex actor-prefixed source-exiled bottom-library move");
        let surface = parse_exiled_with_source_move_surface(&tokens)
            .expect("parse actor-prefixed source-exiled move surface");
        assert_eq!(
            surface.subject,
            ironsmith_core::ExiledWithSourceSubjectSurface::AllCards
        );
    }

    #[test]
    fn source_exiled_return_tail_preserves_other_card_surface_without_a_verb() {
        let tokens = crate::lexer::lex_line(
            "each other card exiled with this Vehicle to the battlefield under its owner's control",
            0,
        )
        .expect("lex source-exiled return tail");
        let surface = parse_exiled_with_source_return_tail_surface(&tokens)
            .expect("parse verb-stripped source-exiled return tail");

        assert_eq!(
            surface.verb,
            ironsmith_core::ExiledWithSourceMoveVerbSurface::Return
        );
        assert_eq!(
            surface.subject,
            ironsmith_core::ExiledWithSourceSubjectSurface::Custom("each other card".to_string())
        );
        assert!(matches!(
            surface.source,
            ironsmith_core::ExiledWithSourceReferenceSurface::Source(_)
        ));
        assert_eq!(
            surface.destination,
            ironsmith_core::ExiledWithSourceDestinationSurface::ItsOwner
        );

        let unrelated = crate::lexer::lex_line(
            "each other Vehicle to the battlefield under its owner's control",
            0,
        )
        .expect("lex unrelated return tail");
        assert!(parse_exiled_with_source_return_tail_surface(&unrelated).is_none());
    }

    #[test]
    fn singular_source_exiled_move_preserves_exactly_one_choice() {
        let tokens = crate::lexer::lex_line(
            "Put a card exiled with this creature into its owner's hand.",
            0,
        )
        .expect("lex singular source-exiled move");
        let effect = parse_put_into_hand(&tokens, None).expect("parse source-exiled move");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    target: TargetAst::WithCount(inner, count),
                    exiled_with_source_surface: Some(surface),
                    zone: Zone::Hand,
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected a counted source-exiled move: {effect:#?}");
        };
        assert_eq!(count, ChoiceCount::exactly(1));
        assert!(matches!(inner.as_ref(), TargetAst::Object(..)));
        assert_eq!(
            surface.subject,
            ironsmith_core::ExiledWithSourceSubjectSurface::OneCard
        );
    }

    #[test]
    fn standalone_tagged_hand_move_preserves_exact_choice_count() {
        let tokens = crate::lexer::lex_line(
            "Put one of those cards into your hand.",
            0,
        )
        .expect("lex looked-card move");
        let effect = parse_put_into_hand(&tokens, None).expect("parse looked-card move");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    target: TargetAst::WithCount(inner, count),
                    zone,
                    destination_player_surface,
                    ..
                }),
            ..
        }) = effect
        else {
            panic!("expected a counted tagged move, got {effect:#?}");
        };

        assert_eq!(count, ChoiceCount::exactly(1));
        assert!(matches!(
            inner.as_ref(),
            TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        ));
        assert_eq!(zone, Zone::Hand);
        assert_eq!(destination_player_surface, Some(PlayerAst::You));
    }

    #[test]
    fn same_hand_cards_can_move_to_top_or_bottom_as_one_typed_choice() {
        let tokens = crate::lexer::lex_line(
            "Put two cards from your hand both on top of your library or both on the bottom of your library.",
            0,
        )
        .expect("lex same-card destination choice");
        let effect = parse_put_into_hand(&tokens, None).expect("parse destination choice");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenChoice { options }),
            ..
        }) = effect
        else {
            panic!("expected a typed destination choice: {effect:#?}");
        };
        assert_eq!(options.len(), 2);
        let zones = options
            .iter()
            .map(|(_, effect)| match effect.as_ref() {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                            target: TargetAst::WithCount(_, count),
                            zone,
                            to_top,
                            ..
                        }),
                    ..
                }) => (*zone, *to_top, *count),
                other => panic!("expected a counted library move: {other:#?}"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            zones,
            vec![
                (Zone::Library, true, ChoiceCount::exactly(2)),
                (Zone::Library, false, ChoiceCount::exactly(2)),
            ]
        );
    }

    #[test]
    fn explicit_you_revealed_collection_binds_the_reveal_producer() {
        let tokens = crate::lexer::lex_line(
            "Put the cards you revealed this way on the bottom of your library in any order.",
            0,
        )
        .expect("lex revealed collection move");
        let effect = parse_put_into_hand(&tokens, None).expect("parse revealed collection move");

        assert!(matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag,
                        keep_tagged: None,
                        order: crate::cards::builders::LibraryBottomOrderAst::ChooserChooses,
                        surface:
                            ironsmith_core::LibraryRemainderSurface::CardsYouRevealedThisWay,
                        ..
                    }),
                ..
            }) if tag.as_str() == "__last_revealed__"
        ));
    }

    #[test]
    fn bare_remainder_library_move_retains_its_order_for_typed_collection_binding() {
        let tokens = crate::lexer::lex_line(
            "Put the rest on the bottom of your library in a random order.",
            0,
        )
        .expect("lex remainder move");
        let effect = parse_put_into_hand(&tokens, None).expect("parse remainder move");

        assert!(matches!(
            &effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                    target: TargetAst::Tagged(tag, _),
                    zone: Zone::Library,
                    to_top: false,
                    library_order: Some(
                        crate::cards::builders::LibraryBottomOrderAst::Random
                    ),
                    ..
                }),
                ..
            }) if tag == &crate::tag::CompilerReferenceTag::Rest.key()
        ), "{effect:#?}");
    }
