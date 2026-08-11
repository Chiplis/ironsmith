#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::object::ObjectKind;

#[derive(Default)]
struct RecordingBooleanDecisionMaker {
    answer: bool,
    players: Vec<PlayerId>,
}

impl RecordingBooleanDecisionMaker {
    fn answering(answer: bool) -> Self {
        Self {
            answer,
            players: Vec::new(),
        }
    }
}

impl DecisionMaker for RecordingBooleanDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        context: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.players.push(context.player);
        self.answer
    }
}

fn vanilla_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn spell_definition(name: &str, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

fn recorded_spell(
    game: &mut crate::GameState,
    caster: PlayerId,
    name: &str,
    card_type: CardType,
    keep_on_stack: bool,
) -> ObjectId {
    let definition = spell_definition(name, card_type);
    let spell = game.create_object_from_definition(&definition, caster, Zone::Stack);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell).expect("recorded spell should exist"),
        game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new_with_snapshot(
            spell,
            caster,
            Zone::Hand,
            snapshot,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
    if keep_on_stack {
        game.push_to_stack(crate::game_state::StackEntry::new(spell, caster));
    } else {
        game.move_object_by_effect(spell, Zone::Graveyard)
            .expect("resolved spell should leave the stack");
    }
    spell
}

fn breathkeeper_death_trigger_count(paired: bool) -> usize {
    let definition = parse_oracle_card_definition("Breathkeeper Seraph");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let seraph = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let partner = game.create_object_from_definition(
        &vanilla_creature("Breathkeeper Partner"),
        alice,
        Zone::Battlefield,
    );
    if paired {
        game.set_soulbond_pair(seraph, partner);
    }

    game.move_object_by_effect(partner, Zone::Graveyard)
        .expect("partner should die");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    queue
        .entries
        .iter()
        .filter(|entry| entry.source == partner)
        .count()
}

#[test]
fn breathkeeper_seraph_returns_a_paired_creature_only_at_its_owners_next_upkeep() {
    let definition = parse_oracle_card_definition("Breathkeeper Seraph");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let seraph = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let partner = game.create_object_from_definition(
        &vanilla_creature("Breathkeeper Partner"),
        alice,
        Zone::Battlefield,
    );
    let stable_id = game
        .object(partner)
        .expect("partner should exist")
        .stable_id;
    game.set_soulbond_pair(seraph, partner);

    game.move_object_by_effect(partner, Zone::Graveyard)
        .expect("paired partner should die");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(
        queue
            .entries
            .iter()
            .filter(|entry| entry.source == partner)
            .count(),
        1,
        "the paired creature should carry exactly one granted dies trigger"
    );

    let mut decisions = RecordingBooleanDecisionMaker::answering(true);
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("the granted dies trigger should go on the stack");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("the granted dies trigger should schedule the return");
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert_eq!(decisions.players, vec![alice]);

    let same_turn_upkeep = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &same_turn_upkeep).is_empty(),
        "the delayed return must not fire during the turn it was created"
    );

    game.turn.turn_number += 2;
    game.turn.active_player = alice;
    let next_upkeep = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(&mut game, &next_upkeep) {
        queue.add(trigger);
    }
    assert_eq!(queue.entries.len(), 1, "the next upkeep should fire once");
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("the delayed return should go on the stack");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("the delayed return should resolve");

    let returned = game
        .find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .expect("the returned card should keep its stable identity");
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(game.controller_of(returned), alice);

    assert_eq!(
        breathkeeper_death_trigger_count(false),
        0,
        "an unpaired creature must not receive Breathkeeper Seraph's dies trigger"
    );
    assert_eq!(
        breathkeeper_death_trigger_count(true),
        1,
        "the same creature should receive the trigger while paired"
    );
}

#[test]
fn second_guess_only_allows_and_counters_the_globally_second_spell() {
    let definition = parse_oracle_card_definition("Second Guess");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Second Guess should have a spell program");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let first = recorded_spell(&mut game, alice, "First Spell", CardType::Instant, true);
    let second = recorded_spell(&mut game, bob, "Second Spell", CardType::Sorcery, true);
    let second_stable_id = game
        .object(second)
        .expect("the second spell should exist")
        .stable_id;
    let third = recorded_spell(&mut game, alice, "Third Spell", CardType::Instant, true);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);

    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        program,
        alice,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 1);
    assert_eq!(
        requirements[0].legal_targets,
        vec![crate::game_state::Target::Object(second)],
        "the first and third spells must be illegal targets even though all three are spells"
    );

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(second)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Second Guess should resolve against its legal target");
    let countered_second = game
        .find_object_by_stable_id(second_stable_id)
        .and_then(|id| game.object(id));
    assert_eq!(
        countered_second.map(|object| object.zone),
        Some(Zone::Graveyard),
        "the globally second spell should be countered"
    );
    assert!(
        game.object(first)
            .is_some_and(|object| object.zone == Zone::Stack)
    );
    assert!(
        game.object(third)
            .is_some_and(|object| object.zone == Zone::Stack)
    );

    let mut no_second_game = crate::tests::test_helpers::setup_two_player_game();
    let first = recorded_spell(
        &mut no_second_game,
        alice,
        "First Spell",
        CardType::Instant,
        true,
    );
    let _resolved_second = recorded_spell(
        &mut no_second_game,
        bob,
        "Resolved Second Spell",
        CardType::Sorcery,
        false,
    );
    let third = recorded_spell(
        &mut no_second_game,
        alice,
        "Third Spell",
        CardType::Instant,
        true,
    );
    let source = no_second_game.create_object_from_definition(&definition, alice, Zone::Stack);
    assert!(
        !crate::game_loop::spell_program_has_legal_targets_with_modes(
            &no_second_game,
            program,
            alice,
            Some(source),
            None,
        ),
        "Second Guess must be uncastable when only the first and third spells remain on the stack"
    );
    assert!(
        no_second_game
            .object(first)
            .is_some_and(|object| object.zone == Zone::Stack)
    );
    assert!(
        no_second_game
            .object(third)
            .is_some_and(|object| object.zone == Zone::Stack)
    );
}

fn resolve_obeka_choice(answer: bool) -> (Vec<PlayerId>, bool, Option<PlayerId>) {
    let definition = parse_oracle_card_definition("Obeka, Brute Chronologist");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Obeka should have an activated ability");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut decisions = RecordingBooleanDecisionMaker::answering(answer);
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Obeka's activated ability should resolve");
    (
        decisions.players,
        game.turn_store.end_turn_procedure_pending,
        game.turn.priority_player,
    )
}

#[test]
fn obeka_lets_the_active_player_accept_or_decline_ending_the_turn() {
    let bob = PlayerId::from_index(1);
    let alice = PlayerId::from_index(0);
    let (accept_players, accepted, accepted_priority) = resolve_obeka_choice(true);
    assert_eq!(accept_players, vec![bob], "the active player must decide");
    assert!(accepted, "accepting should request the end-turn procedure");
    assert_eq!(accepted_priority, None, "ending the turn clears priority");

    let (decline_players, declined, declined_priority) = resolve_obeka_choice(false);
    assert_eq!(
        decline_players,
        vec![bob],
        "the active player must still decide"
    );
    assert!(!declined, "declining must leave the turn running");
    assert_eq!(
        declined_priority,
        Some(alice),
        "declining must not disturb the existing priority state"
    );
}

fn spell_cast_event(
    game: &crate::GameState,
    spell: ObjectId,
    caster: PlayerId,
) -> crate::triggers::TriggerEvent {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell).expect("cast spell should exist"),
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

#[test]
fn bonus_round_copies_each_players_instant_or_sorcery_only_this_turn() {
    let definition = parse_oracle_card_definition("Bonus Round");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Bonus Round should have a spell program");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Bonus Round should install its temporary trigger");
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert_eq!(
        game.effect_store.delayed_triggers[0].expires_at_turn,
        Some(game.turn.turn_number)
    );

    let creature = game.create_object_from_definition(
        &spell_definition("Creature Spell", CardType::Creature),
        bob,
        Zone::Stack,
    );
    game.push_to_stack(crate::game_state::StackEntry::new(creature, bob));
    let creature_event = spell_cast_event(&game, creature, bob);
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &creature_event).is_empty(),
        "creature spells must not trigger Bonus Round"
    );

    let instant = game.create_object_from_definition(
        &spell_definition("Bob Instant", CardType::Instant),
        bob,
        Zone::Stack,
    );
    game.push_to_stack(crate::game_state::StackEntry::new(instant, bob));
    let instant_event = spell_cast_event(&game, instant, bob);
    let entries = crate::triggers::check_delayed_triggers(&mut game, &instant_event);
    assert_eq!(entries.len(), 1, "Bob's instant should trigger once");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Bonus Round's copy trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Bonus Round's copy trigger should resolve");

    let copies = game
        .objects_in_zone(Zone::Stack)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| {
            object.name == "Bob Instant"
                && object.kind == ObjectKind::SpellCopy
                && game.controller_of(object) == bob
        })
        .count();
    assert_eq!(
        copies, 1,
        "the spell's caster should control exactly one copy"
    );
    assert!(
        game.object(instant)
            .is_some_and(|object| object.zone == Zone::Stack),
        "copying must not consume the original spell"
    );

    game.turn.turn_number += 1;
    let later = game.create_object_from_definition(
        &spell_definition("Next Turn Instant", CardType::Instant),
        bob,
        Zone::Stack,
    );
    game.push_to_stack(crate::game_state::StackEntry::new(later, bob));
    let later_event = spell_cast_event(&game, later, bob);
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &later_event).is_empty(),
        "Bonus Round's trigger must expire after the turn in which it resolved"
    );
}
