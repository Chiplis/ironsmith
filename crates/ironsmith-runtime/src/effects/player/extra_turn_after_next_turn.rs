//! Schedule an extra turn after a player's next turn.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::player::ExtraTurnEffect;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::triggers::Trigger;

use crate::effects::delayed::trigger_queue::{
    DelayedTriggerTemplate, DelayedWatcherIdentity, queue_delayed_from_template,
};

/// Effect that schedules a player's extra turn after that player's next turn.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtraTurnAfterNextTurnEffect {
    /// The player who gets the extra turn.
    pub player: PlayerFilter,
}

impl ExtraTurnAfterNextTurnEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl EffectExecutor for ExtraTurnAfterNextTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let delayed = DelayedTriggerTemplate::new(
            Trigger::beginning_of_end_step(PlayerFilter::Specific(player_id)),
            vec![Effect::new(ExtraTurnEffect::new(PlayerFilter::Specific(
                player_id,
            )))],
            true,
            ctx.controller,
        )
        .with_not_before_turn(Some(game.turn.turn_number.saturating_add(1)))
        .with_ability_source(Some(ctx.source));
        queue_delayed_from_template(game, DelayedWatcherIdentity::combined(Vec::new()), delayed);

        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::phase::BeginningOfEndStepEvent;
    use crate::game_loop::{put_triggers_on_stack, resolve_stack_entry};
    use crate::ids::PlayerId;
    use crate::triggers::{TriggerEvent, TriggerQueue, check_delayed_triggers};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn extra_turn_after_next_turn_waits_for_target_players_end_step() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExtraTurnAfterNextTurnEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(
            game.turn_store.extra_turns.is_empty(),
            "the extra turn should not be queued immediately"
        );
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);

        game.next_turn();
        assert_eq!(
            game.turn.active_player, bob,
            "Bob should take their normal turn"
        );
        assert!(
            game.turn_store.extra_turns.is_empty(),
            "the extra turn should still wait until Bob's end step"
        );

        let event = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in check_delayed_triggers(&mut game, &event) {
            trigger_queue.add(trigger);
        }
        assert_eq!(trigger_queue.entries.len(), 1);

        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("delayed trigger should go on stack");
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry(&mut game).expect("delayed trigger should resolve");

        assert_eq!(
            game.turn_store.extra_turns,
            vec![bob],
            "Bob should receive an extra turn after their current turn"
        );

        game.cleanup_player_control_end_of_turn();
        game.next_turn();
        assert_eq!(
            game.turn.active_player, bob,
            "Bob should take the queued extra turn immediately after their turn ends"
        );
    }
}
