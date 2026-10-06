//! Take-the-initiative effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

use crate::events::{KeywordActionEvent, KeywordActionKind};

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
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            let player_id = resolve_player_filter(game, &self.player, ctx)?;
            game.set_initiative(Some(player_id));
            // Initiative's inherent venture trigger waits for the stack.
            // Retaking the initiative still completes this action.
            crate::effects::composition::complete_keyword_action(
                game,
                ctx,
                KeywordActionEvent::new(
                    KeywordActionKind::TakeInitiative,
                    player_id,
                    ctx.source,
                    1,
                ),
            )
        })
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
        assert!(outcome.events.iter().any(|event| {
            event.downcast::<KeywordActionEvent>().is_some_and(|event| {
                event.action == KeywordActionKind::TakeInitiative && event.player == alice
            })
        }));
    }
}
