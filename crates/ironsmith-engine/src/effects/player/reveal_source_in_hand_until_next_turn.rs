//! "Until that player's next turn, that player plays with that card revealed
//! in their hand and can't play it." (Firestorm Phoenix)

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::zone::Zone;

pub type RevealSourceInHandUntilOwnersNextTurnEffect =
    ironsmith_core::RevealSourceInHandUntilOwnersNextTurnEffect;

impl EffectExecutor for RevealSourceInHandUntilOwnersNextTurnEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        // The source card was just returned to its owner's hand (a new object,
        // CR 400.7); follow its stable card identity to that hand object.
        let source = ctx.source;
        let stable = ctx
            .source_snapshot
            .as_ref()
            .filter(|snapshot| snapshot.object_id == source)
            .map(|snapshot| snapshot.stable_id)
            .or_else(|| game.object(source).map(|object| object.stable_id))
            // A zone-change replacement binds its moving object (here the
            // source itself) as "it".
            .or_else(|| {
                ctx.get_tagged_all("__it__").and_then(|snapshots| {
                    snapshots
                        .iter()
                        .find(|snapshot| snapshot.object_id == source)
                        .map(|snapshot| snapshot.stable_id)
                })
            });
        let Some(card) = stable.and_then(|stable| game.find_object_by_stable_id(stable)) else {
            return Ok(EffectOutcome::count(0));
        };
        let Some(owner) = game
            .object(card)
            .filter(|object| object.zone == Zone::Hand)
            .map(|object| object.owner)
        else {
            return Ok(EffectOutcome::count(0));
        };
        game.reveal_hand_card_until_owners_next_turn(card, owner, self.cant_play);
        Ok(EffectOutcome::count(1).with_affected_objects(vec![card]))
    }
}
