use super::*;

    use super::*;

    #[test]
    fn parse_investigate_defaults_to_one() {
        let ast = parse_investigate(&[]).expect("parse investigate");
        assert!(matches!(
            ast,
            EffectAst::Investigate {
                count: Value::Fixed(1)
            }
        ));
    }

    #[test]
    fn parse_investigate_twice() {
        let tokens = tokenize_line("twice", 0);
        let ast = parse_investigate(&tokens).expect("parse investigate twice");
        assert!(matches!(
            ast,
            EffectAst::Investigate {
                count: Value::Fixed(2)
            }
        ));
    }

    #[test]
    fn parse_investigate_n_times() {
        let tokens = tokenize_line("three times", 0);
        let ast = parse_investigate(&tokens).expect("parse investigate three times");
        assert!(matches!(
            ast,
            EffectAst::Investigate {
                count: Value::Fixed(3)
            }
        ));
    }

    #[test]
    fn parse_look_top_x_cards_of_library() {
        let tokens = tokenize_line("the top X cards of your library", 0);
        let ast = parse_look(&tokens, None).expect("parse look with X count");
        assert!(matches!(
            ast,
            EffectAst::LookAtTopCards {
                player: PlayerAst::You,
                count: Value::X,
                ..
            }
        ));
    }

    #[test]
    fn parse_deal_damage_equal_to_each_opponent_wraps_for_each_opponent() {
        let tokens =
            tokenize_line("deals damage equal to the number of cards in your hand to each opponent", 0);
        let ast = parse_effect_clause(&tokens).expect("parse equal-to each opponent damage");
        assert!(matches!(ast, EffectAst::ForEachOpponent { .. }));
    }

    #[test]
    fn parse_deal_damage_equal_to_each_player_wraps_for_each_player() {
        let tokens =
            tokenize_line("deals damage equal to the number of cards in your hand to each player", 0);
        let ast = parse_effect_clause(&tokens).expect("parse equal-to each player damage");
        assert!(matches!(ast, EffectAst::ForEachPlayer { .. }));
    }

    #[test]
    fn parse_deal_damage_equal_to_each_other_player_wraps_for_each_opponent() {
        let tokens = tokenize_line(
            "deals damage equal to the number of cards in your hand to each other player",
            0,
        );
        let ast = parse_effect_clause(&tokens).expect("parse equal-to each other player damage");
        assert!(matches!(ast, EffectAst::ForEachOpponent { .. }));
    }

    #[test]
    fn parse_deal_damage_equal_to_power_each_opponent_wraps_for_each_opponent() {
        let tokens = tokenize_line("this creature deals damage equal to its power to each opponent", 0);
        let effects = parse_effect_sentence(&tokens).expect("parse equal-to-power each opponent");
        assert!(matches!(effects.as_slice(), [EffectAst::ForEachOpponent { .. }]));
    }

    #[test]
    fn parse_deal_damage_equal_to_power_each_player_wraps_for_each_player() {
        let tokens = tokenize_line("this creature deals damage equal to its power to each player", 0);
        let effects = parse_effect_sentence(&tokens).expect("parse equal-to-power each player");
        assert!(matches!(effects.as_slice(), [EffectAst::ForEachPlayer { .. }]));
    }

    #[test]
    fn parse_deal_damage_to_each_opponent_equal_to_power_wraps_for_each_opponent() {
        let tokens = tokenize_line("this creature deals damage to each opponent equal to its power", 0);
        let effects = parse_effect_sentence(&tokens).expect("parse to-each-opponent equal-to-power");
        assert!(matches!(effects.as_slice(), [EffectAst::ForEachOpponent { .. }]));
    }

    #[test]
    fn parse_attached_prevent_all_damage_dealt_by_enchanted_creature() {
        let tokens = tokenize_line(
            "Prevent all damage that would be dealt by enchanted creature.",
            0,
        );
        let abilities = parse_static_ability_line(&tokens)
            .expect("parse static ability line")
            .expect("expected static ability");
        assert_eq!(abilities.len(), 1);
        assert_eq!(abilities[0].id(), StaticAbilityId::AttachedAbilityGrant);
    }

    #[test]
    fn parse_prevent_damage_to_source_remove_counter_static_line() {
        let tokens = tokenize_line(
            "If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature.",
            0,
        );
        let abilities = parse_static_ability_line(&tokens)
            .expect("parse static ability line")
            .expect("expected static ability");
        assert!(abilities
            .iter()
            .any(|ability| ability.id() == StaticAbilityId::PreventDamageToSelfRemoveCounter));
    }

    #[test]
    fn parse_prevent_all_damage_to_source_by_creatures_static_line() {
        let tokens = tokenize_line(
            "Prevent all damage that would be dealt to this creature by creatures.",
            0,
        );
        let abilities = parse_static_ability_line(&tokens)
            .expect("parse static ability line")
            .expect("expected static ability");
        assert!(abilities
            .iter()
            .any(|ability| ability.id() == StaticAbilityId::PreventAllDamageToSelfByCreatures));
    }

    #[test]
    fn parse_line_prevent_all_damage_to_source_by_creatures_prefers_static() {
        let parsed = parse_line(
            "Prevent all damage that would be dealt to this creature by creatures.",
            0,
        )
        .expect("parse line");
        let ability = match parsed {
            LineAst::StaticAbility(ability) => ability,
            LineAst::StaticAbilities(mut abilities) if abilities.len() == 1 => abilities
                .pop()
                .expect("single static ability"),
            other => panic!("expected static ability parse, got {other:?}"),
        };
        assert_eq!(ability.id(), StaticAbilityId::PreventAllDamageToSelfByCreatures);
    }

    #[test]
    fn parse_line_prevent_damage_to_source_remove_counter_prefers_static() {
        let line =
            "If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature.";
        let parsed = parse_line(line, 0).expect("parse line");
        let ability = match parsed {
            LineAst::StaticAbility(ability) => ability,
            LineAst::StaticAbilities(mut abilities) if abilities.len() == 1 => abilities
                .pop()
                .expect("single static ability"),
            other => panic!("expected static ability parse, got {other:?}"),
        };
        assert_eq!(ability.id(), StaticAbilityId::PreventDamageToSelfRemoveCounter);
    }

    #[test]
    fn parse_prevent_all_combat_damage_to_source_static_line() {
        let tokens =
            tokenize_line("Prevent all combat damage that would be dealt to this creature.", 0);
        let abilities = parse_static_ability_line(&tokens)
            .expect("parse static ability line")
            .expect("expected static ability");
        assert!(abilities
            .iter()
            .any(|ability| ability.id() == StaticAbilityId::PreventAllCombatDamageToSelf));
    }

    #[test]
    fn parse_line_prevent_all_combat_damage_to_source_prefers_static() {
        let parsed = parse_line(
            "Prevent all combat damage that would be dealt to this creature.",
            0,
        )
        .expect("parse line");
        let ability = match parsed {
            LineAst::StaticAbility(ability) => ability,
            LineAst::StaticAbilities(mut abilities) if abilities.len() == 1 => abilities
                .pop()
                .expect("single static ability"),
            other => panic!("expected static ability parse, got {other:?}"),
        };
        assert_eq!(ability.id(), StaticAbilityId::PreventAllCombatDamageToSelf);
    }

    #[test]
    fn parse_creatures_with_power_or_greater_dont_untap_static_line() {
        let tokens = tokenize_line(
            "Creatures with power 3 or greater don't untap during their controllers' untap steps.",
            0,
        );
        let abilities = parse_static_ability_line(&tokens)
            .expect("parse static ability line")
            .expect("expected static ability");
        assert!(abilities
            .iter()
            .any(|ability| ability.id() == StaticAbilityId::RuleRestriction));
        assert!(abilities.iter().any(|ability| {
            let text = ability.display().to_ascii_lowercase();
            text.contains("power 3 or greater") && text.contains("untap during")
        }));
    }

    #[test]
    fn parse_line_creatures_with_power_or_greater_dont_untap_prefers_static() {
        let parsed = parse_line(
            "Creatures with power 3 or greater don't untap during their controllers' untap steps.",
            0,
        )
        .expect("parse line");
        let ability = match parsed {
            LineAst::StaticAbility(ability) => ability,
            LineAst::StaticAbilities(mut abilities) if abilities.len() == 1 => abilities
                .pop()
                .expect("single static ability"),
            other => panic!("expected static ability parse, got {other:?}"),
        };
        assert_eq!(ability.id(), StaticAbilityId::RuleRestriction);
    }

    #[test]
    fn parse_put_into_library_second_from_top_clause() {
        let tokens = tokenize_line(
            "Put target creature into its owner's library second from the top.",
            0,
        );
        let effects = parse_effect_sentence(&tokens).expect("parse put second-from-top sentence");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            EffectAst::MoveToLibrarySecondFromTop { .. }
        )));
    }

    #[test]
    fn parse_tap_then_it_doesnt_untap_next_step_clause() {
        let tokens = tokenize_line(
            "Tap that creature and it doesn't untap during its controller's next untap step.",
            0,
        );
        let effects = parse_effect_sentence(&tokens).expect("parse tap+untap-skip sentence");

        assert!(effects
            .iter()
            .any(|effect| matches!(effect, EffectAst::Tap { .. })));
        assert!(effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::Cant {
                    restriction: crate::effect::Restriction::Untap(_),
                    duration: crate::effect::Until::ControllersNextUntapStep,
                }
            )
        }));
    }

    #[test]
    fn parse_keyword_for_mirrodin_line() {
        let tokens = tokenize_line("For Mirrodin!", 0);
        let actions = parse_ability_line(&tokens).expect("expected keyword actions");
        assert!(actions
            .iter()
            .any(|action| matches!(action, KeywordAction::ForMirrodin)));
    }

    #[test]
    fn parse_keyword_living_weapon_line() {
        let tokens = tokenize_line("Living weapon", 0);
        let actions = parse_ability_line(&tokens).expect("expected keyword actions");
        assert!(actions
            .iter()
            .any(|action| matches!(action, KeywordAction::LivingWeapon)));
    }

    #[test]
    fn parse_attach_reverse_order_to_it_any_number_of_auras() {
        let tokens = tokenize_line("to it any number of auras on the battlefield", 0);
        let effect = parse_attach(&tokens).expect("reverse-order attach clause should parse");

        match effect {
            EffectAst::Attach { object: _, target } => match target {
                TargetAst::Tagged(tag, _) => {
                    assert_eq!(tag.as_str(), IT_TAG, "expected attach target to reference 'it'");
                }
                other => panic!("expected tagged attach target, got {other:?}"),
            },
            other => panic!("expected attach effect, got {other:?}"),
        }
    }

    #[test]
    fn parse_clash_clause() {
        let tokens = tokenize_line("Clash with an opponent.", 0);
        let effects = parse_effect_sentence(&tokens).expect("parse effect sentence");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            EffectAst::Clash {
                opponent: ClashOpponentAst::Opponent
            }
        )));
    }

    #[test]
    fn parse_clash_with_defending_player_clause() {
        let tokens = tokenize_line("Clash with defending player.", 0);
        let effects = parse_effect_sentence(&tokens).expect("parse effect sentence");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            EffectAst::Clash {
                opponent: ClashOpponentAst::DefendingPlayer
            }
        )));
    }

    #[test]
    fn parse_clash_then_return_clause() {
        let tokens = tokenize_line(
            "Clash with an opponent, then return target creature to its owner's hand.",
            0,
        );
        let effects = parse_effect_sentence(&tokens).expect("parse effect sentence");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            EffectAst::Clash {
                opponent: ClashOpponentAst::Opponent
            }
        )));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, EffectAst::ReturnToHand { .. })));
    }

    #[test]
    fn parse_soulbond_shared_power_toughness_line() {
        let tokens = tokenize_line(
            "As long as this creature is paired with another creature, each of those creatures gets +2/+2.",
            0,
        );
        let abilities = parse_static_ability_line(&tokens)
            .expect("parse static ability line")
            .expect("expected static abilities");
        assert_eq!(abilities.len(), 1);
        assert!(abilities[0]
            .display()
            .contains("paired with another creature"));
        assert!(abilities[0].display().contains("+2/+2"));
    }

    #[test]
    fn parse_soulbond_shared_keyword_line() {
        let tokens = tokenize_line(
            "As long as this creature is paired with another creature, both creatures have flying.",
            0,
        );
        let abilities = parse_static_ability_line(&tokens)
            .expect("parse static ability line")
            .expect("expected static abilities");
        assert_eq!(abilities.len(), 1);
        assert!(abilities[0]
            .display()
            .contains("both creatures have Flying"));
    }

    #[test]
    fn parse_if_you_win_as_if_result_predicate() {
        let tokens = tokenize_line("If you win, put a +1/+1 counter on this creature.", 0);
        let effects = parse_effect_sentence(&tokens).expect("parse effect sentence");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                ..
            }
        )));
    }

    #[test]
    fn parse_if_that_spell_is_countered_this_way_as_if_result_predicate() {
        let tokens = tokenize_line(
            "If that spell is countered this way, exile it instead of putting it into its owners graveyard.",
            0,
        );
        let effects = parse_effect_sentence(&tokens).expect("parse countered-this-way predicate");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                ..
            }
        )));
    }

    #[test]
    fn parse_predicate_that_player_has_cards_in_hand_or_more() {
        let tokens = tokenize_line("that player has seven or more cards in hand", 0);
        let predicate = parse_predicate(&tokens).expect("parse cards-in-hand predicate");
        assert!(matches!(
            predicate,
            PredicateAst::PlayerCardsInHandOrMore {
                player: PlayerAst::That,
                count: 7
            }
        ));
    }

    #[test]
    fn parse_predicate_that_player_has_cards_in_hand_or_fewer() {
        let tokens = tokenize_line("that player has two or fewer cards in hand", 0);
        let predicate = parse_predicate(&tokens).expect("parse cards-in-hand predicate");
        assert!(matches!(
            predicate,
            PredicateAst::PlayerCardsInHandOrFewer {
                player: PlayerAst::That,
                count: 2
            }
        ));
    }

    #[test]
    fn parse_predicate_creature_died_this_turn() {
        let tokens = tokenize_line("a creature died this turn", 0);
        let predicate = parse_predicate(&tokens).expect("parse creature-died predicate");
        assert!(matches!(predicate, PredicateAst::CreatureDiedThisTurn));
    }

    #[test]
    fn parse_predicate_you_had_land_enter_battlefield_under_your_control_this_turn() {
        let tokens = tokenize_line(
            "you had a land enter the battlefield under your control this turn",
            0,
        );
        let predicate = parse_predicate(&tokens).expect("parse landfall-history predicate");
        assert!(matches!(
            predicate,
            PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
                player: PlayerAst::You
            }
        ));
    }

    #[test]
    fn parse_predicate_you_cast_it() {
        let tokens = tokenize_line("you cast it", 0);
        let predicate = parse_predicate(&tokens).expect("parse you-cast-it predicate");
        assert!(matches!(predicate, PredicateAst::SourceWasCast));
    }

    #[test]
    fn parse_predicate_its_your_turn() {
        let tokens = tokenize_line("its your turn", 0);
        let predicate = parse_predicate(&tokens).expect("parse your-turn predicate");
        assert!(matches!(predicate, PredicateAst::YourTurn));
    }

    #[test]
    fn parse_predicate_cards_in_your_graveyard_threshold() {
        let tokens = tokenize_line("there are seven or more cards in your graveyard", 0);
        let predicate = parse_predicate(&tokens).expect("parse graveyard-threshold predicate");
        assert!(matches!(
            predicate,
            PredicateAst::PlayerControlsAtLeast {
                player: PlayerAst::You,
                filter,
                count: 7
            } if filter.zone == Some(Zone::Graveyard)
        ));
    }

    #[test]
    fn parse_predicate_instant_or_sorcery_cards_in_graveyard_threshold() {
        let tokens = tokenize_line(
            "there are two or more instant and or sorcery cards in your graveyard",
            0,
        );
        let predicate = parse_predicate(&tokens).expect("parse instants-or-sorceries threshold");
        assert!(matches!(
            predicate,
            PredicateAst::PlayerControlsAtLeast {
                player: PlayerAst::You,
                filter,
                count: 2
            } if filter.zone == Some(Zone::Graveyard)
                && filter.card_types.contains(&CardType::Instant)
                && filter.card_types.contains(&CardType::Sorcery)
        ));
    }

    #[test]
    fn parse_predicate_card_types_among_cards_in_graveyard_threshold() {
        let tokens = tokenize_line(
            "there are four or more card types among cards in your graveyard",
            0,
        );
        let predicate = parse_predicate(&tokens).expect("parse delirium predicate");
        assert!(matches!(
            predicate,
            PredicateAst::PlayerHasCardTypesInGraveyardOrMore {
                player: PlayerAst::You,
                count: 4
            }
        ));
    }

    #[test]
    fn parse_if_its_your_turn_sentence_clause() {
        crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Fated Predicate Parse Probe",
        )
        .parse_text(
            "This spell deals 5 damage to target creature.\nIf it's your turn, scry 2.",
        )
        .expect("parse if-its-your-turn conditional clause");
    }

    #[test]
    fn parse_threshold_cards_in_graveyard_clause() {
        crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Threshold Predicate Parse Probe",
        )
        .parse_text("If there are seven or more cards in your graveyard, creatures can't block this turn.")
        .expect("parse threshold-style graveyard card count predicate");
    }

    #[test]
    fn parse_choose_target_creature_prelude_sentence() {
        let tokens = tokenize_line("Choose target creature. It gets +2/+2 until end of turn.", 0);
        let effects =
            parse_effect_sentences(&tokens).expect("parse choose-target prelude sentence");
        assert!(matches!(
            effects.first(),
            Some(EffectAst::TargetOnly {
                target: TargetAst::Object(_, _, _)
            })
        ));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, EffectAst::Pump { .. })));
    }

    #[test]
    fn parse_choose_target_opponent_prelude_sentence() {
        let tokens = tokenize_line("Choose target opponent. That player discards a card.", 0);
        let effects =
            parse_effect_sentences(&tokens).expect("parse choose-target-opponent prelude");
        assert!(matches!(
            effects.first(),
            Some(EffectAst::TargetOnly {
                target: TargetAst::Player(_, _)
            })
        ));
        assert!(effects
            .iter()
            .any(|effect| matches!(effect, EffectAst::Discard { .. })));
    }

    #[test]
    fn parse_spells_cost_modifier_colored_increase() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Mana Cost Increase Parse Probe",
        )
        .parse_text("Black spells you cast cost {B} more to cast.")
        .expect("parse colored cost increase");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if static_ability
                .cost_increase_mana_cost()
                .is_some_and(|modifier| modifier.increase.to_oracle() == "{B}")
            {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected colored mana-symbol cost increase in parsed static abilities"
        );
    }

    #[test]
    fn parse_spells_cost_modifier_multicolor_increase() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Multicolor Cost Reduction Parse Probe",
        )
        .parse_text("Cleric spells you cast cost {W}{B} less to cast.")
        .expect("parse multicolor cost reduction");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if static_ability
                .cost_reduction_mana_cost()
                .is_some_and(|modifier| modifier.reduction.to_oracle() == "{W}{B}")
            {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected multicolor mana-symbol cost reduction in parsed static abilities"
        );
    }

    #[test]
    fn parse_spells_cost_modifier_with_during_other_turns_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Naiad Condition Parse Probe",
        )
        .parse_text("During turns other than yours, spells you cast cost {1} less to cast.")
        .expect("parse turn-conditioned cost reduction");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.cost_reduction()
                && matches!(
                    (&modifier.reduction, &modifier.condition),
                    (
                        Value::Fixed(1),
                        Some(crate::ConditionExpr::Not(inner))
                    ) if matches!(inner.as_ref(), crate::ConditionExpr::YourTurn)
                )
            {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected turn-conditioned generic cost reduction for spells you cast"
        );
    }

    #[test]
    fn parse_spells_cost_modifier_with_as_long_as_tapped_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Centaur Omenreader Parse Probe",
        )
        .parse_text("As long as this creature is tapped, creature spells you cast cost {2} less to cast.")
        .expect("parse tapped-conditioned cost reduction");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.cost_reduction()
                && modifier.reduction == Value::Fixed(2)
                && matches!(
                    modifier.condition,
                    Some(crate::ConditionExpr::SourceIsTapped)
                )
            {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected source-tapped condition on creature spell cost reduction"
        );
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_during_your_turn_and_mixed_mana() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Discontinuity Parse Probe",
        )
        .parse_text(
            "During your turn, this spell costs {2}{U}{U} less to cast.\nDraw a card.",
        )
        .expect("parse this-spell mixed-mana reduction with turn condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction_mana_cost()
                && modifier.reduction.to_oracle() == "{2}{U}{U}"
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::YourTurn
                )
            {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected this-spell mixed-mana reduction with during-your-turn condition"
        );
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_opponent_drew_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Even the Score Parse Probe",
        )
        .parse_text(
            "This spell costs {U}{U}{U} less to cast if an opponent has drawn four or more cards this turn.\nDraw X cards.",
        )
        .expect("parse this-spell colored reduction with draw condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction_mana_cost()
                && modifier.reduction.to_oracle() == "{U}{U}{U}"
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(4)
                )
            {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected conditional this-spell colored reduction with opponent-draw condition"
        );
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_opponent_cast_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Ertai's Scorn Parse Probe",
        )
        .parse_text(
            "This spell costs {U} less to cast if an opponent cast two or more spells this turn.\nCounter target spell.",
        )
        .expect("parse this-spell colored reduction with cast condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction_mana_cost()
                && modifier.reduction.to_oracle() == "{U}"
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(2)
                )
            {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected conditional this-spell colored reduction with opponent-cast condition"
        );
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_you_control_condition_expr() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Wizard Discount Parse Probe",
        )
        .parse_text("This spell costs {1} less to cast if you control a wizard.\nDraw a card.")
        .expect("parse this-spell reduction with you-control condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::Fixed(1)
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::ConditionExpr { .. }
                )
            {
                found = true;
                break;
            }
        }
        assert!(
            found,
            "expected this-spell reduction with parsed condition expression"
        );
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_targets_object_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Tapped Target Discount Parse Probe",
        )
        .parse_text(
            "This spell costs {2} less to cast if it targets a tapped creature.\nDestroy target creature.",
        )
        .expect("parse this-spell reduction with target condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::Fixed(2)
                && let crate::static_abilities::ThisSpellCostCondition::TargetsObject(filter) =
                    &modifier.condition
                && filter.tapped
                && filter.card_types.contains(&CardType::Creature)
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected tapped-creature target condition");
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_graveyard_count_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Graveyard Count Discount Parse Probe",
        )
        .parse_text(
            "This spell costs {3} less to cast if you have nine or more cards in your graveyard.\nDraw a card.",
        )
        .expect("parse this-spell reduction with graveyard-count condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::Fixed(3)
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(9)
                )
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected graveyard-count condition");
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_creature_attacking_you_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Attack Trap Parse Probe",
        )
        .parse_text(
            "This spell costs {2} less to cast if a creature is attacking you.\nDestroy target attacking creature.",
        )
        .expect("parse this-spell reduction with attacking-you condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::Fixed(2)
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::CreatureIsAttackingYou
                )
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected attacking-you condition");
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_night_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Night Discount Parse Probe",
        )
        .parse_text(
            "This spell costs {2} less to cast if it's night.\nThis spell deals 3 damage to any target.",
        )
        .expect("parse this-spell reduction with night condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::Fixed(2)
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::IsNight
                )
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected night condition");
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_sacrificed_artifact_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Artifact Sacrifice Discount Parse Probe",
        )
        .parse_text(
            "This spell costs {3} less to cast if you've sacrificed an artifact this turn.\nThis spell can't be countered.\nThis spell deals 4 damage to target creature.",
        )
        .expect("parse this-spell reduction with artifact-sacrifice condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::Fixed(3)
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::YouSacrificedArtifactThisTurn
                )
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected artifact-sacrifice condition");
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_creature_left_battlefield_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Creature Left Battlefield Discount Parse Probe",
        )
        .parse_text(
            "This spell costs {2} less to cast if a creature left the battlefield under your control this turn.\nDraw a card.",
        )
        .expect("parse this-spell reduction with creature-left condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::Fixed(2)
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::CreatureLeftBattlefieldUnderYourControlThisTurn
                )
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected creature-left-battlefield condition");
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_committed_crime_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Crime Discount Parse Probe",
        )
        .parse_text(
            "This spell costs {1} less to cast if you've committed a crime this turn.\nDraw two cards.",
        )
        .expect("parse this-spell reduction with committed-crime condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::Fixed(1)
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::YouCommittedCrimeThisTurn
                )
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected committed-crime condition");
    }

    #[test]
    fn parse_this_spell_cost_modifier_with_only_named_other_creatures_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Mothrider Condition Parse Probe",
        )
        .parse_text(
            "This spell costs {2} less to cast if you have no other creature cards in hand or if the only other creature cards in your hand are named Mothrider Cavalry.\nFlying\nOther creatures you control get +1/+1.",
        )
        .expect("parse this-spell reduction with named-creatures-in-hand condition");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::Fixed(2)
                && let crate::static_abilities::ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(name) =
                    &modifier.condition
                && name == "mothrider cavalry"
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected named-creatures-in-hand condition");
    }

    #[test]
    fn parse_if_this_spell_costs_x_less_where_difference_condition() {
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Starting Life Difference Discount Parse Probe",
        )
        .parse_text(
            "If your life total is less than your starting life total, this spell costs {X} less to cast, where X is the difference.",
        )
        .expect("parse leading-if this-spell X reduction");

        let mut found = false;
        for ability in &card.abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(modifier) = static_ability.this_spell_cost_reduction()
                && modifier.reduction == Value::X
                && matches!(
                    modifier.condition,
                    crate::static_abilities::ThisSpellCostCondition::LifeTotalLessThanStarting
                )
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected starting-life-difference condition");
    }

    #[test]
    fn parse_object_filter_spell_or_permanent_builds_zone_disjunction() {
        let tokens = tokenize_line("spell or permanent", 0);
        let filter = parse_object_filter(&tokens, false).expect("parse mixed spell/permanent");
        assert_eq!(filter.any_of.len(), 2);
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.zone == Some(Zone::Stack)),
            "expected stack branch for spell targets"
        );
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.zone == Some(Zone::Battlefield)),
            "expected battlefield branch for permanent targets"
        );
    }

    #[test]
    fn parse_object_filter_permanent_spell_stays_stack_only() {
        let tokens = tokenize_line("blue permanent spell", 0);
        let filter = parse_object_filter(&tokens, false).expect("parse permanent spell filter");
        assert!(
            filter.any_of.is_empty(),
            "permanent spell should not become a spell/permanent disjunction"
        );
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert!(
            !filter.card_types.is_empty() || !filter.all_card_types.is_empty(),
            "permanent spell filter should preserve permanent card-type restriction"
        );
    }

    #[test]
    fn parse_object_filter_spell_or_nonland_permanent_preserves_nonland_branch() {
        let tokens = tokenize_line("spell or nonland permanent opponent controls", 0);
        let filter =
            parse_object_filter(&tokens, false).expect("parse spell-or-nonland-permanent filter");
        assert_eq!(filter.any_of.len(), 2);
        let battlefield_branch = filter
            .any_of
            .iter()
            .find(|branch| branch.zone == Some(Zone::Battlefield))
            .expect("expected battlefield branch");
        assert!(
            battlefield_branch.excluded_card_types.contains(&CardType::Land),
            "nonland qualifier should stay on battlefield permanent branch"
        );
    }

    #[test]
    fn parse_object_filter_permanents_and_permanent_spells_split_branches() {
        let tokens = tokenize_line("nonland permanents you control and permanent spells you control", 0);
        let filter =
            parse_object_filter(&tokens, false).expect("parse permanents and permanent spells");
        assert_eq!(filter.any_of.len(), 2);
        let stack_branch = filter
            .any_of
            .iter()
            .find(|branch| branch.zone == Some(Zone::Stack))
            .expect("expected stack branch");
        assert!(
            !stack_branch.card_types.is_empty() || !stack_branch.all_card_types.is_empty(),
            "permanent-spell branch should keep permanent type restriction"
        );
    }

    #[test]
    fn parse_object_filter_spell_from_hand_keeps_origin_zone() {
        let tokens = tokenize_line("instant or sorcery spell from your hand", 0);
        let filter = parse_object_filter(&tokens, false).expect("parse spell-origin filter");
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
    }

    #[test]
    fn parse_object_filter_spell_with_source_linked_exile_reference_stays_on_stack() {
        let tokens = tokenize_line("spell with the same name as a card exiled with this creature", 0);
        let filter =
            parse_object_filter(&tokens, false).expect("parse spell with source-linked exile ref");
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert!(
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            }),
            "expected source-linked exile tagged constraint"
        );
    }

    #[test]
    fn parse_target_phrase_spell_cast_from_graveyard_uses_spell_origin_zone() {
        let tokens = tokenize_line("target spell cast from a graveyard", 0);
        let target = parse_target_phrase(&tokens).expect("parse target spell cast from graveyard");
        let TargetAst::Object(filter, _, _) = target else {
            panic!("expected object target");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parse_trigger_clause_player_subject_attack_uses_one_or_more() {
        let tokens = tokenize_line("you attack", 0);
        let trigger = parse_trigger_clause(&tokens).expect("parse trigger clause");
        match trigger {
            TriggerSpec::AttacksOneOrMore(filter) => {
                assert_eq!(filter.controller, Some(PlayerFilter::You));
                assert!(filter.card_types.contains(&CardType::Creature));
            }
            other => panic!("expected AttacksOneOrMore trigger, got {other:?}"),
        }
    }

    #[test]
    fn parse_trigger_clause_opponent_attacks_you_uses_one_or_more() {
        let tokens = tokenize_line("an opponent attacks you or a planeswalker you control", 0);
        let trigger = parse_trigger_clause(&tokens).expect("parse trigger clause");
        match trigger {
            TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(filter) => {
                assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
                assert!(filter.card_types.contains(&CardType::Creature));
            }
            other => panic!(
                "expected AttacksYouOrPlaneswalkerYouControlOneOrMore trigger, got {other:?}"
            ),
        }
    }

    #[test]
    fn parse_trigger_clause_player_subject_combat_damage_uses_one_or_more() {
        let tokens = tokenize_line("you deal combat damage to a player", 0);
        let trigger = parse_trigger_clause(&tokens).expect("parse trigger clause");
        match trigger {
            TriggerSpec::DealsCombatDamageToPlayerOneOrMore(filter) => {
                assert_eq!(filter.controller, Some(PlayerFilter::You));
                assert!(filter.card_types.contains(&CardType::Creature));
            }
            other => panic!(
                "expected DealsCombatDamageToPlayerOneOrMore trigger, got {other:?}"
            ),
        }
    }

    #[test]
    fn parse_trigger_clause_this_deals_combat_damage_to_creature() {
        let tokens = tokenize_line("this creature deals combat damage to a creature", 0);
        let trigger = parse_trigger_clause(&tokens).expect("parse trigger clause");
        match trigger {
            TriggerSpec::ThisDealsCombatDamageTo(filter) => {
                assert!(filter.card_types.contains(&CardType::Creature));
            }
            other => panic!("expected ThisDealsCombatDamageTo trigger, got {other:?}"),
        }
    }

    #[test]
    fn parse_trigger_clause_filtered_source_deals_combat_damage_to_creature() {
        let tokens = tokenize_line("a sliver deals combat damage to a creature", 0);
        let trigger = parse_trigger_clause(&tokens).expect("parse trigger clause");
        match trigger {
            TriggerSpec::DealsCombatDamageTo { source, target } => {
                assert!(source.card_types.contains(&CardType::Creature));
                assert!(
                    source.description().contains("sliver"),
                    "expected sliver source filter, got {}",
                    source.description()
                );
                assert!(target.card_types.contains(&CardType::Creature));
            }
            other => panic!("expected DealsCombatDamageTo trigger, got {other:?}"),
        }
    }

    #[test]
    fn parse_trigger_clause_combat_damage_to_one_of_your_opponents() {
        let tokens = tokenize_line("a creature deals combat damage to one of your opponents", 0);
        let trigger = parse_trigger_clause(&tokens).expect("parse trigger clause");
        match trigger {
            TriggerSpec::DealsCombatDamageToPlayer(filter) => {
                assert!(filter.card_types.contains(&CardType::Creature));
            }
            other => panic!("expected DealsCombatDamageToPlayer trigger, got {other:?}"),
        }
    }

    #[test]
    fn parse_trigger_clause_this_deals_combat_damage_without_recipient() {
        let tokens = tokenize_line("this creature deals combat damage", 0);
        let trigger = parse_trigger_clause(&tokens).expect("parse trigger clause");
        match trigger {
            TriggerSpec::ThisDealsCombatDamage => {}
            other => panic!("expected ThisDealsCombatDamage trigger, got {other:?}"),
        }
    }

    #[test]
    fn parse_trigger_clause_this_deals_damage_to_an_opponent() {
        let tokens = tokenize_line("this creature deals damage to an opponent", 0);
        let trigger = parse_trigger_clause(&tokens).expect("parse trigger clause");
        match trigger {
            TriggerSpec::ThisDealsDamageToPlayer { player, amount } => {
                assert_eq!(player, PlayerFilter::Opponent);
                assert!(amount.is_none());
            }
            other => panic!("expected ThisDealsDamageToPlayer trigger, got {other:?}"),
        }
    }

    #[test]
    fn parse_trigger_clause_this_deals_damage_to_a_player() {
        let tokens = tokenize_line("this deals damage to a player", 0);
        let trigger = parse_trigger_clause(&tokens).expect("parse trigger clause");
        match trigger {
            TriggerSpec::ThisDealsDamageToPlayer { player, amount } => {
                assert_eq!(player, PlayerFilter::Any);
                assert!(amount.is_none());
            }
            other => panic!("expected ThisDealsDamageToPlayer trigger, got {other:?}"),
        }
    }

    #[test]
    fn parse_effect_clause_player_gets_multiple_poison_counters() {
        let tokens = tokenize_line("that player gets two poison counters", 0);
        let effect = parse_effect_clause(&tokens).expect("parse effect clause");
        match effect {
            EffectAst::PoisonCounters { count, player } => {
                assert_eq!(count, Value::Fixed(2));
                assert_eq!(player, PlayerAst::That);
            }
            other => panic!("expected PoisonCounters effect, got {other:?}"),
        }
    }

    #[test]
    fn parse_effect_clause_remove_all_minus_counters_from_it() {
        let tokens = tokenize_line("remove all -1/-1 counters from it", 0);
        let effect = parse_effect_clause(&tokens).expect("parse effect clause");
        match effect {
            EffectAst::RemoveUpToAnyCounters {
                amount,
                target,
                counter_type,
                up_to,
            } => {
                assert_eq!(amount, Value::CountersOnSource(CounterType::MinusOneMinusOne));
                assert!(matches!(target, TargetAst::Source(_)));
                assert_eq!(counter_type, Some(CounterType::MinusOneMinusOne));
                assert!(!up_to);
            }
            other => panic!("expected RemoveUpToAnyCounters effect, got {other:?}"),
        }
    }


    #[test]
    fn parse_prevent_next_time_damage_sentence_source_of_your_choice_any_target() {
        let tokens = tokenize_line(
            "The next time a source of your choice would deal damage to any target this turn, prevent that damage.",
            0,
        );
        let effects = parse_effect_sentence(&tokens).expect("parse effect sentence");
        assert!(effects.iter().any(|e| matches!(
            e,
            EffectAst::PreventNextTimeDamage {
                source: PreventNextTimeDamageSourceAst::Choice,
                target: PreventNextTimeDamageTargetAst::AnyTarget
            }
        )));
    }

    #[test]
    fn parse_redirect_next_damage_sentence_to_target_creature() {
        let tokens = tokenize_line(
            "The next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead.",
            0,
        );
        let effects = parse_effect_sentence(&tokens).expect("parse effect sentence");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            EffectAst::RedirectNextDamageFromSourceToTarget {
                amount: Value::Fixed(1),
                ..
            }
        )));
    }

    #[test]
    fn parse_redirect_next_time_source_damage_to_this_creature() {
        let tokens = tokenize_line(
            "The next time a source of your choice would deal damage to target creature this turn, that damage is dealt to this creature instead.",
            0,
        );
        let effects = parse_effect_sentence(&tokens).expect("parse effect sentence");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            EffectAst::RedirectNextTimeDamageToSource {
                source: PreventNextTimeDamageSourceAst::Choice,
                ..
            }
        )));
    }

    #[test]
    fn parse_activated_discard_random_cost_to_effect_cost() {
        let tokens = tokenize_line(
            "{R}, Discard a card at random: This creature gets +3/+0 until end of turn.",
            0,
        );
        let parsed = parse_activated_line(&tokens)
            .expect("parse activated line")
            .expect("expected activated ability");

        let AbilityKind::Activated(activated) = parsed.ability.kind else {
            panic!("expected activated ability");
        };

        let has_random_discard_cost = activated.mana_cost.costs().iter().any(|cost| {
            cost.effect_ref().is_some_and(|effect| {
                effect
                    .downcast_ref::<crate::effects::DiscardEffect>()
                    .is_some_and(|discard| discard.random)
            })
        });
        assert!(
            has_random_discard_cost,
            "expected random discard effect-backed cost"
        );
    }

    #[test]
    fn parse_gain_life_equal_to_sacrificed_creature_toughness_clause() {
        let tokens = tokenize_line("life equal to the sacrificed creature's toughness", 0);
        let effect = parse_gain_life(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse gain life equal to sacrificed creature toughness");
        assert!(matches!(
            effect,
            EffectAst::GainLife {
                amount: Value::ToughnessOf(spec),
                player: PlayerAst::You,
            } if matches!(
                spec.as_ref(),
                ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG
            )
        ));
    }

    #[test]
    fn parse_gain_life_equal_to_devotion_clause() {
        let tokens = tokenize_line("life equal to your devotion to green", 0);
        let effect = parse_gain_life(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse gain life equal to devotion");
        assert!(matches!(
            effect,
            EffectAst::GainLife {
                amount: Value::Devotion {
                    player: PlayerFilter::You,
                    color: crate::color::Color::Green
                },
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_gain_life_equal_to_life_lost_this_way_clause() {
        let tokens = tokenize_line("life equal to the life lost this way", 0);
        let effect = parse_gain_life(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse gain life equal to life lost this way");
        assert!(matches!(
            effect,
            EffectAst::GainLife {
                amount: Value::EventValue(EventValueSpec::LifeAmount),
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_cards_equal_to_number_of_named_cards_in_graveyards() {
        let tokens = tokenize_line(
            "cards equal to the number of cards named accumulated knowledge in all graveyards",
            0,
        );
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw equal to number-of filter");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::Count(filter),
                player: PlayerAst::You,
            } if filter.zone == Some(Zone::Graveyard)
                && filter.name.as_deref() == Some("accumulated knowledge")
        ));
    }

    #[test]
    fn parse_draw_cards_equal_to_greatest_power_among_creatures() {
        let tokens = tokenize_line(
            "cards equal to the greatest power among creatures you control",
            0,
        );
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw equal to aggregate filter");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::GreatestPower(filter),
                player: PlayerAst::You,
            } if filter.controller == Some(PlayerFilter::You)
                && filter.card_types.contains(&CardType::Creature)
        ));
    }

    #[test]
    fn parse_draw_cards_equal_to_number_of_hand_plus_one() {
        let tokens = tokenize_line(
            "cards equal to the number of cards in your hand plus one",
            0,
        );
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw equal to number-of filter plus fixed");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::Add(left, right),
                player: PlayerAst::You,
            } if matches!(
                (left.as_ref(), right.as_ref()),
                (Value::Count(filter), Value::Fixed(1))
                    if filter.zone == Some(Zone::Hand)
                        && filter.owner == Some(PlayerFilter::You)
            )
        ));
    }

    #[test]
    fn parse_draw_cards_equal_to_that_spells_mana_value() {
        let tokens = tokenize_line("cards equal to that spells mana value", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw equal to tagged mana value");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::ManaValueOf(spec),
                player: PlayerAst::You,
            } if matches!(
                spec.as_ref(),
                ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG
            )
        ));
    }

    #[test]
    fn parse_draw_another_card_as_fixed_one() {
        let tokens = tokenize_line("another card", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw another card");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::Fixed(1),
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_cards_equal_to_devotion() {
        let tokens = tokenize_line("cards equal to your devotion to red", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw equal to devotion");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::Devotion {
                    player: PlayerFilter::You,
                    color: crate::color::Color::Red,
                },
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_cards_equal_to_number_of_opponents_you_have() {
        let tokens = tokenize_line("cards equal to the number of opponents you have", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw equal to number of opponents");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::CountPlayers(PlayerFilter::Opponent),
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_cards_equal_to_number_of_oil_counters_on_it() {
        let tokens = tokenize_line("cards equal to the number of oil counters on it", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw equal to counters on source");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::CountersOnSource(CounterType::Oil),
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_cards_equal_to_sacrificed_permanent_mana_value() {
        let tokens = tokenize_line(
            "cards equal to the mana value of the sacrificed permanent",
            0,
        );
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw equal to sacrificed permanent mana value");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::ManaValueOf(spec),
                player: PlayerAst::You,
            } if matches!(
                spec.as_ref(),
                ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG
            )
        ));
    }

    #[test]
    fn parse_draw_as_many_cards_as_discarded_this_way() {
        let tokens = tokenize_line("as many cards as they discarded this way", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw as-many previous-event amount");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::EventValue(EventValueSpec::Amount),
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_that_many_cards_plus_one() {
        let tokens = tokenize_line("that many cards plus one", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw that-many cards plus one");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::EventValueOffset(EventValueSpec::Amount, 1),
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_three_cards_instead_trailing_clause() {
        let tokens = tokenize_line("three cards instead", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw with trailing instead clause");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::Fixed(3),
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_an_additional_card_clause() {
        let tokens = tokenize_line("an additional card", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw with additional card wording");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::Fixed(1),
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_two_additional_cards_clause() {
        let tokens = tokenize_line("two additional cards", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw with numeric additional cards wording");
        assert!(matches!(
            effect,
            EffectAst::Draw {
                count: Value::Fixed(2),
                player: PlayerAst::You,
            }
        ));
    }

    #[test]
    fn parse_draw_card_next_turns_upkeep_trailing_clause() {
        let tokens = tokenize_line("a card at the beginning of the next turns upkeep", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw delayed until next turn's upkeep");
        assert!(matches!(
            effect,
            EffectAst::DelayedUntilNextUpkeep {
                player: PlayerAst::Any,
                effects,
            } if matches!(
                effects.as_slice(),
                [EffectAst::Draw {
                    count: Value::Fixed(1),
                    player: PlayerAst::You,
                }]
            )
        ));
    }

    #[test]
    fn parse_draw_card_next_end_step_trailing_clause() {
        let tokens = tokenize_line("a card at the beginning of the next end step", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw delayed until next end step");
        assert!(matches!(
            effect,
            EffectAst::DelayedUntilNextEndStep {
                player: PlayerFilter::Any,
                effects,
            } if matches!(
                effects.as_slice(),
                [EffectAst::Draw {
                    count: Value::Fixed(1),
                    player: PlayerAst::You,
                }]
            )
        ));
    }

    #[test]
    fn parse_draw_card_if_you_have_no_cards_in_hand_trailing_clause() {
        let tokens = tokenize_line("a card if you have no cards in hand", 0);
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw with trailing if predicate");
        assert!(matches!(
            effect,
            EffectAst::Conditional {
                predicate: PredicateAst::YouHaveNoCardsInHand,
                if_true,
                if_false,
            } if if_false.is_empty()
                && matches!(
                    if_true.as_slice(),
                    [EffectAst::Draw {
                        count: Value::Fixed(1),
                        player: PlayerAst::You,
                    }]
                )
        ));
    }

    #[test]
    fn parse_draw_card_unless_target_opponent_action() {
        let tokens = tokenize_line(
            "a card unless target opponent sacrifices a creature of their choice or pays 3 life",
            0,
        );
        let effect = parse_draw(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse draw with trailing unless clause");
        assert!(matches!(
            effect,
            EffectAst::UnlessAction {
                player: PlayerAst::TargetOpponent,
                effects,
                ..
            } if matches!(
                effects.as_slice(),
                [EffectAst::Draw {
                    count: Value::Fixed(1),
                    player: PlayerAst::You,
                }]
                )
        ));
    }

    #[test]
    fn parse_discard_a_red_or_green_card_qualifier() {
        let tokens = tokenize_line("a red or green card", 0);
        let effect = parse_discard(&tokens, Some(SubjectAst::Player(PlayerAst::You)))
            .expect("parse discard with color disjunction qualifier");
        assert!(matches!(
            effect,
            EffectAst::Discard {
                count: Value::Fixed(1),
                player: PlayerAst::You,
                random: false,
                filter: Some(filter),
            } if filter.zone == Some(Zone::Hand)
        ));
    }

    #[test]
    fn parse_surge_of_strength_additional_discard_cost() {
        crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Surge of Strength Parse Probe",
        )
        .parse_text(
            "As an additional cost to cast this spell, discard a red or green card.\nTarget creature gains trample and gets +X/+0 until end of turn, where X is that creature's mana value.",
        )
        .expect("parse surge of strength additional discard cost");
    }

    #[test]
    fn parse_put_counters_that_many_amount() {
        let tokens = tokenize_line("that many +1/+1 counters on this creature", 0);
        let effect = parse_put_counters(&tokens).expect("parse put counters with that-many amount");
        assert!(matches!(
            effect,
            EffectAst::PutCounters {
                counter_type: CounterType::PlusOnePlusOne,
                count: Value::EventValue(EventValueSpec::Amount),
                ..
            }
        ));
    }

    #[test]
    fn parse_put_counters_x_amount() {
        let tokens = tokenize_line("x +1/+1 counters on target creature", 0);
        let effect = parse_put_counters(&tokens).expect("parse put counters with x amount");
        assert!(matches!(
            effect,
            EffectAst::PutCounters {
                counter_type: CounterType::PlusOnePlusOne,
                count: Value::X,
                ..
            }
        ));
    }

    #[test]
    fn parse_put_counters_where_x_replacement() {
        let tokens = tokenize_line(
            "Put X +1/+1 counters on target creature, where X is that creature's power.",
            0,
        );
        let effects = parse_effect_sentence(&tokens).expect("parse put counters where-X sentence");
        assert!(effects.iter().any(|effect| matches!(
            effect,
            EffectAst::PutCounters {
                counter_type: CounterType::PlusOnePlusOne,
                count: Value::PowerOf(_),
                ..
            }
        )));
    }

    #[test]
    fn parse_put_counters_equal_to_devotion_amount() {
        let tokens = tokenize_line(
            "a number of +1/+1 counters on it equal to your devotion to green",
            0,
        );
        let effect =
            parse_put_counters(&tokens).expect("parse put counters with devotion-derived amount");
        assert!(matches!(
            effect,
            EffectAst::PutCounters {
                counter_type: CounterType::PlusOnePlusOne,
                count: Value::Devotion {
                    player: PlayerFilter::You,
                    color: crate::color::Color::Green
                },
                ..
            }
        ));
    }

    #[test]
    fn parse_put_counters_those_counters_moves_all() {
        let tokens = tokenize_line("those counters on target creature you control", 0);
        let effect =
            parse_put_counters(&tokens).expect("parse put those-counters transfer as move-all");
        assert!(matches!(
            effect,
            EffectAst::MoveAllCounters {
                from: TargetAst::Tagged(tag, _),
                ..
            } if tag.as_str() == IT_TAG
        ));
    }
