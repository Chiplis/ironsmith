use super::*;
use crate::ability::AbilityKind;
use crate::card::PowerToughness;
use crate::game_state::{GameState, StackEntry};
use crate::ids::CardId;

#[test]
fn cohort_restricted_mana_accepts_each_spell_union_arm_and_only_matching_ability_sources() {
    let land = crate::CardDefinitionBuilder::new(CardId::new(), "Verovin")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add one mana of any color. Spend this mana only to cast an Assassin spell or a spell that has freerunning, or to activate an ability of an Assassin source.").unwrap();
    let ability = land
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    assert_eq!(ability.mana_usage_restrictions.len(), 1);
    for assassin in [false, true] {
        for freerunning in [false, true] {
            let builder = crate::CardDefinitionBuilder::new(CardId::new(), "Zevrin")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![if assassin {
                    Subtype::Assassin
                } else {
                    Subtype::Human
                }])
                .power_toughness(PowerToughness::fixed(2, 2));
            let candidate = if freerunning {
                builder.parse_text("Freerunning {R}").unwrap()
            } else {
                builder.build()
            };
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&land, alice, Zone::Battlefield);
            game.player_mut(alice).unwrap().add_restricted_mana(
                crate::ability::RestrictedManaUnit {
                    symbol: crate::mana::ManaSymbol::Red,
                    source,
                    source_chosen_creature_type: None,
                    restrictions: ability.mana_usage_restrictions.clone(),
                },
            );
            let spell = game.create_object_from_definition(&candidate, alice, Zone::Stack);
            game.stack.push(StackEntry::new(spell, alice));
            let permanent =
                game.create_object_from_definition(&candidate, alice, Zone::Battlefield);
            let cost =
                crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(1)]);
            assert_eq!(
                game.can_pay_mana_cost_with_reason(
                    alice,
                    Some(spell),
                    &cost,
                    0,
                    crate::costs::PaymentReason::CastSpell
                ),
                assassin || freerunning,
                "assassin={assassin}, freerunning={freerunning}"
            );
            assert_eq!(
                game.can_pay_mana_cost_with_reason(
                    alice,
                    Some(permanent),
                    &cost,
                    0,
                    crate::costs::PaymentReason::ActivateAbility
                ),
                assassin
            );
        }
    }
}

#[test]
fn cohort_ordinal_spell_trigger_counts_only_matching_spells_per_opponent_and_turn() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Vezrith")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("Whenever an opponent casts an instant spell other than the first instant spell that player casts each turn, this creature deals 4 damage to that player.").unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let players = [game.players[0].id, game.players[1].id, game.players[2].id];
    game.create_object_from_definition(&card, players[0], Zone::Battlefield);
    for (who, instant, expected, next_turn) in [
        (1, false, false, false),
        (1, true, false, false),
        (0, true, false, false),
        (1, false, false, false),
        (1, true, true, false),
        (2, true, false, false),
        (2, true, true, false),
        (1, true, true, false),
        (1, true, false, true),
        (1, true, true, false),
    ] {
        if next_turn {
            game.turn_store.turn_history.clear_for_new_turn();
            game.turn.turn_number += 1;
            game.turn.active_player = players[1];
        }
        let spell = crate::CardDefinitionBuilder::new(CardId::new(), "Raveth")
            .card_types(vec![if instant {
                CardType::Instant
            } else {
                CardType::Sorcery
            }])
            .build();
        let id = game.create_object_from_definition(&spell, players[who], Zone::Stack);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::SpellCastEvent::new_with_snapshot(
                id,
                players[who],
                Zone::Hand,
                crate::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), &game),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event.clone());
        game.take_pending_trigger_events();
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            usize::from(expected),
            "who={who}, instant={instant}, next_turn={next_turn}"
        );
        let before = game.player(players[who]).unwrap().life;
        if expected {
            let mut queue = crate::triggers::TriggerQueue::new();
            for trigger in triggers {
                queue.add(trigger);
            }
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        assert_eq!(
            game.player(players[who]).unwrap().life,
            before - if expected { 4 } else { 0 }
        );
        game.move_object_by_effect(id, Zone::Graveyard).unwrap();
    }
    assert_eq!(game.player(players[0]).unwrap().life, 20);
}

#[test]
fn cohort_sacrificed_aura_pumps_attached_and_shared_type_creatures_on_both_sides() {
    let aura = crate::CardDefinitionBuilder::new(CardId::new(), "Zevorin")
        .card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura])
        .parse_text("Enchant creature\nSacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get +1/+0 and gain first strike until end of turn.").unwrap();
    let ability = aura
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    for no_type in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let mut ids = Vec::new();
        for (owner, subtypes) in [
            (
                alice,
                if no_type {
                    vec![]
                } else {
                    vec![Subtype::Human, Subtype::Wizard]
                },
            ),
            (alice, vec![Subtype::Human]),
            (bob, vec![Subtype::Wizard]),
            (bob, vec![Subtype::Elf]),
        ] {
            let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravin")
                .card_types(vec![CardType::Creature])
                .subtypes(subtypes)
                .power_toughness(PowerToughness::fixed(2, 3))
                .build();
            ids.push(game.create_object_from_definition(&card, owner, Zone::Battlefield));
        }
        let source = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
        game.object_mut(source).unwrap().attached_to =
            Some(crate::object::AttachmentTarget::Object(ids[0]));
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let mut cost_ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_source_snapshot(snapshot.clone());
        for cost in ability.mana_cost.as_all().unwrap() {
            if let Some(effect) = cost.effect_ref() {
                crate::effects::execute_effect(&mut game, effect, &mut cost_ctx).unwrap();
            }
        }
        assert!(game.object(source).is_none());
        let mut entry = StackEntry::ability(source, alice, ability.effects.clone());
        entry.source_snapshot = Some(snapshot);
        entry.tagged_objects = cost_ctx.tagged_objects.clone();
        game.stack.push(entry);
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        for (index, id) in ids.iter().enumerate() {
            let affected = index == 0 || (!no_type && index < 3);
            assert_eq!(
                game.current_characteristics(*id).unwrap().power,
                Some(if affected { 3 } else { 2 }),
                "index={index}, no_type={no_type}"
            );
            assert_eq!(
                game.current_has_static_ability_id(
                    *id,
                    crate::static_abilities::StaticAbilityId::FirstStrike
                ),
                affected
            );
        }
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        for id in ids {
            assert_eq!(game.current_characteristics(id).unwrap().power, Some(2));
            assert!(!game.current_has_static_ability_id(
                id,
                crate::static_abilities::StaticAbilityId::FirstStrike
            ));
        }
    }
}

#[test]
fn cohort_hand_and_graveyard_choices_stay_with_target_opponent_and_accumulate() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Zarovin")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target opponent reveals their hand. You choose an artifact or creature card from it, then choose an artifact or creature card from their graveyard. Exile the chosen cards.").unwrap();
    struct Pick {
        chooser: crate::ids::PlayerId,
        owner: crate::ids::PlayerId,
    }
    impl crate::decision::DecisionMaker for Pick {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            assert_eq!(ctx.player, self.chooser);
            for candidate in ctx.candidates.iter().filter(|c| c.legal) {
                let object = game.object(candidate.id).unwrap();
                assert_eq!(object.owner, self.owner);
                assert!(matches!(object.zone, Zone::Hand | Zone::Graveyard));
                assert!(
                    object.card_types.contains(&CardType::Artifact)
                        || object.card_types.contains(&CardType::Creature)
                );
            }
            ctx.candidates
                .iter()
                .find(|c| c.legal)
                .map(|c| vec![c.id])
                .unwrap_or_default()
        }
    }
    for hand_present in [false, true] {
        for grave_present in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let (alice, bob) = (game.players[0].id, game.players[1].id);
            let source = game.create_object_from_definition(&card, alice, Zone::Stack);
            let artifact = crate::CardDefinitionBuilder::new(CardId::new(), "Hand choice")
                .card_types(vec![CardType::Artifact])
                .build();
            let creature = crate::CardDefinitionBuilder::new(CardId::new(), "Graveyard choice")
                .card_types(vec![CardType::Creature])
                .build();
            if hand_present {
                game.create_object_from_definition(&artifact, bob, Zone::Hand);
            }
            if grave_present {
                game.create_object_from_definition(&creature, bob, Zone::Graveyard);
            }
            let own = game.create_object_from_definition(&creature, alice, Zone::Graveyard);
            let land = crate::CardDefinitionBuilder::new(CardId::new(), "Land")
                .card_types(vec![CardType::Land])
                .build();
            let excluded = game.create_object_from_definition(&land, bob, Zone::Hand);
            game.stack.push(
                StackEntry::new(source, alice)
                    .with_targets(vec![crate::game_state::Target::Player(bob)]),
            );
            crate::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut Pick {
                    chooser: alice,
                    owner: bob,
                },
            )
            .unwrap();
            assert!(game.object(own).is_some());
            assert!(game.object(excluded).is_some());
            let exiled = game
                .exile
                .iter()
                .filter_map(|id| game.object(*id))
                .map(|o| o.name.as_str())
                .collect::<Vec<_>>();
            assert_eq!(exiled.contains(&"Hand choice"), hand_present);
            assert_eq!(exiled.contains(&"Graveyard choice"), grave_present);
            assert_eq!(
                exiled.len(),
                usize::from(hand_present) + usize::from(grave_present)
            );
        }
    }
}

#[test]
fn cohort_chosen_hand_card_mana_value_controls_token_creation_including_no_choice() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Nyvorin")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target opponent reveals their hand. You choose a nonland card from it. Exile that card. If the card's mana value is 1 or less, create a 1/1 white and black Spirit creature token with flying.").unwrap();
    for value in [None, Some(0), Some(1), Some(2)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Stack);
        if let Some(value) = value {
            let chosen = crate::CardDefinitionBuilder::new(CardId::new(), "Zevra")
                .card_types(vec![CardType::Instant])
                .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                    crate::mana::ManaSymbol::Generic(value),
                ]))
                .build();
            game.create_object_from_definition(&chosen, bob, Zone::Hand);
        }
        let land = crate::CardDefinitionBuilder::new(CardId::new(), "Untouched land")
            .card_types(vec![CardType::Land])
            .build();
        let excluded = game.create_object_from_definition(&land, bob, Zone::Hand);
        game.stack.push(
            StackEntry::new(source, alice)
                .with_targets(vec![crate::game_state::Target::Player(bob)]),
        );
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.object(excluded).is_some());
        assert_eq!(
            game.exile
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|o| o.name == "Zevra")
                .count(),
            usize::from(value.is_some())
        );
        let tokens = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|o| o.name == "Spirit")
            .collect::<Vec<_>>();
        assert_eq!(
            tokens.len(),
            usize::from(value.is_some_and(|v| v <= 1)),
            "mana value={value:?}"
        );
        for token in tokens {
            let chars = game.current_characteristics(token.id).unwrap();
            assert_eq!((chars.power, chars.toughness), (Some(1), Some(1)));
            assert_eq!(game.controller_of(token), alice);
            assert!(game.current_has_static_ability_id(
                token.id,
                crate::static_abilities::StaticAbilityId::Flying
            ));
        }
    }
}
