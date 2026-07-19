use super::shard_16::parse_oracle_card_definition;
use super::*;

const CASES: &[(&str, Subtype, &str)] = &[
    (
        "Champion of the Path",
        Subtype::Elemental,
        "As an additional cost to cast this spell, behold an Elemental and exile it.",
    ),
    (
        "Champions of the Perfect",
        Subtype::Elf,
        "As an additional cost to cast this spell, behold an Elf and exile it.",
    ),
    (
        "Champions of the Shoal",
        Subtype::Merfolk,
        "As an additional cost to cast this spell, behold a Merfolk and exile it.",
    ),
];

fn unwrap_with_id(effect: &crate::effect::Effect) -> &crate::effect::Effect {
    effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map_or(effect, |with_id| unwrap_with_id(&with_id.effect))
}

fn leave_battlefield_return_effect(
    definition: &crate::CardDefinition,
) -> &crate::effects::ReturnToHandEffect {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .display()
                    .to_ascii_lowercase()
                    .contains("leaves the battlefield") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .flat_map(|triggered| triggered.effects.flattened_default_effects())
        .find_map(|effect| {
            unwrap_with_id(effect).downcast_ref::<crate::effects::ReturnToHandEffect>()
        })
        .expect("Champion should return its linked exiled card when it leaves")
}

#[test]
fn champion_behold_exile_costs_compile_to_exact_typed_surface() {
    for (name, subtype, expected_cost_line) in CASES {
        let definition = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&definition);
        assert_eq!(
            compiled.first().map(String::as_str),
            Some(*expected_cost_line),
            "{name} must retain the complete authored additional-cost surface: {compiled:#?}"
        );

        let costs = definition
            .additional_cost
            .as_all()
            .expect("Champion additional cost should be a mandatory conjunction");
        let [behold_cost, exile_cost] = costs else {
            panic!("{name} should have exactly linked Behold and exile cost components: {costs:#?}");
        };

        let behold_effect = unwrap_with_id(
            behold_cost
                .effect_ref()
                .expect("Behold should be an effect-backed cost"),
        );
        let tagged_behold = behold_effect
            .downcast_ref::<crate::effects::TaggedEffect>()
            .expect("Behold result must be tagged for the exile component");
        let behold = unwrap_with_id(&tagged_behold.effect)
            .downcast_ref::<crate::effects::BeholdEffect>()
            .expect("first cost component must retain typed Behold semantics");
        assert_eq!(behold.subtype, *subtype, "{name} Behold subtype");
        assert_eq!(behold.count, 1, "{name} Behold count");

        let move_to_exile = unwrap_with_id(
            exile_cost
                .effect_ref()
                .expect("exile should be an effect-backed cost"),
        )
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .expect("the linked object should move to exile as a cost");
        assert_eq!(move_to_exile.zone, Zone::Exile, "{name} cost destination");
        assert!(
            matches!(
                move_to_exile.target.base(),
                ChooseSpec::Tagged(tag) if tag == &tagged_behold.tag
            ),
            "{name} must exile the object selected by Behold: {definition:#?}"
        );

        let returned = leave_battlefield_return_effect(&definition);
        assert!(
            matches!(
                returned.spec.base(),
                ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            ),
            "{name} leave trigger must return the object linked to this source: {returned:#?}"
        );
    }
}

#[test]
fn champion_behold_exile_costs_exile_and_return_the_selected_card() {
    for (name, subtype, _) in CASES {
        let definition = parse_oracle_card_definition(name);
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let candidate_definition = CardDefinitionBuilder::new(CardId::new(), "Beheld Candidate")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![*subtype])
            .build();
        let candidate =
            game.create_object_from_definition(&candidate_definition, alice, Zone::Hand);
        let candidate_stable_id = game
            .object(candidate)
            .expect("candidate should exist in hand")
            .stable_id;

        let mut decision_maker = crate::decision::AutoPassDecisionMaker;
        let mut cost_context =
            crate::costs::CostContext::new(source, alice, &mut decision_maker);
        for cost in definition
            .additional_cost
            .as_all()
            .expect("Champion additional cost should be conjunctive")
        {
            cost.pay(&mut game, &mut cost_context)
                .unwrap_or_else(|error| panic!("{name} cost should be payable: {error:?}"));
        }

        let [exiled] = game.get_exiled_with_source_links(source) else {
            panic!("{name} must link exactly one exiled Behold object to its source");
        };
        assert_eq!(
            game.object(*exiled).map(|object| object.zone),
            Some(Zone::Exile),
            "{name} must exile the selected subtype card"
        );

        let exiled_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(*exiled).expect("linked exiled card should exist"),
            &game,
        );
        let mut tagged_objects = HashMap::new();
        tagged_objects.insert(
            crate::TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            vec![exiled_snapshot],
        );
        let return_effect = leave_battlefield_return_effect(&definition);
        let mut return_context =
            crate::effects::ExecutionContext::new_default(source, alice)
                .with_tagged_objects(tagged_objects);
        crate::effects::execute_effect(
            &mut game,
            &crate::effect::Effect::new(return_effect.clone()),
            &mut return_context,
        )
        .unwrap_or_else(|error| panic!("{name} leave return should resolve: {error:?}"));

        let returned = game
            .find_object_by_stable_id(candidate_stable_id)
            .and_then(|id| game.object(id))
            .expect("the beheld card should still exist after returning");
        assert_eq!(
            returned.zone,
            Zone::Hand,
            "{name} must return the same card selected and exiled as its cost"
        );
    }
}
