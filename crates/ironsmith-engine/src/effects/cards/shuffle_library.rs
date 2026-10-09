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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        }
        if game.player(self.player).is_none() {
            return Err(ExecutionError::PlayerNotFound(self.player));
        }
        Ok(commit_library_shuffle_with_outputs(
            game,
            self.player,
            &self.retained,
            self.position_from_top,
            &self.reason,
            ctx.cause.clone(),
            |_| ctx.provenance,
        ))
    }
}

/// Commit one shuffle original and its observation. The enclosing instruction
/// owns player validation and batching; provenance is supplied after the RNG
/// operation so native compound callers retain their existing event ordering.
#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_library_shuffle(
    game: &mut GameState,
    player: crate::ids::PlayerId,
    retained: &[crate::ids::ObjectId],
    position_from_top: usize,
    reason: &str,
    cause: crate::events::cause::EventCause,
    provenance: impl FnOnce(&mut GameState) -> crate::provenance::ProvNodeId,
) -> EffectOutcome {
    commit_library_shuffle_with_outputs(
        game,
        player,
        retained,
        position_from_top,
        reason,
        cause,
        provenance,
    )
    .into_outcome()
}

/// The native randomization owner returns its actual original completion.
/// Scalar and recorded callers project this packet without replaying RNG.
#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_library_shuffle_with_outputs(
    game: &mut GameState,
    player: crate::ids::PlayerId,
    retained: &[crate::ids::ObjectId],
    position_from_top: usize,
    reason: &str,
    cause: crate::events::cause::EventCause,
    provenance: impl FnOnce(&mut GameState) -> crate::provenance::ProvNodeId,
) -> crate::effects::CompletedEffectOutputs {
    if retained.is_empty() {
        game.shuffle_player_library(player);
    } else {
        game.shuffle_library_except_then_insert_from_top(
            player,
            retained,
            position_from_top,
            reason,
        );
    }
    let provenance = provenance(game);
    crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved().with_event(
        TriggerEvent::new_with_provenance(ShuffleLibraryEvent::new(player, cause), provenance),
    ))
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
    shuffle_library_with_outputs(game, ctx, player, retained, position_from_top, reason)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Preserve the actual shuffle packet through the same semantic owner.
pub(crate) fn shuffle_library_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: crate::ids::PlayerId,
    retained: &[crate::ids::ObjectId],
    position_from_top: usize,
    reason: &str,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::execute_effect_with_outputs(
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
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(ShuffleProposal {
            player: resolve_player_filter(game, &self.player, ctx)?,
        }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        // Multiple moved objects may share an owner: shuffle each owner once.
        if let crate::target::PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(tag)) = &self.player
            && let Some(owners) = distinct_tagged_owners(game, ctx, tag)
        {
            let mut children = Vec::with_capacity(owners.len());
            for owner in owners {
                children.push(shuffle_library_with_outputs(game, ctx, owner, &[], 1, "library shuffled")?);
            }
            return Ok(crate::effects::CompletedEffectOutputs::from_children(children, EffectOutcome::aggregate));
        }
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        shuffle_library_with_outputs(game, ctx, player_id, &[], 1, "library shuffled")
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target_spec.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to shuffle"
    }
}

/// The distinct owners of a tag holding more than one object, in APNAP order
/// (CR 101.4). `None` for an empty or single-object tag, which keeps the
/// ordinary single-player resolution.
fn distinct_tagged_owners(
    game: &GameState,
    ctx: &ExecutionContext,
    tag: &crate::tag::TagKey,
) -> Option<Vec<crate::ids::PlayerId>> {
    let snapshots = ctx.get_tagged_all(tag)?;
    if snapshots.len() < 2 {
        return None;
    }
    let mut owners = Vec::new();
    for player in game.team_apnap_player_order() {
        if snapshots.iter().any(|snapshot| snapshot.owner == player) && !owners.contains(&player) {
            owners.push(player);
        }
    }
    for snapshot in snapshots {
        if !owners.contains(&snapshot.owner) {
            owners.push(snapshot.owner);
        }
    }
    Some(owners)
}

/// Shuffling asks no choices and runs no replacement-added programs. Resolve
/// the player before any member of a simultaneous batch changes the world.
#[derive(Debug)]
struct ShuffleProposal {
    player: crate::ids::PlayerId,
}
impl crate::effects::SimultaneousEffectProposal for ShuffleProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(|receipt| receipt.outcome.into_outcome())
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        ShuffleLibraryEffect::new(crate::target::PlayerFilter::Specific(self.player))
            .execute_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::finished)
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
