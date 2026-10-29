//! Legacy in-process source compilation adapter.
//!
//! Browser release builds use the independent compiler wasm module. Native
//! callers and compatibility builds may enable this adapter explicitly.

pub use ironsmith_compiler::CardDefinitionBuilder as CompilerCardDefinitionBuilder;
pub use ironsmith_registry::{
    compile_builder_to_runtime_definition, compile_to_runtime_definition, register_builtin_dungeons,
};
pub use ironsmith_compiler::cards::CardDefinition as CompilerCardDefinition;
pub use ironsmith_compiler::with_custom_tokens;

/// Compile a custom token's own rules text strictly. The compiler-level
/// definition is what token-creation lowering substitutes for a reference
/// (`with_custom_tokens`); the runtime definition creates the token directly.
pub fn compile_builder_to_token_definitions(
    builder: CompilerCardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<(CompilerCardDefinition, ironsmith::cards::CardDefinition), String> {
    let compiled = ironsmith_compiler::CompilerFacade::new()
        .compile_definition(
            builder.token(),
            text,
            ironsmith_compiler::CompilePolicy { allow_unsupported: false },
        )
        .map_err(|error| error.to_string())?;
    let runtime = ironsmith_registry::into_runtime_definition(compiled.definition.clone())
        .map_err(|error| error.to_string())?;
    Ok((compiled.definition, runtime))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bobble() -> CompilerCardDefinition {
        let builder = CompilerCardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), "Bobble");
        compile_builder_to_token_definitions(
            builder,
            "Type: Creature — Goblin\nPower/Toughness: 1/1\nHaste\nWhenever this token attacks, you gain 1 life.",
        )
        .expect("token compiles")
        .0
    }

    fn maker(text: &str) -> ironsmith::cards::CardDefinition {
        let builder =
            CompilerCardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), "Bobble Maker");
        with_custom_tokens(vec![bobble()], || {
            compile_builder_to_runtime_definition(
                builder,
                format!("Mana cost: {{2}}\nType: Sorcery\n{text}"),
                false,
            )
        })
        .expect("card compiles")
    }

    fn created_tokens(definition: &ironsmith::cards::CardDefinition) -> Vec<ironsmith::cards::CardDefinition> {
        fn walk(effect: &ironsmith::effect::Effect, found: &mut Vec<ironsmith::cards::CardDefinition>) {
            effect.visit_card_definitions(&mut |token| found.push(token.clone()));
            effect.visit_child_effects(&mut |child| walk(child, found));
        }
        let mut found = Vec::new();
        for effect in definition.spell_effect.iter().flatten() {
            walk(effect, &mut found);
        }
        found
    }

    #[test]
    fn named_reference_creates_the_custom_token() {
        let definition = maker("Create two 1/1 colorless creature tokens named Bobble.");
        let tokens = created_tokens(&definition);
        assert_eq!(tokens.len(), 1);
        let token = &tokens[0];
        assert!(token.card.is_token);
        assert_eq!(token.card.name, "Bobble");
        assert_eq!(token.abilities.len(), 2, "haste and the attack trigger");
        assert!(token.card.subtypes.iter().any(|subtype| format!("{subtype:?}") == "Goblin"));
    }

    #[test]
    fn unscoped_and_predefined_tokens_are_unchanged() {
        let builder =
            CompilerCardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), "Plain Maker");
        let plain = compile_builder_to_runtime_definition(
            builder,
            "Mana cost: {2}\nType: Sorcery\nCreate two 1/1 colorless creature tokens named Bobble.",
            false,
        )
        .expect("card compiles");
        assert!(created_tokens(&plain)[0].abilities.is_empty());
        let treasure = maker("Create a Treasure token.");
        assert!(created_tokens(&treasure)[0].card.name.starts_with("Treasure"));
    }
}
