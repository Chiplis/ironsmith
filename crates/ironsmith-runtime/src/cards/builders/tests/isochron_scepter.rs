#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::ExecutionContext;

const IMPRINT_LINE: &str = "Imprint — When this artifact enters, you may exile an instant card with mana value 2 or less from your hand.";
const COPY_LINE: &str = "{2}, {T}: You may copy the exiled card. If you do, you may cast the copy without paying its mana cost.";

fn abilities(
    definition: &CardDefinition,
) -> (
    &crate::ability::TriggeredAbility,
    &crate::ability::ActivatedAbility,
) {
    let imprint = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:#?}", triggered.effects).contains("ImprintFromHandEffect") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Isochron Scepter should retain its Imprint trigger");
    let copy = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if format!("{:#?}", activated.effects).contains("CastTaggedEffect") =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Isochron Scepter should retain its linked copy-cast activation");
    (imprint, copy)
}

fn instant(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .card_types(vec![CardType::Instant])
        .build()
}

#[test]
fn isochron_scepter_keeps_exact_linked_copy_cast_structure_and_surface() {
    let definition = parse_oracle_card_definition("Isochron Scepter");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![IMPRINT_LINE.to_string(), COPY_LINE.to_string()]
    );

    let (_, activated) = abilities(&definition);
    let [producer_segment, result_segment] = activated.effects.segments.as_slice() else {
        panic!("copy-cast activation should have two result-linked segments: {activated:#?}");
    };
    assert!(producer_segment.self_replacements.is_empty());
    assert!(result_segment.self_replacements.is_empty());
    let [producer_effect] = producer_segment.default_effects.as_slice() else {
        panic!("copy choice segment should contain one effect: {producer_segment:#?}");
    };
    let producer = producer_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("copy choice should expose a result ID");
    let choice_may = producer
        .effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("copy choice should be optional");
    assert_eq!(choice_may.decider, Some(PlayerFilter::You));
    let [choice_effect] = choice_may.effects.as_slice() else {
        panic!("copy choice should contain one source-linked selection");
    };
    let choice = choice_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("copy choice should select the linked exiled card");
    assert_eq!(choice.zone, Some(Zone::Exile));
    assert_eq!(choice.count, crate::effect::ChoiceCount::exactly(1));
    assert!(choice.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
    }));

    let [result_effect] = result_segment.default_effects.as_slice() else {
        panic!("copy-cast result segment should contain one conditional");
    };
    let result = result_effect
        .downcast_ref::<crate::effects::IfEffect>()
        .expect("casting the copy should depend on accepting the copy action");
    assert_eq!(result.condition, producer.id);
    assert_eq!(result.predicate, crate::effect::EffectPredicate::Happened);
    assert!(result.else_.is_empty());
    let [cast_may_effect] = result.then.as_slice() else {
        panic!("accepted copy should contain one optional cast");
    };
    let cast_may = cast_may_effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("casting the copy should remain optional");
    let [cast_effect] = cast_may.effects.as_slice() else {
        panic!("optional cast should contain one typed cast");
    };
    let cast = cast_effect
        .downcast_ref::<crate::effects::CastTaggedEffect>()
        .expect("accepted action should cast the selected copy");
    assert_eq!(cast.tag, choice.tag);
    assert!(cast.as_copy);
    assert!(cast.without_paying_mana_cost);
}

#[test]
fn isochron_scepter_imprints_only_a_legal_instant_and_casts_a_free_copy() {
    let definition = parse_oracle_card_definition("Isochron Scepter");
    let (imprint, activated) = abilities(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let scepter = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let illegal =
        game.create_object_from_definition(&instant("Too Expensive", 3), alice, Zone::Hand);
    let legal = game.create_object_from_definition(&instant("Scepter Fuel", 2), alice, Zone::Hand);
    let legal_stable = game.object(legal).expect("fuel exists").stable_id;

    let mut imprint_decisions = SelectFirstDecisionMaker;
    let mut imprint_ctx = ExecutionContext::new(scepter, alice, &mut imprint_decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut imprint_ctx,
        alice,
        scepter,
        &imprint.effects,
        None,
        &[],
    )
    .expect("Isochron Scepter's Imprint trigger should resolve");
    assert_eq!(
        game.object(illegal)
            .expect("expensive instant remains")
            .zone,
        Zone::Hand
    );
    let exiled = game
        .find_object_by_stable_id(legal_stable)
        .expect("imprinted instant keeps stable identity");
    assert_eq!(
        game.object(exiled).expect("imprinted instant exists").zone,
        Zone::Exile
    );
    assert!(game.get_exiled_with_source_links(scepter).contains(&exiled));

    let mut copy_decisions = SelectFirstDecisionMaker;
    let mut copy_ctx = ExecutionContext::new(scepter, alice, &mut copy_decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut copy_ctx,
        alice,
        scepter,
        &activated.effects,
        None,
        &[],
    )
    .expect("Isochron Scepter's copy-cast activation should resolve");

    assert_eq!(
        game.object(exiled).expect("original imprint remains").zone,
        Zone::Exile,
        "casting the copy must not move the imprinted original"
    );
    let copied_spell = game
        .stack
        .iter()
        .find(|entry| {
            game.object(entry.object_id)
                .is_some_and(|object| object.name == "Scepter Fuel")
        })
        .expect("the freely cast copy should be on the stack");
    assert_ne!(copied_spell.object_id, exiled);
}
