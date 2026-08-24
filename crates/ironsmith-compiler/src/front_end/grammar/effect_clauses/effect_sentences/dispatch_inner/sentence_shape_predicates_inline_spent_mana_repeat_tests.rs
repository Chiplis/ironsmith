    use super::*;
    use crate::IfResultPredicate;

    #[test]
    fn as_though_no_defender_preempts_the_broad_defender_grant_route() {
        let tokens = crate::lexer::lex_line(
            "This creature can attack this turn as though it didn't have defender.",
            0,
        )
        .expect("permission should lex");
        let effects = parse_effect_sentence_lexed(&tokens).expect("permission should parse");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("CanAttackAsThoughNoDefender"), "{debug}");
        assert!(!debug.contains("KeywordAction(Defender)"), "{debug}");

        let near_miss =
            crate::lexer::lex_line("This creature gains defender until end of turn.", 0)
                .expect("ordinary defender grant should lex");
        let effects = parse_effect_sentence_lexed(&near_miss)
            .expect("ordinary defender grant should still parse");
        let debug = format!("{effects:#?}");
        assert!(!debug.contains("CanAttackAsThoughNoDefender"), "{debug}");
        assert!(debug.contains("Defender"), "{debug}");

        let coordinated = crate::lexer::lex_line(
            "Target creature you control gets +1/+0 until end of turn and can attack as though it didn't have defender.",
            0,
        )
        .expect("coordinated permission should lex");
        let effects =
            parse_effect_sentence_lexed(&coordinated).expect("coordinated permission should parse");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("CanAttackAsThoughNoDefender"), "{debug}");
        assert!(debug.contains("Tagged"), "{debug}");
        assert!(!debug.contains("GrantAbilitiesAll"), "{debug}");
    }

    #[test]
    fn direct_sentence_route_keeps_put_history_inside_target_declaration() {
        let tokens = crate::lexer::lex_line(
            "Choose up to three target permanent cards in graveyards that were put there from the battlefield this turn.",
            0,
        )
        .expect("historical target declaration should lex");
        let effects = parse_effect_sentence_lexed(&tokens)
            .expect("direct sentence route should keep the complete target declaration");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 1, "{debug}");
        assert!(debug.contains("TargetOnly"), "{debug}");
        assert!(
            debug.contains("entered_graveyard_from_battlefield_this_turn: true"),
            "{debug}"
        );
        assert!(!debug.contains("MoveToZone"), "{debug}");
    }

    #[test]
    fn keyword_bundle_list_preempts_leading_duration_chain_splitting() {
        let tokens = crate::lexer::lex_line(
            "until end of turn, each other creature you control gets +1/+1 if it has flying, +1/+1 if it has first strike, and so on for double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, vigilance, and partner.",
            0,
        )
        .expect("keyword-bundle sentence should lex");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("keyword-bundle sentence should parse");

        assert_eq!(effects.len(), 14, "{effects:#?}");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("Flying"), "{debug}");
        assert!(debug.contains("Partner"), "{debug}");
    }

    #[test]
    fn for_each_mana_from_source_repeats_the_typed_effect() {
        let tokens = crate::lexer::lex_line(
            "For each mana from a Desert spent to cast this spell, create a tapped Treasure token.",
            0,
        )
        .expect("spent-mana sentence should lex");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("spent-mana sentence should parse");
        let [EffectAst::RepeatEffects { count, effects }] = effects.as_slice() else {
            panic!("expected one typed repeat effect, got {effects:#?}");
        };
        assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        let Value::ManaFromSourceSpentToCastThisSpell {
            source_filter,
            include_source_noun,
            ..
        } = count.unhinted()
        else {
            panic!("expected a mana-source repeat count, got {count:#?}");
        };
        assert!(!include_source_noun);
        assert_eq!(source_filter.subtypes, [crate::types::Subtype::Desert]);
        assert!(
            format!("{effects:#?}").contains("CreateToken"),
            "{effects:#?}"
        );
    }

    #[test]
    fn for_each_repeated_mana_symbol_uses_a_divided_typed_count() {
        let tokens = crate::lexer::lex_line("For each {U}{U} spent to cast it, draw a card.", 0)
            .expect("mana-symbol sentence should lex");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("mana-symbol sentence should parse");
        let [EffectAst::RepeatEffects { count, effects }] = effects.as_slice() else {
            panic!("expected one typed repeat effect, got {effects:#?}");
        };
        assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        assert!(matches!(
            count.unhinted(),
            Value::DividedRoundedDown(inner, 2)
                if matches!(
                    inner.as_ref(),
                    Value::ManaSymbolSpentToCastThisSpell {
                        symbol: crate::mana::ManaSymbol::Blue,
                        reference: ironsmith_core::ManaSpentCastReferenceSurface::It,
                    }
                )
        ));
        assert!(format!("{effects:#?}").contains("Draw"), "{effects:#?}");
    }

    #[test]
    fn conditional_quoted_grant_keeps_the_outer_gain_semantics() {
        let body_tokens = crate::lexer::lex_line(
            "The copy gains haste and \"At the beginning of the end step, sacrifice this permanent.\"",
            0,
        )
        .expect("quoted gain body should lex");
        let direct_gain = super::super::gain_ability::parse_gain_ability_sentence(&body_tokens)
            .expect("quoted gain body should parse without falling back")
            .expect("quoted gain body should be recognized as a gain");
        assert!(
            format!("{direct_gain:#?}").contains("GrantAbilitiesToTarget"),
            "{direct_gain:#?}"
        );

        let tokens = crate::lexer::lex_line(
            "If it's a permanent spell, the copy gains haste and \"At the beginning of the end step, sacrifice this permanent.\"",
            0,
        )
        .expect("conditional quoted grant should lex");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("conditional quoted grant should parse");
        let [EffectAst::Conditional { if_true, .. }] = effects.as_slice() else {
            panic!("expected one typed conditional, got {effects:#?}");
        };
        let debug = format!("{if_true:#?}");
        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
        assert!(debug.contains("Haste"), "{debug}");
        assert!(debug.contains("BeginningOfTheEndStep"), "{debug}");
        assert!(debug.contains("Sacrifice"), "{debug}");
    }

    #[test]
    fn quoted_restriction_grant_keeps_trailing_defending_player_unless_payment() {
        let tokens = crate::lexer::lex_line(
            "It gains \"This creature can't be blocked.\" until end of turn unless defending player sacrifices a creature of their choice.",
            0,
        )
        .expect("quoted restriction gain should lex");
        let effects = parse_effect_sentence_lexed(&tokens)
            .expect("quoted restriction gain should parse before the broad can't route");

        let [
            EffectAst::UnlessPays {
                effects: granted_effects,
                player: PlayerAst::Defending,
                cost,
                before_delayed_step: false,
            },
        ] = effects.as_slice()
        else {
            panic!("expected a defending-player unless payment, got {effects:#?}");
        };
        assert!(format!("{cost:#?}").contains("Sacrifice"), "{cost:#?}");
        let debug = format!("{granted_effects:#?}");
        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
        assert!(debug.contains("duration: EndOfTurn"), "{debug}");
        assert!(debug.contains("RuleRestriction"), "{debug}");
    }

    #[test]
    fn public_sentence_route_keeps_result_gated_unattach_delayed() {
        let tokens = crate::lexer::lex_line(
            "If you do, unattach it at the beginning of the next end step.",
            0,
        )
        .expect("delayed unattach sentence should lex");
        let effects = parse_effect_sentence_lexed(&tokens)
            .expect("public sentence route should preserve the delayed action");

        let [
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: gated,
            },
        ] = effects.as_slice()
        else {
            panic!("expected an outer result gate, got {effects:#?}");
        };
        let [
            EffectAst::DelayedUntilNextEndStep {
                player: PlayerFilter::Any,
                effects: delayed,
            },
        ] = gated.as_slice()
        else {
            panic!("expected a delayed next-end-step payload, got {gated:#?}");
        };
        assert!(
            matches!(
                delayed.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Unattach { .. },
                    ..
                })]
            ),
            "{delayed:#?}"
        );
    }
