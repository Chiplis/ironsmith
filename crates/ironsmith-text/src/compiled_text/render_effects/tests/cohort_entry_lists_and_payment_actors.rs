use super::*;
use crate::ability::AbilityKind;
use crate::card::PowerToughness;
use crate::game_state::{GameState, StackEntry, Target};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::mana::{ManaCost, ManaSymbol};

fn body(name: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 5))
        .build()
}
fn entered(source: ObjectId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            source,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn cohort_descend_optional_bounce_excludes_source_and_lands_and_rechecks_graveyard() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Ravelon Echo")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(4,4))
        .parse_text("Flying\nDescend 4 — When this creature enters, if there are four or more permanent cards in your graveyard, return up to one target nonland permanent other than this creature to its owner's hand.").unwrap();
    struct Choice {
        target: Option<ObjectId>,
        excluded: Vec<ObjectId>,
    }
    impl crate::decision::DecisionMaker for Choice {
        fn decide_targets(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            assert_eq!(ctx.requirements[0].min_targets, 0);
            assert_eq!(ctx.requirements[0].max_targets, Some(1));
            for id in &self.excluded {
                assert!(
                    !ctx.requirements[0]
                        .legal_targets
                        .contains(&Target::Object(*id))
                );
            }
            self.target.map(Target::Object).into_iter().collect()
        }
    }
    for case in 0..4 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let target =
            game.create_object_from_definition(&body("Other permanent"), bob, Zone::Battlefield);
        let land = crate::CardDefinitionBuilder::new(CardId::new(), "Land")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_definition(&land, bob, Zone::Battlefield);
        let mut graves = vec![];
        for i in 0..4 {
            graves.push(game.create_object_from_definition(
                &body("Permanent card"),
                if case == 0 && i == 3 { bob } else { alice },
                Zone::Graveyard,
            ));
        }
        let instant = crate::CardDefinitionBuilder::new(CardId::new(), "Nonpermanent")
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_definition(&instant, alice, Zone::Graveyard);
        let triggers = crate::triggers::check_triggers(&game, &entered(source));
        assert_eq!(triggers.len(), usize::from(case != 0));
        let mut queue = crate::triggers::TriggerQueue::new();
        for t in triggers {
            queue.add(t);
        }
        let mut choice = Choice {
            target: (case != 1).then_some(target),
            excluded: vec![source, land],
        };
        crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choice)
            .unwrap();
        if case == 3 {
            game.move_object_by_effect(graves[0], Zone::Exile).unwrap();
        }
        if case != 0 {
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        assert_eq!(game.battlefield.contains(&target), case != 2);
        assert!(game.battlefield.contains(&source));
        assert!(game.battlefield.contains(&land));
        assert_eq!(game.player(bob).unwrap().hand.len(), usize::from(case == 2));
    }
}

#[test]
fn cohort_subtype_mana_allows_kindred_spells_and_matching_sources_in_other_zones() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Keeper").card_types(vec![CardType::Creature])
        .parse_text("{T}: Add one mana of any color. Spend this mana only to cast a Dinosaur spell or activate an ability of a Dinosaur source.").unwrap();
    let AbilityKind::Activated(ability) = &card.abilities[0].kind else {
        panic!("mana ability")
    };
    for dinosaur in [false, true] {
        for card_type in [CardType::Creature, CardType::Instant] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            game.player_mut(alice).unwrap().add_restricted_mana(
                crate::ability::RestrictedManaUnit {
                    symbol: ManaSymbol::Green,
                    source,
                    source_chosen_creature_type: None,
                    restrictions: ability.mana_usage_restrictions.clone(),
                },
            );
            let candidate = crate::CardDefinitionBuilder::new(CardId::new(), "Candidate")
                .card_types(vec![card_type, CardType::Kindred])
                .subtypes(vec![if dinosaur {
                    Subtype::Dinosaur
                } else {
                    Subtype::Human
                }])
                .build();
            for zone in [Zone::Stack, Zone::Hand, Zone::Graveyard, Zone::Battlefield] {
                let id = game.create_object_from_definition(&candidate, alice, zone);
                let reason = if zone == Zone::Stack {
                    game.stack.push(StackEntry::new(id, alice));
                    crate::costs::PaymentReason::CastSpell
                } else {
                    crate::costs::PaymentReason::ActivateAbility
                };
                assert_eq!(
                    game.can_pay_mana_cost_with_reason(
                        alice,
                        Some(id),
                        &ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]),
                        0,
                        reason
                    ),
                    dinosaur,
                    "{zone:?} {card_type:?}"
                );
            }
        }
    }
}

#[test]
fn cohort_attacking_aura_charges_defender_per_blocker_and_follows_attachment() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Presence").card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura])
        .parse_text("Enchant creature\nEnchanted creature can't be blocked unless defending player pays {3} for each creature they control that's blocking it.").unwrap();
    for blockers in [1, 2] {
        for reattached in [false, true] {
            for enough in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let (alice, bob) = (game.players[0].id, game.players[1].id);
                let attacker =
                    game.create_object_from_definition(&body("Attacker"), alice, Zone::Battlefield);
                let other =
                    game.create_object_from_definition(&body("Other"), alice, Zone::Battlefield);
                let aura = game.create_object_from_definition(&card, alice, Zone::Battlefield);
                game.attach_object_to_target(
                    aura,
                    crate::object::AttachmentTarget::Object(attacker),
                );
                if reattached {
                    game.attach_object_to_target(
                        aura,
                        crate::object::AttachmentTarget::Object(other),
                    );
                }
                let declarations = (0..blockers)
                    .map(|_| crate::decision::BlockerDeclaration {
                        blocker: game.create_object_from_definition(
                            &body("Blocker"),
                            bob,
                            Zone::Battlefield,
                        ),
                        blocking: attacker,
                    })
                    .collect::<Vec<_>>();
                let mana = if enough {
                    3 * blockers
                } else {
                    3 * blockers - 1
                };
                game.player_mut(bob)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, mana);
                let mut combat = crate::combat_state::CombatState::default();
                combat.attackers.push(crate::combat_state::AttackerInfo {
                    creature: attacker,
                    target: crate::combat_state::AttackTarget::Player(bob),
                });
                let mut queue = crate::triggers::TriggerQueue::new();
                let mut dm = crate::decision::AutoPassDecisionMaker;
                let transaction = crate::game_loop::begin_blocker_declaration_transaction(
                    &game,
                    &combat,
                    &queue,
                    &declarations,
                    bob,
                    &mut dm,
                );
                let result = transaction.and_then(|t| {
                    crate::game_loop::finish_blocker_declaration_transaction(
                        t,
                        &mut game,
                        &mut combat,
                        &mut queue,
                        &mut dm,
                    )
                });
                assert_eq!(
                    result.is_ok(),
                    reattached || enough,
                    "blockers={blockers} moved={reattached} enough={enough}: {result:?}"
                );
                assert_eq!(
                    game.player(bob).unwrap().mana_pool.total(),
                    if !reattached && enough { 0 } else { mana }
                );
            }
        }
    }
}

#[test]
fn cohort_entry_counter_list_is_present_on_entry_and_moves_only_chosen_counter() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Kit").card_types(vec![CardType::Artifact])
        .parse_text("This artifact enters with a +1/+1 counter, a flying counter, a deathtouch counter, and a shield counter on it.\nWhenever a creature you control enters, you may move a counter from this artifact onto that creature.\n{2}, Sacrifice this artifact: Draw a card.").unwrap();
    let lines = crate::compiled_text::compiled_text_lines(&card);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(
        lines[0].contains(
            "a +1/+1 counter, a flying counter, a deathtouch counter, and a shield counter"
        ),
        "{lines:?}"
    );
    assert!(lines[1].contains("may move a counter"), "{lines:?}");
    struct Move {
        accept: bool,
        counter: crate::CounterType,
    }
    impl crate::decision::DecisionMaker for Move {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.accept
        }
        fn decide_counters(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::CountersContext,
        ) -> Vec<(crate::CounterType, u32)> {
            assert!(ctx.available_counters.contains(&(self.counter, 1)));
            vec![(self.counter, 1)]
        }
    }
    let counters = [
        crate::CounterType::PlusOnePlusOne,
        crate::CounterType::Flying,
        crate::CounterType::Deathtouch,
        crate::CounterType::Shield,
    ];
    for selected in counters {
        for accept in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let (alice, bob) = (game.players[0].id, game.players[1].id);
            let stack = game.create_object_from_definition(&card, alice, Zone::Stack);
            let source = game
                .move_object_with_etb_processing(stack, Zone::Battlefield)
                .unwrap()
                .new_id;
            for counter in counters {
                assert_eq!(game.counter_count(source, counter), 1, "{counter:?}");
            }
            game.take_pending_trigger_events();
            let enemy = game.create_object_from_definition(&body("Enemy"), bob, Zone::Battlefield);
            assert!(crate::triggers::check_triggers(&game, &entered(enemy)).is_empty());
            let host = game.create_object_from_definition(&body("Host"), alice, Zone::Battlefield);
            let mut queue = crate::triggers::TriggerQueue::new();
            for t in crate::triggers::check_triggers(&game, &entered(host)) {
                queue.add(t);
            }
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(game.stack.len(), 1);
            crate::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut Move {
                    accept,
                    counter: selected,
                },
            )
            .unwrap();
            for counter in counters {
                let moved = u32::from(accept && counter == selected);
                assert_eq!(game.counter_count(source, counter), 1 - moved);
                assert_eq!(game.counter_count(host, counter), moved);
            }
            game.take_pending_trigger_events();
            game.create_object_from_definition(&body("Drawn card"), alice, Zone::Library);
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 2);
            let action=crate::decision::compute_legal_actions(&game,alice).into_iter().find(|action|matches!(action,crate::decision::LegalAction::ActivateAbility{source:id,..} if *id==source)).expect("sacrifice activation");
            let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
            let mut dm = crate::decision::AutoPassDecisionMaker;
            let mut progress = crate::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &crate::game_loop::PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .unwrap();
            for _ in 0..12 {
                if !game.stack.is_empty() {
                    break;
                }
                let crate::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                    panic!("activation stalled: {progress:?}")
                };
                progress = crate::game_loop::apply_decision_context_with_dm(
                    &mut game, &mut queue, &mut state, &ctx, &mut dm,
                )
                .unwrap();
            }
            assert_eq!(game.stack.len(), 1);
            assert!(!game.battlefield.contains(&source));
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(alice).unwrap().hand.len(), 1);
        }
    }
}

#[test]
fn cohort_bounced_permanents_controller_sacrifices_own_land_before_copying() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Ravelon Chain").card_types(vec![CardType::Instant])
        .parse_text("Return target nonland permanent to its owner's hand. Then that permanent's controller may sacrifice a land of their choice. If the player does, they may copy this spell and may choose a new target for that copy.").unwrap();
    struct Choices {
        payer: PlayerId,
        land: ObjectId,
        target: ObjectId,
        accept: bool,
        copy: bool,
        asked: usize,
    }
    impl crate::decision::DecisionMaker for Choices {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            assert_eq!(ctx.player, self.payer);
            self.asked += 1;
            if self.asked == 1 {
                self.accept
            } else {
                self.copy
            }
        }
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert_eq!(ctx.player, self.payer);
            for c in ctx.candidates.iter().filter(|c| c.legal) {
                assert_eq!(game.current_controller(c.id), Some(self.payer));
                assert!(game.object_has_card_type(c.id, CardType::Land));
            }
            if ctx.candidates.iter().any(|c| c.legal && c.id == self.land) {
                vec![self.land]
            } else {
                vec![]
            }
        }
        fn decide_targets(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            assert_eq!(ctx.player, self.payer);
            vec![Target::Object(self.target)]
        }
    }
    for case in 0..4 {
        let accept = case != 0;
        let has_land = case != 2;
        let copy = case != 3;
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let (alice, bob, charlie) = (game.players[0].id, game.players[1].id, game.players[2].id);
        let bounced =
            game.create_object_from_definition(&body("Bounced"), charlie, Zone::Battlefield);
        game.set_current_controller(bounced, bob);
        let target =
            game.create_object_from_definition(&body("Copy target"), alice, Zone::Battlefield);
        let land = crate::CardDefinitionBuilder::new(CardId::new(), "Land")
            .card_types(vec![CardType::Land])
            .build();
        let ours = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        let theirs = game.create_object_from_definition(&land, bob, Zone::Battlefield);
        if !has_land {
            game.move_object_by_effect(theirs, Zone::Hand).unwrap();
        }
        let source = game.create_object_from_definition(&card, alice, Zone::Stack);
        let mut entry = StackEntry::new(source, alice);
        entry.targets = vec![Target::Object(bounced)];
        game.stack.push(entry);
        let mut dm = Choices {
            payer: bob,
            land: theirs,
            target,
            accept,
            copy,
            asked: 0,
        };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.asked > 0, has_land);
        assert_eq!(game.player(charlie).unwrap().hand.len(), 1);
        assert!(game.battlefield.contains(&ours));
        assert_eq!(game.battlefield.contains(&theirs), has_land && !accept);
        let copied = accept && has_land && copy;
        assert_eq!(game.stack.len(), usize::from(copied), "case={case}");
        if copied {
            assert_eq!(game.stack[0].controller, bob);
            assert_eq!(game.stack[0].targets, vec![Target::Object(target)]);
        }
    }
}
