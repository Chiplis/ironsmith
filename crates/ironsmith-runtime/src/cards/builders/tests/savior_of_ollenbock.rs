#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn savior_effects(
    definition: &CardDefinition,
) -> (
    crate::effects::ExileEffect,
    crate::effects::MoveToZoneEffect,
) {
    let mut exile = None;
    let mut returned = None;
    for ability in &definition.abilities {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            continue;
        };
        for effect in triggered.effects.flattened_default_effects() {
            if exile.is_none()
                && let Some(found) = effect.downcast_ref::<crate::effects::ExileEffect>()
            {
                exile = Some(found.clone());
            }
            if returned.is_none()
                && let Some(found) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
                && (found.exiled_with_source_surface.is_some()
                    || matches!(
                        found.target.base(),
                        ChooseSpec::Tagged(tag)
                            if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                    ))
            {
                returned = Some(found.clone());
            }
        }
    }
    (
        exile.expect("training trigger exile"),
        returned.expect("source-linked leave return"),
    )
}

#[test]
fn savior_keeps_the_zone_specific_target_union_and_exact_return_surface() {
    let definition = parse_oracle_card_definition("Savior of Ollenbock");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Savior of Ollenbock"]
    );

    let (exile, returned) = savior_effects(&definition);
    assert!(exile.spec.is_target());
    assert_eq!(exile.spec.count(), ChoiceCount::up_to(1));
    let ChooseSpec::Object(filter) = exile.spec.base() else {
        panic!("expected one typed object union: {exile:#?}");
    };
    assert!(filter.card_types.is_empty(), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of.iter().any(|branch| {
        branch.zone == Some(Zone::Battlefield)
            && branch.card_types == [CardType::Creature]
            && branch.other
            && !branch.has_explicit_card_noun()
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.zone == Some(Zone::Graveyard)
            && branch.card_types == [CardType::Creature]
            && !branch.other
            && branch.has_explicit_card_noun()
    }));
    assert!(matches!(
        returned.target.base(),
        ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
    ));
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(
        returned.battlefield_controller,
        crate::effects::BattlefieldController::Owner
    );
    assert!(returned.controller_surface_explicit);
}

#[test]
fn savior_exiles_both_legal_zone_arms_and_returns_only_its_linked_cards() {
    let definition = parse_oracle_card_definition("Savior of Ollenbock");
    let (exile, returned) = savior_effects(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let battlefield =
        game.create_object_from_definition(&creature("Battlefield Target"), bob, Zone::Battlefield);
    let graveyard =
        game.create_object_from_definition(&creature("Graveyard Target"), alice, Zone::Graveyard);
    let unrelated =
        game.create_object_from_definition(&creature("Unrelated Exiled"), bob, Zone::Exile);
    let battlefield_stable = game.object(battlefield).expect("target exists").stable_id;
    let graveyard_stable = game.object(graveyard).expect("target exists").stable_id;
    let unrelated_stable = game.object(unrelated).expect("target exists").stable_id;

    let mut context = crate::effects::ExecutionContext::new_default(source, alice);
    for target in [battlefield, graveyard] {
        let mut one_exile = exile.clone();
        one_exile.spec = ChooseSpec::SpecificObject(target);
        crate::effects::execute_effect(&mut game, &Effect::new(one_exile), &mut context)
            .expect("a selected legal union arm should be exiled");
    }
    for stable in [battlefield_stable, graveyard_stable] {
        let object = game
            .find_object_by_stable_id(stable)
            .and_then(|id| game.object(id))
            .expect("exiled target remains tracked");
        assert_eq!(object.zone, Zone::Exile);
    }

    crate::effects::execute_effect(&mut game, &Effect::new(returned), &mut context)
        .expect("the source-linked collection should return");
    for (stable, owner) in [(battlefield_stable, bob), (graveyard_stable, alice)] {
        let id = game
            .find_object_by_stable_id(stable)
            .expect("returned target remains tracked");
        let object = game.object(id).expect("returned target exists");
        assert_eq!(object.zone, Zone::Battlefield);
        assert_eq!(game.controller_of(object), owner);
    }
    assert_eq!(
        game.find_object_by_stable_id(unrelated_stable)
            .and_then(|id| game.object(id))
            .expect("unrelated exile remains tracked")
            .zone,
        Zone::Exile,
        "the leave trigger must not return cards exiled by another source"
    );
}
