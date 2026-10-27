//! Public cost identities needed for announcements (CR 601.5). Selecting an
//! identity here neither pays its cost nor publishes a completed action.
use super::*;
use crate::cost::prospective_references::{self, CostReferenceBindings};
use crate::effects::ChooseObjectsEffect;
use crate::filter::ObjectFilterExt as _;
use ironsmith_core::tag::TagKeyWalk;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub(crate) struct SpellCostReferences {
    pub ready: bool,
    initialized: bool,
    choices: Vec<ChooseObjectsEffect>,
    selected: CostReferenceBindings,
}

fn effects(game: &GameState, pending: &PendingCast) -> Vec<Effect> {
    game.object(pending.spell_id)
        .and_then(|spell| spell.spell_effect.as_ref())
        .map(|program| program.all_effects_owned())
        .unwrap_or_default()
}

fn collect_needed(effect: &Effect, needed: &mut HashSet<crate::tag::TagKey>) {
    if let Some(value) = effect.0.get_target_distribution_value() {
        value.for_each_tag_key(&mut |tag| {
            needed.insert(tag.clone());
        });
    }
    if let Some(spec) = effect.0.get_target_spec().filter(|spec| spec.is_target()) {
        spec.for_each_tag_key(&mut |tag| {
            needed.insert(tag.clone());
        });
    }
    effect.visit_child_effects(&mut |child| collect_needed(child, needed));
}

fn initialize(game: &GameState, pending: &mut PendingCast) -> Result<(), GameLoopError> {
    let body = effects(game, pending);
    let mut needed = HashSet::new();
    for effect in &body {
        collect_needed(effect, &mut needed);
    }
    if needed.is_empty() {
        return Ok(());
    }
    let steps = super::priority_cast::collect_spell_cost_steps(
        game,
        pending.spell_id,
        pending.caster,
        &pending.casting_method,
        &pending.optional_costs_paid,
        &pending.splice_costs,
        pending.chosen_modes.as_deref(),
        pending.chosen_targets.len(),
        pending.from_zone,
    );
    for step in steps {
        let choices = match step {
            ActivationCostStep::Cost(cost) => prospective_references::activation_reference_choices(
                &crate::cost::TotalCost::from_cost(cost),
                &body,
            )
            .map_err(|error| GameLoopError::InvalidState(format!("cost reference: {error}")))?,
            ActivationCostStep::Sacrifice {
                mut filter,
                choice_tag: Some(tag),
                ..
            } if needed.contains(&tag) => {
                filter.zone = Some(Zone::Battlefield);
                vec![ChooseObjectsEffect::new(
                    filter,
                    1,
                    crate::target::PlayerFilter::You,
                    tag,
                )]
            }
            _ => Vec::new(),
        };
        for choice in choices {
            if !pending
                .cost_references
                .choices
                .iter()
                .any(|previous| previous.tag == choice.tag)
            {
                pending.cost_references.choices.push(choice);
            }
        }
    }
    Ok(())
}

pub(super) fn continue_announcement(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if !pending.cost_references.initialized {
        initialize(game, &mut pending)?;
        pending.cost_references.initialized = true;
    }
    if let Some(choice) = pending.cost_references.choices.first() {
        let candidates = prospective_references::public_reference_candidates(
            game,
            pending.spell_id,
            pending.caster,
            choice,
            &pending.tagged_objects,
            pending.x_value,
        );
        if candidates.is_empty() {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "No eligible public cost reference".into(),
            ));
        }
        let context = crate::decisions::context::SelectObjectsContext::new(
            pending.caster,
            Some(pending.spell_id),
            "Choose the object used to determine this spell's targets or division",
            candidates
                .into_iter()
                .map(|id| {
                    crate::decisions::context::SelectableObject::new(
                        id,
                        game.current_name(id).unwrap_or_default(),
                    )
                })
                .collect(),
            1,
            Some(1),
        );
        pending.stage = CastStage::ChoosingCostReferences;
        state.pending_cast = Some(pending);
        return Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(context),
        ));
    }
    pending.cost_references.ready = true;
    super::priority_cast::continue_to_targets_or_mana_payment(
        game,
        trigger_queue,
        state,
        pending,
        decision_maker,
    )
}

pub(super) fn apply_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    selected: &[ObjectId],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state
        .pending_cast
        .take()
        .ok_or_else(|| GameLoopError::InvalidState("No spell cost reference is pending".into()))?;
    let choice = pending
        .cost_references
        .choices
        .first()
        .ok_or_else(|| GameLoopError::InvalidState("Missing spell cost reference".into()))?;
    let candidates = prospective_references::public_reference_candidates(
        game,
        pending.spell_id,
        pending.caster,
        choice,
        &pending.tagged_objects,
        pending.x_value,
    );
    let [id] = selected else {
        state.pending_cast = Some(pending);
        return Err(ResponseError::IllegalChoice("Choose one public cost object".into()).into());
    };
    if !candidates.contains(id) {
        state.pending_cast = Some(pending);
        return Err(
            ResponseError::IllegalChoice("The public cost object is not eligible".into()).into(),
        );
    }
    let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(*id).unwrap(),
        game,
    );
    let tag = choice.tag.clone();
    pending
        .cost_references
        .selected
        .insert(tag.clone(), vec![snapshot.clone()]);
    pending.tagged_objects.insert(tag, vec![snapshot]);
    pending.cost_references.choices.remove(0);
    continue_announcement(game, trigger_queue, state, pending, decision_maker)
}

pub(super) fn lock_payment(
    game: &GameState,
    pending: &mut PendingCast,
) -> Result<(), GameLoopError> {
    if pending.cost_references.selected.is_empty() {
        return Ok(());
    }
    for step in &mut pending.remaining_cost_steps {
        match step {
            ActivationCostStep::Cost(cost) => {
                let locked = prospective_references::lock_activation_reference_cost(
                    game,
                    pending.spell_id,
                    pending.caster,
                    &crate::cost::TotalCost::from_cost(cost.clone()),
                    &pending.tagged_objects,
                    &pending.cost_references.selected,
                    pending.x_value,
                )
                .map_err(|error| GameLoopError::InvalidState(format!("cost reference: {error}")))?;
                *cost = locked.costs()[0].clone();
            }
            ActivationCostStep::Sacrifice {
                filter,
                choice_tag: Some(tag),
                ..
            } => {
                if let Some(cards) = pending.cost_references.selected.get(tag) {
                    filter.specific = Some(cards[0].object_id);
                }
            }
            _ => {}
        }
    }
    Ok(())
}
