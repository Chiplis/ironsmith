//! Exact frozen full bodies. All regressions are source-authored and UNRUN.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration, DecisionMaker, LegalAction,
    SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{ManaPaymentContext, SelectObjectsContext, TargetsContext};
use ironsmith::events::cause::EventCause;
use ironsmith::events::phase::BeginningOfCombatEvent;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_attacker_declarations,
    apply_decision_context_with_dm, apply_multiplayer_blocker_declarations,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::static_abilities::StaticAbility;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{CardId, CardType, CounterType, GameProgress, GameState, ManaSymbol,
    ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/temporary_prevention_bindings.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &materialized] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, materialized]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 30);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}

fn creature(game: &mut GameState, owner: PlayerId, power: i32) -> ObjectId {
    let id = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Secondary witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, 12)).build(), owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn land(game: &mut GameState, zone: Zone, name: &str, subtypes: Vec<Subtype>) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land]).subtypes(subtypes).build(), A, zone)
}

#[derive(Default)]
struct Choices {
    target: Option<Target>,
    forbidden_targets: Vec<Target>,
    discard: Option<ObjectId>,
    forbidden_objects: Vec<ObjectId>,
    target_calls: usize,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.target_calls += 1;
        for requirement in &context.requirements {
            assert!(self.forbidden_targets.iter().all(|target| !requirement.legal_targets.contains(target)));
        }
        if let Some(target) = self.target {
            assert_eq!(context.requirements.len(), 1);
            assert!(context.requirements[0].legal_targets.contains(&target));
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal = context.candidates.iter().filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id).collect::<Vec<_>>();
        assert!(self.forbidden_objects.iter().all(|id| !legal.contains(id)));
        if let Some(discard) = self.discard {
            assert!(legal.contains(&discard));
            vec![discard]
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
    fn decide_mana_payment(&mut self, _: &GameState, context: &ManaPaymentContext)
        -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: context.plan.id, request_hash: context.plan.request_hash,
        }
    }
}

fn announce(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    game.turn.priority_player = Some(A);
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices).unwrap();
    }
    assert!(!state.has_pending_action());
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
    assert_eq!(game.stack.len(), 1);
}

fn cast_action(game: &GameState, spell: ObjectId, alternative: bool) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. }
            if *spell_id == spell && if alternative {
                matches!(casting_method, CastingMethod::Alternative(_))
            } else { matches!(casting_method, CastingMethod::Normal) }))
}

fn activation(game: &GameState, source: ObjectId) -> LegalAction {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source))
        .expect("printed activation should be legal")
}

fn resolve(game: &mut GameState, choices: &mut Choices) {
    *game = game.clone();
    resolve_stack_entry_with(game, choices).unwrap();
    assert!(game.stack.is_empty());
}

fn damage(game: &mut GameState, source: ObjectId, target: DamageTarget, amount: u32,
    combat: bool, unpreventable: bool) -> (u32, u32) {
    game.take_pending_trigger_events();
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(game, source,
        target, amount, combat, unpreventable, EventCause::effect(), None).unwrap();
    let remaining = result.assignments.iter().map(|assignment| assignment.amount).sum();
    let prevented = game.take_pending_trigger_events().iter().filter_map(|event|
        event.downcast::<DamagePreventedEvent>().map(|event| event.amount)).sum();
    (remaining, prevented)
}

fn combat(game: &mut GameState, finish_blocks: bool) -> (ObjectId, ObjectId, ObjectId, ObjectId) {
    let unblocked = creature(game, B, 2);
    let blocked = creature(game, B, 4);
    let blocker = creature(game, C, 6);
    let idle = creature(game, B, 8);
    game.turn.active_player = B;
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut combat, &mut queue, &[
        AttackerDeclaration { creature: unblocked, target: AttackTarget::Player(C) },
        AttackerDeclaration { creature: blocked, target: AttackTarget::Player(C) },
    ]).unwrap();
    game.combat = Some(combat.clone());
    if finish_blocks {
        game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
        apply_multiplayer_blocker_declarations(game, &mut combat, &mut queue,
            &[BlockerDeclaration { blocker, blocking: blocked }]).unwrap();
        game.combat = Some(combat);
    }
    game.turn.priority_player = Some(A);
    (unblocked, blocked, blocker, idle)
}

fn untap(game: &mut GameState, player: PlayerId) {
    game.turn.turn_number += 1;
    game.turn.active_player = player;
    game.turn.phase = ironsmith::Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Untap);
    ironsmith::turn::execute_untap_step_with(game, &mut SelectFirstDecisionMaker).unwrap();
}

#[test]
fn snag_discards_a_forest_subtype_card_as_its_alternative_price_and_filters_damage_sources() {
    for definition in definitions("Snag") {
        let mut game = game();
        let (unblocked, blocked, blocker, idle) = combat(&mut game, true);
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let named_only = land(&mut game, Zone::Hand, "Forest", vec![]);
        let island = land(&mut game, Zone::Hand, "Island witness", vec![Subtype::Island]);
        let battlefield_forest = land(&mut game, Zone::Battlefield, "Forest on battlefield", vec![Subtype::Forest]);
        assert!(cast_action(&game, spell, true).is_none());
        let forest = land(&mut game, Zone::Hand, "Nonbasic Forest witness", vec![Subtype::Forest]);
        let stable = game.object(forest).unwrap().stable_id;
        let mut choices = Choices { discard: Some(forest),
            forbidden_objects: vec![named_only, island, battlefield_forest], ..Default::default() };
        assert!(cast_action(&game, spell, false).is_none());
        let action = cast_action(&game, spell, true).unwrap();
        announce(&mut game, action, &mut choices);
        let paid = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(paid).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(game.stack.last().unwrap().targets.is_empty());
        resolve(&mut game, &mut choices);
        assert_eq!(damage(&mut game, unblocked, DamageTarget::Player(C), 3, true, false), (0, 3));
        for source in [blocked, blocker, idle] {
            assert_eq!(damage(&mut game, source, DamageTarget::Player(C), 3, true, false), (3, 0));
        }
        assert_eq!(damage(&mut game, unblocked, DamageTarget::Player(C), 3, false, false), (3, 0));
        assert_eq!(damage(&mut game, unblocked, DamageTarget::Player(C), 3, true, true), (3, 0));
        game.effect_store.prevention_effects.cleanup_end_of_turn();
        assert_eq!(damage(&mut game, unblocked, DamageTarget::Player(C), 3, true, false), (3, 0));
    }
}

#[test]
fn gossamer_returns_its_exact_source_to_its_owner_before_protecting_the_unblocked_target() {
    for definition in definitions("Gossamer Chains") {
        for remove_target in [false, true] {
            let mut game = game();
            let (unblocked, blocked, blocker, idle) = combat(&mut game, true);
            let chains = game.create_object_from_definition(&definition, B, Zone::Battlefield);
            game.set_current_controller(chains, A).unwrap();
            let stable = game.object(chains).unwrap().stable_id;
            let mut choices = Choices { target: Some(Target::Object(unblocked)),
                forbidden_targets: vec![Target::Object(blocked), Target::Object(blocker), Target::Object(idle)],
                ..Default::default() };
            let action = activation(&game, chains);
            announce(&mut game, action, &mut choices);
            let returned = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(returned).unwrap().zone, Zone::Hand);
            assert!(game.player(B).unwrap().hand.contains(&returned));
            assert!(!game.player(A).unwrap().hand.contains(&returned));
            assert_eq!(game.stack.last().unwrap().targets, vec![Target::Object(unblocked)]);
            if remove_target { game.move_object_by_effect(unblocked, Zone::Graveyard).unwrap(); }
            resolve(&mut game, &mut choices);
            assert_eq!(choices.target_calls, 1);
            assert_eq!(damage(&mut game, blocked, DamageTarget::Player(C), 3, true, false), (3, 0));
            if remove_target {
                assert!(game.effect_store.prevention_effects.shields().is_empty());
            } else {
                assert_eq!(damage(&mut game, unblocked, DamageTarget::Player(C), 3, true, false), (0, 3));
                assert_eq!(damage(&mut game, unblocked, DamageTarget::Object(blocker), 3, true, false), (0, 3));
                assert_eq!(damage(&mut game, unblocked, DamageTarget::Player(C), 3, false, false), (3, 0));
            }
        }
        let mut game = game();
        combat(&mut game, false);
        let chains = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action|
            matches!(action, LegalAction::ActivateAbility { source, .. } if *source == chains)),
            "attackers are not unblocked before the block declaration completes");
    }
}

#[test]
fn boros_uses_paid_red_and_the_selected_creatures_live_power_and_controller() {
    for definition in definitions("Boros Fury-Shield") {
        for red in [false, true] {
            for target_blocker in [false, true] {
                let mut game = game();
                let (attacker, _, blocker, idle) = combat(&mut game, true);
                let target = if target_blocker { blocker } else { attacker };
                let controller = if target_blocker { C } else { B };
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White, 1);
                game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, if red { 1 } else { 2 });
                if red { game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red, 1); }
                let mut choices = Choices { target: Some(Target::Object(target)),
                    forbidden_targets: vec![Target::Object(idle), Target::Player(controller)],
                    ..Default::default() };
                let action = cast_action(&game, spell, false).unwrap();
                announce(&mut game, action, &mut choices);
                let stack_spell = game.stack.last().unwrap().object_id;
                assert_eq!(game.object(stack_spell).unwrap().mana_spent_to_cast.total(), 3);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                assert_eq!(game.stack.last().unwrap().targets, vec![Target::Object(target)]);
                game.add_counters(target, CounterType::PlusOnePlusOne, 3);
                let power = game.current_power(target).unwrap();
                resolve(&mut game, &mut choices);
                assert_eq!(choices.target_calls, 1, "the derived controller is not a second target");
                for player in [A, B, C] {
                    assert_eq!(game.player(player).unwrap().life,
                        30 - if red && player == controller { power } else { 0 });
                }
                assert_eq!(damage(&mut game, target, DamageTarget::Player(A), 3, true, false), (0, 3));
                assert_eq!(damage(&mut game, target, DamageTarget::Player(A), 3, false, false), (3, 0));
                assert_eq!(damage(&mut game, idle, DamageTarget::Player(A), 3, true, false), (3, 0));
            }
        }
    }
}

#[test]
fn loyal_checks_your_commander_at_trigger_and_resolution_and_keeps_vigilance_siblings() {
    for definition in definitions("Loyal Unicorn") {
        for scenario in ["own", "none", "opponents commander", "own elsewhere", "leaves before resolution", "gained late", "opponents turn"] {
            let mut game = game();
            let unicorn = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let own = creature(&mut game, A, 2);
            let enemy = creature(&mut game, B, 2);
            let commander_owner = if scenario == "opponents commander" { B } else { A };
            let commander = creature(&mut game, commander_owner, 4);
            if scenario != "none" { game.set_as_commander(commander, commander_owner); }
            if matches!(scenario, "own elsewhere" | "gained late") {
                game.set_current_controller(commander, B).unwrap();
            } else if scenario == "opponents commander" {
                game.set_current_controller(commander, A).unwrap();
            }
            let active = if scenario == "opponents turn" { B } else { A };
            game.turn.active_player = active;
            game.turn.phase = ironsmith::Phase::Combat;
            game.turn.step = Some(ironsmith::game_state::Step::BeginCombat);
            game.queue_trigger_event(Default::default(), TriggerEvent::new(
                BeginningOfCombatEvent::new(active), Default::default()));
            let mut choices = Choices::default();
            put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut choices).unwrap();
            let should_trigger = matches!(scenario, "own" | "leaves before resolution");
            assert_eq!(game.stack.len(), usize::from(should_trigger), "{scenario}");
            if scenario == "gained late" { game.set_current_controller(commander, A).unwrap(); }
            if scenario == "leaves before resolution" { game.move_object_by_effect(commander, Zone::Hand).unwrap(); }
            if should_trigger { resolve(&mut game, &mut choices); }
            let applies = scenario == "own";
            assert!(game.object_has_ability(unicorn, &StaticAbility::vigilance()));
            assert_eq!(game.object_has_ability(own, &StaticAbility::vigilance()), applies, "{scenario}");
            assert!(!game.object_has_ability(enemy, &StaticAbility::vigilance()));
            let expected = if applies { (0, 3) } else { (3, 0) };
            for recipient in [own, unicorn] {
                assert_eq!(damage(&mut game, enemy, DamageTarget::Object(recipient), 3, true, false), expected, "{scenario}");
            }
            assert_eq!(damage(&mut game, enemy, DamageTarget::Object(own), 3, false, false), (3, 0));
            assert_eq!(damage(&mut game, enemy, DamageTarget::Player(A), 3, true, false), (3, 0));
            let late = creature(&mut game, A, 2);
            assert!(!game.object_has_ability(late, &StaticAbility::vigilance()));
            assert_eq!(damage(&mut game, enemy, DamageTarget::Object(late), 3, true, false), expected,
                "prevention's rule remains live while the ability grant fixes its recipients");
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert!(!game.object_has_ability(own, &StaticAbility::vigilance()));
            assert!(game.object_has_ability(unicorn, &StaticAbility::vigilance()));
            assert_eq!(damage(&mut game, enemy, DamageTarget::Object(own), 3, true, false), (3, 0));
        }
    }
}

#[test]
fn samite_taps_the_same_target_and_keeps_a_four_damage_budget_and_fixed_untap_player() {
    for definition in definitions("Samite Alchemist") {
        for already_tapped in [false, true] {
            for change_controller in [false, true] {
                let mut game = game();
                let alchemist = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                game.remove_summoning_sickness(alchemist);
                let target = creature(&mut game, A, 2);
                let unrelated = creature(&mut game, A, 2);
                let enemy = creature(&mut game, B, 2);
                if already_tapped { game.tap(target); }
                game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White, 2);
                let mut choices = Choices { target: Some(Target::Object(target)),
                    forbidden_targets: vec![Target::Object(enemy)], ..Default::default() };
                let action = activation(&game, alchemist);
                announce(&mut game, action, &mut choices);
                assert!(game.is_tapped(alchemist));
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                game.set_current_controller(alchemist, B).unwrap();
                resolve(&mut game, &mut choices);
                assert_eq!(choices.target_calls, 1);
                assert!(game.is_tapped(target));
                assert!(!game.is_tapped(unrelated));
                assert_eq!(damage(&mut game, enemy, DamageTarget::Object(target), 3, true, true), (3, 0));
                assert_eq!(damage(&mut game, enemy, DamageTarget::Object(target), 3, false, false), (0, 3));
                assert_eq!(damage(&mut game, enemy, DamageTarget::Object(target), 3, true, false), (2, 1));
                assert_eq!(damage(&mut game, enemy, DamageTarget::Object(target), 3, false, false), (3, 0));
                assert_eq!(damage(&mut game, enemy, DamageTarget::Object(unrelated), 3, true, false), (3, 0));
                game.move_object_by_effect(alchemist, Zone::Graveyard).unwrap();
                if change_controller {
                    game.set_current_controller(target, B).unwrap();
                    untap(&mut game, B);
                    assert!(!game.is_tapped(target), "the restriction is tied to A's next untap");
                    game.tap(target);
                    untap(&mut game, A);
                    assert!(game.is_tapped(target));
                    game.set_current_controller(target, A).unwrap();
                    untap(&mut game, A);
                    assert!(!game.is_tapped(target), "A's first untap already consumed the restriction");
                } else {
                    untap(&mut game, A);
                    assert!(game.is_tapped(target));
                    untap(&mut game, A);
                    assert!(!game.is_tapped(target));
                }
            }
        }
    }
}

#[test]
fn boros_all_illegal_target_loses_the_red_followup_without_refunding_its_payment() {
    for definition in definitions("Boros Fury-Shield") {
        let mut game = game();
        let (target, _, _, _) = combat(&mut game, true);
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        for symbol in [ManaSymbol::White, ManaSymbol::Red, ManaSymbol::Colorless] {
            game.player_mut(A).unwrap().mana_pool.add(symbol, 1);
        }
        let mut choices = Choices { target: Some(Target::Object(target)), ..Default::default() };
        let action = cast_action(&game, spell, false).unwrap();
        announce(&mut game, action, &mut choices);
        game.move_object_by_effect(target, Zone::Hand).unwrap();
        resolve(&mut game, &mut choices);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        for player in [A, B, C] { assert_eq!(game.player(player).unwrap().life, 30); }
        assert!(game.effect_store.prevention_effects.shields().is_empty());
    }
}

#[test]
fn samite_all_illegal_target_drops_every_resolution_sibling_but_keeps_paid_costs() {
    for definition in definitions("Samite Alchemist") {
        let mut game = game();
        let alchemist = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(alchemist);
        let target = creature(&mut game, A, 2);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White, 2);
        let mut choices = Choices { target: Some(Target::Object(target)), ..Default::default() };
        let action = activation(&game, alchemist);
        announce(&mut game, action, &mut choices);
        let hand = game.move_object_by_effect(target, Zone::Hand).unwrap();
        let returned = game.move_object_by_effect(hand, Zone::Battlefield).unwrap();
        assert_ne!(returned, target);
        resolve(&mut game, &mut choices);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(game.is_tapped(alchemist));
        assert!(!game.is_tapped(returned));
        assert!(game.effect_store.prevention_effects.shields().is_empty());
        assert!(game.effect_store.restriction_effects.is_empty());
        game.tap(returned);
        untap(&mut game, A);
        assert!(!game.is_tapped(returned));
    }
}
