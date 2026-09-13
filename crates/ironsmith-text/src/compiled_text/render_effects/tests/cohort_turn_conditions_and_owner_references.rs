use super::*;

#[test]
fn cohort_delayed_death_return_chooses_only_from_the_watched_creatures_owners_graveyard() {
    struct GraveyardChoice {
        owner: crate::ids::PlayerId,
        returned: crate::ids::ObjectId,
        calls: usize,
    }
    impl crate::decision::DecisionMaker for GraveyardChoice {
        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            self.calls += 1;
            for candidate in ctx.candidates.iter().filter(|c| c.legal) {
                let object = game.object(candidate.id).unwrap();
                assert_eq!(
                    object.owner, self.owner,
                    "only the watched creature's owner's graveyard is eligible"
                );
                assert_eq!(object.zone, Zone::Graveyard);
                assert!(object.card_types.contains(&CardType::Creature));
            }
            assert!(
                ctx.candidates
                    .iter()
                    .any(|c| c.legal && c.id == self.returned)
            );
            vec![self.returned]
        }
    }
    let spell=crate::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Death Watch")
        .card_types(vec![CardType::Instant])
        .parse_text("Choose target creature. When that creature dies this turn, return a creature card from its owner's graveyard to the battlefield under the control of that creature's owner.").unwrap();
    for watched_owner in [0, 1] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let owner = game.players[watched_owner].id;
        let controller = game.players[1 - watched_owner].id;
        let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
        let body = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Return candidate")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3))
            .build();
        let returned = game.create_object_from_definition(&body, owner, Zone::Graveyard);
        let stable = game.object(returned).unwrap().stable_id;
        game.create_object_from_definition(&body, controller, Zone::Graveyard);
        let watched = game.create_object_from_definition(&body, owner, Zone::Battlefield);
        game.set_current_controller(watched, controller);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(watched)]);
        for effect in spell
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.take_pending_trigger_events();
        game.move_object_by_effect(watched, Zone::Graveyard)
            .unwrap();
        let mut queue = crate::triggers::TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for trigger in crate::triggers::check_delayed_triggers(&mut game, &event) {
                queue.add(trigger);
            }
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert!(
            game.stack.last().unwrap().targets.is_empty(),
            "graveyard card is chosen on resolution"
        );
        let mut choices = GraveyardChoice {
            owner,
            returned,
            calls: 0,
        };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(choices.calls, 1);
        let returned = game
            .battlefield
            .iter()
            .find_map(|id| game.object(*id).filter(|o| o.stable_id == stable))
            .unwrap();
        assert_eq!(game.current_controller(returned.id), Some(owner));
    }
}

#[test]
fn cohort_turn_condition_tracks_the_current_controller_of_the_permanent() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Turn Guard")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 4))
        .parse_text("As long as it's your turn, this creature has first strike.")
        .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    for controller in [alice, bob] {
        game.set_current_controller(source, controller);
        for active in [alice, bob] {
            game.turn.active_player = active;
            assert_eq!(
                game.current_has_static_ability_id(
                    source,
                    crate::static_abilities::StaticAbilityId::FirstStrike
                ),
                active == controller
            );
        }
    }
}

#[test]
fn cohort_counter_condition_counts_the_granted_target_and_not_other_objects() {
    let spell=crate::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Twin Strike")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature you control gains double strike until end of turn. If it has a +1/+1 counter on it, draw a card.").unwrap();
    let body = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter fighter")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    for counters in [0, 1, 2] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
        let target = game.create_object_from_definition(&body, alice, Zone::Battlefield);
        game.add_counters(source, crate::CounterType::PlusOnePlusOne, 1);
        game.add_counters(target, crate::CounterType::PlusOnePlusOne, counters);
        game.create_object_from_definition(&body, alice, Zone::Library);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        for effect in spell
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert!(game.current_has_static_ability_id(
            target,
            crate::static_abilities::StaticAbilityId::DoubleStrike
        ));
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            usize::from(counters > 0)
        );
    }
}

#[test]
fn cohort_static_spell_count_threshold_counts_only_the_controller_this_turn() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Spellcount Soldier")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 4))
        .parse_text("This creature gets +2/+0 as long as you've cast two or more spells this turn.")
        .unwrap();
    let other = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Recorded spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    for own in [0, 1, 2, 3] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        for (caster, count) in [(alice, own), (bob, 3)] {
            for _ in 0..count {
                let id = game.create_object_from_definition(&other, caster, Zone::Stack);
                let snapshot =
                    crate::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), &game);
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::SpellCastEvent::new_with_snapshot(
                        id,
                        caster,
                        Zone::Hand,
                        snapshot,
                    ),
                    crate::provenance::ProvNodeId::default(),
                );
                game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event);
                game.move_object_by_effect(id, Zone::Graveyard);
            }
        }
        assert_eq!(
            game.calculated_characteristics(source).unwrap().power,
            Some(if own >= 2 { 4 } else { 2 })
        );
    }
}

#[test]
fn cohort_revealed_hand_choice_is_nonland_and_scoped_to_the_target_player() {
    struct HandChoice {
        card: crate::ids::ObjectId,
        owner: crate::ids::PlayerId,
        calls: usize,
    }
    impl crate::decision::DecisionMaker for HandChoice {
        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            self.calls += 1;
            for candidate in ctx.candidates.iter().filter(|c| c.legal) {
                let object = game.object(candidate.id).unwrap();
                assert_eq!(object.owner, self.owner);
                assert_eq!(object.zone, Zone::Hand);
                assert!(!object.card_types.contains(&CardType::Land));
            }
            assert!(ctx.candidates.iter().any(|c| c.legal && c.id == self.card));
            vec![self.card]
        }
    }
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Hand Terror")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player reveals their hand. You choose a nonland card from it. Exile that card.",
        )
        .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Hand creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .build();
    let land = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Hand land")
        .card_types(vec![CardType::Land])
        .build();
    let chosen = game.create_object_from_definition(&creature, bob, Zone::Hand);
    let stable = game.object(chosen).unwrap().stable_id;
    game.create_object_from_definition(&spell, bob, Zone::Hand);
    let retained = game.create_object_from_definition(&land, bob, Zone::Hand);
    let own = game.create_object_from_definition(&creature, alice, Zone::Hand);
    let mut choices = HandChoice {
        card: chosen,
        owner: bob,
        calls: 0,
    };
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut choices)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    for effect in spell
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(choices.calls, 1);
    assert!(game.player(alice).unwrap().hand.contains(&own));
    assert!(game.player(bob).unwrap().hand.contains(&retained));
    assert_eq!(game.player(bob).unwrap().hand.len(), 2);
    assert!(
        game.exile
            .iter()
            .any(|id| game.object(*id).is_some_and(|o| o.stable_id == stable))
    );
}
