use super::*;

pub(super) fn try_compile_continuous_and_modifier_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let _ = (effect, ctx);
    Ok(None)
}

pub(super) fn try_compile_player_turn_and_counter_effect(
    _effect: &EffectAst,
    _ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    Ok(None)
}
