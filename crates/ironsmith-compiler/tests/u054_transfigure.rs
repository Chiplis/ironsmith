use ironsmith_compiler::Zone;
use ironsmith_compiler::ability::{AbilityKind, ActivationTiming};
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effect::Value;
use ironsmith_compiler::effects::SearchLibraryEffect;
use ironsmith_compiler::filter::Comparison;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::target::ChooseSpec;
use ironsmith_compiler::types::CardType;

#[test]
fn transfigure_lowers_to_a_sorcery_speed_sacrifice_and_lki_search() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Fleshwrither")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Transfigure {1}{B}{B} ({1}{B}{B}, Sacrifice this creature: Search your library for a creature card with the same mana value as this creature, put that card onto the battlefield, then shuffle. Transfigure only as a sorcery.)",
        )
        .expect("Transfigure should compile");

    assert_eq!(definition.abilities.len(), 1, "{:#?}", definition.abilities);
    let ability = &definition.abilities[0];
    let AbilityKind::Activated(activated) = &ability.kind else {
        panic!("Transfigure must be executable: {ability:#?}");
    };
    assert_eq!(ability.functional_zones, vec![Zone::Battlefield]);
    assert!(matches!(activated.timing, ActivationTiming::SorcerySpeed));
    assert!(
        activated
            .mana_cost
            .costs()
            .iter()
            .any(|cost| matches!(cost, ironsmith_core::Cost::SacrificeSelf))
    );

    let [effect] = activated.effects.flattened_default_effects() else {
        panic!("Transfigure should contain one search effect: {activated:#?}");
    };
    let search = effect
        .downcast_ref::<SearchLibraryEffect>()
        .expect("Transfigure should search the library");
    assert_eq!(search.destination, Zone::Battlefield);
    assert_eq!(search.filter.card_types, vec![CardType::Creature]);
    assert!(matches!(
        search.filter.mana_value.as_ref(),
        Some(Comparison::EqualExpr(value))
            if matches!(value.unhinted(), Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Source))
    ));
}

#[test]
fn transfigure_instances_remain_separately_activatable() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Double Transfigure Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Transfigure {1}{B}\nTransfigure {2}{B}")
        .expect("multiple Transfigure instances should compile");

    assert_eq!(definition.abilities.len(), 2, "{:#?}", definition.abilities);
}
