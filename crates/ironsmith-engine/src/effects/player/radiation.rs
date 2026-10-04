//! Resolution of the inherent triggered ability associated with rad counters (CR 728.1).

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, MillEffect};
use crate::events::LifeLossEvent;
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
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let checkpoint = game.clone();
        let result = self.resolve(game, ctx);
        if ctx.decision_maker.awaiting_choice() || result.is_err() {
            *game = checkpoint;
        }
        result
    }
}

impl RadiationEffect {
    fn resolve(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = ctx.controller;
        let rad_count = game
            .player(player)
            .map_or(0, |player| player.counter_count(CounterType::Rad));
        if rad_count == 0 {
            return Ok(EffectOutcome::resolved());
        }

        let outcome =
            MillEffect::new(rad_count, PlayerFilter::Specific(player)).execute(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let nonland_cards_milled = outcome.affected_object_memory().map_or(0, |memory| {
            memory
                .iter()
                .filter(|card| !card.card_types.contains(&CardType::Land))
                .count()
        });

        let milled_summary = outcome.value.clone();
        let mut outcomes = vec![outcome];
        for _ in 0..nonland_cards_milled {
            // CR 614.1a: the radiation life loss is a life-loss event.
            let mut loss = crate::effects::life::life_change::execute_life_change(
                game,
                ctx,
                crate::events::Event::new_with_provenance(
                    LifeLossEvent::from_radiation(player, 1), ctx.provenance,
                ),
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if let Some((_, event)) =
                game.remove_player_counters_with_source(player, CounterType::Rad, 1, None, None)
            {
                loss.events.push(event);
            }
            outcomes.push(loss);
        }

        let mut outcome = EffectOutcome::aggregate(outcomes);
        outcome.value = milled_summary;
        Ok(outcome)
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
