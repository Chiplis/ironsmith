//! Captain Howler, Sea Scourge: the discard trigger pumps a target creature
//! and schedules a delayed trigger that must watch that creature, not the
//! ability's source, for combat damage to a player.

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::effects::ScheduleDelayedTriggerEffect;
use ironsmith::game_state::{StackEntry, Target, TargetAssignment};
use ironsmith::target::ChooseSpec;
use ironsmith::events::{DamageEvent, DamageTarget};
use ironsmith::ids::CardId;
use ironsmith::triggers::TriggerEvent;
use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone};

const TEXT: &str = "Ward—{2}, Pay 2 life.\nWhenever you discard one or more cards, target creature gets +2/+0 until end of turn for each card discarded this way. Whenever that creature deals combat damage to a player this turn, draw a card.";

fn howler() -> CardDefinition {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(CardId::from_raw(1), "Captain Howler, Sea Scourge");
    ironsmith_registry::compile_builder_to_runtime_definition(builder, TEXT.to_string(), false)
        .expect("Captain Howler compiles")
}

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn combat_damage_to_player(source: ObjectId, player: PlayerId) -> TriggerEvent {
    TriggerEvent::new_with_provenance(
        DamageEvent::with_cause(
            source,
            DamageTarget::Player(player),
            2,
            true,
            ironsmith::events::cause::EventCause::effect(),
        ),
        ironsmith::provenance::ProvNodeId::default(),
    )
}

#[test]
fn captain_howler_watches_the_pumped_creature_for_the_draw() {
    let definition = howler();
    let discard_trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered.effects.flattened_default_effects().iter().any(|effect| {
                    effect.downcast_ref::<ScheduleDelayedTriggerEffect>().is_some()
                }) =>
            {
                Some(triggered.clone())
            }
            _ => None,
        })
        .expect("the discard trigger schedules the linked draw watcher");

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let pumped = game.create_object_from_definition(&creature("Pumped"), alice, Zone::Battlefield);
    let bystander = game.create_object_from_definition(&creature("Bystander"), alice, Zone::Battlefield);

    // The trigger resolves from the stack with the pumped creature as its target,
    // the way the game loop runs it.
    game.push_to_stack(
        StackEntry::ability(source, alice, discard_trigger.effects.clone())
            .with_targets(vec![Target::Object(pumped)])
            // The targeting phase records which requirement the target answers.
            .with_target_assignments(vec![TargetAssignment {
                spec: ChooseSpec::Object(ironsmith::target::ObjectFilter::creature()),
                range: 0..1,
            }])
            // "for each card discarded this way": one card was discarded.
            .with_event_value_amount(1),
    );
    ironsmith::resolve_stack_entry(&mut game).unwrap_or_else(|error| panic!("the discard trigger did not resolve: {error:?}"));

    assert_eq!(game.effect_store.delayed_triggers.len(), 1, "one linked draw watcher");
    assert_eq!(
        game.effect_store.delayed_triggers[0].target_objects,
        vec![pumped],
        "the watcher follows the pumped creature, not the source"
    );
    assert!(
        ironsmith::triggers::check_delayed_triggers(&mut game, &combat_damage_to_player(bystander, bob)).is_empty(),
        "another creature's combat damage does not draw"
    );
    let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &combat_damage_to_player(pumped, bob));
    assert_eq!(entries.len(), 1, "the pumped creature's combat damage to a player draws");

    // The same through the real combat damage step: the pumped creature attacks
    // Bob unblocked, and the step's events must queue the draw trigger.
    let mut combat = ironsmith::combat_state::CombatState::default();
    combat.attackers.push(ironsmith::combat_state::AttackerInfo {
        creature: pumped,
        target: ironsmith::combat_state::AttackTarget::Player(bob),
    });
    combat.blockers.insert(pumped, Vec::new());
    let life_before = game.player(bob).expect("bob exists").life;
    let events = ironsmith::execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(game.player(bob).expect("bob exists").life, life_before - 4, "2/2 pumped +2/+0 hits for 4");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::queue_combat_damage_triggers(&mut game, &events, &mut queue);
    assert_eq!(queue.entries.len(), 1, "the combat damage step queues the linked draw");
}

