//! Take-the-initiative effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::triggers::TriggerEvent;

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
        // CR 725.2: "Whenever a player takes the initiative, that player
        // ventures into Undercity" is an inherent triggered ability, so the
        // venture waits for the stack instead of happening mid-resolution.
        // Retaking the initiative triggers it again (CR 725.5).
        Ok(EffectOutcome::resolved().with_event(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::TakeInitiative, player_id, ctx.source, 1),
            ctx.provenance,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn take_initiative_sets_designation_and_queues_venture_trigger_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(702);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let outcome = TakeInitiativeEffect::you()
            .execute(&mut game, &mut ctx)
            .expect("take initiative should resolve");

        assert_eq!(game.initiative, Some(alice));
        // The Undercity venture is a triggered ability (CR 725.2), not part
        // of this effect's resolution.
        assert!(game.active_dungeon(alice).is_none());
        assert!(outcome.events.iter().any(|event| event
            .downcast::<KeywordActionEvent>()
            .is_some_and(|event| event.action == KeywordActionKind::TakeInitiative
                && event.player == alice)));
    }
}
