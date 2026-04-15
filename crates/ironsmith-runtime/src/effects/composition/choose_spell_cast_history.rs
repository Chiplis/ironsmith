use crate::filter::ObjectFilterExt as _;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_player_filter_to_list};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseSpellCastHistoryEffect {
    pub chooser: PlayerFilter,
    pub cast_by: PlayerFilter,
    pub filter: ObjectFilter,
    pub tag: TagKey,
    pub description: String,
}

impl ChooseSpellCastHistoryEffect {
    pub fn new(
        chooser: PlayerFilter,
        cast_by: PlayerFilter,
        filter: ObjectFilter,
        tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            chooser,
            cast_by,
            filter,
            tag: tag.into(),
            description: "Choose one of those spells".to_string(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

impl EffectExecutor for ChooseSpellCastHistoryEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
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
            let options = candidates
                .iter()
                .enumerate()
                .map(|(idx, snapshot)| {
                    let description = snapshot
                        .cast_order_this_turn
                        .map(|order| format!("{} (cast #{order})", snapshot.name))
                        .unwrap_or_else(|| snapshot.name.clone());
                    SelectableOption::new(idx, description).with_object(snapshot.object_id)
                })
                .collect::<Vec<_>>();
            let choice_ctx = SelectOptionsContext::new(
                chooser,
                Some(ctx.source),
                self.description.clone(),
                options,
                1,
                1,
            );
            let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let Some(chosen_idx) = selected
                .into_iter()
                .next()
                .filter(|idx| *idx < candidates.len())
            else {
                return Ok(EffectOutcome::count(0));
            };
            candidates[chosen_idx].clone()
        };

        ctx.set_tagged_objects(self.tag.clone(), vec![chosen]);
        Ok(EffectOutcome::count(1))
    }
}
