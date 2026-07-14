//! Earthbend effect implementation.

use crate::continuous::{EffectSourceType, EffectTarget, Modification, PtSublayer};
use crate::effect::{Effect, EffectOutcome, Until, Value};
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::{
    ApplyContinuousEffect, EffectExecutor, PutCountersEffect, ScheduleDelayedTriggerEffect,
    TargetReusePolicy,
};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;
use crate::triggers::Trigger;
use crate::triggers::TriggerEvent;
use crate::types::CardType;

/// Earthbend effect: make target land a 0/0 creature with haste, put counters,
/// and return it tapped if it dies or is exiled.
pub type EarthbendEffect = ironsmith_core::EarthbendEffect;

impl EffectExecutor for EarthbendEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;
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
        let counters_outcome =
            ctx.with_temp_targets(vec![ResolvedTarget::Object(target_id)], |ctx| {
                let counters_effect = PutCountersEffect::new(
                    CounterType::PlusOnePlusOne,
                    self.counters,
                    ChooseSpec::AnyTarget,
                );
                execute_effect(game, &Effect::new(counters_effect), ctx)
            })?;
        events.extend(counters_outcome.events);

        let schedule = ScheduleDelayedTriggerEffect::new(
            Trigger::this_dies_or_is_exiled(),
            vec![Effect::return_from_graveyard_or_exile_to_battlefield(true)],
            true,
            vec![target_id],
            crate::target::PlayerFilter::Specific(ctx.controller),
        );
        let _ = execute_effect(game, &Effect::new(schedule), ctx)?;

        events.push(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(
                KeywordActionKind::Earthbend,
                ctx.controller,
                ctx.source,
                self.counters,
            ),
            ctx.provenance,
        ));

        Ok(EffectOutcome::resolved().with_events(events))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_reuse_policy(&self) -> TargetReusePolicy {
        TargetReusePolicy::AlwaysDeclareNew
    }

    fn target_description(&self) -> &'static str {
        "target land you control"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn targeted_earthbend_puts_counters_on_selected_land() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(881_001), "Awaken Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let land = game.create_object_from_definition(
            &crate::cards::definitions::basic_plains(),
            alice,
            Zone::Battlefield,
        );

        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
        let effect = EarthbendEffect::new(target, 4);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(land)]);

        effect
            .execute(&mut game, &mut ctx)
            .expect("targeted earthbend should resolve");

        assert_eq!(
            game.counter_count(land, CounterType::PlusOnePlusOne),
            4,
            "earthbend should put counters on the chosen land"
        );
        assert_eq!(game.calculated_power(land), Some(4));
        assert_eq!(game.calculated_toughness(land), Some(4));
    }
}
