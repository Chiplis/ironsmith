#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn creature(name: &str, subtypes: Vec<Subtype>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(subtypes)
        .power_toughness(PowerToughness::fixed(2, 10))
        .build()
}

fn became_blocked_event(
    game: &crate::GameState,
    attacker: ObjectId,
    blockers: &[ObjectId],
) -> crate::triggers::TriggerEvent {
    let attacker_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(attacker).expect("attacker exists"),
            game,
        );
    let blocker_snapshots = blockers
        .iter()
        .map(|blocker| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(*blocker).expect("blocker exists"),
                game,
            )
        })
        .collect();
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureBecameBlockedEvent::with_target_and_blockers(
            attacker,
            blockers.to_vec(),
            None,
            Some(attacker_snapshot),
            blocker_snapshots,
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn ib_trigger<'a>(
    definition: &'a CardDefinition,
    game: &crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Option<&'a crate::ability::TriggeredAbility> {
    let trigger_context = crate::triggers::TriggerContext::for_source(
        source,
        game.controller_of(game.object(source).expect("Ib exists")),
        game,
    );
    definition.abilities.iter().find_map(|ability| {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            return None;
        };
        triggered
            .trigger
            .matches(event, &trigger_context)
            .then_some(triggered)
    })
}

#[test]
fn ib_halfheart_sacrifices_the_blocked_goblin_and_it_damages_every_one_of_its_blockers() {
    let definition = parse_oracle_card_definition("Ib Halfheart, Goblin Tactician");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let ib = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let goblin = game.create_object_from_definition(
        &creature("Blocked Goblin", vec![Subtype::Goblin]),
        alice,
        Zone::Battlefield,
    );
    let first_blocker = game.create_object_from_definition(
        &creature("First Blocker", Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let second_blocker = game.create_object_from_definition(
        &creature("Second Blocker", Vec::new()),
        bob,
        Zone::Battlefield,
    );
    let decoy_attacker = game.create_object_from_definition(
        &creature("Decoy Attacker", Vec::new()),
        alice,
        Zone::Battlefield,
    );
    let unrelated_blocker = game.create_object_from_definition(
        &creature("Unrelated Blocker", Vec::new()),
        bob,
        Zone::Battlefield,
    );
    game.combat = Some(crate::combat_state::CombatState {
        blockers: std::collections::HashMap::from([
            (goblin, vec![first_blocker, second_blocker]),
            (decoy_attacker, vec![unrelated_blocker]),
        ]),
        ..Default::default()
    });
    let event = became_blocked_event(&game, goblin, &[first_blocker, second_blocker]);
    let triggered = ib_trigger(&definition, &game, ib, &event)
        .expect("another controlled Goblin becoming blocked must trigger Ib");
    let goblin_stable = game.object(goblin).expect("Goblin exists").stable_id;

    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(ib, alice, &mut decisions)
        .with_triggering_event(event);
    let emitted = crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        ib,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Ib's trigger should resolve");

    let sacrificed = game
        .find_object_by_stable_id(goblin_stable)
        .expect("the sacrificed Goblin should retain stable identity");
    assert_eq!(
        game.object(sacrificed).expect("Goblin exists").zone,
        Zone::Graveyard
    );
    assert_eq!(game.object(ib).expect("Ib remains").zone, Zone::Battlefield);
    assert_eq!(game.damage_on(first_blocker), 4);
    assert_eq!(game.damage_on(second_blocker), 4);
    assert_eq!(
        game.damage_on(unrelated_blocker),
        0,
        "a creature blocking another attacker is outside Ib's affected set"
    );

    let damage = emitted
        .iter()
        .filter_map(|event| event.downcast::<crate::events::DamageEvent>())
        .collect::<Vec<_>>();
    assert_eq!(
        damage.len(),
        2,
        "one damage event is required for each blocker"
    );
    assert!(
        damage.iter().all(|event| {
            game.object(event.source)
                .is_some_and(|source| source.stable_id == goblin_stable)
        }),
        "the sacrificed Goblin, not Ib, must be the source of both damage events: {damage:#?}"
    );

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Whenever another Goblin you control becomes blocked, sacrifice it. If you do, that creature deals 4 damage to each creature blocking this creature.\nSacrifice two Mountains: Create two 1/1 red Goblin creature tokens."
    );
}

#[test]
fn ib_halfheart_requires_another_goblin_and_successful_sacrifice_before_damage() {
    let definition = parse_oracle_card_definition("Ib Halfheart, Goblin Tactician");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let ib = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let nongoblin = game.create_object_from_definition(
        &creature("Blocked Nongoblin", Vec::new()),
        alice,
        Zone::Battlefield,
    );
    let goblin = game.create_object_from_definition(
        &creature("Departing Goblin", vec![Subtype::Goblin]),
        alice,
        Zone::Battlefield,
    );
    let blocker = game.create_object_from_definition(
        &creature("Waiting Blocker", Vec::new()),
        bob,
        Zone::Battlefield,
    );
    game.combat = Some(crate::combat_state::CombatState {
        blockers: std::collections::HashMap::from([(goblin, vec![blocker])]),
        ..Default::default()
    });

    let self_event = became_blocked_event(&game, ib, &[blocker]);
    assert!(
        ib_trigger(&definition, &game, ib, &self_event).is_none(),
        "Ib's 'another' qualifier must reject Ib itself"
    );
    let nongoblin_event = became_blocked_event(&game, nongoblin, &[blocker]);
    assert!(
        ib_trigger(&definition, &game, ib, &nongoblin_event).is_none(),
        "the trigger must reject a blocked non-Goblin"
    );

    let event = became_blocked_event(&game, goblin, &[blocker]);
    let triggered = ib_trigger(&definition, &game, ib, &event)
        .expect("the other controlled Goblin should trigger Ib");
    game.move_object_by_effect(goblin, Zone::Exile)
        .expect("the Goblin should leave before the trigger resolves");
    let mut context =
        crate::effects::ExecutionContext::new_default(ib, alice).with_triggering_event(event);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        ib,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Ib's trigger should resolve when the sacrifice is impossible");
    assert_eq!(
        game.damage_on(blocker),
        0,
        "the if-you-do branch must stay off when the Goblin cannot be sacrificed"
    );
}

fn card_for_binding(name: &str, card_type: CardType) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

#[test]
fn binding_of_the_titans_gains_life_only_for_creature_cards_among_mixed_exiled_targets() {
    let definition = parse_oracle_card_definition("The Binding of the Titans");
    let chapter_two = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::SagaChapterTrigger>()
                    .is_some_and(|chapter| chapter.chapters == [2]) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Binding should retain chapter II");
    let [target_spec] = chapter_two.choices.as_slice() else {
        panic!("chapter II should have one up-to-two target requirement: {chapter_two:#?}");
    };

    for (types, expected_life_gain) in [
        ([CardType::Creature, CardType::Instant], 1),
        ([CardType::Instant, CardType::Land], 0),
        ([CardType::Creature, CardType::Creature], 2),
    ] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let first = game.create_object_from_definition(
            &card_for_binding("First Binding Target", types[0]),
            alice,
            Zone::Graveyard,
        );
        let second = game.create_object_from_definition(
            &card_for_binding("Second Binding Target", types[1]),
            alice,
            Zone::Graveyard,
        );
        let unchosen_creature = game.create_object_from_definition(
            &card_for_binding("Unchosen Creature Card", CardType::Creature),
            alice,
            Zone::Graveyard,
        );
        let first_stable = game.object(first).expect("first target exists").stable_id;
        let second_stable = game.object(second).expect("second target exists").stable_id;
        let assignment = crate::game_state::TargetAssignment {
            spec: target_spec.clone(),
            range: 0..2,
        };
        let mut context = crate::effects::ExecutionContext::new_default(source, alice)
            .with_targets(vec![
                crate::effects::ResolvedTarget::Object(first),
                crate::effects::ResolvedTarget::Object(second),
            ])
            .with_target_assignments(vec![assignment.clone()]);
        context.snapshot_targets(&game);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut context,
            alice,
            source,
            &chapter_two.effects,
            None,
            &[assignment],
        )
        .expect("Binding chapter II should resolve");

        for stable in [first_stable, second_stable] {
            let current = game
                .find_object_by_stable_id(stable)
                .expect("an exiled target should retain stable identity");
            assert_eq!(
                game.object(current).expect("target exists").zone,
                Zone::Exile
            );
        }
        assert_eq!(
            game.object(unchosen_creature)
                .expect("unchosen creature exists")
                .zone,
            Zone::Graveyard,
            "an unchosen creature card is not part of the exiled-this-way set"
        );
        assert_eq!(
            game.player(alice).expect("Alice exists").life,
            20 + expected_life_gain,
            "life must count only creature cards in the exact exiled result set for {types:?}"
        );
    }

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "I — Each player mills three cards.\nII — Exile up to two target cards from graveyards. For each creature card exiled this way, you gain 1 life.\nIII — Return target creature or land card from your graveyard to your hand."
    );
}

fn transcendent_dragon_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Transcendent Dragon should retain its enters trigger")
}

fn transcendent_dragon_stack_setup() -> (
    crate::GameState,
    CardDefinition,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
    StableId,
) {
    let definition = parse_oracle_card_definition("Transcendent Dragon");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let dragon = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Captured Spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    let target = game.create_object_from_definition(&target_definition, bob, Zone::Stack);
    let target_stable = game.object(target).expect("target spell exists").stable_id;
    game.push_to_stack(crate::game_state::StackEntry::new(target, bob));
    (game, definition, alice, bob, dragon, target, target_stable)
}

#[test]
fn transcendent_dragon_cast_entry_counters_exiles_and_free_casts_the_target_spell() {
    let (mut game, definition, alice, _bob, dragon, target, target_stable) =
        transcendent_dragon_stack_setup();
    let triggered = transcendent_dragon_trigger(&definition);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::EnterBattlefieldEvent::new(dragon, Zone::Stack),
        crate::provenance::ProvNodeId::default(),
    );
    game.push_to_stack(
        crate::game_state::StackEntry::ability(dragon, alice, triggered.effects.clone())
            .with_targets(vec![crate::game_state::Target::Object(target)])
            .with_triggering_event(event)
            .with_intervening_if(
                triggered
                    .intervening_if
                    .clone()
                    .expect("the trigger should retain its if-you-cast-it gate"),
            ),
    );

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Transcendent Dragon's cast-gated trigger should resolve");

    let recast = game
        .find_object_by_stable_id(target_stable)
        .expect("the captured spell should retain stable identity");
    assert_eq!(
        game.object(recast).expect("the recast spell exists").zone,
        Zone::Stack,
        "the countered spell should be exiled and then cast immediately"
    );
    let recast_entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == recast)
        .expect("the free-cast spell should have a stack entry");
    assert_eq!(recast_entry.controller, alice);
    assert_ne!(
        recast, target,
        "zone changes should produce a fresh object ID"
    );
}

#[test]
fn transcendent_dragon_noncast_entry_does_not_counter_the_target_spell() {
    let (mut game, definition, alice, bob, dragon, target, target_stable) =
        transcendent_dragon_stack_setup();
    let triggered = transcendent_dragon_trigger(&definition);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::EnterBattlefieldEvent::new(dragon, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.push_to_stack(
        crate::game_state::StackEntry::ability(dragon, alice, triggered.effects.clone())
            .with_targets(vec![crate::game_state::Target::Object(target)])
            .with_triggering_event(event)
            .with_intervening_if(
                triggered
                    .intervening_if
                    .clone()
                    .expect("the trigger should retain its if-you-cast-it gate"),
            ),
    );

    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("the false intervening-if branch should resolve without effects");

    assert_eq!(
        game.find_object_by_stable_id(target_stable),
        Some(target),
        "the untouched target should keep its object identity"
    );
    assert_eq!(
        game.object(target).expect("target remains").zone,
        Zone::Stack
    );
    assert!(
        game.stack
            .iter()
            .any(|entry| { entry.object_id == target && entry.controller == bob })
    );
}
