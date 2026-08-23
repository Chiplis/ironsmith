#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;

const ARCBOUND_TRACKER_TEXT: &str = "Menace\nModular 2\nWhenever you cast a spell other than your first spell each turn, put a +1/+1 counter on this creature.";
const CURSE_TEXT: &str = "Enchant player\nWhenever enchanted player casts a spell other than the first spell they cast each turn or copies a spell, this Aura deals 2 damage to that player.";

fn test_spell(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build()
}

fn spell_cast_event(
    game: &crate::GameState,
    spell: ObjectId,
    caster: PlayerId,
) -> crate::triggers::TriggerEvent {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell).expect("test spell should exist"),
        game,
    );
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new_with_snapshot(
            spell,
            caster,
            Zone::Hand,
            snapshot,
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn spell_copied_event(spell: ObjectId, copier: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCopiedEvent::new(spell, copier),
        crate::provenance::ProvNodeId::default(),
    )
}

fn matching_entries(
    game: &crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect()
}

fn resolve_entries(
    game: &mut crate::GameState,
    entries: Vec<crate::triggers::TriggeredAbilityEntry>,
) {
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    let mut decisions = SelectFirstDecisionMaker;
    crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut decisions)
        .expect("matching trigger should go on the stack");
    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry_with(game, &mut decisions)
            .expect("matching trigger should resolve");
    }
}

fn record_cast_and_find_entries(
    game: &mut crate::GameState,
    source: ObjectId,
    caster: PlayerId,
    name: &str,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    let spell = game.create_object_from_definition(&test_spell(name), caster, Zone::Stack);
    let event = spell_cast_event(game, spell, caster);
    game.record_turn_history_event(&event);
    matching_entries(game, source, &event)
}

fn setup_curse_game() -> (crate::GameState, PlayerId, PlayerId, ObjectId, ObjectId) {
    let definition = parse_oracle_card_definition("Curse of Shaken Faith");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.object_mut(source)
        .expect("Curse should exist")
        .attached_to = Some(crate::object::AttachmentTarget::Player(bob));
    game.player_mut(bob)
        .expect("enchanted player should exist")
        .attachments
        .push(source);
    let copied_spell =
        game.create_object_from_definition(&test_spell("Copied Spell"), bob, Zone::Stack);
    (game, alice, bob, source, copied_spell)
}

#[test]
fn named_definitions_preserve_other_than_first_and_shared_copy_actor() {
    let tracker = parse_oracle_card_definition("Arcbound Tracker");
    assert_eq!(
        canonical_compiled_lines(&tracker).join("\n"),
        ARCBOUND_TRACKER_TEXT
    );
    let curse = parse_oracle_card_definition("Curse of Shaken Faith");
    assert_eq!(canonical_compiled_lines(&curse).join("\n"), CURSE_TEXT);
    let triggered = curse
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Curse should have its triggered ability");
    let either = triggered
        .trigger
        .downcast_ref::<crate::triggers::OrTrigger>()
        .expect("Curse should retain the cast-or-copy union");
    let cast = either
        .triggers
        .iter()
        .find_map(|trigger| trigger.downcast_ref::<crate::triggers::SpellCastTrigger>())
        .expect("cast arm");
    let copied = either
        .triggers
        .iter()
        .find_map(|trigger| trigger.downcast_ref::<crate::triggers::SpellCopiedTrigger>())
        .expect("copy arm");
    let enchanted = PlayerFilter::TaggedPlayer(crate::tag::TagKey::from("enchanted"));
    assert_eq!(cast.caster, enchanted);
    assert_eq!(cast.min_spells_this_turn, Some(2));
    assert_eq!(copied.copier, enchanted);
}

#[test]
fn arcbound_tracker_triggers_on_second_and_later_controller_casts_only() {
    let definition = parse_oracle_card_definition("Arcbound Tracker");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let initial_counters = game.counter_count(source, CounterType::PlusOnePlusOne);

    assert!(
        record_cast_and_find_entries(&mut game, source, alice, "Alice First").is_empty(),
        "the first spell each turn must not trigger Arcbound Tracker"
    );
    assert!(
        record_cast_and_find_entries(&mut game, source, bob, "Bob First").is_empty(),
        "an opponent's cast must not trigger Arcbound Tracker"
    );

    let second = record_cast_and_find_entries(&mut game, source, alice, "Alice Second");
    assert_eq!(second.len(), 1);
    resolve_entries(&mut game, second);
    assert_eq!(
        game.counter_count(source, CounterType::PlusOnePlusOne),
        initial_counters + 1
    );

    let third = record_cast_and_find_entries(&mut game, source, alice, "Alice Third");
    assert_eq!(
        third.len(),
        1,
        "the trigger is minimum-two, not exactly-two"
    );
    resolve_entries(&mut game, third);
    assert_eq!(
        game.counter_count(source, CounterType::PlusOnePlusOne),
        initial_counters + 2
    );
}

#[test]
fn curse_cast_branch_tracks_only_the_enchanted_players_second_and_later_spells() {
    let (mut game, alice, bob, source, _copied_spell) = setup_curse_game();

    assert!(
        record_cast_and_find_entries(&mut game, source, bob, "Bob First").is_empty(),
        "enchanted player's first cast must not trigger the Curse"
    );
    let second = record_cast_and_find_entries(&mut game, source, bob, "Bob Second");
    assert_eq!(second.len(), 1);
    resolve_entries(&mut game, second);
    assert_eq!(game.life_total(bob), 18);
    assert_eq!(game.life_total(alice), 20);

    assert!(record_cast_and_find_entries(&mut game, source, alice, "Alice First").is_empty());
    assert!(
        record_cast_and_find_entries(&mut game, source, alice, "Alice Second").is_empty(),
        "the Aura controller is not the enchanted player"
    );
}

#[test]
fn curse_copy_branch_tracks_only_the_enchanted_player_and_resolves_damage() {
    let (mut game, alice, bob, source, copied_spell) = setup_curse_game();

    let controllers_copy = spell_copied_event(copied_spell, alice);
    assert!(
        matching_entries(&game, source, &controllers_copy).is_empty(),
        "a copy made by the Aura controller must not trigger the enchanted-player branch"
    );

    let enchanted_copy = spell_copied_event(copied_spell, bob);
    let entries = matching_entries(&game, source, &enchanted_copy);
    assert_eq!(entries.len(), 1);
    resolve_entries(&mut game, entries);
    assert_eq!(game.life_total(bob), 18);
    assert_eq!(game.life_total(alice), 20);
}
