use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::triggers::TriggerKind;
use ironsmith_compiler::types::CardType;

#[test]
fn ability_trigger_clause_lowers_to_the_typed_trigger_model() {
    for (clause, expected_another) in [
        ("another triggered ability triggers", true),
        ("an ability triggers", false),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Ability Trigger Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(format!("Whenever {clause}, draw a card."))
            .unwrap_or_else(|error| panic!("{clause} should compile: {error}"));

        let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected a triggered ability: {:#?}", definition.abilities);
        };
        assert_eq!(
            triggered.trigger.kind,
            TriggerKind::AbilityTriggered {
                another: expected_another,
                source_filter: None,
                caused_by_source_entering: false,
            }
        );
    }
}
