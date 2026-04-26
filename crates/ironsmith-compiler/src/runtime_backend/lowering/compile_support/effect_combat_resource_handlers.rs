use super::*;

pub(super) fn try_compile_combat_and_damage_effect(
    _effect: &EffectAst,
    _ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    Ok(None)
}

pub(super) fn try_compile_board_state_effect(
    _effect: &EffectAst,
    _ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    Ok(None)
}

pub(super) fn try_compile_player_resource_and_choice_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let _ = (effect, ctx);
    Ok(None)
}
