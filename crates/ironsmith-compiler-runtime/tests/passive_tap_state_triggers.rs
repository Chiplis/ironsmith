//! Passive filtered and grouped tap-state events through actual effect, cost,
//! combat and untap producers. Source-only campaign: all scenarios are unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, SelectFirstDecisionMaker};
use ironsmith::effects::{
    CreateTokenEffect, CrewCostEffect, EffectContext, EffectExecutor, TapEffect, UntapEffect,
};
use ironsmith::game_loop::{
    apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ObjectFilter, PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/passive_tap_state_triggers.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let (artifact, direct) = compile_to_artifact(name, row["text"].as_str().unwrap(), false)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn resource(game: &mut GameState, owner: PlayerId, zone: Zone, kind: &str) -> ObjectId {
    let text = if kind.contains("Creature") {
        format!("Type: {kind}\nPower/Toughness: 1/1")
    } else {
        format!("Type: {kind}")
    };
    let definition = compile_to_runtime_definition("Tap-state resource", &text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn stack(game: &mut GameState, queue: &mut TriggerQueue) -> usize {
    put_triggers_on_stack_with_dm(game, queue, &mut SelectFirstDecisionMaker).unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState) -> usize {
    let mut queue = TriggerQueue::new();
    let count = stack(game, &mut queue);
    for _ in 0..20 {
        if game.stack_is_empty() {
            return count;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
        stack(game, &mut queue);
    }
    panic!("tap-state triggers did not settle");
}
fn execute(game: &mut GameState, source: ObjectId, player: PlayerId, effect: &dyn EffectExecutor) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, player, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(Default::default(), event);
    }
}
fn tokens(game: &GameState, player: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|id| {
            game.object(**id).is_some_and(|object| {
                object.kind == ironsmith::object::ObjectKind::Token
                    && game.controller_of(object) == player
            })
        })
        .count()
}

#[test]
fn all_five_exact_passive_tap_cards_round_trip_with_typed_state_events() {
    assert_eq!(fixtures().len(), 5);
    for fixture in fixtures() {
        for definition in definitions(fixture["name"].as_str().unwrap()) {
            assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Triggered(triggered)
                if triggered.trigger.compiled_model().is_some_and(|model| matches!(model.kind, TriggerKind::BecomesTapped | TriggerKind::BecomesUntapped | TriggerKind::PermanentBecomesTapped { .. } | TriggerKind::PermanentBecomesUntapped { .. })))));
        }
    }
}

#[test]
fn pilgrimage_counts_simultaneous_nontoken_merfolk_once_and_keeps_separate_instructions() {
    for definition in definitions("Deeproot Pilgrimage") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let one = resource(&mut game, A, Zone::Battlefield, "Creature — Merfolk");
        let two = resource(&mut game, A, Zone::Battlefield, "Creature — Merfolk");
        resource(&mut game, B, Zone::Battlefield, "Creature — Merfolk");
        resource(&mut game, A, Zone::Battlefield, "Creature — Goblin");
        let token = compile_to_runtime_definition(
            "Merfolk token",
            "Type: Creature — Merfolk\nPower/Toughness: 1/1",
            false,
        )
        .unwrap();
        execute(
            &mut game,
            source,
            A,
            &CreateTokenEffect::new(token, 1, PlayerFilter::You),
        );
        settle(&mut game);
        execute(
            &mut game,
            source,
            A,
            &TapEffect::all(ObjectFilter::creature()),
        );
        assert_eq!(settle(&mut game), 1);
        assert_eq!(tokens(&game, A), 2);
        // The freshly created Merfolk token doesn't satisfy nontoken.
        execute(
            &mut game,
            source,
            A,
            &TapEffect::all(ObjectFilter::creature()),
        );
        assert_eq!(settle(&mut game), 0);
        // These are two separate tap instructions, although their notifications
        // aren't delivered until both instructions have completed.
        game.untap(one);
        game.untap(two);
        execute(
            &mut game,
            source,
            A,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(one)),
        );
        execute(
            &mut game,
            source,
            A,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(two)),
        );
        assert_eq!(settle(&mut game), 2);
        assert_eq!(tokens(&game, A), 4);
    }
}

#[test]
fn pilgrimage_groups_actual_attack_declaration_and_chosen_crew_cost() {
    for definition in definitions("Deeproot Pilgrimage") {
        for as_cost in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let creatures = [
                resource(&mut game, A, Zone::Battlefield, "Creature — Merfolk"),
                resource(&mut game, A, Zone::Battlefield, "Creature — Merfolk"),
            ];
            for creature in creatures {
                game.remove_summoning_sickness(creature);
            }
            if as_cost {
                let vehicle = resource(&mut game, A, Zone::Battlefield, "Artifact — Vehicle");
                execute(&mut game, vehicle, A, &CrewCostEffect::new(2));
                assert_eq!(settle(&mut game), 1);
            } else {
                game.turn.active_player = A;
                game.turn.phase = Phase::Combat;
                game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
                game.mark_combat_phase_started();
                let mut combat = CombatState::default();
                let declarations = creatures.map(|creature| AttackerDeclaration {
                    creature,
                    target: AttackTarget::Player(B),
                });
                let mut queue = TriggerQueue::new();
                apply_attacker_declarations(&mut game, &mut combat, &mut queue, &declarations)
                    .unwrap();
                assert_eq!(queue.entries.len(), 1);
                stack(&mut game, &mut queue);
                settle(&mut game);
            }
            assert_eq!(tokens(&game, A), 1);
        }
    }
}

#[test]
fn rewrite_history_loots_and_counts_one_plan_per_group_through_the_fourth_trigger() {
    for definition in definitions("Rewrite History") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..8 {
            resource(&mut game, A, Zone::Library, "Sorcery");
        }
        resource(&mut game, A, Zone::Hand, "Sorcery");
        for count in 1..=4 {
            resource(&mut game, A, Zone::Battlefield, "Creature — Merfolk");
            resource(&mut game, A, Zone::Battlefield, "Creature — Merfolk");
            execute(
                &mut game,
                source,
                A,
                &TapEffect::all(ObjectFilter::creature()),
            );
            assert_eq!(settle(&mut game), 1);
            if count < 4 {
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
                assert_eq!(
                    game.object(source)
                        .unwrap()
                        .counters
                        .get(&ironsmith::object::CounterType::Named("plan".into())),
                    Some(&count)
                );
            }
        }
        assert!(
            game.object(source).is_none(),
            "the fourth plan trigger sacrificed its source"
        );
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            3,
            "the reflexive trigger returned two discarded sorceries"
        );
    }
}

#[test]
fn filtered_untap_observes_actual_changes_and_controller_not_owner() {
    let orb = definitions("Mesmeric Orb");
    let wake = definitions("Wake Thrasher");
    for path in 0..2 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&orb[path], B, Zone::Battlefield);
        let wake = game.create_object_from_definition(&wake[path], A, Zone::Battlefield);
        for player in [A, B] {
            for _ in 0..5 {
                resource(&mut game, player, Zone::Library, "Sorcery");
            }
        }
        let mine = resource(&mut game, A, Zone::Battlefield, "Artifact");
        let stolen = resource(&mut game, A, Zone::Battlefield, "Artifact");
        game.set_current_controller(stolen, B).unwrap();
        game.tap(mine);
        game.tap(stolen);
        execute(
            &mut game,
            source,
            B,
            &UntapEffect::with_spec(ChooseSpec::All(ObjectFilter::artifact())),
        );
        assert_eq!(settle(&mut game), 3);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 1);
        assert_eq!(game.current_power(wake), Some(2));
        execute(
            &mut game,
            source,
            B,
            &UntapEffect::with_spec(ChooseSpec::All(ObjectFilter::artifact())),
        );
        assert_eq!(
            settle(&mut game),
            0,
            "already untapped permanents don't emit untap events"
        );
        game.tap(mine);
        game.add_counters(mine, ironsmith::object::CounterType::Stun, 1)
            .unwrap();
        execute(
            &mut game,
            source,
            B,
            &UntapEffect::with_spec(ChooseSpec::SpecificObject(mine)),
        );
        assert_eq!(
            settle(&mut game),
            0,
            "a replaced untap isn't a completed untap"
        );
        assert!(game.is_tapped(mine));
        game.phase_out(mine);
        execute(
            &mut game,
            source,
            B,
            &UntapEffect::with_spec(ChooseSpec::All(ObjectFilter::artifact())),
        );
        assert_eq!(settle(&mut game), 0);
        assert!(game.is_tapped(mine));
    }
}

#[test]
fn plural_named_source_taps_draw_and_untaps_put_counters_only_on_itself() {
    for definition in definitions("Tui and La, Moon and Ocean") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        resource(&mut game, B, Zone::Library, "Sorcery");
        let other = resource(&mut game, B, Zone::Battlefield, "Creature — Spirit");
        execute(
            &mut game,
            source,
            B,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(other)),
        );
        assert_eq!(settle(&mut game), 0);
        execute(
            &mut game,
            source,
            B,
            &TapEffect::with_spec(ChooseSpec::SpecificObject(source)),
        );
        assert_eq!(settle(&mut game), 1);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        execute(
            &mut game,
            source,
            B,
            &UntapEffect::with_spec(ChooseSpec::SpecificObject(source)),
        );
        assert_eq!(settle(&mut game), 1);
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&ironsmith::object::CounterType::PlusOnePlusOne),
            Some(&1)
        );
        assert!(game.object(other).unwrap().counters.is_empty());
    }
}

#[test]
fn grouped_tap_preserves_duplicate_ability_instances_without_multiplying_by_permanents() {
    for mut definition in definitions("Deeproot Pilgrimage") {
        definition.abilities.push(definition.abilities[0].clone());
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for _ in 0..3 {
            resource(&mut game, A, Zone::Battlefield, "Creature — Merfolk");
        }
        execute(
            &mut game,
            source,
            A,
            &TapEffect::all(ObjectFilter::creature()),
        );
        assert_eq!(
            settle(&mut game),
            2,
            "two abilities, rather than six per-permanent instances"
        );
        assert_eq!(tokens(&game, A), 2);
    }
}

#[test]
fn filtered_untap_uses_transition_snapshot_when_subject_changes_controller_or_leaves() {
    for definition in definitions("Wake Thrasher") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for leaves in [false, true] {
            let subject = resource(&mut game, A, Zone::Battlefield, "Artifact");
            game.tap(subject);
            execute(
                &mut game,
                source,
                A,
                &UntapEffect::with_spec(ChooseSpec::SpecificObject(subject)),
            );
            if leaves {
                game.move_object_by_effect(subject, Zone::Exile).unwrap();
            } else {
                game.set_current_controller(subject, B).unwrap();
            }
            assert_eq!(
                settle(&mut game),
                1,
                "the subject was controlled by you when it untapped"
            );
        }
        assert_eq!(game.current_power(source), Some(3));
    }
}

#[test]
fn grouped_tap_reference_retains_all_and_only_matching_event_objects() {
    let text = "Type: Enchantment\nWhenever one or more creatures you control become tapped, put a +1/+1 counter on each of them.";
    let (artifact, direct) = compile_to_artifact("Grouped tap reference", text, false).unwrap();
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    for definition in [direct, materialize_artifact(&restored).unwrap()] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mine = [
            resource(&mut game, A, Zone::Battlefield, "Creature"),
            resource(&mut game, A, Zone::Battlefield, "Creature"),
        ];
        let theirs = resource(&mut game, B, Zone::Battlefield, "Creature");
        execute(
            &mut game,
            source,
            A,
            &TapEffect::all(ObjectFilter::creature()),
        );
        assert_eq!(stack(&mut game, &mut TriggerQueue::new()), 1);
        // A later zone change does not extend the event's original identities.
        let exile = game.move_object_by_effect(mine[0], Zone::Exile).unwrap();
        let returned = game
            .move_object_by_effect(exile, Zone::Battlefield)
            .unwrap();
        settle(&mut game);
        assert!(game.object(returned).unwrap().counters.is_empty());
        for object in [mine[1]] {
            assert_eq!(
                game.object(object)
                    .unwrap()
                    .counters
                    .get(&ironsmith::object::CounterType::PlusOnePlusOne),
                Some(&1)
            );
        }
        assert!(game.object(theirs).unwrap().counters.is_empty());
    }
}

#[test]
fn mesmeric_orb_resolves_current_controller_or_exact_departure_lki_without_following_blink() {
    for definition in definitions("Mesmeric Orb") {
        for leaves_and_returns in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for player in [A, B] {
                resource(&mut game, player, Zone::Library, "Sorcery");
            }
            let subject = resource(&mut game, A, Zone::Battlefield, "Artifact");
            game.tap(subject);
            execute(
                &mut game,
                source,
                A,
                &UntapEffect::with_spec(ChooseSpec::SpecificObject(subject)),
            );
            assert_eq!(stack(&mut game, &mut TriggerQueue::new()), 1);
            game.set_current_controller(subject, B).unwrap();
            if leaves_and_returns {
                let exile = game.move_object_by_effect(subject, Zone::Exile).unwrap();
                let returned = game
                    .move_object_by_effect(exile, Zone::Battlefield)
                    .unwrap();
                assert_eq!(game.controller_of_id(returned), Some(A));
            }
            settle(&mut game);
            assert_eq!(
                game.player(B).unwrap().graveyard.len(),
                1,
                "the controller at resolution or immediately before departure mills"
            );
            assert!(
                game.player(A).unwrap().graveyard.is_empty(),
                "neither the frozen untap controller nor a new incarnation controls this reference"
            );
        }
    }
}
