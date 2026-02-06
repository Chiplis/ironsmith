//! Earthbend effect implementation.

use crate::continuous::{EffectSourceType, EffectTarget, Modification, PtSublayer};
use crate::effect::{Effect, EffectOutcome, Until, Value};
use crate::effects::{
    ApplyContinuousEffect, EffectExecutor, PutCountersEffect, ScheduleDelayedTriggerEffect,
};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;
use crate::triggers::Trigger;
use crate::triggers::TriggerEvent;
use crate::types::CardType;

/// Earthbend effect: make target land a 0/0 creature with haste, put counters,
/// and return it tapped if it dies or is exiled.
#[derive(Debug, Clone, PartialEq)]
pub struct EarthbendEffect {
    pub target: ChooseSpec,
    pub counters: u32,
}

impl EarthbendEffect {
    pub fn new(target: ChooseSpec, counters: u32) -> Self {
        Self { target, counters }
    }
}

impl EffectExecutor for EarthbendEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::effects::helpers::find_target_object;

        let target_id = find_target_object(&ctx.targets)?;
        let locked_targets = vec![target_id];

        let base_effect = ApplyContinuousEffect::new(
            EffectTarget::Specific(target_id),
            Modification::AddCardTypes(vec![CardType::Creature]),
            Until::Forever,
        )
        .with_source_type(EffectSourceType::Resolution {
            locked_targets: locked_targets.clone(),
        });

        let pt_effect = ApplyContinuousEffect::new(
            EffectTarget::Specific(target_id),
            Modification::SetPowerToughness {
                power: Value::Fixed(0),
                toughness: Value::Fixed(0),
                sublayer: PtSublayer::Setting,
            },
            Until::Forever,
        )
        .with_source_type(EffectSourceType::Resolution {
            locked_targets: locked_targets.clone(),
        });

        let haste_effect = ApplyContinuousEffect::new(
            EffectTarget::Specific(target_id),
            Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
            Until::Forever,
        )
        .with_source_type(EffectSourceType::Resolution {
            locked_targets: locked_targets.clone(),
        });

        let _ = execute_effect(game, &Effect::new(base_effect), ctx)?;
        let _ = execute_effect(game, &Effect::new(pt_effect), ctx)?;
        let _ = execute_effect(game, &Effect::new(haste_effect), ctx)?;

        let mut events = Vec::new();
        let original_targets = ctx.targets.clone();
        ctx.targets = vec![ResolvedTarget::Object(target_id)];
        let counters_effect = PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            self.counters,
            ChooseSpec::AnyTarget,
        );
        let counters_outcome = execute_effect(game, &Effect::new(counters_effect), ctx)?;
        if let crate::effect::EffectResult::Count(count) = counters_outcome.result
            && count > 0
        {
            game.continuous_effects.record_counter_change(target_id);
        }
        events.extend(counters_outcome.events);
        ctx.targets = original_targets;

        let schedule = ScheduleDelayedTriggerEffect::new(
            Trigger::this_dies_or_is_exiled(),
            vec![Effect::return_from_graveyard_or_exile_to_battlefield(true)],
            true,
            vec![target_id],
            crate::target::PlayerFilter::Specific(ctx.controller),
        );
        let _ = execute_effect(game, &Effect::new(schedule), ctx)?;

        events.push(TriggerEvent::new(KeywordActionEvent::new(
            KeywordActionKind::Earthbend,
            ctx.controller,
            ctx.source,
            self.counters,
        )));

        Ok(EffectOutcome::resolved().with_events(events))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target land you control"
    }
}
