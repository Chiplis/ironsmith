use crate::decisions::context::SelectableOption;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter_to_list;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
pub type ChooseSpellCastHistoryEffect = ironsmith_core::ChooseSpellCastHistoryEffect;

impl EffectExecutor for ChooseSpellCastHistoryEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let caster_ids =
            resolve_player_filter_to_list(game, &self.cast_by, &ctx.filter_context(game), ctx)?;
        let filter_ctx = ctx.filter_context(game);
        let history = game.turn_store.turn_history.spell_cast_snapshot_history();
        let candidates = history
            .iter()
            .filter(|snapshot| {
                snapshot.cast_order_this_turn.is_some()
                    && caster_ids.contains(&snapshot.controller)
                    && self.filter.matches_snapshot(snapshot, &filter_ctx, game)
            })
            .cloned()
            .collect::<Vec<_>>();

        if candidates.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let chosen = if candidates.len() == 1 {
            candidates[0].clone()
        } else {
            let Some(chosen) = super::scoped_choice::select_typed_menu(
                game,
                ctx.decision_maker,
                chooser,
                ctx.source,
                &self.description,
                &candidates,
                super::scoped_choice::SingleChoicePolicy::First,
                |idx, snapshot| {
                    let description = snapshot
                        .cast_order_this_turn
                        .map(|order| format!("{} (cast #{order})", snapshot.name))
                        .unwrap_or_else(|| snapshot.name.to_string());
                    SelectableOption::new(idx, description).with_object(snapshot.object_id)
                },
            )?
            else {
                return Ok(EffectOutcome::count(0));
            };
            chosen
        };

        ctx.set_tagged_objects(self.tag.clone(), vec![chosen]);
        Ok(EffectOutcome::count(1))
    }
}
