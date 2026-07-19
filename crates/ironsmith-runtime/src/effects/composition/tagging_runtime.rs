//! Shared runtime helpers for tagged effect execution.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_objects_from_spec;
use crate::effects::{ExecutionContext, ResolvedTarget};
use crate::game_state::GameState;
use crate::ids::StableId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::zone::Zone;
use std::collections::HashSet;

/// Runtime state captured before tagged effect execution.
#[derive(Debug, Clone, Default)]
pub(crate) struct TaggedRuntimeState {
    pre_snapshots: Vec<ObjectSnapshot>,
    stable_id_fallback: Option<StableIdFallback>,
}

impl TaggedRuntimeState {
    pub(crate) fn from_pre_snapshot(pre_snapshot: Option<ObjectSnapshot>) -> Self {
        Self {
            pre_snapshots: pre_snapshot.into_iter().collect(),
            stable_id_fallback: None,
        }
    }
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

    let mut seen = HashSet::new();
    let specs = effect.0.get_target_spec().map_or_else(
        || effect.0.decision_related_object_specs(),
        |spec| vec![spec.clone()],
    );
    for spec in specs {
        if let crate::target::ChooseSpec::Tagged(tag) = spec.base() {
            for snapshot in ctx.get_tagged_all(tag).into_iter().flatten() {
                if seen.insert(snapshot.object_id) {
                    snapshots.push(snapshot.clone());
                }
            }
            continue;
        }
        let Ok(object_ids) = resolve_objects_from_spec(game, &spec, ctx) else {
            continue;
        };
        for object_id in object_ids {
            if !seen.insert(object_id) {
                continue;
            }
            if let Some(snapshot) = snapshot_for_object_reference(game, ctx, object_id) {
                snapshots.push(snapshot);
            }
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
    let mut pre_snapshots = capture_all_effect_target_snapshots(game, effect, ctx);
    if pre_snapshots.is_empty()
        && let Some(object_id) = ctx.iteration.iterated_object
        && let Some(obj) = game.object(object_id)
    {
        pre_snapshots.push(ObjectSnapshot::from_object(obj, game));
    }
    if pre_snapshots.is_empty()
        && let Some(snapshot) = capture_effect_target_snapshot(game, effect, ctx)
    {
        pre_snapshots.push(snapshot);
    }

    TaggedRuntimeState {
        pre_snapshots,
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
    // Zone changes create a new object, but later references such as "that
    // permanent's controller" use the characteristics of the object as it
    // last existed in the old zone. Keep that LKI snapshot when the stable
    // card reached the expected destination; object-resolution helpers can
    // still follow its stable id to the new object when a later effect needs
    // to move or otherwise affect the card itself.
    if let Some(fallback) = state.stable_id_fallback.as_ref() {
        let moved_stable_ids = fallback
            .stable_ids
            .iter()
            .copied()
            .filter(|stable_id| {
                game.find_object_by_stable_id(*stable_id)
                    .and_then(|id| game.object(id))
                    .is_some_and(|object| object.zone == fallback.zone)
            })
            .collect::<HashSet<_>>();
        let snapshots = state
            .pre_snapshots
            .iter()
            .filter(|snapshot| moved_stable_ids.contains(&snapshot.stable_id))
            .cloned()
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            ctx.set_tagged_objects(tag, snapshots);
            return;
        }
    }

    // Primary post-map path: if the effect returned object IDs, tag those.
    let output_ids = outcome_object_candidates(outcome);
    if !output_ids.is_empty() {
        let expected_zone = state
            .stable_id_fallback
            .as_ref()
            .map(|fallback| fallback.zone);
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

    // Generic fallback: preserve the pre-effect target snapshots.
    if !state.pre_snapshots.is_empty() {
        ctx.tag_objects(tag, state.pre_snapshots);
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
    if let Some(memory) = outcome.affected_object_memory() {
        ids.extend(memory.iter().map(|memory| memory.object_id));
    }
    if ids.is_empty()
        && let Some(chosen) = outcome.chosen_objects()
    {
        ids.extend(chosen.iter().copied());
    }
    if ids.is_empty()
        && let Some(memory) = outcome.chosen_object_memory()
    {
        ids.extend(memory.iter().map(|memory| memory.object_id));
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
        resolve_objects_from_spec(game, spec, ctx)
            .ok()
            .map(|ids| StableIdFallback {
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
    if let crate::target::ChooseSpec::Tagged(tag) = spec.base() {
        return ctx
            .get_tagged_all(tag)
            .and_then(|snapshots| snapshots.first().cloned());
    }
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
    use crate::effects::ResolvedTarget;
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
    fn test_capture_tagged_target_snapshot_preserves_lki_object_id() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice);
        let source = game.new_object_id();
        let snapshot = ObjectSnapshot::from_object(game.object(creature).expect("creature"), &game);
        game.move_object_by_effect(creature, Zone::Graveyard)
            .expect("creature should move");
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects("subject", vec![snapshot.clone()]);

        let effect = Effect::tap(crate::target::ChooseSpec::Tagged(TagKey::from("subject")));
        let snapshots = capture_all_effect_target_snapshots(&game, &effect, &ctx);

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].object_id, creature);
        assert_eq!(snapshots[0].stable_id, snapshot.stable_id);
    }

    #[test]
    fn test_capture_mass_continuous_filter_as_affected_set() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_creature(&mut game, alice);
        let second = create_creature(&mut game, alice);
        let _opposing = create_creature(&mut game, PlayerId::from_index(1));
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);

        let mut controlled_creature = crate::target::ObjectFilter::creature();
        controlled_creature.controller = Some(crate::target::PlayerFilter::You);
        let effect = Effect::new(crate::effects::ApplyContinuousEffect::new_runtime(
            crate::continuous::EffectTarget::Filter(controlled_creature),
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power: crate::effect::Value::Fixed(1),
                toughness: crate::effect::Value::Fixed(1),
            },
            crate::effect::Until::EndOfTurn,
        ));
        let snapshots = capture_all_effect_target_snapshots(&game, &effect, &ctx);
        let ids = snapshots
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<HashSet<_>>();

        assert_eq!(ids, HashSet::from([first, second]));
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
            .move_object(
                creature,
                Zone::Hand,
                crate::events::cause::EventCause::effect(),
            )
            .expect("creature should move");
        let outcome = EffectOutcome::replaced().with_affected_objects(vec![exile_id]);

        apply_tagged_runtime_state(&game, &mut ctx, TagKey::new("tagged"), &outcome, runtime);

        let tagged = ctx.get_tagged("tagged").expect("tagged object");
        assert_eq!(tagged.zone, Zone::Battlefield);
        assert_eq!(tagged.stable_id, game.object(exile_id).unwrap().stable_id);
    }

    #[test]
    fn test_zone_change_tag_preserves_last_known_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = create_creature(&mut game, alice);
        game.set_current_controller(creature, bob);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let runtime = capture_tagged_runtime_state(
            &game,
            &Effect::new(crate::effects::ReturnToHandEffect::with_spec(
                crate::target::ChooseSpec::SpecificObject(creature),
            )),
            &ctx,
        );
        let hand_id = game
            .move_object_by_effect(creature, Zone::Hand)
            .expect("creature should return to its owner's hand");
        let outcome = EffectOutcome::resolved().with_affected_objects(vec![hand_id]);

        apply_tagged_runtime_state(&game, &mut ctx, TagKey::new("returned"), &outcome, runtime);

        let tagged = ctx.get_tagged("returned").expect("returned object LKI");
        assert_eq!(tagged.object_id, creature);
        assert_eq!(tagged.controller, bob);
        assert_eq!(tagged.zone, Zone::Battlefield);
        assert_eq!(tagged.stable_id, game.object(hand_id).unwrap().stable_id);
    }
}
