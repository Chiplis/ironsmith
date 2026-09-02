use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::Zone;
use ironsmith_compiler::ability::{AbilityKind, ActivationTiming};
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effect::Value;
use ironsmith_compiler::effects::{ChooseObjectsEffect, PutOntoBattlefieldEffect, SequenceEffect};
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
    let sequence = effect.downcast_ref::<SequenceEffect>().expect(
        "Transfigure should keep its choose, battlefield move, and shuffle in one sequence",
    );
    let search = sequence.effects[0]
        .downcast_ref::<ChooseObjectsEffect>()
        .expect("Transfigure should choose a searched creature card");
    assert!(search.is_search);
    assert_eq!(search.zone, Some(Zone::Library));
    assert_eq!(search.filter.card_types, vec![CardType::Creature]);
    assert!(matches!(
        search.filter.mana_value.as_ref(),
        Some(Comparison::EqualExpr(value))
            if matches!(value.unhinted(), Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Source))
    ));
    fn contains_battlefield_move(effect: &ironsmith_compiler::effect::Effect) -> bool {
        if effect.downcast_ref::<PutOntoBattlefieldEffect>().is_some() {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            found |= contains_battlefield_move(child);
        });
        found
    }
    assert!(
        sequence.effects.iter().any(contains_battlefield_move),
        "Transfigure should put the searched card onto the battlefield: {sequence:#?}"
    );
}

#[test]
fn transfigure_instances_remain_separately_activatable() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Double Transfigure Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Transfigure {1}{B}\nTransfigure {2}{B}")
        .expect("multiple Transfigure instances should compile");

    assert_eq!(definition.abilities.len(), 2, "{:#?}", definition.abilities);
}
