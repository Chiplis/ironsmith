use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::ChooseSpec;

pub type RegisterManaRewriteEffect = ironsmith_core::RegisterManaRewriteEffect;

impl EffectExecutor for RegisterManaRewriteEffect {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        let mut rule = self.rule.clone();
        if let Some(target) = &self.target {
            let objects = crate::effects::helpers::resolve_objects_for_effect(game, ctx, target)?;
            let object = match objects.as_slice() {
                [] => return Ok(EffectOutcome::target_invalid()),
                [object] => *object,
                _ => return Err(ExecutionError::UnresolvableValue("mana rewrite target must identify one object".into())),
            };
            // "Target Plains" is selected/validated at announcement and
            // resolution. Its registration follows only this incarnation,
            // without re-testing the announcing noun on every later event.
            rule.source_filter = crate::target::ObjectFilter::specific(object);
        }
        if let Some(controller) = &rule.controller {
            rule.controller = Some(crate::target::PlayerFilter::Specific(
                crate::effects::helpers::resolve_player_filter(game, controller, ctx)?));
        }
        if rule.output == ironsmith_core::ManaRewriteOutput::ChosenColor {
            let color = game.chosen_color(ctx.source).ok_or_else(||
                ExecutionError::UnresolvableValue("mana replacement has no chosen color".into()))?;
            rule.output = ironsmith_core::ManaRewriteOutput::Symbol(crate::mana::ManaSymbol::from_color(color));
        }
        let replacement = ReplacementEffect::with_matcher(ctx.source, ctx.controller,
            crate::events::mana::matchers::ManaRewriteMatcher {rule: rule.clone()},
            ReplacementAction::RewriteMana {input: rule.input, output: rule.output, quantity: rule.quantity});
        ApplyReplacementEffect {effect: replacement, mode: self.mode}.execute(game, ctx)
    }
    fn get_target_spec(&self) -> Option<&ChooseSpec> { self.target.as_ref() }
    fn target_description(&self) -> &'static str { "mana-producing land" }
}
