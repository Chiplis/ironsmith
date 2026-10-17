//! "Note the number and kind of counters that were on that creature"
//! (Tawnos's Coffin).
//!
//! The note is the referenced object's last-known information (CR 608.2h):
//! when that object already left the battlefield earlier in the same
//! resolution, its counters come from the pre-move snapshot of the tagged
//! instruction that moved it, not from its new (counterless, CR 122.2)
//! incarnation. The noted snapshots are stored under the effect's tag, which
//! a delayed ability inherits with the rest of the resolution's tags; a later
//! battlefield entry reads them back by stable identity.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::TaggedOpbjectRelation;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
pub use ironsmith_core::NoteCountersEffect;

/// The tag an object reference names, when it is a back-reference.
fn referenced_tag(spec: &ChooseSpec) -> Option<&TagKey> {
    match spec.base() {
        ChooseSpec::Tagged(tag) => Some(tag),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            let mut tags = filter
                .tagged_constraints
                .iter()
                .filter(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
                .map(|constraint| &constraint.tag);
            let tag = tags.next()?;
            tags.next().is_none().then_some(tag)
        }
        _ => None,
    }
}

fn noted_snapshots(
    game: &GameState,
    ctx: &ExecutionContext,
    spec: &ChooseSpec,
) -> Vec<ObjectSnapshot> {
    if let Some(tag) = referenced_tag(spec) {
        // The object left its zone during this resolution: its counters are
        // the ones it had as it last existed there.
        if let Some(history) =
            ctx.get_tagged_all(format!("__pre_move_history__{}", tag.as_str()))
            && !history.is_empty()
        {
            return history.clone();
        }
        if let Some(current) = ctx.get_tagged_all(tag) {
            return current.clone();
        }
        return Vec::new();
    }
    resolve_objects_from_spec(game, spec, ctx)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| {
            game.object(id).map(|object| {
                ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
            })
        })
        .collect()
}

impl EffectExecutor for NoteCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let snapshots = noted_snapshots(game, ctx, &self.object);
        let object_ids = snapshots
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<Vec<_>>();
        ctx.set_tagged_objects(self.tag.clone(), snapshots);
        Ok(EffectOutcome::with_objects(object_ids))
    }
}

/// The counters noted for the object entering as `stable_id`.
pub(crate) fn noted_entry_counters(
    ctx: &ExecutionContext,
    tag: &TagKey,
    object_id: crate::ids::ObjectId,
    stable_id: crate::ids::StableId,
) -> Vec<(crate::object::CounterType, u32)> {
    ctx.get_tagged_all(tag)
        .into_iter()
        .flatten()
        .find(|snapshot| snapshot.stable_id == stable_id || snapshot.object_id == object_id)
        .map(|snapshot| {
            snapshot
                .counters
                .iter()
                .filter(|(_, count)| **count > 0)
                .map(|(counter, count)| (*counter, *count))
                .collect()
        })
        .unwrap_or_default()
}
