#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const TRIGGER_LINE: &str = "Whenever Altaïr attacks, exile up to one target Assassin creature card from your graveyard with a memory counter on it. Then for each creature card you own in exile with a memory counter on it, create a tapped and attacking token that's a copy of it. Exile those tokens at end of combat.";

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn instant(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .build()
}

fn altair_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Altaïr should have an attack trigger")
}

fn memory_iteration(
    triggered: &crate::ability::TriggeredAbility,
) -> &crate::effects::ForEachObject {
    triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ForEachObject>())
        .expect("Altaïr should iterate the owned memory-counter cards in exile")
}

#[test]
fn altair_preserves_the_exile_iterator_copy_identity_and_lifecycle_surface() {
    let definition = parse_oracle_card_definition("Altaïr Ibn-La'Ahad");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["First strike".to_string(), TRIGGER_LINE.to_string()]
    );

    let for_each = memory_iteration(altair_trigger(&definition));
    assert_eq!(for_each.filter.zone, Some(Zone::Exile));
    assert_eq!(for_each.filter.owner, Some(PlayerFilter::You));
    assert_eq!(for_each.filter.card_types, [CardType::Creature]);
    assert_eq!(
        for_each.filter.with_counter,
        Some(crate::filter::CounterConstraint::Typed(
            crate::object::CounterType::Named("memory")
        ))
    );
    assert!(
        !format!("{:#?}", for_each.effects).contains("MoveToZoneEffect"),
        "the `in exile` zone phrase must not become a phantom exile action: {for_each:#?}"
    );

    let [copy_effect] = for_each.effects.as_slice() else {
        panic!("the iterator should contain only the token-copy action: {for_each:#?}");
    };
    let copy = copy_effect
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()
        .expect("the iterated card itself should be the copy source");
    assert!(matches!(copy.target.base(), ChooseSpec::Iterated));
    assert_eq!(copy.count, crate::effect::Value::Fixed(1));
    assert_eq!(copy.controller, PlayerFilter::You);
    assert!(copy.enters_tapped);
    assert!(copy.enters_attacking);
    assert!(copy.exile_at_end_of_combat);
    assert_eq!(
        copy.exile_at_end_of_combat_reference_surface,
        Some(ironsmith_core::TokenCopyReferenceSurface::ThoseTokens)
    );
}

#[test]
fn altair_copies_each_matching_owned_exiled_card_and_cleans_up_only_the_tokens() {
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::object::{CounterType, ObjectKind};

    let definition = parse_oracle_card_definition("Altaïr Ibn-La'Ahad");
    let for_each = memory_iteration(altair_trigger(&definition));
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let altair = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let first = game.create_object_from_definition(&creature("First Memory"), alice, Zone::Exile);
    let second = game.create_object_from_definition(&creature("Second Memory"), alice, Zone::Exile);
    let no_counter = game.create_object_from_definition(&creature("No Memory"), alice, Zone::Exile);
    let opposing =
        game.create_object_from_definition(&creature("Opposing Memory"), bob, Zone::Exile);
    let noncreature =
        game.create_object_from_definition(&instant("Spell Memory"), alice, Zone::Exile);
    for object in [first, second, opposing, noncreature] {
        game.add_counters(object, CounterType::Named("memory"), 1)
            .expect("the exiled object should accept a memory counter");
    }

    game.combat = Some(CombatState {
        attackers: vec![AttackerInfo {
            creature: altair,
            target: AttackTarget::Player(bob),
        }],
        ..CombatState::default()
    });
    let mut ctx = crate::effects::ExecutionContext::new_default(altair, alice);
    crate::effects::execute_effect(
        &mut game,
        &crate::effect::Effect::new(for_each.clone()),
        &mut ctx,
    )
    .expect("Altaïr's memory-card iterator should resolve");

    let copied_names = ["First Memory", "Second Memory"];
    let token_ids: Vec<_> = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ObjectKind::Token && copied_names.contains(&object.name.as_str())
            })
        })
        .collect();
    assert_eq!(
        token_ids.len(),
        2,
        "only the two matching cards should be copied"
    );
    for token in &token_ids {
        assert!(game.is_tapped(*token), "each copy should enter tapped");
        assert!(
            game.combat
                .as_ref()
                .is_some_and(|combat| combat.attackers.iter().any(|info| {
                    info.creature == *token && info.target == AttackTarget::Player(bob)
                })),
            "each copy should enter attacking Altaïr's defending player"
        );
    }
    assert_eq!(
        game.effect_store
            .delayed_triggers
            .iter()
            .filter(|delayed| {
                delayed.trigger.display().contains("end of combat")
                    && delayed.target_objects.len() == 1
                    && token_ids.contains(&delayed.target_objects[0])
            })
            .count(),
        2,
        "each created token should have its own linked end-of-combat cleanup"
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::EndOfCombatEvent::new(),
        ctx.provenance,
    );
    for entry in crate::triggers::check_delayed_triggers(&mut game, &event) {
        trigger_queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("put Altaïr cleanup triggers on the stack");
    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry(&mut game)
            .expect("resolve Altaïr end-of-combat cleanup");
    }

    for token in token_ids {
        assert!(!game.battlefield.contains(&token));
    }
    for original in [first, second, no_counter, opposing, noncreature] {
        assert_eq!(
            game.object(original)
                .expect("the original card should remain")
                .zone,
            Zone::Exile
        );
    }
}
