use ironsmith::ability::{Ability, TriggeredAbility};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith::continuous::ContinuousEffect;
use ironsmith::events::zones::EnterBattlefieldEvent;
use ironsmith::provenance::ProvNodeId;
use ironsmith::resolution::ResolutionProgram;
use ironsmith::triggers::{
    DelayedTrigger, Trigger, TriggerEvent, TriggeredAbilityEntry, TriggeredAbilitySourceKind,
    compute_trigger_identity,
};
use ironsmith::{
    CardBuilder, CardId, CardType, Effect, EffectExecutor, GameState, Object, ObjectFilter,
    ObjectId, ObjectKind, Phase, PlayerFilter, PlayerId, PowerToughness, StackEntry, Step,
    Supertype, TriggerQueue, TurnAction, TurnRunner, TurnRunnerState, Until, Zone,
    resolve_stack_entry, run_priority_loop_with,
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

fn queued_upkeep_trigger(
    source: ObjectId,
    game: &GameState,
    controller: PlayerId,
) -> TriggeredAbilityEntry {
    let ability = TriggeredAbility {
        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
        effects: ResolutionProgram::from_effects(vec![Effect::gain_life(1)]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };
    TriggeredAbilityEntry {
        source,
        controller,
        x_value: None,
        event_value_amount: None,
        ability: ability.clone(),
        triggering_event: TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(source, Zone::Hand),
            ProvNodeId::default(),
        ),
        source_stable_id: game.object(source).expect("trigger source").stable_id,
        source_name: "Old queued trigger".to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: TriggeredAbilitySourceKind::Object,
        trigger_identity: compute_trigger_identity(&ability),
    }
}

fn delayed_upkeep_trigger(source: ObjectId, controller: PlayerId) -> DelayedTrigger {
    DelayedTrigger {
        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
        effects: ResolutionProgram::from_effects(vec![Effect::gain_life(1)]),
        one_shot: true,
        x_value: None,
        not_before_turn: None,
        expires_at_turn: None,
        expires_before_controller_turn_after: None,
        expires_at_end_of_combat: false,
        target_objects: Vec::new(),
        ability_source: Some(source),
        ability_source_stable_id: None,
        ability_source_name: Some("Future delayed trigger".to_string()),
        ability_source_snapshot: None,
        controller,
        choices: Vec::new(),
        tagged_objects: std::collections::HashMap::new(),
    }
}

#[test]
fn copied_end_turn_spell_exiles_every_stack_object_and_stops_resolution() {
    let (mut game, alice, _) = game();
    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let end_definition = definition(
        "Copied End Turn",
        vec![CardType::Instant],
        vec![Effect::new(ironsmith::effects::SequenceEffect::new(vec![
            Effect::end_turn(),
            Effect::gain_life(9),
        ]))],
    );
    let original = game.create_object_from_definition(&end_definition, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(original, alice));

    let ability_source = game.create_object_from_definition(
        &creature_definition("Lower Ability Source"),
        alice,
        Zone::Battlefield,
    );
    game.push_to_stack(StackEntry::ability(
        ability_source,
        alice,
        vec![Effect::gain_life(4)],
    ));

    let other_definition = definition("Other Stack Spell", vec![CardType::Sorcery], Vec::new());
    let other = game.create_object_from_definition(&other_definition, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(other, alice));

    let copy_id = game.new_object_id();
    let copy = Object::spell_copy_of(
        game.object(original).expect("original spell"),
        copy_id,
        alice,
    );
    game.add_object(copy);
    game.push_to_stack(StackEntry::new(copy_id, alice));

    resolve_stack_entry(&mut game).expect("copied End the Turn spell resolves");

    assert!(game.stack_is_empty());
    assert!(game.turn_store.end_turn_procedure_pending);
    assert_eq!(
        game.player(alice).expect("Alice").life,
        20,
        "later instructions stop"
    );
    assert_eq!(
        game.exile.len(),
        3,
        "resolving copy and both lower spells are exiled"
    );
    assert_eq!(
        game.object(ability_source)
            .expect("an ordinary ability's source remains")
            .zone,
        Zone::Battlefield
    );
    assert!(
        game.exile
            .iter()
            .filter_map(|id| game.object(*id))
            .any(|object| object.kind == ObjectKind::SpellCopy),
        "the resolving spell copy exists in exile until the SBA pass"
    );
}

#[test]
fn copied_end_turn_ability_exiles_the_ability_copy_but_not_its_source() {
    let (mut game, alice, _) = game();
    game.turn.active_player = alice;
    let source = game.create_object_from_definition(
        &creature_definition("End Turn Ability Source"),
        alice,
        Zone::Battlefield,
    );
    let copy_id = game.new_object_id();
    let copy = Object::spell_copy_of(game.object(source).expect("source"), copy_id, alice);
    game.add_object(copy);
    let mut entry = StackEntry::ability(source, alice, vec![Effect::end_turn()]);
    entry.object_id = copy_id;
    entry.is_ability = true;
    entry.ability_effects = Some(ResolutionProgram::from_effects(vec![Effect::end_turn()]));
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("copied End the Turn ability resolves");

    assert_eq!(
        game.object(source).expect("source remains").zone,
        Zone::Battlefield
    );
    assert!(
        game.exile
            .iter()
            .filter_map(|id| game.object(*id))
            .any(|object| object.kind == ObjectKind::SpellCopy)
    );
}

#[test]
fn priority_loop_yields_to_the_end_turn_scheduler_without_granting_priority() {
    let (mut game, alice, _) = game();
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    let end_definition = definition(
        "Priority End Turn",
        vec![CardType::Instant],
        vec![Effect::end_turn()],
    );
    let spell = game.create_object_from_definition(&end_definition, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, alice));
    let mut trigger_queue = TriggerQueue::new();
    let mut decision_maker = ironsmith::AutoPassDecisionMaker;

    assert!(matches!(
        run_priority_loop_with(&mut game, &mut trigger_queue, &mut decision_maker).unwrap(),
        ironsmith::GameProgress::Continue
    ));
    assert!(game.turn_store.end_turn_procedure_pending);
    assert_eq!(game.turn.priority_player, None);
}

#[test]
fn end_turn_uses_no_priority_sbas_then_cleanup_triggers_priority_and_repeats_cleanup() {
    let (mut game, alice, bob) = game();
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    let source = game.create_object_from_definition(
        &creature_definition("End Turn Source"),
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

    let mut watcher = definition("Exile Watcher", vec![CardType::Enchantment], Vec::new());
    watcher.abilities.push(Ability::triggered(
        Trigger::exiled(ObjectFilter::default()),
        vec![Effect::gain_life(1)],
    ));
    game.create_object_from_definition(&watcher, alice, Zone::Battlefield);

    let mut end_step_watcher =
        definition("End Step Watcher", vec![CardType::Enchantment], Vec::new());
    end_step_watcher.abilities.push(Ability::triggered(
        Trigger::beginning_of_end_step(PlayerFilter::You),
        vec![Effect::gain_life(10)],
    ));
    game.create_object_from_definition(&end_step_watcher, alice, Zone::Battlefield);

    let lower = game.create_object_from_definition(
        &definition("Exiled Stack Spell", vec![CardType::Instant], Vec::new()),
        alice,
        Zone::Stack,
    );
    game.push_to_stack(StackEntry::new(lower, alice));

    game.effect_store
        .delayed_triggers
        .push(delayed_upkeep_trigger(source, alice));
    let mut trigger_queue = TriggerQueue::new();
    trigger_queue.add(queued_upkeep_trigger(source, &game, alice));

    let mut ctx = ironsmith::EffectContext::new_default(source, alice);
    ironsmith::effects::EndTurnEffect::new(PlayerFilter::You)
        .execute(&mut game, &mut ctx)
        .expect("end-turn effect");
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        1,
        "untriggered delayed trigger remains"
    );

    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::DeclareBlockersPriority);
    *runner.combat_mut() = combat;
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(
        trigger_queue.entries.is_empty(),
        "old queued trigger ceased to exist"
    );
    assert!(
        game.stack_is_empty(),
        "724.1c does not stack procedure triggers"
    );
    assert!(runner.combat().attackers.is_empty());
    assert!(
        game.combat
            .as_ref()
            .expect("combat state")
            .attackers
            .is_empty()
    );
    assert_eq!(game.turn.phase, Phase::Ending);
    assert_eq!(game.turn.step, Some(Step::Cleanup));

    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::RunPriority
    ));
    assert_eq!(
        game.stack.len(),
        1,
        "stack-exile trigger waits until cleanup"
    );

    resolve_stack_entry(&mut game).expect("cleanup trigger resolves");
    assert_eq!(game.player(alice).expect("Alice").life, 21);
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::TurnComplete
    ));
    assert_eq!(
        game.player(alice).expect("Alice").life,
        21,
        "ordinary end-step trigger never fires"
    );
}

#[test]
fn end_turn_cleanup_discards_removes_damage_and_expires_eot_effects_resumably() {
    let (mut game, alice, _) = game();
    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    let source = game.create_object_from_definition(
        &creature_definition("Cleanup Source"),
        alice,
        Zone::Battlefield,
    );
    let creature = game.create_object_from_definition(
        &creature_definition("Damaged Pumped Creature"),
        alice,
        Zone::Battlefield,
    );
    game.set_damage_marked(creature, 1);
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::pump(
            source,
            alice,
            creature,
            3,
            3,
            Until::EndOfTurn,
        ));
    assert_eq!(game.calculated_power(creature), Some(5));
    for index in 0..8 {
        let card = definition(&format!("Hand Card {index}"), Vec::new(), Vec::new());
        game.create_object_from_definition(&card, alice, Zone::Hand);
    }

    let mut ctx = ironsmith::EffectContext::new_default(source, alice);
    ironsmith::effects::EndTurnEffect::new(PlayerFilter::You)
        .execute(&mut game, &mut ctx)
        .expect("end-turn effect");
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::FirstMainPriority);
    let mut trigger_queue = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    let discard = match runner.advance(&mut game, &mut trigger_queue).unwrap() {
        TurnAction::Decision(ironsmith::DecisionContext::SelectObjects(context)) => {
            context.candidates[0].id
        }
        other => panic!("expected max-hand cleanup discard, got {other:?}"),
    };
    runner.respond_discard(vec![discard]);
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert_eq!(game.player(alice).expect("Alice").hand.len(), 7);
    assert_eq!(game.damage_on(creature), 0);
    assert_eq!(game.calculated_power(creature), Some(2));
}

#[test]
fn cleanup_sba_without_a_trigger_still_grants_priority_before_repeating_cleanup() {
    let (mut game, alice, _) = game();
    game.turn.active_player = alice;
    let source = game.create_object_from_definition(
        &creature_definition("SBA Cleanup Source"),
        alice,
        Zone::Battlefield,
    );
    let zero = CardDefinition::new(
        CardBuilder::new(CardId::new(), "Temporarily Surviving Zero")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 0))
            .build(),
    );
    let zero = game.create_object_from_definition(&zero, alice, Zone::Battlefield);
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::pump(
            source,
            alice,
            zero,
            1,
            1,
            Until::EndOfTurn,
        ));
    assert_eq!(game.calculated_toughness(zero), Some(1));

    let mut ctx = ironsmith::EffectContext::new_default(source, alice);
    ironsmith::effects::EndTurnEffect::new(PlayerFilter::You)
        .execute(&mut game, &mut ctx)
        .expect("end-turn effect");
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::FirstMainPriority);
    let mut trigger_queue = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::RunPriority
    ));
    assert!(
        game.stack_is_empty(),
        "the cleanup branch was caused only by an SBA"
    );
    assert_eq!(game.turn.priority_player, Some(alice));
    assert!(
        game.object(zero).is_none(),
        "the zero-toughness creature ceased to survive"
    );
}

#[test]
fn end_turn_sba_legend_choice_pauses_and_resumes_before_cleanup() {
    let (mut game, alice, _) = game();
    game.turn.active_player = alice;
    let source = game.create_object_from_definition(
        &creature_definition("Legend End Source"),
        alice,
        Zone::Battlefield,
    );
    let legendary = CardBuilder::new(CardId::new(), "Same Legend")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let first = game.create_object_from_card(&legendary, alice, Zone::Battlefield);
    game.create_object_from_card(&legendary, alice, Zone::Battlefield);

    let mut ctx = ironsmith::EffectContext::new_default(source, alice);
    ironsmith::effects::EndTurnEffect::new(PlayerFilter::You)
        .execute(&mut game, &mut ctx)
        .expect("end-turn effect");
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::FirstMainPriority);
    let mut trigger_queue = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Decision(ironsmith::DecisionContext::SelectObjects(_))
    ));
    assert!(
        game.stack_is_empty(),
        "legend choice occurs without priority"
    );
    runner.respond_discard(vec![first]);
    assert!(matches!(
        runner.advance(&mut game, &mut trigger_queue).unwrap(),
        TurnAction::Continue
    ));
    assert!(matches!(runner.state(), TurnRunnerState::CleanupDiscard));
}
