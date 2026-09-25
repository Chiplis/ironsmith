use super::*;
use crate::card::PowerToughness;
use crate::game_state::{GameState, StackEntry};
use crate::ids::CardId;

fn body(name: &str, kind: CardType) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![kind])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build()
}

#[test]
fn cohort_corpus_all_creatures_go_below_existing_library_cards_for_their_owners() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Burial")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Put all creatures on the bottom of their owners' libraries.")
        .unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let creature = body("Creature", CardType::Creature);
    let mut creatures = Vec::new();
    for owner in [alice, alice, bob] {
        let id = game.create_object_from_definition(&creature, owner, Zone::Battlefield);
        creatures.push((owner, game.object(id).unwrap().stable_id));
        game.set_current_controller(id, bob);
    }
    let artifact = game.create_object_from_definition(
        &body("Artifact", CardType::Artifact),
        alice,
        Zone::Battlefield,
    );
    let tops = [alice, bob].map(|owner| {
        let id =
            game.create_object_from_definition(&body("Top", CardType::Land), owner, Zone::Library);
        (owner, game.object(id).unwrap().stable_id)
    });
    let source = game.create_object_from_definition(&card, alice, Zone::Stack);
    game.stack.push(StackEntry::new(source, alice));
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert!(game.battlefield.contains(&artifact));
    for (owner, stable) in tops {
        let drawn = game.draw_cards(owner, 1);
        assert_eq!(drawn.len(), 1);
        assert_eq!(game.object(drawn[0]).unwrap().stable_id, stable);
    }
    for (owner, stable) in creatures {
        assert!(
            game.player(owner).unwrap().library.iter().any(|id| game
                .object(*id)
                .unwrap()
                .stable_id
                == stable)
        );
    }
}

#[test]
fn cohort_corpus_requantified_destroy_disjunction_selects_one_complete_set() {
    struct Pick(usize);
    impl crate::decision::DecisionMaker for Pick {
        fn decide_options(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(ctx.options.iter().filter(|option| option.legal).count(), 2);
            vec![self.0]
        }
    }
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Cataclysm").card_types(vec![CardType::Sorcery])
        .parse_text("Destroy all lands or all creatures. Creatures destroyed this way can't be regenerated.").unwrap();
    for choice in 0..2 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let mut objects = Vec::new();
        for owner in [alice, bob] {
            for kind in [CardType::Land, CardType::Creature, CardType::Artifact] {
                objects.push((
                    kind,
                    game.create_object_from_definition(
                        &body("Permanent", kind),
                        owner,
                        Zone::Battlefield,
                    ),
                ));
            }
        }
        let source = game.create_object_from_definition(&card, alice, Zone::Stack);
        game.stack.push(StackEntry::new(source, alice));
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut Pick(choice)).unwrap();
        for (kind, id) in objects {
            assert_eq!(
                game.battlefield.contains(&id),
                kind != if choice == 0 {
                    CardType::Land
                } else {
                    CardType::Creature
                }
            );
        }
    }
}

#[test]
fn cohort_corpus_attack_unless_requires_a_flying_creature_controlled_by_defender() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Dragon").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Flying\nThis creature can't attack unless defending player controls a creature with flying.").unwrap();
    for enemy in [false, true] {
        for flying in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let (alice, bob) = (game.players[0].id, game.players[1].id);
            let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let other = crate::CardDefinitionBuilder::new(CardId::new(), "Other creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .parse_text(if flying { "Flying" } else { "" })
                .unwrap();
            game.create_object_from_definition(
                &other,
                if enemy { bob } else { alice },
                Zone::Battlefield,
            );
            let mut combat = crate::combat_state::CombatState::default();
            assert_eq!(
                crate::combat_state::declare_attackers(
                    &mut game,
                    &mut combat,
                    vec![(source, crate::combat_state::AttackTarget::Player(bob))]
                )
                .is_ok(),
                enemy && flying
            );
        }
    }
}

#[test]
fn cohort_corpus_card_type_threshold_on_attack_is_checked_at_event_time() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Feeder").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Delirium — Whenever this creature attacks while there are four or more card types among cards in your graveyard, it gets +2/+0 and gains menace until end of turn.").unwrap();
    let text = crate::compiled_text::compiled_text_lines(&card).join("\n");
    for count in [3, 4] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let kinds = [
            CardType::Artifact,
            CardType::Land,
            CardType::Instant,
            CardType::Sorcery,
        ];
        let cards = kinds[..count]
            .iter()
            .map(|kind| {
                game.create_object_from_definition(
                    &body("Grave card", *kind),
                    alice,
                    Zone::Graveyard,
                )
            })
            .collect::<Vec<_>>();
        game.create_object_from_definition(
            &body("Enemy sorcery", CardType::Sorcery),
            bob,
            Zone::Graveyard,
        );
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::combat::CreatureAttackedEvent::new(
                source,
                crate::triggers::AttackEventTarget::Player(bob),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), usize::from(count == 4));
        if count == 4 {
            let mut queue = crate::triggers::TriggerQueue::new();
            for trigger in triggers {
                queue.add(trigger);
            }
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            for id in cards {
                game.move_object_by_effect(id, Zone::Exile).unwrap();
            }
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.current_power(source), Some(4));
            assert!(game.current_has_static_ability_id(
                source,
                crate::static_abilities::StaticAbilityId::Menace
            ));
        }
    }
    assert!(
        text.contains("card types among cards in your graveyard"),
        "{text}"
    );
}

#[test]
fn cohort_corpus_graveyard_entry_checks_creatures_owner_and_source_enchantment_condition() {
    for enemy_owner in [false, true] {
        for enchantment in [false, true] {
            let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Lurker")
            .card_types(vec![if enchantment {CardType::Enchantment} else {CardType::Creature}])
            .parse_text("When a creature is put into an opponent's graveyard from the battlefield, if this permanent is an enchantment, it becomes a 3/2 Phyrexian Imp creature with flying.").unwrap();
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let (alice, bob) = (game.players[0].id, game.players[1].id);
            let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            let victim = game.create_object_from_definition(
                &body("Dying creature", CardType::Creature),
                if enemy_owner { bob } else { alice },
                Zone::Battlefield,
            );
            game.set_current_controller(victim, if enemy_owner { alice } else { bob });
            game.take_pending_trigger_events();
            game.move_object_by_effect(victim, Zone::Graveyard).unwrap();
            let events = game.take_pending_trigger_events();
            let mut triggers = Vec::new();
            for event in events {
                triggers.extend(crate::triggers::check_triggers(&game, &event));
            }
            assert_eq!(triggers.len(), usize::from(enemy_owner && enchantment));
            if !triggers.is_empty() {
                let mut queue = crate::triggers::TriggerQueue::new();
                for trigger in triggers {
                    queue.add(trigger);
                }
                crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.current_power(source), Some(3));
                assert_eq!(game.current_toughness(source), Some(2));
                assert!(game.current_has_static_ability_id(
                    source,
                    crate::static_abilities::StaticAbilityId::Flying
                ));
            }
        }
    }
}
