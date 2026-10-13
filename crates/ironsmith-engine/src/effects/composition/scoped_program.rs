//! Local program bindings resolved before any child can change their referents.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::zone::Zone;

#[derive(Debug, Clone)]
pub(crate) struct ScopedProgram {
    pub players: Vec<(TagKey, PlayerFilter)>,
    pub object_tags: Vec<TagKey>,
    pub effects: Vec<Effect>,
    pub result_objects: (TagKey, Zone),
    /// Preserve an original action receipt even if a later instruction moves it.
    pub retain_result_snapshots: bool,
}
impl EffectExecutor for ScopedProgram {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Inspect
    }
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for child in &self.effects {
            visitor(child);
        }
    }
    fn inspect_library_movement_override(
        &self,
        context: &mut ironsmith_core::library_movement::LibraryMovementContext,
    ) -> Option<bool> {
        let previous = self
            .object_tags
            .iter()
            .map(|tag| (tag.clone(), context.tag_library_origin(tag)))
            .collect::<Vec<_>>();
        for tag in &self.object_tags {
            context.bind_tag(tag.clone(), (false, false));
        }
        let moves = context.inspect_sequence(&self.effects);
        for (tag, origin) in previous {
            context.bind_tag(tag, origin);
        }
        Some(moves)
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let players = self
            .players
            .iter()
            .map(|(tag, filter)| {
                crate::effects::helpers::resolve_player_filter(game, filter, ctx)
                    .map(|player| (tag.clone(), player))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let old_players = players
            .iter()
            .map(|(tag, _)| (tag.clone(), ctx.tagged_players.remove(tag)))
            .collect::<Vec<_>>();
        let old_objects = self
            .object_tags
            .iter()
            .map(|tag| (tag.clone(), ctx.tagged_objects.remove(tag)))
            .collect::<Vec<_>>();
        for (tag, player) in players {
            ctx.set_tagged_players(tag, vec![player]);
        }
        let result = super::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let mut children = Vec::new();
                for effect in &self.effects {
                    children.push(effect.0.execute_child_with_outputs(game, ctx)?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    if ctx.resolution_stopped() {
                        break;
                    }
                }
                let objects = ctx
                    .get_tagged_all(&self.result_objects.0)
                    .into_iter()
                    .flatten()
                    .filter_map(|snapshot| {
                        if self.retain_result_snapshots {
                            (snapshot.zone == self.result_objects.1).then_some(snapshot.object_id)
                        } else {
                            game.object(snapshot.object_id)
                                .filter(|object| object.zone == self.result_objects.1)
                                .map(|object| object.id)
                        }
                    })
                    .collect::<Vec<_>>();
                let primary = if objects.is_empty() {
                    EffectOutcome::count(0)
                } else {
                    EffectOutcome::with_objects(objects)
                };
                Ok(CompletedEffectOutputs::with_primary_result(
                    primary, children,
                ))
            },
        );
        for (tag, old) in old_players {
            ctx.tagged_players.remove(&tag);
            if let Some(old) = old {
                ctx.tagged_players.insert(tag, old);
            }
        }
        for (tag, old) in old_objects {
            ctx.tagged_objects.remove(&tag);
            if let Some(old) = old {
                ctx.tagged_objects.insert(tag, old);
            }
        }
        result
    }
}

/// Read only the current objects named by retained snapshots. This deliberately
/// does not chase stable card identity across a later zone change.
#[derive(Debug, Clone)]
pub(crate) struct ExactTaggedObjects {
    pub tag: TagKey,
    pub zone: Zone,
}
impl EffectExecutor for ExactTaggedObjects {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Leaf
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = ctx
            .get_tagged_all(&self.tag)
            .into_iter()
            .flatten()
            .filter_map(|snapshot| game.object(snapshot.object_id))
            .filter(|object| object.zone == self.zone)
            .map(|object| object.id)
            .collect();
        Ok(EffectOutcome::with_objects(objects))
    }
}
