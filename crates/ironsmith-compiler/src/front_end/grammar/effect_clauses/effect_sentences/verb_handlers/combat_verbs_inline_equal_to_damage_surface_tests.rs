    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn historical_mixed_damage_recipients_keep_both_typed_filters() {
        let tokens = lex_line(
            "1 damage to each opponent and planeswalker it has dealt damage to this game",
            0,
        )
        .expect("historical damage clause should lex");
        let effect = parse_deal_damage(&tokens).expect("historical damage clause should parse");
        let debug = format!("{effect:#?}");
        assert!(
            debug.contains("WasDealtDamageBySourceThisGame"),
            "player history filter missing: {debug}"
        );
        assert!(
            debug.contains("was_dealt_damage_by_source_this_game: true"),
            "object history filter missing: {debug}"
        );
        assert!(
            debug.contains("Planeswalker"),
            "object domain missing: {debug}"
        );
        assert_eq!(debug.matches("DealDamage").count(), 2, "{debug}");
    }

    #[test]
    fn damage_to_each_of_those_preserves_the_demonstrative_set_surface() {
        let tokens = lex_line("X damage to each of those creatures", 0)
            .expect("demonstrative damage clause should lex");
        let effect = parse_deal_damage(&tokens).expect("demonstrative damage clause should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEach { filter, .. },
            ..
        }) = effect
        else {
            panic!("expected typed damage fanout: {effect:#?}");
        };

        assert_eq!(
            filter.set_quantifier_surface(),
            Some(ironsmith_core::SetQuantifierSurface::Those)
        );
    }

    #[test]
    fn fixed_plus_count_damage_keeps_equal_to_surface() {
        let tokens = lex_line(
            "damage equal to 2 plus the number of Lesson cards in your graveyard to target creature",
            0,
        )
        .expect("equal-to damage should lex");
        let effect = parse_deal_damage_equal_to_clause(&tokens)
            .expect("equal-to damage should parse")
            .expect("equal-to damage should match");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamage { amount, .. },
            ..
        }) = effect
        else {
            panic!("expected typed damage effect");
        };

        assert!(amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo));
        assert!(matches!(amount.unhinted(), Value::Add(_, _)));
    }

    #[test]
    fn equal_to_damage_keeps_authored_optional_single_target() {
        for (target_words, expected_count) in [
            (
                "up to one target creature or planeswalker",
                ChoiceCount::up_to(1),
            ),
            ("target creature or planeswalker", ChoiceCount::exactly(1)),
        ] {
            let tokens = lex_line(
                &format!("damage equal to that card's mana value to {target_words}"),
                0,
            )
            .expect("linked damage clause should lex");
            let effect = parse_deal_damage_equal_to_clause(&tokens)
                .expect("linked damage clause should parse")
                .expect("linked damage clause should match");
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::DealDamage { amount, target, .. },
                ..
            }) = effect
            else {
                panic!("expected typed damage effect");
            };
            let actual_count = match target {
                TargetAst::WithCount(_, count) => count,
                _ => ChoiceCount::exactly(1),
            };

            assert_eq!(actual_count, expected_count, "{target_words}");
            assert!(amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo));
            assert!(matches!(amount.unhinted(), Value::ManaValueOf(_)));
        }
    }

    #[test]
    fn relative_controller_count_preempts_the_permissive_filter_value() {
        let tokens = lex_line(
            "damage to target creature equal to the number of nonbasic lands that creature's controller controls",
            0,
        )
        .expect("relative-controller damage should lex");
        let effect = parse_deal_damage(&tokens).expect("relative-controller damage should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::DealDamage {
                    amount,
                    target: TargetAst::Object(target, Some(_), _),
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected typed targeted damage: {effect:#?}");
        };
        let Value::Count(counted) = amount.unhinted() else {
            panic!("expected typed land count: {amount:#?}");
        };

        assert_eq!(target.card_types, vec![crate::CardType::Creature]);
        assert_eq!(counted.card_types, vec![crate::CardType::Land]);
        assert!(!counted.card_types.contains(&crate::CardType::Creature));
        assert!(
            counted
                .excluded_supertypes
                .contains(&crate::Supertype::Basic)
        );
        assert_eq!(
            counted.controller,
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
        );
    }

    #[test]
    fn authored_damage_recipient_pronoun_is_preserved_on_the_amount() {
        for (text, expects_hint) in [
            ("2 damage to them", true),
            ("2 damage to that player", false),
        ] {
            let tokens = lex_line(text, 0).expect("damage clause should lex");
            let effect = parse_deal_damage(&tokens).expect("damage clause should parse");
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DealDamage {
                        amount,
                        target: TargetAst::Player(PlayerFilter::IteratedPlayer, _),
                        ..
                    },
                ..
            }) = effect
            else {
                panic!("expected iterated-player damage for {text}: {effect:#?}");
            };

            assert_eq!(
                amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::DamageRecipientPronoun),
                expects_hint,
                "{text}"
            );
        }
    }

    #[test]
    fn target_spell_controller_damage_materializes_the_spell_target_first() {
        for text in [
            "damage to target spell's controller equal to that spell's mana value",
            "damage equal to that spell's mana value to target spell's controller",
        ] {
            let tokens = lex_line(text, 0).expect("stack-target damage should lex");
            let effect = if text.starts_with("damage to") {
                parse_deal_damage_to_target_equal_to_clause(&tokens)
            } else {
                parse_deal_damage_equal_to_clause(&tokens)
            }
            .expect("stack-target damage should parse")
            .expect("stack-target damage should match");
            let EffectAst::Sequence { effects } = effect else {
                panic!("expected target prelude plus damage for {text}: {effect:#?}");
            };
            let [target, damage] = effects.as_slice() else {
                panic!("expected exactly two typed effects for {text}: {effects:#?}");
            };
            assert!(matches!(
                target,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::TargetOnly {
                        target: TargetAst::Spell(Some(_)),
                        explicit_declaration: false,
                    },
                    ..
                })
            ));
            assert!(matches!(
                damage,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::DealDamage {
                            amount,
                            target:
                                TargetAst::Player(
                                    PlayerFilter::ControllerOf(
                                        crate::target::ObjectRef::Tagged(tag)
                                    ),
                                    None
                                ),
                            ..
                        },
                    ..
                }) if tag.as_str() == IT_TAG
                    && matches!(
                        amount.unhinted(),
                        Value::ManaValueOf(spec)
                            if matches!(
                                spec.unhinted(),
                                ChooseSpec::Tagged(value_tag) if value_tag.as_str() == IT_TAG
                            )
                    )
            ));
        }
    }
