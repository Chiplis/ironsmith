//! Effect-backed cost component.
//!
//! This lets costs flow through the normal effect executor/event pipeline
//! while still being represented as a first-class `Cost` inside `TotalCost`.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::effect::Effect;
#[cfg(test)]
use crate::effects::ExecutionContext;
use crate::effects::execute_effect_payment_with_outputs;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;

/// Convert a CostValidationError to CostPaymentError.
fn convert_validation_error(err: CostValidationError) -> CostPaymentError {
    match err {
        CostValidationError::AlreadyTapped => CostPaymentError::AlreadyTapped,
        CostValidationError::AlreadyUntapped => CostPaymentError::AlreadyUntapped,
        CostValidationError::SummoningSickness => CostPaymentError::SummoningSickness,
        CostValidationError::NotEnoughLife => CostPaymentError::InsufficientLife,
        CostValidationError::NotEnoughEnergy => CostPaymentError::InsufficientEnergy,
        CostValidationError::NotEnoughCards => CostPaymentError::InsufficientCardsInHand,
        CostValidationError::CannotSacrifice => CostPaymentError::NoValidSacrificeTarget,
        CostValidationError::ExecutionFailed(error) => CostPaymentError::ExecutionFailed(error),
        CostValidationError::Other(msg) => CostPaymentError::Other(msg),
    }
}

/// A cost paid by executing a single effect.
#[derive(Debug, Clone)]
pub struct CostEffect {
    /// Effect executed as part of paying this cost.
    effect: Effect,
}

impl CostEffect {
    pub fn new<E: CostExecutableEffect + 'static>(effect: E) -> Self {
        let canonical = effect.canonical_cost_effect();
        Self {
            effect: canonical.unwrap_or_else(|| Effect::new(effect)),
        }
    }

    pub fn effect(&self) -> &Effect {
        &self.effect
    }

    pub fn try_new(effect: Effect) -> Result<Self, String> {
        Self::from_validated_effect(effect)
    }

    pub fn from_validated_effect(effect: Effect) -> Result<Self, String> {
        if effect.0.as_cost_executable().is_some() {
            let canonical = effect
                .0
                .as_cost_executable()
                .and_then(|cost| cost.canonical_cost_effect());
            Ok(Self {
                effect: canonical.unwrap_or(effect),
            })
        } else {
            Err(format!(
                "effect is not marked as cost-executable: {effect:?}"
            ))
        }
    }
}

impl PartialEq for CostEffect {
    fn eq(&self, _other: &Self) -> bool {
        // Effect partial-eq is intentionally behavioral/not structural.
        false
    }
}

/// Peel wrappers that deliberately preserve the payment semantics of their
/// child effect. Cost validation sometimes needs the concrete payload (for
/// example, a move that consumes an object selected by an earlier cost), while
/// execution must still retain the wrappers so their tags and outcomes are
/// recorded.
fn transparent_cost_effect(mut effect: &Effect) -> &Effect {
    while let Some(inner) = effect.0.transparent_cost_precheck_child_effect() {
        effect = inner;
    }
    effect
}

/// Locate the typed selected-cost binding owned by this component. Other
/// completed cost tags in the context belong to earlier components.
fn original_sacrifice_result_tags(mut effect: &Effect) -> Vec<crate::tag::TagKey> {
    use ironsmith_core::tag::SacrificeCostTag;
    let mut tags = Vec::new();
    let mut retain = |tag: &crate::tag::TagKey| {
        if let Some(selected @ SacrificeCostTag::Selected(_)) = SacrificeCostTag::parse(tag) {
            let result = selected.original_result_key();
            if !tags.contains(&result) { tags.push(result); }
        }
    };
    loop {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() { retain(&tagged.tag); }
        let Some(inner) = effect.transparent_child_effect() else { break; };
        effect = inner;
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        for constraint in &sacrifice.filter.tagged_constraints {
            if constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject { retain(&constraint.tag); }
        }
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()
        && let crate::target::ChooseSpec::Tagged(tag) = sacrifice.target.base() { retain(tag); }
    tags
}

/// Cost adapters publish the same actual sacrifice binding for ordinary and
/// prepared payments. A selected original alone does not prove it was sacrificed.
fn retain_original_sacrifice_bindings(
    effect: &Effect, game: &GameState, outcome: &crate::effect::EffectOutcome,
    execution: &mut crate::effects::ExecutionContext,
) -> Result<Option<Vec<crate::snapshot::ObjectSnapshot>>, CostPaymentError> {
    let action = transparent_cost_effect(effect);
    if action.downcast_ref::<crate::effects::SacrificeEffect>().is_none()
        && action.downcast_ref::<crate::effects::SacrificeTargetEffect>().is_none()
    {
        return Ok(None);
    }
    let memory = outcome.instruction_result().execution_facts.iter().rev().find_map(|fact| match fact {
        crate::effect::ExecutionFact::OriginalSacrificeObjects(memory) => Some(memory),
        _ => None,
    }).ok_or_else(|| CostPaymentError::ExecutionFailed(crate::effects::ExecutionError::IncompleteEvidence(
        "completed sacrifice cost lacks its original-action receipt".into(),
    )))?;
    let snapshots = memory.iter().map(|object| object.to_snapshot(game)).collect::<Vec<_>>();
    for tag in original_sacrifice_result_tags(effect) {
        execution.set_tagged_objects(tag, snapshots.clone());
    }
    Ok(Some(snapshots))
}

fn sacrifice_target_cost_object(
    effect: &crate::effects::SacrificeTargetEffect,
    game: &GameState,
    ctx: &CostContext,
) -> Result<crate::ids::ObjectId, CostPaymentError> {
    // Source and captured-object costs name the original permanent. They
    // must not fall back to its later incarnation or a different permanent,
    // and only its current controller can pay by sacrificing it.
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let exec = ctx.execution_bindings().execution_context(&mut decision_maker);
    let objects = if matches!(effect.target.base(), crate::target::ChooseSpec::Source) {
        vec![ctx.source]
    } else {
        match crate::effects::helpers::resolve_objects_from_spec(game, &effect.target, &exec) {
            Ok(objects) => objects,
            Err(crate::effects::ExecutionError::InvalidTarget
                | crate::effects::ExecutionError::TagNotFound(_)) => {
                return Err(CostPaymentError::NoValidSacrificeTarget);
            }
            Err(error) => return Err(CostPaymentError::ExecutionFailed(error)),
        }
    };
    let [id] = objects.as_slice() else {
        return Err(CostPaymentError::NoValidSacrificeTarget);
    };
    game.object(*id).is_some_and(|object| {
        object.zone == crate::zone::Zone::Battlefield
            && game.controller_of(object) == ctx.payer
            && !game.is_phased_out(*id)
            && game.can_be_sacrificed_with_cause(*id, &ctx.event_cause())
    }).then_some(*id).ok_or(CostPaymentError::NoValidSacrificeTarget)
}

fn sacrifice_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    let effect = transparent_cost_effect(effect);
    if let Some(effect) = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>() {
        return Some(sacrifice_target_cost_object(effect, game, ctx).map(|_| ()));
    }
    let (filter, count, player) = if let Some(effect) =
        effect.downcast_ref::<crate::effects::SacrificeEffect>()
    {
        (&effect.filter, &effect.count, &effect.player)
    } else if let Some(effect) = effect.downcast_ref::<ironsmith_core::SacrificePlayerEffect>() {
        (&effect.filter, &effect.count, &effect.player)
    } else {
        return None;
    };

    if filter
        .tagged_constraints
        .iter()
        .any(|constraint| !ctx.tagged_objects.contains_key(constraint.tag.as_str()))
    {
        return Some(Err(CostPaymentError::NoValidSacrificeTarget));
    }

    if player != &crate::target::PlayerFilter::You {
        return Some(Err(CostPaymentError::Other(
            "sacrifice costs support only 'you'".to_string(),
        )));
    }

    let required = match count {
        crate::effect::Value::Fixed(count) => (*count).max(0) as usize,
        crate::effect::Value::Count(count_filter)
            if tagged_selection_tag(count_filter)
                .zip(tagged_selection_tag(filter))
                .is_some_and(|(count_tag, sacrifice_tag)| count_tag == sacrifice_tag) =>
        {
            filter
                .tagged_constraints
                .iter()
                .find(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                })
                .and_then(|constraint| ctx.tagged_objects.get(constraint.tag.as_str()))
                .map_or(0, Vec::len)
        }
        crate::effect::Value::X => ctx.x_value.unwrap_or(0) as usize,
        _ if filter.tagged_constraints.is_empty() => return None,
        _ => {
            let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
            let exec = ctx
                .execution_bindings()
                .execution_context(&mut decision_maker);
            match crate::effects::helpers::resolve_value(game, count, &exec) {
                Ok(value) => value.max(0) as usize,
                Err(error) => {
                    return Some(Err(CostPaymentError::ExecutionFailed(error)));
                }
            }
        }
    };

    if required == 0 {
        return Some(Ok(()));
    }

    let lands_only = ctx.reason.is_cast_or_ability_payment()
        && game.player_cant_sacrifice_nonland_to_cast_or_activate(ctx.payer);
    let filter_ctx = crate::filter::FilterContext::new(ctx.payer)
        .with_source(ctx.source)
        .with_tagged_objects(&ctx.tagged_objects);
    let available = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
        .filter(|(id, obj)| {
            game.controller_of(obj) == ctx.payer
                && (!lands_only || game.current_has_card_type(*id, crate::types::CardType::Land))
                && filter.matches(obj, &filter_ctx, game)
                && game.can_be_sacrificed_with_cause(*id, &ctx.event_cause())
        })
        .count();

    if available < required {
        Some(Err(CostPaymentError::NoValidSacrificeTarget))
    } else {
        Some(Ok(()))
    }
}

fn tagged_selection_tag(filter: &crate::filter::ObjectFilter) -> Option<&crate::tag::TagKey> {
    filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        .map(|constraint| &constraint.tag)
}

fn tagged_move_to_zone_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    let effect = transparent_cost_effect(effect);
    let move_to_zone = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let tag = match move_to_zone.target.base() {
        crate::target::ChooseSpec::Tagged(tag) => tag,
        crate::target::ChooseSpec::Object(filter) => tagged_selection_tag(filter)?,
        _ => return None,
    };

    let Some(chosen) = ctx.tagged_objects.get(tag.as_str()) else {
        return Some(Err(CostPaymentError::Other(
            "move-to-zone cost has no bound choice".into(),
        )));
    };
    if chosen.is_empty()
        || chosen.iter().any(|snapshot| {
            game.object(snapshot.object_id)
                .is_none_or(|object| object.zone != snapshot.zone)
        })
    {
        Some(Err(CostPaymentError::Other(
            "move-to-zone cost has no current chosen object".to_string(),
        )))
    } else {
        Some(Ok(()))
    }
}

fn tagged_exile_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    let effect = transparent_cost_effect(effect);
    let exile = effect.downcast_ref::<crate::effects::ExileEffect>()?;
    let tag = match exile.spec.base() {
        crate::target::ChooseSpec::Tagged(tag) => tag,
        crate::target::ChooseSpec::Object(filter) => tagged_selection_tag(filter)?,
        _ => return None,
    };

    let Some(chosen) = ctx.tagged_objects.get(tag.as_str()) else {
        return Some(Err(CostPaymentError::Other(
            "exile cost has no chosen object".to_string(),
        )));
    };

    // Cardinality belongs to the preceding ChooseObjects cost. An optional
    // choice may legitimately publish an empty tag, while a required choice
    // cannot do so because its own cost validation/execution enforces `min`.
    if chosen.iter().all(|snapshot| {
        game.object(snapshot.object_id)
            .is_some_and(|object| object.zone == snapshot.zone)
    }) {
        Some(Ok(()))
    } else {
        Some(Err(CostPaymentError::Other(
            "chosen object is no longer available to exile".to_string(),
        )))
    }
}

fn tagged_unattach_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    let effect = transparent_cost_effect(effect);
    let unattach = effect.downcast_ref::<crate::effects::UnattachObjectsEffect>()?;
    let tag = match unattach.objects.base() {
        crate::target::ChooseSpec::Tagged(tag) => tag,
        crate::target::ChooseSpec::Object(filter) => tagged_selection_tag(filter)?,
        _ => return None,
    };

    let Some(chosen) = ctx.tagged_objects.get(tag.as_str()) else {
        return Some(Err(CostPaymentError::Other(
            "unattach cost has no chosen object".to_string(),
        )));
    };
    if chosen.is_empty() {
        return Some(Err(CostPaymentError::Other(
            "unattach cost has no chosen object".to_string(),
        )));
    }

    let valid = chosen.iter().any(|snapshot| {
        game.find_object_by_stable_id(snapshot.stable_id)
            .and_then(|id| game.object(id))
            .is_some_and(|object| {
                object.attached_to.and_then(|target| target.object_id()) == Some(ctx.source)
            })
    });
    if valid {
        Some(Ok(()))
    } else {
        Some(Err(CostPaymentError::Other(
            "chosen object is not attached to this source".to_string(),
        )))
    }
}

/// Any other consumer of a preceding choice (for example "return the chosen
/// lands to their owner's hand"): once the choice has published its tag, the
/// consumer is payable while every chosen object is still present. Cardinality
/// belongs to the choice itself.
fn tagged_choice_consumer_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    // Presence of an external tag does not establish a compound's payment
    // eligibility. Retain opaque input scopes and let its contextual owner
    // validate every child rather than treating its first dependency as success.
    let effect = transparent_cost_effect(effect);
    let mut has_children = false;
    effect.0.visit_child_effects(&mut |_| has_children = true);
    if has_children {
        return None;
    }
    let bindings = effect.0.cost_choice_bindings();
    if bindings.required.len() != 1 || !bindings.published.is_empty() {
        return None;
    }
    let tag = &bindings.required[0];
    let Some(chosen) = ctx.tagged_objects.get(tag.as_str()) else {
        return Some(Err(CostPaymentError::Other(
            "cost consumer has no bound choice".into(),
        )));
    };
    if chosen.iter().all(|snapshot| {
        game.object(snapshot.object_id)
            .is_some_and(|object| object.zone == snapshot.zone)
    }) {
        Some(Ok(()))
    } else {
        Some(Err(CostPaymentError::Other(
            "chosen object is no longer available".to_string(),
        )))
    }
}

/// A written tap/untap cost needs an actual state change on each selected
/// current object. The choice handles its count and filter; it does not impose
/// the source-symbol summoning-sickness rule on these chosen permanents.
fn tagged_tap_state_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    let effect = transparent_cost_effect(effect);
    let (spec, tapped_after) = if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>()
    {
        (&tap.target, true)
    } else if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
        (&untap.target, false)
    } else {
        return None;
    };
    let crate::target::ChooseSpec::Tagged(tag) = spec.base() else {
        return None;
    };
    let Some(selected) = ctx.tagged_objects.get(tag.as_str()) else {
        return Some(Err(CostPaymentError::Other(
            "tap-state cost has no chosen objects".into(),
        )));
    };
    let valid = selected.iter().all(|snapshot| {
        game.object(snapshot.object_id).is_some_and(|object| {
            object.zone == crate::zone::Zone::Battlefield
                && !game.is_phased_out(object.id)
                && game.is_tapped(object.id) != tapped_after
                && (tapped_after || game.can_untap(object.id))
        })
    });
    Some(if valid {
        Ok(())
    } else {
        Err(CostPaymentError::Other(
            "chosen objects cannot pay the tap-state cost".into(),
        ))
    })
}

fn dynamic_counter_removal_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    let effect = transparent_cost_effect(effect)
        .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()?;
    let announced_x = ctx.x_value?;
    if !effect.dynamic_count || !effect.display_x {
        return None;
    }
    let available = crate::effects::counters::remove_any_counters_among_total_available(
        effect, game, ctx.source, ctx.payer,
    );
    if announced_x < effect.min_count
        || announced_x > effect.count
        || u64::from(announced_x) > available
    {
        Some(Err(CostPaymentError::Other(
            "not enough counters".to_string(),
        )))
    } else {
        Some(Ok(()))
    }
}

fn simple_exile_from_hand_filter(
    filter: &crate::filter::ObjectFilter,
) -> Option<Option<crate::color::ColorSet>> {
    let mut expected = crate::filter::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Hand)
        .owned_by(crate::target::PlayerFilter::You)
        .other();
    if let Some(colors) = filter.colors {
        expected = expected.with_colors(colors);
    }
    (filter == &expected).then_some(filter.colors)
}

fn simple_exile_from_graveyard_filter(
    filter: &crate::filter::ObjectFilter,
) -> Option<Option<crate::types::CardType>> {
    if filter.card_types.len() > 1 {
        return None;
    }

    let card_type = filter.card_types.first().copied();
    let mut expected = crate::filter::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Graveyard)
        .owned_by(crate::target::PlayerFilter::You);
    if let Some(card_type) = card_type {
        expected = expected.with_type(card_type);
    }
    (filter == &expected).then_some(card_type)
}

fn life_payment_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    let payment =
        transparent_cost_effect(effect).downcast_ref::<crate::effects::PayLifeEffect>()?;
    Some(ctx.with_execution_context(|execution| {
        CostExecutableEffect::can_execute_as_cost_with_context(payment, game, execution, ctx.reason)
            .map_err(convert_validation_error)
    }))
}

fn discard_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
    allow_unannounced_x: bool,
) -> Option<Result<(), CostPaymentError>> {
    let discard =
        transparent_cost_effect(effect).downcast_ref::<crate::effects::DiscardEffect>()?;
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let exec = ctx
        .execution_bindings()
        .execution_context(&mut decision_maker);
    Some(
        discard
            .check_cost_with_context(game, &exec, ctx.reason, allow_unannounced_x)
            .map_err(convert_validation_error),
    )
}

impl CostPayer for CostEffect {
    fn payment_x_from_prepared_payment(
        &self,
        proposal: &dyn crate::effects::SimultaneousEffectProposal,
        execution: &crate::effects::ExecutionContext,
    ) -> Result<Option<u32>, CostPaymentError> {
        self.effect
            .0
            .as_cost_executable()
            .ok_or_else(|| CostPaymentError::Other("cost effect lost its payment contract".into()))?
            .payment_x_from_prepared_payment(proposal, execution)
            .map_err(convert_validation_error)
    }

    fn finalize_payment_bindings(
        &self,
        game: &GameState,
        outcome: &crate::effect::EffectOutcome,
        execution: &mut crate::effects::ExecutionContext,
        payment_x: Option<u32>,
    ) -> Result<(), CostPaymentError> {
        self.effect
            .0
            .as_cost_executable()
            .ok_or_else(|| CostPaymentError::Other("cost effect lost its payment contract".into()))?
            .finalize_payment_bindings(game, outcome, execution, payment_x)?;
        retain_original_sacrifice_bindings(&self.effect, game, outcome, execution)?;
        Ok(())
    }

    fn payment_x_from_outcome(
        &self,
        outcome: &crate::effect::EffectOutcome,
        execution: &crate::effects::ExecutionContext,
    ) -> Result<Option<u32>, CostPaymentError> {
        self.effect
            .0
            .as_cost_executable()
            .ok_or_else(|| CostPaymentError::Other("cost effect lost its payment contract".into()))?
            .payment_x_from_outcome(outcome, execution)
            .map_err(convert_validation_error)
    }

    fn validate_payment_outcome(
        &self,
        outcome: &crate::effect::EffectOutcome,
    ) -> Result<(), CostPaymentError> {
        self.effect
            .0
            .as_cost_executable()
            .ok_or_else(|| CostPaymentError::Other("cost effect lost its payment contract".into()))?
            .validate_payment_outcome(outcome)
            .map_err(convert_validation_error)
    }

    fn supports_prepared_payment(&self) -> bool {
        self.effect
            .0
            .as_cost_executable()
            .is_some_and(|cost| cost.supports_prepared_payment())
    }

    fn prepare_simultaneous_payment(
        &self,
        game: &GameState,
        execution: &mut crate::effects::ExecutionContext,
    ) -> Result<
        Option<Box<dyn crate::effects::SimultaneousEffectProposal>>,
        crate::effects::ExecutionError,
    > {
        if !CostPayer::supports_prepared_payment(self) {
            return Ok(None);
        }
        let proposal = self.effect.prepare_simultaneous_payment(game, execution)?;
        let accepted = self
            .effect
            .0
            .as_cost_executable()
            .is_some_and(|cost| cost.accepts_prepared_payment(proposal.as_ref()));
        Ok(accepted.then_some(proposal))
    }

    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        if let Some(result) = life_payment_cost_precheck(&self.effect, game, ctx) {
            return result;
        }
        if !ctx.tagged_objects.is_empty()
            && let Some(choose) = transparent_cost_effect(&self.effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && choose.chooser == crate::target::PlayerFilter::You
            && choose.count == crate::ChoiceCount::exactly(1)
            && choose.count_value.is_none()
            && choose.aggregate_constraint.is_none()
            && !choose.top_only
            && !choose.bottom_only
            && !choose.is_search
            && choose.additional_zones.is_empty()
            && !crate::effects::composition::selection_relations::has_relations(&choose.filter)
            && matches!(
                choose.filter.zone.or(choose.zone),
                Some(
                    crate::zone::Zone::Battlefield
                        | crate::zone::Zone::Graveyard
                        | crate::zone::Zone::Exile
                )
            )
        {
            let candidates = crate::cost::prospective_references::public_reference_candidates(
                game,
                ctx.source,
                ctx.payer,
                choose,
                &ctx.tagged_objects,
                ctx.x_value,
            );
            return if candidates
                .iter()
                .all(|id| ctx.replacement.entry_reserved_objects.contains(id))
            {
                Err(CostPaymentError::Other(
                    "no eligible referenced cost object".into(),
                ))
            } else {
                Ok(())
            };
        }
        if let Some(choose) = transparent_cost_effect(&self.effect)
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && crate::effects::composition::selection_relations::has_relations(&choose.filter)
        {
            let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
            let exec = ctx
                .execution_bindings()
                .execution_context(&mut decision_maker);
            crate::effects::composition::choose_objects::check_relation_cost_with_context(
                choose, game, &exec,
            )
            .map_err(convert_validation_error)?;
            if choose.aggregate_constraint.is_none()
                && !choose.top_only
                && !choose.bottom_only
                && !choose.is_search
            {
                return Ok(());
            }
        }
        if let Some(reveal) = transparent_cost_effect(&self.effect)
            .downcast_ref::<crate::effects::RevealTaggedEffect>()
        {
            let selected = ctx.tagged_objects.get(&reveal.tag).ok_or_else(|| {
                CostPaymentError::Other("reveal cost has no bound selection".into())
            })?;
            return selected
                .iter()
                .all(|snapshot| {
                    game.object(snapshot.object_id)
                        .is_some_and(|object| object.zone == snapshot.zone)
                })
                .then_some(())
                .ok_or_else(|| {
                    CostPaymentError::Other("reveal cost selection changed zones".into())
                });
        }
        if let Some(result) = discard_cost_precheck(&self.effect, game, ctx, true) {
            return result;
        }
        if !ctx.replacement.entry_reserved_objects.is_empty()
            && let crate::costs::CostProcessingMode::DiscardCards { count, filter } =
                self.processing_mode()
            && crate::costs::legal_discard_cost_cards_in_context(game, ctx, &filter).len()
                < count as usize
        {
            return Err(CostPaymentError::InsufficientCardsInHand);
        }

        if let Some(evidence) = transparent_cost_effect(&self.effect)
            .downcast_ref::<crate::effects::CollectEvidenceEffect>()
        {
            let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
            let exec = ctx
                .execution_bindings()
                .execution_context(&mut decision_maker);
            let required = crate::effects::composition::collect_evidence::evidence_requirement(
                evidence, game, &exec,
            )
            .map_err(CostPaymentError::ExecutionFailed)?;
            let exclude =
                matches!(ctx.reason, crate::costs::PaymentReason::CastSpell).then_some(ctx.source);
            return (crate::effects::composition::collect_evidence::evidence_capacity(
                game, ctx.payer, exclude,
            ) >= required)
                .then_some(())
                .ok_or_else(|| {
                    CostPaymentError::Other("not enough mana value to collect evidence".into())
                });
        }
        if let Some(result) = sacrifice_cost_precheck(&self.effect, game, ctx) {
            return result;
        }
        if let Some(result) = tagged_move_to_zone_cost_precheck(&self.effect, game, ctx) {
            return result;
        }
        if let Some(result) = tagged_exile_cost_precheck(&self.effect, game, ctx) {
            return result;
        }
        if let Some(result) = tagged_unattach_cost_precheck(&self.effect, game, ctx) {
            return result;
        }
        if let Some(result) = tagged_tap_state_cost_precheck(&self.effect, game, ctx) {
            return result;
        }
        if let Some(result) = tagged_choice_consumer_cost_precheck(&self.effect, game, ctx) {
            return result;
        }
        if let Some(result) = dynamic_counter_removal_cost_precheck(&self.effect, game, ctx) {
            return result;
        }
        ctx.with_execution_context(|execution| {
            let cost =
                self.effect.0.as_cost_executable().ok_or_else(|| {
                    CostPaymentError::Other("effect is not cost-executable".into())
                })?;
            cost.can_execute_as_cost_with_context(game, execution, ctx.reason)
                .map_err(convert_validation_error)
        })
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        CostPayer::pay_with_outputs(self, game, ctx).map(|receipt| receipt.result)
    }

    fn pay_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<super::CostPaymentReceipt, CostPaymentError> {
        if let Some(result) = life_payment_cost_precheck(&self.effect, game, ctx) {
            result?;
        }
        if let Some(result) = discard_cost_precheck(&self.effect, game, ctx, false) {
            result?;
        }
        if let Some(result) = sacrifice_cost_precheck(&self.effect, game, ctx) {
            result?;
        } else if let Some(result) = tagged_move_to_zone_cost_precheck(&self.effect, game, ctx) {
            result?;
        } else if let Some(result) = tagged_exile_cost_precheck(&self.effect, game, ctx) {
            result?;
        } else if let Some(result) = tagged_unattach_cost_precheck(&self.effect, game, ctx) {
            result?;
        } else if let Some(result) = tagged_tap_state_cost_precheck(&self.effect, game, ctx) {
            result?;
        } else if let Some(result) = tagged_choice_consumer_cost_precheck(&self.effect, game, ctx) {
            result?;
        } else {
            self.can_pay(game, ctx)?;
        }

        let bindings = ctx.execution_bindings();
        let mut exec_ctx = bindings.execution_context(&mut *ctx.decision_maker);

        let mut outputs = crate::effects::with_per_event_trigger_matching(game, true, |game| {
            let mut outputs =
                execute_effect_payment_with_outputs(game, &self.effect, &mut exec_ctx)?;
            // Retain payment evidence for later typed amount/counter queries,
            // while freezing triggers before the next payment instruction.
            crate::effects::capture_triggers_before_added_program(
                game,
                &exec_ctx,
                None,
                outputs.outcome.events.iter_mut(),
            )?;
            Ok::<_, crate::effects::ExecutionError>(outputs)
        })
        .map_err(CostPaymentError::ExecutionFailed)?;
        if exec_ctx.decision_maker.awaiting_choice() {
            // Keep the legacy neutral return while the pending decision is
            // authoritative. No acknowledgement or payment bindings publish.
            return Ok(super::CostPaymentReceipt::new(CostPaymentResult::Paid));
        }
        outputs.synchronize_observations();
        CostPayer::validate_payment_outcome(self, &outputs.outcome)?;
        for event in outputs.outcome.events.iter().cloned() {
            game.queue_trigger_event(ctx.provenance, event);
        }

        if ctx.x_value.is_none() {
            ctx.x_value = exec_ctx.x_value;
        }

        ctx.completed_sacrifice = retain_original_sacrifice_bindings(
            &self.effect, game, &outputs.outcome, &mut exec_ctx,
        )?;
        ctx.execution_inputs = Some(Box::new(crate::effects::PaymentExecutionInputs::capture(
            &exec_ctx,
        )));
        ctx.tagged_objects = exec_ctx.tagged_objects;
        ctx.effect_outcomes = exec_ctx.effect_outcomes;
        ctx.pre_chosen_cards.clear();

        Ok(super::CostPaymentReceipt::from_outputs(outputs))
    }

    fn display(&self) -> String {
        // A cost is displayed to the player who has to pay it, so an effect
        // without its own cost wording says so plainly rather than printing the
        // compiled structure.
        self.effect
            .0
            .cost_description()
            .unwrap_or_else(|| "Perform the stated effect".to_string())
    }

    fn requires_tap(&self) -> bool {
        self.effect.0.is_tap_source_cost()
    }

    fn requires_untap(&self) -> bool {
        self.effect.0.is_untap_source_cost()
    }

    fn is_life_cost(&self) -> bool {
        self.effect.0.pay_life_amount().is_some()
    }

    fn life_amount(&self) -> Option<u32> {
        self.effect.0.pay_life_amount()
    }

    fn is_sacrifice_self(&self) -> bool {
        self.effect.0.is_sacrifice_source_cost()
    }

    fn is_sacrifice(&self) -> bool {
        self.sacrifice_filter().is_some()
    }

    fn sacrifice_filter(&self) -> Option<&crate::filter::ObjectFilter> {
        transparent_cost_effect(&self.effect)
            .downcast_ref::<crate::effects::SacrificeEffect>()
            .map(|effect| &effect.filter)
    }

    fn is_discard(&self) -> bool {
        self.effect
            .downcast_ref::<crate::effects::DiscardEffect>()
            .is_some()
            || self
                .effect
                .downcast_ref::<crate::effects::DiscardHandEffect>()
                .is_some()
    }

    fn discard_details(&self) -> Option<(u32, Option<crate::types::CardType>)> {
        let effect = self
            .effect
            .downcast_ref::<crate::effects::DiscardEffect>()?;
        let crate::effect::Value::Fixed(count) = effect.count else {
            return None;
        };
        let card_type = match &effect.card_filter {
            None => None,
            Some(filter) => {
                let card_type = match filter.card_types.as_slice() {
                    [] => None,
                    [card_type] => Some(*card_type),
                    _ => return None,
                };
                let mut non_type_filter = filter.clone();
                non_type_filter.card_types.clear();
                if non_type_filter != crate::filter::ObjectFilter::default() {
                    return None;
                }
                card_type
            }
        };
        Some((count.max(0) as u32, card_type))
    }

    fn is_exile_from_hand(&self) -> bool {
        self.exile_from_hand_details().is_some()
    }

    fn exile_from_hand_details(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        self.effect.0.exile_from_hand_cost_info()
    }

    fn exile_from_graveyard_details(&self) -> Option<(u32, &[crate::types::CardType])> {
        let exile = self.effect.downcast_ref::<crate::effects::ExileEffect>()?;
        let crate::target::ChooseSpec::Object(filter) = exile.spec.base() else {
            return None;
        };
        if filter.zone != Some(crate::zone::Zone::Graveyard) {
            return None;
        }
        let count = exile.spec.count();
        if count.min == 0 || count.max != Some(count.min) {
            return None;
        }
        Some((count.min as u32, &filter.card_types))
    }

    fn is_remove_counters(&self) -> bool {
        self.effect
            .downcast_ref::<crate::effects::RemoveCountersEffect>()
            .is_some()
            || self
                .effect
                .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
                .is_some()
            || self
                .effect
                .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                .is_some()
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        use crate::costs::CostProcessingMode;
        use crate::effects::{
            DiscardEffect, DiscardHandEffect, ExileEffect, MillEffect, PayEnergyEffect,
            PutCountersEffect, RemoveAnyCountersFromSourceEffect, RemoveCountersEffect,
            ReturnToHandEffect, RevealFromHandEffect, RevealSourceFromHandEffect, SacrificeEffect,
            SacrificeTargetEffect, TapEffect, UntapEffect,
        };
        use crate::target::{ChooseSpec, PlayerFilter};

        if let Some(effect) = self.effect.downcast_ref::<TapEffect>()
            && matches!(effect.target, ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<UntapEffect>()
            && matches!(effect.target, ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if self
            .effect
            .downcast_ref::<crate::effects::LoseLifeEffect>()
            .is_some()
            || self
                .effect
                .downcast_ref::<crate::effects::PayLifeEffect>()
                .is_some()
            || self.effect.downcast_ref::<PayEnergyEffect>().is_some()
            || self.effect.downcast_ref::<MillEffect>().is_some()
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<PutCountersEffect>()
            && matches!(effect.target.base(), ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<RemoveCountersEffect>()
            && matches!(effect.target.base(), ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if self
            .effect
            .downcast_ref::<RemoveAnyCountersFromSourceEffect>()
            .is_some()
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<DiscardHandEffect>()
            && effect.player == PlayerFilter::You
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<RevealFromHandEffect>() {
            return CostProcessingMode::RevealFromHand {
                count: effect.count.clone(),
                card_type: effect.card_type,
                color_filter: effect.color_filter,
            };
        }

        if self
            .effect
            .downcast_ref::<RevealSourceFromHandEffect>()
            .is_some()
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<SacrificeTargetEffect>()
            && matches!(effect.target, ChooseSpec::Source)
        {
            return CostProcessingMode::InlineWithTriggers;
        }

        if let Some(effect) =
            transparent_cost_effect(&self.effect).downcast_ref::<SacrificeEffect>()
            && effect.player == PlayerFilter::You
            && matches!(effect.count, crate::effect::Value::Fixed(1))
        {
            return CostProcessingMode::SacrificeTarget {
                filter: effect.filter.clone(),
            };
        }

        if let Some(effect) = self.effect.downcast_ref::<DiscardEffect>()
            && effect.player == PlayerFilter::You
            && !effect.references_cost_x()
            && !effect.random
            && let crate::effect::Value::Fixed(count) = effect.count
        {
            if effect
                .card_filter
                .as_ref()
                .is_some_and(|filter| filter.source && filter.zone == Some(crate::zone::Zone::Hand))
            {
                return CostProcessingMode::Immediate;
            }
            return CostProcessingMode::DiscardCards {
                count: count.max(0) as u32,
                filter: effect.card_filter.clone().unwrap_or_default(),
            };
        }

        if let Some(effect) = self.effect.downcast_ref::<ExileEffect>() {
            if matches!(effect.spec.base(), ChooseSpec::Source) {
                return CostProcessingMode::Immediate;
            }

            if let ChooseSpec::Object(filter) = effect.spec.base() {
                let count = effect.spec.count();
                if count.min == 0 || count.dynamic_x {
                    return CostProcessingMode::Immediate;
                }

                if count.max == Some(count.min)
                    && let Some(color_filter) = simple_exile_from_hand_filter(filter)
                {
                    return CostProcessingMode::ExileFromHand {
                        count: count.min as u32,
                        color_filter,
                    };
                }

                if count.max == Some(count.min)
                    && let Some(card_type) = simple_exile_from_graveyard_filter(filter)
                {
                    return CostProcessingMode::ExileFromGraveyard {
                        count: count.min as u32,
                        card_type,
                    };
                }

                if let Some(zone) = filter.zone {
                    return CostProcessingMode::ExileObjects {
                        count: count.min as u32,
                        filter: filter.clone(),
                        zone,
                    };
                }
            }
        }

        if let Some(effect) = self.effect.downcast_ref::<ReturnToHandEffect>() {
            return match effect.spec.base() {
                ChooseSpec::Source => CostProcessingMode::Immediate,
                ChooseSpec::Object(filter) => CostProcessingMode::ReturnToHandTarget {
                    filter: filter.clone(),
                },
                _ => CostProcessingMode::Immediate,
            };
        }

        CostProcessingMode::Immediate
    }

    fn effect_ref(&self) -> Option<&crate::effect::Effect> {
        Some(&self.effect)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::costs::{CostContext, CostPayer, CostPaymentResult};

    #[test]
    fn direct_sacrifice_cost_records_actual_original_snapshots_and_keeps_selection_separate() {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        use ironsmith_core::tag::SacrificeCostTag;
        for scenario in 0..4 {
            let player = crate::PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let card = CardBuilder::new(crate::CardId::new(), "Original payment")
                .card_types(vec![crate::CardType::Creature]).power_toughness(PowerToughness::fixed(3, 4)).build();
            let original = game.create_object_from_card(&card, player, crate::Zone::Battlefield);
            let source = game.create_object_from_card(&card, player, crate::Zone::Battlefield);
            let added = game.create_object_from_card(&card, player, crate::Zone::Battlefield);
            let action = match scenario {
                0 => ReplacementAction::Prevent,
                1 => ReplacementAction::ChangeDestination(crate::Zone::Exile),
                2 => ReplacementAction::Instead(vec![Effect::new(crate::effects::SacrificeTargetEffect::new(crate::ChooseSpec::SpecificObject(added)))]),
                _ => ReplacementAction::Additionally(vec![Effect::new(crate::effects::SacrificeTargetEffect::new(crate::ChooseSpec::SpecificObject(added)))]),
            };
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, player,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(crate::target::ObjectFilter::specific(original), Some(crate::Zone::Battlefield), Some(crate::Zone::Graveyard)), action));
            let chosen_tag = SacrificeCostTag::Selected(7).key();
            let actual_tag = SacrificeCostTag::OriginalResult(7).key();
            let cost = crate::costs::Cost::try_effect(Effect::sacrifice(crate::target::ObjectFilter::specific(original), 1).tag(chosen_tag.clone())).unwrap();
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let mut context = CostContext::new(source, player, &mut dm).with_pre_chosen_cards(vec![original]);
            // UNRUN integration assertion: retain the actual packet and the
            // original sacrifice binding through the same payment owner.
            let receipt = cost.pay_with_outputs(&mut game, &mut context).unwrap();
            assert!(matches!(receipt.result, CostPaymentResult::Paid));
            let outputs = receipt.outputs.expect("effect-backed payment retains its actual packet");
            let retained = outputs.outcome.instruction_result().execution_facts.iter().find_map(|fact| match fact {
                crate::effect::ExecutionFact::OriginalSacrificeObjects(memory) => Some(memory),
                _ => None,
            }).expect("the returned packet keeps the original sacrifice fact");
            assert_eq!(retained.len(), usize::from(scenario == 1 || scenario == 3));
            assert!(retained.iter().all(|object| object.to_snapshot(&game).object_id == original));
            assert_eq!(game.turn_store.turn_history.event_kind_count(crate::events::EventKind::Sacrifice),
                match scenario { 0 => 0, 3 => 2, _ => 1 });
            let actual = context.tagged_objects.get(&actual_tag).expect("completed original receipt, including zero");
            assert_eq!(actual.len(), usize::from(scenario == 1 || scenario == 3));
            if let Some(snapshot) = actual.first() {
                assert_eq!(snapshot.object_id, original); assert_eq!(snapshot.zone, crate::Zone::Battlefield); assert_eq!(snapshot.power, Some(3));
            }
            assert!(context.completed_sacrifice.is_some());
            // An unrelated result cannot supply the original selected cost.
            assert!(actual.iter().all(|snapshot| snapshot.object_id != added));
        }
    }
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::{MoveToZoneEffect, RemoveCountersEffect, SacrificeEffect};
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::PlayerFilter;
    use crate::types::CardType;
    use crate::{card::CardBuilder, game_state::GameState, zone::Zone};

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn discard_cost_excludes_entry_reserved_cards() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let definition = CardBuilder::new(CardId::from_raw(991), "Reserved card")
            .card_types(vec![CardType::Creature])
            .build();
        let card = game.create_object_from_card(&definition, alice, Zone::Hand);
        let cost = crate::costs::Cost::effect(crate::effects::DiscardEffect::you(1));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(card, alice, &mut dm);
        assert!(cost.can_pay(&game, &ctx).is_ok());
        ctx.replacement.entry_reserved_objects.insert(card);
        assert!(cost.can_pay(&game, &ctx).is_err());
        assert!(cost.pay(&mut game, &mut ctx).is_err());
        assert_eq!(game.player(alice).unwrap().hand, vec![card]);
    }

    #[test]
    fn cause_filtered_sacrifice_protection_blocks_opponent_effects_and_requested_costs() {
        use crate::costs::PaymentReason;
        use crate::effects::EffectExecutor;
        use crate::events::cause::{CauseFilter, CauseType, CauseTypeFilter, ControllerFilter};
        for opponent_requested in [false, true] {
            for reason in [
                None,
                Some(PaymentReason::Effect),
                Some(PaymentReason::CastSpell),
            ] {
                let mut game = create_test_game();
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let protection_card = CardBuilder::new(CardId::new(), "Protection Source")
                    .card_types(vec![CardType::Enchantment])
                    .build();
                let protection =
                    game.create_object_from_card(&protection_card, alice, Zone::Battlefield);
                let restriction = crate::effect::Restriction::BeSacrificedByCause {
                    filter: crate::target::ObjectFilter::permanent().you_control(),
                    cause: CauseFilter {
                        cause_type: Some(CauseTypeFilter::OneOf(vec![
                            CauseType::Effect,
                            CauseType::Cost,
                        ])),
                        source_filter: None,
                        controller_filter: Some(ControllerFilter::Opponent),
                    },
                };
                game.object_mut(protection).unwrap().abilities =
                    std::sync::Arc::new(vec![crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::restriction(
                            restriction,
                            "Opponent sacrifice protection".into(),
                        ),
                    )]);
                let source_card = CardBuilder::new(CardId::new(), "Request Source")
                    .card_types(vec![CardType::Sorcery])
                    .build();
                let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
                let victim = CardBuilder::new(CardId::new(), "Victim")
                    .card_types(vec![CardType::Creature])
                    .build();
                let victim = game.create_object_from_card(&victim, alice, Zone::Battlefield);
                game.update_cant_effects();
                assert!(
                    game.can_be_sacrificed(victim),
                    "must not impose an unconditional prohibition"
                );
                let mut ctx = ExecutionContext::new_default(
                    source,
                    if opponent_requested { bob } else { alice },
                );
                let allowed = !opponent_requested || reason == Some(PaymentReason::CastSpell);
                if let Some(reason) = reason {
                    let cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice(
                        crate::target::ObjectFilter::creature(),
                    ));
                    let result = crate::special_actions::pay_total_cost_with_choice_in_context(
                        &mut game, alice, source, &cost, reason, &mut ctx,
                    );
                    assert_eq!(
                        result.is_ok(),
                        allowed,
                        "payment {reason:?}, opponent={opponent_requested}: {result:?}"
                    );
                } else {
                    SacrificeEffect::player(
                        crate::target::ObjectFilter::creature(),
                        1,
                        PlayerFilter::Specific(alice),
                    )
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                }
                assert_eq!(
                    game.battlefield.contains(&victim),
                    !allowed,
                    "{reason:?}, opponent={opponent_requested}"
                );
            }
        }
    }

    #[test]
    fn sacrifice_payment_event_preserves_requesting_effect_controller() {
        use crate::costs::PaymentReason;
        for reason in [PaymentReason::Effect, PaymentReason::CastSpell] {
            for candidates in [1, 2] {
                let mut game = create_test_game();
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let source_card = CardBuilder::new(CardId::new(), "Requesting Spell")
                    .card_types(vec![CardType::Sorcery])
                    .build();
                let source = game.create_object_from_card(&source_card, bob, Zone::Stack);
                let victim = CardBuilder::new(CardId::new(), "Sacrifice Candidate")
                    .card_types(vec![CardType::Creature])
                    .build();
                for _ in 0..candidates {
                    game.create_object_from_card(&victim, alice, Zone::Battlefield);
                }
                game.take_pending_trigger_events();
                let cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice(
                    crate::target::ObjectFilter::creature(),
                ));
                let mut ctx = ExecutionContext::new_default(source, bob);
                crate::special_actions::pay_total_cost_with_choice_in_context(
                    &mut game, alice, source, &cost, reason, &mut ctx,
                )
                .expect("sacrifice payment should succeed");
                assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
                let events = game.take_pending_trigger_events();
                let event = events
                    .iter()
                    .find_map(|e| e.downcast::<crate::events::ZoneChangeEvent>())
                    .unwrap();
                assert_eq!(
                    event.cause.cause_type,
                    crate::events::cause::CauseType::Cost
                );
                assert_eq!(event.cause.source, Some(source));
                assert_eq!(
                    event.cause.source_controller,
                    Some(if reason == PaymentReason::Effect {
                        bob
                    } else {
                        alice
                    }),
                    "{reason:?}, {candidates}"
                );
            }
        }
    }

    #[test]
    fn remove_counters_cost_sets_x_from_marker_removal_events() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(1), "Battery")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(source) {
            obj.counters.insert(CounterType::Charge, 3);
        }

        let cost = CostEffect::new(RemoveCountersEffect::new(
            CounterType::Charge,
            2,
            crate::target::ChooseSpec::Source,
        ));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost
            .pay(&mut game, &mut ctx)
            .expect("cost should be payable");

        assert_eq!(result, CostPaymentResult::Paid);
        assert_eq!(ctx.x_value, Some(2));
        assert_eq!(game.counter_count(source, CounterType::Charge), 1);
    }

    #[test]
    fn remove_all_counters_cost_sets_x_to_zero_when_none_are_removed() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(11), "Empty Battery")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);

        let cost = CostEffect::new(crate::effects::RemoveAnyCountersFromSourceEffect::all(
            Some(CounterType::Charge),
        ));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost
            .pay(&mut game, &mut ctx)
            .expect("remove-all counters cost should be payable with zero counters");

        assert_eq!(result, CostPaymentResult::Paid);
        assert_eq!(ctx.x_value, Some(0));
    }

    #[test]
    fn tagged_sacrifice_cost_can_validate_with_cost_context_tags() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(1), "Bone Splinters")
            .card_types(vec![CardType::Sorcery])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Stack);

        let creature_card = CardBuilder::new(CardId::from_raw(2), "Skarrgan Firebird")
            .card_types(vec![CardType::Creature])
            .build();
        let creature = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
        let snapshot = ObjectSnapshot::from_object(game.object(creature).unwrap(), &game);

        let filter = crate::target::ObjectFilter::default().match_tagged(
            "sacrificed_0",
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
        let cost = CostEffect::new(SacrificeEffect::new(filter, 1, PlayerFilter::You));

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);
        ctx.tagged_objects
            .insert(TagKey::from("sacrificed_0"), vec![snapshot]);

        let result = cost
            .pay(&mut game, &mut ctx)
            .expect("tagged sacrifice cost should be payable");

        assert_eq!(result, CostPaymentResult::Paid);
        assert!(!game.battlefield.contains(&creature));
        assert!(
            game.player(alice)
                .unwrap()
                .graveyard
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|obj| obj.name == "Skarrgan Firebird")
        );
    }

    #[test]
    fn result_tagged_sacrifice_cost_keeps_its_interactive_payment_mode() {
        let filter = crate::target::ObjectFilter::creature().you_control();
        let tagged =
            crate::effect::Effect::new(SacrificeEffect::new(filter.clone(), 1, PlayerFilter::You))
                .tag("sacrifice_cost_0");
        let cost = CostEffect::from_validated_effect(tagged)
            .expect("a transparent result tag should preserve cost executability");

        let crate::costs::CostProcessingMode::SacrificeTarget {
            filter: actual_filter,
        } = cost.processing_mode()
        else {
            panic!("a result tag must preserve the sacrifice choice mode");
        };
        assert_eq!(actual_filter, filter);
    }

    #[test]
    fn counted_exile_object_cost_honors_preselected_cost_choices() {
        use crate::color::{Color, ColorSet};
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::target::ChooseSpec;

        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(10), "Craft Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let mut add_material = |name: &str, mana_cost: ManaCost, card_type: CardType| {
            let card = CardBuilder::new(CardId::new(), name)
                .mana_cost(mana_cost)
                .card_types(vec![card_type])
                .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard)
        };

        let first_red = add_material(
            "Arc Lightning",
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Red]]),
            CardType::Sorcery,
        );
        let chosen_red_one = add_material(
            "Lightning Helix",
            ManaCost::from_pips(vec![vec![ManaSymbol::Red], vec![ManaSymbol::White]]),
            CardType::Instant,
        );
        let chosen_red_two = add_material(
            "Lightning Bolt",
            ManaCost::from_pips(vec![vec![ManaSymbol::Red]]),
            CardType::Instant,
        );
        let blue_instant = add_material(
            "Opt",
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
            CardType::Instant,
        );

        let material_filter = crate::filter::ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .with_colors(ColorSet::from_color(Color::Red))
            .with_type(CardType::Instant)
            .with_type(CardType::Sorcery);
        let cost = crate::costs::Cost::validated_effect(crate::effect::Effect::exile(
            ChooseSpec::Object(material_filter).with_count(crate::effect::ChoiceCount::at_least(2)),
        ));
        assert!(matches!(
            cost.processing_mode(),
            crate::costs::CostProcessingMode::ExileObjects { count: 2, .. }
        ));

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm)
            .with_pre_chosen_cards(vec![chosen_red_one, chosen_red_two]);

        cost.pay(&mut game, &mut ctx)
            .expect("preselected red material cost should be payable");

        let exiled_names = game
            .exile
            .iter()
            .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
            .collect::<Vec<_>>();
        assert!(exiled_names.contains(&"Lightning Helix"));
        assert!(exiled_names.contains(&"Lightning Bolt"));

        assert_eq!(game.object(first_red).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(blue_instant).unwrap().zone, Zone::Graveyard);
    }

    #[test]
    fn tagged_move_to_zone_cost_can_validate_with_cost_context_tags() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(1), "Oracle of Dust")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let creature_card = CardBuilder::new(CardId::from_raw(2), "Silvercoat Lion")
            .card_types(vec![CardType::Creature])
            .build();
        let exiled = game.create_object_from_card(&creature_card, bob, Zone::Exile);
        let snapshot = ObjectSnapshot::from_object(game.object(exiled).unwrap(), &game);

        let tag = TagKey::from("graveyard_cost_0");
        let cost = CostEffect::new(MoveToZoneEffect::to_graveyard(
            crate::target::ChooseSpec::Tagged(tag.clone()),
        ));

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);
        ctx.tagged_objects.insert(tag, vec![snapshot]);

        let result = cost
            .pay(&mut game, &mut ctx)
            .expect("tagged move-to-zone cost should be payable");

        assert_eq!(result, CostPaymentResult::Paid);
        assert!(!game.exile.contains(&exiled));
        assert!(
            game.player(bob)
                .unwrap()
                .graveyard
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|obj| obj.name == "Silvercoat Lion")
        );
    }
}

#[cfg(test)]
mod energy_cost_error_contract_tests {
    use super::*;
    use crate::costs::{CostContext, CostPayer};
    fn setup() -> (GameState, crate::ids::ObjectId, crate::ids::PlayerId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Energy payment source")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let source = game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield);
        game.player_mut(alice).unwrap().energy_counters = 2;
        (game, source, alice)
    }
    fn cost(amount: crate::effect::Value, player: crate::target::PlayerFilter) -> CostEffect {
        CostEffect::new(crate::effects::PayEnergyEffect::new(
            amount,
            crate::target::ChooseSpec::Player(player),
        ))
    }
    #[test]
    fn ordinary_energy_inability_is_distinct_from_execution_failure() {
        let (game, source, alice) = setup();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);
        assert!(
            cost(
                crate::effect::Value::Fixed(1),
                crate::target::PlayerFilter::You
            )
            .can_pay(&game, &ctx)
            .is_ok()
        );
        assert_eq!(
            cost(
                crate::effect::Value::Fixed(3),
                crate::target::PlayerFilter::You
            )
            .can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientEnergy)
        );
        assert_eq!(game.player(alice).unwrap().energy_counters, 2);
    }
    #[test]
    fn energy_precheck_inherits_the_chosen_x_value() {
        let (game, source, alice) = setup();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);
        ctx.x_value = Some(2);
        assert!(
            cost(crate::effect::Value::X, crate::target::PlayerFilter::You)
                .can_pay(&game, &ctx)
                .is_ok()
        );
        assert_eq!(game.player(alice).unwrap().energy_counters, 2);
    }
    #[test]
    fn unresolvable_energy_value_retains_its_typed_execution_error() {
        let (game, source, alice) = setup();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);
        let invalid =
            crate::effect::Value::DividedRoundedDown(Box::new(crate::effect::Value::Fixed(1)), 0);
        assert!(
            matches!(cost(invalid, crate::target::PlayerFilter::You).can_pay(&game, &ctx),
            Err(CostPaymentError::ExecutionFailed(crate::effects::ExecutionError::UnresolvableValue(ref message)))
                if message == "division by zero in dynamic value")
        );
        assert_eq!(game.player(alice).unwrap().energy_counters, 2);
    }
    #[test]
    fn missing_energy_payer_is_a_structural_error() {
        let (game, source, alice) = setup();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);
        assert_eq!(
            cost(
                crate::effect::Value::Fixed(1),
                crate::target::PlayerFilter::Specific(crate::ids::PlayerId::from_index(9))
            )
            .can_pay(&game, &ctx),
            Err(CostPaymentError::PlayerNotFound)
        );
    }
}

#[cfg(test)]
mod unsigned_counter_cost_contract_tests {
    use super::*;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::{RemoveAnyCountersFromSourceEffect, RemoveCountersEffect};
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::types::CardType;
    use crate::zone::Zone;
    fn fixture(amount: u32) -> (GameState, crate::ids::ObjectId, PlayerId) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(CardId::new(), "Counter cost recipient")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .counters
            .insert(CounterType::Charge, amount);
        game.object_mut(source)
            .unwrap()
            .counters
            .insert(CounterType::PlusOnePlusOne, 7);
        (game, source, alice)
    }
    #[test]
    fn fixed_unsigned_counter_cost_pays_exact_requested_amount() {
        for amount in [i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
            let (mut game, source, alice) = fixture(amount);
            let cost = CostEffect::new(RemoveCountersEffect::new(
                CounterType::Charge,
                amount,
                crate::target::ChooseSpec::Source,
            ));
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = CostContext::new(source, alice, &mut dm);
            let result=cost.pay(&mut game,&mut ctx).expect("exact unsigned literal is a fixed payable cost when every required counter is available");
            assert_eq!(result, CostPaymentResult::Paid);
            assert_eq!(ctx.x_value, Some(amount));
            assert_eq!(game.counter_count(source, CounterType::Charge), 0);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 7);
        }
    }
    #[test]
    fn fixed_unsigned_counter_cost_cannot_use_remove_all_semantics() {
        let (mut game, source, alice) = fixture(3);
        let cost = CostEffect::new(RemoveCountersEffect::new(
            CounterType::Charge,
            u32::MAX,
            crate::target::ChooseSpec::Source,
        ));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);
        assert!(cost.pay(&mut game, &mut ctx).is_err());
        assert_eq!(ctx.x_value, None);
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
    }
    #[test]
    fn explicit_all_counter_cost_preserves_zero_and_full_unsigned_totals() {
        for amount in [0, 3, i32::MAX as u32 + 1, u32::MAX] {
            let (mut game, source, alice) = fixture(amount);
            let cost = CostEffect::new(RemoveAnyCountersFromSourceEffect::all(Some(
                CounterType::Charge,
            )));
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = CostContext::new(source, alice, &mut dm);
            assert_eq!(
                cost.pay(&mut game, &mut ctx).unwrap(),
                CostPaymentResult::Paid
            );
            assert_eq!(ctx.x_value, Some(amount));
            assert_eq!(game.counter_count(source, CounterType::Charge), 0);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 7);
        }
    }
    #[test]
    fn cost_builder_removes_exact_unsigned_literal_instead_of_zero() {
        for amount in [i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
            let (mut game, source, alice) = fixture(amount);
            let cost = crate::costs::Cost::remove_counters(CounterType::Charge, amount);
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = CostContext::new(source, alice, &mut dm);
            assert_eq!(
                cost.pay(&mut game, &mut ctx).unwrap(),
                CostPaymentResult::Paid
            );
            assert_eq!(ctx.x_value, Some(amount));
            assert_eq!(game.counter_count(source, CounterType::Charge), 0);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 7);
        }
    }
    #[test]
    fn cost_builder_adds_full_unsigned_literal_instead_of_zero() {
        for amount in [i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
            let (mut game, source, alice) = fixture(0);
            let cost = crate::costs::Cost::add_counters(CounterType::Charge, amount);
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = CostContext::new(source, alice, &mut dm);
            assert_eq!(
                cost.pay(&mut game, &mut ctx).unwrap(),
                CostPaymentResult::Paid
            );
            assert_eq!(game.counter_count(source, CounterType::Charge), amount);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 7);
        }
    }
}

#[cfg(test)]
mod retained_counter_x_independence {
    use super::*;
    #[test]
    fn an_unannounced_counter_quantity_does_not_borrow_a_larger_mana_x() {
        struct Two;
        impl crate::decision::DecisionMaker for Two {
            fn decide_number(&mut self, _: &GameState, context: &crate::decisions::context::NumberContext) -> u32 {
                assert!(!context.is_x_value); assert_eq!((context.min,context.max),(1,2)); 2
            }
        }
        let payer=crate::PlayerId::from_index(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let card=crate::card::CardBuilder::new(crate::CardId::new(),"Independent cost X").card_types(vec![crate::CardType::Artifact]).build();
        let source=game.create_object_from_card(&card,payer,crate::Zone::Battlefield);game.add_counters(source,crate::CounterType::Charge,2);
        let removal=crate::effects::RemoveAnyCountersAmongEffect::dynamic(1,u32::MAX,crate::ObjectFilter::source().in_zone(crate::Zone::Battlefield),false)
            .with_counter_type(Some(crate::CounterType::Charge)).from_single_object();
        let cost=crate::costs::Cost::effect(crate::effects::WithIdEffect::new(crate::effect::EffectId::ACTIVATION_COUNTER_COST,Effect::new(removal)));
        let mut dm=Two;let mut context=CostContext::new(source,payer,&mut dm).with_x(17);
        cost.pay(&mut game,&mut context).unwrap();assert_eq!(context.x_value,Some(17));assert_eq!(game.counter_count(source,crate::CounterType::Charge),0);
        assert_eq!(context.effect_outcomes[&crate::effect::EffectId::ACTIVATION_COUNTER_COST].instruction_result().count_or_zero(),2);
    }
}
