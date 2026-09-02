use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effects::EndCombatPhaseEffect;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::types::CardType;

#[test]
fn u037_end_combat_phase_lowers_to_typed_effect() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Procedure Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("End the combat phase.")
        .expect("end-combat procedure should parse");
    let program = definition.spell_effect.expect("spell program");
    let effects = program.flattened_default_effects();
    assert_eq!(effects.len(), 1, "{effects:#?}");
    assert!(
        effects[0].downcast_ref::<EndCombatPhaseEffect>().is_some(),
        "{effects:#?}"
    );
}

#[test]
fn u037_mandate_of_peace_contains_typed_end_combat_procedure() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Mandate of Peace")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Cast this spell only during combat.\n\
             Your opponents can't cast spells this turn.\n\
             End the combat phase. (Remove all attackers and blockers from combat. Exile all spells and abilities from the stack, including this spell.)",
        )
        .expect("Mandate of Peace should compile");
    let program = definition.spell_effect.expect("spell program");
    let effects = program.flattened_default_effects();
    assert!(
        effects
            .iter()
            .any(|effect| effect.downcast_ref::<EndCombatPhaseEffect>().is_some()),
        "{effects:#?}"
    );
}
