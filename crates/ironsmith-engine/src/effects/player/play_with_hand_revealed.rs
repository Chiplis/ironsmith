//! "You may have defending player play with their hand revealed for as long
//! as this creature remains on the battlefield." (Stromgald Spy)

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_players_from_spec;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

pub type PlayWithHandRevealedEffect = ironsmith_core::PlayWithHandRevealedEffect;

impl EffectExecutor for PlayWithHandRevealedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let while_source = matches!(
            self.duration,
            ironsmith_core::HandRevealDuration::WhileSourceOnBattlefield
        );
        // A source-bound reveal lasts while the source stays on the
        // battlefield as this object; a source that already left establishes
        // nothing.
        if while_source && !game.battlefield.contains(&ctx.source) {
            return Ok(EffectOutcome::count(0));
        }
        let players = resolve_players_from_spec(game, &ChooseSpec::EachPlayer(self.player.clone()), ctx)?;
        for player in &players {
            if while_source {
                game.reveal_hand_while_source_on_battlefield(*player, ctx.source);
            } else {
                game.reveal_hand_this_turn(*player);
            }
        }
        Ok(EffectOutcome::count(players.len() as i32))
    }
}
