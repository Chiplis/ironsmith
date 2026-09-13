use super::*;
use crate::card::PowerToughness;
use crate::game_state::{GameState, StackEntry, Target};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::mana::{ManaCost, ManaSymbol};

fn body(name: &str, symbol: ManaSymbol, toughness: i32) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![symbol]))
        .power_toughness(PowerToughness::fixed(2, toughness))
        .build()
}

#[test]
fn cohort_reflexive_pump_uses_original_life_gain_amount_after_hybrid_payment() {
    struct Payment {
        accept: bool,
        target: ObjectId,
        target_choices: usize,
    }
    impl crate::decision::DecisionMaker for Payment {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.accept
        }
        fn decide_targets(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.target_choices += 1;
            vec![Target::Object(self.target)]
        }
    }
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Honeykeeper")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Whenever you gain life, you may pay {G/W}. When you do, target creature gets +X/+X until end of turn, where X is the amount of life you gained.").unwrap();
    for amount in [1, 3, 7] {
        for mana in [None, Some(ManaSymbol::Green), Some(ManaSymbol::White)] {
            for accept in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let (alice, bob) = (game.players[0].id, game.players[1].id);
                let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
                let target = game.create_object_from_definition(
                    &body("Veloris", ManaSymbol::Blue, 3),
                    bob,
                    Zone::Battlefield,
                );
                if let Some(mana) = mana {
                    game.player_mut(alice).unwrap().mana_pool.add(mana, 1);
                }
                let other_event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::LifeGainEvent::new(bob, amount),
                    crate::provenance::ProvNodeId::default(),
                );
                assert!(crate::triggers::check_triggers(&game, &other_event).is_empty());
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::LifeGainEvent::new(alice, amount),
                    crate::provenance::ProvNodeId::default(),
                );
                let triggers = crate::triggers::check_triggers(&game, &event);
                assert_eq!(triggers.len(), 1);
                let mut queue = crate::triggers::TriggerQueue::new();
                for trigger in triggers {
                    queue.add(trigger);
                }
                crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                let mut payment = Payment {
                    accept,
                    target,
                    target_choices: 0,
                };
                crate::game_loop::resolve_stack_entry_with(&mut game, &mut payment).unwrap();
                let paid = accept && mana.is_some();
                assert_eq!(
                    game.current_power(target),
                    Some(2),
                    "the reflexive ability must use a separate stack entry"
                );
                assert_eq!(game.stack.len(), usize::from(paid));
                assert_eq!(payment.target_choices, usize::from(paid));
                assert_eq!(
                    game.player(alice).unwrap().mana_pool.total(),
                    u32::from(mana.is_some() && !paid)
                );
                if paid {
                    // The event's amount survives the source leaving before
                    // the separate reflexive ability resolves.
                    game.move_object_by_effect(source, Zone::Exile).unwrap();
                    crate::game_loop::resolve_stack_entry_with(&mut game, &mut payment).unwrap();
                }
                let bonus = if paid { amount as i32 } else { 0 };
                assert_eq!(
                    game.current_power(target),
                    Some(2 + bonus),
                    "amount={amount}, mana={mana:?}, accept={accept}"
                );
                assert_eq!(game.current_toughness(target), Some(3 + bonus));
                game.effect_store.continuous_effects.cleanup_end_of_turn();
                game.turn.turn_number += 1;
                assert_eq!(game.current_power(target), Some(2));
            }
        }
    }
}

#[test]
fn cohort_life_gain_where_x_uses_prior_gain_when_there_is_no_trigger_event() {
    for amount in [0, 3, 6] {
        let text = format!(
            "You gain {amount} life. Target creature gets +X/+X until end of turn, where X is the amount of life you gained."
        );
        let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Renewal")
            .card_types(vec![CardType::Instant])
            .parse_text(&text)
            .unwrap();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let target = game.create_object_from_definition(
            &body("Veloris", ManaSymbol::Blue, 3),
            bob,
            Zone::Battlefield,
        );
        let source = game.create_object_from_definition(&card, alice, Zone::Stack);
        game.stack
            .push(StackEntry::new(source, alice).with_targets(vec![Target::Object(target)]));
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 20 + amount);
        assert_eq!(game.current_power(target), Some(2 + amount));
        assert_eq!(game.current_toughness(target), Some(3 + amount));
    }
}

#[test]
fn cohort_attached_color_restriction_tracks_host_not_aura_and_follows_reattachment() {
    for aura_black in [false, true] {
        for host_black in [true, false] {
            let aura = crate::CardDefinitionBuilder::new(CardId::new(), "Veloran Binding")
                .card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura])
                .mana_cost(ManaCost::from_symbols(vec![if aura_black { ManaSymbol::Black } else { ManaSymbol::White }]))
                .parse_text("Enchant creature\nEnchanted creature can't attack.\nEnchanted creature can't block if it's black.").unwrap();
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let (alice, bob) = (game.players[0].id, game.players[1].id);
            let source = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
            let host = game.create_object_from_definition(
                &body(
                    "Host",
                    if host_black {
                        ManaSymbol::Black
                    } else {
                        ManaSymbol::White
                    },
                    3,
                ),
                alice,
                Zone::Battlefield,
            );
            let other = game.create_object_from_definition(
                &body("Other", ManaSymbol::Black, 3),
                alice,
                Zone::Battlefield,
            );
            let attacker = game.create_object_from_definition(
                &body("Attacker", ManaSymbol::Blue, 3),
                bob,
                Zone::Battlefield,
            );
            game.remove_summoning_sickness(host);
            game.remove_summoning_sickness(other);
            game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(host));
            game.update_cant_effects();
            assert!(!crate::rules::combat::can_attack(
                game.object(host).unwrap(),
                &game
            ));
            assert!(crate::rules::combat::can_attack(
                game.object(other).unwrap(),
                &game
            ));
            assert_eq!(
                crate::rules::combat::can_block(
                    game.object(attacker).unwrap(),
                    game.object(host).unwrap(),
                    &game
                ),
                !host_black,
                "aura black={aura_black}, host black={host_black}"
            );
            assert!(crate::rules::combat::can_block(
                game.object(attacker).unwrap(),
                game.object(other).unwrap(),
                &game
            ));
            game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(other));
            game.update_cant_effects();
            assert!(crate::rules::combat::can_attack(
                game.object(host).unwrap(),
                &game
            ));
            assert!(crate::rules::combat::can_block(
                game.object(attacker).unwrap(),
                game.object(host).unwrap(),
                &game
            ));
            assert!(!crate::rules::combat::can_attack(
                game.object(other).unwrap(),
                &game
            ));
            assert!(!crate::rules::combat::can_block(
                game.object(attacker).unwrap(),
                game.object(other).unwrap(),
                &game
            ));
        }
    }
}

#[test]
fn cohort_modified_keyword_checks_counters_equipment_and_aura_controller() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Zeravin Gargoyle")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text("This creature has flying as long as it's modified.")
        .unwrap();
    for mode in 0..4 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let flying = |game: &GameState| {
            game.current_has_static_ability_id(
                source,
                crate::static_abilities::StaticAbilityId::Flying,
            )
        };
        assert!(!flying(&game));
        if mode == 0 {
            game.add_counters(source, crate::CounterType::Shield, 1);
            assert!(flying(&game));
            game.remove_counters(source, crate::CounterType::Shield, 1, None, None);
            assert!(!flying(&game));
            continue;
        }
        let equipment = mode == 1;
        let attachment = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Attachment")
            .card_types(vec![if equipment {
                CardType::Artifact
            } else {
                CardType::Enchantment
            }])
            .subtypes(vec![if equipment {
                Subtype::Equipment
            } else {
                Subtype::Aura
            }])
            .parse_text(if equipment { "" } else { "Enchant creature" })
            .unwrap();
        let controller = if mode == 2 { alice } else { bob };
        let attached =
            game.create_object_from_definition(&attachment, controller, Zone::Battlefield);
        game.attach_object_to_target(attached, crate::object::AttachmentTarget::Object(source));
        assert_eq!(flying(&game), mode != 3, "mode={mode}");
        if mode == 3 {
            game.set_current_controller(attached, alice);
            game.attach_object_to_target(attached, crate::object::AttachmentTarget::Object(source));
            assert!(
                flying(&game),
                "an Aura starts modifying the host when their controllers match"
            );
        }
        game.detach_object_from_current_target(attached);
        assert!(!flying(&game));
    }
}

#[test]
fn cohort_chosen_hand_card_supplies_toughness_before_same_opponent_discards_it() {
    struct Choice {
        selected: Option<ObjectId>,
        chooser: PlayerId,
        expected_legal: Vec<ObjectId>,
    }
    impl crate::decision::DecisionMaker for Choice {
        fn decide_objects(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx.player == self.chooser {
                let mut legal = ctx
                    .candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id)
                    .collect::<Vec<_>>();
                legal.sort();
                let mut expected = self.expected_legal.clone();
                expected.sort();
                assert_eq!(
                    legal, expected,
                    "only green/white creatures from the targeted hand are eligible"
                );
            }
            self.selected.into_iter().collect()
        }
    }
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Rejection")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target opponent reveals their hand. You choose a green or white creature card from it. You gain life equal to that creature card's toughness, then that player discards that card.").unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&card).join("\n");
    assert!(
        rendered.contains("reveals their hand. You choose ")
            && rendered.contains(" creature card from it."),
        "the reveal and the choice from that hand retain their actors: {rendered}"
    );
    for selection in [None, Some(0), Some(1)] {
        for toughness in [1, 5] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let (alice, bob, charlie) =
                (game.players[0].id, game.players[1].id, game.players[2].id);
            let source = game.create_object_from_definition(&card, alice, Zone::Stack);
            let mut eligible = Vec::new();
            if selection.is_some() {
                for symbol in [ManaSymbol::White, ManaSymbol::Green] {
                    eligible.push(game.create_object_from_definition(
                        &body("Eligible", symbol, toughness),
                        bob,
                        Zone::Hand,
                    ));
                }
            }
            let blue = game.create_object_from_definition(
                &body("Blue", ManaSymbol::Blue, 9),
                bob,
                Zone::Hand,
            );
            let spell = crate::CardDefinitionBuilder::new(CardId::new(), "Green spell")
                .card_types(vec![CardType::Sorcery])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
                .build();
            let noncreature = game.create_object_from_definition(&spell, bob, Zone::Hand);
            let other_hand = game.create_object_from_definition(
                &body("Other hand", ManaSymbol::Green, 9),
                charlie,
                Zone::Hand,
            );
            let battlefield = game.create_object_from_definition(
                &body("Battlefield", ManaSymbol::Green, 9),
                bob,
                Zone::Battlefield,
            );
            let selected = selection.map(|index| eligible[index]);
            let mut choice = Choice {
                selected,
                chooser: alice,
                expected_legal: eligible.clone(),
            };
            game.stack
                .push(StackEntry::new(source, alice).with_targets(vec![Target::Player(bob)]));
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut choice).unwrap();
            assert_eq!(
                game.player(alice).unwrap().life,
                20 + if selected.is_some() { toughness } else { 0 },
                "selection={selection:?}, toughness={toughness}"
            );
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(
                game.player(bob).unwrap().graveyard.len(),
                usize::from(selected.is_some())
            );
            for id in eligible {
                assert_eq!(game.object(id).is_none(), Some(id) == selected);
            }
            for id in [blue, noncreature, other_hand, battlefield] {
                assert!(game.object(id).is_some());
            }
        }
    }
}
