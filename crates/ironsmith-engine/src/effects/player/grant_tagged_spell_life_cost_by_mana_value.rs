//! Legacy life-price surface expanded to the shared tagged casting permission.

use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, GrantPlayTaggedEffect};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

pub type GrantTaggedSpellLifeCostByManaValueEffect =
    ironsmith_core::GrantTaggedSpellLifeCostByManaValueEffect;

fn program(effect: &GrantTaggedSpellLifeCostByManaValueEffect) -> GrantPlayTaggedEffect {
    let payment = crate::costs::Cost::try_from_runtime_effect(Effect::new(
        crate::effects::LoseLifeEffect::you(Value::ManaValueOf(Box::new(ChooseSpec::Source))),
    ))
    .expect("life payment is cost-capable");
    GrantPlayTaggedEffect::new(
        effect.tag.clone(),
        effect.player.clone(),
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
        false,
        false,
    )
    .with_filter(ObjectFilter::default().in_zone(Zone::Exile))
    .with_alternative_cost(crate::cost::TotalCost::from_costs(vec![payment]))
}

impl EffectExecutor for GrantTaggedSpellLifeCostByManaValueEffect {
    fn canonical_program(&self) -> Option<Effect> {
        Some(Effect::new(program(self)))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        program(self).execute_child(game, ctx)
    }
}
