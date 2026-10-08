//! Authored evidence only: these tests have not been executed for the source-only
//! counter/exile permission repair.

use crate::{CardDefinition, CardDefinitionBuilder, CardId, ChooseSpec, PowerToughness};
use crate::ability::AbilityKind;
use crate::static_abilities::StaticAbilityPayload;
use ironsmith_compiler::compiler_pipeline::parse_text_with_annotations_lowered;
use ironsmith_compiler::grammar::values::{parse_mana_cost_rewrite, parse_type_line_rewrite};
use ironsmith_core::{CounterEffect, CounterExileGate, CounterExilePermission};

const FROZEN: &str = include_str!(
    "../../../reports/countered-spell-durable-permission-20261008/frozen-inputs.json"
);

fn field<'a>(metadata: &'a serde_json::Value, name: &str) -> &'a str {
    metadata[name].as_str().unwrap_or_else(|| panic!("missing frozen field {name}"))
}

fn builder(metadata: &serde_json::Value) -> CardDefinitionBuilder {
    let types = parse_type_line_rewrite(field(metadata, "type_line")).unwrap();
    let mut builder = CardDefinitionBuilder::new(CardId::new(), field(metadata, "name"))
        .mana_cost(parse_mana_cost_rewrite(field(metadata, "mana_cost")).unwrap())
        .supertypes(types.supertypes)
        .card_types(types.card_types)
        .subtypes(types.subtypes)
        .first_printed_set_name(field(metadata, "first_printed_set_name"));
    if let (Some(power), Some(toughness)) = (metadata["power"].as_str(), metadata["toughness"].as_str()) {
        builder = builder.power_toughness(PowerToughness::fixed(
            power.parse().unwrap(),
            toughness.parse().unwrap(),
        ));
    }
    builder
}

fn assert_atomic_counter(
    program: &crate::resolution::ResolutionProgram,
    permission: CounterExilePermission,
) {
    let effects = program.flattened_default_effects();
    let [effect] = effects else {
        panic!("the counter must be the sole owner, without independent grants or a may prompt: {program:#?}");
    };
    let counter = effect.downcast_ref::<CounterEffect>()
        .expect("the compiled root must be the counter itself, without a pre-move tag wrapper");
    assert_eq!(counter.exile_permission, Some(permission));
    let ChooseSpec::Target(inner) = counter.target.unhinted() else {
        panic!("counter target must remain a chosen spell: {counter:#?}");
    };
    assert_eq!(inner.unhinted(), &ChooseSpec::spell(), "the permanent gate must not narrow the target");
    let mut children = 0;
    effect.visit_child_effects(&mut |_| children += 1);
    assert_eq!(children, 0, "no detached permission, stable-card free-cost grant, or tag consumer");
}

fn assert_complete_card(definition: &CardDefinition, metadata: &serde_json::Value) {
    let id = field(metadata, "oracle_id");
    let types = parse_type_line_rewrite(field(metadata, "type_line")).unwrap();
    assert_eq!(definition.card.name, field(metadata, "name"));
    assert_eq!(definition.card.card_types, types.card_types);
    assert_eq!(definition.card.subtypes, types.subtypes);
    assert_eq!(definition.card.mana_cost, Some(parse_mana_cost_rewrite(field(metadata, "mana_cost")).unwrap()));
    let (gate, allow_land) = match id {
        "7687b2a7-816d-4416-979b-675e35e235fc" => (CounterExileGate::AnySpell, true),
        "d7cba934-02ad-4677-bb4d-50808b01b4f9" => (CounterExileGate::PermanentSpell, false),
        "c01411e0-77b2-4e65-a369-5dbe13745769" => (CounterExileGate::AnySpell, false),
        _ => panic!("unexpected card outside the exact frozen cohort: {id}"),
    };
    let permission = CounterExilePermission { gate, allow_land };
    if id == "c01411e0-77b2-4e65-a369-5dbe13745769" {
        assert_eq!(definition.card.power_toughness, Some(PowerToughness::fixed(3, 3)));
        assert!(definition.spell_effect.is_none());
        assert_eq!(definition.abilities.len(), 2, "retain full morph plus face-up trigger");
        let morph_cost = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => match &ability.payload {
                StaticAbilityPayload::Morph(cost) => Some(cost),
                _ => None,
            },
            _ => None,
        }).expect("full morph ability");
        assert_eq!(morph_cost.mana_cost(), Some(&parse_mana_cost_rewrite("{4}{U}{U}").unwrap()));
        let triggered = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        }).expect("face-up trigger");
        assert_eq!(triggered.trigger.kind, ironsmith_core::TriggerKind::ThisIsTurnedFaceUp);
        assert!(triggered.intervening_if.is_none());
        assert_atomic_counter(&triggered.effects, permission);
    } else {
        assert!(definition.abilities.is_empty());
        assert_atomic_counter(definition.spell_effect.as_ref().expect("instant program"), permission);
    }
}

#[test]
fn complete_frozen_metadata_raw_and_normalized_bodies_compile_strictly_without_loss() {
    let frozen: serde_json::Value = serde_json::from_str(FROZEN).unwrap();
    let records = frozen["actual_card_metadata"].as_array().unwrap();
    assert_eq!(records.len(), 3);
    for metadata in records {
        let retained = frozen["retained_results"].as_array().unwrap().iter()
            .find(|result| result["oracle_id"] == metadata["oracle_id"])
            .expect("matching retained input");
        for body in [field(metadata, "oracle_text"), field(retained, "normalized_oracle_text")] {
            let (parsed, loss) = ironsmith_compiler_api::parse_loss::capture(|| {
                parse_text_with_annotations_lowered(builder(metadata), body.to_owned(), false)
            });
            let (definition, _) = parsed.unwrap_or_else(|error| panic!("{}: {error}", field(metadata, "name")));
            assert!(!loss.is_lossy(), "{}: {}", field(metadata, "name"), loss.reasons_text());
            assert_complete_card(&definition, metadata);
        }
    }
}

#[test]
fn complete_frozen_input_does_not_depend_on_card_names() {
    let frozen: serde_json::Value = serde_json::from_str(FROZEN).unwrap();
    for metadata in frozen["actual_card_metadata"].as_array().unwrap() {
        let mut renamed = metadata.clone();
        renamed["name"] = "Independent Counter Permission Fixture".into();
        let (definition, _) = parse_text_with_annotations_lowered(
            builder(&renamed), field(&renamed, "oracle_text").to_owned(), false,
        ).expect("only grammar and metadata determine the behavior");
        assert_complete_card(&definition, &renamed);
    }
}

#[test]
fn complete_card_entrypoint_rejects_malformed_counter_permission_relations() {
    let frozen: serde_json::Value = serde_json::from_str(FROZEN).unwrap();
    for metadata in frozen["actual_card_metadata"].as_array().unwrap() {
        let retained = frozen["retained_results"].as_array().unwrap().iter()
            .find(|result| result["oracle_id"] == metadata["oracle_id"]).unwrap();
        let body = field(retained, "normalized_oracle_text");
        for changed in [
            format!("{body} Draw a card."),
            body.replace("Counter target spell.", ""),
            body.replace("counter target spell.", ""),
            body.replace("that spell is countered", "a creature spell is countered")
                .replace("a permanent spell is countered", "a creature spell is countered"),
            body.replace("instead of putting it into its owner's graveyard", "after putting it into its owner's graveyard"),
            body.replace("You may", "Its owner may"),
            body.replace(" for as long as it remains exiled", ""),
            body.replace("for as long as it remains exiled", "until your next turn"),
        ] {
            // Only the matching case changes initial capitalisation inside a
            // trigger. Do not accidentally test the unchanged positive body.
            if changed == body { continue; }
            assert!(
                parse_text_with_annotations_lowered(builder(metadata), changed.clone(), false).is_err(),
                "complete strict card entrypoint accepted a malformed relation: {changed}",
            );
        }
    }
}

#[test]
fn deleting_thranduils_entire_permission_from_full_metadata_is_rejected() {
    let frozen: serde_json::Value = serde_json::from_str(FROZEN).unwrap();
    let metadata = frozen["actual_card_metadata"].as_array().unwrap().iter()
        .find(|row| row["oracle_id"] == "d7cba934-02ad-4677-bb4d-50808b01b4f9").unwrap();
    let body = field(metadata, "oracle_text");
    let (truncated, permission) = body.split_once(" You may cast").unwrap();
    assert!(permission.contains("for as long as it remains exiled"));
    assert!(parse_text_with_annotations_lowered(builder(metadata), truncated.to_owned(), false).is_err());

    let unconditional = truncated.replace("If a permanent spell", "If that spell");
    let (result, loss) = ironsmith_compiler_api::parse_loss::capture(|| {
        parse_text_with_annotations_lowered(builder(metadata), unconditional, false)
    });
    assert!(result.is_ok(), "ordinary unconditional counter/exile remains supported");
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
}

#[test]
fn plain_counter_constructor_keeps_no_permission() {
    let (definition, _) = parse_text_with_annotations_lowered(
        CardDefinitionBuilder::new(CardId::new(), "Ordinary Counter Fixture")
            .card_types(vec![crate::CardType::Instant]),
        "Counter target spell.".to_owned(),
        false,
    ).unwrap();
    fn find_counter(effect: &crate::Effect) -> Option<CounterEffect> {
        if let Some(counter) = effect.downcast_ref::<CounterEffect>() {
            return Some(counter.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() { found = find_counter(child); }
        });
        found
    }
    let counter = definition.spell_effect.unwrap().flattened_default_effects().iter()
        .find_map(find_counter).expect("ordinary counter");
    assert_eq!(counter.exile_permission, None);
}
