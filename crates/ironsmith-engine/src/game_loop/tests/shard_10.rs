#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_stormbreath_dragon_has_abilities() {
    use crate::ability::AbilityKind;
    use crate::cards::definitions::stormbreath_dragon;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Stormbreath Dragon on battlefield
    let dragon_def = stormbreath_dragon();
    let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

    let dragon = game.object(dragon_id).unwrap();

    // Verify flying
    let has_flying = dragon.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_flying()
        } else {
            false
        }
    });
    assert!(has_flying, "Stormbreath Dragon should have flying");

    // Verify haste
    let has_haste = dragon.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_haste()
        } else {
            false
        }
    });
    assert!(has_haste, "Stormbreath Dragon should have haste");

    // Verify protection from white
    let has_protection = dragon.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_protection()
        } else {
            false
        }
    });
    assert!(
        has_protection,
        "Stormbreath Dragon should have protection from white"
    );

    // Verify activated ability (monstrosity)
    let has_activated = dragon
        .abilities
        .iter()
        .any(|a| matches!(a.kind, AbilityKind::Activated(_)));
    assert!(
        has_activated,
        "Stormbreath Dragon should have monstrosity activated ability"
    );

    // Verify triggered ability (when becomes monstrous)
    let has_triggered = dragon
        .abilities
        .iter()
        .any(|a| matches!(a.kind, AbilityKind::Triggered(_)));
    assert!(
        has_triggered,
        "Stormbreath Dragon should have 'becomes monstrous' trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_stormbreath_dragon_is_monstrous_field() {
    use crate::cards::definitions::stormbreath_dragon;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Stormbreath Dragon on battlefield
    let dragon_def = stormbreath_dragon();
    let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

    // Verify is_monstrous starts false
    assert!(
        !game.is_monstrous(dragon_id),
        "Dragon should not be monstrous initially"
    );

    // Manually set monstrous (simulating effect execution)
    game.set_monstrous(dragon_id);

    // Verify it's now monstrous
    assert!(
        game.is_monstrous(dragon_id),
        "Dragon should be monstrous after being set"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_stormbreath_dragon_trigger_condition() {
    use crate::ability::AbilityKind;
    use crate::cards::definitions::stormbreath_dragon;
    use crate::events::other::BecameMonstrousEvent;
    use crate::triggers::check_triggers;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Stormbreath Dragon on battlefield
    let dragon_def = stormbreath_dragon();
    let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

    // Verify the trigger condition is ThisBecomesMonstrous
    let dragon = game.object(dragon_id).unwrap();
    let has_monstrous_trigger = dragon.abilities.iter().any(|a| {
        if let AbilityKind::Triggered(triggered) = &a.kind {
            triggered.trigger.display().contains("monstrous")
        } else {
            false
        }
    });
    assert!(
        has_monstrous_trigger,
        "Stormbreath Dragon should have ThisBecomesMonstrous trigger"
    );

    // Simulate the BecameMonstrous event
    let event = TriggerEvent::new_with_provenance(
        BecameMonstrousEvent::new(dragon_id, alice, 3),
        crate::provenance::ProvNodeId::default(),
    );

    // Check if triggers fire
    let triggers = check_triggers(&game, &event);
    assert_eq!(
        triggers.len(),
        1,
        "BecameMonstrous should trigger Stormbreath Dragon's ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_zagoth_mamba_mutates_trigger_condition() {
    use crate::ability::AbilityKind;
    use crate::cards::CardDefinitionBuilder;
    use crate::events::other::{BecameMonstrousEvent, MutatedEvent};
    use crate::triggers::check_triggers;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let mamba_def = CardDefinitionBuilder::new(CardId::from_raw(1), "Zagoth Mamba")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Deathtouch\nWhenever this creature mutates, target creature an opponent controls gets -2/-2 until end of turn.",
        )
        .expect("Zagoth Mamba should parse");
    let mamba_id = game.create_object_from_definition(&mamba_def, alice, Zone::Battlefield);

    let mamba = game.object(mamba_id).expect("mamba permanent exists");
    let has_mutates_trigger = mamba.abilities.iter().any(|ability| {
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            triggered.trigger.display().contains("mutates")
        } else {
            false
        }
    });
    assert!(
        has_mutates_trigger,
        "Zagoth Mamba should have a mutates triggered ability"
    );

    let mutate_event = TriggerEvent::new_with_provenance(
        MutatedEvent::new(mamba_id, alice),
        crate::provenance::ProvNodeId::default(),
    );
    let mutate_triggers = check_triggers(&game, &mutate_event);
    assert_eq!(
        mutate_triggers.len(),
        1,
        "MutatedEvent should trigger Zagoth Mamba's ability"
    );

    let monstrous_event = TriggerEvent::new_with_provenance(
        BecameMonstrousEvent::new(mamba_id, alice, 1),
        crate::provenance::ProvNodeId::default(),
    );
    let monstrous_triggers = check_triggers(&game, &monstrous_event);
    assert_eq!(
        monstrous_triggers.len(),
        0,
        "BecameMonstrousEvent should not trigger Zagoth Mamba's mutates ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_archipelagore_mutate_keyword_and_trigger_condition() {
    use crate::ability::AbilityKind;
    use crate::cards::CardDefinitionBuilder;
    use crate::events::other::{BecameMonstrousEvent, MutatedEvent};
    use crate::triggers::check_triggers;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let archipelagore_def = CardDefinitionBuilder::new(CardId::from_raw(1), "Archipelagore")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Mutate {5}{U}\nWhenever this creature mutates, tap up to X target creatures your opponents control, where X is the number of times this creature has mutated. Those creatures don't untap during their controller's next untap step.",
        )
        .expect("Archipelagore should parse");
    let archipelagore_id =
        game.create_object_from_definition(&archipelagore_def, alice, Zone::Battlefield);

    let archipelagore = game
        .object(archipelagore_id)
        .expect("Archipelagore permanent exists");
    let has_mutates_trigger = archipelagore.abilities.iter().any(|ability| {
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            triggered.trigger.display().contains("mutates")
        } else {
            false
        }
    });
    assert!(
        has_mutates_trigger,
        "Archipelagore should have a mutates triggered ability"
    );

    let mutate_event = TriggerEvent::new_with_provenance(
        MutatedEvent::new(archipelagore_id, alice),
        crate::provenance::ProvNodeId::default(),
    );
    let mutate_triggers = check_triggers(&game, &mutate_event);
    assert_eq!(
        mutate_triggers.len(),
        1,
        "MutatedEvent should trigger Archipelagore's ability"
    );

    let monstrous_event = TriggerEvent::new_with_provenance(
        BecameMonstrousEvent::new(archipelagore_id, alice, 1),
        crate::provenance::ProvNodeId::default(),
    );
    let monstrous_triggers = check_triggers(&game, &monstrous_event);
    assert_eq!(
        monstrous_triggers.len(),
        0,
        "BecameMonstrousEvent should not trigger Archipelagore's mutates ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_illuna_apex_of_wishes_mutate_trigger_condition() {
    use crate::ability::AbilityKind;
    use crate::cards::CardDefinitionBuilder;
    use crate::events::other::{BecameMonstrousEvent, MutatedEvent};
    use crate::triggers::check_triggers;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let illuna_def = CardDefinitionBuilder::new(CardId::from_raw(1), "Illuna, Apex of Wishes")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Mutate {3}{R/G}{U}{U}\nFlying, trample\nWhenever this creature mutates, exile cards from the top of your library until you exile a nonland permanent card. Put that card onto the battlefield or into your hand.",
        )
        .expect("Illuna should parse");
    let illuna_id = game.create_object_from_definition(&illuna_def, alice, Zone::Battlefield);

    let illuna = game.object(illuna_id).expect("Illuna permanent exists");
    let has_mutates_trigger = illuna.abilities.iter().any(|ability| {
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            triggered.trigger.display().contains("mutates")
        } else {
            false
        }
    });
    assert!(
        has_mutates_trigger,
        "Illuna should have a mutates triggered ability"
    );

    let mutate_event = TriggerEvent::new_with_provenance(
        MutatedEvent::new(illuna_id, alice),
        crate::provenance::ProvNodeId::default(),
    );
    let mutate_triggers = check_triggers(&game, &mutate_event);
    assert_eq!(
        mutate_triggers.len(),
        1,
        "MutatedEvent should trigger Illuna's ability"
    );

    let monstrous_event = TriggerEvent::new_with_provenance(
        BecameMonstrousEvent::new(illuna_id, alice, 1),
        crate::provenance::ProvNodeId::default(),
    );
    let monstrous_triggers = check_triggers(&game, &monstrous_event);
    assert_eq!(
        monstrous_triggers.len(),
        0,
        "BecameMonstrousEvent should not trigger Illuna's mutates ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
fn setup_zero_cost_mutate_cast(
    game: &mut GameState,
    controller: PlayerId,
    host_id: ObjectId,
    name: &str,
    as_commander: bool,
) -> (PriorityLoopState, TriggerQueue, ObjectId, ObjectId) {
    let definition = CardDefinitionBuilder::new(CardId::from_raw(998_140), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(6)]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Mutate {0}\nFlying\nWhenever this creature mutates, you gain 1 life.")
        .expect("mutate test creature should parse");
    let card_id = game.create_object_from_definition(&definition, controller, Zone::Hand);
    if as_commander {
        game.set_as_commander(card_id, controller);
    }

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = controller;
    game.turn.priority_player = Some(controller);

    assert!(
        compute_legal_actions(game, controller)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *spell_id == card_id
            )),
        "Mutate should be offered when a legal same-owner non-Human target exists"
    );

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let progress = apply_priority_response(
        game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: card_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Alternative(0),
        }),
    )
    .expect("mutate cast should begin");
    assert!(matches!(
        progress,
        GameProgress::NeedsDecisionCtx(crate::decisions::DecisionContext::Targets(_))
    ));
    apply_priority_response(
        game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(host_id)]),
    )
    .expect("legal mutate target should complete the cast");

    let stack_id = game
        .stack
        .last()
        .map(|entry| entry.object_id)
        .expect("mutating creature spell should be on the stack");
    (state, trigger_queue, stack_id, card_id)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mutate_merges_without_entering_and_splits_into_every_component() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let host_definition = CardDefinitionBuilder::new(CardId::from_raw(998_141), "Vigilant Host")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text("Vigilance\nWhenever this creature mutates, you gain 1 life.")
        .expect("host should parse");
    let host_id = game.create_object_from_definition(&host_definition, alice, Zone::Battlefield);

    let human_definition = CardDefinitionBuilder::new(CardId::from_raw(998_142), "Human Host")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&human_definition, alice, Zone::Battlefield);
    game.create_object_from_definition(&host_definition, bob, Zone::Battlefield);

    let (_state, mut trigger_queue, stack_id, _commander_identity) =
        setup_zero_cost_mutate_cast(&mut game, alice, host_id, "Winged Mutator", false);
    resolve_stack_entry_with_dm_and_triggers(
        &mut game,
        &mut AutoPassDecisionMaker,
        &mut trigger_queue,
    )
    .expect("mutating creature should resolve");

    let merged = game
        .object(host_id)
        .expect("target remains the same permanent");
    assert_eq!(merged.name.as_ref(), "Winged Mutator");
    assert!(
        merged
            .abilities
            .iter()
            .any(|ability| crate::runtime_display::ability_surface_text(ability) == "Flying")
    );
    assert!(
        merged
            .abilities
            .iter()
            .any(|ability| crate::runtime_display::ability_surface_text(ability) == "Vigilance")
    );
    assert!(game.object(stack_id).is_none());
    assert_eq!(game.mutation_count(host_id), 1);
    assert_eq!(
        trigger_queue.entries.len(),
        2,
        "every component's mutates ability must trigger"
    );
    assert_eq!(
        game.merged_permanent(merged.stable_id)
            .expect("component metadata")
            .components
            .len(),
        2
    );
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|id| game
                .object(**id)
                .is_some_and(|object| object.owner == alice))
            .count(),
        2,
        "merge must not create a second Alice permanent or produce an ETB"
    );

    game.move_object_by_effect(host_id, Zone::Graveyard)
        .expect("merged permanent should leave");
    let mut component_names = game
        .player(alice)
        .expect("Alice")
        .graveyard
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect::<Vec<_>>();
    component_names.sort();
    assert_eq!(component_names, vec!["Vigilant Host", "Winged Mutator"]);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mutate_illegal_target_resolves_as_an_ordinary_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let host_id = create_creature(&mut game, "Doomed Mutate Host", alice, 2, 2);
    let (_state, _trigger_queue, stack_id, _commander_identity) =
        setup_zero_cost_mutate_cast(&mut game, alice, host_id, "Fallback Mutator", false);

    game.move_object_by_effect(host_id, Zone::Graveyard)
        .expect("target should leave before resolution");
    resolve_stack_entry(&mut game).expect("illegal-target Mutate must not fizzle");

    let resolved = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Fallback Mutator")
        })
        .expect("mutating spell should resolve as an ordinary creature");
    assert_ne!(
        resolved, stack_id,
        "ordinary zone change still creates a new id"
    );
    assert_eq!(game.mutation_count(resolved), 0);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mutate_bottom_component_keeps_host_characteristics_and_commander_identity() {
    struct PutMutatingSpellOnBottom;

    impl DecisionMaker for PutMutatingSpellOnBottom {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.legal && option.description.contains("bottom"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| {
                    ctx.options
                        .iter()
                        .filter(|option| option.legal)
                        .map(|option| option.index)
                        .take(ctx.min)
                        .collect()
                })
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let host_definition = CardDefinitionBuilder::new(CardId::from_raw(998_143), "Top Host")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text("Vigilance")
        .expect("host should parse");
    let host_id = game.create_object_from_definition(&host_definition, alice, Zone::Battlefield);
    let (_state, mut trigger_queue, _stack_id, commander_identity) =
        setup_zero_cost_mutate_cast(&mut game, alice, host_id, "Commander Mutator", true);

    resolve_stack_entry_with_dm_and_triggers(
        &mut game,
        &mut PutMutatingSpellOnBottom,
        &mut trigger_queue,
    )
    .expect("bottom mutation should resolve");

    let merged = game.object(host_id).expect("host remains on battlefield");
    assert_eq!(merged.name.as_ref(), "Top Host");
    assert_eq!(game.commander_identity(host_id), Some(commander_identity));
    assert_eq!(
        game.current_commander_object(commander_identity),
        Some(host_id)
    );
    assert!(
        merged
            .abilities
            .iter()
            .any(|ability| crate::runtime_display::ability_surface_text(ability) == "Flying"),
        "bottom component still contributes its abilities"
    );
}

fn u046_linked_creature_face(
    id: u32,
    name: &str,
    other_id: u32,
    other_name: &str,
    layout: crate::card::LinkedFaceLayout,
) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(2, 2))
        .other_face(CardId::from_raw(other_id))
        .other_face_name(other_name)
        .linked_face_layout(layout)
        .build()
}

#[test]
fn merged_face_status_uses_the_top_component_and_obeys_turn_restrictions() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let host = CardDefinitionBuilder::new(CardId::from_raw(998_144), "Hidden Host")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let host_id = game.create_object_from_definition(&host, alice, Zone::Battlefield);
    assert!(game.set_face_down(host_id));

    let top = CardDefinitionBuilder::new(CardId::from_raw(998_145), "Visible Mutator")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let top_id = game.create_object_from_definition(&top, alice, Zone::Stack);
    game.merge_mutating_creature_spell(top_id, host_id, true)
        .expect("legal direct merge");

    assert!(
        !game.is_face_down(host_id),
        "putting a face-up component on top makes the merged permanent face up"
    );
    let merged = game
        .merged_permanent(game.object(host_id).expect("merged permanent").stable_id)
        .expect("component state");
    assert!(!merged.components[0].face_down);
    assert!(merged.components[1].face_down);

    assert!(game.set_face_down(host_id));
    let merged = game
        .merged_permanent(game.object(host_id).expect("merged permanent").stable_id)
        .expect("component state");
    assert!(
        merged
            .components
            .iter()
            .all(|component| component.face_down)
    );
    assert!(game.set_face_up(host_id));
    let merged = game
        .merged_permanent(game.object(host_id).expect("merged permanent").stable_id)
        .expect("component state");
    assert!(
        merged
            .components
            .iter()
            .all(|component| !component.face_down)
    );

    let front = u046_linked_creature_face(
        998_146,
        "DFC Host Front",
        998_147,
        "DFC Host Back",
        crate::card::LinkedFaceLayout::TransformLike,
    );
    let back = u046_linked_creature_face(
        998_147,
        "DFC Host Back",
        998_146,
        "DFC Host Front",
        crate::card::LinkedFaceLayout::TransformLike,
    );
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    let dfc_id = game.create_object_from_definition(&front, alice, Zone::Battlefield);
    let under_id = game.create_object_from_definition(&top, alice, Zone::Stack);
    game.merge_mutating_creature_spell(under_id, dfc_id, false)
        .expect("DFC host can merge");
    assert!(
        !game.set_face_down(dfc_id),
        "a face-up merged permanent containing a double-faced card cannot turn face down"
    );
    assert!(!game.is_face_down(dfc_id));

    let hidden_id = game.create_object_from_definition(&host, alice, Zone::Battlefield);
    assert!(game.set_face_down(hidden_id));
    let instant = CardDefinitionBuilder::new(CardId::from_raw(998_148), "Buried Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let instant_id = game.create_object_from_definition(&instant, alice, Zone::Stack);
    game.merge_mutating_creature_spell(instant_id, hidden_id, false)
        .expect("fixture creates a merged instant component");
    assert!(
        !game.set_face_up(hidden_id),
        "a face-down merged permanent containing an instant card cannot turn face up"
    );
    assert!(game.is_face_down(hidden_id));
    let reveal = game
        .ui_effect_events()
        .find(|event| event.kind == "reveal")
        .expect("the failed turn-up action must reveal the merged permanent");
    assert!(
        reveal
            .text
            .as_deref()
            .is_some_and(|text| text.contains("Buried Instant"))
    );
}

#[test]
fn merged_transform_and_flip_actions_update_every_applicable_component() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let host_front = u046_linked_creature_face(
        998_149,
        "Transform Host Front",
        998_150,
        "Transform Host Back",
        crate::card::LinkedFaceLayout::TransformLike,
    );
    let host_back = u046_linked_creature_face(
        998_150,
        "Transform Host Back",
        998_149,
        "Transform Host Front",
        crate::card::LinkedFaceLayout::TransformLike,
    );
    let under_front = u046_linked_creature_face(
        998_151,
        "Transform Under Front",
        998_152,
        "Transform Under Back",
        crate::card::LinkedFaceLayout::TransformLike,
    );
    let under_back = u046_linked_creature_face(
        998_152,
        "Transform Under Back",
        998_151,
        "Transform Under Front",
        crate::card::LinkedFaceLayout::TransformLike,
    );
    for definition in [&host_front, &host_back, &under_front, &under_back] {
        game.register_linked_face_definition(definition);
    }
    let host_id = game.create_object_from_definition(&host_front, alice, Zone::Battlefield);
    let under_id = game.create_object_from_definition(&under_front, alice, Zone::Stack);
    game.merge_mutating_creature_spell(under_id, host_id, false)
        .expect("transform fixture merges");
    assert!(game.transform_permanent(host_id));
    assert_eq!(
        game.object(host_id)
            .expect("live merged object")
            .name
            .as_ref(),
        "Transform Host Back"
    );
    let transformed_names = game
        .merged_permanent(game.object(host_id).expect("live merged object").stable_id)
        .expect("component state")
        .components
        .iter()
        .map(|component| component.object.name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        transformed_names,
        vec!["Transform Host Back", "Transform Under Back"]
    );
    game.move_object_by_game_rule(host_id, Zone::Graveyard)
        .expect("transformed merged permanent should split in the graveyard");
    let destination_names = game
        .take_zone_change_results(host_id)
        .into_iter()
        .map(|object_id| {
            game.object(object_id)
                .expect("split component in the graveyard")
                .name
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        destination_names,
        vec!["Transform Host Front", "Transform Under Front"],
        "transformed components use their immutable front-face snapshots after splitting"
    );

    let flip_host_front = u046_linked_creature_face(
        998_153,
        "Flip Host Front",
        998_154,
        "Flip Host Back",
        crate::card::LinkedFaceLayout::None,
    );
    let flip_host_back = u046_linked_creature_face(
        998_154,
        "Flip Host Back",
        998_153,
        "Flip Host Front",
        crate::card::LinkedFaceLayout::None,
    );
    let flip_under_front = u046_linked_creature_face(
        998_155,
        "Flip Under Front",
        998_156,
        "Flip Under Back",
        crate::card::LinkedFaceLayout::None,
    );
    let flip_under_back = u046_linked_creature_face(
        998_156,
        "Flip Under Back",
        998_155,
        "Flip Under Front",
        crate::card::LinkedFaceLayout::None,
    );
    for definition in [
        &flip_host_front,
        &flip_host_back,
        &flip_under_front,
        &flip_under_back,
    ] {
        game.register_linked_face_definition(definition);
    }
    let flip_id = game.create_object_from_definition(&flip_host_front, alice, Zone::Battlefield);
    let flip_under_id = game.create_object_from_definition(&flip_under_front, alice, Zone::Stack);
    game.merge_mutating_creature_spell(flip_under_id, flip_id, false)
        .expect("flip fixture merges");
    assert!(game.flip_permanent(flip_id));
    assert_eq!(
        game.object(flip_id)
            .expect("live flipped object")
            .name
            .as_ref(),
        "Flip Host Back"
    );
    let flipped_names = game
        .merged_permanent(game.object(flip_id).expect("live flipped object").stable_id)
        .expect("component state")
        .components
        .iter()
        .map(|component| component.object.name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(flipped_names, vec!["Flip Host Back", "Flip Under Back"]);
}

#[test]
fn merged_exile_order_sets_relative_timestamps_in_the_exiling_players_order() {
    #[derive(Debug, Default)]
    struct ReverseMergedOrder;

    impl DecisionMaker for ReverseMergedOrder {
        fn decide_order(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            ctx.items.iter().rev().map(|(id, _)| *id).collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let host = CardDefinitionBuilder::new(CardId::from_raw(998_157), "Timestamp Host")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let middle = CardDefinitionBuilder::new(CardId::from_raw(998_158), "Timestamp Middle")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let top = CardDefinitionBuilder::new(CardId::from_raw(998_159), "Timestamp Top")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let host_id = game.create_object_from_definition(&host, alice, Zone::Battlefield);
    let middle_id = game.create_object_from_definition(&middle, alice, Zone::Stack);
    game.merge_mutating_creature_spell(middle_id, host_id, true)
        .expect("first merge");
    let top_id = game.create_object_from_definition(&top, alice, Zone::Stack);
    game.merge_mutating_creature_spell(top_id, host_id, true)
        .expect("second merge");

    let cause = crate::events::cause::EventCause::from_effect(host_id, alice);
    let outcome = crate::effects::zones::apply_zone_change(
        &mut game,
        host_id,
        Zone::Battlefield,
        Zone::Exile,
        cause,
        &mut ReverseMergedOrder,
    );
    let crate::events::processing::EventOutcome::Proceed(result) = outcome else {
        panic!("merged exile should proceed");
    };
    let timestamps = result
        .new_object_ids
        .iter()
        .map(|object_id| {
            game.effect_store
                .continuous_effects
                .get_entry_timestamp(*object_id)
                .expect("every exiled component gets a timestamp")
        })
        .collect::<Vec<_>>();
    assert!(
        timestamps.windows(2).all(|pair| pair[0] < pair[1]),
        "the chosen exile order must be the relative timestamp order: {timestamps:?}"
    );
}

#[test]
fn token_top_merged_permanent_partitions_card_only_zone_replacement() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let host = CardDefinitionBuilder::new(CardId::from_raw(998_160), "Replacement Host")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let token_top = CardDefinitionBuilder::new(CardId::from_raw(998_161), "Token Mutator")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let replacement_source = create_creature(&mut game, "Replacement Source", alice, 1, 1);
    let host_id = game.create_object_from_definition(&host, alice, Zone::Battlefield);
    let token_id = game.create_object_from_definition(&token_top, alice, Zone::Stack);
    game.object_mut(token_id).expect("token component").kind = crate::object::ObjectKind::Token;
    game.merge_mutating_creature_spell(token_id, host_id, true)
        .expect("token-top fixture merges");
    assert_eq!(
        game.object(host_id).expect("merged permanent").kind,
        crate::object::ObjectKind::Token
    );

    game.effect_store.replacement_effects.add_effect(
        crate::replacement::ZoneReplacementSpec::new(
            crate::target::ObjectFilter::default().nontoken(),
            Zone::Exile,
        )
        .from_zone(Zone::Battlefield)
        .to_zone(Zone::Graveyard)
        .build(replacement_source, alice),
    );
    let cause = crate::events::cause::EventCause::from_effect(replacement_source, alice);
    let outcome = crate::effects::zones::apply_zone_change(
        &mut game,
        host_id,
        Zone::Battlefield,
        Zone::Graveyard,
        cause,
        &mut SelectFirstDecisionMaker,
    );
    let crate::events::processing::EventOutcome::Proceed(result) = outcome else {
        panic!("partitioned merged zone change should proceed");
    };
    let component_zones = result
        .new_object_ids
        .iter()
        .filter_map(|object_id| {
            game.object(*object_id)
                .map(|object| (object.name.to_string(), object.zone))
        })
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(component_zones.get("Token Mutator"), Some(&Zone::Graveyard));
    assert_eq!(component_zones.get("Replacement Host"), Some(&Zone::Exile));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_anger_grants_haste_from_graveyard_when_you_control_mountain() {
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    use crate::cards::definitions::basic_mountain;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbilityId;
    use crate::types::Subtype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let anger_def = CardDefinitionBuilder::new(CardId::from_raw(397), "Anger")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text(
                "Haste\nAs long as this card is in your graveyard and you control a Mountain, creatures you control have haste.",
            )
            .expect("anger text should parse");

    let test_creature = CardBuilder::new(CardId::new(), "Test Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&test_creature, alice, Zone::Battlefield);

    assert!(
        !game.object_has_static_ability_id(creature_id, StaticAbilityId::Haste),
        "creature should not have haste before Anger is in graveyard"
    );

    let _anger_id = game.create_object_from_definition(&anger_def, alice, Zone::Graveyard);
    assert!(
        !game.object_has_static_ability_id(creature_id, StaticAbilityId::Haste),
        "creature should not have haste without a Mountain"
    );

    let mountain_def = basic_mountain();
    let _mountain_id = game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);
    assert!(
        game.object_has_static_ability_id(creature_id, StaticAbilityId::Haste),
        "creature should have haste when Anger is in graveyard and you control a Mountain"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_geist_of_saint_traft_has_abilities() {
    use crate::ability::AbilityKind;
    use crate::cards::definitions::geist_of_saint_traft;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Geist on battlefield
    let geist_def = geist_of_saint_traft();
    let geist_id = game.create_object_from_definition(&geist_def, alice, Zone::Battlefield);

    let geist = game.object(geist_id).unwrap();

    // Verify hexproof
    let has_hexproof = geist.abilities.iter().any(|a| {
        if let AbilityKind::Static(s) = &a.kind {
            s.has_hexproof()
        } else {
            false
        }
    });
    assert!(has_hexproof, "Geist should have hexproof");

    // Verify attack trigger
    let has_attack_trigger = geist.abilities.iter().any(|a| {
        if let AbilityKind::Triggered(triggered) = &a.kind {
            triggered.trigger.display().contains("attacks")
        } else {
            false
        }
    });
    assert!(
        has_attack_trigger,
        "Geist should have 'when this attacks' trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_geist_of_saint_traft_attack_trigger() {
    use crate::ability::AbilityKind;
    use crate::cards::definitions::geist_of_saint_traft;
    use crate::triggers::{AttackEventTarget, check_triggers};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Create Geist on battlefield
    let geist_def = geist_of_saint_traft();
    let geist_id = game.create_object_from_definition(&geist_def, alice, Zone::Battlefield);

    // Remove summoning sickness
    game.remove_summoning_sickness(geist_id);

    // Simulate the attack event
    let event = TriggerEvent::new_with_provenance(
        CreatureAttackedEvent::new(geist_id, AttackEventTarget::Player(bob)),
        crate::provenance::ProvNodeId::default(),
    );

    // Check if triggers fire
    let triggers = check_triggers(&game, &event);
    assert_eq!(
        triggers.len(),
        1,
        "Attacking with Geist should trigger its ability"
    );

    // Verify the trigger creates a token with modifications
    let geist = game.object(geist_id).unwrap();
    let trigger = geist.abilities.iter().find(|a| {
        if let AbilityKind::Triggered(triggered) = &a.kind {
            triggered.trigger.display().contains("attacks")
        } else {
            false
        }
    });
    assert!(trigger.is_some());

    if let Some(ability) = trigger
        && let AbilityKind::Triggered(triggered) = &ability.kind
    {
        // Verify the effect creates a token
        assert!(!triggered.effects.is_empty());
        let has_token_effect = triggered
            .effects
            .iter()
            .any(|e| format!("{:?}", e).contains("CreateToken"));
        assert!(
            has_token_effect,
            "Geist's trigger should create a token with modifications"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_geist_token_has_correct_modifications() {
    use crate::ability::AbilityKind;
    use crate::cards::definitions::geist_of_saint_traft;

    let geist_def = geist_of_saint_traft();

    // Find the triggered ability
    let trigger = geist_def
        .abilities
        .iter()
        .find(|a| matches!(a.kind, AbilityKind::Triggered(_)));
    assert!(trigger.is_some());

    if let Some(ability) = trigger
        && let AbilityKind::Triggered(triggered) = &ability.kind
    {
        // Find the token creation effect
        let token_effect = triggered
            .effects
            .iter()
            .find(|e| format!("{:?}", e).contains("CreateToken"));
        assert!(
            token_effect.is_some(),
            "Should have a token creation effect"
        );

        // The actual token properties are tested via integration tests
        // that create the token and verify its characteristics
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_stormbreath_dragon_monstrosity_adds_counters() {
    use crate::cards::definitions::stormbreath_dragon;
    use crate::effect::Effect;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Stormbreath Dragon on battlefield
    let dragon_def = stormbreath_dragon();
    let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

    // Verify initial state: not monstrous, no +1/+1 counters
    assert!(!game.is_monstrous(dragon_id));
    {
        let dragon = game.object(dragon_id).unwrap();
        assert_eq!(dragon.counters.get(&CounterType::PlusOnePlusOne), None);
        assert_eq!(dragon.power(), Some(4));
        assert_eq!(dragon.toughness(), Some(4));
    }

    // Execute the Monstrosity 3 effect
    let mut ctx = ExecutionContext::new_default(dragon_id, alice);
    let effect = Effect::monstrosity(3);

    let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

    // Verify result indicates monstrosity was applied
    assert!(matches!(
        result.value,
        crate::effect::OutcomeValue::MonstrosityApplied { creature, n } if creature == dragon_id && n == 3
    ));

    // Verify dragon is now monstrous with 3 +1/+1 counters
    assert!(game.is_monstrous(dragon_id), "Dragon should be monstrous");
    let dragon = game.object(dragon_id).unwrap();
    assert_eq!(
        dragon.counters.get(&CounterType::PlusOnePlusOne),
        Some(&3),
        "Dragon should have 3 +1/+1 counters"
    );
    // 4/4 + 3 counters = 7/7
    assert_eq!(dragon.power(), Some(7));
    assert_eq!(dragon.toughness(), Some(7));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_stormbreath_dragon_monstrosity_only_works_once() {
    use crate::cards::definitions::stormbreath_dragon;
    use crate::effect::Effect;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Stormbreath Dragon on battlefield
    let dragon_def = stormbreath_dragon();
    let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

    // Execute monstrosity once
    let mut ctx = ExecutionContext::new_default(dragon_id, alice);
    let effect = Effect::monstrosity(3);
    execute_effect(&mut game, &effect, &mut ctx).unwrap();

    // Verify it worked
    assert!(game.is_monstrous(dragon_id));
    assert_eq!(
        game.object(dragon_id)
            .unwrap()
            .counters
            .get(&CounterType::PlusOnePlusOne),
        Some(&3)
    );

    // Try to execute monstrosity again
    let mut ctx2 = ExecutionContext::new_default(dragon_id, alice);
    let result = execute_effect(&mut game, &effect, &mut ctx2).unwrap();

    // Should return Count(0) - nothing happened
    assert_eq!(
        result.value,
        crate::effect::OutcomeValue::Count(0),
        "Second monstrosity should do nothing"
    );

    // Counters should still be 3 (not 6)
    assert_eq!(
        game.object(dragon_id)
            .unwrap()
            .counters
            .get(&CounterType::PlusOnePlusOne),
        Some(&3),
        "Counters should not have increased"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_stormbreath_dragon_becomes_monstrous_trigger_fires() {
    use crate::cards::definitions::stormbreath_dragon;
    use crate::effect::Effect;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::events::other::BecameMonstrousEvent;
    use crate::triggers::check_triggers;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Stormbreath Dragon on battlefield
    let dragon_def = stormbreath_dragon();
    let dragon_id = game.create_object_from_definition(&dragon_def, alice, Zone::Battlefield);

    // Execute monstrosity
    let mut ctx = ExecutionContext::new_default(dragon_id, alice);
    let effect = Effect::monstrosity(3);
    execute_effect(&mut game, &effect, &mut ctx).unwrap();

    // Now simulate the BecameMonstrous event (which would be generated by the game loop)
    let event = TriggerEvent::new_with_provenance(
        BecameMonstrousEvent::new(dragon_id, alice, 3),
        crate::provenance::ProvNodeId::default(),
    );

    // Check if the dragon's "becomes monstrous" trigger fires
    let triggers = check_triggers(&game, &event);

    assert_eq!(
        triggers.len(),
        1,
        "Stormbreath Dragon's 'becomes monstrous' trigger should fire"
    );

    // Verify the trigger is from the dragon
    assert_eq!(triggers[0].source, dragon_id);
    assert_eq!(triggers[0].controller, alice);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_fleecemane_lion_gains_keywords_when_monstrous() {
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    use crate::effect::Effect;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::static_abilities::StaticAbilityId;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let def = CardDefinitionBuilder::new(CardId::new(), "Fleecemane Lion")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "{3}{G}{W}: Monstrosity 1. (If this creature isn't monstrous, put a +1/+1 counter on it and it becomes monstrous.)\nAs long as this creature is monstrous, it has hexproof and indestructible.",
        )
        .expect("parse Fleecemane Lion text");
    let lion_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert!(
        !game.object_has_static_ability_id(lion_id, StaticAbilityId::Hexproof)
            && !game.object_has_static_ability_id(lion_id, StaticAbilityId::Indestructible),
        "Fleecemane Lion should not have monstrous-only keywords before monstrosity"
    );

    let mut ctx = ExecutionContext::new_default(lion_id, alice);
    execute_effect(&mut game, &Effect::monstrosity(1), &mut ctx).expect("resolve monstrosity");

    assert!(game.is_monstrous(lion_id), "lion should become monstrous");
    assert!(
        game.object_has_static_ability_id(lion_id, StaticAbilityId::Hexproof)
            && game.object_has_static_ability_id(lion_id, StaticAbilityId::Indestructible),
        "Fleecemane Lion should gain hexproof and indestructible once monstrous"
    );
}

// =========================================================================
// Integration Tests for New Features
// =========================================================================

#[test]
pub(super) fn test_once_per_turn_ability_tracking() {
    // Test that OncePerTurn abilities can only be activated once per turn
    use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create a permanent with a OncePerTurn activated ability
    let creature_id = create_creature(&mut game, "Test Creature", alice, 2, 2);

    // Add a OncePerTurn activated ability (e.g., "{T}: Draw a card")
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::from_cost(crate::costs::Cost::tap()),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                timing: ActivationTiming::OncePerTurn,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    // Remove summoning sickness
    game.remove_summoning_sickness(creature_id);

    // Verify the ability hasn't been activated this turn
    assert!(!game.ability_activated_this_turn(creature_id, 0));

    // Record the activation
    game.record_ability_activation(creature_id, 0);

    // Verify the ability is now tracked as activated
    assert!(game.ability_activated_this_turn(creature_id, 0));

    // Simulate next turn - tracking should be cleared
    game.next_turn();
    assert!(!game.ability_activated_this_turn(creature_id, 0));
}

#[test]
pub(super) fn test_activate_no_more_than_twice_each_turn_restriction() {
    use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let creature_id = create_creature(&mut game, "Battleflies Test", alice, 0, 1);

    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec!["Activate no more than twice each turn.".to_string()],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let ability = match &game
        .object(creature_id)
        .expect("battleflies test creature exists")
        .abilities[0]
        .kind
    {
        AbilityKind::Activated(activated) => activated.clone(),
        _ => panic!("expected activated ability"),
    };

    assert!(
        can_activate_ability_with_restrictions(&game, creature_id, 0, &ability),
        "ability should be activatable before any uses this turn"
    );

    game.record_ability_activation(creature_id, 0);
    assert!(
        can_activate_ability_with_restrictions(&game, creature_id, 0, &ability),
        "ability should still be activatable after first use"
    );

    game.record_ability_activation(creature_id, 0);
    assert!(
        !can_activate_ability_with_restrictions(&game, creature_id, 0, &ability),
        "ability should be blocked after two uses in the same turn"
    );
}

#[test]
pub(super) fn test_non_mana_activation_condition_max_activations_per_turn_is_enforced() {
    use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let creature_id = create_creature(&mut game, "Activation Condition Test", alice, 1, 1);

    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: Some(crate::ConditionExpr::MaxActivationsPerTurn(2)),
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let ability = match &game
        .object(creature_id)
        .expect("activation condition test creature exists")
        .abilities[0]
        .kind
    {
        AbilityKind::Activated(activated) => activated.clone(),
        _ => panic!("expected activated ability"),
    };

    assert!(can_activate_ability_with_restrictions(
        &game,
        creature_id,
        0,
        &ability
    ));
    game.record_ability_activation(creature_id, 0);
    assert!(can_activate_ability_with_restrictions(
        &game,
        creature_id,
        0,
        &ability
    ));
    game.record_ability_activation(creature_id, 0);
    assert!(!can_activate_ability_with_restrictions(
        &game,
        creature_id,
        0,
        &ability
    ));
}

#[test]
pub(super) fn test_protection_from_permanents_blocking() {
    use crate::ability::ProtectionFrom;
    use crate::rules::combat::can_block;
    use crate::target::ObjectFilter;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Create attacker with protection from green creatures
    let attacker_id = create_creature(&mut game, "Protected Attacker", alice, 2, 2);
    let green_filter = ObjectFilter {
        colors: Some(crate::color::ColorSet::GREEN),
        card_types: vec![CardType::Creature],
        ..Default::default()
    };
    game.object_mut(attacker_id)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(
            crate::static_abilities::StaticAbility::protection(ProtectionFrom::Permanents(
                green_filter,
            )),
        ));

    // Create a green creature blocker
    let green_blocker_id = create_creature(&mut game, "Green Blocker", bob, 2, 2);
    game.object_mut(green_blocker_id).unwrap().color_override = Some(crate::color::ColorSet::GREEN);

    // Create a red creature blocker
    let red_blocker_id = create_creature(&mut game, "Red Blocker", bob, 2, 2);
    game.object_mut(red_blocker_id).unwrap().color_override = Some(crate::color::ColorSet::RED);

    let attacker = game.object(attacker_id).unwrap();
    let green_blocker = game.object(green_blocker_id).unwrap();
    let red_blocker = game.object(red_blocker_id).unwrap();

    // Green creature should NOT be able to block (protection)
    assert!(
        !can_block(attacker, green_blocker, &game),
        "Green creature should not be able to block creature with protection from green creatures"
    );

    // Red creature SHOULD be able to block
    assert!(
        can_block(attacker, red_blocker, &game),
        "Red creature should be able to block creature with protection from green creatures"
    );
}

#[test]
pub(super) fn test_cleanup_discard_decision() {
    use crate::turn::{apply_cleanup_discard, get_cleanup_discard_spec};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;

    // Add 9 cards to hand (2 over max hand size of 7)
    for i in 0..9 {
        let card = CardBuilder::new(CardId::new(), format!("Card {}", i))
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&card, alice, Zone::Hand);
    }

    assert_eq!(game.player(alice).unwrap().hand.len(), 9);

    // Get the discard spec
    let result = get_cleanup_discard_spec(&game);
    assert!(result.is_some());

    let (player, spec) = result.unwrap();
    assert_eq!(player, alice);
    assert_eq!(spec.count, 2);
    assert_eq!(spec.hand.len(), 9);

    // Simulate player choosing specific cards to discard
    let cards_to_discard = vec![spec.hand[0], spec.hand[1]];
    let mut dm = crate::decision::AutoPassDecisionMaker;
    apply_cleanup_discard(&mut game, &cards_to_discard, &mut dm);

    // Verify hand size is now 7
    assert_eq!(game.player(alice).unwrap().hand.len(), 7);
    // Verify graveyard has 2 cards
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necropotence_cleanup_discard_exiles_discarded_card() {
    use crate::turn::{apply_cleanup_discard, get_cleanup_discard_spec};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;

    let necropotence = CardDefinitionBuilder::new(CardId::new(), "Necropotence")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Skip your draw step.\n\
             Whenever you discard a card, exile that card from your graveyard.\n\
             Pay 1 life: Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step.",
        )
        .expect("Necropotence should parse");
    let necropotence_debug = format!("{:#?}", necropotence.abilities);
    assert!(
        necropotence_debug.contains("YouDiscardCardTrigger")
            && necropotence_debug.contains("TagTriggeringObjectEffect")
            && necropotence_debug.contains("ExileEffect"),
        "Necropotence should compile its discard trigger into a tagged exile effect, got {necropotence_debug}"
    );
    game.create_object_from_definition(&necropotence, alice, Zone::Battlefield);

    let mut hand_ids = Vec::new();
    for idx in 0..8 {
        let card = CardBuilder::new(CardId::new(), format!("Cleanup Card {idx}"))
            .card_types(vec![CardType::Sorcery])
            .build();
        hand_ids.push(game.create_object_from_card(&card, alice, Zone::Hand));
    }
    let discarded_name = game
        .object(hand_ids[0])
        .expect("discard candidate should exist")
        .name
        .clone();

    let mut trigger_queue = TriggerQueue::new();
    let mut decision_maker = AutoPassDecisionMaker;
    let (player, spec) = get_cleanup_discard_spec(&game).expect("cleanup discard should be needed");
    assert_eq!(player, alice);
    assert_eq!(spec.count, 1);

    apply_cleanup_discard(&mut game, &[hand_ids[0]], &mut decision_maker);
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Necropotence should trigger from cleanup discard"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Necropotence discard trigger should go on stack");
    resolve_stack_entry_with(&mut game, &mut decision_maker)
        .expect("Necropotence discard trigger should resolve");

    let graveyard_names: Vec<_> = game
        .player(alice)
        .expect("alice")
        .graveyard
        .iter()
        .filter_map(|id| game.object(*id).map(|obj| obj.name.to_string()))
        .collect();
    let exile_names: Vec<_> = game
        .exile
        .iter()
        .filter_map(|id| game.object(*id).map(|obj| obj.name.to_string()))
        .collect();

    assert_eq!(game.player(alice).expect("alice").hand.len(), 7);
    assert!(
        !graveyard_names.contains(&discarded_name.to_string()),
        "{discarded_name} should not remain in graveyard after Necropotence trigger"
    );
    assert!(
        exile_names.contains(&discarded_name.to_string()),
        "{discarded_name} should be exiled by Necropotence after cleanup discard"
    );
}

#[test]
pub(super) fn test_legend_rule_decision() {
    use crate::rules::state_based::{apply_legend_rule_choice, get_legend_rule_specs};
    use crate::types::Supertype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create two legendary creatures with the same name
    let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();

    let legend1_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
    let _legend2_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);

    // Get legend rule specs
    let specs = get_legend_rule_specs(&game);
    assert_eq!(specs.len(), 1, "Should have one legend rule spec");

    let (player, spec) = &specs[0];
    assert_eq!(*player, alice);
    assert_eq!(spec.name, "Isamaru, Hound of Konda");
    assert_eq!(spec.legends.len(), 2);

    // Player chooses to keep the first legend
    apply_legend_rule_choice(&mut game, legend1_id);

    // Verify only one legend remains on battlefield
    assert_eq!(game.battlefield.len(), 1);
    assert!(game.battlefield.contains(&legend1_id));

    // The second legend should be in graveyard (with new ID due to zone change)
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
}

#[test]
pub(super) fn legend_rule_uses_current_controller_after_control_change() {
    use crate::rules::state_based::{apply_legend_rule_choice, get_legend_rule_specs};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();

    let alice_legend = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
    let bob_legend = game.create_object_from_card(&legend_card, bob, Zone::Battlefield);
    assert!(
        get_legend_rule_specs(&game).is_empty(),
        "separate controllers should not violate the legend rule"
    );

    game.set_current_controller(bob_legend, alice);

    let specs = get_legend_rule_specs(&game);
    assert_eq!(
        specs.len(),
        1,
        "control change should create a legend-rule choice"
    );
    let (player, spec) = &specs[0];
    assert_eq!(*player, alice);
    assert_eq!(spec.legends.len(), 2);
    assert!(spec.legends.contains(&alice_legend));
    assert!(spec.legends.contains(&bob_legend));

    apply_legend_rule_choice(&mut game, alice_legend);

    assert!(game.battlefield.contains(&alice_legend));
    assert!(!game.battlefield.contains(&bob_legend));
    assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
}

#[test]
pub(super) fn test_may_effect_with_callback() {
    use crate::decision::DecisionMaker;
    use crate::effects::ExecutionContext;

    // A decision maker that always accepts May effects
    struct AcceptMayDecisionMaker;
    impl DecisionMaker for AcceptMayDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    // A decision maker that always declines May effects
    struct DeclineMayDecisionMaker;
    impl DecisionMaker for DeclineMayDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let legal: Vec<ObjectId> = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect();
            let required = ctx.min.max(1);
            let count = ctx.max.unwrap_or(required).min(legal.len()).max(required);
            legal.into_iter().take(count).collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Add some cards to library so draw can succeed
    for i in 0..3 {
        let card = CardBuilder::new(CardId::new(), format!("Library Card {}", i))
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }

    let source_id = create_creature(&mut game, "Source", alice, 2, 2);
    let initial_hand_size = game.player(alice).unwrap().hand.len();

    let effect = Effect::may_single(Effect::draw(1));

    // Test 1: May effect with decision maker that accepts
    let mut accept_dm = AcceptMayDecisionMaker;
    let mut ctx =
        ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut accept_dm);

    let result = execute_effect(&mut game, &effect, &mut ctx).unwrap();

    // Effect should have been executed (not declined)
    assert!(
        !matches!(result.status, crate::effect::OutcomeStatus::Declined),
        "Effect should not be declined when decision maker accepts"
    );
    assert_eq!(
        game.player(alice).unwrap().hand.len(),
        initial_hand_size + 1,
        "Should have drawn a card"
    );

    // Test 2: May effect with decision maker that declines
    let mut decline_dm = DeclineMayDecisionMaker;
    let mut ctx2 =
        ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut decline_dm);

    let result2 = execute_effect(&mut game, &effect, &mut ctx2).unwrap();

    // Effect should have been declined
    assert!(
        matches!(result2.status, crate::effect::OutcomeStatus::Declined),
        "Effect should be declined when decision maker declines"
    );
    assert_eq!(
        game.player(alice).unwrap().hand.len(),
        initial_hand_size + 1,
        "Should NOT have drawn another card"
    );

    // Test 3: May effect with AutoPassDecisionMaker (auto-decline)
    let mut autopass_dm = AutoPassDecisionMaker;
    let mut ctx3 =
        ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut autopass_dm);
    let result3 = execute_effect(&mut game, &effect, &mut ctx3).unwrap();

    assert!(
        matches!(result3.status, crate::effect::OutcomeStatus::Declined),
        "Effect should be auto-declined with AutoPassDecisionMaker"
    );
}

#[test]
pub(super) fn test_undying_trigger_generation() {
    use crate::ability::TriggeredAbility;
    use crate::events::zones::ZoneChangeEvent;
    use crate::triggers::{TriggerEvent, check_triggers};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create a creature with Undying (now a triggered ability)
    let creature_id = create_creature(&mut game, "Undying Creature", alice, 2, 2);
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::undying(),
                effects: undying_effects().into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    // Create a snapshot of the creature (no +1/+1 counters)
    let snapshot =
        crate::snapshot::ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

    // Verify the snapshot qualifies for undying
    assert!(
        snapshot.qualifies_for_undying(),
        "Creature with Undying and no +1/+1 counters should qualify for undying"
    );

    // Simulate death event
    let event = TriggerEvent::new_with_provenance(
        ZoneChangeEvent::with_cause(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_sba(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    // Check triggers - should generate an undying trigger
    let triggers = check_triggers(&game, &event);

    assert!(
        triggers
            .iter()
            .any(|t| { t.ability.trigger == Trigger::undying() }),
        "Should generate an undying trigger"
    );
}

#[test]
pub(super) fn test_undying_does_not_trigger_with_plus_counters() {
    use crate::ability::TriggeredAbility;
    use crate::events::zones::ZoneChangeEvent;
    use crate::object::CounterType;
    use crate::triggers::{TriggerEvent, check_triggers};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create a creature with Undying AND +1/+1 counters
    let creature_id = create_creature(&mut game, "Undying Creature", alice, 2, 2);
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::undying(),
                effects: undying_effects().into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        });
    game.object_mut(creature_id)
        .unwrap()
        .add_counters(CounterType::PlusOnePlusOne, 1);

    // Create a snapshot
    let snapshot =
        crate::snapshot::ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

    // Verify the snapshot does NOT qualify for undying
    assert!(
        !snapshot.qualifies_for_undying(),
        "Creature with +1/+1 counters should NOT qualify for undying"
    );

    // Simulate death event
    let event = TriggerEvent::new_with_provenance(
        ZoneChangeEvent::with_cause(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_sba(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    // Check triggers - should NOT generate an undying trigger
    let triggers = check_triggers(&game, &event);

    assert!(
        !triggers
            .iter()
            .any(|t| { t.ability.trigger == Trigger::undying() }),
        "Should NOT generate an undying trigger when creature has +1/+1 counters"
    );
}

#[test]
pub(super) fn test_persist_trigger_generation() {
    use crate::ability::TriggeredAbility;
    use crate::events::zones::ZoneChangeEvent;
    use crate::triggers::{TriggerEvent, check_triggers};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create a creature with Persist (now a triggered ability)
    let creature_id = create_creature(&mut game, "Persist Creature", alice, 2, 2);
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::persist(),
                effects: persist_effects().into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    // Create a snapshot (no -1/-1 counters)
    let snapshot =
        crate::snapshot::ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

    // Verify the snapshot qualifies for persist
    assert!(
        snapshot.qualifies_for_persist(),
        "Creature with Persist and no -1/-1 counters should qualify for persist"
    );

    // Simulate death event
    let event = TriggerEvent::new_with_provenance(
        ZoneChangeEvent::with_cause(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_sba(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    // Check triggers - should generate a persist trigger
    let triggers = check_triggers(&game, &event);

    assert!(
        triggers
            .iter()
            .any(|t| { t.ability.trigger == Trigger::persist() }),
        "Should generate a persist trigger"
    );
}

#[test]
pub(super) fn test_return_from_graveyard_with_counter_effect() {
    use crate::effects::ExecutionContext;
    use crate::events::zones::ZoneChangeEvent;
    use crate::snapshot::ObjectSnapshot;
    use crate::triggers::TriggerEvent;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create a creature and put it in the graveyard
    let creature_id = create_creature(&mut game, "Dead Creature", alice, 2, 2);

    // Take snapshot BEFORE moving (captures stable_id)
    let snapshot = ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

    game.move_object_by_effect(creature_id, Zone::Graveyard);

    // The creature now has a new ID in the graveyard
    let graveyard_id = game.player(alice).unwrap().graveyard[0];

    // Create triggering event with the snapshot
    let trigger_event = TriggerEvent::new_with_provenance(
        ZoneChangeEvent::with_cause(
            creature_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_sba(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut ctx = ExecutionContext::new_default(graveyard_id, alice);
    ctx.triggering_event = Some(trigger_event);
    for effect in undying_effects() {
        execute_effect(&mut game, &effect, &mut ctx).unwrap();
    }

    // Verify the creature is now on the battlefield
    assert_eq!(
        game.battlefield.len(),
        1,
        "Should have one creature on battlefield"
    );

    // Verify graveyard is empty
    assert_eq!(
        game.player(alice).unwrap().graveyard.len(),
        0,
        "Graveyard should be empty"
    );

    // Verify the creature has a +1/+1 counter
    let returned_id = game.battlefield[0];
    let creature = game.object(returned_id).unwrap();
    assert_eq!(
        creature.counters.get(&CounterType::PlusOnePlusOne),
        Some(&1),
        "Creature should have one +1/+1 counter"
    );
}

#[test]
pub(super) fn test_once_per_turn_in_legal_actions() {
    use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up for main phase with priority
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create a creature with a OncePerTurn activated ability
    let creature_id = create_creature(&mut game, "Test Creature", alice, 2, 2);
    game.object_mut(creature_id)
        .unwrap()
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(), // Free ability for testing
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                timing: ActivationTiming::OncePerTurn,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });
    game.remove_summoning_sickness(creature_id);

    // Get legal actions - ability should be available
    let actions1 = compute_legal_actions(&game, alice);
    let can_activate1 = actions1.iter().any(|a| {
        matches!(
            a,
            LegalAction::ActivateAbility { source, .. } if *source == creature_id
        )
    });
    assert!(
        can_activate1,
        "OncePerTurn ability should be available initially"
    );

    // Simulate activating the ability
    game.record_ability_activation(creature_id, 0);

    // Get legal actions again - ability should NOT be available
    let actions2 = compute_legal_actions(&game, alice);
    let can_activate2 = actions2.iter().any(|a| {
        matches!(
            a,
            LegalAction::ActivateAbility { source, ability_index }
                if *source == creature_id && *ability_index == 0
        )
    });
    assert!(
        !can_activate2,
        "OncePerTurn ability should NOT be available after activation"
    );
}

#[test]
pub(super) fn test_loyalty_activation_is_tracked_per_permanent_without_text_cap() {
    use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let chandra = CardBuilder::new(CardId::from_raw(72_980), "Chandra Test")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(6)
        .build();
    let chandra_id = game.create_object_from_card(&chandra, alice, Zone::Battlefield);

    game.object_mut(chandra_id)
        .expect("Chandra should exist")
        .abilities_mut()
        .extend([
            Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: TotalCost::from_cost(Cost::add_counters(CounterType::Loyalty, 1)),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::draw(1),
                    ]),
                    choices: vec![],
                    timing: ActivationTiming::SorcerySpeed,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                    is_loyalty_ability: true,
                }),
                functional_zones: vec![Zone::Battlefield],
            },
            Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: TotalCost::from_cost(Cost::remove_counters(CounterType::Loyalty, 3)),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::draw(1),
                    ]),
                    choices: vec![],
                    timing: ActivationTiming::SorcerySpeed,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                    is_loyalty_ability: true,
                }),
                functional_zones: vec![Zone::Battlefield],
            },
        ]);

    let actions_before = compute_legal_actions(&game, alice);
    assert!(
        actions_before.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility {
                source,
                ability_index
            } if *source == chandra_id && *ability_index == 0
        )),
        "the +1 loyalty ability should be legal before any loyalty activation"
    );
    assert!(
        actions_before.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility {
                source,
                ability_index
            } if *source == chandra_id && *ability_index == 1
        )),
        "a different loyalty ability should also be legal before any loyalty activation"
    );

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source: chandra_id,
            ability_index: 0,
        }),
        &mut dm,
    )
    .expect("loyalty activation should complete");

    assert!(
        game.loyalty_ability_activated_this_turn(chandra_id),
        "activating a loyalty ability should record the permanent immediately"
    );
    assert_eq!(
        game.counter_count(chandra_id, CounterType::Loyalty),
        7,
        "the +1 loyalty cost should still be paid"
    );

    game.stack.clear();
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let actions_after = compute_legal_actions(&game, alice);
    assert!(
        !actions_after.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source, .. } if *source == chandra_id
        )),
        "no loyalty ability of that permanent should be legal after one was activated this turn"
    );
}

#[test]
pub(super) fn test_negative_loyalty_cost_requires_enough_loyalty_in_legal_actions() {
    use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let planeswalker = CardBuilder::new(CardId::from_raw(72_981), "Low Loyalty Walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(2)
        .build();
    let planeswalker_id = game.create_object_from_card(&planeswalker, alice, Zone::Battlefield);

    game.object_mut(planeswalker_id)
        .expect("planeswalker should exist")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::from_cost(Cost::remove_counters(CounterType::Loyalty, 3)),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: true,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility {
                source,
                ability_index
            } if *source == planeswalker_id && *ability_index == 0
        )),
        "a -3 loyalty ability should not be legal with only two loyalty counters"
    );
}

#[test]
pub(super) fn elvish_refueler_exhaust_permission_allows_one_used_exhaust_on_your_turn() {
    use crate::ability::{ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let refueler_id = create_creature(&mut game, "Elvish Refueler", alice, 2, 3);
    game.object_mut(refueler_id)
        .expect("Elvish Refueler should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::exhaust_abilities_as_though_unactivated_this_turn(),
        ));
    game.object_mut(refueler_id)
        .expect("Elvish Refueler should exist")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(1)],
                    vec![ManaSymbol::Green],
                ])),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::put_counters_on_source(crate::object::CounterType::PlusOnePlusOne, 1),
                ]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![
                    "Activate each exhaust ability only once.".to_string(),
                ],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let ability_index = 1;
    let ability = match &game
        .object(refueler_id)
        .expect("Elvish Refueler should exist")
        .abilities[ability_index]
        .kind
    {
        AbilityKind::Activated(activated) => activated.clone(),
        _ => panic!("expected Elvish Refueler exhaust activated ability"),
    };

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    assert!(
        can_activate_ability_with_restrictions(&game, refueler_id, ability_index, &ability),
        "Elvish Refueler's unused exhaust ability should be activatable"
    );
    game.record_ability_activation(refueler_id, ability_index);
    assert!(
        !can_activate_ability_with_restrictions(&game, refueler_id, ability_index, &ability),
        "Elvish Refueler should not let the same exhaust ability be activated twice in one turn"
    );

    game.next_turn();
    game.turn.active_player = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    assert!(
        !can_activate_ability_with_restrictions(&game, refueler_id, ability_index, &ability),
        "Elvish Refueler's static permission should not refresh exhaust abilities outside its controller's turn"
    );

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    assert!(
        can_activate_ability_with_restrictions(&game, refueler_id, ability_index, &ability),
        "Elvish Refueler should refresh one previously activated exhaust ability during its controller's turn before any exhaust activation that turn"
    );
    game.record_ability_activation(refueler_id, ability_index);
    assert!(
        !can_activate_ability_with_restrictions(&game, refueler_id, ability_index, &ability),
        "after activating an exhaust ability this turn, Elvish Refueler's condition should stop refreshing it"
    );
}

#[test]
pub(super) fn test_nonactive_player_keeps_priority_after_activating_ability() {
    use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);

    let creature_id = create_creature(&mut game, "Quick Test Creature", alice, 2, 2);
    game.object_mut(creature_id)
        .expect("test creature should exist")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });

    let actions = compute_legal_actions(&game, alice);
    let activate_action = actions
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == creature_id && *ability_index == 0
            )
        })
        .expect("alice should be able to activate the ability on bob's turn");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("activation should succeed");

    assert_eq!(
        game.turn.priority_player,
        Some(alice),
        "the activating player should keep priority after activating a non-mana ability"
    );
    assert_eq!(
        game.turn.active_player, bob,
        "it should still be bob's turn"
    );
    assert_eq!(
        game.stack.len(),
        1,
        "the activated ability should be on the stack"
    );
    assert!(game.stack[0].is_ability, "stack entry should be an ability");
    assert_eq!(
        game.stack[0].controller, alice,
        "the activated ability should be controlled by the activating player"
    );
}

#[test]
pub(super) fn test_once_per_turn_restriction_survives_control_change() {
    use crate::ability::{AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let creature_id = create_creature(&mut game, "Control Change Test", alice, 2, 2);
    game.object_mut(creature_id)
        .expect("test creature should exist")
        .abilities_mut()
        .push(Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::free(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                timing: ActivationTiming::OncePerTurn,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        });
    game.remove_summoning_sickness(creature_id);

    game.record_ability_activation(creature_id, 0);
    game.set_current_controller(creature_id, bob);

    game.turn.priority_player = Some(bob);

    let same_turn_actions = compute_legal_actions(&game, bob);
    assert!(
        !same_turn_actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == creature_id && *ability_index == 0
            )
        }),
        "the once-per-turn restriction should still apply after the permanent changes controllers"
    );

    game.next_turn();
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(bob);

    let next_turn_actions = compute_legal_actions(&game, bob);
    assert!(
        next_turn_actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == creature_id && *ability_index == 0
            )
        }),
        "the restriction should reset on the next turn for the current controller"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_wall_of_roots_once_per_turn_mana_ability_fast_path() {
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let wall_def = crate::cards::definitions::wall_of_roots();
    let wall_id = game.create_object_from_definition(&wall_def, alice, Zone::Battlefield);

    let ability_index = game
        .object(wall_id)
        .expect("wall of roots exists")
        .abilities
        .iter()
        .position(|ability| ability.is_mana_ability())
        .expect("wall of roots should have a mana ability");

    let actions_before = compute_legal_actions(&game, alice);
    assert!(actions_before.iter().any(|a| {
        matches!(
            a,
            LegalAction::ActivateManaAbility {
                source,
                ability_index: idx
            } if *source == wall_id && *idx == ability_index
        )
    }));

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let response = PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
        source: wall_id,
        ability_index,
    });

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &response,
        &mut decision_maker,
    )
    .expect("wall of roots mana ability should activate");

    assert_eq!(
        game.ability_activation_count_this_turn(wall_id, ability_index),
        1,
        "wall of roots activation should be recorded for this turn"
    );

    let actions_after = compute_legal_actions(&game, alice);
    assert!(!actions_after.iter().any(|a| {
        matches!(
            a,
            LegalAction::ActivateManaAbility {
                source,
                ability_index: idx
            } if *source == wall_id && *idx == ability_index
        )
    }));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_bosh_iron_golem_uses_sacrificed_artifact_mana_value_for_damage() {
    use crate::decision::LegalAction;
    use crate::mana::{ManaCost, ManaSymbol};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let registry = crate::cards::CardRegistry::with_builtin_cards_for_names(["Bosh, Iron Golem"]);
    let bosh_def = registry
        .get("Bosh, Iron Golem")
        .expect("Bosh, Iron Golem should be present in registry");

    let bosh_id = game.create_object_from_definition(bosh_def, alice, Zone::Battlefield);
    let sacrificial_artifact = CardBuilder::new(CardId::new(), "Calibration Relic")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .build();
    let relic_id = game.create_object_from_card(&sacrificial_artifact, alice, Zone::Battlefield);

    if let Some(player) = game.player_mut(alice) {
        player.mana_pool.add(ManaSymbol::Red, 4);
    }

    let ability_index = game
        .object(bosh_id)
        .expect("Bosh should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Bosh should have an activated ability");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;

    let activate = PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
        source: bosh_id,
        ability_index,
    });
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &activate,
        &mut dm,
    )
    .expect("activation should start");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected Bosh to prompt for targets first, got {:?}", other),
    }

    let choose_target = PriorityResponse::Targets(vec![Target::Player(bob)]);
    let cost_order_ctx = match apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &choose_target,
        &mut dm,
    )
    .expect("should choose damage target")
    {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!(
            "expected Bosh next-cost chooser after choosing target, got {:?}",
            other
        ),
    };

    let sacrifice_cost_index = cost_order_ctx
        .options
        .iter()
        .find(|opt| opt.description.to_ascii_lowercase().contains("sacrifice"))
        .map(|opt| opt.index)
        .expect("expected a sacrifice cost option");
    let choose_sacrifice_cost = PriorityResponse::NextCostChoice(sacrifice_cost_index);
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &choose_sacrifice_cost,
        &mut dm,
    )
    .expect("should choose sacrifice cost first");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => {}
        other => panic!(
            "expected sacrifice target prompt after choosing Bosh sacrifice cost, got {:?}",
            other
        ),
    }

    let choose_sacrifice = PriorityResponse::SacrificeTarget(relic_id);
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &choose_sacrifice,
        &mut dm,
    )
    .expect("should choose sacrifice target");

    assert_eq!(game.stack.len(), 1, "Bosh ability should be on stack");
    let bosh_entry = game.stack.last().expect("Bosh ability should be on stack");
    let sacrificed = bosh_entry
        .tagged_objects
        .get(&crate::tag::TagKey::from("sacrifice_cost_0"))
        .expect("Bosh stack entry should keep the sacrificed-artifact tag");
    assert_eq!(sacrificed.len(), 1);
    assert_eq!(sacrificed[0].name, "Calibration Relic");

    resolve_stack_entry(&mut game).expect("Bosh ability should resolve");

    assert_eq!(
        game.player(bob).expect("Bob exists").life,
        18,
        "Bosh should deal damage equal to the sacrificed artifact's mana value (2)"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn run_lyzolda_the_blood_witch_sacrifice_branch(
    colors: crate::color::ColorSet,
    expect_damage: bool,
    expect_draw: bool,
) {
    use crate::decision::LegalAction;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let lyzolda_def =
        CardDefinitionBuilder::new(CardId::from_raw(571_572), "Lyzolda, the Blood Witch")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Red],
            ]))
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human, Subtype::Cleric])
            .power_toughness(PowerToughness::fixed(3, 1))
            .parse_text(
                "{2}, Sacrifice a creature: Lyzolda deals 2 damage to any target if the sacrificed creature was red. Draw a card if the sacrificed creature was black.",
            )
            .expect("Lyzolda, the Blood Witch should parse for runtime tests");

    let lyzolda_id = game.create_object_from_definition(&lyzolda_def, alice, Zone::Battlefield);
    let fodder_card = CardBuilder::new(CardId::new(), "Lyzolda Fodder")
        .card_types(vec![CardType::Creature])
        .color_indicator(colors)
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let fodder_id = game.create_object_from_card(&fodder_card, alice, Zone::Battlefield);
    let draw_card = CardBuilder::new(CardId::new(), "Lyzolda Draw Card")
        .card_types(vec![CardType::Creature])
        .build();
    game.create_object_from_card(&draw_card, alice, Zone::Library);

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let ability_index = game
        .object(lyzolda_id)
        .expect("Lyzolda should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Lyzolda should have an activated ability");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source: lyzolda_id,
            ability_index,
        }),
        &mut dm,
    )
    .expect("Lyzolda activation should start");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected Lyzolda to ask for an any-target target first, got {other:?}"),
    }

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob)]),
        &mut dm,
    )
    .expect("Lyzolda should accept Bob as any target");

    let cost_ctx = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!("expected Lyzolda to ask for cost order after targeting, got {other:?}"),
    };
    let sacrifice_cost_index = cost_ctx
        .options
        .iter()
        .find(|option| {
            option
                .description
                .to_ascii_lowercase()
                .contains("sacrifice")
        })
        .map(|option| option.index)
        .expect("expected a sacrifice cost option for Lyzolda");

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::NextCostChoice(sacrifice_cost_index),
        &mut dm,
    )
    .expect("Lyzolda should accept choosing the sacrifice cost first");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => {}
        other => panic!("expected Lyzolda to ask which creature to sacrifice, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::SacrificeTarget(fodder_id),
        &mut dm,
    )
    .expect("Lyzolda should accept the sacrificed creature");

    assert_eq!(
        game.stack.len(),
        1,
        "Lyzolda ability should be on the stack"
    );
    let stack_entry = game
        .stack
        .last()
        .expect("Lyzolda ability should be stacked");
    let sacrificed = stack_entry
        .tagged_objects
        .get(&crate::tag::TagKey::from("sacrifice_cost_0"))
        .expect("Lyzolda stack entry should remember the sacrificed creature");
    assert_eq!(sacrificed.len(), 1);
    assert_eq!(sacrificed[0].name, "Lyzolda Fodder");

    let hand_before = game.player(alice).expect("Alice exists").hand.len();
    resolve_stack_entry(&mut game).expect("Lyzolda ability should resolve");

    let expected_bob_life = if expect_damage { 18 } else { 20 };
    assert_eq!(
        game.player(bob).expect("Bob exists").life,
        expected_bob_life,
        "red sacrificed creatures should be the only ones that make Lyzolda deal damage"
    );
    let hand_after = game.player(alice).expect("Alice exists").hand.len();
    assert_eq!(
        hand_after,
        hand_before + usize::from(expect_draw),
        "black sacrificed creatures should be the only ones that make Lyzolda draw"
    );
    let drew_fixture_card = game
        .player(alice)
        .expect("Alice exists")
        .hand
        .iter()
        .filter_map(|&id| game.object(id))
        .any(|object| object.name == "Lyzolda Draw Card");
    assert_eq!(
        drew_fixture_card, expect_draw,
        "Lyzolda should move a library card to hand exactly when the black branch is true"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_lyzolda_the_blood_witch_red_sacrifice_deals_damage_only() {
    run_lyzolda_the_blood_witch_sacrifice_branch(crate::color::ColorSet::RED, true, false);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_lyzolda_the_blood_witch_black_sacrifice_draws_only() {
    run_lyzolda_the_blood_witch_sacrifice_branch(crate::color::ColorSet::BLACK, false, true);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_lyzolda_the_blood_witch_black_red_sacrifice_damages_and_draws() {
    run_lyzolda_the_blood_witch_sacrifice_branch(
        crate::color::ColorSet::BLACK.union(crate::color::ColorSet::RED),
        true,
        true,
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_lyzolda_the_blood_witch_non_red_non_black_sacrifice_does_neither_branch() {
    run_lyzolda_the_blood_witch_sacrifice_branch(crate::color::ColorSet::GREEN, false, false);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn maestros_ascendancy_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(92_180), "Maestros Ascendancy")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Once during each of your turns, you may cast an instant or sorcery spell from your graveyard by sacrificing a creature in addition to paying its other costs. If a spell cast this way would be put into your graveyard, exile it instead.",
        )
        .expect("Maestros Ascendancy should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn zero_mana_spell_card(name: &str, card_type: CardType) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .mana_cost(ManaCost::new())
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn maestros_graveyard_cast_action(
    game: &GameState,
    player: PlayerId,
    source_id: ObjectId,
    spell_id: ObjectId,
) -> Option<LegalAction> {
    compute_legal_actions(game, player)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: action_spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::PlayFrom {
                        source,
                        zone: Zone::Graveyard,
                        use_alternative: Some(_),
                    },
                } if *action_spell_id == spell_id && *source == source_id
            )
        })
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn finish_maestros_cast_with_sacrifice(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut progress: GameProgress,
    sacrifice_id: ObjectId,
) {
    for _ in 0..6 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let option_index = ctx
                    .options
                    .iter()
                    .find(|option| {
                        option
                            .description
                            .to_ascii_lowercase()
                            .contains("sacrifice")
                    })
                    .map(|option| option.index)
                    .expect("Maestros cast should prompt for the sacrifice cost step");
                apply_priority_response(
                    game,
                    trigger_queue,
                    state,
                    &PriorityResponse::NextCostChoice(option_index),
                )
                .expect("Maestros cast should accept sacrifice cost step")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ) => {
                assert!(
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.id == sacrifice_id && candidate.legal),
                    "Maestros cast should allow sacrificing a creature you control"
                );
                apply_priority_response(
                    game,
                    trigger_queue,
                    state,
                    &PriorityResponse::CardCostChoice(sacrifice_id),
                )
                .expect("Maestros cast should accept the sacrificed creature")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => return,
            other => panic!("unexpected Maestros cast flow state: {other:?}"),
        };
    }
    panic!("Maestros cast did not finish paying costs");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn maestros_ascendancy_casts_graveyard_spell_by_sacrificing_creature_and_exiles_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source_id = game.create_object_from_definition(
        &maestros_ascendancy_definition(),
        alice,
        Zone::Battlefield,
    );
    let buried_spell = zero_mana_spell_card("Buried Scheme", CardType::Sorcery);
    let spell_id = game.create_object_from_card(&buried_spell, alice, Zone::Graveyard);
    let fodder = CardBuilder::new(CardId::new(), "Maestros Fodder")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let fodder_id = game.create_object_from_card(&fodder, alice, Zone::Battlefield);

    let action = maestros_graveyard_cast_action(&game, alice, source_id, spell_id)
        .expect("Maestros Ascendancy should offer the graveyard sorcery cast");
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
    )
    .expect("Maestros graveyard cast should start");
    finish_maestros_cast_with_sacrifice(
        &mut game,
        &mut trigger_queue,
        &mut state,
        progress,
        fodder_id,
    );

    assert!(
        game.player(alice)
            .expect("Alice exists")
            .graveyard
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|obj| obj.name == "Maestros Fodder")),
        "the sacrificed creature should be put into its owner's graveyard as a cost"
    );
    assert!(
        game.stack.iter().any(|entry| game
            .object(entry.object_id)
            .is_some_and(|obj| obj.name == "Buried Scheme")),
        "the graveyard spell should be on the stack after paying Maestros Ascendancy's cost"
    );

    resolve_stack_entry(&mut game).expect("Maestros-cast spell should resolve");
    assert!(
        game.exile.iter().any(|id| game
            .object(*id)
            .is_some_and(|obj| obj.name == "Buried Scheme")),
        "a spell cast with Maestros Ascendancy should be exiled instead of returning to the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn maestros_ascendancy_requires_creature_cost_and_instant_or_sorcery_card() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source_id = game.create_object_from_definition(
        &maestros_ascendancy_definition(),
        alice,
        Zone::Battlefield,
    );
    let buried_instant = zero_mana_spell_card("Buried Instant", CardType::Instant);
    let instant_id = game.create_object_from_card(&buried_instant, alice, Zone::Graveyard);
    assert!(
        maestros_graveyard_cast_action(&game, alice, source_id, instant_id).is_none(),
        "Maestros Ascendancy should not offer the cast when you cannot sacrifice a creature"
    );

    let fodder = CardBuilder::new(CardId::new(), "Spare Informant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&fodder, alice, Zone::Battlefield);
    let artifact = zero_mana_spell_card("Buried Artifact", CardType::Artifact);
    let artifact_id = game.create_object_from_card(&artifact, alice, Zone::Graveyard);
    assert!(
        maestros_graveyard_cast_action(&game, alice, source_id, artifact_id).is_none(),
        "Maestros Ascendancy should not grant the alternative cast to non-instant non-sorcery cards"
    );
    assert!(
        maestros_graveyard_cast_action(&game, alice, source_id, instant_id).is_some(),
        "after a creature is available, Maestros Ascendancy should offer an instant from the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn maestros_ascendancy_is_once_each_turn_and_only_on_your_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source_id = game.create_object_from_definition(
        &maestros_ascendancy_definition(),
        alice,
        Zone::Battlefield,
    );
    let first_spell = zero_mana_spell_card("First Buried Spell", CardType::Sorcery);
    let first_spell_id = game.create_object_from_card(&first_spell, alice, Zone::Graveyard);
    let second_spell = zero_mana_spell_card("Second Buried Spell", CardType::Sorcery);
    let second_spell_id = game.create_object_from_card(&second_spell, alice, Zone::Graveyard);
    let first_fodder = CardBuilder::new(CardId::new(), "First Fodder")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let first_fodder_id = game.create_object_from_card(&first_fodder, alice, Zone::Battlefield);
    let second_fodder = CardBuilder::new(CardId::new(), "Second Fodder")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&second_fodder, alice, Zone::Battlefield);

    let action = maestros_graveyard_cast_action(&game, alice, source_id, first_spell_id)
        .expect("first Maestros cast should be available");
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
    )
    .expect("first Maestros cast should start");
    finish_maestros_cast_with_sacrifice(
        &mut game,
        &mut trigger_queue,
        &mut state,
        progress,
        first_fodder_id,
    );
    resolve_stack_entry(&mut game).expect("first Maestros spell should resolve");

    assert!(
        maestros_graveyard_cast_action(&game, alice, source_id, second_spell_id).is_none(),
        "Maestros Ascendancy should not offer a second graveyard cast from the same source in one turn"
    );

    let mut opponent_turn_game = setup_game();
    opponent_turn_game.turn.phase = Phase::FirstMain;
    opponent_turn_game.turn.step = None;
    opponent_turn_game.turn.active_player = bob;
    opponent_turn_game.turn.priority_player = Some(alice);
    let opponent_turn_source = opponent_turn_game.create_object_from_definition(
        &maestros_ascendancy_definition(),
        alice,
        Zone::Battlefield,
    );
    let flash_spell = CardDefinitionBuilder::new(CardId::new(), "Grave Flash Probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::new())
        .parse_text("Flash")
        .expect("flash probe should parse");
    let flash_spell_id =
        opponent_turn_game.create_object_from_definition(&flash_spell, alice, Zone::Graveyard);
    let fodder = CardBuilder::new(CardId::new(), "Opponent Turn Fodder")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    opponent_turn_game.create_object_from_card(&fodder, alice, Zone::Battlefield);
    assert!(
        maestros_graveyard_cast_action(
            &opponent_turn_game,
            alice,
            opponent_turn_source,
            flash_spell_id,
        )
        .is_none(),
        "Maestros Ascendancy's once-during-your-turn permission should not function on another player's turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn demilich_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(92_190), "Demilich")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Skeleton, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(4, 3))
        .parse_text(
            "This spell costs {U} less to cast for each instant and sorcery spell you've cast this turn.\n\
             Whenever this creature attacks, exile up to one target instant or sorcery card from your graveyard. Copy it. You may cast the copy.\n\
             You may cast this card from your graveyard by exiling four instant and/or sorcery cards from your graveyard in addition to paying its other costs.",
        )
        .expect("Demilich should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn demilich_graveyard_cast_action(
    game: &GameState,
    player: PlayerId,
    spell_id: ObjectId,
) -> Option<LegalAction> {
    compute_legal_actions(game, player)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: action_spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::PlayFrom {
                        source,
                        zone: Zone::Graveyard,
                        use_alternative: Some(_),
                    },
                } if *action_spell_id == spell_id && *source == spell_id
            )
        })
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn graveyard_cost_card(name: &str, card_type: CardType) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .mana_cost(ManaCost::new())
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn record_demilich_test_spell_cast(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    card_type: CardType,
) {
    let spell_id =
        game.create_object_from_card(&graveyard_cost_card(name, card_type), player, Zone::Stack);
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, player, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn demilich_cost_reduction_removes_one_blue_for_each_instant_or_sorcery_cast() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let demilich_id = game.create_object_from_definition(&demilich_definition(), alice, Zone::Hand);
    let demilich = game.object(demilich_id).expect("Demilich should exist");
    let base_cost = demilich
        .mana_cost
        .as_ref()
        .expect("Demilich has mana cost")
        .clone();
    assert_eq!(base_cost.to_oracle(), "{U}{U}{U}{U}");

    record_demilich_test_spell_cast(&mut game, alice, "Cast Instant", CardType::Instant);
    let demilich = game.object(demilich_id).expect("Demilich should exist");
    let reduced_once =
        crate::decision::calculate_effective_mana_cost(&game, alice, demilich, &base_cost);
    assert_eq!(reduced_once.to_oracle(), "{U}{U}{U}");

    record_demilich_test_spell_cast(&mut game, alice, "Cast Sorcery", CardType::Sorcery);
    record_demilich_test_spell_cast(&mut game, alice, "Cast Creature", CardType::Creature);
    let demilich = game.object(demilich_id).expect("Demilich should exist");
    let reduced_twice =
        crate::decision::calculate_effective_mana_cost(&game, alice, demilich, &base_cost);
    assert_eq!(reduced_twice.to_oracle(), "{U}{U}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn demilich_casts_from_graveyard_by_exiling_four_instant_or_sorcery_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let demilich_id =
        game.create_object_from_definition(&demilich_definition(), alice, Zone::Graveyard);
    for idx in 0..4 {
        record_demilich_test_spell_cast(
            &mut game,
            alice,
            &format!("Previously Cast Instant {idx}"),
            CardType::Instant,
        );
    }
    let valid_cards = [
        game.create_object_from_card(
            &graveyard_cost_card("First Buried Instant", CardType::Instant),
            alice,
            Zone::Graveyard,
        ),
        game.create_object_from_card(
            &graveyard_cost_card("Second Buried Instant", CardType::Instant),
            alice,
            Zone::Graveyard,
        ),
        game.create_object_from_card(
            &graveyard_cost_card("Third Buried Instant", CardType::Instant),
            alice,
            Zone::Graveyard,
        ),
        game.create_object_from_card(
            &graveyard_cost_card("Buried Sorcery", CardType::Sorcery),
            alice,
            Zone::Graveyard,
        ),
    ];
    let artifact_id = game.create_object_from_card(
        &graveyard_cost_card("Buried Artifact", CardType::Artifact),
        alice,
        Zone::Graveyard,
    );

    let action = demilich_graveyard_cast_action(&game, alice, demilich_id).expect(
        "Demilich should be castable from graveyard with four instant/sorcery cards to exile",
    );
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .expect("Demilich graveyard cast should start");

    for _ in 0..8 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let option_index = ctx
                    .options
                    .iter()
                    .find(|option| option.description.to_ascii_lowercase().contains("exile"))
                    .map(|option| option.index)
                    .expect("Demilich cast should offer an exile cost step");
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(option_index),
                    &mut dm,
                )
                .expect("Demilich cast should accept the exile cost step")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ) => {
                assert!(
                    ctx.candidates
                        .iter()
                        .all(|candidate| candidate.id != artifact_id || !candidate.legal),
                    "Demilich exile cost must not allow artifact cards"
                );
                let card_id = ctx
                    .candidates
                    .iter()
                    .find(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .expect("Demilich exile cost should have an instant or sorcery choice");
                assert!(
                    valid_cards.contains(&card_id),
                    "Demilich exile cost should choose only the prepared instant/sorcery cards"
                );
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::CardCostChoice(card_id),
                    &mut dm,
                )
                .expect("Demilich cast should accept an instant or sorcery exile choice")
            }
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => break,
            other => panic!("expected Demilich exile cost flow, got {other:?}"),
        };
    }

    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Priority(_))
        ),
        "Demilich cast should finish after four exile choices, got {progress:?}"
    );
    assert!(
        game.stack.iter().any(|entry| game
            .object(entry.object_id)
            .is_some_and(|object| object.name == "Demilich")),
        "Demilich should be on the stack after paying its graveyard-cast cost"
    );
    for name in [
        "First Buried Instant",
        "Second Buried Instant",
        "Third Buried Instant",
        "Buried Sorcery",
    ] {
        assert!(
            game.exile
                .iter()
                .any(|id| game.object(*id).is_some_and(|object| object.name == name)),
            "{name} should be exiled to pay Demilich's graveyard-cast cost"
        );
    }
    assert!(
        game.player(alice)
            .expect("Alice exists")
            .graveyard
            .contains(&artifact_id),
        "the artifact card should remain in the graveyard because it is not a legal Demilich cost card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn demilich_graveyard_cast_requires_four_instant_or_sorcery_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let demilich_id =
        game.create_object_from_definition(&demilich_definition(), alice, Zone::Graveyard);
    for idx in 0..4 {
        record_demilich_test_spell_cast(
            &mut game,
            alice,
            &format!("Previously Cast Instant {idx}"),
            CardType::Instant,
        );
    }
    for idx in 0..3 {
        game.create_object_from_card(
            &graveyard_cost_card(&format!("Buried Instant {idx}"), CardType::Instant),
            alice,
            Zone::Graveyard,
        );
    }
    game.create_object_from_card(
        &graveyard_cost_card("Buried Artifact", CardType::Artifact),
        alice,
        Zone::Graveyard,
    );

    assert!(
        demilich_graveyard_cast_action(&game, alice, demilich_id).is_none(),
        "Demilich should not be castable from graveyard with only three instant/sorcery cards plus an artifact"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn demon_of_fates_design_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(92_200), "Demon of Fate's Design")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Demon])
        .power_toughness(PowerToughness::fixed(6, 6))
        .parse_text(
            "Flying, trample\n\
             Once during each of your turns, you may cast an enchantment spell by paying life equal to its mana value rather than paying its mana cost.\n\
             {2}{B}, Sacrifice another enchantment: This creature gets +X/+0 until end of turn, where X is the sacrificed enchantment's mana value.",
        )
        .expect("Demon of Fate's Design should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn demon_of_fates_design_life_cost_casts_only_enchantments_once_during_your_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let demon_id = game.create_object_from_definition(
        &demon_of_fates_design_definition(),
        alice,
        Zone::Battlefield,
    );
    let enchantment = CardBuilder::new(CardId::from_raw(92_201), "Fate Test Enchantment")
        .card_types(vec![CardType::Enchantment])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let enchantment_id = game.create_object_from_card(&enchantment, alice, Zone::Hand);
    let second_enchantment = CardBuilder::new(CardId::from_raw(92_202), "Second Fate Enchantment")
        .card_types(vec![CardType::Enchantment])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .build();
    let second_enchantment_id =
        game.create_object_from_card(&second_enchantment, alice, Zone::Hand);
    let artifact = CardBuilder::new(CardId::from_raw(92_203), "Fate Test Artifact")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .build();
    let artifact_id = game.create_object_from_card(&artifact, alice, Zone::Hand);

    game.player_mut(alice).expect("Alice exists").life = 20;

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::PlayFrom {
                    source,
                    zone: Zone::Hand,
                    use_alternative: Some(_),
                    ..
                },
            } if *spell_id == enchantment_id && *source == demon_id
        )),
        "Demon of Fate's Design should offer a life-cost alternative cast for enchantments"
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                casting_method: CastingMethod::PlayFrom { use_alternative: Some(_), .. },
                ..
            } if *spell_id == artifact_id
        )),
        "Demon of Fate's Design should not grant the alternative cost to non-enchantments"
    );

    let cast_action = actions
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::PlayFrom {
                        source,
                        zone: Zone::Hand,
                        use_alternative: Some(_),
                        ..
                    },
                } if *spell_id == enchantment_id && *source == demon_id
            )
        })
        .expect("expected Demon alternative cast action");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
    )
    .expect("Demon alternative cast should succeed");

    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        17,
        "casting the three-mana-value enchantment should cost 3 life"
    );
    assert!(
        game.stack.iter().any(|entry| game
            .object(entry.object_id)
            .is_some_and(|object| object.name == "Fate Test Enchantment")),
        "the enchantment should be on the stack after paying the life alternative cost"
    );

    resolve_stack_entry(&mut game).expect("enchantment spell should resolve");
    let actions_after_use = compute_legal_actions(&game, alice);
    assert!(
        !actions_after_use.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                casting_method: CastingMethod::PlayFrom { use_alternative: Some(_), .. },
                ..
            } if *spell_id == second_enchantment_id
        )),
        "Demon of Fate's Design should not offer a second life-cost enchantment cast in the same turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn eye_of_duskmantle_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(92_210), "Eye of Duskmantle")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 8))
        .parse_text(
            "Flying, lifelink\n\
             You may play lands and cast spells from among cards in your graveyard you've surveilled this turn. If you cast a spell this way, you pay life equal to its mana value rather than paying its mana cost.",
        )
        .expect("Eye of Duskmantle should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn eye_of_duskmantle_casts_only_surveilled_graveyard_spells_for_life() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};
    use crate::events::{KeywordActionEvent, KeywordActionKind, RawEvent};
    use crate::provenance::ProvNodeId;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::{SURVEILLED_THIS_TURN_TAG, TagKey};
    use std::collections::HashMap;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice).expect("Alice exists").life = 20;

    let eye_id = game.create_object_from_definition(
        &eye_of_duskmantle_definition(),
        alice,
        Zone::Battlefield,
    );
    let spell_card = CardBuilder::new(CardId::from_raw(92_211), "Seen Graveyard Spell")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let seen_spell_id = game.create_object_from_card(&spell_card, alice, Zone::Graveyard);
    let unseen_spell_id = game.create_object_from_card(&spell_card, alice, Zone::Graveyard);
    let land_card = CardBuilder::new(CardId::from_raw(92_212), "Seen Graveyard Land")
        .card_types(vec![CardType::Land])
        .build();
    let seen_land_id = game.create_object_from_card(&land_card, alice, Zone::Graveyard);

    let mut object_tags = HashMap::new();
    object_tags.insert(
        TagKey::from(SURVEILLED_THIS_TURN_TAG),
        vec![
            ObjectSnapshot::from_object(game.object(seen_spell_id).unwrap(), &game),
            ObjectSnapshot::from_object(game.object(seen_land_id).unwrap(), &game),
        ],
    );
    let event = RawEvent::new(
        KeywordActionEvent::new(KeywordActionKind::Surveil, alice, eye_id, 2)
            .with_object_tags(object_tags),
        ProvNodeId::default(),
    );
    game.turn_store
        .turn_history
        .record_event(&event, None, None);

    let actions = compute_legal_actions(&game, alice);
    let seen_spell_casts = actions
        .iter()
        .filter(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    ..
                } if *spell_id == seen_spell_id
            )
        })
        .count();
    assert_eq!(
        seen_spell_casts, 1,
        "Eye should offer exactly one graveyard cast for a surveilled spell"
    );

    let cast_action = actions
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::PlayFrom {
                        source,
                        zone: Zone::Graveyard,
                        use_alternative: Some(_),
                    },
                } if *spell_id == seen_spell_id && *source == eye_id
            )
        })
        .expect("Eye should offer the surveilled spell with the life alternative");

    assert!(
        !compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    casting_method: CastingMethod::PlayFrom {
                        source,
                        use_alternative: None,
                        ..
                    },
                    ..
                } if *spell_id == seen_spell_id && *source == eye_id
            )),
        "Eye should not also allow the surveilled spell for its normal mana cost"
    );
    assert!(
        !compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::CastSpell { spell_id, .. } if *spell_id == unseen_spell_id
            )),
        "Eye should not allow non-surveilled graveyard spells"
    );
    assert!(
        compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::PlayLand { land_id } if *land_id == seen_land_id
            )),
        "Eye should allow surveilled graveyard lands to be played"
    );

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
    )
    .expect("Eye life-cost graveyard cast should succeed");

    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        17,
        "casting the three-mana-value spell this way should cost 3 life"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn demon_of_fates_design_life_cost_requires_enough_life() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    game.create_object_from_definition(
        &demon_of_fates_design_definition(),
        alice,
        Zone::Battlefield,
    );
    let enchantment = CardBuilder::new(CardId::from_raw(92_204), "Expensive Fate Enchantment")
        .card_types(vec![CardType::Enchantment])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .build();
    let enchantment_id = game.create_object_from_card(&enchantment, alice, Zone::Hand);
    game.player_mut(alice).expect("Alice exists").life = 2;

    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                casting_method: CastingMethod::PlayFrom { use_alternative: Some(_), .. },
                ..
            } if *spell_id == enchantment_id
        )),
        "Demon of Fate's Design should not offer the life-cost alternative if its mana value exceeds your life total"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn demon_of_fates_design_life_cost_is_only_during_your_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);

    game.create_object_from_definition(
        &demon_of_fates_design_definition(),
        alice,
        Zone::Battlefield,
    );
    let flash_enchantment =
        CardDefinitionBuilder::new(CardId::from_raw(92_206), "Flash Fate Enchantment")
            .card_types(vec![CardType::Enchantment])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .parse_text("Flash")
            .expect("flash enchantment should parse");
    let enchantment_id = game.create_object_from_definition(&flash_enchantment, alice, Zone::Hand);
    game.player_mut(alice).expect("Alice exists").life = 20;

    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                casting_method: CastingMethod::PlayFrom { use_alternative: Some(_), .. },
                ..
            } if *spell_id == enchantment_id
        )),
        "Demon of Fate's Design should not offer its life-cost alternative outside your turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn demon_of_fates_design_sacrificed_enchantment_mana_value_sets_pump_amount() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let demon_id = game.create_object_from_definition(
        &demon_of_fates_design_definition(),
        alice,
        Zone::Battlefield,
    );
    assert!(
        !compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == demon_id
            )),
        "Demon of Fate's Design should not be able to sacrifice itself for its another-enchantment cost"
    );

    let sacrificial_enchantment = CardBuilder::new(CardId::from_raw(92_205), "Sacrificial Saga")
        .card_types(vec![CardType::Enchantment])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .build();
    let sacrifice_id =
        game.create_object_from_card(&sacrificial_enchantment, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Black, 1);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let ability_index = game
        .object(demon_id)
        .expect("Demon should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Demon should have an activated ability");
    let activate = PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
        source: demon_id,
        ability_index,
    });

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;
    let cost_order_ctx = match apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &activate,
        &mut dm,
    )
    .expect("Demon activation should start")
    {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!("expected next-cost chooser for Demon activation, got {other:?}"),
    };
    let sacrifice_cost_index = cost_order_ctx
        .options
        .iter()
        .find(|option| {
            option
                .description
                .to_ascii_lowercase()
                .contains("sacrifice")
        })
        .map(|option| option.index)
        .expect("expected Demon sacrifice cost option");
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::NextCostChoice(sacrifice_cost_index),
        &mut dm,
    )
    .expect("should choose Demon sacrifice cost first");
    match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => {}
        other => panic!("expected sacrifice object chooser for Demon activation, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::SacrificeTarget(sacrifice_id),
        &mut dm,
    )
    .expect("should choose the enchantment to sacrifice");
    assert!(
        game.stack.last().is_some_and(|entry| entry
            .tagged_objects
            .get(&crate::tag::TagKey::from("sacrifice_cost_0"))
            .is_some_and(|objects| objects
                .iter()
                .any(|object| object.name == "Sacrificial Saga"))),
        "Demon stack entry should remember the sacrificed enchantment"
    );

    resolve_stack_entry(&mut game).expect("Demon activation should resolve");
    assert_eq!(
        game.calculated_power(demon_id),
        Some(9),
        "Demon should get +3/+0 from the sacrificed enchantment's mana value"
    );
}
