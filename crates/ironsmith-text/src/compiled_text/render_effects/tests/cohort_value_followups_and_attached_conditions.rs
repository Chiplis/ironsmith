use super::*;
use crate::card::PowerToughness;
use crate::game_state::GameState;
use crate::ids::CardId;

#[test]
fn cohort_attached_power_condition_tracks_host_and_attachment_changes() {
    let equipment = crate::CardDefinitionBuilder::new(CardId::new(), "Varelon")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text("Equipped creature can't be blocked as long as its power is 3 or less.")
        .unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&equipment, alice, Zone::Battlefield);
    let creature = crate::CardDefinitionBuilder::new(CardId::new(), "Host")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let first = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let blocker =
        game.create_object_from_definition(&creature, game.players[1].id, Zone::Battlefield);
    game.object_mut(source).unwrap().attached_to =
        Some(crate::object::AttachmentTarget::Object(first));
    let check = |game: &mut GameState, a, b| {
        game.update_cant_effects();
        assert_eq!(
            !crate::rules::combat::can_block(
                game.object(first).unwrap(),
                game.object(blocker).unwrap(),
                game
            ),
            a
        );
        assert_eq!(
            !crate::rules::combat::can_block(
                game.object(second).unwrap(),
                game.object(blocker).unwrap(),
                game
            ),
            b
        );
    };
    check(&mut game, true, false);
    game.add_counters(first, crate::CounterType::PlusOnePlusOne, 1);
    check(&mut game, true, false);
    game.add_counters(first, crate::CounterType::PlusOnePlusOne, 1);
    check(&mut game, false, false);
    game.object_mut(source).unwrap().attached_to =
        Some(crate::object::AttachmentTarget::Object(second));
    check(&mut game, false, true);
    let pump = crate::CardDefinitionBuilder::new(CardId::new(), "Tavoris")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature gets +2/+0 until end of turn.")
        .unwrap();
    let pump_source = game.create_object_from_definition(&pump, alice, Zone::Stack);
    let mut ctx = crate::effects::EffectContext::new_default(pump_source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(second)]);
    for effect in pump
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    check(&mut game, false, false);
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.turn.turn_number += 1;
    game.turn.active_player = game.players[1].id;
    check(&mut game, false, true);
    game.object_mut(source).unwrap().attached_to = None;
    check(&mut game, false, false);
}

#[test]
fn cohort_where_x_followup_mills_then_loses_actual_milled_mana_value() {
    struct Offer {
        accept: bool,
        asked: Vec<crate::ids::PlayerId>,
    }
    impl crate::decision::DecisionMaker for Offer {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.asked.push(ctx.player);
            self.accept
        }
    }
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Varelon")
        .card_types(vec![CardType::Artifact])
        .parse_text("At the beginning of your end step, put an influence counter on Varelon and scry 2. Then target opponent may have you draw a card. If that player doesn't, you mill X cards, where X is the number of influence counters on Varelon, and that player loses life equal to the total mana value of those cards.").unwrap();
    for accept in [false, true] {
        for initial_counters in [0, 1, 4] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let (alice, bob) = (game.players[0].id, game.players[1].id);
            let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            game.add_counters(
                source,
                crate::CounterType::Named("influence".into()),
                initial_counters,
            );
            for cost in [2, 3] {
                let def = crate::CardDefinitionBuilder::new(CardId::new(), "Library candidate")
                    .card_types(vec![CardType::Sorcery])
                    .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                        crate::mana::ManaSymbol::Generic(cost),
                    ]))
                    .build();
                game.create_object_from_definition(&def, alice, Zone::Library);
            }
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::phase::BeginningOfEndStepEvent::new(alice),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), 1);
            let mut queue = crate::triggers::TriggerQueue::new();
            for trigger in triggers {
                queue.add(trigger);
            }
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            let mut offer = Offer {
                accept,
                asked: vec![],
            };
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut offer).unwrap();
            assert_eq!(offer.asked, [bob]);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(alice).unwrap().hand.len(), usize::from(accept));
            if accept {
                assert!(game.player(alice).unwrap().graveyard.is_empty());
                assert_eq!(game.player(bob).unwrap().life, 20);
            } else {
                let graveyard = &game.player(alice).unwrap().graveyard;
                assert_eq!(graveyard.len(), (initial_counters as usize + 1).min(2));
                let total: i32 = graveyard
                    .iter()
                    .map(|id| {
                        game.object(*id)
                            .unwrap()
                            .mana_cost
                            .as_ref()
                            .map_or(0, |cost| cost.mana_value()) as i32
                    })
                    .sum();
                assert_eq!(game.player(bob).unwrap().life, 20 - total);
            }
        }
    }
}

#[test]
fn cohort_set_base_pt_locks_recipients_until_controllers_next_turn() {
    let spell = crate::CardDefinitionBuilder::new(CardId::new(), "Zarevis")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Until your next turn, creatures target player controls have base power and toughness 1/1.").unwrap();
    let creature = crate::CardDefinitionBuilder::new(CardId::new(), "Fighter")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 6))
        .build();
    for target_index in [0, 1] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let target_player = game.players[target_index].id;
        let other_player = game.players[1 - target_index].id;
        let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
        let first = game.create_object_from_definition(&creature, target_player, Zone::Battlefield);
        let second =
            game.create_object_from_definition(&creature, target_player, Zone::Battlefield);
        let bystander =
            game.create_object_from_definition(&creature, other_player, Zone::Battlefield);
        game.add_counters(first, crate::CounterType::PlusOnePlusOne, 1);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Player(target_player)]);
        for effect in spell
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let entrant =
            game.create_object_from_definition(&creature, target_player, Zone::Battlefield);
        game.set_current_controller(second, other_player);
        let check = |game: &GameState, active| {
            for (id, power, toughness) in [
                (
                    first,
                    if active { 2 } else { 6 },
                    if active { 2 } else { 7 },
                ),
                (
                    second,
                    if active { 1 } else { 5 },
                    if active { 1 } else { 6 },
                ),
                (bystander, 5, 6),
                (entrant, 5, 6),
            ] {
                let characteristics = game.current_characteristics(id).unwrap();
                assert_eq!(
                    (characteristics.power, characteristics.toughness),
                    (Some(power), Some(toughness))
                );
            }
        };
        check(&game, true);
        game.turn.turn_number += 1;
        game.turn.active_player = bob;
        check(&game, true);
        game.turn.turn_number += 1;
        game.turn.active_player = alice;
        check(&game, false);
    }
}

#[test]
fn cohort_second_spell_copies_only_player_or_permanent_targets_otherwise_draws() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Velorin")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Whenever you cast your second spell each turn, copy that spell if it targets a permanent or player, and you may choose new targets for the copy. If you don't copy a spell this way, draw a card.").unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card),
        vec![
            "Whenever you cast your second spell each turn, copy that spell if it targets a permanent or player, and you may choose new targets for the copy. If you don't copy a spell this way, draw a card."
        ]
    );
    let candidate = crate::CardDefinitionBuilder::new(CardId::new(), "Candidate")
        .card_types(vec![CardType::Instant])
        .build();
    let body = crate::CardDefinitionBuilder::new(CardId::new(), "Fighter")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    struct Decline;
    impl crate::decision::DecisionMaker for Decline {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }
    }
    for ordinal in [1, 2, 3] {
        for target_kind in [0, 1, 2, 3] {
            for removed in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let (alice, bob) = (game.players[0].id, game.players[1].id);
                game.create_object_from_definition(&card, alice, Zone::Battlefield);
                let permanent = game.create_object_from_definition(&body, bob, Zone::Battlefield);
                let stack_target = game.create_object_from_definition(&candidate, bob, Zone::Stack);
                game.stack
                    .push(crate::game_state::StackEntry::new(stack_target, bob));
                game.create_object_from_definition(&candidate, alice, Zone::Library);
                for _ in 1..ordinal {
                    let id = game.create_object_from_definition(&candidate, alice, Zone::Stack);
                    let event = crate::triggers::TriggerEvent::new_with_provenance(
                        crate::events::SpellCastEvent::new(id, alice, Zone::Hand),
                        crate::provenance::ProvNodeId::default(),
                    );
                    game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event);
                    game.move_object_by_effect(id, Zone::Graveyard);
                }
                game.take_pending_trigger_events();
                let original = game.create_object_from_definition(&candidate, alice, Zone::Stack);
                let mut entry = crate::game_state::StackEntry::new(original, alice);
                entry.targets = match target_kind {
                    1 => vec![crate::game_state::Target::Player(bob)],
                    2 => vec![crate::game_state::Target::Object(permanent)],
                    3 => vec![crate::game_state::Target::Object(stack_target)],
                    _ => vec![],
                };
                game.stack.push(entry);
                let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                    game.object(original).unwrap(),
                    &game,
                );
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::SpellCastEvent::new_with_snapshot(
                        original,
                        alice,
                        Zone::Hand,
                        snapshot,
                    ),
                    crate::provenance::ProvNodeId::default(),
                );
                game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event.clone());
                let triggers = crate::triggers::check_triggers(&game, &event);
                assert_eq!(triggers.len(), usize::from(ordinal == 2));
                if ordinal != 2 {
                    continue;
                }
                let mut queue = crate::triggers::TriggerQueue::new();
                for trigger in triggers {
                    queue.add(trigger);
                }
                crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                if removed {
                    game.move_object_by_effect(original, Zone::Graveyard)
                        .unwrap();
                }
                let before = game.stack.len() - 1;
                crate::game_loop::resolve_stack_entry_with(&mut game, &mut Decline).unwrap();
                let copied = !removed && matches!(target_kind, 1 | 2);
                assert_eq!(
                    game.stack.len(),
                    before + usize::from(copied),
                    "target kind {target_kind}, removed {removed}"
                );
                assert_eq!(
                    game.player(alice).unwrap().hand.len(),
                    usize::from(!copied),
                    "target kind {target_kind}, removed {removed}"
                );
            }
        }
    }
}

#[test]
fn cohort_multiword_source_alias_preserves_entry_and_excludes_only_source_identity() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Nyvora Silverbranch of the Vale")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("When Nyvora Silverbranch enters, create a Food token.\nWhenever you create a token, put a +1/+1 counter on target creature you control other than Nyvora Silverbranch.").unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card),
        [
            "When Nyvora Silverbranch enters, create a Food token.",
            "Whenever you create a token, put a +1/+1 counter on target creature you control other than Nyvora Silverbranch.",
        ]
    );
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let same_name = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let opponent = game.create_object_from_definition(&card, bob, Zone::Battlefield);
    let counter = card
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .last()
        .unwrap();
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &counter.effects,
        alice,
        Some(source),
        None,
    );
    assert_eq!(
        requirements.len(),
        1,
        "effects={:#?}; choices={:#?}",
        counter.effects,
        counter.choices
    );
    assert!(
        !requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(source))
    );
    assert!(
        requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(same_name))
    );
    assert!(
        !requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(opponent))
    );
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(same_name)]);
    for effect in counter.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(
        game.counter_count(same_name, crate::CounterType::PlusOnePlusOne),
        1
    );
    assert_eq!(
        game.counter_count(source, crate::CounterType::PlusOnePlusOne),
        0
    );
    assert_eq!(
        game.counter_count(opponent, crate::CounterType::PlusOnePlusOne),
        0
    );
}
