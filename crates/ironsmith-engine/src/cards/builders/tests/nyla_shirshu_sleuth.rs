#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn creature_card(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

fn entering_event(game: &crate::GameState, object: ObjectId) -> crate::triggers::TriggerEvent {
    let mut snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(object).expect("Nyla should exist"),
        game,
    );
    snapshot.zone = Zone::Stack;
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            object,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn source_entries(
    game: &crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect()
}

#[test]
fn nyla_uses_the_exiled_graveyard_cards_mana_value_and_returns_that_card() {
    let definition = parse_oracle_card_definition("Nyla, Shirshu Sleuth");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let nyla = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let exiled_candidate = game.create_object_from_definition(
        &creature_card("Four-Mana Graveyard Creature", 4),
        alice,
        Zone::Graveyard,
    );
    let exiled_stable = game
        .object(exiled_candidate)
        .expect("graveyard creature should exist")
        .stable_id;
    game.create_object_from_definition(
        &creature_card("Seven-Mana Hand Decoy", 7),
        alice,
        Zone::Hand,
    );

    let etb = entering_event(&game, nyla);
    let entries = source_entries(&game, nyla, &etb);
    assert_eq!(entries.len(), 1, "Nyla should have one ETB trigger");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("Nyla's ETB trigger should go on the stack");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Nyla's ETB trigger should resolve");

    assert_eq!(
        game.life_total(alice),
        16,
        "life loss must use the exiled four-mana card, not a card in hand"
    );
    let clues = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).is_some_and(|object| object.name == "Clue"))
        .collect::<Vec<_>>();
    assert_eq!(
        clues.len(),
        4,
        "the same exiled card should set the Clue count"
    );
    let exiled = game
        .find_object_by_stable_id(exiled_stable)
        .and_then(|id| game.object(id))
        .expect("the exiled card should remain identifiable");
    assert_eq!(exiled.zone, Zone::Exile);

    let end_step = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        source_entries(&game, nyla, &end_step).is_empty(),
        "Nyla's return trigger must remain gated while its controller has Clues"
    );
    for clue in clues {
        game.move_object(
            clue,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
        )
        .expect("test Clue should leave the battlefield");
    }

    let entries = source_entries(&game, nyla, &end_step);
    assert_eq!(
        entries.len(),
        1,
        "no Clues should enable Nyla's end-step trigger"
    );
    let entry = entries
        .into_iter()
        .next()
        .expect("Nyla's enabled return trigger should be available");
    let mut context = crate::effects::ExecutionContext::new(nyla, alice, &mut decisions)
        .with_triggering_event(entry.triggering_event)
        .with_tagged_objects(entry.tagged_objects);
    if let Some(snapshot) = entry.source_snapshot {
        context = context.with_source_snapshot(snapshot);
    }
    let exiled_snapshot = game
        .find_object_by_stable_id(exiled_stable)
        .and_then(|id| game.object(id))
        .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, &game))
        .expect("Nyla's source-linked exiled card should still exist");
    context.set_tagged_objects("__source_exiled__", vec![exiled_snapshot]);
    for effect in entry.ability.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut context)
            .expect("Nyla's return trigger should resolve");
    }

    let returned = game
        .find_object_by_stable_id(exiled_stable)
        .and_then(|id| game.object(id))
        .expect("the returned card should remain identifiable");
    assert_eq!(returned.zone, Zone::Hand);
    assert_eq!(returned.owner, alice);
}
