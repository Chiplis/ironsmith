//! Explicit mode boundaries compose with an optional program and outer siblings.
fn compile(text: &str) -> ironsmith_compiler::cards::CardDefinition {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_core::CardId::new(), "Modal scope witness");
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler::CompilerFacade::new().compile_definition(builder, text, ironsmith_compiler::CompilePolicy { allow_unsupported: false })));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    result.unwrap().definition
}
#[test]
fn compound_modes_preserve_nested_scope_and_outer_sibling_in_artifact() {
    let text = "Type: Sorcery\nYou may do the following: [Choose one of the following: [Draw a card; You gain 1 life] or [Create a Treasure token]]. You lose 1 life.";
    let definition = compile(text);
    let root = definition.spell_effect.as_ref().unwrap().all_effects_owned();
    assert_eq!(root.len(), 2, "the final life loss belongs outside the optional choice");
    let debug = format!("{:?}", root[0]);
    assert!(debug.contains("ChooseMode"));
    assert!(debug.contains("Draw"));
    assert!(debug.contains("GainLife"));
    let (artifact, _) = ironsmith_compiler_runtime::compile_to_artifact("Modal scope witness", text, false).unwrap();
    assert_eq!(ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap(), artifact);
}

#[test]
fn quoted_cost_menu_after_a_selection_uses_complete_header_claims(){
    let text="Type: Creature\nPower/Toughness: 2/2\nWhen this creature enters, you may pay the cost \"Choose this creature. Choose one of the following: [Sacrifice a creature you control other than the chosen creature] or [Exile a creature card from your graveyard]\". If you do, draw a card.";
    let definition=compile(text);
    let debug=format!("{:?}",definition.abilities);
    assert!(debug.contains("ChooseMode"),"both complete cost alternatives must survive the selected-object prelude");
    assert!(debug.contains("Sacrifice")&&debug.contains("Exile"));
    assert!(debug.contains("May"),"the payment keeps its single optional owner");
}
