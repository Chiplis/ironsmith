//! Grant a temporary mana ability to a player until end of turn.
//!
//! This models effects like Channel:
//! "Until end of turn, any time you could activate a mana ability, you may pay 1 life.
//! If you do, add {C}."

use crate::ability::ActivatedAbility;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, GrantedManaAbility};

#[derive(Debug, Clone, PartialEq)]
pub struct GrantManaAbilityUntilEotEffect {
    pub ability: ActivatedAbility,
}

impl GrantManaAbilityUntilEotEffect {
    pub fn new(ability: ActivatedAbility) -> Self {
        Self { ability }
    }
}

impl EffectExecutor for GrantManaAbilityUntilEotEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let expires_end_of_turn = game.turn.turn_number;
        game.granted_mana_abilities.push(GrantedManaAbility {
            controller: ctx.controller,
            ability: self.ability.clone(),
            expires_end_of_turn,
        });
        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
