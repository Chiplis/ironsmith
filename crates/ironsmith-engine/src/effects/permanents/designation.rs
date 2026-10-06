//! Permanent designations, independent of the actions that precede them.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, Copy)]
pub(super) enum PermanentDesignation {
    Monstrous,
    Renowned,
}

impl PermanentDesignation {
    pub(super) fn is_present(self, game: &GameState, object: ObjectId) -> bool {
        match self {
            Self::Monstrous => game.is_monstrous(object),
            Self::Renowned => game.is_renowned(object),
        }
    }
}

#[derive(Debug, Clone)]
struct ApplyDesignation {
    object: ObjectId,
    designation: PermanentDesignation,
    amount: u32,
}

impl EffectExecutor for ApplyDesignation {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            if ctx.decision_maker.awaiting_choice()
                || !game
                    .object(self.object)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                || game.is_phased_out(self.object)
                || self.designation.is_present(game, self.object)
            {
                return Ok(EffectOutcome::count(0));
            }
            let (label, outcome) = match self.designation {
                PermanentDesignation::Monstrous => {
                    game.set_monstrous(self.object);
                    let completion = crate::effects::observe_action_completion(
                        game,
                        TriggerEvent::new_with_provenance(
                            crate::events::BecameMonstrousEvent::new(
                                self.object,
                                ctx.controller,
                                self.amount,
                            ),
                            ctx.provenance,
                        ),
                        Some(ctx.provenance),
                    )?;
                    (
                        "monstrous",
                        EffectOutcome::monstrosity_applied(self.object, self.amount)
                            .with_event(completion),
                    )
                }
                PermanentDesignation::Renowned => {
                    game.set_renowned(self.object);
                    (
                        "renown",
                        crate::effects::composition::complete_keyword_action_with_result(
                            game,
                            ctx,
                            EffectOutcome::count(1),
                            KeywordActionEvent::new(
                                KeywordActionKind::Renown,
                                ctx.controller,
                                self.object,
                                self.amount,
                            ),
                        )?,
                    )
                }
            };
            if let Some(stable_id) = game.object(self.object).map(|object| object.stable_id) {
                game.record_ui_effect_event(
                    "level_up",
                    Some(ctx.controller),
                    None,
                    vec![stable_id],
                    Some(i64::from(self.amount)),
                    Some(label.into()),
                );
            }
            Ok(outcome)
        })
    }
}

pub(super) fn apply_designation(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object: ObjectId,
    designation: PermanentDesignation,
    amount: u32,
) -> Result<EffectOutcome, ExecutionError> {
    ApplyDesignation {
        object,
        designation,
        amount,
    }
    .execute_child(game, ctx)
}
