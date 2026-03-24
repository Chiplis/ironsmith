use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, RegisterZoneReplacementEffect};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;

/// Execute an effect while temporary replacement effects are scoped to that execution.
///
/// This models self-replacement patterns like "Counter target spell. If that spell is
/// countered this way, exile it instead..." where the replacement applies only to the
/// event caused by the antecedent effect.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalRewriteEffect {
    pub effect: Box<Effect>,
    pub zone_replacements: Vec<RegisterZoneReplacementEffect>,
}

impl LocalRewriteEffect {
    pub fn new(effect: Effect, zone_replacements: Vec<RegisterZoneReplacementEffect>) -> Self {
        Self {
            effect: Box::new(effect),
            zone_replacements,
        }
    }
}

impl EffectExecutor for LocalRewriteEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut replacements = Vec::new();
        for replacement in &self.zone_replacements {
            replacements.extend(replacement.resolve_replacements(game, ctx)?);
        }

        ctx.with_temp_additional_replacement_effects(replacements, |ctx| {
            execute_effect(game, &self.effect, ctx)
        })
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        self.effect.0.get_target_spec()
    }

    fn target_description(&self) -> &'static str {
        self.effect.0.target_description()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        self.effect.0.get_target_count()
    }
}
