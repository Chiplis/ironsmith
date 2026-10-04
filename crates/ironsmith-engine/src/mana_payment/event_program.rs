//! Immediate mana-event closure without cloning or mutating the game.
//!
//! This stage accepts only expressions proven independent of its own writes.
//! Credits retain their real source/recipient and ordinary triggers remain data
//! for authoritative replay. Unsupported state dependencies are not negatives.
use super::program::ManaProduction;
use super::resources::{ManaCredit, ManaCreditContext};
use crate::ability::AbilityKind;
use crate::derived_view::DerivedGameView;
use crate::effects::ExecutionContext;
use crate::events::ManaAddedEvent;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
#[cfg(test)]
use crate::target::PlayerFilter;
use crate::triggers::{TriggerEvent, TriggeredAbilityEntry};

#[derive(Clone)]
pub(super) struct EvaluatedManaEvents {
    pub credits: Vec<ManaCredit>,
    pub ordinary_triggers: Vec<TriggeredAbilityEntry>,
}

pub(super) fn triggers_are_independent(game: &GameState, view: &DerivedGameView<'_>) -> bool {
    game.objects_map().values().all(|object| {
        view.abilities_rc(object.id).is_none_or(|abilities| {
            abilities.iter().all(|ability| {
                let AbilityKind::Triggered(trigger) = &ability.kind else {
                    return true;
                };
                if trigger.choices.iter().any(crate::target::ChooseSpec::is_target)
                    || !ability.functions_in(&object.zone)
                    || trigger.trigger.subscribed_kinds().is_some_and(|kinds| {
                        !kinds.iter().any(|kind| matches!(kind,
                            crate::events::EventKind::ManaAdded | crate::events::EventKind::AbilityActivated))
                    })
                    || !trigger
                        .effects
                        .all_effects()
                        .into_iter()
                        .any(|effect| effect.contains_mana_production())
                {
                    return true;
                }
                let filter_is_independent = |filter: &crate::target::ObjectFilter| {
                    !GameState::filter_reads_tapped_state_or_activation_history(filter, true)
                };
                let known = if let Some(matcher) = trigger
                    .trigger
                    .downcast_ref::<crate::triggers::TapForManaTrigger>()
                {
                    filter_is_independent(&matcher.filter)
                } else if let Some(matcher) = trigger
                    .trigger
                    .downcast_ref::<crate::triggers::AbilityActivatedTrigger>(
                ) {
                    filter_is_independent(&matcher.filter)
                } else if trigger
                    .trigger
                    .downcast_ref::<crate::triggers::ManaAddedTrigger>()
                    .is_some()
                {
                    true
                } else {
                    trigger.trigger.compiled_model().is_some_and(|model| {
                        use ironsmith_core::trigger_model::TriggerKind;
                        match &model.kind {
                            TriggerKind::PlayerTapsForMana { filter, .. }
                            | TriggerKind::AbilityActivatedQualified { filter, .. } => {
                                filter_is_independent(filter)
                            }
                            _ => false,
                        }
                    })
                };
                known
                    && trigger.choices.is_empty()
                    && trigger.intervening_if.is_none()
                    && trigger
                        .effects
                        .segments
                        .iter()
                        .all(|segment| segment.self_replacements.is_empty())
                    && trigger.effects.iter().all(|effect| {
                        effect.mana_production().is_some_and(ManaProduction::stable_for_trigger)
                    })
            })
        })
    })
}

pub(super) fn evaluate_with_context(
    game: &GameState,
    view: &DerivedGameView<'_>,
    initial: Vec<TriggerEvent>,
    tapped_source: ObjectId,
    event_budget: usize,
    replacements: &super::replacement_program::CompiledManaReplacements,
    mut credit_context: ManaCreditContext,
) -> Option<EvaluatedManaEvents> {
    let mut result = EvaluatedManaEvents {
        credits: Vec::new(),
        ordinary_triggers: Vec::new(),
    };
    let mut events = initial;
    let mut evaluated = 0usize;
    let mut replacement_resources = super::replacement_program::ReplacementResources::default();
    while !events.is_empty() {
        let mut pending = Vec::new();
        for event in events.drain(..) {
            evaluated = evaluated.checked_add(1)?;
            if evaluated > event_budget {
                return None;
            }
            let event = if let Some(mana) = event.downcast::<ManaAddedEvent>() {
                let mana = replacements.deterministic_event(
                    game,
                    mana.clone(),
                    &mut replacement_resources,
                )?;
                if mana.mana.is_empty() {
                    continue;
                }
                result.credits.push(ManaCredit { event: mana.clone(), context: credit_context.clone() });
                mana.into_trigger_event()
            } else {
                event
            };
            pending.extend(crate::triggers::check::check_triggers_with_view(
                game, &event, view,
            ));
        }
        events = resolve_trigger_batch(game, view, pending, tapped_source, &mut result)?;
        credit_context = ManaCreditContext::default();
    }
    Some(result)
}

fn resolve_trigger_batch(
    game: &GameState,
    view: &DerivedGameView<'_>,
    pending: Vec<TriggeredAbilityEntry>,
    tapped_source: ObjectId,
    result: &mut EvaluatedManaEvents,
) -> Option<Vec<TriggerEvent>> {
    let mut branches = resolve_trigger_batch_branches(game, view, pending, tapped_source, result, 1)?;
    (branches.len() == 1).then(|| branches.remove(0))
}

fn resolve_trigger_batch_branches(
    game: &GameState,
    view: &DerivedGameView<'_>,
    mut pending: Vec<TriggeredAbilityEntry>,
    tapped_source: ObjectId,
    result: &mut EvaluatedManaEvents,
    budget: usize,
) -> Option<Vec<Vec<TriggerEvent>>> {
    let mut branches = vec![Vec::new()];
    // These admitted expressions are independent of within-batch ordering.
    pending.sort_by_key(|entry| !game.is_active_player(entry.controller));
    for entry in pending {
        if !crate::game_loop::is_triggered_mana_ability(game, &entry) {
            result.ordinary_triggers.push(entry);
            continue;
        }
        let object = game.object(entry.source)?;
        let chars = view.calculated_characteristics_arc(entry.source)?;
        let mut snapshot = crate::snapshot::ObjectSnapshot::from_object_with_known_characteristics(
            object, game, Some(&chars),
        );
        if entry.source == tapped_source { snapshot.tapped = true; }
        let ctx = ExecutionContext::new_default(entry.source, entry.controller)
            .with_triggering_event(entry.triggering_event.clone())
            .with_source_snapshot(snapshot.clone());
        for effect in entry.ability.effects.iter() {
            let production = effect.mana_production()?;
            if !production.stable_for_trigger() { return None; }
            let resolved = production.resolve(game, &ctx).ok()?;
            let outputs = resolved.output.alternatives(budget)?;
            // A plan cannot prescribe another player's production choice.
            if outputs.len() > 1 && resolved.player != view.current_controller(tapped_source)? {
                return None;
            }
            if branches.len().checked_mul(outputs.len())? > budget { return None; }
            branches = branches.into_iter().flat_map(|prefix| outputs.iter().map(|symbols| {
                let mut events = prefix.clone();
                if !symbols.is_empty() {
                    events.push(ManaAddedEvent::new(entry.source, entry.controller, resolved.player, symbols.clone())
                        .with_snapshot(Some(snapshot.clone())).into_trigger_event());
                }
                events
            }).collect::<Vec<_>>()).collect();
        }
    }
    Some(branches)
}

/// Replacement history is grouped by original production instruction, so
/// identical successive mana events cannot consume each other's witnesses.
#[derive(Debug, Clone)]
pub(super) struct ManaEventWitness {
    pub original: ManaAddedEvent,
    pub decisions: Vec<super::replacement_program::ReplacementDecision>,
}

#[derive(Clone)]
pub(super) struct BranchedManaEvents {
    pub result: EvaluatedManaEvents,
    pub resources: super::replacement_program::ReplacementResources,
    pub witnesses: Vec<ManaEventWitness>,
}

/// Enumerate immediate event closure while carrying resource state from each
/// production into the next. No branch owns or clones a GameState.
pub(super) fn evaluate_branches_with_context(
    game: &GameState,
    view: &DerivedGameView<'_>,
    initial: Vec<TriggerEvent>,
    tapped_source: ObjectId,
    budget: usize,
    replacements: &super::replacement_program::CompiledManaReplacements,
    resources: &super::replacement_program::ReplacementResources,
    credit_context: ManaCreditContext,
) -> Option<Vec<BranchedManaEvents>> {
    #[derive(Clone)]
    struct Work {
        events: std::collections::VecDeque<TriggerEvent>,
        pending: Vec<TriggeredAbilityEntry>,
        credit_context: ManaCreditContext,
        branch: BranchedManaEvents,
    }
    let mut work = vec![Work {
        events: initial.into(),
        credit_context,
        pending: vec![],
        branch: BranchedManaEvents {
            result: EvaluatedManaEvents {
                credits: vec![],
                ordinary_triggers: vec![],
            },
            resources: resources.clone(),
            witnesses: vec![],
        },
    }];
    let mut complete = Vec::new();
    let mut visited = 0usize;
    while let Some(mut state) = work.pop() {
        visited = visited.checked_add(1)?;
        if visited > budget {
            return None;
        }
        if state.events.is_empty() {
            if state.pending.is_empty() {
                complete.push(state.branch);
                continue;
            }
            state.credit_context = ManaCreditContext::default();
            let batches = resolve_trigger_batch_branches(
                game, view, std::mem::take(&mut state.pending), tapped_source,
                &mut state.branch.result, budget - visited,
            )?;
            for events in batches.into_iter().rev() {
                let mut next = state.clone();
                next.events = events.into();
                work.push(next);
            }
            continue;
        }
        let event = state.events.pop_front()?;
        if let Some(original) = event.downcast::<ManaAddedEvent>() {
            let branches = replacements.branches(
                game,
                original.clone(),
                &state.branch.resources,
                budget - visited,
            )?;
            for replacement in branches.into_iter().rev() {
                let mut next = state.clone();
                next.branch.resources = replacement.resources;
                next.branch.witnesses.push(ManaEventWitness {
                    original: original.clone(),
                    decisions: replacement.decisions,
                });
                if !replacement.event.mana.is_empty() {
                    next.branch.result.credits.push(ManaCredit { event: replacement.event.clone(), context: next.credit_context.clone() });
                    next.pending
                        .extend(crate::triggers::check::check_triggers_with_view(
                            game,
                            &replacement.event.into_trigger_event(),
                            view,
                        ));
                }
                work.push(next);
            }
        } else {
            state
                .pending
                .extend(crate::triggers::check::check_triggers_with_view(
                    game, &event, view,
                ));
            work.push(state);
        }
    }
    Some(complete)
}

#[cfg(test)]
fn evaluate(game: &GameState, view: &DerivedGameView<'_>, initial: Vec<TriggerEvent>,
    tapped_source: ObjectId, budget: usize,
    replacements: &super::replacement_program::CompiledManaReplacements) -> Option<EvaluatedManaEvents> {
    evaluate_with_context(game, view, initial, tapped_source, budget, replacements, ManaCreditContext::default())
}

#[cfg(test)]
fn evaluate_branches(game: &GameState, view: &DerivedGameView<'_>, initial: Vec<TriggerEvent>,
    tapped_source: ObjectId, budget: usize, replacements: &super::replacement_program::CompiledManaReplacements,
    resources: &super::replacement_program::ReplacementResources) -> Option<Vec<BranchedManaEvents>> {
    evaluate_branches_with_context(game, view, initial, tapped_source, budget, replacements, resources, ManaCreditContext::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::ids::CardId;
    use crate::target::ObjectFilter;
    use crate::triggers::Trigger;
    use crate::types::CardType;
    use crate::{Ability, CardBuilder, Zone};

    fn permanent(game: &mut GameState, player: PlayerId, name: &str, kind: CardType) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![kind])
            .build();
        game.create_object_from_card(&card, player, Zone::Battlefield)
    }

    #[test]
    fn immediate_chain_preserves_recipients_sources_and_ordinary_triggers() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let land = permanent(&mut game, alice, "Land", CardType::Land);
        let bonus = permanent(&mut game, alice, "Tap bonus", CardType::Enchantment);
        let chain = permanent(&mut game, alice, "Mana response", CardType::Enchantment);
        game.object_mut(bonus)
            .unwrap()
            .abilities_mut()
            .push(Ability::triggered(
                Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::land()),
                vec![Effect::add_mana_player(
                    vec![ManaSymbol::Blue],
                    PlayerFilter::Specific(bob),
                )],
            ));
        game.object_mut(chain)
            .unwrap()
            .abilities_mut()
            .push(Ability::triggered(
                Trigger::mana_added(PlayerFilter::Opponent),
                vec![Effect::add_mana(vec![ManaSymbol::Red])],
            ));
        game.object_mut(chain)
            .unwrap()
            .abilities_mut()
            .push(Ability::triggered(
                Trigger::mana_added(PlayerFilter::You),
                vec![Effect::gain_life(1)],
            ));
        game.refresh_continuous_state().unwrap();
        let initial = ManaAddedEvent::new(land, alice, alice, vec![ManaSymbol::Green])
            .with_production_provenance(
                crate::events::mana::ManaProductionProvenance::TappedSourceForMana,
            )
            .into_trigger_event();
        let projected = {
            let view = DerivedGameView::new(&game);
            assert!(triggers_are_independent(&game, &view));
            evaluate(
                &game,
                &view,
                vec![initial.clone()],
                land,
                16,
                &super::super::replacement_program::CompiledManaReplacements::compile(&game)
                    .unwrap(),
            )
            .unwrap()
        };
        assert_eq!(projected.credits.len(), 3);
        assert_eq!(
            projected
                .credits
                .iter()
                .map(|credit| (credit.event.source, credit.event.player, credit.event.mana.clone()))
                .collect::<Vec<_>>(),
            vec![
                (land, alice, vec![ManaSymbol::Green]),
                (bonus, bob, vec![ManaSymbol::Blue]),
                (chain, alice, vec![ManaSymbol::Red])
            ]
        );
        assert_eq!(projected.ordinary_triggers.len(), 2);
        assert!(projected.credits[1..].iter().all(
            |credit| credit.event.provenance == crate::events::mana::ManaProductionProvenance::Unknown
        ));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);

        // Differential execution starts with the same original committed event.
        game.tap(land);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        game.queue_trigger_event(crate::provenance::ProvNodeId::default(), initial);
        crate::game_loop::resolve_pending_mana_triggers(
            &mut game,
            &mut crate::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);
        assert_eq!(game.player(bob).unwrap().mana_pool.blue, 1);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.effect_store.pending_trigger_entries.len(), 2);
    }

    #[test]
    fn branching_events_preserve_optional_resource_history_even_for_equal_pools() {
        use super::super::replacement_program::{CompiledManaReplacements, ReplacementResources};
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = permanent(&mut game, alice, "Branching land", CardType::Land);
        let mut replacement = crate::replacement::ReplacementEffect::with_matcher(
            source,
            alice,
            crate::events::mana::matchers::ManaProducedBySourceMatcher::new(ObjectFilter::land()),
            crate::replacement::ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue; 2]),
        );
        replacement.optional = true;
        let id = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        game.refresh_continuous_state().unwrap();
        let program = CompiledManaReplacements::compile(&game).unwrap();
        let initial =
            ManaAddedEvent::new(source, alice, alice, vec![ManaSymbol::Green]).into_trigger_event();
        let view = DerivedGameView::new(&game);
        let branches = evaluate_branches(
            &game,
            &view,
            vec![initial.clone(), initial.clone()],
            source,
            128,
            &program,
            &ReplacementResources::default(),
        )
        .unwrap();
        assert_eq!(
            branches.len(),
            3,
            "apply first, apply second, or decline both"
        );
        assert_eq!(
            branches
                .iter()
                .filter(|branch| branch.resources.consumed.len() == 1)
                .count(),
            2
        );
        for branch in &branches {
            assert_eq!(branch.witnesses.len(), 2);
            let mut resources = ReplacementResources::default();
            let mut replayed = Vec::new();
            for witness in &branch.witnesses {
                let replay = program
                    .replay(
                        &game,
                        witness.original.clone(),
                        &resources,
                        &witness.decisions,
                    )
                    .unwrap();
                resources = replay.resources;
                replayed.push(replay.event.mana);
            }
            assert_eq!(
                replayed,
                branch
                    .result
                    .credits
                    .iter()
                    .map(|credit| credit.event.mana.clone())
                    .collect::<Vec<_>>()
            );
            assert_eq!(resources.consumed, branch.resources.consumed);
            let next = evaluate_branches(
                &game,
                &view,
                vec![initial.clone()],
                source,
                64,
                &program,
                &resources,
            )
            .unwrap();
            assert_eq!(
                next.len(),
                if resources.consumed.is_empty() { 2 } else { 1 }
            );
        }
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(id)
                .is_some()
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn unbounded_mana_trigger_chain_returns_unknown_without_mutation() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = permanent(
            &mut game,
            alice,
            "Recurring production",
            CardType::Enchantment,
        );
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(Ability::triggered(
                Trigger::mana_added(PlayerFilter::You),
                vec![Effect::add_mana(vec![ManaSymbol::Green])],
            ));
        game.refresh_continuous_state().unwrap();
        let view = DerivedGameView::new(&game);
        assert!(triggers_are_independent(&game, &view));
        let event =
            ManaAddedEvent::new(source, alice, alice, vec![ManaSymbol::Green]).into_trigger_event();
        assert!(
            evaluate(
                &game,
                &view,
                vec![event],
                source,
                8,
                &super::super::replacement_program::CompiledManaReplacements::compile(&game)
                    .unwrap()
            )
            .is_none()
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert!(!game.is_tapped(source));
    }
}
