//! Take-the-initiative effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

use super::venture_into_dungeon::advance_player_dungeon;

#[derive(Debug, Clone, PartialEq)]
pub struct TakeInitiativeEffect {
    pub player: PlayerFilter,
}

impl TakeInitiativeEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for TakeInitiativeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        game.set_initiative(Some(player_id));
        advance_player_dungeon(game, ctx, player_id, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn take_initiative_sets_designation_and_starts_undercity() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(702);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        TakeInitiativeEffect::you()
            .execute(&mut game, &mut ctx)
            .expect("take initiative should resolve");

        assert_eq!(game.initiative, Some(alice));
        let progress = game
            .active_dungeon(alice)
            .expect("taking initiative should start undercity");
        assert_eq!(progress.dungeon_name, "Undercity");
        assert_eq!(progress.room_name, "Secret Entrance");
    }
}
