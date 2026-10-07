//! Authored, unrun regressions for the frozen mixed opponent/object target
//! cohort. Both entry paths compile the complete Oracle body. The artifact
//! path also serializes and materializes the definition before gameplay.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Target;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CardId, CardType, CounterType, GameState, ObjectId, PlayerId, Zone};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const ALL_WILL: &str = "All Will Be One";
const BOLAS: &str = "Nicol Bolas, God-Pharaoh";

fn payload(name: &str) -> ironsmith_tools::CardPayload {
    // Exact complete bodies from cards-20261003.json.xz; self-contained so
    // deferred execution does not depend on a mutable cards.json export.
    let (metadata, oracle) = match name {
        ALL_WILL => (
            "Mana cost: {3}{R}{R}\nType: Enchantment",
            "Whenever you put one or more counters on a permanent or player, this enchantment deals that much damage to target opponent, creature an opponent controls, or planeswalker an opponent controls.",
        ),
        BOLAS => (
            "Mana cost: {4}{U}{B}{R}\nType: Legendary Planeswalker — Bolas\nLoyalty: 7",
            "+2: Target opponent exiles cards from the top of their library until they exile a nonland card. Until end of turn, you may cast that card without paying its mana cost.\n+1: Each opponent exiles two cards from their hand.\n−4: Nicol Bolas deals 7 damage to target opponent, creature an opponent controls, or planeswalker an opponent controls.\n−12: Exile each nonland permanent your opponents control.",
        ),
        _ => panic!("unknown fixture"),
    };
    ironsmith_tools::CardPayload {
        name: name.into(), parse_name: None, oracle_text: oracle.into(),
        raw_oracle_text: oracle.into(), metadata_lines: metadata.lines().map(str::to_string).collect(),
        parse_input: format!("{metadata}\n{oracle}"), other_face_name: None, linked_face_layout: None,
    }
}

fn definition(name: &str, artifact: bool) -> CardDefinition {
    let payload = payload(name);
    if !artifact {
        return ironsmith_tools::compile_definition_from_payload(&payload).unwrap();
    }
    let compiled = ironsmith_compiler::CompilerFacade::new().compile_definition(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
        payload.parse_input,
        ironsmith_compiler::CompilePolicy { allow_unsupported: false },
    ).unwrap();
    let wire = serde_json::from_value(serde_json::to_value(&compiled.definition).unwrap()).unwrap();
    ironsmith::artifact_materializer::materialize_definition(wire).unwrap()
}

fn new_game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game
}

fn fixture(game: &mut GameState, player: PlayerId, kind: CardType, zone: Zone) -> ObjectId {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Mixed target witness")
        .card_types(vec![kind]);
    if kind == CardType::Creature {
        builder = builder.power_toughness(ironsmith::card::PowerToughness::fixed(1, 20));
    }
    if kind == CardType::Planeswalker { builder = builder.loyalty(20); }
    if kind == CardType::Sorcery {
        builder = builder.mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(5)]]));
    }
    let id = game.create_object_from_definition(&builder.build(), player, zone);
    if kind == CardType::Planeswalker { game.add_counters(id, CounterType::Loyalty, 20); }
    id
}

struct Pick {
    chosen: Target,
    expected: Vec<Target>,
    prompts: usize,
}
impl DecisionMaker for Pick {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.prompts += 1;
        assert_eq!(ctx.requirements.len(), 1, "the mixed domain is one target slot");
        let requirement = &ctx.requirements[0];
        assert_eq!((requirement.min_targets, requirement.max_targets), (1, Some(1)));
        assert_eq!(requirement.legal_targets.len(), self.expected.len());
        assert!(self.expected.iter().all(|target| requirement.legal_targets.contains(target)), "{ctx:#?}");
        assert!(requirement.legal_targets.contains(&self.chosen));
        vec![self.chosen]
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
}

fn target_fixture(game: &mut GameState) -> (Vec<Target>, [Target; 3]) {
    let mut legal = vec![Target::Player(B), Target::Player(C)];
    let mut bob = Vec::new();
    for player in [A, B, C] {
        for kind in [CardType::Creature, CardType::Planeswalker, CardType::Artifact, CardType::Battle, CardType::Land] {
            let id = fixture(game, player, kind, Zone::Battlefield);
            if player != A && matches!(kind, CardType::Creature | CardType::Planeswalker) {
                legal.push(Target::Object(id));
                if player == B { bob.push(Target::Object(id)); }
            }
        }
        fixture(game, player, CardType::Creature, Zone::Hand);
        fixture(game, player, CardType::Planeswalker, Zone::Graveyard);
    }
    // The card is owned by an opponent but controlled by Alice: ownership
    // must not make it an eligible opponent-controlled permanent.
    let borrowed = fixture(game, B, CardType::Creature, Zone::Battlefield);
    game.set_current_controller(borrowed, A).unwrap();
    let donated = fixture(game, A, CardType::Creature, Zone::Battlefield);
    game.set_current_controller(donated, B).unwrap();
    legal.push(Target::Object(donated));
    (legal, [Target::Player(B), bob[0], bob[1]])
}

fn marker_event(source: ObjectId, recipient: Target, actor: PlayerId, removed: bool) -> TriggerEvent {
    let location = match recipient {
        Target::Object(id) => ironsmith::marker::MarkerLocation::Object(id),
        Target::Player(id) => ironsmith::marker::MarkerLocation::Player(id),
    };
    let event = if removed {
        ironsmith::events::MarkersChangedEvent::removed(CounterType::Charge, location, 4, Some(source), Some(actor))
    } else {
        ironsmith::events::MarkersChangedEvent::added(CounterType::Charge, location, 4, Some(source), Some(actor))
    };
    TriggerEvent::new(event, Default::default())
}

fn assert_damage(game: &GameState, target: Target, amount: u32, original_loyalty: u32) {
    assert_eq!(game.player(A).unwrap().life, 20);
    assert_eq!(game.player(C).unwrap().life, 20);
    assert_eq!(game.player(B).unwrap().life, if target == Target::Player(B) { 20 - amount as i32 } else { 20 });
    if let Target::Object(id) = target {
        if game.object(id).is_some_and(|object| object.card_types.contains(&CardType::Planeswalker)) {
            assert_eq!(game.counter_count(id, CounterType::Loyalty), original_loyalty - amount);
        } else {
            assert_eq!(game.damage_on(id), amount);
        }
    }
}

#[test]
fn complete_frozen_bodies_compile_strictly_without_lost_abilities() {
    for name in [ALL_WILL, BOLAS] {
        let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload(name));
        assert_eq!(snapshot.parse_status, ironsmith_tools::ParseStatus::StrictCompiled, "{snapshot:#?}");
        assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented && snapshot.parse_error.is_none());
        for artifact in [false, true] {
            let definition = definition(name, artifact);
            let relevant = definition.abilities.iter().filter(|ability| matches!(&ability.kind,
                AbilityKind::Triggered(_) | AbilityKind::Activated(_))).count();
            assert_eq!(relevant, if name == ALL_WILL { 1 } else { 4 });
        }
    }
}

#[test]
fn complete_bodies_cannot_hide_an_unknown_tail_after_the_mixed_target() {
    for name in [ALL_WILL, BOLAS] {
        for tail in ["unrecognized qualifier", "with \"unrecognized ability\""] {
            let payload = payload(name);
            let input = payload.parse_input.replace(
                "or planeswalker an opponent controls.",
                &format!("or planeswalker an opponent controls {tail}."),
            );
            let result = ironsmith_compiler::CompilerFacade::new().compile_definition(
                ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
                input,
                ironsmith_compiler::CompilePolicy { allow_unsupported: false },
            );
            assert!(result.is_err(), "the complete body must retain and reject {tail}: {name}");
        }
    }
}

#[test]
fn all_will_be_one_retains_actor_event_amount_and_one_saved_target() {
    for artifact in [false, true] {
        let definition = definition(ALL_WILL, artifact);
        for recipient_is_player in [false, true] {
            for target_index in 0..3 {
                for change in 0..5 {
                    if target_index == 0 && matches!(change, 2 | 3) { continue; }
                    let mut game = new_game();
                    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                    let (legal, targets) = target_fixture(&mut game);
                    let chosen = targets[target_index];
                    let chosen_stable = match chosen {
                        Target::Object(id) => Some(game.object(id).unwrap().stable_id),
                        Target::Player(_) => None,
                    };
                    let sentinels = legal.iter().filter_map(|target| match target {
                        Target::Object(id) if *target != chosen => Some((
                            *id, game.damage_on(*id), game.counter_count(*id, CounterType::Loyalty),
                        )),
                        _ => None,
                    }).collect::<Vec<_>>();
                    let loyalty = if let Target::Object(id) = chosen { game.counter_count(id, CounterType::Loyalty) } else { 0 };
                    let recipient = if recipient_is_player { Target::Player(C) } else { targets[1] };
                    assert!(check_triggers(&game, &marker_event(source, recipient, B, false)).is_empty(), "opponent's placements do not trigger your enchantment");
                    assert!(check_triggers(&game, &marker_event(source, recipient, A, true)).is_empty(), "removal is not placement");
                    let event = marker_event(source, recipient, A, false);
                    let mut queue = TriggerQueue::new();
                    for trigger in check_triggers(&game, &event) { queue.add(trigger); }
                    assert_eq!(queue.entries.len(), 1, "four counters produce one trigger");
                    let mut dm = Pick { chosen, expected: legal, prompts: 0 };
                    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                    assert_eq!(game.stack.last().unwrap().targets, vec![chosen]);
                    match change {
                        1 => { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                        2 => if let Target::Object(id) = chosen { game.set_current_controller(id, A).unwrap(); },
                        3 => if let Target::Object(id) = chosen { game.move_object_by_effect(id, Zone::Graveyard).unwrap(); },
                        4 => { game.set_current_controller(source, B).unwrap(); }
                        _ => {}
                    }
                    resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert_eq!(dm.prompts, 1, "resolution must not choose a replacement target");
                    if change == 3 {
                        let moved = game.find_object_by_stable_id(chosen_stable.unwrap()).unwrap();
                        assert_ne!(Some(moved), match chosen { Target::Object(id) => Some(id), _ => None });
                        assert_eq!(game.object(moved).unwrap().zone, Zone::Graveyard);
                        for player in [A, B, C] { assert_eq!(game.player(player).unwrap().life, 20); }
                    } else {
                        assert_damage(&game, chosen, if change == 2 { 0 } else { 4 }, loyalty);
                    }
                    for (id, damage, counters) in sentinels {
                        assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield);
                        assert_eq!(game.damage_on(id), damage, "no replacement creature may be hit");
                        assert_eq!(game.counter_count(id, CounterType::Loyalty), counters,
                            "no replacement planeswalker may be hit");
                    }
                    assert!(game.stack.is_empty());
                }
            }
        }
    }
}

fn activate(game: &mut GameState, source: ObjectId, ordinal: usize, dm: &mut impl DecisionMaker) {
    let index = game.object(source).unwrap().abilities.iter().enumerate()
        .filter(|(_, ability)| matches!(&ability.kind, AbilityKind::Activated(a) if a.is_loyalty_ability()))
        .nth(ordinal).unwrap().0;
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(
        action, LegalAction::ActivateAbility { source: id, ability_index, .. } if *id == source && *ability_index == index
    )).expect("the loyalty ability is legal");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm,
    ).unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() && state.pending_activation.is_none() { return; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:#?}"); };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("activation did not finish");
}

fn bolas(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.add_counters(source, CounterType::Loyalty, 20);
    source
}

#[test]
fn bolas_minus_four_pays_loyalty_and_keeps_one_of_three_target_domains() {
    for artifact in [false, true] {
        let definition = definition(BOLAS, artifact);
        for target_index in 0..3 {
            for invalidate in [false, true] {
                if target_index == 0 && invalidate { continue; }
                let mut game = new_game();
                let source = bolas(&mut game, &definition);
                let old_loyalty = game.counter_count(source, CounterType::Loyalty);
                let (legal, targets) = target_fixture(&mut game);
                let chosen = targets[target_index];
                let loyalty = if let Target::Object(id) = chosen { game.counter_count(id, CounterType::Loyalty) } else { 0 };
                let mut dm = Pick { chosen, expected: legal, prompts: 0 };
                activate(&mut game, source, 2, &mut dm);
                assert_eq!(game.counter_count(source, CounterType::Loyalty), old_loyalty - 4);
                assert_eq!(game.stack.last().unwrap().targets, vec![chosen]);
                if invalidate {
                    if let Target::Object(id) = chosen { game.set_current_controller(id, A).unwrap(); }
                }
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.prompts, 1);
                assert_damage(&game, chosen, if invalidate { 0 } else { 7 }, loyalty);
            }
        }
    }
}

#[test]
fn bolas_other_loyalty_bodies_keep_opponent_and_exiled_card_scopes() {
    for artifact in [false, true] {
        let definition = definition(BOLAS, artifact);
        // +2 traverses only the chosen opponent's library and grants the
        // ability controller access only to the matched nonland card.
        let mut game = new_game();
        let source = bolas(&mut game, &definition);
        let matched = fixture(&mut game, B, CardType::Sorcery, Zone::Library);
        let land = fixture(&mut game, B, CardType::Land, Zone::Library);
        let untouched = fixture(&mut game, C, CardType::Sorcery, Zone::Library);
        let matched_stable = game.object(matched).unwrap().stable_id;
        let land_stable = game.object(land).unwrap().stable_id;
        game.player_mut(B).unwrap().library = vec![matched, land].into();
        let mut dm = Pick { chosen: Target::Player(B), expected: vec![Target::Player(B), Target::Player(C)], prompts: 0 };
        activate(&mut game, source, 0, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let matched = game.find_object_by_stable_id(matched_stable).unwrap();
        let land = game.find_object_by_stable_id(land_stable).unwrap();
        assert_eq!(game.object(matched).unwrap().zone, Zone::Exile);
        assert_eq!(game.object(land).unwrap().zone, Zone::Exile);
        assert_eq!(game.object(untouched).unwrap().zone, Zone::Library);
        game.turn.priority_player = Some(A);
        let actions = compute_legal_actions(&game, A).unwrap();
        assert!(actions.iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == matched)));
        assert!(!actions.iter().any(|action| matches!(action, LegalAction::PlayLand { land_id, .. } if *land_id == land)));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == matched)));

        // +1 makes each opponent choose from their own hand, including the
        // partial hand case, without involving the controller's hand.
        let mut game = new_game();
        let source = bolas(&mut game, &definition);
        for (player, count) in [(A, 3), (B, 3), (C, 1)] {
            for _ in 0..count { fixture(&mut game, player, CardType::Sorcery, Zone::Hand); }
        }
        activate(&mut game, source, 1, &mut SelectFirstDecisionMaker);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), 3);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert!(game.player(C).unwrap().hand.is_empty());
        assert_eq!(game.exile.len(), 3);

        // -12 selects by current controller, excludes lands, and preserves
        // Alice's board. Check permanent identities through zone changes.
        let mut game = new_game();
        let source = bolas(&mut game, &definition);
        let mut tracked = Vec::new();
        for player in [A, B, C] {
            for kind in [CardType::Creature, CardType::Artifact, CardType::Land] {
                let id = fixture(&mut game, player, kind, Zone::Battlefield);
                tracked.push((game.object(id).unwrap().stable_id, player != A && kind != CardType::Land));
            }
        }
        activate(&mut game, source, 3, &mut SelectFirstDecisionMaker);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        for (stable, exiled) in tracked {
            let id = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(id).unwrap().zone, if exiled { Zone::Exile } else { Zone::Battlefield });
        }
        assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
    }
}

fn defense_definition(type_line: &str, oracle: &str, artifact: bool) -> CardDefinition {
    let input = format!("Type: {type_line}\n{oracle}");
    if !artifact {
        return ironsmith_tools::compile_definition_from_payload(&ironsmith_tools::CardPayload {
            name: "Player defense witness".into(), parse_name: None,
            oracle_text: oracle.into(), raw_oracle_text: oracle.into(),
            metadata_lines: vec![format!("Type: {type_line}")], parse_input: input,
            other_face_name: None, linked_face_layout: None,
        }).unwrap();
    }
    let compiled = ironsmith_compiler::CompilerFacade::new().compile_definition(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), "Player defense witness"),
        input,
        ironsmith_compiler::CompilePolicy { allow_unsupported: false },
    ).unwrap();
    let wire = serde_json::from_value(serde_json::to_value(&compiled.definition).unwrap()).unwrap();
    ironsmith::artifact_materializer::materialize_definition(wire).unwrap()
}

fn player_defense(game: &mut GameState, oracle: &str, artifact: bool) {
    let definition = defense_definition("Enchantment", oracle, artifact);
    game.create_object_from_definition(&definition, B, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
}

#[test]
fn all_will_player_hexproof_fizzles_retained_trigger_after_theft_departure_or_phasing() {
    for artifact in [false, true] {
        let definition = definition(ALL_WILL, artifact);
        for change in 0..3 {
            let mut game = new_game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let (legal, _) = target_fixture(&mut game);
            let event = marker_event(source, Target::Player(C), A, false);
            let mut queue = TriggerQueue::new();
            for trigger in check_triggers(&game, &event) { queue.add(trigger); }
            let mut dm = Pick { chosen: Target::Player(B), expected: legal, prompts: 0 };
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            game.set_current_controller(source, B).unwrap();
            if change == 1 { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            if change == 2 { game.phase_out(source); }
            player_defense(&mut game, "You have hexproof.", artifact);
            assert_eq!(game.stack.last().unwrap().controller, A);
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(dm.prompts, 1);
            assert!(game.stack.is_empty());
            for player in [A, B, C] { assert_eq!(game.player(player).unwrap().life, 20); }
            for id in &game.battlefield {
                assert_eq!(game.damage_on(*id), 0, "no replacement object may receive damage");
            }
        }
    }
}

#[test]
fn bolas_minus_four_and_plus_two_fizzle_on_player_hexproof_after_source_theft() {
    for artifact in [false, true] {
        let definition = definition(BOLAS, artifact);
        for ordinal in [0, 2] {
            for leaves in [false, true] {
                let mut game = new_game();
                let source = bolas(&mut game, &definition);
                let library_card = fixture(&mut game, B, CardType::Sorcery, Zone::Library);
                let expected = if ordinal == 2 { target_fixture(&mut game).0 }
                    else { vec![Target::Player(B), Target::Player(C)] };
                let mut dm = Pick { chosen: Target::Player(B), expected, prompts: 0 };
                activate(&mut game, source, ordinal, &mut dm);
                game.set_current_controller(source, B).unwrap();
                if leaves { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                player_defense(&mut game, "You have hexproof.", artifact);
                assert_eq!(game.stack.last().unwrap().controller, A);
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.prompts, 1);
                assert!(game.stack.is_empty());
                for player in [A, B, C] { assert_eq!(game.player(player).unwrap().life, 20); }
                assert_eq!(game.object(library_card).unwrap().zone, Zone::Library);
                assert!(game.exile.is_empty(), "the illegal +2 target must not exile a card or grant access");
            }
        }
    }
}

#[test]
fn stolen_all_will_source_keeps_its_actual_protection_qualities() {
    for artifact in [false, true] {
        let definition = definition(ALL_WILL, artifact);
        for (defense, expected_life) in [
            ("You have protection from each of your opponents.", 16),
            ("You have protection from red.", 20),
        ] {
            let mut game = new_game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let event = marker_event(source, Target::Player(C), A, false);
            let mut queue = TriggerQueue::new();
            for trigger in check_triggers(&game, &event) { queue.add(trigger); }
            let mut dm = Pick { chosen: Target::Player(B),
                expected: vec![Target::Player(B), Target::Player(C)], prompts: 0 };
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            game.set_current_controller(source, B).unwrap();
            player_defense(&mut game, defense, artifact);
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(B).unwrap().life, expected_life);
            assert_eq!(dm.prompts, 1);
        }
    }
}

#[test]
fn mixed_target_abilities_keep_actual_ignore_hexproof_permission_after_theft_and_departure() {
    const SPOTLIGHT_PERMISSION: &str = "Creatures your opponents control with hexproof can be the targets of spells and abilities you control as though they didn't have hexproof.";
    for artifact in [false, true] {
        for name in [ALL_WILL, BOLAS] {
            let definition = definition(name, artifact);
            for defense in [
                "Creatures you control have hexproof.",
                "Creatures you control have hexproof from red.",
            ] {
                for permission_controller in [Some(A), Some(B), None] {
                    let mut game = new_game();
                    let source = if name == ALL_WILL {
                        game.create_object_from_definition(&definition, A, Zone::Battlefield)
                    } else { bolas(&mut game, &definition) };
                    let (legal, targets) = target_fixture(&mut game);
                    let chosen = targets[1];
                    let mut dm = Pick { chosen, expected: legal, prompts: 0 };
                    if name == ALL_WILL {
                        let event = marker_event(source, Target::Player(C), A, false);
                        let mut queue = TriggerQueue::new();
                        for trigger in check_triggers(&game, &event) { queue.add(trigger); }
                        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                    } else { activate(&mut game, source, 2, &mut dm); }
                    game.set_current_controller(source, B).unwrap();
                    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
                    player_defense(&mut game, defense, artifact);
                    if let Some(controller) = permission_controller {
                        let permission = defense_definition("Artifact", SPOTLIGHT_PERMISSION, artifact);
                        game.create_object_from_definition(&permission, controller, Zone::Battlefield);
                        game.refresh_continuous_state().unwrap();
                    }
                    let entry = game.stack.last().unwrap();
                    assert_eq!(entry.controller, A);
                    assert_eq!(entry.targets, vec![chosen]);
                    assert_eq!(entry.source_snapshot.as_ref().unwrap().controller, B,
                        "physical LKI and retained ability controller deliberately differ");
                    resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert_eq!(dm.prompts, 1);
                    let amount = if permission_controller == Some(A) {
                        if name == ALL_WILL { 4 } else { 7 }
                    } else { 0 };
                    assert_damage(&game, chosen, amount, 0);
                    assert!(game.stack.is_empty());
                    for id in &game.battlefield {
                        if chosen != Target::Object(*id) {
                            assert_eq!(game.damage_on(*id), 0, "no replacement recipient");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn compiled_all_player_hexproof_keeps_each_players_team_and_expires() {
    use ironsmith::target::{ChooseSpec, PlayerFilter};
    for artifact in [false, true] {
        let definition = defense_definition("Instant", "Players gain hexproof until end of turn.", artifact);
        let mut game = new_game();
        game.set_teams(vec![vec![A, C], vec![B]]).unwrap();
        let source = fixture(&mut game, A, CardType::Enchantment, Zone::Battlefield);
        let spell = game.create_object_from_definition(&definition, A, Zone::Stack);
        game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, A));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        game.set_current_controller(source, B).unwrap();
        game.refresh_continuous_state().unwrap();
        let spec = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any));
        for controller in [A, B, C] {
            let targets = ironsmith::targeting::compute_legal_targets(&game, &spec, controller, Some(source));
            for player in [A, B, C] {
                assert_eq!(targets.contains(&Target::Player(player)), !game.are_opponents(controller, player));
            }
        }
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.refresh_continuous_state().unwrap();
        assert_eq!(ironsmith::targeting::compute_legal_targets(&game, &spec, A, Some(source)),
            vec![Target::Player(A), Target::Player(B), Target::Player(C)]);
    }
}
