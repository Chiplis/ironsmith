//! Two-phase per-object production with correlated result consumption.

use std::collections::HashSet;

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, execute_effect};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{ChooseSpec, TaggedOpbjectRelation};

pub type ForEachObjectCorrelatedResultEffect =
    ironsmith_core::ForEachObjectCorrelatedResultEffect<Effect>;

fn restore_tag(ctx: &mut ExecutionContext, tag: TagKey, original: Option<Vec<ObjectSnapshot>>) {
    match original {
        Some(snapshots) => {
            ctx.tagged_objects.insert(tag, snapshots);
        }
        None => {
            ctx.tagged_objects.remove(&tag);
        }
    }
}

impl EffectExecutor for ForEachObjectCorrelatedResultEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.producer_effects {
            visitor(effect);
        }
        for effect in &self.consumer_effects {
            visitor(effect);
        }
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        vec![ChooseSpec::All(self.filter.clone())]
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);
        let has_only_is_tagged_constraints = !self.filter.tagged_constraints.is_empty()
            && self
                .filter
                .tagged_constraints
                .iter()
                .all(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject);
        let candidate_ids = if has_only_is_tagged_constraints {
            let mut seen = HashSet::new();
            let mut ids = Vec::new();
            for constraint in &self.filter.tagged_constraints {
                for snapshot in ctx.get_tagged_all(&constraint.tag).into_iter().flatten() {
                    if seen.insert(snapshot.object_id) {
                        ids.push(snapshot.object_id);
                    }
                }
            }
            ids
        } else if let Some(zone) = self.filter.zone {
            game.zone_ids(zone).collect::<Vec<_>>()
        } else {
            game.battlefield.clone()
        };
        let matching = candidate_ids
            .into_iter()
            .filter_map(|id| game.object(id).map(|object| (id, object)))
            .filter(|(_, object)| self.filter.matches(object, &filter_ctx, game))
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        let it_tag = TagKey::from("__it__");
        let original_it = ctx.tagged_objects.remove(&it_tag);
        let original_result = ctx.tagged_objects.remove(&self.result_tag);
        let original_source_binding = ctx.tagged_objects.remove(&self.source_binding_tag);
        let original_result_binding = ctx.tagged_objects.remove(&self.result_binding_tag);

        let mut all_results = Vec::<ObjectSnapshot>::new();
        let mut pairs = Vec::<(ObjectSnapshot, ObjectSnapshot)>::new();
        let mut outcomes = Vec::new();

        let execution = (|| {
            // Complete every producer iteration first. This preserves the
            // sentence-level timing of "create ... . Each of those ...".
            for source_id in matching {
                let Some(source) = game.object(source_id) else {
                    continue;
                };
                let source_snapshot = ObjectSnapshot::from_object(source, game);
                ctx.set_tagged_objects(it_tag.clone(), vec![source_snapshot.clone()]);
                ctx.tagged_objects.remove(&self.result_tag);

                ctx.with_temp_iterated_object(Some(source_id), |ctx| {
                    ctx.with_temp_iterated_player(Some(source_snapshot.controller), |ctx| {
                        for effect in &self.producer_effects {
                            outcomes.push(execute_effect(game, effect, ctx)?);
                        }
                        Ok::<(), ExecutionError>(())
                    })
                })?;

                let iteration_results = ctx
                    .get_tagged_all(&self.result_tag)
                    .cloned()
                    .unwrap_or_default();
                if let Some(first_result) = iteration_results.first() {
                    // One source consumes at most one produced result. If a
                    // replacement creates extras, the distinct-pair contract
                    // still never reuses a source object.
                    pairs.push((source_snapshot, first_result.clone()));
                }
                for snapshot in iteration_results {
                    if !all_results
                        .iter()
                        .any(|existing| existing.object_id == snapshot.object_id)
                    {
                        all_results.push(snapshot);
                    }
                }
            }

            ctx.set_tagged_objects(self.result_tag.clone(), all_results.clone());

            // Consumers run only after production is complete, with explicit
            // one-object bindings for both sides of each retained pair.
            for (source_snapshot, result_snapshot) in &pairs {
                ctx.set_tagged_objects(
                    self.source_binding_tag.clone(),
                    vec![source_snapshot.clone()],
                );
                ctx.set_tagged_objects(
                    self.result_binding_tag.clone(),
                    vec![result_snapshot.clone()],
                );
                ctx.set_tagged_objects(it_tag.clone(), vec![result_snapshot.clone()]);
                ctx.with_temp_iterated_object(Some(result_snapshot.object_id), |ctx| {
                    ctx.with_temp_iterated_player(Some(result_snapshot.controller), |ctx| {
                        for effect in &self.consumer_effects {
                            outcomes.push(execute_effect(game, effect, ctx)?);
                        }
                        Ok::<(), ExecutionError>(())
                    })
                })?;
            }

            Ok(EffectOutcome::aggregate_summing_counts(outcomes))
        })();

        restore_tag(ctx, it_tag, original_it);
        restore_tag(
            ctx,
            self.source_binding_tag.clone(),
            original_source_binding,
        );
        restore_tag(
            ctx,
            self.result_binding_tag.clone(),
            original_result_binding,
        );
        if execution.is_err() {
            restore_tag(ctx, self.result_tag.clone(), original_result);
        } else {
            ctx.set_tagged_objects(self.result_tag.clone(), all_results);
        }

        execution
    }
}
