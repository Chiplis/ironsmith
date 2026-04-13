//! Shared runtime helpers for tagged effect execution.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_objects_from_spec;
use crate::executor::{ExecutionContext, ResolvedTarget};
use crate::game_state::GameState;
use crate::ids::StableId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::zone::Zone;
use std::collections::HashSet;

/// Runtime state captured before tagged effect execution.
#[derive(Debug, Clone, Default)]
pub(crate) struct TaggedRuntimeState {
    pre_snapshot: Option<ObjectSnapshot>,
    stable_id_fallback: Option<StableIdFallback>,
}

#[derive(Debug, Clone)]
struct StableIdFallback {
    stable_ids: Vec<StableId>,
    zone: Zone,
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

pub(crate) fn capture_all_effect_target_snapshots(
    game: &GameState,
    effect: &Effect,
    ctx: &ExecutionContext,
) -> Vec<ObjectSnapshot> {
    let mut snapshots = capture_target_object_snapshots(game, ctx);
    if !snapshots.is_empty() {
        return snapshots;
    }

    let Some(spec) = effect.0.get_target_spec() else {
        return snapshots;
    };
    let Ok(object_ids) = resolve_objects_from_spec(game, spec, ctx) else {
        return snapshots;
    };

    let mut seen = HashSet::new();
    for object_id in object_ids {
        if !seen.insert(object_id) {
            continue;
        }
        if let Some(snapshot) = snapshot_for_object_reference(game, ctx, object_id) {
            snapshots.push(snapshot);
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
    let output_ids = outcome_object_candidates(outcome);
    if !output_ids.is_empty() {
        let expected_zone = state.stable_id_fallback.as_ref().map(|fallback| fallback.zone);
        let snapshots = output_ids
            .iter()
            .filter_map(|id| {
                game.object(*id).and_then(|obj| {
                    expected_zone
                        .map_or(true, |zone| obj.zone == zone)
                        .then(|| ObjectSnapshot::from_object(obj, game))
                })
            })
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            ctx.set_tagged_objects(tag, snapshots);
            return;
        }
    }

    // Zone-change fallback: remap stable IDs to the objects' current zone after
    // the effect resolves. This keeps tagged follow-up effects pointed at the
    // new object ids created by rule 400.7 zone changes.
    if let Some(fallback) = state.stable_id_fallback {
        let snapshots = fallback
            .stable_ids
            .into_iter()
            .filter_map(|stable_id| game.find_object_by_stable_id(stable_id))
            .filter_map(|id| {
                game.object(id).and_then(|obj| {
                    (obj.zone == fallback.zone).then(|| ObjectSnapshot::from_object(obj, game))
                })
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
    }
}

fn outcome_object_candidates(outcome: &EffectOutcome) -> Vec<crate::ids::ObjectId> {
    let mut ids = Vec::new();
    if let Some(objects) = outcome.objects() {
        ids.extend(objects.iter().copied());
    }
    if let Some(affected) = outcome.affected_objects() {
        ids.extend(affected.iter().copied());
    }
    if ids.is_empty()
        && let Some(chosen) = outcome.chosen_objects()
    {
        ids.extend(chosen.iter().copied());
    }
    let mut seen = HashSet::new();
    ids.retain(|id| seen.insert(*id));
    ids
}

fn capture_stable_id_fallback(
    game: &GameState,
    effect: &Effect,
    ctx: &ExecutionContext,
) -> Option<StableIdFallback> {
    let capture = |spec: &crate::target::ChooseSpec, zone: Zone| {
        resolve_objects_from_spec(game, spec, ctx).ok().map(|ids| StableIdFallback {
            stable_ids: ids
                .into_iter()
                .filter_map(|id| game.object(id).map(|obj| obj.stable_id))
                .collect::<Vec<_>>(),
            zone,
        })
    };

    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return capture(&exile.spec, Zone::Exile);
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return capture(&move_to_zone.target, move_to_zone.zone);
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        return capture(&return_to_hand.spec, Zone::Hand);
    }
    if let Some(return_all) = effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
    {
        let spec = crate::target::ChooseSpec::all(return_all.filter.clone());
        return capture(&spec, Zone::Battlefield);
    }

    None
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

    #[test]
    fn test_apply_tagged_runtime_state_ignores_objects_outside_expected_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let runtime = capture_tagged_runtime_state(
            &game,
            &Effect::new(crate::effects::ExileEffect::specific(creature)),
            &ctx,
        );
        let exile_id = game
            .move_object(creature, Zone::Hand, crate::events::cause::EventCause::effect())
            .expect("creature should move");
        let outcome = EffectOutcome::replaced().with_affected_objects(vec![exile_id]);

        apply_tagged_runtime_state(&game, &mut ctx, TagKey::new("tagged"), &outcome, runtime);

        let tagged = ctx.get_tagged("tagged").expect("tagged object");
        assert_eq!(tagged.zone, Zone::Battlefield);
        assert_eq!(tagged.stable_id, game.object(exile_id).unwrap().stable_id);
    }
}
