use super::*;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;
#[cfg(test)]
use ironsmith_compiler_lowering::CardDefinitionBuilder;

const FLAMES_OF_REBIRTH: &str = "Return any number of target creature cards with total mana value 6 or less from your graveyard to the battlefield.";

#[test]
fn targeted_graveyard_return_keeps_total_mana_value_as_a_set_constraint() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Flames of Rebirth Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(FLAMES_OF_REBIRTH)
        .expect("Flames of Rebirth should compile");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("the return instruction should remain a spell program");
    let graveyard_return = program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| {
            super::find_nested_effect::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
                effect,
            )
        })
        .unwrap_or_else(|| panic!("expected a targeted graveyard return: {program:#?}"));

    assert!(graveyard_return.target.is_target());
    assert_eq!(graveyard_return.target.count(), ChoiceCount::any_number());
    let crate::target::ChooseSpec::Object(filter) = graveyard_return.target.base() else {
        panic!("expected an object target: {:#?}", graveyard_return.target);
    };
    assert!(
        filter.mana_value.is_none(),
        "the total must not become a per-card mana-value filter: {filter:#?}"
    );
    assert_eq!(
        filter.target_set_aggregate_constraint.as_deref(),
        Some(&crate::effect::ChoiceAggregateConstraint::total_mana_value_at_most(6))
    );
}
