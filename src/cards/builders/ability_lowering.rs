use super::*;

pub(crate) fn lower_activation_sentence_effects(
    effects_ast: &[EffectAst],
) -> Result<Vec<Effect>, CardTextError> {
    compile_statement_effects(effects_ast)
}

pub(crate) fn lower_activation_primary_mana_effect(
    mana_ast: &EffectAst,
) -> Result<Vec<Effect>, CardTextError> {
    let mut compile_ctx = CompileContext::new();
    let (effects, choices) = compile_effect(mana_ast, &mut compile_ctx)?;
    if !choices.is_empty() {
        return Err(CardTextError::ParseError(
            "unsupported target choice in mana ability".to_string(),
        ));
    }
    Ok(effects)
}

pub(crate) fn lower_activated_ability_effects_seeded(
    effects_ast: &[EffectAst],
    seed_last_object_tag: Option<String>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    compile_trigger_effects_seeded(None, effects_ast, seed_last_object_tag)
}
