#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn crag_saurian_binds_control_to_the_damage_source_controller() {
    let definition = parse_oracle_card_definition("Crag Saurian");
    let debug = format!("{definition:#?}");
    let lines = unprocessed_compiled_lines(&definition);

    assert!(
        debug.contains("TagTriggeringSourceEffect"),
        "the damage source must be snapshotted for its controller relation: {debug}"
    );
    assert!(
        debug.contains("ControllerOf") && debug.contains("triggering_source"),
        "the controller change must reference the triggering damage source: {debug}"
    );
    assert!(
        lines.iter().any(|line| {
            line
                == "Whenever a source deals damage to this creature, that source's controller gains control of this creature."
        }),
        "Crag Saurian should retain the exact relational control surface; got {lines:#?}"
    );
}

#[test]
fn belltower_sphinx_binds_mill_to_the_damage_source_controller() {
    let definition = parse_oracle_card_definition("Belltower Sphinx");
    let debug = format!("{definition:#?}");
    let lines = unprocessed_compiled_lines(&definition);

    assert!(
        debug.contains("TagTriggeringSourceEffect"),
        "the damage source must be snapshotted for the mill player relation: {debug}"
    );
    assert!(
        debug.contains("MillEffect")
            && debug.contains("ControllerOf")
            && debug.contains("triggering_source"),
        "mill must reference the triggering damage source's controller: {debug}"
    );
    assert!(
        lines.iter().any(|line| {
            line
                == "Whenever a source deals damage to this creature, that source's controller mills that many cards."
        }),
        "Belltower Sphinx should retain the exact relational mill surface; got {lines:#?}"
    );
}

#[test]
fn belltower_sphinx_mills_the_opponent_controlling_the_damage_source() {
    let definition = parse_oracle_card_definition("Belltower Sphinx");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let sphinx = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let damage_source_definition = CardDefinitionBuilder::new(CardId::new(), "Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let damage_source =
        game.create_object_from_definition(&damage_source_definition, bob, Zone::Battlefield);
    let library_card = CardDefinitionBuilder::new(CardId::new(), "Library Card")
        .card_types(vec![CardType::Sorcery])
        .build();
    for _ in 0..5 {
        game.create_object_from_definition(&library_card, alice, Zone::Library);
        game.create_object_from_definition(&library_card, bob, Zone::Library);
    }

    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            damage_source,
            crate::events::DamageTarget::Object(sphinx),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &damage_event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Belltower Sphinx should trigger exactly once for the damage event"
    );
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Belltower Sphinx's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Belltower Sphinx's trigger should resolve");

    assert_eq!(
        game.player(bob).expect("Bob should exist").graveyard.len(),
        3,
        "the opponent controlling the damage source must mill the event amount"
    );
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .graveyard
            .len(),
        0,
        "the triggered ability's controller must not mill instead"
    );
}

#[test]
fn lava_runner_makes_the_targeting_spell_or_abilitys_controller_sacrifice_a_land() {
    let definition = parse_oracle_card_definition("Lava Runner");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("TagTriggeringSourceEffect"),
        "the targeting stack object must be snapshotted separately from Lava Runner: {debug}"
    );
    assert!(
        debug.contains("ControllerOf") && debug.contains("triggering_source"),
        "the sacrifice chooser and affected player must follow the targeting source's controller: {debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let runner = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let land_definition = CardDefinitionBuilder::new(CardId::new(), "Lava Runner Probe Land")
        .card_types(vec![CardType::Land])
        .build();
    let alice_land = game.create_object_from_definition(&land_definition, alice, Zone::Battlefield);
    let bob_land = game.create_object_from_definition(&land_definition, bob, Zone::Battlefield);
    let alice_land_stable = game
        .object(alice_land)
        .expect("Alice's land exists")
        .stable_id;
    let bob_land_stable = game.object(bob_land).expect("Bob's land exists").stable_id;

    let targeting_spell_definition =
        CardDefinitionBuilder::new(CardId::new(), "Bob's Targeting Spell")
            .card_types(vec![CardType::Instant])
            .build();
    let targeting_spell =
        game.create_object_from_definition(&targeting_spell_definition, bob, Zone::Stack);
    let targeted_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::BecomesTargetedEvent::new(runner, targeting_spell, bob, false),
        crate::provenance::ProvNodeId::default(),
    );
    let matching = crate::triggers::check_triggers(&game, &targeted_event)
        .into_iter()
        .filter(|entry| entry.source == runner)
        .collect::<Vec<_>>();
    assert_eq!(matching.len(), 1, "Lava Runner should trigger exactly once");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in matching {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Lava Runner's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Lava Runner's trigger should resolve");

    let bob_land_after = game
        .find_object_by_stable_id(bob_land_stable)
        .and_then(|id| game.object(id))
        .expect("Bob's land should remain identifiable");
    let alice_land_after = game
        .find_object_by_stable_id(alice_land_stable)
        .and_then(|id| game.object(id))
        .expect("Alice's land should remain identifiable");
    assert_eq!(
        bob_land_after.zone,
        Zone::Graveyard,
        "the controller of the targeting spell must sacrifice"
    );
    assert_eq!(
        alice_land_after.zone,
        Zone::Battlefield,
        "Lava Runner's controller must not sacrifice instead"
    );
}

#[test]
fn lava_runner_uses_the_controller_of_a_targeting_ability_too() {
    let definition = parse_oracle_card_definition("Lava Runner");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let runner = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let land_definition = CardDefinitionBuilder::new(CardId::new(), "Ability Probe Land")
        .card_types(vec![CardType::Land])
        .build();
    let alice_land = game.create_object_from_definition(&land_definition, alice, Zone::Battlefield);
    let bob_land = game.create_object_from_definition(&land_definition, bob, Zone::Battlefield);
    let alice_land_stable = game
        .object(alice_land)
        .expect("Alice's land exists")
        .stable_id;
    let bob_land_stable = game.object(bob_land).expect("Bob's land exists").stable_id;
    let ability_source_definition =
        CardDefinitionBuilder::new(CardId::new(), "Bob's Targeting Ability Source")
            .card_types(vec![CardType::Artifact])
            .build();
    let ability_source =
        game.create_object_from_definition(&ability_source_definition, bob, Zone::Battlefield);
    let targeted_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::BecomesTargetedEvent::new(runner, ability_source, bob, true),
        crate::provenance::ProvNodeId::default(),
    );
    let matching = crate::triggers::check_triggers(&game, &targeted_event)
        .into_iter()
        .filter(|entry| entry.source == runner)
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        1,
        "Lava Runner should trigger for an ability"
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in matching {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Lava Runner's ability-target trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Lava Runner's ability-target trigger should resolve");

    let bob_land_after = game
        .find_object_by_stable_id(bob_land_stable)
        .and_then(|id| game.object(id))
        .expect("Bob's land should remain identifiable");
    let alice_land_after = game
        .find_object_by_stable_id(alice_land_stable)
        .and_then(|id| game.object(id))
        .expect("Alice's land should remain identifiable");
    assert_eq!(bob_land_after.zone, Zone::Graveyard);
    assert_eq!(alice_land_after.zone, Zone::Battlefield);
}

#[test]
fn reaper_of_sheoldred_binds_poison_to_the_damage_source_controller() {
    let definition = parse_oracle_card_definition("Reaper of Sheoldred");
    let debug = format!("{definition:#?}");
    let lines = unprocessed_compiled_lines(&definition);

    assert!(
        debug.contains("TagTriggeringSourceEffect")
            && debug.contains("PoisonCountersEffect")
            && debug.contains("ControllerOf")
            && debug.contains("triggering_source"),
        "the poison counter must be assigned to the triggering damage source's controller: {debug}"
    );
    assert!(
        lines.iter().any(|line| {
            line
                == "Whenever a source deals damage to this creature, that source's controller gets a poison counter."
        }),
        "Reaper of Sheoldred should retain the exact relational poison surface; got {lines:#?}"
    );
}

#[test]
fn rona_tolarian_obliterator_binds_the_random_hand_to_the_damage_source_controller() {
    let definition = parse_oracle_card_definition("Rona, Tolarian Obliterator");
    let debug = format!("{definition:#?}");
    let lines = unprocessed_compiled_lines(&definition);

    assert!(
        debug.contains("TagTriggeringSourceEffect")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("ControllerOf")
            && debug.contains("triggering_source"),
        "Rona's random hand card must come from the triggering damage source's controller: {debug}"
    );
    assert!(
        !debug.contains("IteratedPlayer"),
        "Rona's trigger must not leave an unbound iterated-player hand reference: {debug}"
    );
    assert_eq!(
        debug.matches("CastTaggedEffect").count(),
        1,
        "the nonland arm should contain one optional cast, not a second cast when the land offer is declined: {debug}"
    );
    assert!(
        debug.matches("MayEffect").count() >= 2,
        "both Rona follow-up actions are optional: {debug}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("Otherwise, you may cast")),
        "the nonland branch must remain the conditional's otherwise arm: {lines:#?}"
    );
}

#[derive(Default)]
struct AcceptMayRecordingDecisionMaker {
    players: Vec<PlayerId>,
}

impl crate::decision::DecisionMaker for AcceptMayRecordingDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.players.push(ctx.player);
        true
    }
}

fn put_rona_damage_trigger_on_stack(
    game: &mut crate::game_state::GameState,
    rona: ObjectId,
    damage_source: ObjectId,
) {
    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            damage_source,
            crate::events::DamageTarget::Object(rona),
            1,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(game, &damage_event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Rona should trigger exactly once for the damage event"
    );
    crate::game_loop::put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Rona's trigger should go on the stack");
}

fn setup_rona_damage_game() -> (
    crate::game_state::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
) {
    let definition = parse_oracle_card_definition("Rona, Tolarian Obliterator");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let rona = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let damage_source_definition = CardDefinitionBuilder::new(CardId::new(), "Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let damage_source =
        game.create_object_from_definition(&damage_source_definition, bob, Zone::Battlefield);
    (game, alice, bob, rona, damage_source)
}

#[test]
fn rona_puts_the_damage_source_controllers_random_land_under_her_controllers_control() {
    let (mut game, alice, bob, rona, damage_source) = setup_rona_damage_game();
    let land_definition = CardDefinitionBuilder::new(CardId::new(), "Bob's Land")
        .card_types(vec![CardType::Land])
        .build();
    let land = game.create_object_from_definition(&land_definition, bob, Zone::Hand);
    let land_stable = game
        .object(land)
        .expect("Bob's land should exist")
        .stable_id;
    let alice_card_definition = CardDefinitionBuilder::new(CardId::new(), "Alice's Card")
        .card_types(vec![CardType::Sorcery])
        .build();
    let alice_card = game.create_object_from_definition(&alice_card_definition, alice, Zone::Hand);

    put_rona_damage_trigger_on_stack(&mut game, rona, damage_source);
    let mut decisions = AcceptMayRecordingDecisionMaker::default();
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Rona's land branch should resolve");

    assert!(
        game.player(bob).expect("Bob should exist").hand.is_empty(),
        "the source controller's only hand card should be exiled"
    );
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .hand
            .contains(&alice_card),
        "Rona must not choose from her controller's hand"
    );
    let moved_land = game
        .find_object_by_stable_id(land_stable)
        .and_then(|id| game.object(id))
        .expect("the exiled land should still exist");
    assert_eq!(moved_land.zone, Zone::Battlefield);
    assert_eq!(moved_land.owner, bob);
    assert_eq!(game.controller_of(moved_land), alice);
    assert_eq!(
        decisions.players,
        vec![alice],
        "Rona's controller decides whether to put the land onto the battlefield"
    );
}

#[test]
fn rona_lets_her_controller_cast_the_damage_source_controllers_random_nonland() {
    let (mut game, alice, bob, rona, damage_source) = setup_rona_damage_game();
    let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Bob's Spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell = game.create_object_from_definition(&spell_definition, bob, Zone::Hand);
    let spell_stable = game
        .object(spell)
        .expect("Bob's spell should exist")
        .stable_id;
    let alice_card_definition = CardDefinitionBuilder::new(CardId::new(), "Alice's Card")
        .card_types(vec![CardType::Sorcery])
        .build();
    let alice_card = game.create_object_from_definition(&alice_card_definition, alice, Zone::Hand);

    put_rona_damage_trigger_on_stack(&mut game, rona, damage_source);
    let mut decisions = AcceptMayRecordingDecisionMaker::default();
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Rona's nonland branch should resolve");

    assert!(
        game.player(bob).expect("Bob should exist").hand.is_empty(),
        "the source controller's only hand card should be exiled"
    );
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .hand
            .contains(&alice_card),
        "Rona must not choose from her controller's hand"
    );
    let cast_spell = game
        .find_object_by_stable_id(spell_stable)
        .expect("the exiled nonland should be cast");
    let cast_object = game.object(cast_spell).expect("cast spell should exist");
    assert_eq!(cast_object.zone, Zone::Stack);
    assert_eq!(cast_object.owner, bob);
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == cast_spell && entry.controller == alice),
        "Rona's controller should cast the source controller's exiled spell"
    );
    assert_eq!(
        decisions.players,
        vec![alice],
        "Rona's controller decides whether to cast the nonland"
    );
}
