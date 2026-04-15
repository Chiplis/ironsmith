//! Prompt a player to optionally move an object to a zone.

use crate::decision::FallbackStrategy;
use crate::decisions::ask_may_choice;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct MayMoveToZoneEffect {
    pub target: ChooseSpec,
    pub zone: Zone,
    pub decider: PlayerFilter,
}

impl MayMoveToZoneEffect {
    pub fn new(target: ChooseSpec, zone: Zone, decider: PlayerFilter) -> Self {
        Self {
            target,
            zone,
            decider,
        }
    }

    fn describe_move(&self, game: &GameState, object_id: crate::ids::ObjectId) -> String {
        let object_name = game
            .object(object_id)
            .map(|obj| obj.name.clone())
            .unwrap_or_else(|| "that card".to_string());
        match self.zone {
            Zone::Hand => format!("Put {object_name} into your hand?"),
            Zone::Exile => format!("Exile {object_name}?"),
            Zone::Graveyard => format!("Put {object_name} into its owner's graveyard?"),
            Zone::Library => format!("Put {object_name} into its owner's library?"),
            Zone::Battlefield => format!("Put {object_name} onto the battlefield?"),
            Zone::Command => format!("Put {object_name} into the command zone?"),
            Zone::Stack => format!("Move {object_name} to the stack?"),
        }
    }
}

impl EffectExecutor for MayMoveToZoneEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        let Some(object_id) = object_ids.first().copied() else {
            return Ok(EffectOutcome::count(0));
        };
        let decider = resolve_player_filter(game, &self.decider, ctx)?;
        let should_move = ask_may_choice(
            game,
            &mut ctx.decision_maker,
            decider,
            ctx.source,
            self.describe_move(game, object_id),
            FallbackStrategy::Decline,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if !should_move {
            return Ok(EffectOutcome::count(0).with_execution_fact(ExecutionFact::Declined));
        }

        let move_effect =
            crate::effects::MoveToZoneEffect::new(self.target.clone(), self.zone, false);
        move_effect.execute(game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to optionally move"
    }
}
