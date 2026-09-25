//! Frozen atlas observation 24458188: both canonical faces and waterbend.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::CardId;
use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, PlayerId, Zone};
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_payloads_by_name,
};

fn definitions() -> Vec<CardDefinition> {
    let payloads = load_card_payloads_by_name(
        default_cards_path().to_str().unwrap(),
        "Aang, Swift Savior // Aang and La, Ocean's Fury",
    )
    .unwrap();
    assert_eq!(payloads.len(), 2);
    payloads
        .iter()
        .map(|payload| {
            let snapshot = compile_authoritative_snapshot_from_payload(payload);
            assert_eq!(
                snapshot.parse_status,
                ParseStatus::StrictCompiled,
                "{snapshot:#?}"
            );
            assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
            compile_definition_from_payload(payload).unwrap()
        })
        .collect()
}

#[test]
fn waterbend_eight_preserves_every_mana_and_tap_payment_branch() {
    use ironsmith::mana::ManaSymbol;
    let defs = definitions();
    let front = defs
        .iter()
        .find(|d| d.card.name == "Aang, Swift Savior")
        .unwrap();
    let activated = front
        .abilities
        .iter()
        .find_map(|a| {
            if let AbilityKind::Activated(a) = &a.kind {
                Some(a)
            } else {
                None
            }
        })
        .unwrap();
    let branches = activated
        .mana_cost
        .as_one_of()
        .expect("waterbend alternatives");
    assert_eq!(branches.len(), 9);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for taps in 0..=8 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for d in &defs {
            game.register_linked_face_definition(d);
        }
        let source = game.create_object_from_definition(front, alice, Zone::Battlefield);
        let mut eligible = vec![source];
        for n in 0..7 {
            let fixture =
                CardDefinitionBuilder::new(CardId::new(), format!("Waterbend material {n}"))
                    .card_types(vec![if n % 2 == 0 {
                        CardType::Artifact
                    } else {
                        CardType::Creature
                    }])
                    .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1))
                    .build();
            eligible.push(game.create_object_from_definition(&fixture, alice, Zone::Battlefield));
        }
        let decoy = CardDefinitionBuilder::new(CardId::new(), "Waterbend opponent material")
            .card_types(vec![CardType::Artifact])
            .build();
        let opponent = game.create_object_from_definition(&decoy, bob, Zone::Battlefield);
        let land = CardDefinitionBuilder::new(CardId::new(), "Waterbend plain land")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, (8 - taps) as u32);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ironsmith::costs::CostContext::new(source, alice, &mut dm);
        for cost in branches[taps].costs() {
            cost.pay(&mut game, &mut ctx)
                .expect("exact waterbend resources suffice");
        }
        assert_eq!(
            eligible.iter().filter(|id| game.is_tapped(**id)).count(),
            taps
        );
        assert!(!game.is_tapped(opponent) && !game.is_tapped(land));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(
            game.object(source).unwrap().name,
            "Aang, Swift Savior",
            "paying does not itself resolve transformation"
        );
        assert!(game.transform_permanent(source));
        assert_eq!(
            game.object(source).unwrap().name,
            "Aang and La, Ocean's Fury"
        );
    }
}

#[test]
fn waterbend_activation_uses_stack_and_transforms_only_its_source() {
    use ironsmith::decision::{GameProgress, LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::triggers::TriggerQueue;
    let defs = definitions();
    let front = defs
        .iter()
        .find(|d| d.card.name == "Aang, Swift Savior")
        .unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for leave_and_return in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for d in &defs {
            game.register_linked_face_definition(d);
        }
        game.turn.active_player = bob;
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        game.turn.priority_player = Some(alice);
        let source = game.create_object_from_definition(front, alice, Zone::Battlefield);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 8);
        let action = compute_legal_actions(&game, alice)
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id==source))
            .unwrap();
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..24 {
            if !game.stack.is_empty() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}");
            };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &ctx, &mut dm,
            )
            .unwrap();
        }
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object(source).unwrap().name, "Aang, Swift Savior");
        let retained = if leave_and_return {
            let hand = game.move_object_by_effect(source, Zone::Hand).unwrap();
            game.move_object_by_effect(hand, Zone::Battlefield).unwrap()
        } else {
            source
        };
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.object(retained).unwrap().name,
            if leave_and_return {
                "Aang, Swift Savior"
            } else {
                "Aang and La, Ocean's Fury"
            }
        );
    }
}

#[test]
fn ocean_attack_counts_tapped_creatures_at_resolution_for_trigger_controller() {
    use ironsmith::events::CreatureAttackedEvent;
    use ironsmith::object::CounterType;
    use ironsmith::triggers::event::AttackEventTarget;
    use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
    let defs = definitions();
    let back = defs
        .iter()
        .find(|d| d.card.name == "Aang and La, Ocean's Fury")
        .unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for depart in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(back, alice, Zone::Battlefield);
        let fixture = CardDefinitionBuilder::new(CardId::new(), "Ocean creature fixture")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .build();
        let untaps = game.create_object_from_definition(&fixture, alice, Zone::Battlefield);
        let taps = game.create_object_from_definition(&fixture, alice, Zone::Battlefield);
        let stolen = game.create_object_from_definition(&fixture, bob, Zone::Battlefield);
        game.set_current_controller(stolen, alice);
        let opponent = game.create_object_from_definition(&fixture, bob, Zone::Battlefield);
        let artifact = CardDefinitionBuilder::new(CardId::new(), "Ocean artifact fixture")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
        for id in [source, untaps, stolen, opponent, artifact] {
            game.tap(id);
        }
        let attack = |id| {
            TriggerEvent::new_with_provenance(
                CreatureAttackedEvent::new(id, AttackEventTarget::Player(bob)),
                ironsmith::provenance::ProvNodeId::default(),
            )
        };
        assert!(
            check_triggers(&game, &attack(taps)).is_empty(),
            "another creature attacking does not trigger it"
        );
        let mut queue = TriggerQueue::new();
        for entry in check_triggers(&game, &attack(source)) {
            queue.add(entry);
        }
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        game.untap(untaps);
        game.tap(taps);
        if depart {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        } else {
            game.set_current_controller(source, bob);
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        for (id, expected) in [
            (untaps, 0),
            (taps, 1),
            (stolen, 1),
            (opponent, 0),
            (artifact, 0),
        ] {
            assert_eq!(
                game.object(id)
                    .unwrap()
                    .counters
                    .get(&CounterType::PlusOnePlusOne)
                    .copied()
                    .unwrap_or(0),
                expected
            );
        }
        if !depart {
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&CounterType::PlusOnePlusOne)
                    .copied()
                    .unwrap_or(0),
                0
            );
        }
    }
}

#[test]
fn swift_airbend_can_decline_or_exile_a_creature_or_noncreature_spell() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::game_state::{StackEntry, Target};
    use ironsmith::triggers::{TriggerQueue, check_triggers};
    struct Pick(Option<ironsmith::ObjectId>);
    impl DecisionMaker for Pick {
        fn decide_targets(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.0.map(Target::Object).into_iter().collect()
        }
    }
    let defs = definitions();
    let front = defs
        .iter()
        .find(|d| d.card.name == "Aang, Swift Savior")
        .unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for mode in 0..4 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut fixture =
            CardDefinitionBuilder::new(CardId::new(), "Airbend spell or creature fixture")
                .card_types(vec![if mode >= 2 {
                    CardType::Instant
                } else {
                    CardType::Creature
                }])
                .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
                .build();
        if mode == 3 {
            fixture.abilities.push(ironsmith::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::uncounterable(),
            ));
        }
        let target = game.create_object_from_definition(
            &fixture,
            bob,
            if mode >= 2 {
                Zone::Stack
            } else {
                Zone::Battlefield
            },
        );
        let stable = game.object(target).unwrap().stable_id;
        if mode >= 2 {
            game.push_to_stack(StackEntry::new(target, bob));
        }
        let hand = game.create_object_from_definition(front, alice, Zone::Hand);
        let source = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut game,
            &mut queue,
            &mut Pick((mode != 0).then_some(target)),
        )
        .unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let retained = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(retained).unwrap().zone,
            if mode == 0 {
                Zone::Battlefield
            } else {
                Zone::Exile
            }
        );
        assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.effect_store
                .grant_registry
                .granted_alternative_casts_for_card(&game, retained, Zone::Exile, bob)
                .len(),
            usize::from(mode != 0)
        );
        if mode >= 2 {
            assert!(
                game.stack.is_empty(),
                "the exiled spell cannot remain on the stack"
            );
        }
    }
}

#[test]
fn swift_flash_and_face_combat_keywords_change_legal_actions() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    let defs = definitions();
    let front = defs
        .iter()
        .find(|d| d.card.name == "Aang, Swift Savior")
        .unwrap();
    let back = defs
        .iter()
        .find(|d| d.card.name == "Aang and La, Ocean's Fury")
        .unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let hand = game.create_object_from_definition(front, alice, Zone::Hand);
    let normal = CardDefinitionBuilder::new(CardId::new(), "Ordinary creature timing fixture")
        .card_types(vec![CardType::Creature])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
            ManaSymbol::Generic(1),
        ]))
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build();
    let normal_hand = game.create_object_from_definition(&normal, alice, Zone::Hand);
    game.turn.active_player = bob;
    game.turn.phase = ironsmith::game_state::Phase::Combat;
    game.turn.priority_player = Some(alice);
    for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Colorless] {
        game.player_mut(alice).unwrap().mana_pool.add(symbol, 1);
    }
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==hand))
    );
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==normal_hand))
    );
    let flying = game.create_object_from_definition(front, bob, Zone::Battlefield);
    let grounded = game.create_object_from_definition(&normal, alice, Zone::Battlefield);
    let reach = game.create_object_from_definition(back, alice, Zone::Battlefield);
    assert!(!ironsmith::rules::combat::can_block(
        game.object(flying).unwrap(),
        game.object(grounded).unwrap(),
        &game
    ));
    assert!(ironsmith::rules::combat::can_block(
        game.object(flying).unwrap(),
        game.object(reach).unwrap(),
        &game
    ));
    assert!(
        game.object(reach)
            .unwrap()
            .has_static_ability_id(ironsmith::static_abilities::StaticAbilityId::Trample)
    );
}

#[test]
fn ocean_trample_deals_only_damage_beyond_lethal_to_defending_player() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    let defs = definitions();
    let back = defs
        .iter()
        .find(|d| d.card.name == "Aang and La, Ocean's Fury")
        .unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for toughness in [2, 5, 7] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(back, alice, Zone::Battlefield);
        let blocker = CardDefinitionBuilder::new(CardId::new(), "Ocean trample blocker")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(1, toughness))
            .build();
        let blocker = game.create_object_from_definition(&blocker, bob, Zone::Battlefield);
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(source, vec![blocker]);
        ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(game.damage_on(blocker), std::cmp::min(5, toughness) as u32);
        assert_eq!(game.damage_on(source), 1);
        assert_eq!(game.player(bob).unwrap().life, 20 - (5 - toughness).max(0));
    }
}
