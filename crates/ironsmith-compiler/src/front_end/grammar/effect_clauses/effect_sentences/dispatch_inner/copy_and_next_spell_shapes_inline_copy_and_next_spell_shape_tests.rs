    use super::*;

    #[test]
    fn delayed_end_step_header_uses_captured_step_owner() {
        let tokens = crate::lexer::lex_line(
            "At the beginning of your next end step, draw a card.",
            0,
        )
        .expect("delayed end-step text should lex");

        let effects = parse_delayed_until_next_end_step_sentence(&tokens)
            .expect("delayed end-step parser should not error")
            .expect("delayed end-step parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedUntilNextEndStep"), "{debug}");
        assert!(debug.contains("player: You"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn delayed_end_step_header_uses_captured_turn_owner() {
        let tokens = crate::lexer::lex_line(
            "At the beginning of the end step of that player's next turn, draw a card.",
            0,
        )
        .expect("extra-turn delayed end-step text should lex");

        let effects = parse_delayed_until_next_end_step_sentence(&tokens)
            .expect("extra-turn delayed end-step parser should not error")
            .expect("extra-turn delayed end-step parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedUntilEndStepOfExtraTurn"), "{debug}");
        assert!(debug.contains("player: That"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn delayed_end_step_body_uses_typed_consult_bundle_dispatch() {
        let tokens = crate::lexer::lex_line(
            "At the beginning of the next end step, reveal cards from the top of your library until you reveal that many creature cards, put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library.",
            0,
        )
        .expect("delayed consult text should lex");

        let effects = parse_delayed_until_next_end_step_sentence(&tokens)
            .expect("delayed consult parser should not error")
            .expect("delayed consult parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedUntilNextEndStep"), "{debug}");
        assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
        assert!(debug.contains("ShuffleLibrary"), "{debug}");
    }

    #[test]
    fn delayed_dies_this_way_uses_captured_filter() {
        let tokens = crate::lexer::lex_line(
            "If a creature dealt damage this way would die this turn, exile it instead.",
            0,
        )
        .expect("dies-this-way delayed text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("dies-this-way parser should not error")
            .expect("dies-this-way parser should match");
        let debug = format!("{effects:#?}");

        assert!(
            debug.contains("DelayedWhenLastObjectDiesThisTurn"),
            "{debug}"
        );
        assert!(debug.contains("filter: Some"), "{debug}");
        assert!(debug.contains("card_types"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");
    }

    #[test]
    fn delayed_that_dies_this_turn_uses_captured_effect_tail() {
        let tokens =
            crate::lexer::lex_line("When that creature dies this turn, draw a card.", 0)
                .expect("that-dies delayed text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("that-dies parser should not error")
            .expect("that-dies parser should match");
        let debug = format!("{effects:#?}");

        assert!(
            debug.contains("DelayedWhenLastObjectDiesThisTurn"),
            "{debug}"
        );
        assert!(debug.contains("filter: None"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn delayed_death_exiles_the_referenced_objects_controllers_whole_graveyard() {
        let tokens = crate::lexer::lex_line(
            "When that creature dies this turn, exile its controller's graveyard.",
            0,
        )
        .expect("whole-graveyard delayed text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("delayed death parser should not error")
            .expect("delayed death parser should match");
        let [
            EffectAst::DelayedWhenLastObjectDiesThisTurn {
                effects: delayed, ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one delayed watcher: {effects:#?}");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileAll { filter, .. },
                ..
            }),
        ] = delayed.as_slice()
        else {
            panic!("expected exhaustive graveyard exile: {delayed:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(matches!(
            &filter.owner,
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag)))
                if tag.as_str() == crate::cards::builders::IT_TAG
        ));
    }

    #[test]
    fn definite_filtered_death_subject_watches_the_prior_object_once() {
        let tokens = crate::lexer::lex_line(
            "When the permanent you don't control dies this turn, you gain 2 life.",
            0,
        )
        .expect("definite delayed-death text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("definite delayed-death parser should not error")
            .expect("definite delayed-death parser should match");
        let [
            EffectAst::DelayedWhenLastObjectDiesThisTurn {
                filter: Some(filter),
                effects,
            },
        ] = effects.as_slice()
        else {
            panic!("expected a prior-object delayed death watcher: {effects:#?}");
        };

        assert_eq!(
            filter.demonstrative_antecedent_surface(),
            Some(ironsmith_core::DemonstrativeAntecedentSurface::Permanent)
        );
        assert_eq!(filter.controller, Some(PlayerFilter::NotYou));
        assert!(matches!(effects.as_slice(), [EffectAst::SubjectVerb(_)]));
    }

    #[test]
    fn prior_target_damage_then_definite_death_lowers_to_tagged_this_dies() {
        let definition = crate::CardDefinitionBuilder::new(
            crate::CardId::new(),
            "Prior Target Death Watch",
        )
        .card_types(vec![crate::CardType::Sorcery])
        .parse_text(
            "Target creature you control deals damage equal to its power to target creature or planeswalker you don't control. When the permanent you don't control dies this turn, you gain 2 life.",
        )
        .expect("a definite delayed-death subject should compile against the prior target");
        let debug = format!("{:#?}", definition.spell_effect);

        assert!(debug.contains("ThisDies"), "{debug}");
        assert!(debug.contains("one_shot: true"), "{debug}");
        assert!(debug.contains("target_tag: Some"), "{debug}");
        assert!(
            debug.contains("demonstrative_antecedent: Some(\n") && debug.contains("Permanent"),
            "{debug}"
        );
        assert!(!debug.contains("ThisLeavesBattlefield"), "{debug}");
    }

    #[test]
    fn indefinite_damage_history_subject_keeps_this_way_collection_shape() {
        let tokens = crate::lexer::lex_line(
            "Whenever a creature dealt damage this way dies this turn, you gain 2 life.",
            0,
        )
        .expect("damage-history delayed-death text should lex");

        let effects = parse_delayed_when_that_dies_this_turn_sentence(&tokens)
            .expect("damage-history delayed-death parser should not error")
            .expect("damage-history delayed-death parser should match");
        let [
            EffectAst::DelayedWhenLastObjectDiesThisTurn {
                filter: Some(filter),
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected a this-way delayed death watcher: {effects:#?}");
        };

        assert_eq!(filter.demonstrative_antecedent_surface(), None);
    }

    #[test]
    fn delayed_that_creature_leaves_uses_captured_effect_tail() {
        let tokens = crate::lexer::lex_line(
            "When that creature leaves the battlefield, return this card from exile to the battlefield under its owner's control.",
            0,
        )
        .expect("that-leaves delayed text should lex");

        let effects = parse_delayed_when_that_leaves_battlefield_sentence(&tokens)
            .expect("that-leaves parser should not error")
            .expect("that-leaves parser should match");
        let debug = format!("{effects:#?}");

        assert!(
            debug.contains("DelayedWhenLastObjectLeavesBattlefield"),
            "{debug}"
        );
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("Return"), "{debug}");
    }

    #[test]
    fn this_turn_delayed_trigger_uses_captured_duration_tail() {
        let tokens = crate::lexer::lex_line(
            "This turn, whenever you draw a card, draw a card.",
            0,
        )
        .expect("this-turn delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("this-turn delayed trigger parser should not error")
            .expect("this-turn delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("YouDrawCard"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn suffix_this_turn_delayed_trigger_uses_captured_trigger_and_effect() {
        let tokens =
            crate::lexer::lex_line("Whenever you draw a card this turn, draw a card.", 0)
                .expect("suffix-this-turn delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("suffix-this-turn delayed trigger parser should not error")
            .expect("suffix-this-turn delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("YouDrawCard"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn public_effect_sequence_keeps_suffix_this_turn_delayed_trigger() {
        let tokens = crate::lexer::lex_line(
            "Whenever you cast a creature spell this turn, draw a card.",
            0,
        )
        .expect("duration-scoped cast trigger should lex");

        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
            .expect("public effect sequence should parse");
        let [EffectAst::DelayedTriggerThisTurn {
            trigger: TriggerSpec::SpellCast { .. },
            effects: delayed,
            ..
        }] = effects.as_slice()
        else {
            panic!("expected intact delayed spell trigger: {effects:#?}");
        };

        assert!(
            delayed
                .iter()
                .any(|effect| format!("{effect:?}").contains("Draw")),
            "{delayed:#?}"
        );
    }

    #[test]
    fn public_effect_sequence_keeps_coordinated_delayed_trigger_payload() {
        let tokens = crate::lexer::lex_line(
            "Whenever a creature you control enters this turn, each opponent loses 1 life and you gain 1 life.",
            0,
        )
        .expect("coordinated duration-scoped trigger should lex");

        let shape = delayed_shapes::parse_delayed_this_turn_shape(&tokens)
            .expect("the complete duration-scoped sentence should retain its payload");
        assert_eq!(
            crate::lexer::render_token_slice(shape.trigger_tokens).trim(),
            "a creature you control enters"
        );

        let effects = crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
            .expect("public effect sequence should keep the delayed schedule");
        let [EffectAst::DelayedTriggerThisTurn { effects: delayed, .. }] = effects.as_slice()
        else {
            panic!("expected one delayed trigger: {effects:#?}");
        };

        let debug = format!("{delayed:#?}");
        assert!(debug.contains("LoseLife"), "{debug}");
        assert!(debug.contains("GainLife"), "{debug}");
    }

    #[test]
    fn next_single_opponent_or_permanent_copy_keeps_each_other_opponent_choice() {
        let tokens = crate::lexer::lex_line(
            "When you next cast an instant or sorcery spell that targets only a single opponent or a single permanent an opponent controls this turn, for each other opponent, choose that player or a permanent they control, copy that spell, and the copy targets the chosen player or permanent.",
            0,
        )
        .expect("correlated next-spell copy text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("correlated next-spell copy parser should not error")
            .expect("correlated next-spell copy parser should match");
        let [
            EffectAst::DelayedTriggerThisTurn {
                trigger:
                    TriggerSpec::SpellCast {
                        filter: Some(filter),
                        caster: PlayerFilter::You,
                        ..
                    },
                effects: delayed,
                one_shot: true,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one exact delayed spell watcher: {effects:#?}");
        };
        assert_eq!(filter.card_types, [CardType::Instant, CardType::Sorcery]);
        assert_eq!(filter.target_count, Some(ChoiceCount::exactly(1)));
        assert_eq!(filter.targets_only_player, Some(PlayerFilter::Opponent));
        assert!(filter.targets_only_any_of);
        assert_eq!(
            filter
                .targets_only_object
                .as_deref()
                .and_then(|target| target.controller.as_ref()),
            Some(&PlayerFilter::Opponent)
        );
        let [
            EffectAst::ForEachPlayersFiltered {
                filter: player_filter,
                effects: per_opponent,
            },
        ] = delayed.as_slice()
        else {
            panic!("expected one per-other-opponent loop: {delayed:#?}");
        };
        assert_eq!(
            player_filter,
            &PlayerFilter::excluding(
                PlayerFilter::Opponent,
                PlayerFilter::TargetPlayerOrControllerOfTarget,
            )
        );
        let debug = format!("{per_opponent:#?}");
        assert!(debug.contains("ObjectOrPlayer"), "{debug}");
        assert!(debug.contains("CopySpell"), "{debug}");
        assert!(debug.contains("RetargetStackObject"), "{debug}");
    }

    #[test]
    fn next_copy_loop_does_not_claim_a_broader_target_or_missing_choice() {
        for text in [
            "When you next cast an instant or sorcery spell that targets an opponent or a permanent an opponent controls this turn, for each other opponent, choose that player or a permanent they control, copy that spell, and the copy targets the chosen player or permanent.",
            "When you next cast an instant or sorcery spell that targets only a single opponent or a single permanent an opponent controls this turn, copy that spell.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).expect("near miss should lex");
            assert!(
                parse_next_cast_single_opponent_or_permanent_copy_loop(&tokens).is_none(),
                "near miss must not use the exact correlated route: {text}"
            );
        }
    }

    #[test]
    fn delayed_death_after_damage_by_previous_creature_keeps_both_identities() {
        let tokens = crate::lexer::lex_line(
            "Whenever a creature dealt damage by that creature dies this turn, its controller loses 2 life.",
            0,
        )
        .expect("damage-history death watcher should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("damage-history death watcher should not error")
            .expect("damage-history death watcher should match");
        let [
            EffectAst::DelayedTriggerThisTurn {
                trigger: TriggerSpec::Dies(victim),
                effects: delayed_effects,
                one_shot,
                attach_to_previous_ability,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one delayed death watcher: {effects:#?}");
        };

        assert_eq!(victim.card_types, [CardType::Creature]);
        assert_eq!(
            victim.dealt_damage_by_source_this_turn,
            Some(ironsmith_core::DamagedBySource::ThisCreature)
        );
        assert!(!*one_shot, "`Whenever` must remain repeatable this turn");
        assert!(
            *attach_to_previous_ability,
            "the damager must be the preceding creature target"
        );
        assert!(format!("{delayed_effects:#?}").contains("LoseLife"));
    }

    #[test]
    fn suffix_this_turn_first_matching_cast_is_one_shot() {
        let tokens = crate::lexer::lex_line(
            "When you cast a spell with the chosen name for the first time this turn, draw two cards.",
            0,
        )
        .expect("first-matching-cast delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("first-matching-cast delayed trigger parser should not error")
            .expect("first-matching-cast delayed trigger parser should match");

        let [
            EffectAst::DelayedTriggerThisTurn {
                trigger:
                    TriggerSpec::SpellCast {
                        filter: Some(filter),
                        caster: PlayerFilter::You,
                        ..
                    },
                effects: delayed_effects,
                one_shot,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one named-spell delayed trigger, got {effects:#?}");
        };

        assert_eq!(filter.name.as_deref(), Some("{chosen name}"));
        assert!(
            *one_shot,
            "the first matching cast must consume the trigger"
        );
        assert!(format!("{delayed_effects:#?}").contains("Draw"));
    }

    #[test]
    fn suffix_this_turn_delayed_trigger_supports_spell_or_loyalty_union() {
        let tokens = crate::lexer::lex_line(
            "When you next cast an instant spell, cast a sorcery spell, or activate a loyalty ability this turn, copy that spell or ability twice. You may choose new targets for the copies.",
            0,
        )
        .expect("next spell-or-loyalty delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("spell-or-loyalty delayed trigger parser should not error")
            .expect("spell-or-loyalty delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("Either"), "{debug}");
        assert!(debug.contains("SpellCast"), "{debug}");
        assert!(debug.contains("AbilityActivated"), "{debug}");
        assert!(debug.contains("loyalty_only: true"), "{debug}");
        assert!(debug.contains("CopySpell"), "{debug}");
        let [
            EffectAst::DelayedTriggerThisTurn {
                effects: delayed_effects,
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one delayed trigger effect, got {effects:#?}");
        };
        let [EffectAst::SubjectVerb(subject_verb)] = delayed_effects.as_slice() else {
            panic!("expected one delayed copy effect, got {delayed_effects:#?}");
        };
        let SubjectVerbActionAst::CopySpell {
            count,
            may_choose_new_targets,
            ..
        } = &subject_verb.action
        else {
            panic!("expected delayed copy spell action, got {subject_verb:#?}");
        };
        assert_eq!(*count, Value::Fixed(2));
        assert!(*may_choose_new_targets);
    }

    #[test]
    fn leading_this_turn_target_attack_unblocked_uses_captured_subject() {
        let tokens = crate::lexer::lex_line(
            "This turn, when target creature you control attacks and isn't blocked, draw a card.",
            0,
        )
        .expect("targeted attack-unblocked delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("targeted attack-unblocked delayed trigger parser should not error")
            .expect("targeted attack-unblocked delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("ChooseObjects"), "{debug}");
        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("AttacksAndIsntBlocked"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn suffix_this_turn_tagged_dealt_damage_uses_captured_kind() {
        let tokens = crate::lexer::lex_line(
            "Whenever that creature is dealt damage this turn, draw a card.",
            0,
        )
        .expect("tagged dealt-damage delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("tagged dealt-damage delayed trigger parser should not error")
            .expect("tagged dealt-damage delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("IsDealtDamage"), "{debug}");
        assert!(debug.contains("TaggedObjectConstraint"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }

    #[test]
    fn suffix_this_turn_tagged_combat_damage_uses_captured_marker() {
        let tokens = crate::lexer::lex_line(
            "Whenever that permanent is dealt combat damage this turn, draw a card.",
            0,
        )
        .expect("tagged combat-damage delayed trigger text should lex");

        let effects = parse_sentence_delayed_trigger_this_turn(&tokens)
            .expect("tagged combat-damage delayed trigger parser should not error")
            .expect("tagged combat-damage delayed trigger parser should match");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("DelayedTriggerThisTurn"), "{debug}");
        assert!(debug.contains("IsDealtCombatDamage"), "{debug}");
        assert!(debug.contains("TaggedObjectConstraint"), "{debug}");
        assert!(debug.contains("Draw"), "{debug}");
    }
