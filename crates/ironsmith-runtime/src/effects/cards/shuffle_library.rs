//! Shuffle library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::ShuffleLibraryEvent;
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::TriggerEvent;
pub use ironsmith_core::ShuffleLibraryEffect;

/// Effect that shuffles a player's library.
///
/// # Fields
///
/// * `player` - Which player's library to shuffle
///
/// # Example
///
/// ```ignore
/// // Shuffle your library
/// let effect = ShuffleLibraryEffect::you();
/// ```
impl EffectExecutor for ShuffleLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        game.shuffle_player_library(player_id);

        Ok(
            EffectOutcome::resolved().with_event(TriggerEvent::new_with_provenance(
                ShuffleLibraryEvent::new(player_id, ctx.cause.clone()),
                ctx.provenance,
            )),
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target_spec.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to shuffle"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_library_card(game: &mut GameState, owner: PlayerId, name: &str) {
        let card = CardBuilder::new(CardId::new(), name).build();
        game.create_object_from_card(&card, owner, Zone::Library);
    }

    #[test]
    fn shuffle_library_emits_shuffle_event_for_singleton_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_library_card(&mut game, alice, "Only Card");
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ShuffleLibraryEffect::you()
            .execute(&mut game, &mut ctx)
            .expect("shuffle should resolve");

        assert!(
            outcome.events.iter().any(|event| event
                .downcast::<ShuffleLibraryEvent>()
                .is_some_and(|shuffle| { shuffle.player == alice })),
            "single-card library shuffles should still emit a shuffle event"
        );
    }

    #[test]
    fn shuffle_library_emits_shuffle_event_for_empty_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ShuffleLibraryEffect::you()
            .execute(&mut game, &mut ctx)
            .expect("shuffle should resolve");

        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.downcast::<ShuffleLibraryEvent>().is_some()),
            "empty-library shuffles should still emit a shuffle event"
        );
    }
}
