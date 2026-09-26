//! Resolution of the inherent triggered ability associated with rad counters (CR 728.1).

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, MillEffect};
use crate::events::LifeLossEvent;
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
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
        let player = ctx.controller;
        let rad_count = game
            .player(player)
            .map_or(0, |player| player.counter_count(CounterType::Rad));
        if rad_count == 0 {
            return Ok(EffectOutcome::resolved());
        }

        let mut outcome =
            MillEffect::new(rad_count as i32, PlayerFilter::Specific(player)).execute(game, ctx)?;
        let nonland_cards_milled = outcome.affected_object_memory().map_or(0, |memory| {
            memory
                .iter()
                .filter(|card| !card.card_types.contains(&CardType::Land))
                .count()
        });

        for _ in 0..nonland_cards_milled {
            // CR 614.1a: the radiation life loss is a life-loss event.
            let amount =
                crate::events::processing::process_life_loss_with_event(game, player, 1, false);
            let lost = game.lose_life(player, amount);
            if lost > 0 {
                outcome.events.push(TriggerEvent::new_with_provenance(
                    LifeLossEvent::from_radiation(player, 1).with_amount(lost),
                    ctx.provenance,
                ));
            }
            if let Some((_, event)) =
                game.remove_player_counters_with_source(player, CounterType::Rad, 1, None, None)
            {
                outcome.events.push(event);
            }
        }

        Ok(outcome)
    }
}
