use ironsmith::ability::Ability;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith::continuous::ContinuousEffect;
use ironsmith::resolution::ResolutionProgram;
use ironsmith::triggers::{DelayedTrigger, Trigger};
use ironsmith::{
    CardBuilder, CardId, CardType, Effect, EffectExecutor, GameState, Object, ObjectFilter,
    ObjectKind, Phase, PlayerFilter, PlayerId, PowerToughness, StackEntry, Step, TriggerQueue,
    TurnAction, TurnRunner, TurnRunnerState, Until, Zone, resolve_stack_entry,
    run_priority_loop_with,
};

fn game() -> (GameState, PlayerId, PlayerId) {
    let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    (game, PlayerId::from_index(0), PlayerId::from_index(1))
}

fn definition(name: &str, card_types: Vec<CardType>, effects: Vec<Effect>) -> CardDefinition {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build();
    let mut definition = CardDefinition::new(card);
    if !effects.is_empty() {
        definition.spell_effect = Some(ResolutionProgram::from_effects(effects));
    }
    definition
}

fn creature_definition(name: &str) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
    )
}

#[test]
fn u037_end_combat_phase_does_nothing_outside_combat() {
    let (mut game, alice, _) = game();
    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    let source = game.create_object_from_definition(
        &creature_definition("Procedure Source"),
        alice,
        Zone::Battlefield,
    );
    let lower = game.create_object_from_definition(
        &definition(
            "Unaffected Stack Spell",
            vec![CardType::Instant],
            Vec::new(),
        ),
        alice,
        Zone::Stack,
    );
    game.push_to_stack(StackEntry::new(lower, alice));

    let mut ctx = ironsmith::EffectContext::new_default(source, alice);
    ironsmith::effects::EndCombatPhaseEffect::new()
        .execute(&mut game, &mut ctx)
        .expect("effect resolves");

    assert_eq!(game.stack.len(), 1);
    assert!(!game.turn_store.end_combat_phase_procedure_pending);
    assert_eq!(game.object(lower).expect("spell remains").zone, Zone::Stack);
}

#[test]
fn u037_stack_exile_includes_resolving_copy_and_stops_later_instructions() {
    let (mut game, alice, _) = game();
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    let definition = definition(
        "End Combat Copy",
        vec![CardType::Instant],
        vec![Effect::new(ironsmith::effects::SequenceEffect::new(vec![
            Effect::end_combat_phase(),
            Effect::gain_life(9),
        ]))],
    );
    let original = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(original, alice));
    let copy_id = game.new_object_id();
    let copy = Object::spell_copy_of(game.object(original).expect("original"), copy_id, alice);
    game.add_object(copy);
    game.push_to_stack(StackEntry::new(copy_id, alice));

    resolve_stack_entry(&mut game).expect("copy resolves");

    assert!(game.stack_is_empty());
    assert!(game.turn_store.end_combat_phase_procedure_pending);
    assert_eq!(game.player(alice).expect("Alice").life, 20);
    assert_eq!(game.exile.len(), 2);
    let exiled_copy = game
        .exile
        .iter()
        .copied()
        .find(|object| {
            game.object(*object)
                .is_some_and(|object| object.kind == ObjectKind::SpellCopy)
        })
        .expect("copy awaits SBA in exile");
    assert_eq!(
        game.object(exiled_copy).expect("copy").kind,
        ObjectKind::SpellCopy
    );

    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::DeclareBlockersPriority);
    let mut trigger_queue = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(
        game.object(exiled_copy).is_none(),
        "the SBA pass removes the copy"
    );
}

#[test]
fn u037_scheduler_skips_end_combat_triggers_and_defers_procedure_triggers() {
    let (mut game, alice, bob) = game();
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    let source = game.create_object_from_definition(
        &creature_definition("Procedure Source"),
        alice,
        Zone::Battlefield,
    );
    let attacker = game.create_object_from_definition(
        &creature_definition("Attacker"),
        alice,
        Zone::Battlefield,
    );
    let blocker =
        game.create_object_from_definition(&creature_definition("Blocker"), bob, Zone::Battlefield);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo {
        creature: attacker,
        target: AttackTarget::Player(bob),
    });
    combat.blockers.insert(attacker, vec![blocker]);
    game.combat = Some(combat.clone());

    let mut watcher = definition("Procedure Watcher", vec![CardType::Enchantment], Vec::new());
    watcher.abilities.push(Ability::triggered(
        Trigger::exiled(ObjectFilter::default()),
        vec![Effect::gain_life(1)],
    ));
    watcher.abilities.push(Ability::triggered(
        Trigger::end_of_combat(),
        vec![Effect::gain_life(10)],
    ));
    game.create_object_from_definition(&watcher, alice, Zone::Battlefield);

    let lower = game.create_object_from_definition(
        &definition("Exiled Stack Spell", vec![CardType::Instant], Vec::new()),
        alice,
        Zone::Stack,
    );
    game.push_to_stack(StackEntry::new(lower, alice));
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::pump(
            source,
            alice,
            attacker,
            3,
            3,
            Until::EndOfCombat,
        ));
    game.suppress_combat_damage_assignment(attacker, Until::EndOfCombat);
    game.effect_store.delayed_triggers.push(DelayedTrigger {
        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
        effects: ResolutionProgram::from_effects(vec![Effect::gain_life(50)]),
        one_shot: true,
        x_value: None,
        not_before_turn: None,
        expires_at_turn: None,
        expires_before_controller_turn_after: None,
        expires_at_end_of_combat: true,
        target_objects: Vec::new(),
        ability_source: Some(source),
        ability_source_stable_id: None,
        ability_source_name: Some("Combat-scoped watcher".into()),
        ability_source_snapshot: None,
        controller: alice,
        choices: Vec::new(),
        tagged_objects: std::collections::HashMap::new(),
    });
    assert_eq!(game.calculated_power(attacker), Some(5));
    assert!(game.combat_damage_assignment_is_suppressed(attacker));

    let mut ctx = ironsmith::EffectContext::new_default(source, alice);
    ironsmith::effects::EndCombatPhaseEffect::new()
        .execute(&mut game, &mut ctx)
        .expect("effect resolves");
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::DeclareBlockersPriority);
    *runner.combat_mut() = combat;
    let mut trigger_queue = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));

    assert!(runner.combat().attackers.is_empty());
    assert!(
        game.combat
            .as_ref()
            .expect("combat state")
            .attackers
            .is_empty()
    );
    assert_eq!(game.calculated_power(attacker), Some(2));
    assert!(!game.combat_damage_assignment_is_suppressed(attacker));
    assert!(game.effect_store.delayed_triggers.is_empty());
    assert!(game.stack_is_empty(), "procedure triggers remain deferred");
    assert_eq!(game.player(alice).expect("Alice").life, 20);

    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::RunPriority
    ));
    assert_eq!(game.turn.phase, Phase::NextMain);
    let mut decision_maker = ironsmith::AutoPassDecisionMaker;
    run_priority_loop_with(&mut game, &mut trigger_queue, &mut decision_maker)
        .expect("following phase priority resolves deferred trigger");
    assert_eq!(
        game.player(alice).expect("Alice").life,
        21,
        "only the exile trigger fires; the skipped end-combat trigger does not"
    );
}
