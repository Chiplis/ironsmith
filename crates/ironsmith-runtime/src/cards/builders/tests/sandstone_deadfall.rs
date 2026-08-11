#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effects::ExecutionContext;

const ORACLE_TEXT: &str =
    "{T}, Sacrifice two lands and this artifact: Destroy target attacking creature.";

fn activated_ability(definition: &CardDefinition) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Sandstone Deadfall should have its activated ability")
}

fn land(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build()
}

fn current_zone(game: &crate::GameState, stable_id: StableId) -> Zone {
    game.find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .expect("the cost permanent should remain tracked")
        .zone
}

#[test]
fn sandstone_deadfall_keeps_two_distinct_sacrifice_requirements_and_exact_surface() {
    let definition = parse_oracle_card_definition("Sandstone Deadfall");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE_TEXT]);

    let costs = activated_ability(&definition).mana_cost.costs();
    let [tap, lands, source] = costs else {
        panic!("expected tap, two-land sacrifice, and source sacrifice: {costs:#?}");
    };
    assert!(tap.requires_tap());
    let sacrifice = lands
        .effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::SacrificeEffect>())
        .expect("the land requirement should be a typed sacrifice effect");
    assert_eq!(sacrifice.count, crate::effect::Value::Fixed(2));
    assert_eq!(sacrifice.filter.card_types, [CardType::Land]);
    assert!(
        !sacrifice.filter.card_types.contains(&CardType::Artifact),
        "the source artifact must not be folded into the chosen-land filter"
    );
    assert!(source.is_sacrifice_self());
}

#[test]
fn sandstone_deadfall_requires_two_lands_in_addition_to_its_source() {
    let definition = parse_oracle_card_definition("Sandstone Deadfall");
    let activated = activated_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.create_object_from_definition(&land("Only Sandstone Land"), alice, Zone::Battlefield);

    assert!(
        crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost).is_err(),
        "one land plus the source artifact must not satisfy a two-land and source cost"
    );
}

#[test]
fn sandstone_deadfall_pays_both_land_sacrifices_and_the_source_sacrifice_atomically() {
    let definition = parse_oracle_card_definition("Sandstone Deadfall");
    let activated = activated_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let first =
        game.create_object_from_definition(&land("First Sandstone Land"), alice, Zone::Battlefield);
    let second = game.create_object_from_definition(
        &land("Second Sandstone Land"),
        alice,
        Zone::Battlefield,
    );
    let stable_ids = [source, first, second].map(|id| {
        game.object(id)
            .expect("cost permanent should exist")
            .stable_id
    });

    crate::cost::can_pay_cost(&game, source, alice, &activated.mana_cost)
        .expect("two lands plus the source should make the cost payable");
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
    crate::special_actions::pay_total_cost_with_choice_in_context(
        &mut game,
        alice,
        source,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut ctx,
    )
    .expect("the complete conjunctive activation cost should be paid");

    for stable_id in stable_ids {
        assert_eq!(
            current_zone(&game, stable_id),
            Zone::Graveyard,
            "both selected lands and the source artifact must be sacrificed"
        );
    }
}
