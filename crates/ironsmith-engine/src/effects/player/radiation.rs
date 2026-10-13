//! Resolution of the inherent triggered ability associated with rad counters (CR 728.1).

use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError, MillEffect,
};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::types::CardType;

/// The sourceless game-rule effect associated with rad counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RadiationEffect;

impl RadiationEffect {
    pub const fn new() -> Self {
        Self
    }
}

impl EffectExecutor for RadiationEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        visitor(&Self::body(PlayerFilter::You, crate::effect::Value::PlayerCounters(PlayerFilter::You, CounterType::Rad)));
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| self.resolve_with_outputs(game, ctx),
        )
    }
}

impl RadiationEffect {
    fn body(player: PlayerFilter, count: crate::effect::Value) -> crate::effect::Effect {
        use crate::effect::{Effect, EffectId, Value};
        use ironsmith_core::{EffectMetric, EffectMetricSource, PriorEffectMetricQuery, PriorEffectAction};
        const MILLED: EffectId = EffectId(0xFFFF_FD01);
        let mut query = PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects, EffectMetric::Count)
            .with_filter(crate::target::ObjectFilter::default().without_type(CardType::Land));
        query.action = Some(PriorEffectAction::Milled);
        let count_milled = Value::PriorEffectMetric { effect_id: MILLED, query };
        Effect::new(crate::effects::composition::ScopedOutcomeProgram {
            id: MILLED, preserve_producer_result: true,
            producer: Effect::new(MillEffect::new(count, player.clone())),
            continuation: vec![Effect::repeat_effects(count_milled, vec![
                Effect::new(crate::effects::composition::ResolvedLifeLoss {player: player.clone(), amount: 1, from_radiation: true}),
                Effect::new(crate::effects::composition::ResolvedPlayerCounterRemoval {
                    player, counter: CounterType::Rad, count: 1, source: None, controller: None,
                }),
            ])],
        })
    }
    fn resolve_with_outputs(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<CompletedEffectOutputs, ExecutionError> {
        let count = game.player(ctx.controller).map_or(0, |player| player.counter_count(CounterType::Rad));
        if count == 0 { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved())); }
        Self::body(PlayerFilter::Specific(ctx.controller), count.into()).0.execute_child_with_outputs(game, ctx)
    }
}

#[cfg(test)]
mod unsigned_radiation_and_mill_public_contract_tests {
use crate::{GameState,PlayerId,Zone,CardId,Effect};
use crate::effects::{EffectContext,RadiationEffect,MillEffect,execute_effect};
use crate::target::PlayerFilter;
use crate::object::CounterType;
fn setup(amount:u32)->(GameState,crate::ObjectId,PlayerId){let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let card=crate::cards::builders::CardDefinitionBuilder::new(CardId::new(),"Natural mill quantity owner").card_types(vec![crate::types::CardType::Artifact]).build();let source=game.create_object_from_definition(&card,alice,Zone::Battlefield);let card=crate::cards::builders::CardDefinitionBuilder::new(CardId::new(),"Nonland library object").card_types(vec![crate::types::CardType::Artifact]).build();game.create_object_from_definition(&card,alice,Zone::Library);let out=game.add_player_counters_with_source(alice,CounterType::Rad,amount,Some(source),Some(alice)).unwrap();assert_eq!(out.as_count(),Some(i64::from(amount)));assert_eq!(game.player(alice).unwrap().library.len(),1);(game,source,alice)}
fn radiation(amount:u32){let (mut game,source,alice)=setup(amount);let mut ctx=EffectContext::new_default(source,alice);let out=execute_effect(&mut game,&Effect::new(RadiationEffect::new()),&mut ctx).unwrap();assert!(game.player(alice).unwrap().library.is_empty(),"radiation must retain its unsigned mill instruction");assert_eq!(game.player(alice).unwrap().graveyard.len(),1);assert_eq!(game.player(alice).unwrap().life,19);assert_eq!(game.player(alice).unwrap().counter_count(CounterType::Rad),amount-1);assert_eq!(out.affected_object_memory().unwrap().len(),1);}
#[test] fn bounded_radiation_control(){radiation(7);}
#[test] fn unsigned_radiation_instruction_reaches_actual_library(){radiation(u32::MAX);}
#[test] fn unsigned_direct_mill_caps_to_actual_library(){let (mut game,source,alice)=setup(0);let mut ctx=EffectContext::new_default(source,alice);execute_effect(&mut game,&Effect::new(MillEffect::new(u32::MAX,PlayerFilter::You)),&mut ctx).expect("natural mill quantity must cap to actual library before signed narrowing");assert!(game.player(alice).unwrap().library.is_empty());assert_eq!(game.player(alice).unwrap().graveyard.len(),1);}

}
