#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn durable_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 10))
        .build()
}

fn artifact(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

fn damage_target_spec(definition: &CardDefinition) -> ChooseSpec {
    fn find(effect: &crate::effect::Effect) -> Option<ChooseSpec> {
        if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
            return Some(damage.target.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find(child);
            }
        });
        found
    }

    definition
        .spell_effect
        .as_ref()
        .expect("damage spell should have a resolution program")
        .flattened_default_effects()
        .iter()
        .find_map(|effect| find(effect))
        .expect("damage spell should have a target specification")
}

fn record_block(game: &mut crate::game_state::GameState, blocker: ObjectId, attacker: ObjectId) {
    let blocker_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(blocker).expect("blocker exists"),
            game,
        );
    let attacker_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(attacker).expect("attacker exists"),
            game,
        );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureBlockedEvent::with_snapshots(
            blocker,
            attacker,
            blocker_snapshot.clone(),
            attacker_snapshot,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&event, Some(blocker_snapshot), None);
}

#[test]
fn due_respect_registers_a_turn_scoped_entry_replacement_and_expires_at_cleanup() {
    let definition = parse_oracle_card_definition("Due Respect");
    assert_eq!(
        canonical_compiled_lines(&definition),
        ["Permanents enter tapped this turn.\nDraw a card."]
    );
    assert!(
        definition.abilities.is_empty(),
        "an instant must not carry the entry rule as a battlefield static ability: {:#?}",
        definition.abilities
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell,
        definition
            .spell_effect
            .as_ref()
            .expect("Due Respect should have a resolution program"),
        None,
        &[],
    )
    .expect("Due Respect should resolve");
    assert_eq!(
        game.effect_store.replacement_effects.effects().len(),
        1,
        "Due Respect should register exactly one turn-scoped entry replacement: {:#?}",
        definition.spell_effect
    );

    let during_turn =
        game.create_object_from_definition(&artifact("During-Turn Permanent"), alice, Zone::Hand);
    let during_turn = game
        .move_object_with_etb_processing(during_turn, Zone::Battlefield)
        .expect("permanent should enter during the protected turn");
    assert!(
        during_turn.enters_tapped && game.is_tapped(during_turn.new_id),
        "every permanent entering after Due Respect resolves must enter tapped"
    );

    crate::turn::execute_cleanup_step(&mut game);
    let after_cleanup =
        game.create_object_from_definition(&artifact("Post-Cleanup Permanent"), alice, Zone::Hand);
    let after_cleanup = game
        .move_object_with_etb_processing(after_cleanup, Zone::Battlefield)
        .expect("permanent should enter after cleanup");
    assert!(
        !after_cleanup.enters_tapped && !game.is_tapped(after_cleanup.new_id),
        "the entry replacement must expire at cleanup rather than persist indefinitely"
    );
}

#[test]
fn sizzling_barrage_targets_only_a_creature_that_actually_blocked_this_turn() {
    let definition = parse_oracle_card_definition("Sizzling Barrage");
    assert_eq!(
        canonical_compiled_lines(&definition),
        ["Sizzling Barrage deals 4 damage to target creature that blocked this turn."]
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let blocker = game.create_object_from_definition(
        &durable_creature("Historical Blocker"),
        bob,
        Zone::Battlefield,
    );
    let blocked_attacker = game.create_object_from_definition(
        &durable_creature("Blocked Attacker"),
        alice,
        Zone::Battlefield,
    );
    let uninvolved = game.create_object_from_definition(
        &durable_creature("Uninvolved Creature"),
        bob,
        Zone::Battlefield,
    );
    record_block(&mut game, blocker, blocked_attacker);

    let target_spec = damage_target_spec(&definition);
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let legal = crate::game_loop::compute_legal_targets(&game, &target_spec, alice, Some(source));
    assert_eq!(
        legal,
        vec![crate::game_state::Target::Object(blocker)],
        "the historical blocker is legal after combat, while the blocked attacker and an uninvolved creature are not"
    );

    let barrage = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(barrage, alice)
            .with_targets(vec![crate::game_state::Target::Object(blocker)]),
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("historical blocker should be legal");
    assert_eq!(game.damage_on(blocker), 4);

    let illegal_barrage = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(illegal_barrage, alice)
            .with_targets(vec![crate::game_state::Target::Object(blocked_attacker)]),
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("an illegal historical target should make the spell fizzle cleanly");
    assert_eq!(
        game.damage_on(blocked_attacker),
        0,
        "a creature that was blocked did not itself block and must remain an illegal target"
    );
    assert_eq!(game.damage_on(uninvolved), 0);
}

#[test]
fn glyph_of_reincarnation_keeps_historical_controller_provenance_and_exact_order() {
    let definition = parse_oracle_card_definition("Glyph of Reincarnation");
    let program_debug = format!("{:#?}", definition.spell_effect);
    assert_eq!(
        unprocessed_compiled_lines(&definition).join("\n"),
        "Cast this spell only after combat.\nDestroy all creatures that were blocked by target Wall this turn. They can't be regenerated. For each creature that died this way, put a creature card from the graveyard of the player who controlled that creature the last time it became blocked by that Wall onto the battlefield under its owner's control.",
        "the exact historical-block renderer must consume the compiler-model wrapper shape: {program_debug}"
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Glyph should have a spell resolution program");
    let effects = program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .collect::<Vec<_>>();
    let historical_loop = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ForEachTaggedEffect>());
    let historical_loop = historical_loop
        .expect("Glyph should iterate only the creatures successfully destroyed this way");
    let blocker_tag = historical_loop
        .controller_at_last_blocked_by
        .as_ref()
        .expect("the historical-controller loop should name its linked blocker tag");
    assert!(
        effects.iter().any(|effect| {
            let aggregate_target = effect
                .downcast_ref::<crate::effects::TagAllEffect>()
                .is_some_and(|tagged| {
                    tagged.tag == *blocker_tag
                        && tagged
                            .effect
                            .downcast_ref::<crate::effects::TargetOnlyEffect>()
                            .is_some()
                });
            let compiler_model_target = effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .is_some_and(|tagged| {
                    tagged.tag == *blocker_tag
                        && tagged
                            .effect
                            .downcast_ref::<crate::effects::TargetOnlyEffect>()
                            .is_some()
                });
            aggregate_target || compiler_model_target
        }),
        "the historical-controller lookup must use the same stable blocker target captured by the spell: {program:#?}"
    );
}
