//! Shuffle library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::ShuffleLibraryEvent;
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::ShuffleLibraryEffect;

#[derive(Debug, Clone)]
struct ShuffleLibraryAction {
    player: crate::ids::PlayerId,
    // Internal insertion order, bottom-to-top. These cards are excluded from
    // randomization and restored at the original instruction's boundary.
    retained: Vec<crate::ids::ObjectId>,
    position_from_top: usize,
    reason: String,
}

impl EffectExecutor for ShuffleLibraryAction {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::resolved());
        }
        if game.player(self.player).is_none() {
            return Err(ExecutionError::PlayerNotFound(self.player));
        }
        game.shuffle_library_except_then_insert_from_top(
            self.player,
            &self.retained,
            self.position_from_top,
            &self.reason,
        );
        Ok(
            EffectOutcome::resolved().with_event(TriggerEvent::new_with_provenance(
                ShuffleLibraryEvent::new(self.player, ctx.cause.clone()),
                ctx.provenance,
            )),
        )
    }
}

/// One owner for randomization and its completion observation, including
/// search instructions that retain selected cards outside the shuffled set.
pub(crate) fn shuffle_library(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: crate::ids::PlayerId,
    retained: &[crate::ids::ObjectId],
    position_from_top: usize,
    reason: &str,
) -> Result<EffectOutcome, ExecutionError> {
    crate::effects::execute_effect(
        game,
        &shuffle_library_action(player, retained, position_from_top, reason),
        ctx,
    )
}

/// Compose the existing shuffle owner as an actual child instruction, keeping
/// the selected player, retained insertion ordering and authored reason.
pub(crate) fn shuffle_library_action(
    player: crate::ids::PlayerId,
    retained: &[crate::ids::ObjectId],
    position_from_top: usize,
    reason: &str,
) -> crate::effect::Effect {
    crate::effect::Effect::new(ShuffleLibraryAction {
        player,
        retained: retained.to_vec(),
        position_from_top,
        reason: reason.into(),
    })
}

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
    fn supports_simultaneous_player_action(&self) -> bool { true }

    fn prepare_simultaneous_player_action(&self, game: &GameState, ctx: &mut ExecutionContext)
        -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(ShuffleProposal { player: resolve_player_filter(game, &self.player, ctx)? }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        shuffle_library(game, ctx, player_id, &[], 1, "library shuffled")
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target_spec.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to shuffle"
    }
}

/// Shuffling asks no choices and runs no replacement-added programs. Resolve
/// the player before any member of a simultaneous batch changes the world.
#[derive(Debug)]
struct ShuffleProposal { player: crate::ids::PlayerId }
impl crate::effects::SimultaneousEffectProposal for ShuffleProposal {
    fn commit(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<EffectOutcome, ExecutionError> {
        ShuffleLibraryEffect::new(crate::target::PlayerFilter::Specific(self.player)).execute(game, ctx)
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
