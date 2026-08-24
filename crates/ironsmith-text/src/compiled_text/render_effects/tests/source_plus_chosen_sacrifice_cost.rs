use super::*;

#[test]
fn source_plus_any_number_sacrifice_renders_as_one_compound_cost() {
    let tag = TagKey::from("sacrifice_cost_0");
    let chosen = ObjectFilter::creature()
        .you_control()
        .in_zone(Zone::Battlefield);
    let cost = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::sacrifice_self(),
        crate::costs::Cost::try_from_runtime_effect(Effect::choose_objects(
            chosen,
            ChoiceCount::any_number(),
            PlayerFilter::You,
            tag.clone(),
        ))
        .expect("object choice should be a valid cost effect"),
        crate::costs::Cost::try_from_runtime_effect(Effect::sacrifice_player(
            ObjectFilter::tagged(tag.clone()),
            Value::Count(ObjectFilter::tagged(tag)),
            PlayerFilter::You,
        ))
        .expect("sacrifice should be a valid cost effect"),
    ]);

    assert_eq!(
        describe_total_cost(&cost),
        "Sacrifice this source and any number of creatures you control"
    );
}

#[test]
fn chosen_set_then_source_sacrifice_renders_in_authored_order() {
    let lands = ObjectFilter::default()
        .with_type(CardType::Land)
        .you_control()
        .in_zone(Zone::Battlefield);
    let cost = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::try_from_runtime_effect(Effect::sacrifice(lands, 2))
            .expect("sacrifice should be a valid cost effect"),
        crate::costs::Cost::sacrifice_self(),
    ]);

    assert_eq!(
        describe_total_cost(&cost),
        "Sacrifice two lands and this source"
    );
}

#[test]
fn two_ordinary_sacrifice_costs_do_not_inherit_the_source_compactor() {
    let land = crate::costs::Cost::try_from_runtime_effect(Effect::sacrifice(
        ObjectFilter::default().with_type(CardType::Land),
        1,
    ))
    .expect("land sacrifice should be a valid cost effect");
    let artifact = crate::costs::Cost::try_from_runtime_effect(Effect::sacrifice(
        ObjectFilter::default().with_type(CardType::Artifact),
        1,
    ))
    .expect("artifact sacrifice should be a valid cost effect");
    let cost = crate::cost::TotalCost::from_costs(vec![land, artifact]);

    assert_eq!(
        describe_cost_component_parts(cost.costs()).len(),
        2,
        "the conjunction requires a typed sacrifice-self component"
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

#[test]
fn aggregate_of_a_sacrifice_cost_result_set_uses_a_definite_article() {
    let sacrificed = ObjectFilter::creature().match_tagged(
        TagKey::from("sacrifice_cost_0"),
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    );
    let total_power = Value::TotalPower(sacrificed);

    assert_eq!(
        describe_where_x_basis(&total_power).as_deref(),
        Some("the total power of the creatures sacrificed this way")
    );
    assert_eq!(
        describe_value(&total_power),
        "the total power of the creatures sacrificed this way"
    );
}

#[test]
fn aggregate_of_an_open_controlled_set_remains_unarticled() {
    let total_power = Value::TotalPower(ObjectFilter::creature().you_control());

    assert_eq!(
        describe_where_x_basis(&total_power).as_deref(),
        Some("the total power of creatures you control")
    );
    assert_eq!(
        describe_value(&total_power),
        "the total power of creatures you control"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn source_plus_any_number_sacrifice_preserves_the_result_set() {
    let oracle = "This artifact enters tapped.\n\
                  {T}, Sacrifice this artifact and any number of creatures you control: \
                  This artifact deals X damage to any target, where X is the total power of the \
                  creatures sacrificed this way, then exile this artifact and those creature cards.";
    let definition = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Sword of the Ages",
    )
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
