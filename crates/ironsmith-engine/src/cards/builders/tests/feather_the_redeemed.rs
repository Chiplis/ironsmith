#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effect::OutcomeStatus;
use crate::effects::{ExecutionContext, execute_effect};

fn put_delayed_end_step_trigger_on_stack(game: &mut crate::GameState, player: PlayerId) -> usize {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    );
    let entries = crate::triggers::check_delayed_triggers(game, &event);
    let count = entries.len();
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    if count > 0 {
        crate::game_loop::put_triggers_on_stack(game, &mut queue)
            .expect("Feather's delayed return should go on the stack");
    }
    count
}

fn targeted_instant(
    game: &mut crate::GameState,
    owner: PlayerId,
    name: &str,
    target: ObjectId,
) -> ObjectId {
    let target_spec = ChooseSpec::target_creature();
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::draw(1)])
        .build();
    let spell = game.create_object_from_definition(&definition, owner, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, owner)
            .with_targets(vec![crate::game_state::Target::Object(target)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: target_spec,
                range: 0..1,
            }]),
    );
    spell
}

#[test]
fn feather_exiles_only_the_resolving_triggering_spell_and_returns_it_only_if_exiled() {
    let definition = parse_oracle_card_definition("Feather, the Redeemed");
    let rendered = canonical_compiled_lines(&definition).join(" ");
    assert!(
        rendered
            .contains("exile that card instead of putting it into your graveyard as it resolves")
            && rendered.contains(
                "If you do, return it to your hand at the beginning of the next end step"
            ),
        "Feather must retain both the replacement and its conditional delayed return: {rendered}"
    );

    let (triggered, cast_trigger) = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::SpellCastTrigger>()
                .map(|cast| (triggered, cast)),
            _ => None,
        })
        .expect("Feather should have an instant-or-sorcery cast trigger");
    assert_eq!(cast_trigger.caster, PlayerFilter::You);

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let feather = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature_definition = CardDefinitionBuilder::new(CardId::new(), "Feather Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let controlled_creature =
        game.create_object_from_definition(&creature_definition, alice, Zone::Battlefield);
    let opponent_creature =
        game.create_object_from_definition(&creature_definition, bob, Zone::Battlefield);

    let spell = targeted_instant(&mut game, alice, "Feathered Instant", controlled_creature);
    let stable_id = game.object(spell).expect("spell should exist").stable_id;
    let spell_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell).expect("spell should exist"),
        &game,
    );
    let cast_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new_with_snapshot(
            spell,
            alice,
            Zone::Hand,
            spell_snapshot,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        triggered.trigger.matches(
            &cast_event,
            &crate::triggers::TriggerContext::for_source(feather, alice, &game),
        ),
        "an instant targeting a creature Feather's controller controls must trigger"
    );

    let wrong_target_spell = targeted_instant(
        &mut game,
        alice,
        "Opponent-Targeting Instant",
        opponent_creature,
    );
    let wrong_target_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(wrong_target_spell, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        !triggered.trigger.matches(
            &wrong_target_event,
            &crate::triggers::TriggerContext::for_source(feather, alice, &game),
        ),
        "a spell targeting only an opponent's creature must not trigger Feather"
    );

    let mut trigger_ctx =
        ExecutionContext::new_default(feather, alice).with_triggering_event(cast_event);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut trigger_ctx)
            .expect("Feather's cast trigger should register its replacement");
    }
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        0,
        "the return must not be scheduled before the replacement actually exiles the spell"
    );

    let move_outcome = execute_effect(
        &mut game,
        &Effect::move_to_zone(ChooseSpec::SpecificObject(spell), Zone::Graveyard, false),
        &mut trigger_ctx,
    )
    .expect("the resolving spell should attempt to enter its owner's graveyard");
    assert_eq!(move_outcome.status, OutcomeStatus::Replaced);
    let exiled = game
        .find_object_by_stable_id(stable_id)
        .expect("the replaced spell should retain its stable identity");
    assert_eq!(
        game.object(exiled).expect("exiled spell should exist").zone,
        Zone::Exile
    );
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        1,
        "the successful exile should create exactly one delayed return"
    );

    assert_eq!(put_delayed_end_step_trigger_on_stack(&mut game, alice), 1);
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Feather's delayed return should resolve");
    let returned = game
        .find_object_by_stable_id(stable_id)
        .expect("the returned card should retain its stable identity");
    assert_eq!(
        game.object(returned)
            .expect("returned card should exist")
            .zone,
        Zone::Hand
    );

    let bypass_spell = targeted_instant(
        &mut game,
        alice,
        "Spell Exiled Some Other Way",
        controlled_creature,
    );
    let bypass_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(bypass_spell)
            .expect("bypass spell should exist"),
        &game,
    );
    let bypass_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new_with_snapshot(
            bypass_spell,
            alice,
            Zone::Hand,
            bypass_snapshot,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut bypass_ctx =
        ExecutionContext::new_default(feather, alice).with_triggering_event(bypass_event);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut bypass_ctx)
            .expect("the second Feather trigger should register its replacement");
    }
    let delayed_before_bypass = game.effect_store.delayed_triggers.len();
    execute_effect(
        &mut game,
        &Effect::move_to_zone(ChooseSpec::SpecificObject(bypass_spell), Zone::Exile, false),
        &mut bypass_ctx,
    )
    .expect("the spell should be exiled without using Feather's graveyard replacement");
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        delayed_before_bypass,
        "`If you do` must not schedule a return when Feather's replacement did not apply"
    );
}
