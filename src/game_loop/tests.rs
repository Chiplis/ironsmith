#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::ability::AbilityKind;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::AttackTarget;
    use crate::decision::AutoPassDecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::events::EventKind;
    use crate::game_state::Phase;
    use crate::ids::CardId;
    use crate::static_abilities::StaticAbility;
    use crate::triggers::Trigger;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_generate_damage_triggers_emits_life_loss_for_player_damage() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();

        let events = vec![CombatDamageEvent {
            source: ObjectId::from_raw(99),
            target: DamageEventTarget::Player(PlayerId::from_index(1)),
            amount: 3,
            life_lost: 3,
            result: DamageResult::default(),
        }];

        generate_damage_triggers(&mut game, &events, &mut trigger_queue);

        assert_eq!(
            game.trigger_event_kind_count_this_turn(EventKind::Damage),
            1
        );
        assert_eq!(
            game.trigger_event_kind_count_this_turn(EventKind::LifeLoss),
            1
        );
        assert_eq!(
            game.damage_to_players_this_turn
                .get(&PlayerId::from_index(1)),
            Some(&3)
        );
    }

    #[test]
    fn test_queue_triggers_tracks_noncombat_damage_to_players_this_turn() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(200);

        let event = TriggerEvent::new(DamageEvent::new(
            source,
            EventDamageTarget::Player(bob),
            4,
            false,
        ));
        queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

        assert_eq!(game.damage_to_players_this_turn.get(&bob), Some(&4));
        assert_eq!(
            game.noncombat_damage_to_players_this_turn.get(&bob),
            Some(&4)
        );
    }

    #[test]
    fn test_queue_triggers_tracks_life_gained_this_turn() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let alice = PlayerId::from_index(0);

        let event = TriggerEvent::new(LifeGainEvent::new(alice, 5));
        queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

        assert_eq!(game.life_gained_this_turn.get(&alice), Some(&5));
    }

    #[test]
    fn test_queue_triggers_tracks_life_lost_this_turn() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let bob = PlayerId::from_index(1);

        let event = TriggerEvent::new(LifeLossEvent::from_effect(bob, 3));
        queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

        assert_eq!(game.life_lost_this_turn.get(&bob), Some(&3));
    }

    #[test]
    fn test_drain_pending_events_checks_delayed_zone_change_triggers() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let alice = PlayerId::from_index(0);

        let stangg_id = create_creature(&mut game, "Stangg", alice, 3, 4);
        let twin_id = create_creature(&mut game, "Stangg Twin", alice, 3, 4);

        game.delayed_triggers.push(crate::triggers::DelayedTrigger {
            trigger: Trigger::this_leaves_battlefield(),
            effects: vec![Effect::move_to_zone(
                ChooseSpec::SpecificObject(twin_id),
                Zone::Exile,
                true,
            )],
            one_shot: true,
            x_value: None,
            not_before_turn: None,
            expires_at_turn: None,
            target_objects: vec![stangg_id],
            ability_source: None,
            controller: alice,
            choices: vec![],
        });

        let moved = game.move_object(stangg_id, Zone::Graveyard);
        assert!(moved.is_some(), "expected Stangg to move to graveyard");
        assert!(
            !game.pending_trigger_events.is_empty(),
            "moving Stangg off battlefield should queue a zone-change trigger event"
        );

        drain_pending_trigger_events(&mut game, &mut trigger_queue);

        assert!(
            !trigger_queue.entries.is_empty(),
            "pending zone-change events should check delayed triggers"
        );
        assert_eq!(
            trigger_queue.entries[0].source, stangg_id,
            "delayed trigger source should be the leaving permanent"
        );

        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("delayed trigger should be put on stack");
        assert_eq!(
            game.stack.len(),
            1,
            "expected delayed trigger ability on stack"
        );

        resolve_stack_entry(&mut game).expect("delayed trigger should resolve");

        assert!(
            !game.battlefield.contains(&twin_id),
            "Stangg Twin should no longer be on battlefield after delayed exile resolves"
        );
    }

    #[test]
    fn test_pending_zone_change_still_drives_non_delayed_triggered_abilities() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let alice = PlayerId::from_index(0);

        let stangg_id = create_creature(&mut game, "Stangg", alice, 3, 4);
        let twin_id = create_creature(&mut game, "Stangg Twin", alice, 3, 4);

        if let Some(stangg) = game.object_mut(stangg_id) {
            let filter = ObjectFilter::default().token().named("Stangg Twin");
            stangg.abilities.push(Ability::triggered(
                Trigger::leaves_battlefield(filter),
                vec![Effect::sacrifice_source()],
            ));
        } else {
            panic!("expected Stangg object to exist");
        }

        let moved = game.move_object(twin_id, Zone::Graveyard);
        assert!(moved.is_some(), "expected Stangg Twin to move to graveyard");

        drain_pending_trigger_events(&mut game, &mut trigger_queue);
        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("triggered ability should be put on stack");
        assert_eq!(
            game.stack.len(),
            1,
            "expected sacrifice trigger on stack after Stangg Twin left"
        );

        resolve_stack_entry(&mut game).expect("sacrifice trigger should resolve");

        assert!(
            !game.battlefield.contains(&stangg_id),
            "Stangg should be sacrificed when Stangg Twin leaves the battlefield"
        );
    }

    #[test]
    fn test_stangg_linked_twin_sacrifice_survives_legend_rule_for_other_twin() {
        use crate::ability::AbilityKind;
        use crate::cards::CardDefinitionBuilder;
        use crate::events::zones::EnterBattlefieldEvent;
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::ids::CardId;
        use crate::triggers::TriggerEvent;
        use crate::zone::Zone;

        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let alice = PlayerId::from_index(0);

        let oracle = "When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the battlefield.";
        let def = CardDefinitionBuilder::new(CardId::new(), "Stangg")
            .parse_text(oracle)
            .expect("parse stangg text");
        let etb = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered)
                    if format!("{:?}", triggered.effects).contains("CreateTokenEffect") =>
                {
                    Some(triggered.clone())
                }
                _ => None,
            })
            .expect("expected stangg ETB trigger");

        let stangg_a = create_creature(&mut game, "Stangg", alice, 3, 4);
        let stangg_b = create_creature(&mut game, "Stangg", alice, 3, 4);

        for source in [stangg_a, stangg_b] {
            let mut dm = crate::decision::AutoPassDecisionMaker;
            let event = TriggerEvent::new(EnterBattlefieldEvent::new(source, Zone::Hand));
            let mut ctx =
                ExecutionContext::new(source, alice, &mut dm).with_triggering_event(event);
            for effect in &etb.effects {
                execute_effect(&mut game, effect, &mut ctx)
                    .expect("stangg ETB effect should resolve");
            }
        }

        let twins_before_sba = game
            .battlefield
            .iter()
            .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Stangg Twin"))
            .count();
        assert_eq!(
            twins_before_sba, 2,
            "expected two Stangg Twin tokens before legend rule applies"
        );

        check_and_apply_sbas(&mut game, &mut trigger_queue).expect("apply SBAs");

        let twins_after_sba = game
            .battlefield
            .iter()
            .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Stangg Twin"))
            .count();
        assert_eq!(twins_after_sba, 1, "legend rule should keep only one Twin");

        put_triggers_on_stack(&mut game, &mut trigger_queue).expect("queue triggered abilities");
        while !game.stack_is_empty() {
            resolve_stack_entry(&mut game).expect("resolve trigger");
        }

        let stangg_after_resolution = game
            .battlefield
            .iter()
            .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Stangg"))
            .count();
        assert_eq!(
            stangg_after_resolution, 1,
            "only the Stangg linked to the Twin that left should be sacrificed"
        );
    }

    #[test]
    fn test_turn_face_up_action_puts_turned_face_up_trigger_on_stack() {
        use crate::decision::{LegalAction, SelectFirstDecisionMaker};
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::static_abilities::StaticAbility;
        use crate::triggers::Trigger;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let card = CardBuilder::new(CardId::from_raw(42), "Face-Up Trigger Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities
                .push(Ability::static_ability(StaticAbility::morph(
                    ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
                )));
            obj.abilities.push(Ability::triggered(
                Trigger::this_is_turned_face_up(),
                vec![Effect::draw(1)],
            ));
        }
        game.set_face_down(creature_id);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Green, 1);

        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = SelectFirstDecisionMaker;
        let response = PriorityResponse::PriorityAction(LegalAction::TurnFaceUp { creature_id });
        let result = apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &response,
            &mut dm,
        );
        assert!(result.is_ok(), "turn-face-up action should succeed");

        let top = game
            .stack
            .last()
            .expect("triggered ability should be on stack");
        assert!(
            top.is_ability,
            "turned-face-up trigger should use ability stack entry"
        );
        assert_eq!(top.object_id, creature_id);
        assert!(
            top.triggering_event
                .as_ref()
                .is_some_and(|event| event.kind() == EventKind::TurnedFaceUp),
            "trigger stack entry should carry TurnedFaceUp trigger event"
        );
    }

    // === Target Extraction Tests ===

    #[cfg(feature = "net")]
    #[test]
    fn test_pip_payment_trace_order() {
        use crate::mana::ManaSymbol;

        let mut trace = Vec::new();
        let actions = vec![
            ManaPipPaymentAction::ActivateManaAbility {
                source_id: ObjectId::from_raw(5),
                ability_index: 1,
            },
            ManaPipPaymentAction::UseFromPool(ManaSymbol::Blue),
            ManaPipPaymentAction::PayViaAlternative {
                permanent_id: ObjectId::from_raw(6),
                effect: crate::decision::AlternativePaymentEffect::Convoke,
            },
            ManaPipPaymentAction::PayLife(2),
        ];

        for action in &actions {
            record_pip_payment_action(&mut trace, action);
        }

        assert_eq!(trace.len(), 4);
        assert!(matches!(
            trace[0],
            CostStep::Payment(CostPayment::ActivateManaAbility { .. })
        ));
        assert!(matches!(
            trace[1],
            CostStep::Mana(ManaSymbolSpec {
                symbol: ManaSymbolCode::Blue,
                ..
            })
        ));
        assert!(matches!(
            trace[2],
            CostStep::Payment(CostPayment::Tap { .. })
        ));
        assert!(matches!(
            trace[3],
            CostStep::Mana(ManaSymbolSpec {
                symbol: ManaSymbolCode::Life,
                value: 2,
            })
        ));
    }

    #[test]
    fn test_extract_target_spec_single_target() {
        // Destroy effect has single target
        let effect = Effect::destroy(ChooseSpec::creature());

        let extracted = extract_target_spec(&effect).expect("Should extract target");
        assert_eq!(extracted.min_targets, 1);
        assert_eq!(extracted.max_targets, Some(1));
    }

    #[test]
    fn test_extract_target_spec_any_number() {
        // Exile with any_number count (using exile_any_number helper)
        let effect = Effect::exile_any_number(ChooseSpec::spell());

        let extracted = extract_target_spec(&effect).expect("Should extract target");
        // ChoiceCount::any_number() returns min: 0, max: None
        assert_eq!(extracted.min_targets, 0, "any_number has min 0");
        assert_eq!(extracted.max_targets, None, "any_number has no max");
    }

    #[test]
    fn test_extract_target_spec_no_count() {
        // Exile with no count defaults to single target
        let effect = Effect::exile(ChooseSpec::creature());

        let extracted = extract_target_spec(&effect).expect("Should extract target");
        assert_eq!(extracted.min_targets, 1, "should default to min 1");
        assert_eq!(extracted.max_targets, Some(1), "should default to max 1");
    }

    #[test]
    fn test_extract_target_specs_pump_and_gain_clause_uses_single_target_selection() {
        use crate::cards::CardDefinitionBuilder;

        let def = CardDefinitionBuilder::new(CardId::new(), "Viashino Shanktail Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{2}{R}, Discard this card: Target attacking creature gets +3/+1 and gains first strike until end of turn.",
            )
            .expect("pump-and-gain clause should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected activated ability");

        let target_specs = activated
            .effects
            .iter()
            .filter_map(extract_target_spec)
            .filter(|extracted| requires_target_selection(extracted.spec))
            .count();

        assert_eq!(
            target_specs, 1,
            "expected a single target selection for combined pump+gain clause, got {target_specs}"
        );
    }

    #[test]
    fn test_spell_has_legal_targets_any_number_with_no_targets() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        // Exile any number of target spells
        let effects = vec![Effect::exile_any_number(ChooseSpec::spell())];

        // No spells on stack - but "any number" (min_targets == 0) means 0 targets is valid
        let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
        // "Any number" effects can be cast with 0 targets
        assert!(has_targets, "any_number effects can be cast with 0 targets");
    }

    #[test]
    fn test_spell_has_legal_targets_single_target_needs_target() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        // Single target exile spell - needs at least one target
        let effects = vec![Effect::exile(ChooseSpec::spell())];

        // No spells on stack
        let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
        assert!(
            !has_targets,
            "Single-target spell needs at least one legal target"
        );
    }

    #[test]
    fn test_spell_has_legal_targets_with_may_wrapper_needs_target() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let effects = vec![Effect::may(vec![Effect::counter(ChooseSpec::spell())])];

        let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
        assert!(
            !has_targets,
            "may-wrapped targeted effects must still require legal targets"
        );
    }

    #[test]
    fn test_spell_has_legal_targets_with_unless_action_wrapper_needs_target() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let effects = vec![Effect::unless_action(
            vec![Effect::counter(ChooseSpec::spell())],
            vec![Effect::gain_life(1)],
            crate::target::PlayerFilter::You,
        )];

        let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
        assert!(
            !has_targets,
            "unless-action wrapped targeted effects must still require legal targets"
        );
    }

    #[test]
    fn test_spell_has_legal_targets_with_sequence_wrapper_needs_target() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let effects = vec![Effect::new(crate::effects::SequenceEffect::new(vec![
            Effect::gain_life(1),
            Effect::counter(ChooseSpec::spell()),
        ]))];

        let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
        assert!(
            !has_targets,
            "sequence-wrapped targeted effects must still require legal targets"
        );
    }

    #[test]
    fn test_apply_blocker_declarations_allows_blocking_multiple_attackers_with_ability() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut combat = CombatState::default();

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker1 = create_creature(&mut game, "Attacker 1", alice, 2, 2);
        let attacker2 = create_creature(&mut game, "Attacker 2", alice, 2, 2);
        let blocker = create_creature(&mut game, "Blocker", bob, 1, 4);

        // Grant: "can block an additional creature each combat" so it can block two attackers.
        game.object_mut(blocker)
            .expect("blocker exists")
            .abilities
            .push(Ability {
                kind: AbilityKind::Static(
                    StaticAbility::can_block_additional_creature_each_combat(1),
                ),
                functional_zones: vec![Zone::Battlefield],
                text: None,
            });

        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker1,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker2,
            target: AttackTarget::Player(bob),
        });

        let decls = vec![
            BlockerDeclaration {
                blocker,
                blocking: attacker1,
            },
            BlockerDeclaration {
                blocker,
                blocking: attacker2,
            },
        ];

        apply_blocker_declarations(&mut game, &mut combat, &mut tq, &decls, bob)
            .expect("should allow blocker to block multiple attackers with ability");
    }

    #[test]
    fn test_apply_blocker_declarations_enforces_maximum_blockers() {
        let mut game = setup_game();
        let mut tq = TriggerQueue::new();
        let mut combat = CombatState::default();

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker = create_creature(&mut game, "Elusive Attacker", alice, 2, 2);
        let blocker1 = create_creature(&mut game, "Blocker 1", bob, 1, 1);
        let blocker2 = create_creature(&mut game, "Blocker 2", bob, 1, 1);

        // "Can't be blocked by more than one creature."
        game.object_mut(attacker)
            .expect("attacker exists")
            .abilities
            .push(Ability {
                kind: AbilityKind::Static(StaticAbility::cant_be_blocked_by_more_than(1)),
                functional_zones: vec![Zone::Battlefield],
                text: None,
            });

        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });

        let decls = vec![
            BlockerDeclaration {
                blocker: blocker1,
                blocking: attacker,
            },
            BlockerDeclaration {
                blocker: blocker2,
                blocking: attacker,
            },
        ];

        let err = apply_blocker_declarations(&mut game, &mut combat, &mut tq, &decls, bob)
            .expect_err("should reject too many blockers");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("InvalidBlockers"),
            "expected invalid blockers error, got {msg}"
        );
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn undying_effects() -> Vec<Effect> {
        let trigger_tag = "undying_trigger";
        let return_tag = "undying_return";
        let returned_tag = "undying_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let choose =
            Effect::choose_objects(filter, 1, crate::target::PlayerFilter::You, return_tag);
        let move_to_battlefield = Effect::move_to_zone(
            ChooseSpec::Tagged(return_tag.into()),
            Zone::Battlefield,
            true,
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );

        vec![
            Effect::tag_triggering_object(trigger_tag),
            choose,
            move_to_battlefield,
            counters,
        ]
    }

    fn persist_effects() -> Vec<Effect> {
        let trigger_tag = "persist_trigger";
        let return_tag = "persist_return";
        let returned_tag = "persist_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let choose =
            Effect::choose_objects(filter, 1, crate::target::PlayerFilter::You, return_tag);
        let move_to_battlefield = Effect::move_to_zone(
            ChooseSpec::Tagged(return_tag.into()),
            Zone::Battlefield,
            true,
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::MinusOneMinusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );

        vec![
            Effect::tag_triggering_object(trigger_tag),
            choose,
            move_to_battlefield,
            counters,
        ]
    }

    // === Stack Resolution Tests ===

    #[test]
    fn test_resolve_empty_stack() {
        let mut game = setup_game();
        let result = resolve_stack_entry(&mut game);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_stack_entry_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a simple instant
        let card = CardBuilder::new(CardId::from_raw(1), "Test Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Stack);

        // Put on stack
        let entry = StackEntry::new(spell_id, alice);
        game.push_to_stack(entry);

        // Resolve
        let result = resolve_stack_entry(&mut game);
        assert!(result.is_ok());

        // Stack should be empty
        assert!(game.stack_is_empty());

        // Spell should be in graveyard
        let player = game.player(alice).unwrap();
        assert_eq!(player.graveyard.len(), 1);
    }

    // === Combat Damage Tests ===

    #[test]
    fn test_unblocked_attacker_deals_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Attacker", alice, 3, 3);

        // Set up combat with attacker attacking Bob
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        // Execute combat damage
        let events = execute_combat_damage_step(&mut game, &combat, false);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 3);

        // Bob should have taken 3 damage
        assert_eq!(game.player(bob).unwrap().life, 17);
    }

    #[test]
    fn test_unblocked_attacker_uses_calculated_power_from_conditional_anthem() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Tek Test", alice, 2, 2);

        if let Some(attacker) = game.object_mut(attacker_id) {
            let swamp_condition = crate::ConditionExpr::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(
                    crate::filter::ObjectFilter::land()
                        .with_subtype(crate::types::Subtype::Swamp)
                        .you_control(),
                ),
                comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                display: Some("you control a Swamp".to_string()),
            };
            let anthem =
                crate::static_abilities::Anthem::for_source(2, 0).with_condition(swamp_condition);
            attacker.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::new(anthem),
            ));
        }

        let swamp = CardBuilder::new(CardId::new(), "Swamp")
            .card_types(vec![CardType::Land])
            .subtypes(vec![crate::types::Subtype::Swamp])
            .build();
        game.create_object_from_card(&swamp, alice, Zone::Battlefield);

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 4);
        assert_eq!(game.player(bob).unwrap().life, 16);
    }

    #[test]
    fn test_unblocked_attacker_uses_toughness_for_combat_damage_when_static_applies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let enabler = CardBuilder::new(CardId::new(), "Brontodon Enabler")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 6))
            .build();
        let enabler_id = game.create_object_from_card(&enabler, alice, Zone::Battlefield);
        if let Some(object) = game.object_mut(enabler_id) {
            object.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::creatures_you_control_assign_combat_damage_using_toughness(),
            ));
        }

        let attacker_id = create_creature(&mut game, "Wall Fighter", alice, 0, 3);

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].amount, 3,
            "attacker should assign combat damage equal to toughness"
        );
        assert_eq!(game.player(bob).unwrap().life, 17);
    }

    #[test]
    fn test_blocked_attacker_deals_damage_to_blocker() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Attacker", alice, 3, 3);
        let blocker_id = create_creature(&mut game, "Blocker", bob, 2, 2);

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, vec![blocker_id]);

        // Execute combat damage
        let events = execute_combat_damage_step(&mut game, &combat, false);

        // Should have events for attacker->blocker and blocker->attacker
        assert!(events.len() >= 2);

        // Blocker should have 2 damage (lethal - without trample, attacker only assigns lethal)
        assert_eq!(game.damage_on(blocker_id), 2);

        // Attacker should have 2 damage
        assert_eq!(game.damage_on(attacker_id), 2);

        // Bob should not have taken damage (attacker was blocked)
        assert_eq!(game.player(bob).unwrap().life, 20);
    }

    #[test]
    fn test_first_strike_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "First Striker", alice, 2, 2);

        // Add first strike
        if let Some(obj) = game.object_mut(attacker_id) {
            obj.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::first_strike(),
            ));
        }

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        // First strike damage step - should deal damage
        let events = execute_combat_damage_step(&mut game, &combat, true);
        assert_eq!(events.len(), 1);
        assert_eq!(game.player(bob).unwrap().life, 18);

        // Regular damage step - first strike creature shouldn't deal damage again
        let events = execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(events.len(), 0);
        assert_eq!(game.player(bob).unwrap().life, 18);
    }

    #[test]
    fn test_lifelink_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Lifelinker", alice, 3, 3);

        // Add lifelink
        if let Some(obj) = game.object_mut(attacker_id) {
            obj.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::lifelink(),
            ));
        }

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        // Execute combat damage
        let _events = execute_combat_damage_step(&mut game, &combat, false);

        // Bob took 3 damage
        assert_eq!(game.player(bob).unwrap().life, 17);

        // Alice gained 3 life
        assert_eq!(game.player(alice).unwrap().life, 23);
    }

    #[test]
    fn test_trample_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let attacker_id = create_creature(&mut game, "Trampler", alice, 5, 5);
        let blocker_id = create_creature(&mut game, "Small Blocker", bob, 2, 2);

        // Add trample
        if let Some(obj) = game.object_mut(attacker_id) {
            obj.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::trample(),
            ));
        }

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, vec![blocker_id]);

        // Execute combat damage
        let events = execute_combat_damage_step(&mut game, &combat, false);

        // Should have events: attacker->blocker, attacker->player (trample), blocker->attacker
        assert!(events.len() >= 3);

        // Blocker should have 2 damage (lethal)
        assert_eq!(game.damage_on(blocker_id), 2);

        // Attacker should have 2 damage (from blocker)
        assert_eq!(game.damage_on(attacker_id), 2);

        // Bob should have taken 3 trample damage (5 power - 2 toughness = 3 excess)
        assert_eq!(game.player(bob).unwrap().life, 17);
    }

    // === State-Based Actions Tests ===

    #[test]
    fn test_sba_creature_dies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = create_creature(&mut game, "Doomed", alice, 2, 2);

        // Deal lethal damage
        game.mark_damage(creature_id, 2);

        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Creature should be in graveyard
        assert_eq!(game.battlefield.len(), 0);
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    }

    #[test]
    fn test_sba_player_loses() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set life to 0
        game.player_mut(alice).unwrap().life = 0;

        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Alice should have lost
        assert!(game.player(alice).unwrap().has_lost);
    }

    // === Priority Loop Tests ===

    #[test]
    fn test_priority_loop_empty_stack() {
        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();

        // With empty stack and all passing, phase should end
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let result = run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm).unwrap();
        assert!(matches!(result, GameProgress::Continue));
    }

    // === Triggered Ability Tests ===

    #[test]
    fn test_etb_trigger_fires() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create creature with ETB trigger
        let creature_id = create_creature(&mut game, "ETB Creature", alice, 2, 2);
        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities.push(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::draw(1)],
            ));
        }

        // Simulate ETB event
        let event = TriggerEvent::new(crate::events::zones::ZoneChangeEvent::new(
            creature_id,
            Zone::Stack,
            Zone::Battlefield,
            None,
        ));

        let mut trigger_queue = TriggerQueue::new();
        let triggers = check_triggers(&game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }

        assert!(!trigger_queue.is_empty());
        assert_eq!(trigger_queue.entries.len(), 1);
    }

    #[test]
    fn test_dies_trigger_from_sba() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist-like creature
        let blood_artist_id = create_creature(&mut game, "Blood Artist", alice, 0, 1);
        if let Some(obj) = game.object_mut(blood_artist_id) {
            obj.abilities.push(Ability::triggered(
                Trigger::dies(crate::target::ObjectFilter::creature()),
                vec![Effect::gain_life(1)],
            ));
        }

        // Create victim creature with lethal damage
        let victim_id = create_creature(&mut game, "Victim", alice, 1, 1);
        game.mark_damage(victim_id, 1);

        // Apply SBAs - should trigger Blood Artist
        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Blood Artist should have triggered
        assert!(!trigger_queue.is_empty());
    }

    // === Integration Tests ===

    #[test]
    fn test_combat_damage_with_triggers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create attacker with "deals combat damage to player" trigger
        let attacker_id = create_creature(&mut game, "Ninja", alice, 2, 2);
        if let Some(obj) = game.object_mut(attacker_id) {
            obj.abilities.push(Ability::triggered(
                Trigger::this_deals_combat_damage_to_player(),
                vec![Effect::draw(1)],
            ));
        }

        // Set up combat
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker_id, Vec::new());

        // Execute combat damage
        let events = execute_combat_damage_step(&mut game, &combat, false);

        // Generate triggers
        let mut trigger_queue = TriggerQueue::new();
        generate_damage_triggers(&mut game, &events, &mut trigger_queue);

        // Should have triggered
        assert!(!trigger_queue.is_empty());
    }

    // === Full Game Flow Integration Test ===

    #[test]
    fn test_full_game_lightning_bolt_wins() {
        use crate::cards::definitions::{basic_mountain, lightning_bolt};
        use crate::mana::ManaSymbol;

        // Create a game with 2 players at 3 life (so Lightning Bolt is lethal)
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 3);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up for main phase (when spells can be cast)
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create Mountain on Alice's battlefield (using CardDefinition for abilities)
        let mountain = basic_mountain();
        let mountain_id = game.create_object_from_definition(&mountain, alice, Zone::Battlefield);

        // Remove summoning sickness from Mountain (it's a land)
        game.remove_summoning_sickness(mountain_id);

        // Create Lightning Bolt in Alice's hand
        let bolt = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Hand);

        // Verify initial state
        assert_eq!(game.player(alice).unwrap().life, 3);
        assert_eq!(game.player(bob).unwrap().life, 3);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);

        // Step 1: Activate Mountain's mana ability to add {R}
        // Find the mana ability index
        let mountain_obj = game.object(mountain_id).unwrap();
        let _mana_ability_index = mountain_obj
            .abilities
            .iter()
            .position(|a| a.is_mana_ability())
            .expect("Mountain should have a mana ability");

        // Tap mountain for red mana
        game.tap(mountain_id);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Verify mana was added
        assert_eq!(
            game.player(alice)
                .unwrap()
                .mana_pool
                .amount(ManaSymbol::Red),
            1
        );

        // Step 2: Cast Lightning Bolt targeting Bob
        // Move Lightning Bolt from hand to stack
        let stack_bolt_id = game.move_object(bolt_id, Zone::Stack).unwrap();

        // Create stack entry with Bob as target
        let entry = StackEntry::new(stack_bolt_id, alice).with_targets(vec![Target::Player(bob)]);
        game.push_to_stack(entry);

        // Pay the mana cost (remove red mana from pool)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .remove(ManaSymbol::Red, 1);

        // Verify spell is on stack
        assert!(!game.stack_is_empty());

        // Step 3: Resolve the stack (both players pass priority)
        let result = resolve_stack_entry(&mut game);
        assert!(result.is_ok(), "Stack resolution should succeed");

        // Verify Lightning Bolt dealt 3 damage to Bob
        assert_eq!(game.player(bob).unwrap().life, 0);

        // Lightning Bolt should be in graveyard
        assert!(game.stack_is_empty());
        let alice_graveyard = &game.player(alice).unwrap().graveyard;
        assert_eq!(alice_graveyard.len(), 1);

        // Step 4: Check state-based actions - Bob should lose
        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Bob should have lost the game
        assert!(
            game.player(bob).unwrap().has_lost,
            "Bob should have lost the game with 0 life"
        );
    }

    #[test]
    fn test_full_game_with_decision_maker() {
        use crate::cards::definitions::{basic_mountain, fireball};
        use crate::decision::DecisionMaker;

        #[derive(Debug)]
        struct TestResponseDecisionMaker {
            responses: Vec<PriorityResponse>,
            index: usize,
        }

        impl TestResponseDecisionMaker {
            fn new(responses: Vec<PriorityResponse>) -> Self {
                Self {
                    responses,
                    index: 0,
                }
            }
        }

        impl DecisionMaker for TestResponseDecisionMaker {
            fn decide_priority(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::PriorityContext,
            ) -> LegalAction {
                if self.index < self.responses.len()
                    && let PriorityResponse::PriorityAction(action) = &self.responses[self.index]
                {
                    self.index += 1;
                    return action.clone();
                }
                ctx.legal_actions
                    .iter()
                    .find(|a| matches!(a, LegalAction::PassPriority))
                    .cloned()
                    .unwrap_or_else(|| ctx.legal_actions[0].clone())
            }

            fn decide_number(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                if self.index < self.responses.len() {
                    if let PriorityResponse::XValue(x) = self.responses[self.index] {
                        self.index += 1;
                        return x;
                    }
                    if let PriorityResponse::NumberChoice(n) = self.responses[self.index] {
                        self.index += 1;
                        return n;
                    }
                }
                ctx.min
            }

            fn decide_targets(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::TargetsContext,
            ) -> Vec<Target> {
                if self.index < self.responses.len()
                    && let PriorityResponse::Targets(targets) = &self.responses[self.index]
                {
                    self.index += 1;
                    return targets.clone();
                }
                ctx.requirements
                    .iter()
                    .filter(|r| r.min_targets > 0)
                    .filter_map(|r| r.legal_targets.first().cloned())
                    .collect()
            }
        }

        // Create a game with 2 players at 3 life
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 3);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up for main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create 4 Mountains on Alice's battlefield (Fireball with X=3 costs {3}{R} = 4 mana)
        let mountain_def = basic_mountain();
        let mut mountain_ids = Vec::new();
        for _ in 0..4 {
            let mountain_id =
                game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
            game.remove_summoning_sickness(mountain_id);
            mountain_ids.push(mountain_id);
        }

        // Create Fireball in Alice's hand
        let fireball_def = fireball();
        let fireball_id = game.create_object_from_definition(&fireball_def, alice, Zone::Hand);

        // Find mana ability index (same for all mountains)
        let mana_ability_index = game
            .object(mountain_ids[0])
            .unwrap()
            .abilities
            .iter()
            .position(|a| a.is_mana_ability())
            .expect("Mountain should have a mana ability");

        // Create scripted responses:
        // 1-4. Alice activates mana ability on each mountain (adds 4R to pool)
        // 5. Alice casts Fireball (prompts for X value since it has X in cost)
        // 6. Alice chooses X=3 (deals 3 damage)
        // 7. Alice selects Bob as target
        // 8. Bob passes priority
        // 9. Alice passes priority (spell resolves, dealing 3 damage to Bob)
        let mut responses = Vec::new();

        // Tap all 4 mountains for mana
        for &mountain_id in &mountain_ids {
            responses.push(PriorityResponse::PriorityAction(
                LegalAction::ActivateManaAbility {
                    source: mountain_id,
                    ability_index: mana_ability_index,
                },
            ));
        }

        // Cast Fireball
        responses.push(PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: fireball_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }));

        // Choose X=3 (after CastSpell triggers ChooseXValue decision)
        responses.push(PriorityResponse::XValue(3));

        // Choose Bob as target (after X value triggers ChooseTargets decision)
        responses.push(PriorityResponse::Targets(vec![Target::Player(bob)]));

        // Both players pass priority
        responses.push(PriorityResponse::PriorityAction(LegalAction::PassPriority)); // Bob passes
        responses.push(PriorityResponse::PriorityAction(LegalAction::PassPriority)); // Alice passes

        let mut decision_maker = TestResponseDecisionMaker::new(responses);
        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());

        // Run the decision-based priority loop
        let mut iterations = 0;
        loop {
            iterations += 1;
            if iterations > 20 {
                panic!("Too many iterations - decision loop may be stuck");
            }

            // Advance to get next decision
            let progress = advance_priority(&mut game, &mut trigger_queue)
                .expect("advance_priority should not fail");

            // Helper closure to handle a decision and any nested decisions
            let handle_result = |mut result: GameProgress,
                                 game: &mut GameState,
                                 trigger_queue: &mut TriggerQueue,
                                 state: &mut PriorityLoopState,
                                 dm: &mut TestResponseDecisionMaker|
             -> Option<GameProgress> {
                loop {
                    match result {
                        GameProgress::Continue => return Some(GameProgress::Continue),
                        GameProgress::GameOver(r) => return Some(GameProgress::GameOver(r)),
                        GameProgress::StackResolved => return Some(GameProgress::StackResolved),
                        GameProgress::NeedsDecisionCtx(ctx) => {
                            result = apply_decision_context_with_dm(
                                game,
                                trigger_queue,
                                state,
                                &ctx,
                                dm,
                            )
                            .expect("apply_decision_context_with_dm should not fail");
                        }
                    }
                }
            };

            match progress {
                GameProgress::NeedsDecisionCtx(ctx) => {
                    // Apply the response
                    let result = apply_decision_context_with_dm(
                        &mut game,
                        &mut trigger_queue,
                        &mut state,
                        &ctx,
                        &mut decision_maker,
                    )
                    .expect("apply_decision_context_with_dm should not fail");

                    // Handle any nested decisions
                    if let Some(final_result) = handle_result(
                        result,
                        &mut game,
                        &mut trigger_queue,
                        &mut state,
                        &mut decision_maker,
                    ) {
                        match final_result {
                            GameProgress::GameOver(r) => {
                                assert!(
                                    matches!(r, GameResult::Winner(winner) if winner == alice),
                                    "Alice should win (Bob at 0 life)"
                                );
                                break;
                            }
                            GameProgress::Continue => break,
                            GameProgress::StackResolved => {} // Continue outer loop
                            _ => {}
                        }
                    }
                }
                GameProgress::Continue => {
                    // Phase ended - in a full game we'd continue, but for this test we're done
                    break;
                }
                GameProgress::GameOver(result) => {
                    // Game ended
                    assert!(
                        matches!(result, GameResult::Winner(winner) if winner == alice),
                        "Alice should win (Bob at 0 life)"
                    );
                    break;
                }
                GameProgress::StackResolved => {
                    // Stack resolved, continue loop to re-advance priority
                }
            }
        }

        // Verify final state
        assert_eq!(game.player(bob).unwrap().life, 0, "Bob should be at 0 life");
        assert!(
            game.player(bob).unwrap().has_lost,
            "Bob should have lost the game"
        );
    }

    // ============================================================================
    // Card-Specific Integration Tests
    // ============================================================================

    #[test]
    fn test_darksteel_colossus_shuffle_into_library() {
        use crate::cards::definitions::darksteel_colossus;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Darksteel Colossus on battlefield
        let colossus_def = darksteel_colossus();
        let colossus_id =
            game.create_object_from_definition(&colossus_def, alice, Zone::Battlefield);

        // Verify it has the ShuffleIntoLibraryFromGraveyard ability
        let colossus = game.object(colossus_id).unwrap();
        let has_ability = colossus.abilities.iter().any(|a| {
            if let crate::ability::AbilityKind::Static(s) = &a.kind {
                s.id() == crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
            } else {
                false
            }
        });
        assert!(
            has_ability,
            "Darksteel Colossus should have ShuffleIntoLibraryFromGraveyard"
        );

        // Record initial library size
        let _initial_library_size = game.player(alice).unwrap().library.len();

        // Verify it's on battlefield
        assert!(game.battlefield.contains(&colossus_id));
        assert_eq!(game.object(colossus_id).unwrap().zone, Zone::Battlefield);

        // Note: The actual zone change interception would happen in move_object
        // This test verifies the ability is present; full behavior would require
        // implementing the replacement effect handling in game_state.rs
    }

    #[test]
    fn test_thorn_elemental_has_ability() {
        use crate::cards::definitions::thorn_elemental;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Thorn Elemental on battlefield
        let thorn_def = thorn_elemental();
        let thorn_id = game.create_object_from_definition(&thorn_def, alice, Zone::Battlefield);

        // Verify it has trample
        let thorn = game.object(thorn_id).unwrap();
        let has_trample = thorn.abilities.iter().any(|a| {
            if let crate::ability::AbilityKind::Static(s) = &a.kind {
                s.has_trample()
            } else {
                false
            }
        });
        assert!(has_trample, "Thorn Elemental should have trample");

        // Verify it has MayAssignDamageAsUnblocked
        let has_unblocked_ability = thorn.abilities.iter().any(|a| {
            if let crate::ability::AbilityKind::Static(s) = &a.kind {
                s.id() == crate::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked
            } else {
                false
            }
        });
        assert!(
            has_unblocked_ability,
            "Thorn Elemental should have MayAssignDamageAsUnblocked"
        );
    }

    #[test]
    fn test_thorn_elemental_combat_decision() {
        use crate::cards::definitions::thorn_elemental;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Thorn Elemental on battlefield
        let thorn_def = thorn_elemental();
        let thorn_id = game.create_object_from_definition(&thorn_def, alice, Zone::Battlefield);

        // Create a blocker
        let blocker_id = create_creature(&mut game, "Blocker", bob, 2, 2);

        // Remove summoning sickness
        game.remove_summoning_sickness(thorn_id);

        // Set up combat: Thorn Elemental attacks Bob, Blocker blocks
        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: thorn_id,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(thorn_id, vec![blocker_id]);

        // Verify the thorn elemental has the ability that would trigger the decision
        let thorn = game.object(thorn_id).unwrap();
        let has_ability = thorn.abilities.iter().any(|a| {
            if let crate::ability::AbilityKind::Static(s) = &a.kind {
                s.id() == crate::static_abilities::StaticAbilityId::MayAssignDamageAsUnblocked
            } else {
                false
            }
        });
        assert!(has_ability);

        // Without the decision (normal combat), damage goes to blocker
        // With trample, Thorn Elemental deals 7 damage: 2 to blocker (lethal), 5 to Bob
        let events = execute_combat_damage_step(&mut game, &combat, false);

        // Verify damage was dealt (trample behavior)
        assert!(!events.is_empty());
        // Blocker takes lethal damage (2)
        assert_eq!(game.damage_on(blocker_id), 2);
        // Bob takes trample damage (7 - 2 = 5)
        assert_eq!(game.player(bob).unwrap().life, 15);
    }

    #[test]
    fn test_stormbreath_dragon_has_abilities() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::stormbreath_dragon;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        let dragon = game.object(dragon_id).unwrap();

        // Verify flying
        let has_flying = dragon.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_flying()
            } else {
                false
            }
        });
        assert!(has_flying, "Stormbreath Dragon should have flying");

        // Verify haste
        let has_haste = dragon.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_haste()
            } else {
                false
            }
        });
        assert!(has_haste, "Stormbreath Dragon should have haste");

        // Verify protection from white
        let has_protection = dragon.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_protection()
            } else {
                false
            }
        });
        assert!(
            has_protection,
            "Stormbreath Dragon should have protection from white"
        );

        // Verify activated ability (monstrosity)
        let has_activated = dragon
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Activated(_)));
        assert!(
            has_activated,
            "Stormbreath Dragon should have monstrosity activated ability"
        );

        // Verify triggered ability (when becomes monstrous)
        let has_triggered = dragon
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Triggered(_)));
        assert!(
            has_triggered,
            "Stormbreath Dragon should have 'becomes monstrous' trigger"
        );
    }

    #[test]
    fn test_stormbreath_dragon_is_monstrous_field() {
        use crate::cards::definitions::stormbreath_dragon;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Verify is_monstrous starts false
        assert!(
            !game.is_monstrous(dragon_id),
            "Dragon should not be monstrous initially"
        );

        // Manually set monstrous (simulating effect execution)
        game.set_monstrous(dragon_id);

        // Verify it's now monstrous
        assert!(
            game.is_monstrous(dragon_id),
            "Dragon should be monstrous after being set"
        );
    }

    #[test]
    fn test_stormbreath_dragon_trigger_condition() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::stormbreath_dragon;
        use crate::events::other::BecameMonstrousEvent;
        use crate::triggers::check_triggers;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Verify the trigger condition is ThisBecomesMonstrous
        let dragon = game.object(dragon_id).unwrap();
        let has_monstrous_trigger = dragon.abilities.iter().any(|a| {
            if let AbilityKind::Triggered(triggered) = &a.kind {
                triggered.trigger.display().contains("monstrous")
            } else {
                false
            }
        });
        assert!(
            has_monstrous_trigger,
            "Stormbreath Dragon should have ThisBecomesMonstrous trigger"
        );

        // Simulate the BecameMonstrous event
        let event = TriggerEvent::new(BecameMonstrousEvent::new(dragon_id, alice, 3));

        // Check if triggers fire
        let triggers = check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            1,
            "BecameMonstrous should trigger Stormbreath Dragon's ability"
        );
    }

    #[test]
    fn test_geist_of_saint_traft_has_abilities() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::geist_of_saint_traft;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Geist on battlefield
        let geist_def = geist_of_saint_traft();
        let geist_id = game.create_object_from_definition(&geist_def, alice, Zone::Battlefield);

        let geist = game.object(geist_id).unwrap();

        // Verify hexproof
        let has_hexproof = geist.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_hexproof()
            } else {
                false
            }
        });
        assert!(has_hexproof, "Geist should have hexproof");

        // Verify attack trigger
        let has_attack_trigger = geist.abilities.iter().any(|a| {
            if let AbilityKind::Triggered(triggered) = &a.kind {
                triggered.trigger.display().contains("attacks")
            } else {
                false
            }
        });
        assert!(
            has_attack_trigger,
            "Geist should have 'when this attacks' trigger"
        );
    }

    #[test]
    fn test_geist_of_saint_traft_attack_trigger() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::geist_of_saint_traft;
        use crate::triggers::{AttackEventTarget, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Geist on battlefield
        let geist_def = geist_of_saint_traft();
        let geist_id = game.create_object_from_definition(&geist_def, alice, Zone::Battlefield);

        // Remove summoning sickness
        game.remove_summoning_sickness(geist_id);

        // Simulate the attack event
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            geist_id,
            AttackEventTarget::Player(bob),
        ));

        // Check if triggers fire
        let triggers = check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            1,
            "Attacking with Geist should trigger its ability"
        );

        // Verify the trigger creates a token with modifications
        let geist = game.object(geist_id).unwrap();
        let trigger = geist.abilities.iter().find(|a| {
            if let AbilityKind::Triggered(triggered) = &a.kind {
                triggered.trigger.display().contains("attacks")
            } else {
                false
            }
        });
        assert!(trigger.is_some());

        if let Some(ability) = trigger {
            if let AbilityKind::Triggered(triggered) = &ability.kind {
                // Verify the effect creates a token
                assert!(!triggered.effects.is_empty());
                let has_token_effect = triggered
                    .effects
                    .iter()
                    .any(|e| format!("{:?}", e).contains("CreateToken"));
                assert!(
                    has_token_effect,
                    "Geist's trigger should create a token with modifications"
                );
            }
        }
    }

    #[test]
    fn test_geist_token_has_correct_modifications() {
        use crate::ability::AbilityKind;
        use crate::cards::definitions::geist_of_saint_traft;

        let geist_def = geist_of_saint_traft();

        // Find the triggered ability
        let trigger = geist_def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)));
        assert!(trigger.is_some());

        if let Some(ability) = trigger {
            if let AbilityKind::Triggered(triggered) = &ability.kind {
                // Find the token creation effect
                let token_effect = triggered
                    .effects
                    .iter()
                    .find(|e| format!("{:?}", e).contains("CreateToken"));
                assert!(
                    token_effect.is_some(),
                    "Should have a token creation effect"
                );

                // The actual token properties are tested via integration tests
                // that create the token and verify its characteristics
            }
        }
    }

    #[test]
    fn test_stormbreath_dragon_monstrosity_adds_counters() {
        use crate::cards::definitions::stormbreath_dragon;
        use crate::effect::{Effect, EffectResult};
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Verify initial state: not monstrous, no +1/+1 counters
        assert!(!game.is_monstrous(dragon_id));
        {
            let dragon = game.object(dragon_id).unwrap();
            assert_eq!(dragon.counters.get(&CounterType::PlusOnePlusOne), None);
            assert_eq!(dragon.power(), Some(4));
            assert_eq!(dragon.toughness(), Some(4));
        }

        // Execute the Monstrosity 3 effect
        let mut ctx = ExecutionContext::new_default(dragon_id, alice);
        let effect = Effect::monstrosity(3);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Verify result indicates monstrosity was applied
        assert!(matches!(
            result.result,
            EffectResult::MonstrosityApplied { creature, n } if creature == dragon_id && n == 3
        ));

        // Verify dragon is now monstrous with 3 +1/+1 counters
        assert!(game.is_monstrous(dragon_id), "Dragon should be monstrous");
        let dragon = game.object(dragon_id).unwrap();
        assert_eq!(
            dragon.counters.get(&CounterType::PlusOnePlusOne),
            Some(&3),
            "Dragon should have 3 +1/+1 counters"
        );
        // 4/4 + 3 counters = 7/7
        assert_eq!(dragon.power(), Some(7));
        assert_eq!(dragon.toughness(), Some(7));
    }

    #[test]
    fn test_stormbreath_dragon_monstrosity_only_works_once() {
        use crate::cards::definitions::stormbreath_dragon;
        use crate::effect::{Effect, EffectResult};
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Execute monstrosity once
        let mut ctx = ExecutionContext::new_default(dragon_id, alice);
        let effect = Effect::monstrosity(3);
        execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Verify it worked
        assert!(game.is_monstrous(dragon_id));
        assert_eq!(
            game.object(dragon_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&3)
        );

        // Try to execute monstrosity again
        let mut ctx2 = ExecutionContext::new_default(dragon_id, alice);
        let result = execute_effect(&mut game, &effect, &mut ctx2).unwrap();

        // Should return Count(0) - nothing happened
        assert_eq!(
            result.result,
            EffectResult::Count(0),
            "Second monstrosity should do nothing"
        );

        // Counters should still be 3 (not 6)
        assert_eq!(
            game.object(dragon_id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&3),
            "Counters should not have increased"
        );
    }

    #[test]
    fn test_stormbreath_dragon_becomes_monstrous_trigger_fires() {
        use crate::cards::definitions::stormbreath_dragon;
        use crate::effect::Effect;
        use crate::events::other::BecameMonstrousEvent;
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::triggers::check_triggers;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Stormbreath Dragon on battlefield
        let dragon_def = stormbreath_dragon();
        let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

        // Execute monstrosity
        let mut ctx = ExecutionContext::new_default(dragon_id, alice);
        let effect = Effect::monstrosity(3);
        execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Now simulate the BecameMonstrous event (which would be generated by the game loop)
        let event = TriggerEvent::new(BecameMonstrousEvent::new(dragon_id, alice, 3));

        // Check if the dragon's "becomes monstrous" trigger fires
        let triggers = check_triggers(&game, &event);

        assert_eq!(
            triggers.len(),
            1,
            "Stormbreath Dragon's 'becomes monstrous' trigger should fire"
        );

        // Verify the trigger is from the dragon
        assert_eq!(triggers[0].source, dragon_id);
        assert_eq!(triggers[0].controller, alice);
    }

    // =========================================================================
    // Integration Tests for New Features
    // =========================================================================

    #[test]
    fn test_once_per_turn_ability_tracking() {
        // Test that OncePerTurn abilities can only be activated once per turn
        use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a permanent with a OncePerTurn activated ability
        let creature_id = create_creature(&mut game, "Test Creature", alice, 2, 2);

        // Add a OncePerTurn activated ability (e.g., "{T}: Draw a card")
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: crate::ability::merge_cost_effects(
                        TotalCost::free(),
                        vec![Effect::tap_source()],
                    ),
                    effects: vec![Effect::draw(1)],
                    choices: vec![],
                    timing: ActivationTiming::OncePerTurn,
                    additional_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: None,
            });

        // Remove summoning sickness
        game.remove_summoning_sickness(creature_id);

        // Verify the ability hasn't been activated this turn
        assert!(!game.ability_activated_this_turn(creature_id, 0));

        // Record the activation
        game.record_ability_activation(creature_id, 0);

        // Verify the ability is now tracked as activated
        assert!(game.ability_activated_this_turn(creature_id, 0));

        // Simulate next turn - tracking should be cleared
        game.next_turn();
        assert!(!game.ability_activated_this_turn(creature_id, 0));
    }

    #[test]
    fn test_protection_from_permanents_blocking() {
        use crate::ability::ProtectionFrom;
        use crate::rules::combat::can_block;
        use crate::target::ObjectFilter;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create attacker with protection from green creatures
        let attacker_id = create_creature(&mut game, "Protected Attacker", alice, 2, 2);
        let green_filter = ObjectFilter {
            colors: Some(crate::color::ColorSet::GREEN),
            card_types: vec![CardType::Creature],
            ..Default::default()
        };
        game.object_mut(attacker_id)
            .unwrap()
            .abilities
            .push(Ability::static_ability(
                crate::static_abilities::StaticAbility::protection(ProtectionFrom::Permanents(
                    green_filter,
                )),
            ));

        // Create a green creature blocker
        let green_blocker_id = create_creature(&mut game, "Green Blocker", bob, 2, 2);
        game.object_mut(green_blocker_id).unwrap().color_override =
            Some(crate::color::ColorSet::GREEN);

        // Create a red creature blocker
        let red_blocker_id = create_creature(&mut game, "Red Blocker", bob, 2, 2);
        game.object_mut(red_blocker_id).unwrap().color_override = Some(crate::color::ColorSet::RED);

        let attacker = game.object(attacker_id).unwrap();
        let green_blocker = game.object(green_blocker_id).unwrap();
        let red_blocker = game.object(red_blocker_id).unwrap();

        // Green creature should NOT be able to block (protection)
        assert!(
            !can_block(attacker, green_blocker, &game),
            "Green creature should not be able to block creature with protection from green creatures"
        );

        // Red creature SHOULD be able to block
        assert!(
            can_block(attacker, red_blocker, &game),
            "Red creature should be able to block creature with protection from green creatures"
        );
    }

    #[test]
    fn test_cleanup_discard_decision() {
        use crate::turn::{apply_cleanup_discard, get_cleanup_discard_spec};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;

        // Add 9 cards to hand (2 over max hand size of 7)
        for i in 0..9 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, alice, Zone::Hand);
        }

        assert_eq!(game.player(alice).unwrap().hand.len(), 9);

        // Get the discard spec
        let result = get_cleanup_discard_spec(&game);
        assert!(result.is_some());

        let (player, spec) = result.unwrap();
        assert_eq!(player, alice);
        assert_eq!(spec.count, 2);
        assert_eq!(spec.hand.len(), 9);

        // Simulate player choosing specific cards to discard
        let cards_to_discard = vec![spec.hand[0], spec.hand[1]];
        let mut dm = crate::decision::AutoPassDecisionMaker;
        apply_cleanup_discard(&mut game, &cards_to_discard, &mut dm);

        // Verify hand size is now 7
        assert_eq!(game.player(alice).unwrap().hand.len(), 7);
        // Verify graveyard has 2 cards
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
    }

    #[test]
    fn test_legend_rule_decision() {
        use crate::rules::state_based::{apply_legend_rule_choice, get_legend_rule_specs};
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two legendary creatures with the same name
        let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let legend1_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let _legend2_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);

        // Get legend rule specs
        let specs = get_legend_rule_specs(&game);
        assert_eq!(specs.len(), 1, "Should have one legend rule spec");

        let (player, spec) = &specs[0];
        assert_eq!(*player, alice);
        assert_eq!(spec.name, "Isamaru, Hound of Konda");
        assert_eq!(spec.legends.len(), 2);

        // Player chooses to keep the first legend
        apply_legend_rule_choice(&mut game, legend1_id);

        // Verify only one legend remains on battlefield
        assert_eq!(game.battlefield.len(), 1);
        assert!(game.battlefield.contains(&legend1_id));

        // The second legend should be in graveyard (with new ID due to zone change)
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    }

    #[test]
    fn test_may_effect_with_callback() {
        use crate::decision::DecisionMaker;
        use crate::effect::EffectResult;
        use crate::executor::ExecutionContext;

        // A decision maker that always accepts May effects
        struct AcceptMayDecisionMaker;
        impl DecisionMaker for AcceptMayDecisionMaker {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }

        // A decision maker that always declines May effects
        struct DeclineMayDecisionMaker;
        impl DecisionMaker for DeclineMayDecisionMaker {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                false
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add some cards to library so draw can succeed
        for i in 0..3 {
            let card = CardBuilder::new(CardId::new(), &format!("Library Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let source_id = create_creature(&mut game, "Source", alice, 2, 2);
        let initial_hand_size = game.player(alice).unwrap().hand.len();

        let effect = Effect::may_single(Effect::draw(1));

        // Test 1: May effect with decision maker that accepts
        let mut accept_dm = AcceptMayDecisionMaker;
        let mut ctx =
            ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut accept_dm);

        let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

        // Effect should have been executed (not declined)
        assert!(
            !matches!(result.result, EffectResult::Declined),
            "Effect should not be declined when decision maker accepts"
        );
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            initial_hand_size + 1,
            "Should have drawn a card"
        );

        // Test 2: May effect with decision maker that declines
        let mut decline_dm = DeclineMayDecisionMaker;
        let mut ctx2 =
            ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut decline_dm);

        let result2 = execute_effect(&mut game, &effect, &mut ctx2).unwrap();

        // Effect should have been declined
        assert!(
            matches!(result2.result, EffectResult::Declined),
            "Effect should be declined when decision maker declines"
        );
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            initial_hand_size + 1,
            "Should NOT have drawn another card"
        );

        // Test 3: May effect with AutoPassDecisionMaker (auto-decline)
        let mut autopass_dm = AutoPassDecisionMaker;
        let mut ctx3 =
            ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut autopass_dm);
        let result3 = execute_effect(&mut game, &effect, &mut ctx3).unwrap();

        assert!(
            matches!(result3.result, EffectResult::Declined),
            "Effect should be auto-declined with AutoPassDecisionMaker"
        );
    }

    #[test]
    fn test_undying_trigger_generation() {
        use crate::ability::TriggeredAbility;
        use crate::events::zones::ZoneChangeEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature with Undying (now a triggered ability)
        let creature_id = create_creature(&mut game, "Undying Creature", alice, 2, 2);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::undying(),
                    effects: undying_effects(),
                    choices: vec![],
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some("Undying".to_string()),
            });

        // Create a snapshot of the creature (no +1/+1 counters)
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        // Verify the snapshot qualifies for undying
        assert!(
            snapshot.qualifies_for_undying(),
            "Creature with Undying and no +1/+1 counters should qualify for undying"
        );

        // Simulate death event
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));

        // Check triggers - should generate an undying trigger
        let triggers = check_triggers(&game, &event);

        assert!(
            triggers
                .iter()
                .any(|t| { t.ability.trigger == Trigger::undying() }),
            "Should generate an undying trigger"
        );
    }

    #[test]
    fn test_undying_does_not_trigger_with_plus_counters() {
        use crate::ability::TriggeredAbility;
        use crate::events::zones::ZoneChangeEvent;
        use crate::object::CounterType;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature with Undying AND +1/+1 counters
        let creature_id = create_creature(&mut game, "Undying Creature", alice, 2, 2);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::undying(),
                    effects: undying_effects(),
                    choices: vec![],
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some("Undying".to_string()),
            });
        game.object_mut(creature_id)
            .unwrap()
            .add_counters(CounterType::PlusOnePlusOne, 1);

        // Create a snapshot
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        // Verify the snapshot does NOT qualify for undying
        assert!(
            !snapshot.qualifies_for_undying(),
            "Creature with +1/+1 counters should NOT qualify for undying"
        );

        // Simulate death event
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));

        // Check triggers - should NOT generate an undying trigger
        let triggers = check_triggers(&game, &event);

        assert!(
            !triggers
                .iter()
                .any(|t| { t.ability.trigger == Trigger::undying() }),
            "Should NOT generate an undying trigger when creature has +1/+1 counters"
        );
    }

    #[test]
    fn test_persist_trigger_generation() {
        use crate::ability::TriggeredAbility;
        use crate::events::zones::ZoneChangeEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature with Persist (now a triggered ability)
        let creature_id = create_creature(&mut game, "Persist Creature", alice, 2, 2);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::persist(),
                    effects: persist_effects(),
                    choices: vec![],
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some("Persist".to_string()),
            });

        // Create a snapshot (no -1/-1 counters)
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        // Verify the snapshot qualifies for persist
        assert!(
            snapshot.qualifies_for_persist(),
            "Creature with Persist and no -1/-1 counters should qualify for persist"
        );

        // Simulate death event
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));

        // Check triggers - should generate a persist trigger
        let triggers = check_triggers(&game, &event);

        assert!(
            triggers
                .iter()
                .any(|t| { t.ability.trigger == Trigger::persist() }),
            "Should generate a persist trigger"
        );
    }

    #[test]
    fn test_return_from_graveyard_with_counter_effect() {
        use crate::events::zones::ZoneChangeEvent;
        use crate::executor::ExecutionContext;
        use crate::snapshot::ObjectSnapshot;
        use crate::triggers::TriggerEvent;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature and put it in the graveyard
        let creature_id = create_creature(&mut game, "Dead Creature", alice, 2, 2);

        // Take snapshot BEFORE moving (captures stable_id)
        let snapshot = ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        game.move_object(creature_id, Zone::Graveyard);

        // The creature now has a new ID in the graveyard
        let graveyard_id = game.player(alice).unwrap().graveyard[0];

        // Create triggering event with the snapshot
        let trigger_event = TriggerEvent::new(ZoneChangeEvent::new(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));

        let mut ctx = ExecutionContext::new_default(graveyard_id, alice);
        ctx.triggering_event = Some(trigger_event);
        for effect in undying_effects() {
            execute_effect(&mut game, &effect, &mut ctx).unwrap();
        }

        // Verify the creature is now on the battlefield
        assert_eq!(
            game.battlefield.len(),
            1,
            "Should have one creature on battlefield"
        );

        // Verify graveyard is empty
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            0,
            "Graveyard should be empty"
        );

        // Verify the creature has a +1/+1 counter
        let returned_id = game.battlefield[0];
        let creature = game.object(returned_id).unwrap();
        assert_eq!(
            creature.counters.get(&CounterType::PlusOnePlusOne),
            Some(&1),
            "Creature should have one +1/+1 counter"
        );
    }

    #[test]
    fn test_once_per_turn_in_legal_actions() {
        use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
        use crate::cost::TotalCost;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up for main phase with priority
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a creature with a OncePerTurn activated ability
        let creature_id = create_creature(&mut game, "Test Creature", alice, 2, 2);
        game.object_mut(creature_id)
            .unwrap()
            .abilities
            .push(Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: crate::ability::merge_cost_effects(TotalCost::free(), Vec::new()), // Free ability for testing
                    effects: vec![Effect::draw(1)],
                    choices: vec![],
                    timing: ActivationTiming::OncePerTurn,
                    additional_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: None,
            });
        game.remove_summoning_sickness(creature_id);

        // Get legal actions - ability should be available
        let actions1 = compute_legal_actions(&game, alice);
        let can_activate1 = actions1.iter().any(|a| {
            matches!(
                a,
                LegalAction::ActivateAbility { source, .. } if *source == creature_id
            )
        });
        assert!(
            can_activate1,
            "OncePerTurn ability should be available initially"
        );

        // Simulate activating the ability
        game.record_ability_activation(creature_id, 0);

        // Get legal actions again - ability should NOT be available
        let actions2 = compute_legal_actions(&game, alice);
        let can_activate2 = actions2.iter().any(|a| {
            matches!(
                a,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == creature_id && *ability_index == 0
            )
        });
        assert!(
            !can_activate2,
            "OncePerTurn ability should NOT be available after activation"
        );
    }

    #[test]
    fn test_cleanup_discard_no_decision_when_under_limit() {
        use crate::turn::get_cleanup_discard_spec;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;

        // Add only 5 cards to hand (under max hand size of 7)
        for i in 0..5 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, alice, Zone::Hand);
        }

        // Get the discard spec - should be None
        let spec = get_cleanup_discard_spec(&game);
        assert!(
            spec.is_none(),
            "Should not require discard when under hand limit"
        );
    }

    #[test]
    fn test_legend_rule_no_decision_when_different_names() {
        use crate::rules::state_based::get_legend_rule_specs;
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two legendary creatures with DIFFERENT names
        let legend1_card = CardBuilder::new(CardId::from_raw(1), "Isamaru")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let legend2_card = CardBuilder::new(CardId::from_raw(2), "Ragavan")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 1))
            .build();

        game.create_object_from_card(&legend1_card, alice, Zone::Battlefield);
        game.create_object_from_card(&legend2_card, alice, Zone::Battlefield);

        // Get legend rule specs - should be empty (different names)
        let specs = get_legend_rule_specs(&game);
        assert!(
            specs.is_empty(),
            "Should not have legend rule specs for different legendary names"
        );
    }

    // ============================================================================
    // Game Loop Integration Tests for Legend Rule and Cleanup Discard
    // ============================================================================

    /// Custom decision maker for testing legend rule choices
    struct LegendRuleDecisionMaker {
        /// Which legend to keep (index into the legends list)
        keep_index: usize,
        /// Record of decisions made
        decisions_made: Vec<String>,
    }

    impl LegendRuleDecisionMaker {
        fn new(keep_index: usize) -> Self {
            Self {
                keep_index,
                decisions_made: Vec::new(),
            }
        }
    }

    impl crate::decision::DecisionMaker for LegendRuleDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            // Record that a legend rule decision was made
            self.decisions_made.push(format!(
                "Legend rule for '{}' with {} options",
                ctx.description,
                ctx.candidates.len()
            ));
            // Return the legend to keep based on index
            let legal_candidates: Vec<ObjectId> = ctx
                .candidates
                .iter()
                .filter(|c| c.legal)
                .map(|c| c.id)
                .collect();
            let keep_id = legal_candidates
                .get(
                    self.keep_index
                        .min(legal_candidates.len().saturating_sub(1)),
                )
                .copied()
                .unwrap_or_else(|| ctx.candidates[0].id);
            vec![keep_id]
        }
    }

    #[test]
    fn test_legend_rule_via_game_loop() {
        use crate::triggers::TriggerQueue;
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two legendary creatures with the same name
        let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let legend1_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let legend2_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);

        // Verify both are on battlefield
        assert_eq!(game.battlefield.len(), 2);

        // Create a decision maker that chooses the SECOND legend to keep
        let mut dm = LegendRuleDecisionMaker::new(1);
        let mut trigger_queue = TriggerQueue::new();

        // Run SBAs through the game loop - this should prompt for legend rule choice
        let result = check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm);
        assert!(result.is_ok());

        // Verify the decision was made
        assert_eq!(dm.decisions_made.len(), 1);
        assert!(dm.decisions_made[0].contains("Isamaru"));

        // Verify only one legend remains on battlefield
        assert_eq!(
            game.battlefield.len(),
            1,
            "Should have one legend remaining"
        );

        // The SECOND legend should be the one kept (since we chose index 1)
        assert!(
            game.battlefield.contains(&legend2_id),
            "Second legend should be kept"
        );
        assert!(
            !game.battlefield.contains(&legend1_id),
            "First legend should be gone"
        );

        // First legend should be in graveyard
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            1,
            "One legend should be in graveyard"
        );
    }

    #[test]
    fn test_legend_rule_keeps_first_legend() {
        use crate::triggers::TriggerQueue;
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create three legendary creatures with the same name
        let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let legend1_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let _legend2_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let _legend3_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);

        // Verify all three are on battlefield
        assert_eq!(game.battlefield.len(), 3);

        // Create a decision maker that chooses the FIRST legend to keep
        let mut dm = LegendRuleDecisionMaker::new(0);
        let mut trigger_queue = TriggerQueue::new();

        // Run SBAs through the game loop
        let result = check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm);
        assert!(result.is_ok());

        // Verify only one legend remains on battlefield
        assert_eq!(
            game.battlefield.len(),
            1,
            "Should have one legend remaining"
        );

        // The FIRST legend should be the one kept
        assert!(
            game.battlefield.contains(&legend1_id),
            "First legend should be kept"
        );

        // Two legends should be in graveyard
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            2,
            "Two legends should be in graveyard"
        );
    }

    /// Custom decision maker for testing cleanup discard choices
    struct CleanupDiscardDecisionMaker {
        /// Which card indices to discard (from the hand list)
        discard_indices: Vec<usize>,
        /// Record of decisions made
        decisions_made: Vec<String>,
    }

    impl CleanupDiscardDecisionMaker {
        fn new(discard_indices: Vec<usize>) -> Self {
            Self {
                discard_indices,
                decisions_made: Vec::new(),
            }
        }
    }

    impl crate::decision::DecisionMaker for CleanupDiscardDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.decisions_made.push(format!(
                "Discard {} cards from hand of {}",
                ctx.min,
                ctx.candidates.len()
            ));
            // Select cards at the specified indices
            self.discard_indices
                .iter()
                .filter_map(|&idx| ctx.candidates.get(idx).map(|c| c.id))
                .take(ctx.min)
                .collect()
        }

        fn decide_priority(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::PriorityContext,
        ) -> LegalAction {
            LegalAction::PassPriority
        }
    }

    #[test]
    fn test_cleanup_discard_via_game_loop() {
        use crate::decisions::make_decision;
        use crate::turn::get_cleanup_discard_spec;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;

        // Add 10 cards to hand (3 over max hand size of 7)
        let mut card_ids = Vec::new();
        for i in 0..10 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            let obj_id = game.create_object_from_card(&card, alice, Zone::Hand);
            card_ids.push(obj_id);
        }

        assert_eq!(game.player(alice).unwrap().hand.len(), 10);

        // Create a decision maker that discards the first 3 cards
        let mut dm = CleanupDiscardDecisionMaker::new(vec![0, 1, 2]);

        // Manually run cleanup discard decision flow
        if let Some((player, spec)) = get_cleanup_discard_spec(&game) {
            let cards: Vec<ObjectId> = make_decision(&game, &mut dm, player, None, spec);
            let mut auto_dm = crate::decision::AutoPassDecisionMaker;
            crate::turn::apply_cleanup_discard(&mut game, &cards, &mut auto_dm);
        }

        // Verify the decision was made
        assert_eq!(dm.decisions_made.len(), 1);
        assert!(dm.decisions_made[0].contains("Discard 3 cards"));

        // Verify hand size is now 7
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            7,
            "Hand should have 7 cards after discard"
        );

        // Verify graveyard has 3 cards
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            3,
            "Graveyard should have 3 discarded cards"
        );

        // Verify the specific cards that were discarded (first 3)
        let graveyard = &game.player(alice).unwrap().graveyard;
        // The cards get new IDs when moving zones, so we check by name
        let discarded_names: Vec<String> = graveyard
            .iter()
            .filter_map(|id| game.object(*id).map(|o| o.name.clone()))
            .collect();

        // Cards 0, 1, 2 should be in graveyard
        assert!(
            discarded_names.contains(&"Card 0".to_string()),
            "Card 0 should be in graveyard"
        );
        assert!(
            discarded_names.contains(&"Card 1".to_string()),
            "Card 1 should be in graveyard"
        );
        assert!(
            discarded_names.contains(&"Card 2".to_string()),
            "Card 2 should be in graveyard"
        );
        let _ = card_ids; // Suppress unused warning
    }

    #[test]
    fn test_cleanup_discard_specific_card_choice() {
        use crate::decisions::make_decision;
        use crate::turn::get_cleanup_discard_spec;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;

        // Add 9 cards to hand (2 over max hand size of 7)
        for i in 0..9 {
            let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, alice, Zone::Hand);
        }

        let initial_hand = game.player(alice).unwrap().hand.clone();
        assert_eq!(initial_hand.len(), 9);

        // Get the names of cards at indices 3 and 7 (the ones we'll discard)
        let card_3_name = game.object(initial_hand[3]).unwrap().name.clone();
        let card_7_name = game.object(initial_hand[7]).unwrap().name.clone();

        // Create a decision maker that discards cards at indices 3 and 7
        let mut dm = CleanupDiscardDecisionMaker::new(vec![3, 7]);

        // Run cleanup discard decision flow
        if let Some((player, spec)) = get_cleanup_discard_spec(&game) {
            let cards: Vec<ObjectId> = make_decision(&game, &mut dm, player, None, spec);
            let mut auto_dm = crate::decision::AutoPassDecisionMaker;
            crate::turn::apply_cleanup_discard(&mut game, &cards, &mut auto_dm);
        }

        // Verify hand size is now 7
        assert_eq!(game.player(alice).unwrap().hand.len(), 7);

        // Verify the correct cards were discarded by checking names in graveyard
        let graveyard_names: Vec<String> = game
            .player(alice)
            .unwrap()
            .graveyard
            .iter()
            .filter_map(|id| game.object(*id).map(|o| o.name.clone()))
            .collect();

        assert!(
            graveyard_names.contains(&card_3_name),
            "Card at index 3 ({}) should be in graveyard",
            card_3_name
        );
        assert!(
            graveyard_names.contains(&card_7_name),
            "Card at index 7 ({}) should be in graveyard",
            card_7_name
        );

        // Verify those cards are NOT in hand anymore
        let hand_names: Vec<String> = game
            .player(alice)
            .unwrap()
            .hand
            .iter()
            .filter_map(|id| game.object(*id).map(|o| o.name.clone()))
            .collect();

        assert!(
            !hand_names.contains(&card_3_name),
            "Card at index 3 should NOT be in hand"
        );
        assert!(
            !hand_names.contains(&card_7_name),
            "Card at index 7 should NOT be in hand"
        );
    }

    #[test]
    fn test_legend_rule_with_different_controllers() {
        use crate::triggers::TriggerQueue;
        use crate::types::Supertype;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create the same legendary creature for two different players
        let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let alice_legend = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
        let bob_legend = game.create_object_from_card(&legend_card, bob, Zone::Battlefield);

        // Verify both are on battlefield
        assert_eq!(game.battlefield.len(), 2);

        // Create a decision maker
        let mut dm = LegendRuleDecisionMaker::new(0);
        let mut trigger_queue = TriggerQueue::new();

        // Run SBAs - legend rule should NOT apply because they have different controllers
        let result = check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm);
        assert!(result.is_ok());

        // No legend rule decisions should have been made
        assert_eq!(
            dm.decisions_made.len(),
            0,
            "No legend rule decisions for different controllers"
        );

        // Both legends should still be on battlefield
        assert_eq!(game.battlefield.len(), 2);
        assert!(game.battlefield.contains(&alice_legend));
        assert!(game.battlefield.contains(&bob_legend));
    }

    // ============================================================================
    // Flashback Tests
    // ============================================================================

    #[test]
    fn test_flashback_appears_in_legal_actions_from_graveyard() {
        use crate::cards::definitions::think_twice;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase for sorcery-timing spells
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add 3 blue mana directly (for flashback cost {2}{U})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);

        // Create Think Twice IN GRAVEYARD
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find a CastSpell action for Think Twice with Alternative casting method
        let flashback_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            flashback_action.is_some(),
            "Should be able to cast Think Twice with flashback from graveyard"
        );
    }

    #[test]
    fn test_flashback_not_available_from_hand() {
        use crate::cards::definitions::think_twice;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add 3 blue mana directly
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);

        // Create Think Twice IN HAND
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Hand);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find a CastSpell action for Think Twice from hand with Normal casting
        let normal_cast = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            normal_cast.is_some(),
            "Should be able to cast Think Twice normally from hand"
        );

        // Should NOT find flashback from hand
        let flashback_from_hand = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(_),
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            flashback_from_hand.is_none(),
            "Should NOT be able to use flashback from hand"
        );
    }

    #[test]
    fn test_flashback_exiles_after_resolution() {
        use crate::cards::definitions::think_twice;
        use crate::mana::ManaSymbol;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add 3 blue mana directly
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);

        // Add a card to alice's library so draw can succeed
        use crate::cards::definitions::basic_island;
        let island_def = basic_island();
        let _library_card = game.create_object_from_definition(&island_def, alice, Zone::Library);

        // Create Think Twice in graveyard
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

        // Record initial hand size
        let initial_hand_size = game.player(alice).unwrap().hand.len();

        // Cast with flashback
        let mut state = PriorityLoopState::new(2);
        let mut trigger_queue = TriggerQueue::new();

        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: think_twice_id,
            from_zone: Zone::Graveyard,
            casting_method: CastingMethod::Alternative(0),
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
        assert!(result.is_ok(), "Casting with flashback should succeed");

        // Spell should be on stack now
        assert_eq!(game.stack.len(), 1, "Spell should be on stack");
        let stack_entry = &game.stack[0];
        assert_eq!(
            stack_entry.casting_method,
            CastingMethod::Alternative(0),
            "Stack entry should record flashback casting method"
        );

        // Resolve the spell
        resolve_stack_entry(&mut game).expect("Resolution should succeed");

        // Verify draw happened
        let final_hand_size = game.player(alice).unwrap().hand.len();
        assert_eq!(
            final_hand_size,
            initial_hand_size + 1,
            "Should have drawn 1 card"
        );

        // Verify spell is in exile (not graveyard)
        let player = game.player(alice).unwrap();
        let in_graveyard = player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(
            !in_graveyard,
            "Think Twice should NOT be in graveyard after flashback"
        );

        let in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(in_exile, "Think Twice SHOULD be in exile after flashback");
    }

    #[test]
    fn test_rebound_exiles_on_resolution_and_schedules_next_upkeep_cast() {
        use crate::ability::Ability;
        use crate::cards::CardDefinition;
        use crate::effect::Effect;
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::static_abilities::StaticAbility;
        use crate::triggers::TriggerQueue;
        use crate::types::CardType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        let card = crate::card::CardBuilder::new(CardId::new(), "Rebound Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Instant])
            .build();
        let mut definition = CardDefinition::new(card);
        definition.spell_effect = Some(vec![Effect::gain_life(1)]);
        definition.abilities.push(
            Ability::static_ability(StaticAbility::rebound())
                .in_zones(vec![Zone::Stack])
                .with_text("Rebound"),
        );

        let rebound_id = game.create_object_from_definition(&definition, alice, Zone::Hand);

        let mut state = PriorityLoopState::new(2);
        let mut trigger_queue = TriggerQueue::new();

        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: rebound_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        });
        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
        assert!(result.is_ok(), "normal cast with rebound should succeed");

        assert_eq!(game.stack.len(), 1, "spell should be on stack");
        resolve_stack_entry(&mut game).expect("rebound spell should resolve");

        let in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Rebound Probe")
                .unwrap_or(false)
        });
        assert!(in_exile, "rebound spell should be exiled on resolution");

        assert_eq!(
            game.delayed_triggers.len(),
            1,
            "rebound should schedule exactly one next-upkeep cast trigger"
        );
        let delayed_debug = format!("{:?}", game.delayed_triggers[0].effects);
        assert!(
            delayed_debug.contains("CastSourceEffect"),
            "rebound delayed trigger should cast the exiled source, got {delayed_debug}"
        );
    }

    #[test]
    fn test_flashback_pays_alternative_cost() {
        use crate::cards::definitions::think_twice;
        use crate::mana::ManaSymbol;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add exactly 3 blue mana (flashback cost is {2}{U})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);

        // Create Think Twice in graveyard
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

        // Cast with flashback
        let mut state = PriorityLoopState::new(2);
        let mut trigger_queue = TriggerQueue::new();

        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: think_twice_id,
            from_zone: Zone::Graveyard,
            casting_method: CastingMethod::Alternative(0),
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
        assert!(result.is_ok(), "Casting with flashback should succeed");

        // Verify mana was spent (flashback costs {2}{U} = 3 total, we had 3 blue)
        let mana_pool = &game.player(alice).unwrap().mana_pool;
        assert_eq!(mana_pool.blue, 0, "Should have spent all mana on flashback");
    }

    #[test]
    fn test_normal_cast_goes_to_graveyard() {
        use crate::cards::definitions::think_twice;
        use crate::mana::ManaSymbol;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add 2 blue mana (normal cost is {1}{U})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        // Create Think Twice in HAND
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Hand);

        // Cast normally
        let mut state = PriorityLoopState::new(2);
        let mut trigger_queue = TriggerQueue::new();

        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: think_twice_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
        assert!(result.is_ok(), "Normal casting should succeed");

        // Resolve the spell
        resolve_stack_entry(&mut game).expect("Resolution should succeed");

        // Verify spell is in graveyard (not exile) after normal cast
        let player = game.player(alice).unwrap();
        let in_graveyard = player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(
            in_graveyard,
            "Think Twice SHOULD be in graveyard after normal cast"
        );

        let in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Think Twice")
                .unwrap_or(false)
        });
        assert!(
            !in_exile,
            "Think Twice should NOT be in exile after normal cast"
        );
    }

    #[test]
    fn test_flashback_requires_enough_mana() {
        use crate::cards::definitions::think_twice;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Add only 2 mana (flashback costs {2}{U} = 3 total)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        // Create Think Twice in graveyard
        let think_twice_def = think_twice();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find flashback action (not enough mana)
        let flashback_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    casting_method: CastingMethod::Alternative(_),
                    ..
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            flashback_action.is_none(),
            "Should NOT be able to cast with flashback without enough mana"
        );
    }

    // =========================================================================
    // Everflowing Chalice / Multikicker Tests
    // =========================================================================

    #[test]
    fn test_everflowing_chalice_no_kicks() {
        use crate::cards::definitions::everflowing_chalice;
        use crate::cost::OptionalCostsPaid;
        use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Everflowing Chalice directly on battlefield with 0 kicks
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

        // Simulate that it entered with 0 kicks by running the ETB effect
        // with an ExecutionContext that has 0 kicks
        let paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
        let mut ctx = ExecutionContext::new_default(chalice_id, alice)
            .with_optional_costs_paid(paid)
            .with_targets(vec![ResolvedTarget::Object(chalice_id)]);

        // Execute the ETB effect (put charge counters equal to kick count)
        let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
        execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

        // Should have 0 charge counters
        let chalice = game.object(chalice_id).unwrap();
        let charge_counters = chalice
            .counters
            .get(&CounterType::Charge)
            .copied()
            .unwrap_or(0);
        assert_eq!(
            charge_counters, 0,
            "Should have 0 charge counters with 0 kicks"
        );

        // Tap for mana - should produce 0 colorless
        let mana_effect = Effect::add_colorless_mana(Value::CountersOnSource(CounterType::Charge));
        let mut mana_ctx = ExecutionContext::new_default(chalice_id, alice);
        execute_effect(&mut game, &mana_effect, &mut mana_ctx).unwrap();

        assert_eq!(
            game.player(alice).unwrap().mana_pool.colorless,
            0,
            "Should produce 0 colorless mana with 0 counters"
        );
    }

    #[test]
    fn test_everflowing_chalice_one_kick() {
        use crate::cards::definitions::everflowing_chalice;
        use crate::cost::OptionalCostsPaid;
        use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Everflowing Chalice directly on battlefield
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

        // Simulate that it entered with 1 kick
        let mut paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
        paid.pay(0); // Pay multikicker once
        let mut ctx = ExecutionContext::new_default(chalice_id, alice)
            .with_optional_costs_paid(paid)
            .with_targets(vec![ResolvedTarget::Object(chalice_id)]);

        // Execute the ETB effect
        let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
        execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

        // Should have 1 charge counter
        let chalice = game.object(chalice_id).unwrap();
        assert_eq!(
            chalice.counters.get(&CounterType::Charge),
            Some(&1),
            "Should have 1 charge counter with 1 kick"
        );

        // Tap for mana - should produce 1 colorless
        let mana_effect = Effect::add_colorless_mana(Value::CountersOnSource(CounterType::Charge));
        let mut mana_ctx = ExecutionContext::new_default(chalice_id, alice);
        execute_effect(&mut game, &mana_effect, &mut mana_ctx).unwrap();

        assert_eq!(
            game.player(alice).unwrap().mana_pool.colorless,
            1,
            "Should produce 1 colorless mana with 1 counter"
        );
    }

    #[test]
    fn test_everflowing_chalice_two_kicks() {
        use crate::cards::definitions::everflowing_chalice;
        use crate::cost::OptionalCostsPaid;
        use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Everflowing Chalice directly on battlefield
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

        // Simulate that it entered with 2 kicks
        let mut paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
        paid.pay_times(0, 2); // Pay multikicker twice
        let mut ctx = ExecutionContext::new_default(chalice_id, alice)
            .with_optional_costs_paid(paid)
            .with_targets(vec![ResolvedTarget::Object(chalice_id)]);

        // Execute the ETB effect
        let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
        execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

        // Should have 2 charge counters
        let chalice = game.object(chalice_id).unwrap();
        assert_eq!(
            chalice.counters.get(&CounterType::Charge),
            Some(&2),
            "Should have 2 charge counters with 2 kicks"
        );

        // Tap for mana - should produce 2 colorless
        let mana_effect = Effect::add_colorless_mana(Value::CountersOnSource(CounterType::Charge));
        let mut mana_ctx = ExecutionContext::new_default(chalice_id, alice);
        execute_effect(&mut game, &mana_effect, &mut mana_ctx).unwrap();

        assert_eq!(
            game.player(alice).unwrap().mana_pool.colorless,
            2,
            "Should produce 2 colorless mana with 2 counters"
        );
    }

    #[test]
    fn test_everflowing_chalice_etb_trigger_uses_object_kick_count() {
        // This test verifies that when an ETB trigger fires, it can read
        // the kick count from the permanent that entered (not from ctx)
        use crate::cards::definitions::everflowing_chalice;
        use crate::cost::OptionalCostsPaid;
        use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
        use crate::object::CounterType;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Everflowing Chalice directly on battlefield
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

        // Set the optional_costs_paid on the object itself (simulating what
        // resolve_stack_entry does when a permanent enters)
        {
            let chalice = game.object_mut(chalice_id).unwrap();
            let mut paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
            paid.pay_times(0, 3); // 3 kicks
            chalice.optional_costs_paid = paid;
        }

        // Now execute the ETB effect with an EMPTY context (simulating a trigger)
        // The effect should still read the kick count from the source object
        let mut ctx = ExecutionContext::new_default(chalice_id, alice)
            .with_targets(vec![ResolvedTarget::Object(chalice_id)]);
        // Note: ctx.optional_costs_paid is empty, but the source object has it

        let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
        execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

        // Should have 3 charge counters (read from source object)
        let chalice = game.object(chalice_id).unwrap();
        assert_eq!(
            chalice.counters.get(&CounterType::Charge),
            Some(&3),
            "Should have 3 charge counters (read from object's optional_costs_paid)"
        );
    }

    // =========================================================================
    // Force of Will / Alternative Cost Tests
    // =========================================================================

    #[test]
    fn test_force_of_will_alternative_cost_available() {
        use crate::cards::definitions::force_of_will;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up - alice needs something to counter
        // Put a spell on the stack that bob cast
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice another blue card in hand to exile (an Island counts as blue for this test)
        // Actually, lands are colorless. Let's use a Counterspell instead.
        use crate::cards::definitions::counterspell;
        let cs_def = counterspell();
        let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice 20 life (default)
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find alternative cost option
        let alt_cost_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(
            alt_cost_action.is_some(),
            "Should be able to cast Force of Will with alternative cost when blue card available"
        );
    }

    #[test]
    fn test_force_of_will_alternative_cost_casting_flow() {
        use crate::alternative_cast::CastingMethod;
        use crate::cards::definitions::force_of_will;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up - alice needs something to counter
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice another blue card in hand to exile
        use crate::cards::definitions::counterspell;
        let cs_def = counterspell();
        let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Verify alternative method has cost effects but no mana cost
        let fow_obj = game.object(fow_id).unwrap();
        assert_eq!(fow_obj.alternative_casts.len(), 1);
        let method = &fow_obj.alternative_casts[0];
        assert!(
            !method.cost_effects().is_empty(),
            "Force of Will should have cost effects"
        );
        assert!(
            method.mana_cost().is_none(),
            "Force of Will alternative should NOT have a mana cost"
        );

        // Now test the casting flow
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut trigger_queue = TriggerQueue::new();

        // Execute the CastSpell action via apply_priority_response
        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: fow_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Alternative(0),
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
        assert!(result.is_ok(), "CastSpell action should succeed");

        // The result should be a target selection decision
        let progress = result.unwrap();
        match &progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_),
            ) => {
                // Good - now let's choose the target (Lightning Bolt)
            }
            _ => {
                panic!(
                    "Expected Targets context decision after casting Force of Will, got {:?}",
                    progress
                );
            }
        }

        // Now handle the target selection
        let pending = state.pending_cast.take().unwrap();
        let target = Target::Object(bolt_id);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let result = continue_to_mana_payment(
            &mut game,
            &mut trigger_queue,
            &mut state,
            pending,
            vec![target],
            &mut dm,
        );

        // This should NOT be a PayMana decision since there's no mana cost!
        // It should go straight to casting and then return a Priority decision
        match result {
            Ok(GameProgress::NeedsDecisionCtx(ref ctx)) => {
                // Check if this is a mana payment context
                if let crate::decisions::context::DecisionContext::SelectOptions(opts_ctx) = ctx {
                    if opts_ctx.description.contains("mana") {
                        panic!(
                            "Should NOT require mana payment for Force of Will alternative cost!"
                        );
                    }
                }
                // Other context types are acceptable (including Priority)
            }
            Ok(GameProgress::Continue) => {
                // Also acceptable
            }
            Ok(GameProgress::StackResolved) => {
                // Also acceptable
            }
            Ok(GameProgress::GameOver(_)) => {
                // Shouldn't happen but handle it
            }
            Err(e) => panic!("Error during casting: {:?}", e),
        }

        // Verify the alternative costs were paid
        // - Life should have decreased by 1
        let life = game.player(alice).unwrap().life;
        assert_eq!(life, 19, "Alice should have paid 1 life (got {})", life);

        // - The blue card should have been exiled
        // Note: move_object changes the ObjectId, so we need to look in exile
        let exiled_blue_card = game.exile.iter().any(|&id| {
            if let Some(obj) = game.object(id) {
                obj.name == "Counterspell"
            } else {
                false
            }
        });
        assert!(
            exiled_blue_card,
            "Blue card (Counterspell) should be in exile"
        );

        // - Force of Will should be on the stack
        assert!(
            game.stack.iter().any(|e| {
                if let Some(obj) = game.object(e.object_id) {
                    obj.name == "Force of Will"
                } else {
                    false
                }
            }),
            "Force of Will should be on the stack"
        );
    }

    #[test]
    fn test_force_of_will_alternative_cost_not_available_without_card() {
        use crate::cards::definitions::force_of_will;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand (this is her ONLY card)
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find alternative cost option (no other blue card to exile)
        let alt_cost_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(
            alt_cost_action.is_none(),
            "Should NOT be able to use alternative cost without another blue card"
        );
    }

    #[test]
    fn test_force_of_will_alternative_cost_not_available_with_only_nonblue_card() {
        use crate::cards::definitions::force_of_will;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice a non-blue card (Lightning Bolt is red)
        let red_card_def = lightning_bolt();
        let _red_card_id = game.create_object_from_definition(&red_card_def, alice, Zone::Hand);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find alternative cost option (no blue card to exile)
        let alt_cost_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(
            alt_cost_action.is_none(),
            "Should NOT be able to use alternative cost with only non-blue cards"
        );
    }

    #[test]
    fn test_force_of_will_normal_cast_available_with_mana() {
        use crate::cards::definitions::force_of_will;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice enough mana to cast normally: {3}{U}{U}
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find normal cast option
        let normal_cast = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == fow_id
            )
        });

        assert!(
            normal_cast.is_some(),
            "Should be able to cast Force of Will normally with 3UU"
        );
    }

    #[test]
    fn test_force_of_will_both_options_available() {
        use crate::cards::definitions::{counterspell, force_of_will, lightning_bolt};
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice another blue card (for alternative cost)
        let cs_def = counterspell();
        let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice enough mana to cast normally: {3}{U}{U}
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // When both Normal and Alternative are available, only Normal should appear in actions
        // The ChooseCastingMethod decision will present both options when the spell is selected
        let normal_cast = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == fow_id
            )
        });

        let alt_cast = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(normal_cast.is_some(), "Should be able to cast normally");
        assert!(
            alt_cast.is_none(),
            "Alternative should NOT be a separate action when Normal is also available"
        );

        // Count total CastSpell actions for Force of Will from hand
        let fow_cast_count = actions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    LegalAction::CastSpell {
                        spell_id,
                        from_zone: Zone::Hand,
                        ..
                    } if *spell_id == fow_id
                )
            })
            .count();
        assert_eq!(
            fow_cast_count, 1,
            "Should only have one CastSpell action for Force of Will"
        );
    }

    #[test]
    fn test_choose_casting_method_flow() {
        use crate::cards::definitions::{counterspell, force_of_will, lightning_bolt};
        use crate::decision::GameProgress;
        use crate::mana::ManaSymbol;
        use crate::triggers::TriggerQueue;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);

        // Create a spell on the stack for alice to counter
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_id, bob));

        // Give alice Force of Will in hand
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

        // Give alice another blue card (for alternative cost)
        let cs_def = counterspell();
        let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice enough mana to cast normally: {3}{U}{U}
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Now test the ChooseCastingMethod flow
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut trigger_queue = TriggerQueue::new();

        // Cast using Normal method - should trigger ChooseCastingMethod since both methods available
        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: fow_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        });

        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);

        // Should get a ChooseCastingMethod decision
        match result {
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            )) => {
                assert_eq!(ctx.player, alice);
                assert_eq!(ctx.source, Some(fow_id));
                assert_eq!(ctx.options.len(), 2, "Should have 2 casting method options");
                assert!(ctx.description.contains("Choose casting method"));
            }
            other => panic!(
                "Expected SelectOptions context for casting method, got {:?}",
                other
            ),
        }

        // Now choose the alternative cost (index 1)
        let method_response = PriorityResponse::CastingMethodChoice(1);
        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &method_response);

        // Should get ChooseTargets decision next (Force of Will targets a spell)
        // After targets, it will ask for card to exile
        match result {
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(ctx),
            )) => {
                assert_eq!(ctx.player, alice, "Should be alice choosing targets");
            }
            other => panic!(
                "Expected Targets context decision after method choice, got {:?}",
                other
            ),
        }
    }

    // =========================================================================
    // Underworld Breach / Granted Escape Tests
    // =========================================================================

    #[test]
    fn test_underworld_breach_grants_escape_to_graveyard_cards() {
        use crate::cards::definitions::{lightning_bolt, underworld_breach};
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put Lightning Bolt in graveyard
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Add 3 more cards to graveyard (for escape cost)
        let _bolt2_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt3_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt4_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Give alice enough mana to cast Lightning Bolt (R)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find a GrantedEscape cast option for Lightning Bolt
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == bolt_id
            )
        });

        assert!(
            escape_action.is_some(),
            "Should be able to cast Lightning Bolt with granted escape from graveyard"
        );
    }

    #[test]
    fn test_underworld_breach_no_escape_without_enough_cards_to_exile() {
        use crate::cards::definitions::{lightning_bolt, underworld_breach};
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put Lightning Bolt in graveyard (ONLY card)
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Only 1 card in graveyard - need 3 MORE to exile for escape
        // So escape should not be available

        // Give alice enough mana to cast Lightning Bolt (R)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find escape option (not enough cards to exile)
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == bolt_id
            )
        });

        assert!(
            escape_action.is_none(),
            "Should NOT be able to use escape without enough cards to exile"
        );
    }

    #[test]
    fn test_underworld_breach_escape_needs_3_other_cards() {
        // Regression test: with 3 cards in graveyard, you can only exile 2 OTHER cards,
        // so escape (which requires exiling 3) should NOT be available
        use crate::cards::definitions::{
            counterspell, force_of_will, think_twice, underworld_breach,
        };
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put 3 cards in graveyard
        let think_twice_def = think_twice();
        let fow_def = force_of_will();
        let cs_def = counterspell();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);
        let _fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Graveyard);
        let _cs_id = game.create_object_from_definition(&cs_def, alice, Zone::Graveyard);

        // Give alice enough mana to cast any of these
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 5);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Escape requires exiling 3 OTHER cards - but with only 3 total,
        // each card has only 2 other cards available, so NO escape should be available
        let escape_actions: Vec<_> = actions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    LegalAction::CastSpell {
                        from_zone: Zone::Graveyard,
                        casting_method: CastingMethod::GrantedEscape { .. },
                        ..
                    }
                )
            })
            .collect();

        assert!(
            escape_actions.is_empty(),
            "Should NOT be able to use escape with only 3 cards in graveyard (need 3 OTHER cards). Found {} escape actions: {:?}",
            escape_actions.len(),
            escape_actions
                .iter()
                .map(|a| if let LegalAction::CastSpell { spell_id, .. } = a {
                    game.object(*spell_id)
                        .map(|o| o.name.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                })
                .collect::<Vec<_>>()
        );

        // Flashback for Think Twice SHOULD still be available though
        let flashback_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == think_twice_id
            )
        });
        assert!(
            flashback_action.is_some(),
            "Think Twice's intrinsic flashback should still be available"
        );
    }

    #[test]
    fn test_underworld_breach_doesnt_grant_escape_to_lands() {
        use crate::cards::definitions::{basic_mountain, underworld_breach};
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put a land in graveyard
        let mountain_def = basic_mountain();
        let mountain_id = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);

        // Add 3 more cards to graveyard (for potential escape cost)
        use crate::cards::definitions::lightning_bolt;
        let bolt_def = lightning_bolt();
        let _bolt2_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt3_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt4_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find escape option for the land
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == mountain_id
            )
        });

        assert!(
            escape_action.is_none(),
            "Underworld Breach should NOT grant escape to lands"
        );
    }

    #[test]
    fn test_underworld_breach_no_escape_without_breach_on_battlefield() {
        use crate::cards::definitions::lightning_bolt;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // NO Underworld Breach on battlefield

        // Put Lightning Bolt in graveyard
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Add 3 more cards to graveyard
        let _bolt2_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt3_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _bolt4_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Give alice mana
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should NOT find escape option (no Underworld Breach)
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == bolt_id
            )
        });

        assert!(
            escape_action.is_none(),
            "Should NOT be able to use escape without Underworld Breach on battlefield"
        );
    }

    #[test]
    fn test_force_of_will_cannot_use_alt_cost_when_escaping() {
        // This tests a tricky interaction:
        // Force of Will has an alternative cost (pay 1 life, exile a blue card from hand)
        // Underworld Breach grants escape (pay mana cost + exile 3 cards from graveyard)
        //
        // According to MTG rules, you CANNOT combine alternative costs.
        // When casting via granted escape, you must pay the escape cost (card's mana cost + exile 3).
        // You cannot use Force of Will's own alternative cost from the graveyard.

        use crate::cards::definitions::{
            counterspell, force_of_will, lightning_bolt, underworld_breach,
        };
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up main phase with something on the stack to counter
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put a spell on the stack for alice to counter
        let bolt_def = lightning_bolt();
        let bolt_stack_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        game.stack.push(StackEntry::new(bolt_stack_id, bob));

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put Force of Will in GRAVEYARD
        let fow_def = force_of_will();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Graveyard);

        // Add 3 more cards to graveyard (for escape cost)
        let _extra1 = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _extra2 = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
        let _extra3 = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

        // Give alice a blue card in hand (would be needed for FoW's own alternative cost)
        let cs_def = counterspell();
        let _blue_card_in_hand = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

        // Give alice enough mana to cast Force of Will normally (3UU = 5 mana)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);

        // Give alice 20 life
        game.player_mut(alice).unwrap().life = 20;

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find granted escape option (from graveyard via Underworld Breach)
        let granted_escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == fow_id
            )
        });

        assert!(
            granted_escape_action.is_some(),
            "Should be able to cast Force of Will via granted escape from graveyard"
        );

        // Should NOT find Force of Will's own alternative cost from graveyard
        // (Alternative cost says "from hand", not "from graveyard")
        let fow_alt_cost_from_graveyard = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == fow_id
            )
        });

        assert!(
            fow_alt_cost_from_graveyard.is_none(),
            "Should NOT be able to use Force of Will's own alternative cost from graveyard - \
             alternative costs cannot be combined, and FoW's alt cost requires casting from hand"
        );

        // Also verify: no weird hybrid option that combines both costs
        // (There shouldn't be any action that lets you pay "1 life + exile blue card + exile 3 from GY")
        // This is implicitly tested by the above - we only have GrantedEscape, not Alternative(0)
    }

    #[test]
    fn test_underworld_breach_escape_works_with_4_cards() {
        // With 4 cards in graveyard, escape SHOULD be available (3 other cards to exile)
        // This tests the positive case - escape IS legal when there are enough cards
        use crate::cards::definitions::{basic_mountain, think_twice, underworld_breach};
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put Underworld Breach on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Put 4 cards in graveyard: Think Twice + 3 others (Mountain is a land but can still be exiled)
        let think_twice_def = think_twice();
        let mountain_def = basic_mountain();
        let think_twice_id =
            game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);
        let _m1 = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);
        let _m2 = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);
        let _m3 = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);

        // Give alice enough mana for flashback (2U = 3 mana, more expensive than escape's 1U)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        // Should find Think Twice [Escape] option
        let escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            escape_action.is_some(),
            "Think Twice [Escape] should be available with 4 cards in graveyard (3 other cards to exile)"
        );

        // Also verify Think Twice's normal Flashback is still available
        let flashback_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == think_twice_id
            )
        });

        assert!(
            flashback_action.is_some(),
            "Think Twice's intrinsic Flashback should also be available"
        );
    }

    #[test]
    fn test_force_of_will_escape_with_spell_on_stack() {
        // Simulates:
        // - Player 1 has Underworld Breach, 5 Islands, Force of Will + 3 cards in graveyard
        // - Player 2 casts Lightning Bolt
        // - Player 1 should be able to counter with Force of Will via Escape
        use crate::cards::definitions::{
            basic_mountain, counterspell, force_of_will, lightning_bolt, think_twice,
            underworld_breach,
        };
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up - it's Player 2's turn, main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = bob;

        // Player 1's setup: Underworld Breach + 5 Islands on battlefield
        let breach_def = underworld_breach();
        let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

        // Player 1's graveyard: Force of Will, Counterspell, Think Twice, Mountain (4 cards)
        let fow_def = force_of_will();
        let cs_def = counterspell();
        let tt_def = think_twice();
        let mtn_def = basic_mountain();
        let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Graveyard);
        let _cs_id = game.create_object_from_definition(&cs_def, alice, Zone::Graveyard);
        let _tt_id = game.create_object_from_definition(&tt_def, alice, Zone::Graveyard);
        let _mtn_id = game.create_object_from_definition(&mtn_def, alice, Zone::Graveyard);

        // Player 2 casts Lightning Bolt targeting Player 1
        let bolt_def = lightning_bolt();
        let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
        let mut bolt_entry = StackEntry::new(bolt_id, bob);
        bolt_entry.targets = vec![Target::Player(alice)];
        game.stack.push(bolt_entry);

        // Now Player 1 has priority to respond
        game.turn.priority_player = Some(alice);

        // Give Player 1 mana to cast Force of Will (3UU = 5 mana)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 5);

        // Compute legal actions for Player 1
        let actions = compute_legal_actions(&game, alice);

        // Should find Force of Will [Escape] option - there's a spell on the stack to counter!
        let fow_escape_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                } if *spell_id == fow_id
            )
        });

        assert!(
            fow_escape_action.is_some(),
            "Force of Will [Escape] should be available when there's a spell on the stack to counter. \
             Graveyard has 4 cards (3 others to exile), and Lightning Bolt is on the stack as a legal target."
        );

        // Now actually cast Force of Will via Escape
        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(2); // 2 players

        // Cast the spell
        let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: fow_id,
            from_zone: Zone::Graveyard,
            casting_method: CastingMethod::GrantedEscape {
                source: game
                    .battlefield
                    .iter()
                    .find(|&&id| {
                        game.object(id)
                            .map(|o| o.name == "Underworld Breach")
                            .unwrap_or(false)
                    })
                    .copied()
                    .unwrap(),
                exile_count: 3,
            },
        });

        let progress =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);

        // Should need to choose targets (Lightning Bolt is the only legal target)
        assert!(
            matches!(
                progress,
                Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Targets(_)
                ))
            ),
            "Should prompt for targets after casting Force of Will. Got: {:?}",
            progress
        );

        // Provide the target (Lightning Bolt on stack - spells are objects)
        let targets_response = PriorityResponse::Targets(vec![Target::Object(bolt_id)]);
        let progress2 =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &targets_response);

        assert!(
            progress2.is_ok(),
            "Targeting should succeed. Got: {:?}",
            progress2
        );

        // Verify the escape cost was paid:
        // - Force of Will should now be on the stack
        // - 3 cards should have been exiled from Alice's graveyard
        // - Alice's graveyard should now have only 0 cards (FoW moved to stack, 3 exiled)

        let alice_graveyard_count = game.player(alice).unwrap().graveyard.len();
        assert_eq!(
            alice_graveyard_count, 0,
            "Alice's graveyard should be empty after casting FoW via escape (1 cast + 3 exiled). Got: {}",
            alice_graveyard_count
        );

        // Verify 3 cards were exiled
        let alice_exile_count = game
            .exile
            .iter()
            .filter(|&&id| game.object(id).map(|o| o.owner == alice).unwrap_or(false))
            .count();
        assert_eq!(
            alice_exile_count, 3,
            "3 cards should have been exiled from Alice's graveyard for escape cost. Got: {}",
            alice_exile_count
        );

        // Verify Force of Will is on the stack
        let fow_on_stack = game.stack.iter().any(|entry| {
            game.object(entry.object_id)
                .map(|o| o.name == "Force of Will")
                .unwrap_or(false)
        });
        assert!(fow_on_stack, "Force of Will should be on the stack");
    }

    // ============================================================================
    // Affinity for Artifacts Tests
    // ============================================================================

    #[test]
    fn test_affinity_reduces_mana_cost() {
        // Frogmite costs {4} with affinity for artifacts
        // With 4 artifacts in play, it should cost {0}
        use crate::cards::definitions::frogmite;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 4 artifacts on the battlefield
        for i in 0..4 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Put Frogmite in hand with NO mana in pool
        let frogmite_def = frogmite();
        let frogmite_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

        // Compute legal actions - Frogmite should be castable with 0 mana
        let actions = compute_legal_actions(&game, alice);

        let can_cast_frogmite = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == frogmite_id
            )
        });

        assert!(
            can_cast_frogmite,
            "Should be able to cast Frogmite for free with 4 artifacts in play"
        );

        // Verify the effective cost is 0
        let frogmite_obj = game.object(frogmite_id).unwrap();
        let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            0,
            "Effective cost should be 0 with 4 artifacts"
        );
    }

    #[test]
    fn test_affinity_partial_reduction() {
        // Frogmite costs {4} with affinity for artifacts
        // With 2 artifacts in play, it should cost {2}
        use crate::cards::definitions::frogmite;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 2 artifacts on the battlefield
        for i in 0..2 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Put Frogmite in hand
        let frogmite_def = frogmite();
        let frogmite_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

        // Verify the effective cost is 2
        let frogmite_obj = game.object(frogmite_id).unwrap();
        let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            2,
            "Effective cost should be 2 with 2 artifacts"
        );

        // With only 1 mana, should NOT be able to cast
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == frogmite_id
            )
        });
        assert!(
            !can_cast,
            "Should NOT be able to cast Frogmite with only 1 mana when cost is 2"
        );

        // With 2 mana, should be able to cast
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == frogmite_id
            )
        });
        assert!(
            can_cast,
            "Should be able to cast Frogmite with 2 mana when cost is 2"
        );
    }

    #[test]
    fn test_affinity_only_counts_own_artifacts() {
        // Affinity only counts artifacts YOU control
        use crate::cards::definitions::frogmite;
        use crate::decision::calculate_effective_mana_cost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create 2 artifacts controlled by Alice
        for i in 0..2 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Alice Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Create 2 artifacts controlled by Bob (should NOT count)
        for i in 10..12 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Bob Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, bob, Zone::Battlefield);
        }

        // Put Frogmite in Alice's hand
        let frogmite_def = frogmite();
        let frogmite_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

        // Verify effective cost is 2 (only Alice's artifacts count)
        let frogmite_obj = game.object(frogmite_id).unwrap();
        let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            2,
            "Effective cost should be 2 - only Alice's 2 artifacts count, not Bob's"
        );
    }

    #[test]
    fn test_frogmite_counts_as_artifact_for_affinity_when_on_battlefield() {
        // Frogmite is an artifact creature, so once on battlefield it counts for other affinity costs
        use crate::cards::definitions::frogmite;
        use crate::decision::calculate_effective_mana_cost;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Put one Frogmite on the battlefield
        let frogmite_def = frogmite();
        let _battlefield_frogmite_id =
            game.create_object_from_definition(&frogmite_def, alice, Zone::Battlefield);

        // Put another Frogmite in hand
        let frogmite_in_hand_id =
            game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

        // The first Frogmite on battlefield should count as an artifact
        let frogmite_obj = game.object(frogmite_in_hand_id).unwrap();
        let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            3,
            "Effective cost should be 3 - one artifact (the other Frogmite) on battlefield"
        );
    }

    // ============================================================================
    // Delve Tests
    // ============================================================================

    #[test]
    fn test_delve_reduces_mana_cost() {
        // Treasure Cruise costs {7}{U} with Delve
        // With 7 cards in graveyard, it should cost just {U}
        use crate::cards::definitions::treasure_cruise;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put 7 cards in graveyard
        for i in 0..7 {
            let card = CardBuilder::new(CardId::new(), &format!("Graveyard Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        // Put Treasure Cruise in hand
        let tc_def = treasure_cruise();
        let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

        // Give alice just 1 blue mana (the {U} part)
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);

        // Verify the effective cost is just {U} (mana value 1)
        let tc_obj = game.object(tc_id).unwrap();
        let base_cost = tc_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, tc_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            1,
            "Effective cost should be 1 (just U) with 7 cards in graveyard to delve"
        );

        // Compute legal actions - Treasure Cruise should be castable with 1 blue mana
        let actions = compute_legal_actions(&game, alice);

        let can_cast_tc = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == tc_id
            )
        });

        assert!(
            can_cast_tc,
            "Should be able to cast Treasure Cruise with 7 cards to delve and 1 blue mana"
        );
    }

    #[test]
    fn test_delve_partial_reduction() {
        // Treasure Cruise costs {7}{U} with Delve
        // With 3 cards in graveyard, it should cost {4}{U}
        use crate::cards::definitions::treasure_cruise;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put 3 cards in graveyard
        for i in 0..3 {
            let card = CardBuilder::new(CardId::new(), &format!("Graveyard Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        // Put Treasure Cruise in hand
        let tc_def = treasure_cruise();
        let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

        // Verify effective cost is {4}{U} = 5
        let tc_obj = game.object(tc_id).unwrap();
        let base_cost = tc_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, tc_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            5,
            "Effective cost should be 5 (4U) with 3 cards to delve"
        );

        // With only 3 mana (not enough), should NOT be able to cast
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == tc_id
            )
        });
        assert!(
            !can_cast,
            "Should NOT be able to cast with only 3 mana when effective cost is 5"
        );

        // With 5 mana, should be able to cast
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);

        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == tc_id
            )
        });
        assert!(
            can_cast,
            "Should be able to cast with 5 mana when effective cost is 5"
        );
    }

    #[test]
    fn test_delve_exiles_cards_on_cast() {
        // When casting with Delve, cards should be exiled from graveyard
        use crate::cards::definitions::treasure_cruise;
        use crate::decision::LegalAction;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Put 7 cards in graveyard
        let mut gy_cards = Vec::new();
        for i in 0..7 {
            let card = CardBuilder::new(CardId::new(), &format!("Graveyard Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            let id = game.create_object_from_card(&card, alice, Zone::Graveyard);
            gy_cards.push(id);
        }

        // Put Treasure Cruise in hand
        let tc_def = treasure_cruise();
        let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

        // Give alice 1 blue mana
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);

        // Verify initial state
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 7);
        assert_eq!(game.exile.len(), 0);

        // Cast Treasure Cruise
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        let response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: tc_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        });

        let result = apply_priority_response(&mut game, &mut trigger_queue, &mut state, &response);
        assert!(result.is_ok(), "Casting should succeed");

        // Verify 7 cards were exiled from graveyard
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            0,
            "Graveyard should be empty after delving 7 cards"
        );
        assert_eq!(
            game.exile.len(),
            7,
            "7 cards should be in exile after delving"
        );

        // Treasure Cruise should be on the stack
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn test_delve_cannot_cast_without_enough_graveyard_or_mana() {
        // Treasure Cruise costs {7}{U}
        // With 0 cards in graveyard and only 3 mana, should NOT be castable
        use crate::cards::definitions::treasure_cruise;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Empty graveyard
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 0);

        // Put Treasure Cruise in hand
        let tc_def = treasure_cruise();
        let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

        // Give alice 3 mana (not enough for {7}{U})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);

        // Should NOT be able to cast
        let actions = compute_legal_actions(&game, alice);
        let can_cast = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    ..
                } if *spell_id == tc_id
            )
        });

        assert!(
            !can_cast,
            "Should NOT be able to cast Treasure Cruise with no graveyard and only 3 mana"
        );
    }

    #[test]
    fn test_convoke_reduces_mana_cost_with_creatures() {
        // Stoke the Flames costs {2}{R}{R} with Convoke
        // With 2 untapped creatures (one red), it should cost {1}{R}
        use crate::cards::definitions::stoke_the_flames;
        use crate::color::ColorSet;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 2 untapped creatures on battlefield (one red, one colorless)
        let red_creature = CardBuilder::new(CardId::from_raw(800), "Red Creature")
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let colorless_creature = CardBuilder::new(CardId::from_raw(801), "Colorless Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();

        let red_id = game.create_object_from_card(&red_creature, alice, Zone::Battlefield);
        let colorless_id =
            game.create_object_from_card(&colorless_creature, alice, Zone::Battlefield);

        // Mark them as not summoning sick
        game.remove_summoning_sickness(red_id);
        game.remove_summoning_sickness(colorless_id);

        // Put Stoke the Flames in hand
        let stoke_def = stoke_the_flames();
        let stoke_id = game.create_object_from_definition(&stoke_def, alice, Zone::Hand);

        // Give alice {1}{R} mana (red creature pays one {R}, colorless pays {1} of the {2})
        // Use Colorless for generic since Generic(1) doesn't add to pool
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 1);

        // Verify the effective cost is reduced
        let stoke_obj = game.object(stoke_id).unwrap();
        let base_cost = stoke_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, stoke_obj, base_cost);

        // With red creature paying {R} and colorless paying {1}, remaining should be {1}{R}
        assert_eq!(
            effective_cost.mana_value(),
            2,
            "Effective cost should be 2 (1 generic + 1 red) with 2 creatures to convoke"
        );

        // Compute legal actions - Stoke should be castable
        let actions = compute_legal_actions(&game, alice);

        let can_cast_stoke = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == stoke_id
            )
        });

        assert!(
            can_cast_stoke,
            "Should be able to cast Stoke the Flames with 2 creatures to convoke and 2 mana"
        );
    }

    #[test]
    fn test_convoke_taps_creatures_on_cast() {
        // When casting with Convoke, the creatures used should be tapped
        use crate::cards::definitions::stoke_the_flames;
        use crate::color::ColorSet;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 4 red creatures (enough to pay the entire cost with Convoke)
        let mut creature_ids = Vec::new();
        for i in 0..4 {
            let creature = CardBuilder::new(CardId::new(), &format!("Red Creature {}", i))
                .card_types(vec![CardType::Creature])
                .color_indicator(ColorSet::RED)
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
            let id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
            game.remove_summoning_sickness(id);
            creature_ids.push(id);
        }

        // Put Stoke the Flames in hand
        let stoke_def = stoke_the_flames();
        let stoke_id = game.create_object_from_definition(&stoke_def, alice, Zone::Hand);

        // Give alice no mana - should still be able to cast with 4 creatures
        // (2 pay generic, 2 pay red)

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        let cast_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == stoke_id
            )
        });

        assert!(
            cast_action.is_some(),
            "Should be able to cast Stoke the Flames with 4 creatures to convoke (paying all costs)"
        );

        // Cast the spell - since it requires targeting, we need to handle that
        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());

        // Apply the cast action - this returns the ChooseTargets decision
        let response = PriorityResponse::PriorityAction(cast_action.unwrap().clone());
        let result =
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &response).unwrap();

        // The spell requires targets, so we should get a ChooseTargets decision
        if let GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) = result
        {
            // Choose bob as target - this finalizes the cast and taps creatures
            let target_response = PriorityResponse::Targets(vec![Target::Player(bob)]);
            apply_priority_response(&mut game, &mut trigger_queue, &mut state, &target_response)
                .unwrap();
        } else {
            panic!("Expected ChooseTargets decision, got {:?}", result);
        }

        // Now the spell should be on the stack and creatures should be tapped
        // Check how many creatures are tapped
        let tapped_count = creature_ids
            .iter()
            .filter(|&&id| game.is_tapped(id))
            .count();

        assert!(
            tapped_count >= 2,
            "At least 2 creatures should be tapped for Convoke (tapped: {})",
            tapped_count
        );
    }

    #[test]
    fn test_convoke_colored_creatures_pay_colored_mana() {
        // Red creatures should be used to pay {R} pips first
        use crate::color::ColorSet;
        use crate::decision::{calculate_convoke_cost, get_convoke_creatures};
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 2 red creatures and 2 colorless creatures
        let red1 = CardBuilder::new(CardId::from_raw(800), "Red Creature 1")
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let red2 = CardBuilder::new(CardId::from_raw(801), "Red Creature 2")
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let colorless1 = CardBuilder::new(CardId::from_raw(802), "Colorless Creature 1")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let colorless2 = CardBuilder::new(CardId::from_raw(803), "Colorless Creature 2")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();

        let red1_id = game.create_object_from_card(&red1, alice, Zone::Battlefield);
        let red2_id = game.create_object_from_card(&red2, alice, Zone::Battlefield);
        let colorless1_id = game.create_object_from_card(&colorless1, alice, Zone::Battlefield);
        let colorless2_id = game.create_object_from_card(&colorless2, alice, Zone::Battlefield);

        // Mark them as not summoning sick
        for id in [red1_id, red2_id, colorless1_id, colorless2_id] {
            game.remove_summoning_sickness(id);
        }

        // Get convoke creatures
        let convoke_creatures = get_convoke_creatures(&game, alice);
        assert_eq!(
            convoke_creatures.len(),
            4,
            "Should have 4 creatures available for convoke"
        );

        // Calculate convoke cost for Stoke the Flames: {2}{R}{R}
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]);

        let (creatures_to_tap, remaining_cost) = calculate_convoke_cost(&game, alice, &cost);

        // Should tap all 4 creatures: 2 red for the {R}{R}, 2 colorless for the {2}
        assert_eq!(
            creatures_to_tap.len(),
            4,
            "Should tap 4 creatures to pay the entire cost"
        );

        // Remaining cost should be empty (mana value 0)
        assert_eq!(
            remaining_cost.mana_value(),
            0,
            "Remaining cost should be 0 after tapping 4 creatures"
        );
    }

    #[test]
    fn test_convoke_summoning_sick_creatures_cannot_be_tapped() {
        // Summoning sick creatures cannot be used for Convoke (unless they have haste)
        use crate::decision::get_convoke_creatures;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 2 creatures - one summoning sick, one not
        let creature1 = CardBuilder::new(CardId::from_raw(800), "Regular Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let creature2 = CardBuilder::new(CardId::from_raw(801), "Sick Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();

        let id1 = game.create_object_from_card(&creature1, alice, Zone::Battlefield);
        let id2 = game.create_object_from_card(&creature2, alice, Zone::Battlefield);

        // from_card sets summoning_sick to false by default, so we need to:
        // - Keep id1 as not summoning sick (can be used for convoke)
        // - Set id2 as summoning sick (cannot be used for convoke)
        game.set_summoning_sick(id2);

        // Get convoke creatures
        let convoke_creatures = get_convoke_creatures(&game, alice);

        // Should only get the non-summoning-sick creature
        assert_eq!(
            convoke_creatures.len(),
            1,
            "Only 1 creature should be available (summoning sick creatures can't convoke)"
        );
        assert_eq!(
            convoke_creatures[0].0, id1,
            "Only the non-summoning-sick creature should be available"
        );
    }

    #[test]
    fn test_improvise_reduces_mana_cost_with_artifacts() {
        // Reverse Engineer costs {3}{U}{U} with Improvise
        // With 3 untapped artifacts, it should cost just {U}{U}
        use crate::cards::definitions::reverse_engineer;
        use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 3 untapped artifacts on battlefield
        for i in 0..3 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Put Reverse Engineer in hand
        let re_def = reverse_engineer();
        let re_id = game.create_object_from_definition(&re_def, alice, Zone::Hand);

        // Give alice {U}{U} mana (3 artifacts pay the {3})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        // Verify the effective cost is just {U}{U} (mana value 2)
        let re_obj = game.object(re_id).unwrap();
        let base_cost = re_obj.mana_cost.as_ref().unwrap();
        let effective_cost = calculate_effective_mana_cost(&game, alice, re_obj, base_cost);
        assert_eq!(
            effective_cost.mana_value(),
            2,
            "Effective cost should be 2 (just UU) with 3 artifacts to improvise"
        );

        // Compute legal actions - Reverse Engineer should be castable
        let actions = compute_legal_actions(&game, alice);

        let can_cast_re = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == re_id
            )
        });

        assert!(
            can_cast_re,
            "Should be able to cast Reverse Engineer with 3 artifacts to improvise and 2 blue mana"
        );
    }

    #[test]
    fn test_improvise_taps_artifacts_on_cast() {
        // When casting with Improvise, the artifacts used should be tapped
        use crate::cards::definitions::reverse_engineer;
        use crate::decision::compute_legal_actions;
        use crate::mana::ManaSymbol;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set up main phase
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;

        // Create 3 untapped artifacts
        let mut artifact_ids = Vec::new();
        for i in 0..3 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            let id = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
            artifact_ids.push(id);
        }

        // Put Reverse Engineer in hand
        let re_def = reverse_engineer();
        let re_id = game.create_object_from_definition(&re_def, alice, Zone::Hand);

        // Give alice {U}{U} mana (3 artifacts pay the {3})
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        // Compute legal actions
        let actions = compute_legal_actions(&game, alice);

        let cast_action = actions.iter().find(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == re_id
            )
        });

        assert!(
            cast_action.is_some(),
            "Should be able to cast Reverse Engineer"
        );

        // Cast the spell (no targets needed for draw spell)
        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());

        let response = PriorityResponse::PriorityAction(cast_action.unwrap().clone());
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &response).unwrap();

        // Now the spell should be on the stack and artifacts should be tapped
        let tapped_count = artifact_ids
            .iter()
            .filter(|&&id| game.is_tapped(id))
            .count();

        assert_eq!(
            tapped_count, 3,
            "All 3 artifacts should be tapped for Improvise"
        );
    }

    #[cfg(feature = "net")]
    #[test]
    fn test_direct_finalize_trace_includes_delve_exile() {
        use crate::cards::CardDefinitionBuilder;
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Build a spell with Delve + Convoke + Improvise and a generic cost.
        let spell_def = CardDefinitionBuilder::new(CardId::new(), "Trace Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(6)]]))
            .delve()
            .convoke()
            .improvise()
            .build();

        let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Hand);

        // Add 2 creatures for Convoke.
        for i in 0..2 {
            let creature = CardBuilder::new(CardId::new(), &format!("Convoke {}", i))
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
            let id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
            game.remove_summoning_sickness(id);
        }

        // Add 2 artifacts for Improvise.
        for i in 0..2 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Improvise {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Add 2 cards to graveyard for Delve.
        for i in 0..2 {
            let card = CardBuilder::new(CardId::new(), &format!("Graveyard {}", i))
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        let expected_delve: Vec<GameObjectId> = game
            .player(alice)
            .unwrap()
            .graveyard
            .iter()
            .map(|id| GameObjectId(id.0))
            .collect();

        let mut payment_trace = Vec::new();
        let mut trigger_queue = TriggerQueue::new();
        let mut dm = AutoPassDecisionMaker;
        let mut state = PriorityLoopState::new(game.players_in_game());

        finalize_spell_cast(
            &mut game,
            &mut trigger_queue,
            &mut state,
            spell_id,
            Zone::Hand,
            alice,
            Vec::new(),
            None,
            CastingMethod::Normal,
            OptionalCostsPaid::default(),
            None,
            Vec::new(),
            ManaPool::default(),
            Vec::new(),
            &mut payment_trace,
            false,
            spell_id,
            &mut dm,
        )
        .unwrap();

        // finalize_spell_cast no longer applies Convoke/Improvise fallback taps directly.
        // Those are now represented as pip-payment alternatives before finalize.
        assert_eq!(payment_trace.len(), 1);

        match &payment_trace[0] {
            CostStep::Payment(CostPayment::Exile { objects, from_zone }) => {
                assert_eq!(*from_zone, ZoneCode::Graveyard);
                assert_eq!(objects, &expected_delve);
            }
            other => panic!("Expected delve exile step first, got {:?}", other),
        }
    }

    #[test]
    fn test_improvise_only_pays_generic_mana() {
        // Improvise cannot pay for colored mana pips
        use crate::decision::{calculate_improvise_cost, get_improvise_artifacts};
        use crate::mana::{ManaCost, ManaSymbol};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 5 untapped artifacts (more than enough)
        for i in 0..5 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }

        // Verify artifacts are available
        let artifacts = get_improvise_artifacts(&game, alice);
        assert_eq!(artifacts.len(), 5, "Should have 5 artifacts available");

        // Calculate improvise cost for {3}{U}{U} - should only reduce the {3}
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]);

        let (artifacts_to_tap, remaining_cost) = calculate_improvise_cost(&game, alice, &cost);

        // Should tap 3 artifacts to pay the {3}
        assert_eq!(
            artifacts_to_tap.len(),
            3,
            "Should tap 3 artifacts to pay the generic mana"
        );

        // Remaining cost should be {U}{U} (mana value 2)
        assert_eq!(
            remaining_cost.mana_value(),
            2,
            "Remaining cost should be 2 (UU) - Improvise doesn't pay colored"
        );
    }

    #[test]
    fn test_improvise_already_tapped_artifacts_cannot_be_used() {
        // Tapped artifacts cannot be used for Improvise
        use crate::decision::get_improvise_artifacts;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 3 artifacts - 2 tapped, 1 untapped
        for i in 0..3 {
            let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
                .card_types(vec![CardType::Artifact])
                .build();
            let id = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
            if i < 2 {
                game.tap(id);
            }
        }

        // Get improvise artifacts
        let artifacts = get_improvise_artifacts(&game, alice);

        // Should only get the 1 untapped artifact
        assert_eq!(
            artifacts.len(),
            1,
            "Only 1 artifact should be available (tapped artifacts can't improvise)"
        );
    }

    // =========================================================================
    // Search Library Tests (The Birth of Meletis)
    // =========================================================================

    #[test]
    fn test_search_library_finds_matching_card() {
        use crate::cards::definitions::{basic_plains, the_birth_of_meletis};
        use crate::decision::DecisionMaker;
        use crate::effect::Effect;
        use crate::executor::ExecutionContext;

        // Decision maker that always selects the first matching card
        struct SelectFirstDecisionMaker;
        impl DecisionMaker for SelectFirstDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Select the first legal candidate
                ctx.candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id)
                    .take(1)
                    .collect()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add basic Plains to library
        let plains_def = basic_plains();
        let _plains_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);

        // Also add some non-Plains cards to make the search interesting
        for i in 0..3 {
            let card = CardBuilder::new(CardId::new(), &format!("Random Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let initial_hand_size = game.player(alice).unwrap().hand.len();
        let initial_library_size = game.player(alice).unwrap().library.len();

        // Create a dummy source object for the context
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Execute the search effect directly
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter::default()
                .with_supertype(crate::types::Supertype::Basic)
                .with_subtype(crate::types::Subtype::Plains),
            Zone::Hand,
            crate::target::PlayerFilter::You,
            true,
        );

        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should succeed");

        // Verify Plains moved to hand
        let final_hand_size = game.player(alice).unwrap().hand.len();
        assert_eq!(
            final_hand_size,
            initial_hand_size + 1,
            "Should have one more card in hand"
        );

        // Verify library has one fewer card
        let final_library_size = game.player(alice).unwrap().library.len();
        assert_eq!(
            final_library_size,
            initial_library_size - 1,
            "Should have one fewer card in library"
        );

        // Verify the card in hand is a Plains
        let hand = &game.player(alice).unwrap().hand;
        let plains_in_hand = hand.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Plains" && o.subtypes.contains(&crate::types::Subtype::Plains))
                .unwrap_or(false)
        });
        assert!(plains_in_hand, "Plains should be in hand after search");
    }

    #[test]
    fn test_search_library_no_matching_cards() {
        use crate::cards::definitions::the_birth_of_meletis;
        use crate::decision::DecisionMaker;
        use crate::effect::{Effect, EffectResult};
        use crate::executor::ExecutionContext;

        // Decision maker for search (shouldn't be called if no matches)
        struct NoMatchDecisionMaker;
        impl DecisionMaker for NoMatchDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Should have no matching cards
                assert!(ctx.candidates.is_empty(), "Should have no matching cards");
                vec![]
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add only non-Plains cards to library (no basic Plains)
        for i in 0..3 {
            let card = CardBuilder::new(CardId::new(), &format!("Non-Plains Card {}", i))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_card(&card, alice, Zone::Library);
        }

        let initial_hand_size = game.player(alice).unwrap().hand.len();

        // Create source
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Execute the search effect
        let mut dm = NoMatchDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter::default()
                .with_supertype(crate::types::Supertype::Basic)
                .with_subtype(crate::types::Subtype::Plains),
            Zone::Hand,
            crate::target::PlayerFilter::You,
            true,
        );

        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should complete without error");

        // Result should indicate nothing was found
        if let Ok(outcome) = result {
            if let EffectResult::Count(n) = outcome.result {
                assert_eq!(n, 0, "Should find 0 cards when no Plains in library");
            }
        }

        // Hand size should be unchanged
        let final_hand_size = game.player(alice).unwrap().hand.len();
        assert_eq!(
            final_hand_size, initial_hand_size,
            "Hand size should be unchanged when no matching cards"
        );
    }

    #[test]
    fn test_search_library_fail_to_find() {
        use crate::cards::definitions::{basic_plains, the_birth_of_meletis};
        use crate::decision::DecisionMaker;
        use crate::effect::{Effect, EffectResult};
        use crate::executor::ExecutionContext;

        // Decision maker that always chooses to "fail to find" even with matching cards
        struct FailToFindDecisionMaker;
        impl DecisionMaker for FailToFindDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Verify there ARE matching cards, but we choose not to find them
                assert!(
                    !ctx.candidates.is_empty(),
                    "Should have matching cards available"
                );
                // Return empty to "fail to find"
                vec![]
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add basic Plains to library
        let plains_def = basic_plains();
        let _plains_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);

        let initial_hand_size = game.player(alice).unwrap().hand.len();
        let initial_library_size = game.player(alice).unwrap().library.len();

        // Create source
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Execute the search effect with fail-to-find decision maker
        let mut dm = FailToFindDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter::default()
                .with_supertype(crate::types::Supertype::Basic)
                .with_subtype(crate::types::Subtype::Plains),
            Zone::Hand,
            crate::target::PlayerFilter::You,
            true,
        );

        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should complete without error");

        // Result should indicate nothing was found (player chose to fail)
        if let Ok(outcome) = result {
            if let EffectResult::Count(n) = outcome.result {
                assert_eq!(
                    n, 0,
                    "Should report 0 cards found when player fails to find"
                );
            }
        }

        // Hand size should be unchanged (player declined to take the Plains)
        let final_hand_size = game.player(alice).unwrap().hand.len();
        assert_eq!(
            final_hand_size, initial_hand_size,
            "Hand size should be unchanged when player fails to find"
        );

        // Library size should also be unchanged (no card moved)
        let final_library_size = game.player(alice).unwrap().library.len();
        assert_eq!(
            final_library_size, initial_library_size,
            "Library size should be unchanged when player fails to find"
        );

        // Plains should still be in library
        let library = &game.player(alice).unwrap().library;
        let plains_in_library = library
            .iter()
            .any(|&id| game.object(id).map(|o| o.name == "Plains").unwrap_or(false));
        assert!(
            plains_in_library,
            "Plains should still be in library after fail to find"
        );
    }

    #[test]
    fn test_search_library_selects_specific_card() {
        use crate::cards::definitions::{basic_island, basic_plains, the_birth_of_meletis};
        use crate::decision::DecisionMaker;
        use crate::effect::Effect;
        use crate::executor::ExecutionContext;

        // Decision maker that selects the second matching card (if available)
        struct SelectSecondDecisionMaker;
        impl DecisionMaker for SelectSecondDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Select second card if available, otherwise first
                let legal_ids: Vec<ObjectId> = ctx
                    .candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id)
                    .collect();
                if legal_ids.len() > 1 {
                    vec![legal_ids[1]]
                } else if let Some(&id) = legal_ids.first() {
                    vec![id]
                } else {
                    vec![]
                }
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Add multiple basic Plains to library
        let plains_def = basic_plains();
        let plains1_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);
        let plains2_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);

        // Add a non-matching card between them
        let island_def = basic_island();
        let _island_id = game.create_object_from_definition(&island_def, alice, Zone::Library);

        // Create source
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Execute the search effect
        let mut dm = SelectSecondDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter::default()
                .with_supertype(crate::types::Supertype::Basic)
                .with_subtype(crate::types::Subtype::Plains),
            Zone::Hand,
            crate::target::PlayerFilter::You,
            true,
        );

        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should succeed");

        // Verify exactly one Plains moved to hand
        let hand = &game.player(alice).unwrap().hand;
        let plains_count_in_hand = hand
            .iter()
            .filter(|&&id| game.object(id).map(|o| o.name == "Plains").unwrap_or(false))
            .count();
        assert_eq!(
            plains_count_in_hand, 1,
            "Exactly one Plains should be in hand"
        );

        // Verify one Plains remains in library
        let library = &game.player(alice).unwrap().library;
        let plains_count_in_library = library
            .iter()
            .filter(|&&id| game.object(id).map(|o| o.name == "Plains").unwrap_or(false))
            .count();
        assert_eq!(
            plains_count_in_library, 1,
            "One Plains should remain in library"
        );

        // Check that one of the specific Plains IDs moved
        // (Note: IDs change on zone change, so we check by name)
        let moved_to_hand = !game.player(alice).unwrap().library.contains(&plains1_id)
            || !game.player(alice).unwrap().library.contains(&plains2_id);
        assert!(moved_to_hand, "One of the Plains should have moved to hand");
    }

    // ============================================================================
    // Saga Integration Tests
    // ============================================================================

    #[test]
    fn test_saga_etb_adds_lore_counter() {
        use crate::cards::definitions::the_birth_of_meletis;

        // Test that a saga entering the battlefield gets its initial lore counter
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Put saga directly on battlefield (simulating resolution)
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Add initial lore counter and check chapters (what resolve_stack_entry_full does)
        add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);

        // Verify saga has 1 lore counter
        let saga = game.object(saga_id).unwrap();
        let lore_count = saga.counters.get(&CounterType::Lore).copied().unwrap_or(0);
        assert_eq!(lore_count, 1, "Saga should have 1 lore counter after ETB");

        // Verify chapter 1 trigger is queued
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Chapter 1 trigger should be in queue"
        );
    }

    #[test]
    fn test_saga_precombat_main_adds_lore_counter() {
        use crate::cards::definitions::the_birth_of_meletis;

        // Test that sagas get a lore counter at precombat main phase
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Put saga on battlefield with 1 lore counter already (simulating after ETB)
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 1);

        // Simulate precombat main phase - add lore counters to sagas
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        // Verify saga now has 2 lore counters
        let saga = game.object(saga_id).unwrap();
        let lore_count = saga.counters.get(&CounterType::Lore).copied().unwrap_or(0);
        assert_eq!(
            lore_count, 2,
            "Saga should have 2 lore counters after precombat main"
        );

        // Verify chapter 2 trigger is queued (threshold crossed from 1 to 2)
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Chapter 2 trigger should be in queue"
        );
    }

    #[test]
    fn test_saga_final_chapter_marks_for_sacrifice() {
        use crate::cards::definitions::the_birth_of_meletis;

        // Test that when the final chapter ability resolves, the saga is marked for sacrifice
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put saga on battlefield with 3 lore counters (final chapter)
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 3);

        // Verify saga is not marked as final_chapter_resolved yet
        assert!(
            !game.is_saga_final_chapter_resolved(saga_id),
            "Saga should not be marked as resolved yet"
        );

        // Simulate final chapter ability resolving by calling mark_saga_final_chapter_resolved
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // Verify saga is now marked as final_chapter_resolved
        assert!(
            game.is_saga_final_chapter_resolved(saga_id),
            "Saga should be marked as resolved after final chapter ability"
        );
    }

    #[test]
    fn test_saga_sacrifice_sba() {
        use crate::cards::definitions::the_birth_of_meletis;
        use crate::rules::state_based::check_state_based_actions;

        // Test that a saga marked as final_chapter_resolved is sacrificed by SBA
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put saga on battlefield with final chapter resolved
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 3);
        game.mark_saga_final_chapter_resolved(saga_id);

        // Verify saga is on battlefield
        assert!(
            game.battlefield.contains(&saga_id),
            "Saga should be on battlefield"
        );

        // Check SBAs - should include saga sacrifice
        let sbas = check_state_based_actions(&game);

        // Verify saga sacrifice SBA is present
        let has_saga_sacrifice = sbas.iter().any(|sba| {
            matches!(
                sba,
                crate::rules::state_based::StateBasedAction::SagaSacrifice(id) if *id == saga_id
            )
        });
        assert!(
            has_saga_sacrifice,
            "SBA should include saga sacrifice for resolved saga"
        );

        // Apply SBAs
        let mut trigger_queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Verify saga is no longer on battlefield
        assert!(
            !game.battlefield.contains(&saga_id),
            "Saga should no longer be on battlefield after SBA"
        );

        // Verify a saga is in graveyard (note: zone change creates new object ID per rule 400.7)
        let alice_player = game.player(alice).unwrap();
        let saga_in_graveyard = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "The Birth of Meletis")
                .unwrap_or(false)
        });
        assert!(
            saga_in_graveyard,
            "Saga should be in graveyard after sacrifice"
        );
    }

    #[test]
    fn test_saga_full_lifecycle() {
        use crate::cards::definitions::the_birth_of_meletis;

        // Test the full saga lifecycle: ETB -> chapter triggers -> sacrifice
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Create saga and simulate entering battlefield
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Add initial lore counter and check chapters
        add_lore_counter_and_check_chapters(&mut game, saga_id, &mut trigger_queue);

        // Verify: 1 lore counter, chapter 1 triggered
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            1
        );
        assert_eq!(trigger_queue.entries.len(), 1);

        // Clear trigger queue (simulating triggers going on stack and resolving)
        trigger_queue.clear();

        // Simulate turn 2 - add lore counter at precombat main
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        // Verify: 2 lore counters, chapter 2 triggered
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2
        );
        assert_eq!(trigger_queue.entries.len(), 1);

        // Clear trigger queue
        trigger_queue.clear();

        // Simulate turn 3 - add lore counter at precombat main (final chapter)
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        // Verify: 3 lore counters, chapter 3 triggered
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            3
        );
        assert_eq!(trigger_queue.entries.len(), 1);

        // Verify saga is NOT marked for sacrifice yet (final chapter hasn't resolved)
        assert!(!game.is_saga_final_chapter_resolved(saga_id));

        // Simulate final chapter ability resolving
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // Verify saga IS marked for sacrifice
        assert!(game.is_saga_final_chapter_resolved(saga_id));

        // Apply SBAs - saga should be sacrificed
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Verify saga is no longer on battlefield
        assert!(
            !game.battlefield.contains(&saga_id),
            "Saga should no longer be on battlefield"
        );

        // Verify saga is in graveyard (note: zone change creates new object ID per rule 400.7)
        let alice_player = game.player(alice).unwrap();
        let saga_in_graveyard = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "The Birth of Meletis")
                .unwrap_or(false)
        });
        assert!(
            saga_in_graveyard,
            "Saga should be in graveyard after sacrifice"
        );
    }

    #[test]
    fn test_saga_survives_when_lore_counter_removed() {
        use crate::cards::definitions::{hex_parasite, ornithopter, urzas_saga};
        use crate::executor::execute_effect;

        // Test that removing a lore counter from a saga at its final chapter prevents sacrifice
        // This simulates: Urza's Saga with 2 counters, gets 3rd counter (final chapter),
        // respond with Hex Parasite to remove a counter, saga survives
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Put Urza's Saga on battlefield with 2 lore counters
        let saga_def = urzas_saga();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 2);

        // Put Hex Parasite on battlefield (not summoning sick for this test)
        let parasite_def = hex_parasite();
        let parasite_id =
            game.create_object_from_definition(&parasite_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(parasite_id);

        // Put Ornithopter in library (for Urza's Saga to find)
        let ornithopter_def = ornithopter();
        let _ornithopter_id =
            game.create_object_from_definition(&ornithopter_def, alice, Zone::Library);

        // Verify initial state
        assert_eq!(
            game.object(saga_id)
                .unwrap()
                .counters
                .get(&CounterType::Lore)
                .copied()
                .unwrap_or(0),
            2,
            "Saga should start with 2 lore counters"
        );

        // Simulate precombat main phase - saga gets 3rd lore counter (final chapter)
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        // Verify saga now has 3 lore counters and chapter 3 triggered
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            3,
            "Saga should have 3 lore counters"
        );
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Chapter 3 trigger should be in queue"
        );

        // The chapter 3 trigger is now in the queue, but BEFORE it resolves,
        // we respond by activating Hex Parasite to remove a lore counter.
        // (In a real game, the trigger would go on the stack, and we'd respond)

        // Simulate Hex Parasite's ability: remove 1 lore counter from Urza's Saga
        // (Paying 2 life for the phyrexian black mana)
        let remove_effect = Effect::remove_counters(
            CounterType::Lore,
            1, // Remove 1 counter (X=1)
            ChooseSpec::SpecificObject(saga_id),
        );
        let mut ctx = ExecutionContext::new_default(parasite_id, alice)
            .with_x(1)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(saga_id)]);
        let result = execute_effect(&mut game, &remove_effect, &mut ctx);
        assert!(result.is_ok(), "Counter removal should succeed");

        // Pay the life cost (2 life for phyrexian black)
        game.player_mut(alice).unwrap().life -= 2;

        // Verify saga now has 2 lore counters (not 3)
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2,
            "Saga should have 2 lore counters after Hex Parasite"
        );

        // Now the chapter 3 trigger resolves - search for artifact with MV 0 or 1
        // For this test, we'll manually resolve it
        // Create a decision maker that selects the ornithopter
        struct SelectOrnithopterDecisionMaker;
        impl DecisionMaker for SelectOrnithopterDecisionMaker {
            fn decide_objects(
                &mut self,
                game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Find ornithopter in candidates
                ctx.candidates
                    .iter()
                    .filter(|c| c.legal)
                    .find(|c| {
                        game.object(c.id)
                            .map(|o| o.name == "Ornithopter")
                            .unwrap_or(false)
                    })
                    .map(|c| vec![c.id])
                    .unwrap_or_default()
            }
        }

        let search_effect = Effect::search_library(
            crate::target::ObjectFilter {
                card_types: vec![CardType::Artifact],
                mana_value: Some(crate::target::Comparison::LessThanOrEqual(1)),
                ..Default::default()
            },
            Zone::Battlefield,
            crate::target::PlayerFilter::You,
            false,
        );
        let mut dm = SelectOrnithopterDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);
        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should succeed");

        // Simulate the final chapter ability resolving - this would mark the saga
        // But ONLY if it still has enough lore counters
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // The saga is marked as final_chapter_resolved, but it only has 2 counters
        assert!(
            game.is_saga_final_chapter_resolved(saga_id),
            "Saga should be marked as final chapter resolved"
        );
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2,
            "Saga should still have only 2 lore counters"
        );

        // Now check SBAs - the saga should NOT be sacrificed because it doesn't have
        // enough lore counters (need 3, only has 2)
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

        // Verify saga is STILL on the battlefield
        assert!(
            game.battlefield.contains(&saga_id),
            "Saga should STILL be on battlefield - it survived because lore counter was removed!"
        );

        // Verify Ornithopter is on the battlefield (it was fetched)
        let ornithopter_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Ornithopter")
                .unwrap_or(false)
        });
        assert!(
            ornithopter_on_battlefield,
            "Ornithopter should be on battlefield (fetched by Urza's Saga)"
        );

        // Verify Hex Parasite is still on battlefield
        assert!(
            game.battlefield.contains(&parasite_id),
            "Hex Parasite should still be on battlefield"
        );

        // Verify Alice paid 2 life
        assert_eq!(
            game.player(alice).unwrap().life,
            18,
            "Alice should have 18 life (paid 2 for Hex Parasite)"
        );

        // Final summary of board state
        println!("Board state after Hex Parasite saves Urza's Saga:");
        println!("- Urza's Saga: on battlefield with 2 lore counters");
        println!("- Hex Parasite: on battlefield");
        println!("- Ornithopter: on battlefield (fetched)");
        println!("- Alice's life: 18");
    }

    #[test]
    fn test_saga_chapter_triggers_again_after_counter_removed() {
        use crate::cards::definitions::urzas_saga;

        // Test scenario: Hex Parasite + Urza's Saga
        // 1. Urza's Saga has 2 lore counters
        // 2. Precombat main: lore counter added (now 3), Chapter III triggers
        // 3. In response: remove a lore counter (now 2)
        // 4. Chapter III resolves (saga survives because 2 < 3)
        // 5. NEXT TURN: lore counter added (now 3), Chapter III should trigger AGAIN
        // 6. Chapter III resolves, saga gets sacrificed
        //
        // This tests MTG Rule 714.2c: chapters can trigger multiple times if the
        // threshold is crossed multiple times (e.g., by removing and re-adding counters).

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let mut trigger_queue = TriggerQueue::new();

        // Put Urza's Saga on battlefield with 2 lore counters
        let saga_def = urzas_saga();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 2);

        // Set active player to Alice (needed for add_saga_lore_counters)
        game.turn.active_player = alice;

        // --- TURN 1: Precombat main phase ---
        // Add lore counter (2 -> 3), Chapter III triggers
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        assert_eq!(
            game.object(saga_id)
                .unwrap()
                .counters
                .get(&CounterType::Lore)
                .copied()
                .unwrap_or(0),
            3,
            "Turn 1: Saga should have 3 lore counters after precombat main"
        );
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Turn 1: Chapter III should have triggered"
        );

        // Simulate responding with Hex Parasite: remove 1 lore counter
        game.object_mut(saga_id)
            .unwrap()
            .remove_counters(CounterType::Lore, 1);

        assert_eq!(
            game.object(saga_id)
                .unwrap()
                .counters
                .get(&CounterType::Lore)
                .copied()
                .unwrap_or(0),
            2,
            "Turn 1: Saga should have 2 lore counters after Hex Parasite"
        );

        // Chapter III trigger resolves (the search effect)
        // For this test, we just clear the queue to simulate resolution
        trigger_queue.clear();

        // Mark final chapter as resolved
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // Check SBAs - saga should survive because 2 < 3
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
        assert!(
            game.battlefield.contains(&saga_id),
            "Turn 1: Saga should survive - only has 2 lore counters"
        );

        // --- TURN 2: Precombat main phase ---
        // Reset final_chapter_resolved for next turn's processing
        // (In a real game, this would be a new chapter trigger instance)
        game.clear_saga_final_chapter_resolved(saga_id);

        // Add lore counter (2 -> 3), Chapter III should trigger AGAIN!
        // This is the key test: the threshold crossing logic should allow re-triggering
        add_saga_lore_counters(&mut game, &mut trigger_queue);

        assert_eq!(
            game.object(saga_id)
                .unwrap()
                .counters
                .get(&CounterType::Lore)
                .copied()
                .unwrap_or(0),
            3,
            "Turn 2: Saga should have 3 lore counters"
        );
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "Turn 2: Chapter III should have triggered AGAIN (threshold crossed again)"
        );

        // Chapter III trigger resolves
        trigger_queue.clear();
        mark_saga_final_chapter_resolved(&mut game, saga_id);

        // Check SBAs - saga should now be sacrificed because 3 >= 3
        check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();
        assert!(
            !game.battlefield.contains(&saga_id),
            "Turn 2: Saga should be sacrificed - has 3 lore counters"
        );

        println!("Test passed: Chapter III triggered twice after counter manipulation!");
    }

    #[test]
    fn test_urzas_saga_excludes_x_cost_artifacts() {
        use crate::cards::definitions::{everflowing_chalice, ornithopter, urzas_saga};
        use crate::executor::execute_effect;
        use crate::target::FilterContext;

        // Test that Urza's Saga's search filter correctly excludes X-cost artifacts
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put Urza's Saga on battlefield
        let saga_def = urzas_saga();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

        // Put Everflowing Chalice in library (has X in its cost)
        let chalice_def = everflowing_chalice();
        let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Library);

        // Put Ornithopter in library (no X in cost, mana value 0)
        let ornithopter_def = ornithopter();
        let _ornithopter_id =
            game.create_object_from_definition(&ornithopter_def, alice, Zone::Library);

        // Create the filter from Urza's Saga chapter III
        let filter = crate::target::ObjectFilter {
            card_types: vec![CardType::Artifact],
            mana_value: Some(crate::target::Comparison::LessThanOrEqual(1)),
            has_mana_cost: true,
            no_x_in_cost: true,
            ..Default::default()
        };

        let ctx = FilterContext::new(alice).with_source(saga_id);

        // Verify Everflowing Chalice does NOT match (has X in cost)
        let chalice_obj = game.object(chalice_id).unwrap();
        assert!(
            !filter.matches(chalice_obj, &ctx, &game),
            "Everflowing Chalice should NOT match - has X in cost"
        );

        // Verify Ornithopter DOES match (mana value 0, no X, has mana cost)
        let ornithopter_obj = game
            .player(alice)
            .unwrap()
            .library
            .iter()
            .find_map(|&id| {
                let obj = game.object(id)?;
                if obj.name == "Ornithopter" {
                    Some(obj)
                } else {
                    None
                }
            })
            .unwrap();
        assert!(
            filter.matches(ornithopter_obj, &ctx, &game),
            "Ornithopter SHOULD match - mana value 0, no X, has mana cost"
        );

        // Now test the full search effect
        struct SelectFirstMatchDecisionMaker;
        impl DecisionMaker for SelectFirstMatchDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                ctx.candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id)
                    .take(1)
                    .collect()
            }
        }

        let search_effect = Effect::search_library(
            filter,
            Zone::Battlefield,
            crate::target::PlayerFilter::You,
            false,
        );

        let mut dm = SelectFirstMatchDecisionMaker;
        let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);
        let result = execute_effect(&mut game, &search_effect, &mut ctx);
        assert!(result.is_ok(), "Search should succeed");

        // Verify Ornithopter is on battlefield (should have been selected)
        let ornithopter_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Ornithopter")
                .unwrap_or(false)
        });
        assert!(
            ornithopter_on_battlefield,
            "Ornithopter should be on battlefield"
        );

        // Verify Everflowing Chalice is NOT on battlefield (should not have been searchable)
        let chalice_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Everflowing Chalice")
                .unwrap_or(false)
        });
        assert!(
            !chalice_on_battlefield,
            "Everflowing Chalice should NOT be on battlefield - has X in cost"
        );
    }

    #[test]
    fn test_hex_parasite_pump_effect() {
        use crate::cards::definitions::{hex_parasite, the_birth_of_meletis};
        use crate::executor::execute_effect;

        // Test that Hex Parasite gets +1/+0 for each counter removed
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put a saga on battlefield with some lore counters
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 2);

        // Put Hex Parasite on battlefield
        let parasite_def = hex_parasite();
        let parasite_id =
            game.create_object_from_definition(&parasite_def, alice, Zone::Battlefield);
        game.remove_summoning_sickness(parasite_id);

        // Verify initial state - Hex Parasite is 1/1
        let parasite = game.object(parasite_id).unwrap();
        assert_eq!(parasite.power().unwrap(), 1, "Hex Parasite base power is 1");
        assert_eq!(
            parasite.toughness().unwrap(),
            1,
            "Hex Parasite base toughness is 1"
        );

        // Execute the counter removal + pump effect sequence
        // First, remove 2 counters (X=2)
        let remove_effect = Effect::with_id(
            0,
            Effect::remove_counters(
                CounterType::Lore,
                2,
                crate::target::ChooseSpec::SpecificObject(saga_id),
            ),
        );

        let mut ctx = ExecutionContext::new_default(parasite_id, alice)
            .with_x(2)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(saga_id)]);
        let result = execute_effect(&mut game, &remove_effect, &mut ctx);
        assert!(result.is_ok(), "Counter removal should succeed");

        // Check that 2 counters were removed
        assert_eq!(
            result.unwrap().as_count().unwrap_or(0),
            2,
            "Should have removed 2 counters"
        );

        // Now execute the pump effect (which uses the stored result)
        let pump_effect = Effect::if_then(
            crate::effect::EffectId(0),
            crate::effect::EffectPredicate::Happened,
            vec![Effect::pump(
                Value::EffectValue(crate::effect::EffectId(0)),
                Value::Fixed(0),
                crate::target::ChooseSpec::Source,
                crate::effect::Until::EndOfTurn,
            )],
        );

        let result = execute_effect(&mut game, &pump_effect, &mut ctx);
        assert!(result.is_ok(), "Pump effect should succeed");

        // Verify the continuous effect was added
        let effects = game.continuous_effects.effects_for_object(parasite_id);
        assert!(
            !effects.is_empty(),
            "Should have a continuous effect on Hex Parasite"
        );

        // Verify the effect is a +2/+0 modification
        let pump_effect = effects.iter().find(|e| {
            matches!(
                &e.modification,
                crate::continuous::Modification::ModifyPowerToughness {
                    power: 2,
                    toughness: 0
                }
            )
        });
        assert!(
            pump_effect.is_some(),
            "Should have a +2/+0 continuous effect"
        );
    }

    #[test]
    fn test_remove_up_to_counters_player_choice() {
        use crate::cards::definitions::the_birth_of_meletis;
        use crate::executor::execute_effect;

        // Test that RemoveUpToCounters allows player to choose how many counters to remove
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put a saga on battlefield with 3 lore counters
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 3);

        // Create a decision maker that chooses to remove only 1 counter (not the max)
        struct ChooseOneDecisionMaker;
        impl DecisionMaker for ChooseOneDecisionMaker {
            fn decide_number(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                // Verify the range is correct (0 to 3, since X=5 but only 3 available)
                assert_eq!(ctx.min, 0, "Min should be 0 for 'up to' effect");
                assert_eq!(ctx.max, 3, "Max should be 3 (number available)");
                // Choose to remove only 1 counter
                1
            }
        }

        let source_id = game.new_object_id();
        let mut dm = ChooseOneDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_x(5) // Pay X=5, but only 3 counters available
            .with_targets(vec![crate::executor::ResolvedTarget::Object(saga_id)])
            .with_decision_maker(&mut dm);

        // Use RemoveUpToCounters - player should be able to choose 0-3
        let effect = Effect::remove_up_to_counters(
            CounterType::Lore,
            Value::X,
            crate::target::ChooseSpec::SpecificObject(saga_id),
        );

        let result = execute_effect(&mut game, &effect, &mut ctx);
        assert!(result.is_ok(), "Effect should succeed");

        // Verify only 1 counter was removed (player's choice)
        let removed = result.unwrap().as_count().unwrap_or(0);
        assert_eq!(
            removed, 1,
            "Should have removed exactly 1 counter (player's choice)"
        );

        // Verify saga still has 2 lore counters
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2,
            "Saga should have 2 lore counters remaining"
        );
    }

    #[test]
    fn test_remove_up_to_counters_choose_zero() {
        use crate::cards::definitions::the_birth_of_meletis;
        use crate::executor::execute_effect;

        // Test that player can choose to remove 0 counters with "up to" effect
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Put a saga on battlefield with 2 lore counters
        let saga_def = the_birth_of_meletis();
        let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
        game.object_mut(saga_id)
            .unwrap()
            .add_counters(CounterType::Lore, 2);

        // Create a decision maker that chooses to remove 0 counters
        struct ChooseZeroDecisionMaker;
        impl DecisionMaker for ChooseZeroDecisionMaker {
            fn decide_number(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                // Choose to remove 0 counters
                0
            }
        }

        let source_id = game.new_object_id();
        let mut dm = ChooseZeroDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_x(3)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(saga_id)])
            .with_decision_maker(&mut dm);

        let effect = Effect::remove_up_to_counters(
            CounterType::Lore,
            Value::X,
            crate::target::ChooseSpec::SpecificObject(saga_id),
        );

        let result = execute_effect(&mut game, &effect, &mut ctx);
        assert!(result.is_ok(), "Effect should succeed");

        // Verify 0 counters were removed
        let removed = result.unwrap().as_count().unwrap_or(-1);
        assert_eq!(
            removed, 0,
            "Should have removed 0 counters (player's choice)"
        );

        // Verify saga still has all 2 lore counters
        let saga = game.object(saga_id).unwrap();
        assert_eq!(
            saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
            2,
            "Saga should still have all 2 lore counters"
        );
    }

    #[test]
    fn test_remove_up_to_any_counters_multiple_types() {
        use crate::executor::execute_effect;

        // Test that RemoveUpToAnyCounters works with multiple counter types
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a creature with multiple types of counters
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(999), "Test Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
        let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        // Add multiple types of counters
        game.object_mut(creature_id)
            .unwrap()
            .add_counters(CounterType::PlusOnePlusOne, 3);
        game.object_mut(creature_id)
            .unwrap()
            .add_counters(CounterType::Charge, 2);

        // Verify initial state: 5 total counters
        let creature = game.object(creature_id).unwrap();
        assert_eq!(
            creature
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            3
        );
        assert_eq!(
            creature
                .counters
                .get(&CounterType::Charge)
                .copied()
                .unwrap_or(0),
            2
        );

        // Create a decision maker that chooses to remove 4 counters (2 Charge + 2 +1/+1)
        struct ChooseFourDecisionMaker;
        impl DecisionMaker for ChooseFourDecisionMaker {
            fn decide_counters(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::CountersContext,
            ) -> Vec<(CounterType, u32)> {
                // max_total is capped to min(X, total_available_counters) = min(10, 5) = 5
                assert_eq!(
                    ctx.max_total, 5,
                    "Max should be capped to available counters"
                );
                assert_eq!(
                    ctx.available_counters.len(),
                    2,
                    "Should have 2 counter types"
                );
                // Choose to remove 2 Charge and 2 +1/+1 = 4 total
                vec![(CounterType::Charge, 2), (CounterType::PlusOnePlusOne, 2)]
            }
        }

        let source_id = game.new_object_id();
        let mut dm = ChooseFourDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_x(10) // X=10, but only 5 counters available
            .with_targets(vec![crate::executor::ResolvedTarget::Object(creature_id)])
            .with_decision_maker(&mut dm);

        let effect = Effect::remove_up_to_any_counters(
            Value::X,
            crate::target::ChooseSpec::SpecificObject(creature_id),
        );

        let result = execute_effect(&mut game, &effect, &mut ctx);
        assert!(result.is_ok(), "Effect should succeed");

        // Verify 4 counters were removed
        let removed = result.unwrap().as_count().unwrap_or(0);
        assert_eq!(removed, 4, "Should have removed 4 counters");

        // Verify final state: 1 +1/+1 counter remaining, 0 Charge remaining
        // (We chose to remove 2 Charge and 2 +1/+1)
        let creature = game.object(creature_id).unwrap();
        let charge_remaining = creature
            .counters
            .get(&CounterType::Charge)
            .copied()
            .unwrap_or(0);
        let plus_remaining = creature
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0);

        assert_eq!(charge_remaining, 0, "All Charge counters should be removed");
        assert_eq!(plus_remaining, 1, "Should have 1 +1/+1 counter remaining");
    }

    #[test]
    fn test_hex_parasite_removes_loyalty_counters() {
        use crate::executor::execute_effect;

        // Test that Hex Parasite can remove loyalty counters from a planeswalker
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create a planeswalker with loyalty counters
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(998), "Test Planeswalker")
                .card_types(vec![CardType::Planeswalker])
                .build();
        let pw_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        // Add loyalty counters
        game.object_mut(pw_id)
            .unwrap()
            .add_counters(CounterType::Loyalty, 4);

        // Create a decision maker that removes 2 loyalty counters
        struct ChooseTwoDecisionMaker;
        impl DecisionMaker for ChooseTwoDecisionMaker {
            fn decide_counters(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::CountersContext,
            ) -> Vec<(CounterType, u32)> {
                assert_eq!(
                    ctx.available_counters.len(),
                    1,
                    "Should only have Loyalty counters"
                );
                assert_eq!(ctx.available_counters[0].0, CounterType::Loyalty);
                // Choose to remove 2 Loyalty counters
                vec![(CounterType::Loyalty, 2)]
            }
        }

        let source_id = game.new_object_id();
        let mut dm = ChooseTwoDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_x(5)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(pw_id)])
            .with_decision_maker(&mut dm);

        // Use the same effect Hex Parasite uses
        let effect = Effect::remove_up_to_any_counters(
            Value::X,
            crate::target::ChooseSpec::SpecificObject(pw_id),
        );

        let result = execute_effect(&mut game, &effect, &mut ctx);
        assert!(result.is_ok(), "Effect should succeed");

        // Verify 2 loyalty counters were removed
        let removed = result.unwrap().as_count().unwrap_or(0);
        assert_eq!(removed, 2, "Should have removed 2 counters");

        // Verify planeswalker has 2 loyalty remaining
        let pw = game.object(pw_id).unwrap();
        assert_eq!(
            pw.counters.get(&CounterType::Loyalty).copied().unwrap_or(0),
            2,
            "Planeswalker should have 2 loyalty remaining"
        );
    }

    // ========================================================================
    // Valley Floodcaller Tests
    // ========================================================================

    #[test]
    fn test_valley_floodcaller_grants_flash_to_sorceries() {
        use crate::cards::definitions::valley_floodcaller;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice enough mana
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Blue, 5);

        // Create Valley Floodcaller on battlefield
        let floodcaller_def = valley_floodcaller();
        let _floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create a sorcery in Alice's hand
        let sorcery = CardBuilder::new(CardId::from_raw(100), "Test Sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Blue,
            ]]))
            .build();
        let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

        // Check that the sorcery has been granted flash
        let flash_ability = crate::static_abilities::StaticAbility::flash();
        let has_granted_flash = game.grant_registry.card_has_granted_ability(
            &game,
            sorcery_id,
            Zone::Hand,
            alice,
            &flash_ability,
        );
        assert!(
            has_granted_flash,
            "Valley Floodcaller should grant flash to sorceries in hand"
        );
    }

    #[test]
    fn test_valley_floodcaller_does_not_grant_flash_to_creatures() {
        use crate::cards::definitions::valley_floodcaller;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Valley Floodcaller on battlefield
        let floodcaller_def = valley_floodcaller();
        let _floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create a creature in Alice's hand
        let creature = CardBuilder::new(CardId::from_raw(100), "Test Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);

        // Check that the creature has NOT been granted flash
        let flash_ability = crate::static_abilities::StaticAbility::flash();
        let has_granted_flash = game.grant_registry.card_has_granted_ability(
            &game,
            creature_id,
            Zone::Hand,
            alice,
            &flash_ability,
        );
        assert!(
            !has_granted_flash,
            "Valley Floodcaller should NOT grant flash to creatures in hand"
        );
    }

    #[test]
    fn test_valley_floodcaller_flash_grant_removed_when_floodcaller_leaves() {
        use crate::cards::definitions::valley_floodcaller;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Valley Floodcaller on battlefield
        let floodcaller_def = valley_floodcaller();
        let floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create a sorcery in Alice's hand
        let sorcery = CardBuilder::new(CardId::from_raw(100), "Test Sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Blue,
            ]]))
            .build();
        let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

        let flash_ability = crate::static_abilities::StaticAbility::flash();

        // Verify sorcery has flash while Floodcaller is on battlefield
        assert!(
            game.grant_registry.card_has_granted_ability(
                &game,
                sorcery_id,
                Zone::Hand,
                alice,
                &flash_ability,
            ),
            "Sorcery should have flash while Floodcaller is on battlefield"
        );

        // Remove Floodcaller from battlefield
        game.move_object(floodcaller_id, Zone::Graveyard);

        // Verify sorcery no longer has flash
        assert!(
            !game.grant_registry.card_has_granted_ability(
                &game,
                sorcery_id,
                Zone::Hand,
                alice,
                &flash_ability,
            ),
            "Sorcery should NOT have flash after Floodcaller leaves battlefield"
        );
    }

    #[test]
    fn test_valley_floodcaller_sorcery_castable_during_combat() {
        use crate::cards::definitions::valley_floodcaller;
        use crate::decision::compute_legal_actions;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice mana
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Blue, 5);

        // Create Valley Floodcaller on battlefield
        let floodcaller_def = valley_floodcaller();
        let _floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create a sorcery in Alice's hand
        let sorcery = CardBuilder::new(CardId::from_raw(100), "Draw Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Blue,
            ]]))
            .build();
        let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

        // Set to combat phase (not main phase)
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);

        // Check that the sorcery can be cast during combat (has flash)
        let actions = compute_legal_actions(&game, alice);
        let can_cast_sorcery = actions.iter().any(|a| {
            matches!(
                a,
                LegalAction::CastSpell { spell_id, .. } if *spell_id == sorcery_id
            )
        });

        assert!(
            can_cast_sorcery,
            "Should be able to cast sorcery during combat thanks to Valley Floodcaller granting flash"
        );
    }

    #[test]
    fn test_valley_floodcaller_only_grants_to_controller() {
        use crate::cards::definitions::valley_floodcaller;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Valley Floodcaller on Alice's battlefield
        let floodcaller_def = valley_floodcaller();
        let _floodcaller_id =
            game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

        // Create sorceries in both players' hands
        let alice_sorcery = CardBuilder::new(CardId::from_raw(100), "Alice Sorcery")
            .card_types(vec![CardType::Sorcery])
            .build();
        let alice_sorcery_id = game.create_object_from_card(&alice_sorcery, alice, Zone::Hand);

        let bob_sorcery = CardBuilder::new(CardId::from_raw(101), "Bob Sorcery")
            .card_types(vec![CardType::Sorcery])
            .build();
        let bob_sorcery_id = game.create_object_from_card(&bob_sorcery, bob, Zone::Hand);

        let flash_ability = crate::static_abilities::StaticAbility::flash();

        // Alice's sorcery should have flash
        assert!(
            game.grant_registry.card_has_granted_ability(
                &game,
                alice_sorcery_id,
                Zone::Hand,
                alice,
                &flash_ability,
            ),
            "Alice's sorcery should have flash from her Floodcaller"
        );

        // Bob's sorcery should NOT have flash (Alice's Floodcaller doesn't grant to opponents)
        assert!(
            !game.grant_registry.card_has_granted_ability(
                &game,
                bob_sorcery_id,
                Zone::Hand,
                bob,
                &flash_ability,
            ),
            "Bob's sorcery should NOT have flash from Alice's Floodcaller"
        );
    }
}
