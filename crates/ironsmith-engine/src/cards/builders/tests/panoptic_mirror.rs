#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;
use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
use crate::effects::ExecutionContext;

fn panoptic_abilities(
    definition: &CardDefinition,
) -> (
    &crate::ability::ActivatedAbility,
    &crate::ability::TriggeredAbility,
) {
    let imprint = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if format!("{:#?}", activated.effects).contains("ImprintFromHandEffect") =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Panoptic Mirror should retain its activated Imprint ability");
    let upkeep = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:#?}", triggered.effects).contains("CastTaggedEffect") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Panoptic Mirror should retain its linked upkeep copy ability");
    (imprint, upkeep)
}

#[test]
fn panoptic_mirror_preserves_the_linked_imprint_copy_structure_and_floor() {
    let definition = parse_oracle_card_definition("Panoptic Mirror");
    let (imprint, upkeep) = panoptic_abilities(&definition);
    let imprint_debug = format!("{imprint:#?}");
    let upkeep_debug = format!("{upkeep:#?}");

    assert!(
        imprint_debug.contains("ManaPaymentCost")
            && imprint_debug.contains("X")
            && imprint_debug.contains("TapEffect")
            && imprint_debug.contains("ImprintFromHandEffect")
            && imprint_debug.contains("EqualExpr(\n")
            && imprint_debug.contains("Hand"),
        "the activated ability must pay X, tap, and imprint a matching hand card: {imprint_debug}"
    );
    assert!(
        upkeep_debug.contains(crate::tag::SOURCE_EXILED_TAG)
            && upkeep_debug.contains("CastTaggedEffect")
            && upkeep_debug.contains("as_copy: true")
            && upkeep_debug.contains("without_paying_mana_cost: true"),
        "the upkeep ability must choose a source-linked card and free-cast a copy: {upkeep_debug}"
    );

    let oracle = oracle_text_by_name()
        .get("Panoptic Mirror")
        .expect("Panoptic Mirror oracle text should be present");
    let compiled = unprocessed_compiled_lines(&definition);
    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        "Panoptic Mirror",
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "Panoptic Mirror must retain the semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}

fn two_mana_instant(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Instant])
        .build()
}

fn three_mana_instant(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Instant])
        .build()
}

#[test]
fn panoptic_mirror_imprints_the_x_card_then_casts_a_copy_without_moving_the_original() {
    let definition = parse_oracle_card_definition("Panoptic Mirror");
    let (imprint, upkeep) = panoptic_abilities(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let mirror = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let wrong_cost = game.create_object_from_definition(
        &three_mana_instant("Wrong-Cost Mirror Fuel"),
        alice,
        Zone::Hand,
    );
    let fuel = game.create_object_from_definition(
        &two_mana_instant("Panoptic Mirror Fuel"),
        alice,
        Zone::Hand,
    );
    let fuel_stable = game.object(fuel).expect("fuel should exist").stable_id;

    let mut imprint_decisions = SelectFirstDecisionMaker;
    let mut imprint_ctx = ExecutionContext::new(mirror, alice, &mut imprint_decisions).with_x(2);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut imprint_ctx,
        alice,
        mirror,
        &imprint.effects,
        None,
        &[],
    )
    .expect("Panoptic Mirror's activated Imprint ability should resolve");

    assert_eq!(
        game.object(wrong_cost)
            .expect("wrong-cost card remains")
            .zone,
        Zone::Hand,
        "X=2 must not imprint the mana-value-three card"
    );
    let exiled_fuel = game
        .find_object_by_stable_id(fuel_stable)
        .expect("imprinted fuel should retain stable identity");
    assert_eq!(
        game.object(exiled_fuel)
            .expect("imprinted fuel exists")
            .zone,
        Zone::Exile
    );
    assert!(
        game.get_exiled_with_source_links(mirror)
            .contains(&exiled_fuel),
        "the imprinted card must be linked to this Panoptic Mirror"
    );

    let mut upkeep_decisions = SelectFirstDecisionMaker;
    let mut upkeep_ctx = ExecutionContext::new(mirror, alice, &mut upkeep_decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut upkeep_ctx,
        alice,
        mirror,
        &upkeep.effects,
        None,
        &[],
    )
    .expect("Panoptic Mirror's upkeep ability should cast the chosen copy");

    assert_eq!(
        game.object(exiled_fuel)
            .expect("original fuel remains")
            .zone,
        Zone::Exile,
        "casting the copy must not move the original imprinted card"
    );
    let copied_spell = game
        .stack
        .iter()
        .find(|entry| {
            game.object(entry.object_id)
                .is_some_and(|object| object.name == "Panoptic Mirror Fuel")
        })
        .expect("the free-cast copy should be on the stack");
    assert_ne!(copied_spell.object_id, exiled_fuel);
}

struct BooleanSequence {
    answers: std::vec::IntoIter<bool>,
}

impl BooleanSequence {
    fn new(answers: Vec<bool>) -> Self {
        Self {
            answers: answers.into_iter(),
        }
    }
}

impl DecisionMaker for BooleanSequence {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.answers.next().unwrap_or(false)
    }

    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .find(|candidate| candidate.legal)
            .map(|candidate| vec![candidate.id])
            .unwrap_or_default()
    }
}

#[test]
fn panoptic_mirror_may_choose_the_linked_card_and_decline_to_cast_its_copy() {
    let definition = parse_oracle_card_definition("Panoptic Mirror");
    let (imprint, upkeep) = panoptic_abilities(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let mirror = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let fuel = game.create_object_from_definition(
        &two_mana_instant("Declined Mirror Fuel"),
        alice,
        Zone::Hand,
    );
    let fuel_stable = game.object(fuel).expect("fuel should exist").stable_id;

    let mut imprint_decisions = SelectFirstDecisionMaker;
    let mut imprint_ctx = ExecutionContext::new(mirror, alice, &mut imprint_decisions).with_x(2);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut imprint_ctx,
        alice,
        mirror,
        &imprint.effects,
        None,
        &[],
    )
    .expect("Panoptic Mirror should imprint the test fuel");

    let exiled_fuel = game
        .find_object_by_stable_id(fuel_stable)
        .expect("imprinted fuel should retain stable identity");
    let stack_len = game.stack.len();
    let mut decisions = BooleanSequence::new(vec![true, false]);
    let mut upkeep_ctx = ExecutionContext::new(mirror, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut upkeep_ctx,
        alice,
        mirror,
        &upkeep.effects,
        None,
        &[],
    )
    .expect("declining the free cast should still resolve the upkeep ability");

    assert_eq!(
        game.stack.len(),
        stack_len,
        "declining the second may must not put a copied spell on the stack"
    );
    assert_eq!(
        game.object(exiled_fuel)
            .expect("original fuel remains")
            .zone,
        Zone::Exile,
        "declining the cast must leave the imprinted original in exile"
    );
}
