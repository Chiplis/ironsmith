use super::*;

fn verify_block_comparison(definition: &crate::cards::CardDefinition, counted: CardType) {
    for (own, bob_count, carol_count) in [(2, 1, 3), (2, 2, 2), (4, 3, 5), (3, 4, 1)] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        let source = game.create_object_from_definition(definition, alice, Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let friend = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let bob_attacker = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let carol_attacker = game.create_object_from_card(&creature, carol, Zone::Battlefield);
        let counted_card =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Counted Object")
                .card_types(vec![counted])
                .build();
        for (player, total, creatures) in [
            (alice, own, 2),
            (bob, bob_count, 1),
            (carol, carol_count, 1),
        ] {
            let existing = if counted == CardType::Creature {
                creatures
            } else {
                0
            };
            for _ in existing..total {
                game.create_object_from_card(&counted_card, player, Zone::Battlefield);
            }
        }
        game.turn.active_player = carol;
        game.refresh_continuous_state();
        for (attacker, count) in [(bob_attacker, bob_count), (carol_attacker, carol_count)] {
            assert_eq!(
                game.can_block_attacker(source, attacker),
                own > count,
                "{counted:?}: own={own}, attacker={count}, active Carol"
            );
            assert_eq!(
                crate::rules::combat::can_block(
                    game.object(attacker).unwrap(),
                    game.object(source).unwrap(),
                    &game
                ),
                own > count
            );
            assert!(game.can_block_attacker(friend, attacker));
        }
        let mut combat = crate::combat_state::CombatState::default();
        combat.attackers = [bob_attacker, carol_attacker]
            .into_iter()
            .map(|creature| crate::combat_state::AttackerInfo {
                creature,
                target: crate::combat_state::AttackTarget::Player(alice),
            })
            .collect();
        let support = game.create_object_from_card(&counted_card, alice, Zone::Battlefield);
        for increased in [true, false] {
            if !increased {
                game.move_object_by_effect(support, Zone::Graveyard);
            }
            game.refresh_continuous_state();
            let actual_own = own + if increased { 1 } else { 0 };
            let options = crate::decision::compute_legal_blockers(&game, &combat, alice);
            for (attacker, opposing) in [(bob_attacker, bob_count), (carol_attacker, carol_count)] {
                assert_eq!(
                    game.can_block_attacker(source, attacker),
                    actual_own > opposing
                );
                let option = options
                    .iter()
                    .find(|option| option.attacker == attacker)
                    .unwrap();
                assert_eq!(
                    option.valid_blockers.contains(&source),
                    actual_own > opposing
                );
                assert!(option.valid_blockers.contains(&friend));
            }
        }
        assert!(game.can_attack(source));
    }
}

#[test]
fn conditional_block_rule_evaluates_each_attacking_controller() {
    for counted in [CardType::Creature, CardType::Land] {
        let mut definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Runtime Block Probe")
                .card_types(vec![CardType::Creature])
                .parse_text("Vigilance")
                .unwrap();
        let mut own = ObjectFilter::default();
        own.card_types = vec![counted];
        own.controller = Some(PlayerFilter::You);
        let mut other = own.clone();
        other.controller = Some(PlayerFilter::Attacking);
        let condition =
            crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::ValueComparison {
                left: Value::Count(own),
                operator: crate::effect::ValueComparisonOperator::GreaterThan,
                right: Value::Count(other),
            }));
        definition.abilities = vec![crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::restriction(
                crate::effect::Restriction::block(ObjectFilter::source()),
                "conditional block".to_string(),
            )
            .with_condition(condition)
            .unwrap(),
        )];
        verify_block_comparison(&definition, counted);
    }
}

#[test]
fn compiled_block_comparison_preserves_attacking_player_and_count_domain() {
    for (noun, counted) in [("creatures", CardType::Creature), ("lands", CardType::Land)] {
        let oracle = format!(
            "This creature can't block unless you control more {noun} than attacking player."
        );
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Parsed Block Probe")
                .card_types(vec![CardType::Creature])
                .parse_text(&oracle)
                .unwrap();
        assert!(definition.spell_effect.is_none());
        verify_block_comparison(&definition, counted);
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition)
                .join("\n")
                .to_ascii_lowercase(),
            oracle.to_ascii_lowercase()
        );
    }
}
