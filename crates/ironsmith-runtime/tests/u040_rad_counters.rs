use ironsmith::effects::RadiationEffect;
use ironsmith::triggers::TriggeredAbilitySourceKind;
use ironsmith::{
    CardBuilder, CardId, CardType, CounterType, EffectContext, EffectExecutor, GameEventType,
    GameState, LifeLossEvent, Phase, PlayerId, TriggerQueue, Zone,
    generate_and_queue_step_triggers, put_triggers_on_stack, resolve_stack_entry,
};

fn game() -> (GameState, PlayerId, PlayerId) {
    let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    (game, PlayerId::from_index(0), PlayerId::from_index(1))
}

fn add_rad(game: &mut GameState, player: PlayerId, amount: u32) {
    game.add_player_counters_with_source(player, CounterType::Rad, amount, None, None)
        .expect("rad counters are added");
}

fn add_library_card(game: &mut GameState, owner: PlayerId, name: &str, card_type: CardType) {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build();
    game.create_object_from_card(&card, owner, Zone::Library);
}

fn begin_precombat_main(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}

#[test]
fn u040_only_precombat_main_with_rad_queues_a_sourceless_rule_trigger() {
    let (mut game, alice, _) = game();
    begin_precombat_main(&mut game, alice);
    let mut queue = TriggerQueue::new();

    generate_and_queue_step_triggers(&mut game, &mut queue);
    assert!(queue.is_empty(), "zero rad counters must not trigger");

    add_rad(&mut game, alice, 3);
    generate_and_queue_step_triggers(&mut game, &mut queue);
    assert_eq!(queue.entries.len(), 1);
    let entry = &queue.entries[0];
    assert_eq!(entry.controller, alice);
    assert_eq!(entry.source_kind, TriggeredAbilitySourceKind::GameRule);
    assert!(entry.source_snapshot.is_none());
    assert!(
        game.object(entry.source).is_none(),
        "the ability has no source"
    );

    queue.clear();
    game.turn.phase = Phase::NextMain;
    generate_and_queue_step_triggers(&mut game, &mut queue);
    assert!(
        queue.is_empty(),
        "postcombat main phases do not cause radiation"
    );
}

#[test]
fn u040_intervening_if_is_checked_before_stacking_and_at_resolution() {
    let (mut game, alice, _) = game();
    begin_precombat_main(&mut game, alice);
    add_rad(&mut game, alice, 1);
    add_library_card(&mut game, alice, "Top Spell", CardType::Instant);
    let mut queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut queue);

    game.remove_player_counters_with_source(alice, CounterType::Rad, 1, None, None)
        .expect("rad counter removed before stacking");
    put_triggers_on_stack(&mut game, &mut queue).expect("stack trigger pass");
    assert!(game.stack_is_empty());
    assert_eq!(game.player(alice).expect("Alice").library.len(), 1);

    add_rad(&mut game, alice, 1);
    generate_and_queue_step_triggers(&mut game, &mut queue);
    put_triggers_on_stack(&mut game, &mut queue).expect("stack radiation");
    assert_eq!(game.stack.len(), 1);
    game.remove_player_counters_with_source(alice, CounterType::Rad, 1, None, None)
        .expect("rad counter removed before resolution");
    resolve_stack_entry(&mut game).expect("resolve radiation");
    assert_eq!(game.player(alice).expect("Alice").library.len(), 1);
    assert_eq!(game.player(alice).expect("Alice").life, 20);
}

#[test]
fn u040_resolution_uses_current_rad_and_counts_only_nonlands_actually_milled() {
    let (mut game, alice, _) = game();
    begin_precombat_main(&mut game, alice);
    add_rad(&mut game, alice, 4);
    add_library_card(&mut game, alice, "Land One", CardType::Land);
    add_library_card(&mut game, alice, "Spell One", CardType::Instant);
    add_library_card(&mut game, alice, "Land Two", CardType::Land);
    add_library_card(&mut game, alice, "Creature One", CardType::Creature);

    let mut queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut queue);
    put_triggers_on_stack(&mut game, &mut queue).expect("stack radiation");
    resolve_stack_entry(&mut game).expect("resolve radiation");

    let player = game.player(alice).expect("Alice");
    assert!(player.library.is_empty());
    assert_eq!(player.graveyard.len(), 4);
    assert_eq!(player.life, 18);
    assert_eq!(player.counter_count(CounterType::Rad), 2);
}

#[test]
fn u040_short_library_and_cant_lose_life_still_remove_rad_per_nonland() {
    let (mut game, alice, _) = game();
    add_rad(&mut game, alice, 3);
    add_library_card(&mut game, alice, "Only Card", CardType::Sorcery);
    game.effect_store.cant_effects.cant_lose_life.insert(alice);
    let source = game.new_object_id();
    let mut ctx = EffectContext::new_default(source, alice);

    let outcome = RadiationEffect::new()
        .execute(&mut game, &mut ctx)
        .expect("resolve radiation effect");

    let player = game.player(alice).expect("Alice");
    assert_eq!(player.life, 20);
    assert_eq!(player.counter_count(CounterType::Rad), 2);
    assert_eq!(player.graveyard.len(), 1);
    assert!(outcome.events.iter().all(|event| {
        event
            .downcast::<LifeLossEvent>()
            .is_none_or(|loss| !loss.from_radiation)
    }));
}

#[test]
fn u040_life_loss_events_are_identified_as_from_radiation() {
    let (mut game, alice, _) = game();
    add_rad(&mut game, alice, 1);
    add_library_card(&mut game, alice, "Rad Spell", CardType::Instant);
    let source = game.new_object_id();
    let mut ctx = EffectContext::new_default(source, alice);

    let outcome = RadiationEffect::new()
        .execute(&mut game, &mut ctx)
        .expect("resolve radiation effect");
    let losses = outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<LifeLossEvent>())
        .collect::<Vec<_>>();
    assert_eq!(losses.len(), 1);
    assert_eq!(losses[0].amount, 1);
    assert!(!losses[0].from_damage);
    assert!(losses[0].from_radiation);
    assert!(losses[0].source_object().is_none());
}
