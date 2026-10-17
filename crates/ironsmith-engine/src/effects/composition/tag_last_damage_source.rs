//! Tag the source of the most recent matching damage event this turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use ironsmith_core::{DamageHistoryRecipients, DamageHistorySources};
pub use ironsmith_core::TagLastDamageSourceEffect;

impl EffectExecutor for TagLastDamageSourceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);
        let mut found: Option<ObjectSnapshot> = None;
        for record in game.turn_store.turn_history.projected_records().rev() {
            let Some(damage) = record.event.downcast::<crate::events::DamageEvent>() else {
                continue;
            };
            if damage.amount == 0
                || self
                    .query
                    .combat
                    .is_some_and(|combat| combat != damage.is_combat)
            {
                continue;
            }
            let matches_recipient = match &self.query.recipients {
                DamageHistoryRecipients::Any => true,
                DamageHistoryRecipients::Players(players) => matches!(
                    damage.target,
                    crate::events::DamageTarget::Player(id)
                        if crate::filter::player_filter_matches_game(players, id, game, &filter_ctx)
                ),
                DamageHistoryRecipients::MatchingObjects(filter) => {
                    matches!(damage.target, crate::events::DamageTarget::Object(_))
                        && damage
                            .target_snapshot
                            .as_ref()
                            .is_some_and(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
                }
                // Exact recipient references are read by the numeric history
                // query; this capture names a class of recipients.
                DamageHistoryRecipients::Reference(_) => false,
            };
            if !matches_recipient {
                continue;
            }
            // The source's characteristics when it dealt the damage.
            let Some(snapshot) = record
                .event
                .source_snapshot()
                .or(record.source_snapshot.as_ref())
                .filter(|snapshot| snapshot.object_id == damage.source)
            else {
                continue;
            };
            let matches_source = match &self.query.sources {
                DamageHistorySources::Any => true,
                DamageHistorySources::Matching(filter) => {
                    filter.matches_snapshot(snapshot, &filter_ctx, game)
                }
                DamageHistorySources::Reference(_) | DamageHistorySources::SourceAttachedObject => {
                    false
                }
            };
            if matches_source {
                found = Some(snapshot.clone());
                break;
            }
        }
        let object_ids = found
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<Vec<_>>();
        ctx.set_tagged_objects(self.tag.clone(), found.into_iter().collect());
        Ok(EffectOutcome::with_objects(object_ids))
    }
}
