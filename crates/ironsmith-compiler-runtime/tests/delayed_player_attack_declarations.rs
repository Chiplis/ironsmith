//! Source-authored regression scenarios; direct and artifact paths are independent.
//! All scenarios are intentionally UNRUN pending the coordinated compatibility gate.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effects::{EffectContext, EffectExecutor, ResolvedTarget, ScheduleDelayedTriggerEffect, execute_effect};
use ironsmith::game_loop::{apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Target;
use ironsmith::triggers::{Trigger, TriggerEvent, TriggerQueue, check_delayed_triggers};
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{DelayedTriggerSpec, PlayerFilter, TriggerKind};
use ironsmith_core::trigger_model::PlayerAttackGrouping;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/delayed_player_attack_declarations.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    // Do not substitute the artifact compiler's companion for the direct route.
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}
fn card(game: &mut GameState, owner: PlayerId, text: &str) -> ObjectId {
    let def = compile_to_runtime_definition("Test resource", text, false).unwrap();
    let id = game.create_object_from_definition(&def, owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}
fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
    card(game, owner, "Type: Creature\nPower/Toughness: 2/10")
}
fn program(definition: &CardDefinition) -> &ironsmith::resolution::ResolutionProgram {
    definition.abilities.iter().find_map(|ability| {
        let effects = match &ability.kind {
            AbilityKind::Activated(ability) => &ability.effects,
            AbilityKind::Triggered(ability) => &ability.effects,
            _ => return None,
        };
        effects.all_effects().iter().any(|effect| effect.downcast_ref::<ScheduleDelayedTriggerEffect>()
            .is_some_and(|schedule| schedule.trigger.downcast_ref::<ironsmith::triggers::PlayerAttackDeclarationTrigger>().is_some()))
            .then_some(effects)
    }).expect("full body retains its player-attack delayed program")
}
fn register(game: &mut GameState, definition: &CardDefinition, source: ObjectId, target: Option<ObjectId>) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    if let Some(target) = target { ctx = ctx.with_targets(vec![ResolvedTarget::Object(target)]); }
    // Resolve the authored ability body, including its preceding target/tag
    // instructions. Activation cost and Saga chapter dispatch are not simulated.
    let before = game.effect_store.delayed_triggers.len();
    for segment in &program(definition).segments {
        assert!(segment.self_replacements.is_empty(), "this helper only executes linear bodies");
        for effect in &segment.default_effects { execute_effect(game, effect, &mut ctx).unwrap(); }
    }
    assert_eq!(game.effect_store.delayed_triggers.len(), before + 1);
    let delayed = game.effect_store.delayed_triggers.last().unwrap();
    assert!(!delayed.one_shot);
    assert_eq!(delayed.expires_at_turn, Some(game.turn.turn_number));
}
fn declare(game: &mut GameState, attacks: &[(ObjectId, AttackTarget)]) -> TriggerQueue {
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.turn_store.combat_phases_started_this_turn += 1;
    game.combat = None;
    let declarations = attacks.iter().map(|(creature, target)| {
        game.untap(*creature);
        ironsmith::decision::AttackerDeclaration { creature: *creature, target: target.clone() }
    }).collect::<Vec<_>>();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut CombatState::default(), &mut queue, &declarations).unwrap();
    queue
}
fn settle(game: &mut GameState, mut queue: TriggerQueue) {
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        if game.stack.is_empty() { return; }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    panic!("delayed attack stack did not settle");
}
fn tokens(game: &GameState) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|id| game.object(*id)
        .is_some_and(|object| object.kind == ironsmith::object::ObjectKind::Token)).collect()
}
struct PickCreature(ObjectId);
impl DecisionMaker for PickCreature {
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        let target = Target::Object(self.0);
        assert_eq!(context.requirements.len(), 1);
        assert!(context.requirements[0].legal_targets.contains(&target));
        vec![target]
    }
}
#[test]
fn full_bodies_preserve_exact_ids_and_native_grouping_after_independent_compilation() {
    let expected = [
        ("Dalkovan Encampment", "33a90122-7280-4481-9b97-5879194cae40"),
        ("Jaya, Fiery Negotiator", "2458aa66-5b20-4811-a4a4-8375ad0a6498"),
        ("Roads Go Ever, Ever On", "3cd3cdc9-2ed2-4cdf-b58a-12c3a2ef218b"),
    ];
    assert_eq!(rows().len(), expected.len());
    for (name, id) in expected {
        assert!(rows().iter().any(|row| row["name"] == name && row["oracle_id"] == id));
        for definition in definitions(name) {
            let schedule = program(&definition).all_effects().into_iter()
                .find_map(|effect| effect.downcast_ref::<ScheduleDelayedTriggerEffect>()).unwrap();
            assert!(!schedule.one_shot);
            assert_eq!(schedule.duration, ironsmith_core::DelayedTriggerDuration::EndOfTurn);
            let native = schedule.trigger.downcast_ref::<ironsmith::triggers::PlayerAttackDeclarationTrigger>().unwrap();
            assert_eq!(native.attacker, PlayerFilter::You);
            assert_eq!(native.defender, PlayerFilter::Any);
            assert_eq!(native.grouping, PlayerAttackGrouping::AttackerAnyTarget);
            assert!(matches!(&schedule.trigger.compiled_model().unwrap().kind,
                TriggerKind::PlayerAttackDeclaration { grouping: PlayerAttackGrouping::AttackerAnyTarget, .. }));
        }
    }
}
#[test]
fn delayed_wire_round_trip_preserves_every_grouping_and_player_filter() {
    for grouping in [PlayerAttackGrouping::AttackerAnyTarget, PlayerAttackGrouping::Attacker,
        PlayerAttackGrouping::Defender, PlayerAttackGrouping::Pair] {
        let spec = DelayedTriggerSpec::PlayerAttackDeclaration {
            attacker: PlayerFilter::Specific(B), defender: PlayerFilter::Opponent, grouping,
        };
        let restored = serde_json::from_str::<DelayedTriggerSpec>(&serde_json::to_string(&spec).unwrap()).unwrap();
        assert_eq!(spec, restored);
        let trigger = Trigger::from_delayed_trigger_spec(restored);
        let native = trigger.downcast_ref::<ironsmith::triggers::PlayerAttackDeclarationTrigger>().unwrap();
        assert_eq!(native.attacker, PlayerFilter::Specific(B));
        assert_eq!(native.defender, PlayerFilter::Opponent);
        assert_eq!(native.grouping, grouping);
        assert!(matches!(&trigger.compiled_model().unwrap().kind, TriggerKind::PlayerAttackDeclaration { .. }));
    }
}
#[test]
fn dalkovan_groups_split_targets_repeats_in_extra_combat_and_sacrifices_each_created_batch() {
    for definition in definitions("Dalkovan Encampment") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let one = creature(&mut game, A);
        let two = creature(&mut game, A);
        let three = creature(&mut game, A);
        let walker = card(&mut game, B, "Type: Planeswalker\nLoyalty: 5");
        let battle = card(&mut game, A, "Type: Battle — Siege\nDefense: 5");
        assert!(game.set_battle_protector(battle, C));
        register(&mut game, &definition, source, None);
        let queue = declare(&mut game, &[(one, AttackTarget::Player(B)),
            (two, AttackTarget::Player(C)), (three, AttackTarget::Planeswalker(walker))]);
        assert_eq!(queue.entries.len(), 1, "one declaration across three targets");
        settle(&mut game, queue);
        assert_eq!(tokens(&game).len(), 2, "entering attacking must not retrigger");
        for token in tokens(&game) {
            assert!(game.is_tapped(token));
            assert!(game.combat.as_ref().unwrap().attackers.iter().any(|attack| attack.creature == token));
        }
        let queue = declare(&mut game, &[(one, AttackTarget::Planeswalker(walker))]);
        assert_eq!(queue.entries.len(), 1, "another combat, including a planeswalker-only attack");
        settle(&mut game, queue);
        let queue = declare(&mut game, &[(one, AttackTarget::Battle(battle))]);
        assert_eq!(queue.entries.len(), 1, "a battle-only declaration is still a player attack");
        settle(&mut game, queue);
        let created = tokens(&game);
        assert_eq!(created.len(), 6);
        let event = TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfEndStepEvent::new(A), Default::default());
        let mut queue = TriggerQueue::new();
        for entry in check_delayed_triggers(&mut game, &event) { queue.add(entry); }
        assert_eq!(queue.entries.len(), 3, "one sacrifice registration per token batch");
        settle(&mut game, queue);
        assert!(created.iter().all(|id| !game.battlefield.contains(id)));
        assert!(game.battlefield.contains(&one));
        game.turn.turn_number += 1;
        let queue = declare(&mut game, &[(one, AttackTarget::Player(B))]);
        assert!(queue.entries.is_empty(), "this-turn watcher has expired");
    }
}
#[test]
fn jaya_remembers_the_chosen_incarnation_and_counts_current_attackers_on_resolution() {
    for definition in definitions("Jaya, Fiery Negotiator") {
        for blink_target in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let chosen = creature(&mut game, B);
            let other = creature(&mut game, B);
            let one = creature(&mut game, A);
            let two = creature(&mut game, A);
            register(&mut game, &definition, source, Some(chosen));
            let mut queue = declare(&mut game, &[(one, AttackTarget::Player(B)), (two, AttackTarget::Player(C))]);
            assert_eq!(queue.entries.len(), 1);
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.stack.len(), 1);
            assert!(game.stack[0].targets.is_empty(), "Jaya's delayed ability does not retarget");
            let current_chosen = if blink_target {
                let exiled = game.move_object_by_effect(chosen, Zone::Exile).unwrap();
                game.move_object_by_effect(exiled, Zone::Battlefield).unwrap()
            } else { chosen };
            // Recount at resolution, rather than capturing two at declaration.
            game.move_object_by_effect(two, Zone::Graveyard).unwrap();
            // The delayed trigger keeps working after Jaya has left.
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            settle(&mut game, queue);
            assert_eq!(game.damage_on(current_chosen), if blink_target { 0 } else { 1 });
            assert_eq!(game.damage_on(other), 0, "do not choose a replacement creature");
            let queue = declare(&mut game, &[(one, AttackTarget::Player(B))]);
            assert_eq!(queue.entries.len(), 1);
            settle(&mut game, queue);
            assert_eq!(game.damage_on(current_chosen), if blink_target { 0 } else { 2 });
        }
    }
}
#[test]
fn roads_chapter_registers_a_new_targeted_reward_each_combat_even_after_saga_leaves() {
    for definition in definitions("Roads Go Ever, Ever On") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        // These scenarios resolve chapter IV's complete body directly; clear
        // the independently emitted entry/lore event from this fixture setup.
        game.take_pending_trigger_events();
        let one = creature(&mut game, A);
        let two = creature(&mut game, A);
        card(&mut game, A, "Type: Basic Land — Plains");
        card(&mut game, A, "Type: Basic Land — Plains");
        register(&mut game, &definition, source, None);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let mut queue = declare(&mut game, &[(one, AttackTarget::Player(B)), (two, AttackTarget::Player(C))]);
        assert_eq!(queue.entries.len(), 1);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut PickCreature(one)).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![Target::Object(one)]);
        // The Plains total changes after declaration and target selection.
        card(&mut game, A, "Type: Basic Land — Plains");
        settle(&mut game, queue);
        assert_eq!(game.calculated_power(one), Some(5));
        assert_eq!(game.calculated_power(two), Some(2));
        let mut queue = declare(&mut game, &[(one, AttackTarget::Player(B))]);
        assert_eq!(queue.entries.len(), 1);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut PickCreature(two)).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![Target::Object(two)], "a new target is chosen each combat");
        settle(&mut game, queue);
        assert_eq!(game.calculated_power(one), Some(5));
        assert_eq!(game.calculated_power(two), Some(5));
    }
}
#[test]
fn separate_registrations_do_not_coalesce_and_empty_or_opponent_declarations_do_not_match() {
    for definition in definitions("Dalkovan Encampment") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let one = creature(&mut game, A);
        register(&mut game, &definition, source, None);
        register(&mut game, &definition, source, None);
        assert!(declare(&mut game, &[]).entries.is_empty());
        let queue = declare(&mut game, &[(one, AttackTarget::Player(B))]);
        assert_eq!(queue.entries.len(), 2);
        settle(&mut game, queue);
        assert_eq!(tokens(&game).len(), 4);
        game.turn.active_player = B;
        game.turn.priority_player = Some(B);
        let opponent = creature(&mut game, B);
        assert!(declare(&mut game, &[(opponent, AttackTarget::Player(A))]).entries.is_empty());
    }
}

#[test]
fn direct_declaration_queue_preserves_ordinary_and_delayed_creature_attack_triggers_once() {
    let mut game = game();
    let one = card(&mut game, A, "Type: Creature\nPower/Toughness: 2/10\nWhenever this creature attacks, you gain 7 life.");
    let two = creature(&mut game, A);
    let filter = ironsmith_core::ObjectFilter::creature().you_control();
    for (trigger, amount) in [
        (Trigger::attacks(filter.clone()), 1),
        (Trigger::attacks_one_or_more(filter), 10),
    ] {
        ScheduleDelayedTriggerEffect::new(trigger, vec![ironsmith::Effect::gain_life(amount)],
            false, vec![], PlayerFilter::You).until_end_of_turn()
            .execute(&mut game, &mut EffectContext::new_default(one, A)).unwrap();
    }
    let queue = declare(&mut game, &[(one, AttackTarget::Player(B)), (two, AttackTarget::Player(C))]);
    assert_eq!(queue.entries.len(), 4, "one ordinary, two singular delayed, one grouped delayed");
    settle(&mut game, queue);
    assert_eq!(game.player(A).unwrap().life, 39, "pending-event drains must not replay declarations");
}

#[test]
fn native_player_filters_and_direct_target_category_are_applied_before_grouping() {
    use ironsmith::events::PlayerAttackDeclarationEvent;
    use ironsmith::triggers::check_delayed_triggers_for_simultaneous_events;
    for grouping in [PlayerAttackGrouping::Attacker, PlayerAttackGrouping::AttackerAnyTarget,
        PlayerAttackGrouping::Defender, PlayerAttackGrouping::Pair] {
        for direct in [false, true] {
            let mut game = game();
            let source = card(&mut game, A, "Type: Artifact");
            let trigger = Trigger::from_delayed_trigger_spec(DelayedTriggerSpec::PlayerAttackDeclaration {
                attacker: PlayerFilter::Specific(B), defender: PlayerFilter::Specific(C), grouping,
            });
            ScheduleDelayedTriggerEffect::new(trigger, vec![ironsmith::Effect::gain_life(1)],
                false, vec![], PlayerFilter::You).until_end_of_turn()
                .execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
            let events = [(A, C), (B, A), (B, C)].map(|(attacker, defender)| {
                TriggerEvent::new_with_provenance(PlayerAttackDeclarationEvent {
                    attacker, defender, turn_number: game.turn.turn_number, combat_phase: 1,
                    directly_attacked_player: direct, declaration: None,
                }, Default::default())
            });
            let entries = check_delayed_triggers_for_simultaneous_events(&mut game, &events);
            assert_eq!(entries.len(), usize::from(direct || grouping == PlayerAttackGrouping::AttackerAnyTarget));
            for entry in entries {
                let event = entry.triggering_event.downcast::<PlayerAttackDeclarationEvent>().unwrap();
                assert_eq!((event.attacker, event.defender), (B, C));
            }
        }
    }
}

#[test]
fn native_delayed_grouping_is_per_actor_defender_or_pair_and_one_shot_consumption_is_after_grouping() {
    use ironsmith::events::PlayerAttackDeclarationEvent;
    use ironsmith::triggers::check_delayed_triggers_for_simultaneous_events;
    for (grouping, count) in [(PlayerAttackGrouping::Attacker, 2),
        (PlayerAttackGrouping::AttackerAnyTarget, 2), (PlayerAttackGrouping::Defender, 2),
        (PlayerAttackGrouping::Pair, 3)] {
        for one_shot in [false, true] {
            let mut game = game();
            let source = card(&mut game, A, "Type: Artifact");
            let trigger = Trigger::from_delayed_trigger_spec(DelayedTriggerSpec::PlayerAttackDeclaration {
                attacker: PlayerFilter::Any, defender: PlayerFilter::Any, grouping,
            });
            let schedule = ScheduleDelayedTriggerEffect::new(trigger, vec![ironsmith::Effect::gain_life(1)],
                one_shot, vec![], PlayerFilter::You).until_end_of_turn();
            schedule.execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
            let turn_number = game.turn.turn_number;
            let events = |combat_phase| [(A, B), (A, C), (B, C)].map(|(attacker, defender)| {
                TriggerEvent::new_with_provenance(PlayerAttackDeclarationEvent {
                    attacker, defender, turn_number, combat_phase,
                    directly_attacked_player: true, declaration: None,
                }, Default::default())
            });
            let entries = check_delayed_triggers_for_simultaneous_events(&mut game, &events(1));
            assert_eq!(entries.len(), if one_shot { 1 } else { count });
            let pairs = entries.iter().map(|entry| {
                let event = entry.triggering_event.downcast::<PlayerAttackDeclarationEvent>().unwrap();
                (event.attacker, event.defender)
            }).collect::<Vec<_>>();
            let expected = if one_shot {
                vec![(A, B)]
            } else {
                match grouping {
                    PlayerAttackGrouping::Attacker | PlayerAttackGrouping::AttackerAnyTarget => vec![(A, B), (B, C)],
                    PlayerAttackGrouping::Defender => vec![(A, B), (A, C)],
                    PlayerAttackGrouping::Pair => vec![(A, B), (A, C), (B, C)],
                }
            };
            assert_eq!(pairs, expected);
            assert_eq!(game.effect_store.delayed_triggers.len(), usize::from(!one_shot));
            let again = check_delayed_triggers_for_simultaneous_events(&mut game, &events(2));
            assert_eq!(again.len(), if one_shot { 0 } else { count }, "later declaration gets fresh grouping");
        }
    }
}
