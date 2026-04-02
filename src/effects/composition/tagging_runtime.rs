//! Shared runtime helpers for tagged effect execution.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_objects_from_spec;
use crate::executor::{ExecutionContext, ResolvedTarget};
use crate::game_state::GameState;
use crate::ids::StableId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::zone::Zone;

/// Runtime state captured before tagged effect execution.
#[derive(Debug, Clone, Default)]
pub(crate) struct TaggedRuntimeState {
    pre_snapshot: Option<ObjectSnapshot>,
    stable_id_fallback: Option<Vec<StableId>>,
}

/// Capture snapshots of all object targets currently present in context.
pub(crate) fn capture_target_object_snapshots(
    game: &GameState,
    ctx: &ExecutionContext,
) -> Vec<ObjectSnapshot> {
    let mut snapshots = Vec::new();
    for target in &ctx.targets {
        if let ResolvedTarget::Object(object_id) = target
            && let Some(obj) = game.object(*object_id)
        {
            snapshots.push(ObjectSnapshot::from_object(obj, game));
        }
    }
    snapshots
}

/// Capture pre-resolution tagging state for a tagged effect execution.
pub(crate) fn capture_tagged_runtime_state(
    game: &GameState,
    effect: &Effect,
    ctx: &ExecutionContext,
) -> TaggedRuntimeState {
    let mut pre_snapshot = capture_target_object_snapshots(game, ctx)
        .into_iter()
        .next();
    if pre_snapshot.is_none()
        && let Some(object_id) = ctx.iterated_object
        && let Some(obj) = game.object(object_id)
    {
        pre_snapshot = Some(ObjectSnapshot::from_object(obj, game));
    }
    if pre_snapshot.is_none() {
        pre_snapshot = capture_effect_target_snapshot(game, effect, ctx);
    }

    TaggedRuntimeState {
        pre_snapshot,
        stable_id_fallback: capture_stable_id_fallback(game, effect, ctx),
    }
}

/// Apply tagging semantics after the inner effect has resolved.
pub(crate) fn apply_tagged_runtime_state(
    game: &GameState,
    ctx: &mut ExecutionContext,
    tag: TagKey,
    outcome: &EffectOutcome,
    state: TaggedRuntimeState,
) {
    // Primary post-map path: if the effect returned object IDs, tag those.
    let output_ids = outcome.output_objects();
    if !output_ids.is_empty() {
        let snapshots = output_ids
            .iter()
            .filter_map(|id| {
                game.object(*id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            ctx.set_tagged_objects(tag, snapshots);
            return;
        }
    }

    // Generic fallback: preserve the pre-effect target snapshot.
    if let Some(snapshot) = state.pre_snapshot {
        ctx.tag_object(tag, snapshot);
        return;
    }

    // Effect-specific fallback: remap stable IDs to current battlefield objects.
    if let Some(stable_ids) = state.stable_id_fallback {
        let snapshots = stable_ids
            .into_iter()
            .filter_map(|stable_id| game.find_object_by_stable_id(stable_id))
            .filter_map(|id| {
                game.object(id).and_then(|obj| {
                    (obj.zone == Zone::Battlefield).then(|| ObjectSnapshot::from_object(obj, game))
                })
            })
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            ctx.set_tagged_objects(tag, snapshots);
        }
    }
}

fn capture_stable_id_fallback(
    game: &GameState,
    effect: &Effect,
    ctx: &ExecutionContext,
) -> Option<Vec<StableId>> {
    // Effect-specific adapter for return-all zone changes that don't always
    // return object IDs in the outcome.
    effect
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
        .and_then(|return_all| {
            let spec = crate::target::ChooseSpec::all(return_all.filter.clone());
            resolve_objects_from_spec(game, &spec, ctx).ok().map(|ids| {
                ids.into_iter()
                    .filter_map(|id| game.object(id).map(|obj| obj.stable_id))
                    .collect::<Vec<_>>()
            })
        })
}

fn capture_effect_target_snapshot(
    game: &GameState,
    effect: &Effect,
    ctx: &ExecutionContext,
) -> Option<ObjectSnapshot> {
    let spec = effect.0.get_target_spec()?;
    let object_id = resolve_objects_from_spec(game, spec, ctx)
        .ok()?
        .into_iter()
        .next()?;
    snapshot_for_object_reference(game, ctx, object_id)
}

fn snapshot_for_object_reference(
    game: &GameState,
    ctx: &ExecutionContext,
    object_id: crate::ids::ObjectId,
) -> Option<ObjectSnapshot> {
    if let Some(obj) = game.object(object_id) {
        return Some(ObjectSnapshot::from_object(obj, game));
    }
    if let Some(snapshot) = ctx.target_snapshots.get(&object_id) {
        return Some(snapshot.clone());
    }
    if let Some(snapshot) = ctx.source_snapshot.as_ref()
        && snapshot.object_id == object_id
    {
        return Some(snapshot.clone());
    }
    ctx.tagged_objects
        .values()
        .flat_map(|snapshots| snapshots.iter())
        .find(|snapshot| snapshot.object_id == object_id)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, owner: PlayerId) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), "Test Creature")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, owner, Zone::Battlefield));
        id
    }

    #[test]
    fn test_capture_target_object_snapshots() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let snapshots = capture_target_object_snapshots(&game, &ctx);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].object_id, creature);
    }

    #[test]
    fn test_apply_tagged_runtime_state_uses_outcome_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = EffectOutcome::with_objects(vec![creature]);
        apply_tagged_runtime_state(
            &game,
            &mut ctx,
            TagKey::new("tagged"),
            &outcome,
            TaggedRuntimeState::default(),
        );

        let tagged = ctx.get_tagged("tagged").expect("tagged object");
        assert_eq!(tagged.object_id, creature);
    }

    #[test]
    fn test_apply_tagged_runtime_state_falls_back_to_pre_snapshot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let runtime = capture_tagged_runtime_state(&game, &Effect::gain_life(1), &ctx);
        let outcome = EffectOutcome::resolved();
        apply_tagged_runtime_state(&game, &mut ctx, TagKey::new("tagged"), &outcome, runtime);

        let tagged = ctx.get_tagged("tagged").expect("tagged object");
        assert_eq!(tagged.object_id, creature);
    }
}
