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
    pre_snapshots_from_decision_hints: bool,
    stable_id_fallback: Option<StableIdFallback>,
    pub(crate) outcome_only: bool,
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
            snapshots.push(ObjectSnapshot::from_object_with_calculated_characteristics(
                obj, game,
            ));
        }
    }
    snapshots
}

pub(crate) fn capture_all_effect_target_snapshots(
    game: &GameState,
    effect: &Effect,
    ctx: &ExecutionContext,
) -> Vec<ObjectSnapshot> {
    capture_effect_snapshots_with_provenance(game, effect, ctx).0
}

fn capture_effect_snapshots_with_provenance(
    game: &GameState,
    effect: &Effect,
    ctx: &ExecutionContext,
) -> (Vec<ObjectSnapshot>, bool) {
    let explicit_target_spec = effect.0.get_target_spec();
    if explicit_target_spec.is_some_and(|spec| spec.is_target()) {
        let snapshots = capture_target_object_snapshots(game, ctx);
        if !snapshots.is_empty() {
            return (snapshots, false);
        }
    }
    let mut seen = HashSet::new();
    let specs = explicit_target_spec.map_or_else(
        || effect.0.decision_related_object_specs(),
        |spec| vec![spec.clone()],
    );
    let mut snapshots = Vec::new();
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
    let from_decision_hints = explicit_target_spec.is_none() && !snapshots.is_empty();
    if snapshots.is_empty() && explicit_target_spec.is_none() {
        snapshots = capture_target_object_snapshots(game, ctx);
    }
    (snapshots, from_decision_hints)
}

/// Capture pre-resolution tagging state for a tagged effect execution.
pub(crate) fn capture_tagged_runtime_state(
    game: &GameState,
    effect: &Effect,
    ctx: &ExecutionContext,
) -> TaggedRuntimeState {
    let (mut pre_snapshots, pre_snapshots_from_decision_hints) =
        capture_effect_snapshots_with_provenance(game, effect, ctx);
    if pre_snapshots.is_empty()
        && let Some(object_id) = ctx.iteration.iterated_object
        && let Some(obj) = game.object(object_id)
    {
        pre_snapshots.push(ObjectSnapshot::from_object_with_calculated_characteristics(
            obj, game,
        ));
    }
    if pre_snapshots.is_empty()
        && let Some(snapshot) = capture_effect_target_snapshot(game, effect, ctx)
    {
        pre_snapshots.push(snapshot);
    }

    TaggedRuntimeState {
        pre_snapshots,
        pre_snapshots_from_decision_hints,
        stable_id_fallback: capture_stable_id_fallback(game, effect, ctx),
        outcome_only: false,
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
    let outcome = outcome.instruction_result();
    // An explicit object payload or ResultObjects fact is a result-object
    // contract: the inner effect is returning the identities that subsequent
    // effects should use. This is distinct from affected-object facts and
    // memory, which may be LKI for actions such as destroy. Prefer only this
    // explicit contract before the pre-effect zone-change fallback so tagged
    // moves follow the new object created by rule 400.7 without changing
    // destroy-then-controller semantics.
    if let Some(result_ids) = outcome
        .explicit_objects()
        .or_else(|| outcome.result_objects())
    {
        let snapshots = result_ids
            .iter()
            .filter_map(|id| {
                game.object(*id)
                    .map(|object| ObjectSnapshot::from_object(object, game))
            })
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            ctx.set_tagged_objects(tag, snapshots);
            return;
        }
    }

    // Decision hints describe candidates, not the objects actually selected.
    // Resolve these from exact instruction facts before a destination-based
    // fallback: a replacement may redirect some or all selected objects.
    // Explicit target snapshots retain their existing LKI contract.
    if state.pre_snapshots_from_decision_hints {
        let chosen = !state.outcome_only
            && outcome.execution_facts.iter().any(|fact| matches!(
                fact, crate::effect::ExecutionFact::ChosenObjectMemory(_)
                    | crate::effect::ExecutionFact::ChosenObjects(_)
            ));
        let mut seen = HashSet::new();
        let mut snapshots = outcome.execution_facts.iter().filter_map(|fact| match fact {
            crate::effect::ExecutionFact::ChosenObjectMemory(memory) if chosen => Some(memory),
            crate::effect::ExecutionFact::AffectedObjectMemory(memory) if !chosen => Some(memory),
            _ => None,
        }).flatten().filter(|memory| seen.insert(memory.object_id))
            .map(|memory| memory.to_snapshot_with_fallback(game,
                state.pre_snapshots.iter().find(|snapshot| snapshot.object_id == memory.object_id)))
            .collect::<Vec<_>>();
        // Some producers return exact IDs without memories. Their pre-effect
        // candidate snapshots can supply LKI, but only for those exact IDs.
        for object_id in outcome.execution_facts.iter().filter_map(|fact| match fact {
            crate::effect::ExecutionFact::ChosenObjects(ids) if chosen => Some(ids),
            crate::effect::ExecutionFact::AffectedObjects(ids) if !chosen => Some(ids),
            _ => None,
        }).flatten() {
            if seen.insert(*object_id) {
                if let Some(snapshot) = state.pre_snapshots.iter()
                    .find(|snapshot| snapshot.object_id == *object_id).cloned()
                    .or_else(|| game.object(*object_id).and_then(|object| {
                        state.pre_snapshots.iter().find(|snapshot|
                            snapshot.stable_id == object.stable_id).cloned()
                    }))
                    .or_else(|| snapshot_for_object_reference(game, ctx, *object_id)) {
                    snapshots.push(snapshot);
                }
            }
        }
        // A declined or empty instruction has no chosen/affected set. Display
        // candidates must never become its result through generic fallback.
        // "Moved/exiled this way" tags qualify successful instruction
        // results by the authored destination. Ordinary reference tags still
        // retain exact selected LKI when replacements redirect the movement.
        if state.outcome_only && let Some(fallback) = state.stable_id_fallback.as_ref() {
            snapshots.retain(|snapshot| game.find_object_by_stable_id(snapshot.stable_id)
                .and_then(|id| game.object(id)).is_some_and(|object| object.zone == fallback.zone));
        }
        let mut distinct_snapshots = HashSet::new();
        snapshots.retain(|snapshot| distinct_snapshots.insert(snapshot.object_id));
        ctx.set_tagged_objects(tag, snapshots);
        return;
    }

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
        if !snapshots.is_empty() || (state.outcome_only && !state.pre_snapshots.is_empty()) {
            ctx.set_tagged_objects(tag, snapshots);
            return;
        }
    }

    // A composition wrapper such as `ForPlayersEffect` may not expose its
    // children's object specs before execution, but its aggregate outcome
    // still carries exact last-known affected-object memories. Prefer those
    // memories before looking up the old IDs in current state; a replacement
    // can leave an object with that ID present but with a different controller
    // or characteristics.
    if state.pre_snapshots.is_empty() {
        // A coordinated group appends one fact per child. The singular
        // accessor returns only the first fact, so collect all successful
        // affected-object memories before exposing the group's result set.
        let mut seen = HashSet::new();
        let snapshots = outcome
            .execution_facts
            .iter()
            .filter_map(|fact| match fact {
                crate::effect::ExecutionFact::AffectedObjectMemory(memory) => Some(memory),
                _ => None,
            })
            .flatten()
            .filter(|memory| seen.insert(memory.object_id))
            .map(|memory| memory.to_snapshot(game))
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
                    expected_zone.is_none_or(|zone| obj.zone == zone).then(|| {
                        ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                    })
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
                    (obj.zone == fallback.zone).then(|| {
                        ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                    })
                })
            })
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            ctx.set_tagged_objects(tag, snapshots);
            return;
        }
    }

    // Generic fallback: preserve the pre-effect target snapshots.
    if state.outcome_only {
        ctx.set_tagged_objects(tag, Vec::new());
    } else if !state.pre_snapshots.is_empty() {
        ctx.tag_objects(tag, state.pre_snapshots);
    }
}

fn outcome_object_candidates(outcome: &EffectOutcome) -> Vec<crate::ids::ObjectId> {
    let mut ids = Vec::new();
    if let Some(objects) = outcome.objects() {
        ids.extend(objects.iter().copied());
    }
    if let Some(results) = outcome.result_objects() {
        ids.extend(results.iter().copied());
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
        return Some(ObjectSnapshot::from_object_with_calculated_characteristics(
            obj, game,
        ));
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

    fn assert_candidate_preview_uses_exact_results(kind: u8) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let selected = [create_creature(&mut game, alice), create_creature(&mut game, alice)];
        let unchosen = create_creature(&mut game, alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::new(crate::effects::SacrificeEffect::player(
            crate::filter::ObjectFilter::creature(), 2, crate::target::PlayerFilter::You));
        let mut runtime = capture_tagged_runtime_state(&game, &effect, &ctx);
        assert!(runtime.pre_snapshots_from_decision_hints);
        runtime.outcome_only = kind == 1;
        let memories = selected.iter().map(|id| crate::effect::OutcomeObjectMemory::from_snapshot(
            &ObjectSnapshot::from_object(game.object(*id).unwrap(), &game))).collect();
        if kind != 3 {
            game.move_object_by_effect(selected[0], Zone::Graveyard).unwrap();
            game.move_object_by_effect(selected[1], Zone::Exile).unwrap();
        }
        let outcome = match kind {
            0 => EffectOutcome::count(2).with_chosen_object_memory(memories),
            1 => EffectOutcome::count(2).with_affected_object_memory(memories)
                .with_affected_objects(vec![game.player(alice).unwrap().graveyard[0], game.exile[0]]),
            2 => EffectOutcome::count(2).with_execution_fact(crate::effect::ExecutionFact::ChosenObjects(selected.to_vec())),
            3 => EffectOutcome::declined(),
            _ => unreachable!(),
        };
        apply_tagged_runtime_state(&game, &mut ctx, TagKey::new("chosen"), &outcome, runtime);
        let snapshots = ctx.get_tagged_all("chosen").cloned().unwrap_or_default();
        let actual = snapshots.iter().map(|snapshot| snapshot.object_id).collect::<HashSet<_>>();
        let expected = if kind == 3 {HashSet::new()} else {HashSet::from(selected)};
        assert_eq!(actual, expected);
        assert!(!actual.contains(&unchosen));
        assert!(snapshots.iter().all(|snapshot| snapshot.zone == Zone::Battlefield && snapshot.controller == alice));
        assert_eq!(snapshots.len(), expected.len());
        assert!(snapshots.iter().all(|snapshot| snapshot.card.is_some()));
    }

    #[test]
    fn preview_chosen_memory_preserves_both_original_and_redirected_destinations() {
        assert_candidate_preview_uses_exact_results(0);
    }

    #[test]
    fn preview_outcome_only_memory_preserves_both_destinations() {
        assert_candidate_preview_uses_exact_results(1);
    }

    #[test]
    fn preview_chosen_ids_preserve_exact_selection_without_memory() {
        assert_candidate_preview_uses_exact_results(2);
    }

    #[test]
    fn preview_declined_action_does_not_tag_candidates() {
        assert_candidate_preview_uses_exact_results(3);
    }

    #[test]
    fn preview_exile_outcome_tag_qualifies_destination_but_reference_tag_keeps_redirected_lki() {
        for outcome_only in [false, true] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let first = create_creature(&mut game, alice);
            let second = create_creature(&mut game, alice);
            let source = game.new_object_id();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let effect = Effect::new(crate::effects::ExileEffect::all(crate::filter::ObjectFilter::creature()));
            let mut runtime = capture_tagged_runtime_state(&game, &effect, &ctx);
            runtime.outcome_only = outcome_only;
            let memories = [first,second].iter().map(|id| crate::effect::OutcomeObjectMemory::from_snapshot(
                &ObjectSnapshot::from_object(game.object(*id).unwrap(), &game))).collect();
            let exile = game.move_object_by_effect(first, Zone::Exile).unwrap();
            let grave = game.move_object_by_effect(second, Zone::Graveyard).unwrap();
            let outcome = EffectOutcome::count(1).with_affected_object_memory(memories)
                .with_affected_objects(vec![exile,grave]);
            apply_tagged_runtime_state(&game, &mut ctx, TagKey::new("exiled"), &outcome, runtime);
            let snapshots = ctx.get_tagged_all("exiled").unwrap();
            assert_eq!(snapshots.len(), if outcome_only {1} else {2});
            assert_eq!(snapshots[0].object_id, first);
            assert!(snapshots.iter().all(|snapshot| snapshot.zone == Zone::Battlefield && snapshot.card.is_some()));
            assert_eq!(snapshots.iter().any(|snapshot| snapshot.object_id == second), !outcome_only);
        }
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
    fn test_capture_non_target_mass_continuous_filter_precedes_ambient_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_creature(&mut game, alice);
        let second = create_creature(&mut game, alice);
        let opposing = create_creature(&mut game, PlayerId::from_index(1));
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(opposing)]);

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
    fn test_capture_explicit_target_precedes_non_target_affected_set_metadata() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let explicit_target = create_creature(&mut game, PlayerId::from_index(1));
        let _other_creature = create_creature(&mut game, alice);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(explicit_target)]);

        let effect = Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
            crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                crate::target::ObjectFilter::creature(),
            )),
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power: crate::effect::Value::Fixed(1),
                toughness: crate::effect::Value::Fixed(1),
            },
            crate::effect::Until::EndOfTurn,
        ));
        let snapshots = capture_all_effect_target_snapshots(&game, &effect, &ctx);

        assert_eq!(
            snapshots
                .iter()
                .map(|snapshot| snapshot.object_id)
                .collect::<Vec<_>>(),
            vec![explicit_target]
        );
    }

    #[test]
    fn aggregate_tag_keeps_each_childs_last_known_affected_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let a = create_creature(&mut game, alice);
        let b = create_creature(&mut game, bob);
        let memories = [a, b].map(|id| {
            crate::effect::OutcomeObjectMemory::from_snapshot(
                &ObjectSnapshot::from_object_with_calculated_characteristics(
                    game.object(id).unwrap(),
                    &game,
                ),
            )
        });
        game.move_object_by_effect(a, Zone::Graveyard).unwrap();
        game.move_object_by_effect(b, Zone::Graveyard).unwrap();
        let outcome = EffectOutcome::count(2)
            .with_affected_object_memory(vec![memories[0].clone()])
            .with_affected_object_memory(vec![memories[1].clone()]);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        apply_tagged_runtime_state(
            &game,
            &mut ctx,
            TagKey::new("group"),
            &outcome,
            TaggedRuntimeState::default(),
        );
        let snapshots = ctx.get_tagged_all("group").unwrap();
        assert_eq!(
            snapshots
                .iter()
                .map(|s| (s.object_id, s.controller, s.zone))
                .collect::<Vec<_>>(),
            vec![(a, alice, Zone::Battlefield), (b, bob, Zone::Battlefield)]
        );
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
    fn test_explicit_result_object_overrides_zone_change_lki_fallback() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, alice);
        let source = game.new_object_id();
        let pre_move_snapshot =
            ObjectSnapshot::from_object(game.object(creature).expect("creature"), &game);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let runtime = capture_tagged_runtime_state(
            &game,
            &Effect::new(crate::effects::MoveToZoneEffect::new(
                crate::target::ChooseSpec::SpecificObject(creature),
                Zone::Graveyard,
                false,
            )),
            &ctx,
        );
        let graveyard_id = game
            .move_object_by_effect(creature, Zone::Graveyard)
            .expect("creature should move");
        let outcome = EffectOutcome::count(1)
            .with_result_objects(vec![graveyard_id])
            .with_affected_objects(vec![graveyard_id])
            .with_affected_object_memory(vec![crate::effect::OutcomeObjectMemory::from_snapshot(
                &pre_move_snapshot,
            )]);

        apply_tagged_runtime_state(&game, &mut ctx, TagKey::new("moved"), &outcome, runtime);

        let tagged = ctx.get_tagged("moved").expect("moved result object");
        assert_eq!(tagged.object_id, graveyard_id);
        assert_eq!(tagged.zone, Zone::Graveyard);
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
    fn test_composed_tag_uses_affected_memory_when_no_target_prelude_exists() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = create_creature(&mut game, alice);
        let second = create_creature(&mut game, bob);
        game.set_current_controller(first, bob).expect("finite controller fixture must refresh successfully");
        game.set_current_controller(second, alice).expect("finite controller fixture must refresh successfully");
        let source = game.new_object_id();
        let first_snapshot = ObjectSnapshot::from_object(game.object(first).unwrap(), &game);
        let second_snapshot = ObjectSnapshot::from_object(game.object(second).unwrap(), &game);
        let first_memory = crate::effect::OutcomeObjectMemory::from_snapshot(&first_snapshot);
        let second_memory = crate::effect::OutcomeObjectMemory::from_snapshot(&second_snapshot);
        game.set_current_controller(first, alice).expect("finite controller fixture must refresh successfully");
        game.set_current_controller(second, bob).expect("finite controller fixture must refresh successfully");
        let outcome = EffectOutcome::aggregate_summing_counts([
            EffectOutcome::count(1).with_affected_object_memory(vec![first_memory]),
            EffectOutcome::count(1).with_affected_object_memory(vec![second_memory]),
        ]);
        let mut ctx = ExecutionContext::new_default(source, alice);

        apply_tagged_runtime_state(
            &game,
            &mut ctx,
            TagKey::new("sacrificed"),
            &outcome,
            TaggedRuntimeState::default(),
        );

        let tagged = ctx
            .get_tagged_all("sacrificed")
            .expect("complete sacrifice LKI result set");
        assert_eq!(tagged.len(), 2);
        let tagged_first = tagged
            .iter()
            .find(|snapshot| snapshot.object_id == first)
            .expect("first sacrificed permanent");
        let tagged_second = tagged
            .iter()
            .find(|snapshot| snapshot.object_id == second)
            .expect("second sacrificed permanent");
        assert_eq!(tagged_first.controller, bob);
        assert_eq!(tagged_second.controller, alice);
        assert_eq!(tagged_first.zone, Zone::Battlefield);
        assert_eq!(tagged_second.zone, Zone::Battlefield);
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
        game.set_current_controller(creature, bob).expect("finite controller fixture must refresh successfully");
        let equipment = crate::card::CardBuilder::new(crate::ids::CardId::new(), "LKI Equipment")
            .card_types(vec![crate::types::CardType::Artifact])
            .subtypes(vec![crate::types::Subtype::Equipment])
            .build();
        let attachment = game.create_object_from_card(&equipment, alice, Zone::Battlefield);
        assert!(
            crate::effects::permanents::attach_battlefield_object_to_target(
                &mut game,
                attachment,
                crate::object::AttachmentTarget::Object(creature),
            )
        );
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
        assert_eq!(
            tagged.attachments,
            vec![attachment],
            "zone-change tags must retain the complete pre-move attachment set"
        );
    }
}
