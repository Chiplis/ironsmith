use super::*;
use crate::ability::AbilityKind;
use crate::card::PowerToughness;
use crate::game_state::{GameState, StackEntry};
use crate::ids::CardId;

#[test]
fn cohort_no_other_permanents_and_empty_hand_are_checked_at_trigger_and_resolution() {
    let oracle = "At the beginning of your upkeep, if you control no permanents other than this enchantment and have no cards in hand, you win the game.";
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Vezaral")
        .card_types(vec![CardType::Enchantment])
        .parse_text(oracle)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card).join("\n"),
        oracle
    );
    let other = crate::CardDefinitionBuilder::new(CardId::new(), "Zarovin")
        .card_types(vec![CardType::Artifact])
        .build();
    for permanent in [false, true] {
        for hand in [false, true] {
            for late_card in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let (alice, bob) = (game.players[0].id, game.players[1].id);
                game.create_object_from_definition(&card, alice, Zone::Battlefield);
                game.create_object_from_definition(&other, bob, Zone::Battlefield);
                game.create_object_from_definition(&other, alice, Zone::Graveyard);
                if permanent {
                    game.create_object_from_definition(&other, alice, Zone::Battlefield);
                }
                if hand {
                    game.create_object_from_definition(&other, alice, Zone::Hand);
                }
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::BeginningOfUpkeepEvent::new(alice),
                    crate::provenance::ProvNodeId::default(),
                );
                let triggers = crate::triggers::check_triggers(&game, &event);
                assert_eq!(triggers.len(), usize::from(!permanent && !hand));
                if !triggers.is_empty() {
                    let mut queue = crate::triggers::TriggerQueue::new();
                    for trigger in triggers {
                        queue.add(trigger);
                    }
                    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                    if late_card {
                        game.create_object_from_definition(&other, alice, Zone::Hand);
                    }
                    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
                assert_eq!(
                    game.player(bob).unwrap().has_lost,
                    !permanent && !hand && !late_card
                );
            }
        }
    }
}

#[test]
fn cohort_experience_cost_reduction_preserves_colored_mana_and_spell_trigger_threshold() {
    let oracle = "Whenever you cast an instant or sorcery spell with mana value greater than the number of experience counters you have, you get an experience counter.\nInstant and sorcery spells you cast cost {1} less to cast for each experience counter you have.";
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Velarin")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card).join("\n"),
        oracle
    );
    for experience in [0, 2, 5] {
        for kind in [CardType::Instant, CardType::Sorcery, CardType::Creature] {
            for opponent in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let (alice, bob) = (game.players[0].id, game.players[1].id);
                game.create_object_from_definition(&card, alice, Zone::Battlefield);
                game.player_mut(alice).unwrap().experience_counters = experience;
                game.player_mut(bob).unwrap().experience_counters = 7;
                let caster = if opponent { bob } else { alice };
                let base = crate::mana::ManaCost::from_symbols(vec![
                    crate::mana::ManaSymbol::Generic(3),
                    crate::mana::ManaSymbol::Blue,
                ]);
                let spell = crate::CardDefinitionBuilder::new(CardId::new(), "Zalaren")
                    .card_types(vec![kind])
                    .mana_cost(base.clone())
                    .build();
                let id = game.create_object_from_definition(&spell, caster, Zone::Stack);
                game.stack.push(StackEntry::new(id, caster));
                let qualifies = !opponent && kind != CardType::Creature;
                let cost = crate::decision::calculate_effective_mana_cost(
                    &game,
                    caster,
                    game.object(id).unwrap(),
                    &base,
                );
                assert_eq!(
                    cost.generic_mana_total(),
                    if qualifies {
                        3u32.saturating_sub(experience)
                    } else {
                        3
                    }
                );
                assert!(cost.to_oracle().ends_with("{U}"));
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::SpellCastEvent::new_with_snapshot(
                        id,
                        caster,
                        Zone::Hand,
                        crate::snapshot::ObjectSnapshot::from_object(
                            game.object(id).unwrap(),
                            &game,
                        ),
                    ),
                    crate::provenance::ProvNodeId::default(),
                );
                let triggers = crate::triggers::check_triggers(&game, &event);
                let expected = qualifies && 4 > experience;
                assert_eq!(triggers.len(), usize::from(expected));
                if expected {
                    let mut queue = crate::triggers::TriggerQueue::new();
                    for trigger in triggers {
                        queue.add(trigger);
                    }
                    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
                assert_eq!(
                    game.player(alice).unwrap().experience_counters,
                    experience + u32::from(expected)
                );
                assert_eq!(game.player(bob).unwrap().experience_counters, 7);
            }
        }
    }
}

#[test]
fn cohort_defender_attack_permission_keeps_defender_and_expires_for_fixed_recipients() {
    let oracle = "Defender, flying\n{1}{W}: Creatures you control with defender can attack this turn as though they didn't have defender.";
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Vorazel")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .parse_text(oracle)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card).join("\n"),
        oracle
    );
    let ability = card
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    let wall = crate::CardDefinitionBuilder::new(CardId::new(), "Velzor")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text("Defender")
        .unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let own = game.create_object_from_definition(&wall, alice, Zone::Battlefield);
    let enemy = game.create_object_from_definition(&wall, bob, Zone::Battlefield);
    let locked_card = crate::CardDefinitionBuilder::new(CardId::new(), "Voralin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text("Defender\nThis creature can't attack.")
        .unwrap();
    let locked = game.create_object_from_definition(&locked_card, alice, Zone::Battlefield);
    game.remove_summoning_sickness(locked);
    for id in [source, own, enemy] {
        game.remove_summoning_sickness(id);
        assert!(!crate::rules::combat::can_attack(
            game.object(id).unwrap(),
            &game
        ));
    }
    game.stack
        .push(StackEntry::ability(source, alice, ability.effects.clone()));
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    for id in [source, own] {
        assert!(
            crate::rules::combat::can_attack(game.object(id).unwrap(), &game),
            "recipient {id:?} still cannot attack after gaining permission"
        );
        assert!(
            game.current_has_static_ability_id(
                id,
                crate::static_abilities::StaticAbilityId::Defender
            )
        );
    }
    assert!(game.current_has_static_ability_id(
        locked,
        crate::static_abilities::StaticAbilityId::CanAttackAsThoughNoDefender
    ));
    assert!(!crate::rules::combat::can_attack(
        game.object(locked).unwrap(),
        &game
    ));
    assert!(!game.current_has_static_ability_id(
        enemy,
        crate::static_abilities::StaticAbilityId::CanAttackAsThoughNoDefender
    ));
    let later = game.create_object_from_definition(&wall, alice, Zone::Battlefield);
    game.remove_summoning_sickness(later);
    assert!(!crate::rules::combat::can_attack(
        game.object(later).unwrap(),
        &game
    ));
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.turn.turn_number += 1;
    for id in [source, own] {
        assert!(!crate::rules::combat::can_attack(
            game.object(id).unwrap(),
            &game
        ));
    }
}

#[test]
fn cohort_destroyed_creature_life_loss_counts_last_controller_and_only_actual_destruction() {
    let oracle = "Destroy all creatures. Each player loses life equal to the number of creatures they controlled that were destroyed this way.";
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Zaverin")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card).join("\n"),
        oracle
    );
    let creature = crate::CardDefinitionBuilder::new(CardId::new(), "Vezral")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let survivor = crate::CardDefinitionBuilder::new(CardId::new(), "Zoralin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Indestructible")
        .unwrap();
    for owned in [0, 1, 3] {
        for enemy in [0, 2] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let (alice, bob) = (game.players[0].id, game.players[1].id);
            let source = game.create_object_from_definition(&card, alice, Zone::Stack);
            for i in 0..owned {
                let id = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
                if i == 0 {
                    game.set_current_controller(id, bob);
                }
            }
            for _ in 0..enemy {
                game.create_object_from_definition(&creature, bob, Zone::Battlefield);
            }
            let survivors = [
                game.create_object_from_definition(&survivor, alice, Zone::Battlefield),
                game.create_object_from_definition(&survivor, bob, Zone::Battlefield),
            ];
            game.create_object_from_definition(&creature, alice, Zone::Graveyard);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            for effect in card
                .spell_effect
                .as_ref()
                .unwrap()
                .flattened_default_effects()
            {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            assert_eq!(game.player(alice).unwrap().life, 20 - (owned - 1).max(0));
            assert_eq!(
                game.player(bob).unwrap().life,
                20 - enemy - i32::from(owned > 0)
            );
            for id in survivors {
                assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
            }
        }
    }
}

#[test]
fn cohort_damage_prevention_followup_matches_combat_source_and_returns_only_recipient_to_owner() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Voranel").card_types(vec![CardType::Artifact,CardType::Creature]).power_toughness(PowerToughness::fixed(2,2))
        .parse_text("Flash\nFirst strike, vigilance\nWhenever an opponent casts a creature spell, this creature isn't a creature until end of turn.\nIf this creature would deal combat damage to a creature, prevent that damage and that creature's owner shuffles it into their library.").unwrap();
    assert!(
        card.spell_effect.is_none(),
        "damage prevention must be an event-processing static"
    );
    for creature_target in [false, true] {
        for combat in [false, true] {
            for own_source in [false, true] {
                for unpreventable in [false, true] {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let (alice, bob) = (game.players[0].id, game.players[1].id);
                    let source =
                        game.create_object_from_definition(&card, alice, Zone::Battlefield);
                    let body = crate::CardDefinitionBuilder::new(CardId::new(), "Zalorin")
                        .card_types(vec![if creature_target {
                            CardType::Creature
                        } else {
                            CardType::Artifact
                        }])
                        .power_toughness(PowerToughness::fixed(3, 3))
                        .build();
                    let target =
                        game.create_object_from_definition(&body, alice, Zone::Battlefield);
                    let stable = game.object(target).unwrap().stable_id;
                    game.set_current_controller(target, bob);
                    let other = game.create_object_from_definition(&body, bob, Zone::Battlefield);
                    let damage_source = if own_source { source } else { other };
                    let result=crate::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts(&mut game,damage_source,crate::events::DamageTarget::Object(target),2,combat,unpreventable,crate::events::cause::EventCause::effect(),None);
                    let matches = creature_target && combat && own_source;
                    assert_eq!(
                        result.assignments.iter().map(|a| a.amount).sum::<u32>(),
                        if matches && !unpreventable { 0 } else { 2 }
                    );
                    let moved = game
                        .player(alice)
                        .unwrap()
                        .library
                        .iter()
                        .any(|id| game.object(*id).unwrap().stable_id == stable);
                    assert_eq!(
                        moved, matches,
                        "creature={creature_target}, combat={combat}, own_source={own_source}, unpreventable={unpreventable}"
                    );
                    assert!(game.player(bob).unwrap().library.is_empty());
                    assert_eq!(game.object(other).unwrap().zone, Zone::Battlefield);
                }
            }
        }
    }
    // Damage to a player is outside the recipient filter.
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let (damage, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        source,
        crate::events::DamageTarget::Player(bob),
        2,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(damage, 2);
}

#[test]
fn cohort_first_strike_prevention_removes_blocker_without_dealing_combat_damage() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Voranel").card_types(vec![CardType::Artifact,CardType::Creature]).power_toughness(PowerToughness::fixed(2,2))
        .parse_text("First strike\nIf this creature would deal combat damage to a creature, prevent that damage and that creature's owner shuffles it into their library.").unwrap();
    let body = crate::CardDefinitionBuilder::new(CardId::new(), "Zoravel")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 5))
        .build();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let blocker = game.create_object_from_definition(&body, bob, Zone::Battlefield);
    let stable = game.object(blocker).unwrap().stable_id;
    let combat = crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: source,
            target: crate::combat_state::AttackTarget::Player(bob),
        }],
        blockers: std::collections::HashMap::from([(source, vec![blocker])]),
        ..Default::default()
    };
    game.take_pending_trigger_events();
    let events = crate::game_loop::execute_combat_damage_step(&mut game, &combat, true);
    assert!(events.iter().all(|event| event.amount == 0));
    assert!(
        game.player(bob)
            .unwrap()
            .library
            .iter()
            .any(|id| game.object(*id).unwrap().stable_id == stable)
    );
    assert_eq!(game.damage_on(source), 0);
    assert!(
        game.take_pending_trigger_events()
            .iter()
            .all(|event| event.downcast::<crate::events::DamageEvent>().is_none())
    );
}

#[test]
fn cohort_opponent_creature_spell_temporarily_removes_creature_type_and_subtypes() {
    for kindred in [false, true] {
        let card = crate::CardDefinitionBuilder::new(CardId::new(), "Voranel")
        .card_types(if kindred {vec![CardType::Artifact, CardType::Creature, CardType::Kindred]} else {vec![CardType::Artifact, CardType::Creature]})
        .subtypes(vec![crate::types::Subtype::Alien, crate::types::Subtype::Angel, crate::types::Subtype::Equipment])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Whenever an opponent casts a creature spell, this creature isn't a creature until end of turn.").unwrap();
        for opponent in [false, true] {
            for creature in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let (alice, bob) = (game.players[0].id, game.players[1].id);
                let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
                let caster = if opponent { bob } else { alice };
                let spell = crate::CardDefinitionBuilder::new(CardId::new(), "Zalorin")
                    .card_types(vec![if creature {
                        CardType::Creature
                    } else {
                        CardType::Instant
                    }])
                    .build();
                let id = game.create_object_from_definition(&spell, caster, Zone::Stack);
                game.stack.push(StackEntry::new(id, caster));
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::SpellCastEvent::new_with_snapshot(
                        id,
                        caster,
                        Zone::Hand,
                        crate::snapshot::ObjectSnapshot::from_object(
                            game.object(id).unwrap(),
                            &game,
                        ),
                    ),
                    crate::provenance::ProvNodeId::default(),
                );
                let triggers = crate::triggers::check_triggers(&game, &event);
                let removes_type = opponent && creature;
                assert_eq!(triggers.len(), usize::from(removes_type));
                let mut queue = crate::triggers::TriggerQueue::new();
                for trigger in triggers {
                    queue.add(trigger);
                }
                crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                if removes_type {
                    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
                let chars = game.current_characteristics(source).unwrap();
                assert!(chars.card_types.contains(&CardType::Artifact));
                assert_eq!(
                    chars.card_types.contains(&CardType::Creature),
                    !removes_type
                );
                assert_eq!(
                    chars.subtypes.contains(&crate::types::Subtype::Angel),
                    !removes_type || kindred
                );
                assert_eq!(
                    chars.subtypes.contains(&crate::types::Subtype::Alien),
                    !removes_type || kindred
                );
                assert!(chars.subtypes.contains(&crate::types::Subtype::Equipment));
                game.effect_store.continuous_effects.cleanup_end_of_turn();
                assert!(game.current_is_creature(source));
                assert!(
                    game.current_characteristics(source)
                        .unwrap()
                        .subtypes
                        .contains(&crate::types::Subtype::Angel)
                );
            }
        }
    }
}

#[test]
fn cohort_unpreventable_combat_damage_happens_before_prevention_followup() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Voranel")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("First strike, lifelink\nDamage can't be prevented.\nIf this creature would deal combat damage to a creature, prevent that damage and that creature's owner shuffles it into their library.").unwrap();
    let body = crate::CardDefinitionBuilder::new(CardId::new(), "Zoravel")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 5))
        .build();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let blocker = game.create_object_from_definition(&body, bob, Zone::Battlefield);
    let stable = game.object(blocker).unwrap().stable_id;
    let combat = crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: source,
            target: crate::combat_state::AttackTarget::Player(bob),
        }],
        blockers: std::collections::HashMap::from([(source, vec![blocker])]),
        ..Default::default()
    };
    let events = crate::game_loop::execute_combat_damage_step(&mut game, &combat, true);
    assert_eq!(events.iter().map(|e| e.amount).sum::<u32>(), 2);
    assert_eq!(game.player(alice).unwrap().life, 22);
    assert!(
        game.player(bob)
            .unwrap()
            .library
            .iter()
            .any(|id| game.object(*id).unwrap().stable_id == stable)
    );
}
