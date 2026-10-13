//! Register a permission for exact current object incarnations.
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::Grantable;
use crate::grant_registry::GrantSource;
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;

#[derive(Debug, Clone)]
pub(crate) struct GrantExactObjects {
    pub objects: Vec<ObjectId>,
    pub zone: Zone,
    pub player: PlayerId,
    pub permission: Grantable,
    pub source: GrantSource,
    pub starts_on_turn: Option<u32>,
}
impl EffectExecutor for GrantExactObjects {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Deferred
    }
    fn execute(
        &self,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut granted = 0;
        for &object in &self.objects {
            if !game
                .object(object)
                .is_some_and(|object| object.zone == self.zone)
            {
                continue;
            }
            if let Some(turn) = self.starts_on_turn {
                game.effect_store
                    .grant_registry
                    .grant_to_card_starting_on_turn(
                        object,
                        self.zone,
                        self.player,
                        self.permission.clone(),
                        turn,
                        self.source.clone(),
                    );
            } else {
                game.effect_store.grant_registry.grant_to_card(
                    object,
                    self.zone,
                    self.player,
                    self.permission.clone(),
                    self.source.clone(),
                );
            }
            granted += 1;
        }
        Ok(EffectOutcome::count(granted))
    }
}
