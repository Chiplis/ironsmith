#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ENTRY_LINE: &str = "As Mimeoplasm enters, exile up to X creature cards from your graveyard. It enters with three +1/+1 counters on it for each creature card exiled this way.";
const COPY_LINE: &str = "{2}: Mimeoplasm becomes a copy of target creature card exiled with it, except it's 0/0 and has this ability.";

fn card(name: &str, card_types: Vec<CardType>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build()
}

fn source_exiled_counter_value(definition: &CardDefinition) -> Value {
    definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let model = static_ability.compiled_model()?;
            let ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { count, .. } =
                &model.payload
            else {
                return None;
            };
            Some(count.clone())
        })
        .expect("Mimeoplasm should retain its typed linked-exile counter value")
}

fn count_scaled_filter(value: &Value) -> Option<(&crate::target::ObjectFilter, i32)> {
    match value {
        Value::SurfaceHinted { value, .. } => count_scaled_filter(value),
        Value::CountScaled(filter, multiplier) => Some((filter, *multiplier)),
        _ => None,
    }
}

#[test]
fn mimeoplasm_keeps_creature_card_domain_and_linked_exile_surface() {
    let definition = parse_oracle_card_definition("Mimeoplasm, Revered One");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![ENTRY_LINE.to_string(), COPY_LINE.to_string()]
    );

    let value = source_exiled_counter_value(&definition);
    let (filter, multiplier) =
        count_scaled_filter(&value).expect("entry value should be a scaled object count");
    assert_eq!(multiplier, 3);
    assert_eq!(filter.zone, Some(Zone::Exile));
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }),
        "the entry counter value must count cards linked to this source: {filter:#?}"
    );
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("CopyOf") && debug.contains("TargetOnlyEffect"),
        "the activated ability must copy its targeted linked card: {debug}"
    );
}

#[test]
fn mimeoplasm_counter_value_counts_only_linked_exiled_creature_cards() {
    let definition = parse_oracle_card_definition("Mimeoplasm, Revered One");
    let value = source_exiled_counter_value(&definition);
    let alice = PlayerId::from_index(0);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let linked_creature = game.create_object_from_definition(
        &card("Linked Creature", vec![CardType::Creature]),
        alice,
        Zone::Exile,
    );
    let linked_instant = game.create_object_from_definition(
        &card("Linked Instant", vec![CardType::Instant]),
        alice,
        Zone::Exile,
    );
    let unlinked_creature = game.create_object_from_definition(
        &card("Unlinked Creature", vec![CardType::Creature]),
        alice,
        Zone::Exile,
    );
    game.add_exiled_with_source_link(source, linked_creature);
    game.add_exiled_with_source_link(source, linked_instant);

    let ctx = crate::effects::ExecutionContext::new_default(source, alice);
    assert_eq!(
        crate::effects::helpers::resolve_value(&game, &value, &ctx)
            .expect("linked-exile counter value should resolve"),
        3,
        "the linked creature contributes three counters; the linked instant and unlinked creature do not"
    );
    assert!(game.object(unlinked_creature).is_some());
}
