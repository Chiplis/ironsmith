use super::*;

#[test]
fn source_plus_any_number_sacrifice_renders_as_one_compound_cost() {
    let tag = TagKey::from("sacrifice_cost_0");
    let chosen = ObjectFilter::creature()
        .you_control()
        .in_zone(Zone::Battlefield);
    let cost = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::sacrifice_self(),
        crate::costs::Cost::validated_effect(Effect::choose_objects(
            chosen,
            ChoiceCount::any_number(),
            PlayerFilter::You,
            tag.clone(),
        )),
        crate::costs::Cost::validated_effect(Effect::sacrifice_player(
            ObjectFilter::tagged(tag.clone()),
            Value::Count(ObjectFilter::tagged(tag)),
            PlayerFilter::You,
        )),
    ]);

    assert_eq!(
        describe_total_cost(&cost),
        "Sacrifice this source and any number of creatures you control"
    );
}

#[test]
fn comma_then_source_and_plural_cost_set_exile_stays_one_instruction() {
    let leading = Effect::new(crate::effects::DrawCardsEffect::you(Value::Fixed(1)));
    let source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType("this artifact".to_string()),
        ),
    );
    let source_exile = Effect::new(crate::effects::MoveToZoneEffect::to_exile(source));
    let mut creatures = ObjectFilter::creature();
    creatures.set_explicit_card_noun(true);
    creatures.set_plural_object_noun_surface(true);
    creatures
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: TagKey::from("sacrifice_cost_0"),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    let set_exile = Effect::new(crate::effects::MoveToZoneEffect::to_exile(
        ChooseSpec::Object(creatures),
    ));
    let mut sequence = crate::effects::SequenceEffect::new(vec![leading, source_exile, set_exile]);
    sequence.surface = ironsmith_core::SequenceSurface::CommaThen;

    assert_eq!(
        describe_effect(&Effect::new(sequence)),
        "You draw a card, then exile this artifact and those creature cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn source_plus_any_number_sacrifice_preserves_the_result_set() {
    let oracle = "This artifact enters tapped.\n\
                  {T}, Sacrifice this artifact and any number of creatures you control: \
                  This artifact deals X damage to any target, where X is the total power of the \
                  creatures sacrificed this way, then exile this artifact and those creature cards.";
    let definition =
        crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sword of the Ages")
            .card_types(vec![CardType::Artifact])
            .parse_text(oracle)
            .expect("source-plus-chosen sacrifice ability should parse");

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("TotalPower") && debug.contains("sacrifice_cost_0"),
        "the total-power value should reference the chosen sacrifice set: {debug}"
    );
    assert!(
        debug.matches("sacrifice_cost_0").count() >= 5
            && debug.contains("plural_object_noun: true"),
        "the final plural card move should retain the chosen sacrifice set instead of rebinding to the intervening source exile: {debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&definition).join("\n");
    assert!(
        rendered.contains("Sacrifice this artifact and any number of creatures"),
        "compound sacrifice cost should survive rendering: {rendered}"
    );
    assert!(
        rendered.contains("total power of the creatures sacrificed this way"),
        "the chosen sacrifice set should remain the X basis: {rendered}"
    );
    assert_eq!(rendered, oracle);
}
