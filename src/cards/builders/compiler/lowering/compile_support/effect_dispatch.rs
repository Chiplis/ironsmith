use super::*;

type EffectCompileOutcome = (Vec<Effect>, Vec<ChooseSpec>);
type EffectCompileHandler = fn(
    &EffectAst,
    &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError>;

#[derive(Clone, Copy)]
struct EffectCompileHandlerDef {
    run: EffectCompileHandler,
}

const EFFECT_COMPILE_HANDLERS: [EffectCompileHandlerDef; 14] = [
    EffectCompileHandlerDef {
        run: effect_combat_resource_handlers::try_compile_combat_and_damage_effect,
    },
    EffectCompileHandlerDef {
        run: effect_combat_resource_handlers::try_compile_board_state_effect,
    },
    EffectCompileHandlerDef {
        run: effect_combat_resource_handlers::try_compile_player_resource_and_choice_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_timing_and_control_effect,
    },
    EffectCompileHandlerDef {
        run: effect_flow_search_handlers::try_compile_flow_and_iteration_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_destroy_and_exile_effect,
    },
    EffectCompileHandlerDef {
        run: effect_visibility_object_handlers::try_compile_visibility_and_card_selection_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_stack_and_condition_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_attachment_and_setup_effect,
    },
    EffectCompileHandlerDef {
        run: effect_flow_search_handlers::try_compile_token_generation_effect,
    },
    EffectCompileHandlerDef {
        run: effect_continuous_turn_handlers::try_compile_continuous_and_modifier_effect,
    },
    EffectCompileHandlerDef {
        run: effect_flow_search_handlers::try_compile_search_and_reorder_effect,
    },
    EffectCompileHandlerDef {
        run: effect_visibility_object_handlers::try_compile_object_zone_and_exchange_effect,
    },
    EffectCompileHandlerDef {
        run: effect_continuous_turn_handlers::try_compile_player_turn_and_counter_effect,
    },
];

pub(crate) fn compile_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    stacker::maybe_grow(1024 * 1024, 2 * 1024 * 1024, || {
        compile_effect_inner(effect, ctx)
    })
}

fn compile_effect_inner(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    if matches!(
        effect,
        EffectAst::RepeatThisProcess | EffectAst::RepeatThisProcessOnce
    ) {
        return Err(CardTextError::ParseError(
            "unsupported repeat this process effect tail".to_string(),
        ));
    }
    if let Some(compiled) = try_compile_effect_via_handlers(effect, ctx)? {
        return Ok(compiled);
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing compile-effect dispatch route for effect variant: {effect:?}"
    )))
}

fn try_compile_effect_via_handlers(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError> {
    for EffectCompileHandlerDef { run, .. } in EFFECT_COMPILE_HANDLERS {
        if let Some(compiled) = run(effect, ctx)? {
            return Ok(Some(compiled));
        }
    }
    Ok(None)
}
