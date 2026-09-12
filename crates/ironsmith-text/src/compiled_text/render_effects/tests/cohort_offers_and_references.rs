use super::*;

struct OfferChoices {
    acceptor: Option<crate::ids::PlayerId>,
    asked: Vec<crate::ids::PlayerId>,
}
impl crate::decision::DecisionMaker for OfferChoices {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.asked.push(ctx.player);
        Some(ctx.player) == self.acceptor
    }
    fn decide_objects(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        ctx.candidates
            .iter()
            .filter(|c| c.legal)
            .take(ctx.max.unwrap_or(ctx.candidates.len()))
            .map(|c| c.id)
            .collect()
    }
}

#[test]
fn cohort_any_opponent_offer_stops_at_acceptor_and_binds_damage_and_fallback() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Offer Keeper")
        .card_types(vec![CardType::Creature])
        .parse_text("At the beginning of your upkeep, reveal the top card of your library. Any opponent may have you put that card into your graveyard. If a player does, this creature deals damage to that player equal to its mana value. Otherwise, put that card into your hand.").unwrap();
    let crate::ability::AbilityKind::Triggered(ability) = &card.abilities[0].kind else {
        panic!("triggered")
    };
    for accept_index in [None, Some(1), Some(2)] {
        for cost in [0, 3] {
            for prevent in [false, true] {
                let mut game = crate::game_state::GameState::new(
                    vec!["Alice".into(), "Bob".into(), "Carol".into()],
                    20,
                );
                let [alice, bob, carol] = std::array::from_fn(|i| game.players[i].id);
                let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
                let revealed =
                    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Revealed")
                        .card_types(vec![CardType::Sorcery])
                        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                            crate::mana::ManaSymbol::Generic(cost),
                        ]))
                        .build();
                let top = game.create_object_from_definition(&revealed, alice, Zone::Library);
                let stable = game.object(top).unwrap().stable_id;
                let acceptor = accept_index.map(|i| game.players[i].id);
                if prevent {
                    let shield = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Shield")
                        .card_types(vec![CardType::Instant])
                        .parse_text("Prevent the next 10 damage that would be dealt to target player this turn.").unwrap();
                    for player in [bob, carol] {
                        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                            .with_targets(vec![crate::effects::ResolvedTarget::Player(player)]);
                        for effect in shield
                            .spell_effect
                            .as_ref()
                            .unwrap()
                            .flattened_default_effects()
                        {
                            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                        }
                    }
                }
                let mut choices = OfferChoices {
                    acceptor,
                    asked: Vec::new(),
                };
                {
                    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut choices);
                    for effect in ability.effects.flattened_default_effects() {
                        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    }
                }
                assert_eq!(
                    choices.asked,
                    if accept_index == Some(1) {
                        vec![bob]
                    } else {
                        vec![bob, carol]
                    }
                );
                for player in [alice, bob, carol] {
                    let damage = if Some(player) == acceptor && !prevent {
                        cost as i32
                    } else {
                        0
                    };
                    assert_eq!(
                        game.player(player).unwrap().life,
                        20 - damage,
                        "accept={accept_index:?} cost={cost} prevent={prevent}"
                    );
                }
                let zone = if acceptor.is_some() {
                    Zone::Graveyard
                } else {
                    Zone::Hand
                };
                assert!(
                    game.player(alice)
                        .unwrap()
                        .hand
                        .iter()
                        .chain(game.player(alice).unwrap().graveyard.iter())
                        .filter_map(|id| game.object(*id))
                        .any(|object| object.stable_id == stable && object.zone == zone),
                    "accept={accept_index:?} cost={cost} prevent={prevent}"
                );
            }
        }
    }
}

#[test]
fn cohort_definite_creature_pump_keeps_attack_event_across_land_return_and_reattachment() {
    let equipment = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Returning Hammer")
        .card_types(vec![CardType::Artifact]).subtypes(vec![crate::types::Subtype::Equipment])
        .parse_text("Whenever equipped creature attacks, you may return a land you control to its owner's hand. If you do, the creature gets +2/+2 until end of turn.").unwrap();
    for accept in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&equipment, alice, Zone::Battlefield);
        let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3))
            .build();
        let attacker = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let land = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Land")
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        game.object_mut(source).unwrap().attached_to =
            Some(crate::object::AttachmentTarget::Object(attacker));
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::combat::CreatureAttackedEvent::new(
                attacker,
                crate::triggers::event::AttackEventTarget::Player(bob),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers {
            queue.add(trigger);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        game.object_mut(source).unwrap().attached_to =
            Some(crate::object::AttachmentTarget::Object(other));
        let mut choices = OfferChoices {
            acceptor: accept.then_some(alice),
            asked: Vec::new(),
        };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        for (id, power) in [(attacker, if accept { 5 } else { 3 }), (other, 3)] {
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    game.object(id).unwrap(),
                    &game,
                );
            assert_eq!(snapshot.power, Some(power));
            assert_eq!(snapshot.toughness, Some(power));
        }
        assert_eq!(game.battlefield.contains(&land_id), !accept);
    }
}

#[test]
fn cohort_filtered_graveyard_combat_restriction_is_static_and_tracks_count() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Lesson Guardian")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack or block unless there are three or more Lesson cards in your graveyard.").unwrap();
    assert!(
        definition.spell_effect.is_none(),
        "restriction became spell effect"
    );
    let lesson = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Lesson")
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![crate::types::Subtype::Lesson])
        .build();
    let ordinary = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ordinary")
        .card_types(vec![CardType::Creature])
        .build();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let enemy = game.create_object_from_definition(&ordinary, bob, Zone::Battlefield);
    for _ in 0..4 {
        game.create_object_from_definition(&ordinary, alice, Zone::Graveyard);
        game.create_object_from_definition(&lesson, bob, Zone::Graveyard);
        game.create_object_from_definition(&lesson, alice, Zone::Hand);
    }
    let mut added = Vec::new();
    for count in 0..=4 {
        game.refresh_continuous_state();
        assert_eq!(game.can_attack(source), count >= 3);
        assert_eq!(game.can_block_attacker(source, enemy), count >= 3);
        if count < 4 {
            added.push(game.create_object_from_definition(&lesson, alice, Zone::Graveyard));
        }
    }
    for id in added.iter().take(2) {
        game.move_object_by_effect(*id, Zone::Exile);
    }
    game.refresh_continuous_state();
    assert!(!game.can_attack(source));
    assert!(!game.can_block_attacker(source, enemy));
}
