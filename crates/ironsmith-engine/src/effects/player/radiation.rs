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
            MillEffect::new(rad_count as i32, PlayerFilter::Specific(player)).execute(game, ctx)?;
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
