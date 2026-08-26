use super::*;

const BURDEN_OF_PROOF_ORACLE: &str = "Flash\nEnchant creature\nEnchanted creature gets +2/+2 as long as it's a Detective you control. Otherwise, it has base power and toughness 1/1 and can't block Detectives.";

#[test]
fn attached_conditional_anthem_keeps_the_complete_otherwise_branch() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Burden of Proof")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(BURDEN_OF_PROOF_ORACLE)
            .expect("the conditional attached program should compile");

    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("Anthem"), "{debug}");
    assert!(debug.contains("SetBasePowerToughness"), "{debug}");
    assert!(debug.contains("BlockSpecificAttacker"), "{debug}");
    assert_eq!(debug.matches("Not(").count(), 2, "{debug}");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        BURDEN_OF_PROOF_ORACLE
    );
}
