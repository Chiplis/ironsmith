//! Authored, UNRUN producer regressions. Complete All Will Be One bodies use
//! both direct compilation and serialized artifact materialization. Physical
//! receipts come from Proliferate/DoubleCounters, never synthetic markers.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{ProliferateContext, TargetsContext};
use ironsmith::decisions::specs::ProliferateResponse;
use ironsmith::effect::{EventValueSpec, Value};
use ironsmith::effects::{DoubleCountersEffect, EffectExecutor, EffectContext as ExecutionContext, ExecutionError, ProliferateEffect};
use ironsmith::game_loop::{GameLoopError, try_drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Target;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerQueue, TriggeredAbilityEntry};
use ironsmith::{CardId, CardType, CounterType, GameState, ObjectId, PlayerId, Zone};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const BODY: &str = "Mana cost: {3}{R}{R}\nType: Enchantment\nWhenever you put one or more counters on a permanent or player, this enchantment deals that much damage to target opponent, creature an opponent controls, or planeswalker an opponent controls.";

fn definition(artifact: bool) -> CardDefinition {
    if artifact {
        let compiled = ironsmith_compiler::CompilerFacade::new().compile_definition(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), "All Will Be One"),
            BODY,
            ironsmith_compiler::CompilePolicy { allow_unsupported: false },
        ).unwrap();
        let wire = serde_json::from_value(serde_json::to_value(&compiled.definition).unwrap()).unwrap();
        ironsmith::artifact_materializer::materialize_definition(wire).unwrap()
    } else {
        let oracle = BODY.lines().last().unwrap().to_string();
        ironsmith_tools::compile_definition_from_payload(&ironsmith_tools::CardPayload {
            name: "All Will Be One".into(), parse_name: None,
            oracle_text: oracle.clone(), raw_oracle_text: oracle,
            metadata_lines: BODY.lines().take(2).map(str::to_string).collect(),
            parse_input: BODY.into(), other_face_name: None, linked_face_layout: None,
        }).unwrap()
    }
}

fn permanent(game: &mut GameState, owner: PlayerId) -> ObjectId {
    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Counter recipient")
            .card_types(vec![CardType::Artifact]).build(),
        owner, Zone::Battlefield,
    )
}

fn setup(artifact: bool) -> (GameState, ObjectId, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition(artifact), A, Zone::Battlefield);
    let producer = permanent(&mut game, A);
    (game, source, producer)
}

fn seed(game: &mut GameState, recipient: ObjectId, kinds: &[CounterType], amount: u32) {
    for kind in kinds {
        game.object_mut(recipient).unwrap().counters.insert(*kind, amount);
    }
}

#[derive(Clone, Default)]
struct Picks {
    objects: Vec<ObjectId>,
    players: Vec<PlayerId>,
    target_prompts: usize,
}
impl DecisionMaker for Picks {
    fn decide_proliferate(&mut self, _: &GameState, _: &ProliferateContext) -> ProliferateResponse {
        ProliferateResponse { permanents: self.objects.clone(), players: self.players.clone() }
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.target_prompts += 1;
        assert_eq!(ctx.requirements.len(), 1);
        assert!(ctx.requirements[0].legal_targets.contains(&Target::Player(B)));
        vec![Target::Player(B)]
    }
}

struct PauseTarget(bool);
impl DecisionMaker for PauseTarget {
    fn answers_player_choices(&self) -> bool { true }
    fn awaiting_choice(&self) -> bool { self.0 }
    fn decide_targets(&mut self, _: &GameState, _: &TargetsContext) -> Vec<Target> {
        self.0 = true;
        Vec::new()
    }
}

fn proliferate(game: &mut GameState, producer: ObjectId, actor: PlayerId, picks: &mut Picks, count: i32) -> ironsmith::effect::EffectOutcome {
    let mut context = ExecutionContext::new(producer, actor, picks);
    ProliferateEffect::new(count).execute(game, &mut context).unwrap()
}

fn publish(game: &mut GameState, outcomes: impl IntoIterator<Item = ironsmith::effect::EffectOutcome>) -> TriggerQueue {
    for outcome in outcomes {
        for event in outcome.events {
            game.queue_trigger_event(event.provenance(), event);
        }
    }
    let mut queue = TriggerQueue::new();
    try_drain_pending_trigger_events(game, &mut queue).unwrap();
    queue
}

fn amount(game: &GameState, entry: &TriggeredAbilityEntry) -> i64 {
    let mut context = ExecutionContext::new_default(entry.source, entry.controller)
        .with_triggering_event(entry.triggering_event.clone());
    context.event_value_amount = entry.event_value_amount;
    ironsmith::effects::helpers::resolve_value_wide(
        game, &Value::EventValue(EventValueSpec::Amount), &context,
    ).unwrap()
}

#[test]
fn proliferate_groups_kinds_by_exact_permanent_or_player_and_resolves_saved_sums() {
    for artifact in [false, true] {
        let (mut game, source, producer) = setup(artifact);
        let first = permanent(&mut game, B);
        let second = permanent(&mut game, A);
        seed(&mut game, first, &[CounterType::Charge, CounterType::Stun], 3);
        seed(&mut game, second, &[CounterType::Charge, CounterType::Stun, CounterType::Time], 1);
        game.player_mut(B).unwrap().energy_counters = 2;
        game.player_mut(B).unwrap().experience_counters = 4;
        let mut picks = Picks { objects: vec![first, second], players: vec![B], ..Default::default() };
        let outcome = proliferate(&mut game, producer, A, &mut picks, 1);
        let physical: Vec<_> = outcome.events.iter()
            .filter_map(|event| event.downcast::<ironsmith::events::MarkersChangedEvent>()).collect();
        assert_eq!(physical.len(), 7);
        assert!(physical.iter().all(|event| event.amount == 1 && event.source_controller == Some(A)));
        let mut queue = publish(&mut game, [outcome]);
        let grouped: Vec<_> = queue.entries.iter().map(|entry| amount(&game, entry)).collect();
        assert_eq!(grouped, vec![2, 3, 2], "first receipt order and distinct recipient identity survive");
        for entry in &queue.entries {
            assert_eq!(entry.triggering_event.downcast::<ironsmith::events::MarkersChangedEvent>().unwrap().amount, 1,
                "group projection must not rewrite the physical kind receipt");
        }
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut picks).unwrap();
        assert_eq!(game.stack.len(), 3);
        assert!(game.stack.iter().all(|entry| entry.targets == vec![Target::Player(B)]));
        // Both recipient state and source control can change after matching.
        game.object_mut(first).unwrap().counters.clear();
        game.object_mut(second).unwrap().counters.clear();
        game.set_current_controller(source, B).unwrap();
        for _ in 0..3 { resolve_stack_entry_with(&mut game, &mut picks).unwrap(); }
        assert_eq!(picks.target_prompts, 3);
        assert_eq!(game.player(B).unwrap().life, 13);
        assert_eq!(game.player(A).unwrap().life, 20);
        assert!(game.stack.is_empty());
    }
}

#[test]
fn repeated_proliferation_and_separate_instructions_keep_independent_recipient_sums() {
    for artifact in [false, true] {
        for one_repeated_effect in [false, true] {
            let (mut game, _, producer) = setup(artifact);
            let recipient = permanent(&mut game, B);
            seed(&mut game, recipient, &[CounterType::Charge, CounterType::Stun], 1);
            let mut picks = Picks { objects: vec![recipient], ..Default::default() };
            let outcomes = if one_repeated_effect {
                vec![proliferate(&mut game, producer, A, &mut picks, 2)]
            } else {
                vec![proliferate(&mut game, producer, A, &mut picks, 1),
                    proliferate(&mut game, producer, A, &mut picks, 1)]
            };
            let mut queue = publish(&mut game, outcomes);
            assert_eq!(queue.entries.iter().map(|entry| amount(&game, entry)).collect::<Vec<_>>(), vec![2, 2]);
            assert_ne!(queue.entries[0].triggering_event.simultaneous_batch(), queue.entries[1].triggering_event.simultaneous_batch());
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut picks).unwrap();
            for _ in 0..2 { resolve_stack_entry_with(&mut game, &mut picks).unwrap(); }
            assert_eq!(game.player(B).unwrap().life, 16);
            assert_eq!(picks.target_prompts, 2);
        }
    }
}

#[test]
fn unbatched_counter_receipts_never_coalesce_by_recipient_alone() {
    for artifact in [false, true] {
        let (mut game, _, producer) = setup(artifact);
        let recipient = permanent(&mut game, B);
        for kind in [CounterType::Charge, CounterType::Stun] {
            let event = game.add_counters_with_source(recipient, kind, 2, Some(producer), Some(A)).unwrap();
            assert!(event.simultaneous_batch().is_none());
            game.queue_trigger_event(event.provenance(), event);
        }
        let mut queue = TriggerQueue::new();
        try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        assert_eq!(queue.entries.iter().map(|entry| amount(&game, entry)).collect::<Vec<_>>(), vec![2, 2]);
    }
}

#[test]
fn actual_placement_actor_is_independent_of_recipient_and_producer_ownership() {
    for artifact in [false, true] {
        for actor in [A, B] {
            let (mut game, _, producer) = setup(artifact);
            let recipient = permanent(&mut game, B);
            seed(&mut game, recipient, &[CounterType::Charge, CounterType::Stun], 1);
            game.player_mut(B).unwrap().energy_counters = 1;
            game.player_mut(B).unwrap().experience_counters = 1;
            // The source is owned by Alice even when Bob performs the action.
            let mut picks = Picks { objects: vec![recipient], players: vec![B], ..Default::default() };
            let outcome = proliferate(&mut game, producer, actor, &mut picks, 1);
            let queue = publish(&mut game, [outcome]);
            assert_eq!(queue.entries.len(), if actor == A { 2 } else { 0 });
            assert!(queue.entries.iter().all(|entry| amount(&game, entry) == 2));
            assert_eq!(game.counter_count(recipient, CounterType::Charge), 2);
            assert_eq!(game.player(B).unwrap().energy_counters, 2);
        }
    }
}

#[test]
fn identical_ability_instances_each_receive_one_complete_sum() {
    for artifact in [false, true] {
        let (mut game, source, producer) = setup(artifact);
        let ability = game.object(source).unwrap().abilities[0].clone();
        game.object_mut(source).unwrap().abilities_mut().push(ability);
        let recipient = permanent(&mut game, B);
        seed(&mut game, recipient, &[CounterType::Charge, CounterType::Stun], 1);
        let mut picks = Picks { objects: vec![recipient], ..Default::default() };
        let outcome = proliferate(&mut game, producer, A, &mut picks, 1);
        let queue = publish(&mut game, [outcome]);
        assert_eq!(queue.entries.iter().map(|entry| amount(&game, entry)).collect::<Vec<_>>(), vec![2, 2]);
    }
}

#[test]
fn recipient_amount_survives_target_suspension_and_native_stack_checkpoint() {
    for artifact in [false, true] {
        let (mut game, _, producer) = setup(artifact);
        let recipient = permanent(&mut game, B);
        seed(&mut game, recipient, &[CounterType::Charge, CounterType::Stun], 1);
        let mut picks = Picks { objects: vec![recipient], ..Default::default() };
        let outcome = proliferate(&mut game, producer, A, &mut picks, 1);
        let mut queue = publish(&mut game, [outcome]);
        let mut pause = PauseTarget(false);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut pause).unwrap();
        assert!(pause.0);
        assert!(game.stack.is_empty());
        assert_eq!(queue.entries.len(), 1);
        assert_eq!(amount(&game, &queue.entries[0]), 2);
        // Native replay/suspension keeps value-owned envelope metadata.
        let mut resumed_game = game.clone();
        let mut resumed_queue = queue.clone();
        put_triggers_on_stack_with_dm(&mut resumed_game, &mut resumed_queue, &mut picks).unwrap();
        assert_eq!(resumed_game.stack.len(), 1);
        let mut restored_stack = resumed_game.clone();
        resolve_stack_entry_with(&mut restored_stack, &mut picks).unwrap();
        assert_eq!(restored_stack.player(B).unwrap().life, 18);
        assert_eq!(picks.target_prompts, 1);
        assert_eq!(game.player(B).unwrap().life, 20, "the suspended checkpoint stays independent");
    }
}

#[test]
fn plural_objects_each_counter_and_kind_filtered_watchers_keep_their_contracts() {
    let (mut game, _, producer) = setup(false);
    let mut watchers = Vec::new();
    for (index, trigger) in [
        ironsmith::triggers::CounterPutOnTrigger::new(ironsmith::target::ObjectFilter::permanent())
            .count(ironsmith::triggers::CountMode::OneOrMore).one_or_more_objects(),
        ironsmith::triggers::CounterPutOnTrigger::new(ironsmith::target::ObjectFilter::permanent())
            .count(ironsmith::triggers::CountMode::Each),
        ironsmith::triggers::CounterPutOnTrigger::new(ironsmith::target::ObjectFilter::permanent())
            .count(ironsmith::triggers::CountMode::OneOrMore).counter_type(CounterType::Charge),
    ].into_iter().enumerate() {
        let card = CardDefinitionBuilder::new(CardId::new(), &format!("Counter watcher {index}"))
            .card_types(vec![CardType::Enchantment])
            .with_trigger(ironsmith::triggers::Trigger::new(trigger), Vec::new()).build();
        watchers.push(game.create_object_from_definition(&card, A, Zone::Battlefield));
    }
    let first = permanent(&mut game, B);
    let second = permanent(&mut game, B);
    seed(&mut game, first, &[CounterType::Charge, CounterType::Stun], 1);
    seed(&mut game, second, &[CounterType::Charge, CounterType::Stun, CounterType::Time], 1);
    let mut picks = Picks { objects: vec![first, second], ..Default::default() };
    let outcome = proliferate(&mut game, producer, A, &mut picks, 1);
    let queue = publish(&mut game, [outcome]);
    for (watcher, expected) in watchers.into_iter().zip([1, 5, 2]) {
        assert_eq!(queue.entries.iter().filter(|entry| entry.source == watcher).count(), expected);
    }
}

#[test]
fn double_counters_keeps_wide_group_amount_and_rejects_damage_narrowing_without_mutation() {
    for artifact in [false, true] {
        for kind_count in [2, 3] {
            let (mut game, _, producer) = setup(artifact);
            let recipient = permanent(&mut game, B);
            let kinds = [CounterType::Charge, CounterType::Stun, CounterType::Time];
            seed(&mut game, recipient, &kinds[..kind_count], i32::MAX as u32);
            let outcome = DoubleCountersEffect::new(None, ChooseSpec::SpecificObject(recipient))
                .execute(&mut game, &mut ExecutionContext::new_default(producer, A)).unwrap();
            let mut queue = publish(&mut game, [outcome]);
            assert_eq!(queue.entries.len(), 1);
            let total = i64::from(i32::MAX) * kind_count as i64;
            assert_eq!(amount(&game, &queue.entries[0]), total, "the i32 trigger override must not narrow counters");
            assert!(total > i64::from(i32::MAX));
            for kind in &kinds[..kind_count] {
                assert_eq!(game.counter_count(recipient, *kind), (i32::MAX as u32) * 2);
            }
            if kind_count == 3 {
                let mut picks = Picks::default();
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut picks).unwrap();
                assert!(matches!(resolve_stack_entry_with(&mut game, &mut picks),
                    Err(GameLoopError::ExecutionFailed(ExecutionError::UnresolvableValue(_)))));
                assert_eq!(game.player(B).unwrap().life, 20);
                assert_eq!(game.stack.len(), 1, "failed damage resolution retains the queued amount for recovery");
                assert_eq!(picks.target_prompts, 1);
            }
        }
    }
}

#[test]
fn delayed_counter_watchers_group_before_one_shot_alternatives() {
    for artifact in [false, true] {
        for one_shot in [false, true] {
            let (mut game, source, producer) = setup(artifact);
            let ability = game.object(source).unwrap().abilities.iter().find_map(|ability| {
                if let AbilityKind::Triggered(triggered) = &ability.kind { Some(triggered.clone()) } else { None }
            }).unwrap();
            game.object_mut(source).unwrap().abilities_mut().clear();
            ironsmith::effects::delayed::queue_delayed_trigger(&mut game,
                ironsmith::effects::delayed::DelayedTriggerConfig::new(
                    ability.trigger, ability.effects, one_shot, Vec::new(), A,
                ).with_ability_source(Some(source)));
            let first = permanent(&mut game, B);
            let second = permanent(&mut game, B);
            seed(&mut game, first, &[CounterType::Charge, CounterType::Stun], 1);
            seed(&mut game, second, &[CounterType::Charge, CounterType::Stun, CounterType::Time], 1);
            let mut picks = Picks { objects: vec![first, second], ..Default::default() };
            let outcome = proliferate(&mut game, producer, A, &mut picks, 1);
            let queue = publish(&mut game, [outcome]);
            assert_eq!(queue.entries.iter().map(|entry| amount(&game, entry)).collect::<Vec<_>>(),
                if one_shot { vec![2] } else { vec![2, 3] });
            assert_eq!(game.effect_store.delayed_triggers.len(), if one_shot { 0 } else { 1 });
        }
    }
}
