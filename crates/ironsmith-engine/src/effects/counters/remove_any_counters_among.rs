//! Effect for removing counters from among matching permanents.

use super::remove_counters::SelectedCounterRemovalPlan;
use crate::decision::FallbackStrategy;
use crate::decisions::DecisionSpec as _;
use crate::decisions::{
    ChooseObjectsSpec, CounterRemovalSpec, DistributeSpec, NumberSpec, make_decision_with_fallback,
};
use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, CostExecutableEffect, CostValidationError, EffectExecutor,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::filter::{
    AlternativeCastKind, CounterConstraint, FilterContext, ObjectFilter, PlayerFilter,
};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;
pub use ironsmith_core::RemoveAnyCountersAmongEffect;
use std::collections::HashMap;

/// Remove a total number of counters from among permanents matching a filter.
///
/// When used as a cost, this is wrapped by `CostEffect` via `Cost::effect(...)`.
pub(crate) fn valid_targets_with_tags(
    effect: &RemoveAnyCountersAmongEffect,
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
    tagged_objects: &HashMap<TagKey, Vec<ObjectSnapshot>>,
) -> Vec<ObjectId> {
    let filter_ctx = FilterContext::new(payer)
        .with_source(source)
        .with_tagged_objects(tagged_objects);

    counter_removal_candidate_ids(&effect.filter, game)
        .into_iter()
        .filter(|id| {
            let Some(obj) = game.object(*id) else {
                return false;
            };
            let available = available_counter_count(effect, obj);
            effect.filter.matches(obj, &filter_ctx, game) && available > 0
        })
        .collect()
}

fn available_counter_count(
    effect: &RemoveAnyCountersAmongEffect,
    object: &crate::object::Object,
) -> u64 {
    if let Some(counter_type) = effect.counter_type {
        u64::from(object.counters.get(&counter_type).copied().unwrap_or(0))
    } else {
        object
            .counters
            .values()
            .fold(0u64, |sum, count| sum.saturating_add(u64::from(*count)))
    }
}

#[allow(dead_code)]
pub fn valid_targets(
    effect: &RemoveAnyCountersAmongEffect,
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
) -> Vec<ObjectId> {
    valid_targets_with_tags(effect, game, source, payer, &HashMap::new())
}

pub(crate) fn total_available_with_tags(
    effect: &RemoveAnyCountersAmongEffect,
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
    tagged_objects: &HashMap<TagKey, Vec<ObjectSnapshot>>,
) -> u64 {
    let available = valid_targets_with_tags(effect, game, source, payer, tagged_objects)
        .into_iter()
        .filter_map(|id| game.object(id))
        .map(|object| available_counter_count(effect, object));
    if effect.single_object {
        available.max().unwrap_or(0)
    } else {
        available.fold(0u64, |sum, count| sum.saturating_add(count))
    }
}

pub(crate) fn total_available(
    effect: &RemoveAnyCountersAmongEffect,
    game: &GameState,
    source: ObjectId,
    payer: PlayerId,
) -> u64 {
    total_available_with_tags(effect, game, source, payer, &HashMap::new())
}

pub fn cost_display(effect: &RemoveAnyCountersAmongEffect) -> String {
    let target_phrase_single = remove_counters_target_phrase(&effect.filter, false);
    let target_phrase_plural = remove_counters_target_phrase(&effect.filter, true);
    if effect.dynamic_count {
        let amount_text = if effect.display_x {
            "X".to_string()
        } else if effect.min_count > 0 {
            "one or more".to_string()
        } else if effect.count != u32::MAX {
            // "remove up to three stun counters from among ..."
            let bound = ironsmith_core::cardinal_word(effect.count)
                .unwrap_or_else(|| effect.count.to_string());
            format!("up to {bound}")
        } else {
            "any number of".to_string()
        };
        let from = if effect.filter.source || effect.single_object {
            "from"
        } else {
            "from among"
        };
        let target_phrase = if effect.single_object {
            target_phrase_single
        } else {
            target_phrase_plural
        };
        return match effect.counter_type {
            Some(counter_type) => format!(
                "Remove {amount_text} {} counters {from} {}",
                counter_type.description(),
                target_phrase
            ),
            None => format!("Remove {amount_text} counters {from} {target_phrase}"),
        };
    }
    match (effect.count, effect.counter_type) {
        (1, Some(counter_type)) => {
            let counter_name = counter_type.description();
            format!(
                "Remove {} {} counter from {}",
                counter_article(&counter_name),
                counter_name,
                target_phrase_single
            )
        }
        (count, Some(counter_type)) if effect.single_object => {
            let counter_name = counter_type.description();
            format!(
                "Remove {} {} counters from {}",
                count, counter_name, target_phrase_single
            )
        }
        (count, Some(counter_type)) => {
            let counter_name = counter_type.description();
            format!(
                "Remove {} {} counters from among {}",
                count, counter_name, target_phrase_plural
            )
        }
        (1, None) => format!("Remove a counter from {}", target_phrase_single),
        (count, None) if effect.single_object => {
            format!("Remove {} counters from {}", count, target_phrase_single)
        }
        (count, None) => {
            format!(
                "Remove {} counters from among {}",
                count, target_phrase_plural
            )
        }
    }
}

fn counter_removal_candidate_ids(filter: &ObjectFilter, game: &GameState) -> Vec<ObjectId> {
    let mut zones = Vec::new();
    collect_counter_removal_candidate_zones(filter, &mut zones);
    if zones.is_empty() {
        zones.push(Zone::Battlefield);
    }

    let mut ids = Vec::new();
    for zone in zones {
        for id in game.zone_ids(zone) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    ids
}

fn collect_counter_removal_candidate_zones(filter: &ObjectFilter, zones: &mut Vec<Zone>) {
    if let Some(zone) = filter.zone
        && !zones.contains(&zone)
    {
        zones.push(zone);
    }
    for arm in &filter.any_of {
        collect_counter_removal_candidate_zones(arm, zones);
    }
}

impl EffectExecutor for RemoveAnyCountersAmongEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(super::remove_counters::selected_counter_removal_proposal(
            super::remove_counters::CounterRemovalSelector::Among(self.clone()),
        ))
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        vec![ChooseSpec::All(self.filter.clone())]
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
        game.clear_pending_decision_controllers();
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| execute_distributed_counter_removal(self, game, ctx),
        );
        // The shared transaction restores the action; keep this adapter's
        // existing neutral suspension policy even if the child failed.
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }

    fn cost_description(&self) -> Option<String> {
        Some(cost_display(self))
    }
}

impl CostExecutableEffect for RemoveAnyCountersAmongEffect {
    fn supports_prepared_payment(&self) -> bool {
        true
    }

    fn prepare_simultaneous_payment(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let plan = select_distributed_counter_removal(self, game, ctx)?;
        let (events, requested) = match plan {
            SelectedCounterRemovalPlan::Groups { events, requested } => (events, requested),
            SelectedCounterRemovalPlan::Finished(outputs) => {
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(Box::new(super::placement::PreparedCounterCost::Finished(
                        outputs.outcome,
                    )));
                }
                if outputs.outcome.requested_amount() != Some(0)
                    || outputs.outcome.status == crate::effect::OutcomeStatus::Impossible
                {
                    return Err(ExecutionError::Impossible(
                        "distributed counter payment did not fulfill its selection".into(),
                    ));
                }
                (Vec::new(), 0)
            }
            SelectedCounterRemovalPlan::Single(_) | SelectedCounterRemovalPlan::Recorded(_) => {
                return Err(ExecutionError::InternalError(
                    "distributed counter payment requires a grouped quantity plan".into(),
                ));
            }
        };
        let proposal = super::capture_counter_payment_with_quantity(game, ctx, events)?;
        if proposal.nominal_payment_quantity() != Some(requested) {
            return Err(ExecutionError::Impossible(
                "distributed counter payment does not match the requested quantity".into(),
            ));
        }
        Ok(proposal)
    }

    fn accepts_prepared_payment(
        &self,
        proposal: &dyn crate::effects::SimultaneousEffectProposal,
    ) -> bool {
        super::prepared_payment::accepts_counter_quantity_payment(proposal, self.counter_type)
    }

    fn payment_x_from_prepared_payment(
        &self,
        proposal: &dyn crate::effects::SimultaneousEffectProposal,
        _execution: &ExecutionContext,
    ) -> Result<Option<u32>, CostValidationError> {
        super::prepared_payment::counter_quantity_payment_x(proposal, self.counter_type)
    }

    fn validate_payment_outcome(&self, outcome: &EffectOutcome) -> Result<(), CostValidationError> {
        super::prepared_payment::validate_counter_quantity_payment(outcome)
    }

    fn payment_x_from_outcome(
        &self,
        outcome: &EffectOutcome,
        execution: &ExecutionContext,
    ) -> Result<Option<u32>, CostValidationError> {
        super::counter_cost_x_from_outcome(outcome, execution)
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if total_available(self, game, source, controller) < u64::from(self.min_count) {
            return Err(CostValidationError::Other(
                "not enough counters".to_string(),
            ));
        }
        Ok(())
    }
}

fn counter_article(counter_name: &str) -> &'static str {
    let starts_with_vowel = counter_name
        .chars()
        .next()
        .map(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
        .unwrap_or(false);
    if starts_with_vowel { "an" } else { "a" }
}

fn remove_counters_target_phrase(filter: &ObjectFilter, plural: bool) -> String {
    fn join_type_names(
        names: &[String],
        connective: ironsmith_core::ObjectFilterUnionConnective,
        plural: bool,
    ) -> String {
        let conjunction = match connective {
            // Oracle uses "artifact or creature" for a singular choice, but
            // list-style "artifacts, creatures, and planeswalkers" for an
            // aggregate "from among" cost. Preserve both established surfaces.
            ironsmith_core::ObjectFilterUnionConnective::Or if plural => "and",
            ironsmith_core::ObjectFilterUnionConnective::Or => "or",
            ironsmith_core::ObjectFilterUnionConnective::AndOr => "and/or",
        };
        match names.len() {
            0 => String::new(),
            1 => names[0].clone(),
            2 => format!("{} {conjunction} {}", names[0], names[1]),
            _ => {
                let mut out = names[..names.len() - 1].join(", ");
                out.push_str(", ");
                out.push_str(conjunction);
                out.push(' ');
                out.push_str(&names[names.len() - 1]);
                out
            }
        }
    }

    if filter.source {
        if let Some(surface) = &filter.source_surface {
            return surface.display_text();
        }
        if plural {
            return "this source".to_string();
        }
        if filter.card_types.len() == 1 {
            return format!("this {}", filter.card_types[0].name().to_ascii_lowercase());
        }
        return "this source".to_string();
    }

    if is_simple_permanent_you_control_filter(filter) {
        return if plural {
            "permanents you control".to_string()
        } else {
            "a permanent you control".to_string()
        };
    }

    if is_simple_nonland_permanent_you_control_filter(filter) {
        return if plural {
            "nonland permanents you control".to_string()
        } else {
            "a nonland permanent you control".to_string()
        };
    }

    if is_permanent_you_control_or_suspended_card_you_own_filter(filter) {
        return if plural {
            "permanents you control or suspended cards you own".to_string()
        } else {
            "a permanent you control or suspended card you own".to_string()
        };
    }

    if let Some(card_type) = simple_you_controlled_battlefield_card_type(filter) {
        let noun = if plural {
            card_type.plural_name()
        } else {
            card_type.name()
        };
        let other_prefix = if filter.other && plural { "other " } else { "" };
        return if plural {
            format!("{other_prefix}{noun} you control")
        } else {
            format!("a {noun} you control")
        };
    }

    // "from among all permanents": the permanent card types spelled out as
    // a union are every permanent.
    {
        use crate::types::CardType as T;
        let every_permanent_type = [
            T::Artifact,
            T::Creature,
            T::Enchantment,
            T::Land,
            T::Planeswalker,
            T::Battle,
        ];
        if plural
            && filter.card_types.len() == every_permanent_type.len()
            && every_permanent_type
                .iter()
                .all(|card_type| filter.card_types.contains(card_type))
        {
            let mut rest = filter.clone();
            rest.card_types.clear();
            rest.union_surface = Default::default();
            rest.zone = None;
            if rest == ObjectFilter::default() {
                return "all permanents".to_string();
            }
        }
    }
    let mut noun = if filter.card_types.is_empty() {
        if plural {
            "permanents".to_string()
        } else {
            "a permanent".to_string()
        }
    } else {
        let type_names = filter
            .card_types
            .iter()
            .map(|card_type| {
                if plural {
                    card_type.plural_name().to_ascii_lowercase()
                } else {
                    card_type.name().to_ascii_lowercase()
                }
            })
            .collect::<Vec<_>>();
        let joined = join_type_names(&type_names, filter.union_connective(), plural);
        if plural {
            if filter.other {
                format!("other {joined}")
            } else {
                joined
            }
        } else {
            let article = if joined.chars().next().is_some_and(|letter| {
                matches!(letter.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u')
            }) {
                "an"
            } else {
                "a"
            };
            format!("{article} {joined}")
        }
    };

    if filter.controller == Some(PlayerFilter::You) {
        noun.push_str(" you control");
    }

    noun
}

fn is_permanent_you_control_or_suspended_card_you_own_filter(filter: &ObjectFilter) -> bool {
    if filter.any_of.len() != 2 || filter.zone.is_some() {
        return false;
    }

    let permanent = ObjectFilter::permanent().you_control();
    let permanent_with_time = ObjectFilter::permanent()
        .you_control()
        .with_counter_type(CounterType::Time);
    let suspended = ObjectFilter::default()
        .in_zone(Zone::Exile)
        .owned_by(PlayerFilter::You)
        .with_alternative_cast(AlternativeCastKind::Suspend);
    let suspended_with_time = ObjectFilter::default()
        .in_zone(Zone::Exile)
        .owned_by(PlayerFilter::You)
        .with_alternative_cast(AlternativeCastKind::Suspend)
        .with_counter_type(CounterType::Time);

    filter.any_of.iter().any(|arm| {
        arm == &permanent
            || arm == &permanent_with_time
            || same_filter_except_time_counter(arm, &permanent)
    }) && filter.any_of.iter().any(|arm| {
        arm == &suspended
            || arm == &suspended_with_time
            || same_filter_except_time_counter(arm, &suspended)
    })
}

fn same_filter_except_time_counter(left: &ObjectFilter, right: &ObjectFilter) -> bool {
    let mut normalized = left.clone();
    if normalized.with_counter == Some(CounterConstraint::Typed(CounterType::Time)) {
        normalized.with_counter = None;
    }
    &normalized == right
}

fn is_simple_permanent_you_control_filter(filter: &ObjectFilter) -> bool {
    let base = ObjectFilter::permanent().you_control();
    if *filter == base {
        return true;
    }

    let mut expanded = base;
    expanded.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    *filter == expanded
}

fn simple_you_controlled_battlefield_card_type(filter: &ObjectFilter) -> Option<CardType> {
    if filter.card_types.len() != 1 {
        return None;
    }

    let mut expected = ObjectFilter::default();
    expected.zone = Some(Zone::Battlefield);
    expected.controller = Some(PlayerFilter::You);
    expected.card_types = vec![filter.card_types[0]];
    if *filter == expected {
        Some(filter.card_types[0])
    } else {
        None
    }
}

fn is_simple_nonland_permanent_you_control_filter(filter: &ObjectFilter) -> bool {
    let mut expected = ObjectFilter::default();
    expected.zone = Some(Zone::Battlefield);
    expected.controller = Some(PlayerFilter::You);
    expected.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    expected.excluded_card_types = vec![CardType::Land];
    *filter == expected
}

fn execute_distributed_counter_removal(
    effect: &RemoveAnyCountersAmongEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let plan = select_distributed_counter_removal(effect, game, ctx)?;
    super::remove_counters::complete_selected_counter_removal_plan(game, ctx, plan)
}

pub(super) fn select_distributed_counter_removal(
    effect: &RemoveAnyCountersAmongEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Result<SelectedCounterRemovalPlan, ExecutionError> {
    let total_available = total_available_with_tags(
        effect,
        game,
        ctx.source,
        ctx.controller,
        &ctx.tagged_objects,
    );
    if total_available < u64::from(effect.min_count) {
        return Ok(SelectedCounterRemovalPlan::Finished(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::impossible()),
        ));
    }
    let requested_count = if effect.dynamic_count {
        let max_count = effect
            .count
            .min(u32::try_from(total_available).unwrap_or(u32::MAX));
        if max_count < effect.min_count {
            return Ok(SelectedCounterRemovalPlan::Finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::impossible()),
            ));
        }
        let chosen = if effect.display_x
            && let Some(x) = ctx.x_value
        {
            if x < effect.min_count || x > max_count {
                return Ok(SelectedCounterRemovalPlan::Finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::impossible()),
                ));
            }
            x
        } else {
            make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                NumberSpec::range(
                    ctx.source,
                    effect.min_count,
                    max_count,
                    "counters to remove",
                ),
                FallbackStrategy::Maximum,
            )
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SelectedCounterRemovalPlan::Finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        chosen.clamp(effect.min_count, max_count)
    } else {
        effect.count
    };

    if requested_count == 0 {
        return Ok(SelectedCounterRemovalPlan::Finished(
            CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0).with_requested_amount(0u64),
            ),
        ));
    }

    let mut valid_targets = valid_targets_with_tags(
        effect,
        game,
        ctx.source,
        ctx.controller,
        &ctx.tagged_objects,
    );
    let mut allocations: std::collections::BTreeMap<ObjectId, u32> =
        std::collections::BTreeMap::new();
    if effect.single_object {
        valid_targets.retain(|object_id| {
            game.object(*object_id).is_some_and(|object| {
                available_counter_count(effect, object) >= u64::from(requested_count)
            })
        });
        let chosen = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            ChooseObjectsSpec::new(
                ctx.source,
                "Choose one object to remove counters from",
                valid_targets.clone(),
                1,
                Some(1),
            ),
            FallbackStrategy::Maximum,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SelectedCounterRemovalPlan::Finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let Some(object_id) = chosen
            .into_iter()
            .find(|object_id| valid_targets.contains(object_id))
        else {
            return Ok(SelectedCounterRemovalPlan::Finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::impossible()),
            ));
        };
        allocations.insert(object_id, requested_count);
        valid_targets = vec![object_id];
    } else {
        let distribute_targets: Vec<Target> =
            valid_targets.iter().copied().map(Target::Object).collect();
        let distribution = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            DistributeSpec::counters(ctx.source, requested_count, distribute_targets),
            FallbackStrategy::Maximum,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SelectedCounterRemovalPlan::Finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        for (target, amount) in distribution {
            if let Target::Object(object_id) = target {
                let available = game
                    .object(object_id)
                    .map(|object| available_counter_count(effect, object))
                    .unwrap_or(0);
                let already_allocated = allocations.get(&object_id).copied().unwrap_or(0);
                let free_capacity = available.saturating_sub(u64::from(already_allocated));
                let total_allocated: u32 = allocations.values().copied().sum();
                let remaining_total = requested_count.saturating_sub(total_allocated);
                let accepted = amount
                    .min(u32::try_from(free_capacity).unwrap_or(u32::MAX))
                    .min(remaining_total);
                if accepted > 0 {
                    *allocations.entry(object_id).or_insert(0) += accepted;
                }
            }
        }
    }

    let distributed_total: u32 = allocations.values().copied().sum();
    if distributed_total > requested_count {
        return Ok(SelectedCounterRemovalPlan::Finished(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::impossible()),
        ));
    }

    if distributed_total < requested_count {
        let mut remaining = requested_count - distributed_total;
        for object_id in &valid_targets {
            if remaining == 0 {
                break;
            }
            let available_total = game
                .object(*object_id)
                .map(|obj| available_counter_count(effect, obj))
                .unwrap_or(0);
            let already_allocated = allocations.get(object_id).copied().unwrap_or(0);
            let free_capacity = available_total.saturating_sub(u64::from(already_allocated));
            if free_capacity == 0 {
                continue;
            }
            let add = remaining.min(u32::try_from(free_capacity).unwrap_or(u32::MAX));
            *allocations.entry(*object_id).or_insert(0) += add;
            remaining -= add;
        }
        if remaining > 0 {
            return Ok(SelectedCounterRemovalPlan::Finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::impossible()),
            ));
        }
    }

    let mut events = Vec::new();
    for (object_id, amount_for_target) in allocations {
        if amount_for_target == 0 {
            continue;
        }
        let selections = if let Some(kind) = effect.counter_type {
            vec![(kind, amount_for_target)]
        } else {
            let available_counters: Vec<(CounterType, u32)> = game
                .object(object_id)
                .map(|object| {
                    object
                        .counters
                        .iter()
                        .filter(|(_, count)| **count > 0)
                        .map(|(kind, count)| (*kind, *count))
                        .collect()
                })
                .unwrap_or_default();
            make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                CounterRemovalSpec::new(
                    ctx.source,
                    object_id,
                    amount_for_target,
                    available_counters,
                ),
                FallbackStrategy::Maximum,
            )
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SelectedCounterRemovalPlan::Finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let mut selected_for_target = 0u32;
        for (kind, requested) in selections {
            if selected_for_target >= amount_for_target {
                break;
            }
            let amount = requested.min(amount_for_target - selected_for_target);
            if amount == 0 {
                continue;
            }
            let event = crate::events::Event::remove_counters(object_id, kind, amount)
                .with_provenance(ctx.provenance);
            selected_for_target += amount;
            events.push(event);
        }
        if selected_for_target != amount_for_target {
            return Err(ExecutionError::Impossible(
                "counter-removal choices did not fulfill the assigned amount".into(),
            ));
        }
    }
    Ok(SelectedCounterRemovalPlan::Groups {
        events,
        requested: u64::from(requested_count),
    })
}

/// Plan a wide total as sparse per-object/per-kind amounts. The enclosing
/// instruction owns rollback on pending replacement/selection or failure.
pub(super) fn select_wide_counter_removal_among(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    filter: ObjectFilter,
    counter_type: Option<CounterType>,
    maximum: u64,
    up_to: bool,
) -> Result<SelectedCounterRemovalPlan, ExecutionError> {
    let selector = RemoveAnyCountersAmongEffect::dynamic(0, u32::MAX, filter, false)
        .with_counter_type(counter_type);
    let candidates = valid_targets_with_tags(
        &selector,
        game,
        ctx.source,
        ctx.controller,
        &ctx.tagged_objects,
    );
    let capacities: Vec<(ObjectId, Vec<(CounterType, u32)>, u64)> = candidates
        .into_iter()
        .filter_map(|id| {
            game.object(id).map(|object| {
                let counters: Vec<_> = object
                    .counters
                    .iter()
                    .filter(|(kind, count)| {
                        **count > 0 && counter_type.is_none_or(|wanted| wanted == **kind)
                    })
                    .map(|(kind, count)| (*kind, *count))
                    .collect();
                let total = counters.iter().fold(0u64, |sum, (_, count)| {
                    sum.saturating_add(u64::from(*count))
                });
                (id, counters, total)
            })
        })
        .collect();
    // Capacity saturation is sufficient for a budget bounded by an i64 Value;
    // selected and actual outcome totals below use checked arithmetic.
    let available = capacities
        .iter()
        .fold(0u64, |sum, (_, _, n)| sum.saturating_add(*n));
    let budget = maximum.min(available);
    let minimum = if up_to { 0 } else { budget };
    let mut selected_total = 0u64;
    let mut plan = Vec::new();
    for (index, (object_id, counters, capacity)) in capacities.iter().enumerate() {
        if selected_total == budget {
            break;
        }
        let remaining = budget - selected_total;
        let future = capacities[index + 1..]
            .iter()
            .fold(0u64, |sum, (_, _, n)| sum.saturating_add(*n));
        let local_minimum = minimum
            .saturating_sub(selected_total)
            .saturating_sub(future);
        let local_maximum = remaining.min(*capacity);
        let spec = CounterRemovalSpec::for_target_wide(
            ctx.source,
            Target::Object(*object_id),
            local_maximum,
            counters.clone(),
        )
        .with_min_total_wide(local_minimum);
        let fallback = spec.default_response(FallbackStrategy::Maximum);
        let chosen = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
            FallbackStrategy::Maximum,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SelectedCounterRemovalPlan::Finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let normalize = |chosen: Vec<(CounterType, u32)>| {
            let mut remaining_by_kind: std::collections::BTreeMap<_, _> =
                counters.iter().copied().collect();
            // Coalesce duplicates at their first selected position. Sorting this
            // sequence would reorder replacement payloads and later choices.
            let mut amounts = Vec::<(CounterType, u32)>::new();
            let mut total = 0u64;
            for (kind, requested) in chosen {
                let Some(available) = remaining_by_kind.get_mut(&kind) else {
                    continue;
                };
                let amount = requested
                    .min(*available)
                    .min(u32::try_from(local_maximum - total).unwrap_or(u32::MAX));
                *available -= amount;
                if amount > 0 {
                    if let Some((_, previous)) =
                        amounts.iter_mut().find(|(selected, _)| *selected == kind)
                    {
                        *previous += amount; // Bounded by the original per-kind capacity.
                    } else {
                        amounts.push((kind, amount));
                    }
                    total += u64::from(amount);
                }
            }
            (amounts, total)
        };
        let (mut amounts, mut total) = normalize(chosen);
        if total < local_minimum {
            (amounts, total) = normalize(fallback);
        }
        if total < local_minimum {
            return Err(ExecutionError::Impossible(
                "counter-removal choices did not fulfill the remaining budget".into(),
            ));
        }
        selected_total = selected_total.checked_add(total).ok_or_else(|| {
            ExecutionError::InternalError("selected counter total overflow".into())
        })?;
        plan.push((*object_id, amounts));
    }
    if selected_total < minimum {
        return Err(ExecutionError::Impossible(
            "counter-removal choices did not fulfill the total budget".into(),
        ));
    }
    let mut events = Vec::new();
    for (object_id, amounts) in plan {
        for (kind, amount) in amounts {
            events.push(
                crate::events::Event::remove_counters(object_id, kind, amount)
                    .with_provenance(ctx.provenance),
            );
        }
    }
    Ok(SelectedCounterRemovalPlan::Groups {
        events,
        requested: selected_total,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::costs::{Cost, CostContext};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_test_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn simple_card(name: &str, raw_id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(raw_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .build()
    }

    #[test]
    fn can_pay_with_total_across_permanents() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card_a = simple_card("A", 1);
        let a_id = game.create_object_from_card(&card_a, alice, Zone::Battlefield);
        let card_b = simple_card("B", 2);
        let b_id = game.create_object_from_card(&card_b, alice, Zone::Battlefield);

        if let Some(obj) = game.object_mut(a_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 1);
        }
        if let Some(obj) = game.object_mut(b_id) {
            obj.counters.insert(CounterType::Charge, 2);
        }

        let cost = Cost::effect(RemoveAnyCountersAmongEffect::new(
            3,
            ObjectFilter::creature().you_control(),
        ));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(a_id, alice, &mut dm);
        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn pay_removes_counters() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("A", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        }

        let cost = Cost::effect(RemoveAnyCountersAmongEffect::new(
            2,
            ObjectFilter::creature().you_control(),
        ));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(crate::costs::CostPaymentResult::Paid));
        assert_eq!(game.counter_count(card_id, CounterType::PlusOnePlusOne), 1);
        assert_eq!(ctx.x_value, Some(2));
    }

    #[test]
    fn dynamic_cost_removes_chosen_counter_total_and_sets_x() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card_a = simple_card("A", 1);
        let a_id = game.create_object_from_card(&card_a, alice, Zone::Battlefield);
        let card_b = simple_card("B", 2);
        let b_id = game.create_object_from_card(&card_b, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(a_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 2);
        }
        if let Some(obj) = game.object_mut(b_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 1);
        }

        let cost = Cost::effect(
            RemoveAnyCountersAmongEffect::dynamic(
                1,
                u32::MAX / 4,
                ObjectFilter::creature().you_control(),
                false,
            )
            .with_counter_type(Some(CounterType::PlusOnePlusOne)),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(a_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(crate::costs::CostPaymentResult::Paid));
        assert_eq!(game.counter_count(a_id, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(b_id, CounterType::PlusOnePlusOne), 0);
        assert_eq!(ctx.x_value, Some(3));
    }

    #[test]
    fn single_object_dynamic_cost_cannot_pool_counters_across_permanents() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card_a = simple_card("A", 31);
        let a_id = game.create_object_from_card(&card_a, alice, Zone::Battlefield);
        let card_b = simple_card("B", 32);
        let b_id = game.create_object_from_card(&card_b, alice, Zone::Battlefield);
        game.object_mut(a_id)
            .unwrap()
            .counters
            .insert(CounterType::Charge, 2);
        game.object_mut(b_id)
            .unwrap()
            .counters
            .insert(CounterType::Charge, 1);

        let effect = RemoveAnyCountersAmongEffect::dynamic(
            0,
            u32::MAX / 4,
            ObjectFilter::creature().you_control(),
            true,
        )
        .from_single_object();
        assert_eq!(
            cost_display(&effect),
            "Remove X counters from a creature you control"
        );

        let impossible = Cost::effect(effect.clone());
        let mut impossible_dm = crate::decision::SelectFirstDecisionMaker;
        let mut impossible_ctx = CostContext::new(a_id, alice, &mut impossible_dm);
        impossible_ctx.x_value = Some(3);
        assert!(
            impossible.can_pay(&game, &impossible_ctx).is_err(),
            "three counters spread over two creatures cannot pay a single-object cost"
        );

        let payable = Cost::effect(effect);
        let mut payable_dm = crate::decision::SelectFirstDecisionMaker;
        let mut payable_ctx = CostContext::new(a_id, alice, &mut payable_dm);
        payable_ctx.x_value = Some(2);
        assert_eq!(
            payable.pay(&mut game, &mut payable_ctx),
            Ok(crate::costs::CostPaymentResult::Paid)
        );
        assert_eq!(game.counter_count(a_id, CounterType::Charge), 0);
        assert_eq!(game.counter_count(b_id, CounterType::Charge), 1);
    }

    #[test]
    fn single_object_union_cost_preserves_or_surface_and_article() {
        let mut filter = ObjectFilter::default().you_control();
        filter.card_types = vec![CardType::Artifact, CardType::Creature];
        let effect = RemoveAnyCountersAmongEffect::dynamic(0, u32::MAX / 4, filter, true)
            .from_single_object();
        assert_eq!(
            cost_display(&effect),
            "Remove X counters from an artifact or creature you control"
        );
    }

    #[test]
    fn single_counter_cost_preserves_explicit_permanent_type_list() {
        let mut filter = ObjectFilter::default().you_control();
        filter.card_types = vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Land,
            CardType::Planeswalker,
        ];
        let effect = RemoveAnyCountersAmongEffect::new(1, filter).from_single_object();

        assert_eq!(
            cost_display(&effect),
            "Remove a counter from an artifact, creature, land, or planeswalker you control"
        );
    }

    #[test]
    fn aggregate_union_cost_preserves_list_style_and_surface() {
        let mut filter = ObjectFilter::default().you_control();
        filter.card_types = vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Planeswalker,
        ];
        let effect = RemoveAnyCountersAmongEffect::new(3, filter);
        assert_eq!(
            cost_display(&effect),
            "Remove 3 counters from among artifacts, creatures, and planeswalkers you control"
        );
    }

    #[test]
    fn display_permanent_you_control_singular() {
        let effect = RemoveAnyCountersAmongEffect::new(1, ObjectFilter::permanent().you_control());
        assert_eq!(
            cost_display(&effect),
            "Remove a counter from a permanent you control"
        );
    }

    #[test]
    fn display_dynamic_one_or_more_typed_counters_among_creatures() {
        let effect = RemoveAnyCountersAmongEffect::dynamic(
            1,
            u32::MAX / 4,
            ObjectFilter::creature().you_control(),
            false,
        )
        .with_counter_type(Some(CounterType::PlusOnePlusOne));
        assert_eq!(
            cost_display(&effect),
            "Remove one or more +1/+1 counters from among creatures you control"
        );
    }

    #[test]
    fn display_time_counter_from_permanent_or_suspended_card_cost() {
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![
            ObjectFilter::permanent().you_control(),
            ObjectFilter::default()
                .in_zone(Zone::Exile)
                .owned_by(PlayerFilter::You)
                .with_alternative_cast(AlternativeCastKind::Suspend),
        ];
        let effect =
            RemoveAnyCountersAmongEffect::new(1, filter).with_counter_type(Some(CounterType::Time));

        assert_eq!(
            cost_display(&effect),
            "Remove a time counter from a permanent you control or suspended card you own"
        );
    }

    #[test]
    fn time_counter_cost_can_remove_from_owned_suspended_card_in_exile() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source_card = simple_card("Source", 1);
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let suspended_card = simple_card("Suspended", 2);
        let suspended_id = game.create_object_from_card(&suspended_card, alice, Zone::Exile);
        if let Some(obj) = game.object_mut(suspended_id) {
            obj.alternative_casts.push(
                crate::alternative_cast::AlternativeCastingMethod::Suspend {
                    cost: ManaCost::default(),
                    time: ironsmith_core::SuspendTime::Fixed(1),
                },
            );
            obj.counters.insert(CounterType::Time, 1);
        }

        let mut filter = ObjectFilter::default();
        filter.any_of = vec![
            ObjectFilter::permanent().you_control(),
            ObjectFilter::default()
                .in_zone(Zone::Exile)
                .owned_by(PlayerFilter::You)
                .with_alternative_cast(AlternativeCastKind::Suspend),
        ];
        let cost = Cost::effect(
            RemoveAnyCountersAmongEffect::new(1, filter).with_counter_type(Some(CounterType::Time)),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source_id, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
        assert_eq!(
            cost.pay(&mut game, &mut ctx),
            Ok(crate::costs::CostPaymentResult::Paid)
        );
        assert_eq!(game.counter_count(suspended_id, CounterType::Time), 0);
    }

    #[test]
    fn typed_counters_cannot_pay_without_type() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("A", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::Charge, 2);
        }

        let cost = Cost::effect(
            RemoveAnyCountersAmongEffect::new(1, ObjectFilter::creature().you_control())
                .with_counter_type(Some(CounterType::PlusOnePlusOne)),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(card_id, alice, &mut dm);
        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(crate::cost::CostPaymentError::Other(
                "not enough counters".to_string()
            ))
        );
    }

    #[test]
    fn typed_counters_pay_removes_only_typed_counters() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("A", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 3);
            obj.counters.insert(CounterType::Charge, 2);
        }

        let cost = Cost::effect(
            RemoveAnyCountersAmongEffect::new(2, ObjectFilter::creature().you_control())
                .with_counter_type(Some(CounterType::PlusOnePlusOne)),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(crate::costs::CostPaymentResult::Paid));
        assert_eq!(game.counter_count(card_id, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(card_id, CounterType::Charge), 2);
    }
}

#[cfg(test)]
mod distributed_quantity_event_contract_tests {
    use super::*;
    use crate::effect::{Effect,EffectId,Value};
    use crate::effects::{execute_effect,PutCountersEffect,RemoveUpToCountersEffect,RemoveUpToAnyCountersEffect};
    use crate::ids::CardId;
    fn object(game:&mut GameState,alice:PlayerId)->ObjectId {
        let card=crate::card::CardBuilder::new(CardId::new(),"Distributed counter budget owner").card_types(vec![CardType::Artifact]).build();game.create_object_from_card(&card,alice,Zone::Battlefield)
    }
    fn fixture()->(GameState,ObjectId,PlayerId) {
        let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);(game,source,alice)
    }
    fn put(game:&mut GameState,ctx:&mut ExecutionContext,id:ObjectId,kind:CounterType,count:u32,receipt:u32) {
        let out=execute_effect(game,&Effect::with_id(receipt,Effect::new(PutCountersEffect::new(kind,count,ChooseSpec::SpecificObject(id)))),ctx).unwrap();assert_eq!(out.as_count(),Some(i64::from(count)));assert_eq!(game.counter_count(id,kind),count);
    }
    fn source_filter()->ObjectFilter {let mut filter=ObjectFilter::permanent();filter.source=true;filter}
    #[test] fn fixed_among_small_budget_accepts_mixed_available_above_u32() {
        let (mut game,source,alice)=fixture();let mut ctx=ExecutionContext::new_default(source,alice);for (kind,id) in [(CounterType::Charge,31),(CounterType::PlusOnePlusOne,32)] {put(&mut game,&mut ctx,source,kind,u32::MAX,id);}
        let out=RemoveAnyCountersAmongEffect::new(5,source_filter()).execute(&mut game,&mut ctx).expect("mixed availability must not overflow before small removal");assert_eq!(out.as_count(),Some(5));assert_eq!(u64::from(game.counter_count(source,CounterType::Charge))+u64::from(game.counter_count(source,CounterType::PlusOnePlusOne)),2*u64::from(u32::MAX)-5);
    }
    #[test] fn fixed_among_small_budget_accepts_total_across_large_objects() {
        let (mut game,source,alice)=fixture();let second=object(&mut game,alice);let mut ctx=ExecutionContext::new_default(source,alice);for (object,id) in [(source,31),(second,32)] {put(&mut game,&mut ctx,object,CounterType::Charge,u32::MAX,id);}
        let out=RemoveAnyCountersAmongEffect::new(5,ObjectFilter::permanent().you_control()).with_counter_type(Some(CounterType::Charge)).execute(&mut game,&mut ctx).expect("cross-object availability must not overflow before small removal");assert_eq!(out.as_count(),Some(5));assert_eq!(u64::from(game.counter_count(source,CounterType::Charge))+u64::from(game.counter_count(second,CounterType::Charge)),2*u64::from(u32::MAX)-5);
    }
    #[test] fn among_cost_validation_accepts_small_cost_with_mixed_available() {
        let (mut game,source,alice)=fixture();let mut ctx=ExecutionContext::new_default(source,alice);for (kind,id) in [(CounterType::Charge,31),(CounterType::PlusOnePlusOne,32)] {put(&mut game,&mut ctx,source,kind,u32::MAX,id);}
        let effect=RemoveAnyCountersAmongEffect::new(5,source_filter());assert!(CostExecutableEffect::can_execute_as_cost(&effect,&game,source,alice).is_ok());assert_eq!(game.counter_count(source,CounterType::Charge),u32::MAX);assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),u32::MAX);
    }
    #[test] fn typed_all_caps_actual_unsigned_prior_by_available_matching_counters() {
        let (mut game,source,alice)=fixture();let donor=object(&mut game,alice);let mut ctx=ExecutionContext::new_default(source,alice);put(&mut game,&mut ctx,donor,CounterType::Charge,u32::MAX,31);put(&mut game,&mut ctx,source,CounterType::Charge,3,32);
        let out=RemoveUpToCountersEffect::new(CounterType::Charge,Value::EffectValue(EffectId(31)),ChooseSpec::All(source_filter())).execute(&mut game,&mut ctx).expect("distributed typed limit must be bounded by real available counters");assert_eq!(out.as_count(),Some(3));assert_eq!(game.counter_count(source,CounterType::Charge),0);assert_eq!(game.counter_count(donor,CounterType::Charge),u32::MAX);
    }
    fn wide_all(typed:bool) {
        let (mut game,source,alice)=fixture();let second=if typed{object(&mut game,alice)}else{source};let following=object(&mut game,alice);let mut ctx=ExecutionContext::new_default(source,alice);put(&mut game,&mut ctx,source,CounterType::Charge,u32::MAX,31);put(&mut game,&mut ctx,second,if typed{CounterType::Charge}else{CounterType::PlusOnePlusOne},u32::MAX,32);
        let count=Value::Add(Box::new(Value::EffectValue(EffectId(31))),Box::new(Value::EffectValue(EffectId(32))));let effect=if typed{Effect::new(RemoveUpToCountersEffect::new(CounterType::Charge,count,ChooseSpec::All(ObjectFilter::permanent().you_control())))}else{Effect::new(RemoveUpToAnyCountersEffect::exact(count,ChooseSpec::All(source_filter())))};
        let out=execute_effect(&mut game,&Effect::with_id(57,effect),&mut ctx).expect("distributed budget must preserve both actual prior receipts");assert_eq!(out.as_count(),Some(2*i64::from(u32::MAX)));assert_eq!(game.counter_count(source,CounterType::Charge),0);assert_eq!(game.counter_count(second,if typed{CounterType::Charge}else{CounterType::PlusOnePlusOne}),0);assert_eq!(out.events.len(),2);
        let follow=Effect::new(PutCountersEffect::new(CounterType::Charge,Value::HalfRoundedDown(Box::new(Value::EffectValue(EffectId(57)))),ChooseSpec::SpecificObject(following)));assert_eq!(execute_effect(&mut game,&follow,&mut ctx).unwrap().as_count(),Some(i64::from(u32::MAX)));assert_eq!(game.counter_count(following,CounterType::Charge),u32::MAX);
    }
    #[test] fn typed_all_wide_budget_spans_two_objects_and_receipt_drives_followup() {wide_all(true);}
    #[test] fn any_all_wide_budget_spans_two_kinds_and_receipt_drives_followup() {wide_all(false);}
}

#[cfg(test)]
mod wide_distributed_continuation_contract_tests {
    use super::*;
    use crate::effect::{Effect,EffectId,EffectOutcome,Value};
    use crate::effects::{execute_effect,PutCountersEffect,RemoveUpToAnyCountersEffect,RemoveUpToCountersEffect};
    use crate::replacement::{ReplacementAction,ReplacementEffect,EventModification};
    use crate::events::counters::matchers::WouldRemoveCountersMatcher;
    use crate::ids::CardId;
    fn object(game:&mut GameState,alice:PlayerId)->ObjectId {
        let card=crate::card::CardBuilder::new(CardId::new(),"Wide continuation owner").card_types(vec![CardType::Artifact]).build();game.create_object_from_card(&card,alice,Zone::Battlefield)
    }
    fn fixture(split:bool)->(GameState,ObjectId,ObjectId,PlayerId) {
        let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);let other=if split{object(&mut game,alice)}else{source};(game,source,other,alice)
    }
    fn seed(game:&mut GameState,source:ObjectId,other:ObjectId,alice:PlayerId,ctx:&mut ExecutionContext) {
        for (id,kind,receipt) in [(source,CounterType::Charge,31),(other,if other==source{CounterType::PlusOnePlusOne}else{CounterType::Charge},32)] {
            let put=Effect::with_id(receipt,Effect::new(PutCountersEffect::new(kind,u32::MAX,ChooseSpec::SpecificObject(id))));assert_eq!(execute_effect(game,&put,ctx).unwrap().as_count(),Some(i64::from(u32::MAX)));
        }
        ctx.store_outcome(EffectId(57),EffectOutcome::count(7));ctx.set_tagged_players("retained",vec![alice]);game.take_pending_trigger_events();
    }
    fn instruction(split:bool)->Effect {
        let count=Value::Add(Box::new(Value::EffectValue(EffectId(31))),Box::new(Value::EffectValue(EffectId(32))));let mut filter=ObjectFilter::permanent().you_control();if !split{filter.source=true;}
        let effect=if split{Effect::new(RemoveUpToCountersEffect::new(CounterType::Charge,count,ChooseSpec::All(filter)))}else{Effect::new(RemoveUpToAnyCountersEffect::exact(count,ChooseSpec::All(filter)))};Effect::with_id(57,effect)
    }
    fn replacement(game:&mut GameState,source:ObjectId,alice:PlayerId,kind:CounterType,action:ReplacementAction)->crate::replacement::ReplacementEffectId {
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source,alice,WouldRemoveCountersMatcher::new(ObjectFilter::permanent(),Some(kind)),action))
    }
    struct Decisions {pause_replacement:bool,pause_selection:bool,pending:bool,selections:usize,choices:usize,source:ObjectId,other:ObjectId}
    impl crate::decision::DecisionMaker for Decisions {
        fn awaiting_choice(&self)->bool{self.pending}
        fn decide_counters(&mut self,game:&GameState,ctx:&crate::decisions::context::CountersContext)->Vec<(CounterType,u32)> {
            assert!(!self.pending,"continued past unanswered wide selection");self.selections+=1;
            assert_eq!(game.counter_count(self.source,CounterType::Charge),u32::MAX,"all allocations must be selected before commit");
            if self.pause_selection && self.selections==2 {assert_eq!(game.counter_count(self.other,CounterType::Charge),u32::MAX);self.pending=true;return vec![];}
            [CounterType::Charge,CounterType::PlusOnePlusOne].into_iter().filter(|kind|ctx.available_counters.iter().any(|(k,_)|k==kind)).map(|kind|(kind,u32::MAX)).collect()
        }
        fn decide_options(&mut self,game:&GameState,ctx:&crate::decisions::context::SelectOptionsContext)->Vec<usize> {
            assert!(!self.pending);self.choices+=1;assert_eq!(ctx.player,PlayerId::from_index(0));assert_eq!(ctx.options.len(),2);assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life,21,"first selected kind's replacement must execute before the next kind's choice");
            if self.pause_replacement{self.pending=true;vec![]}else{vec![ctx.options.iter().find(|option|option.legal).unwrap().index]}
        }
    }
    #[derive(Debug,Clone)]struct FailLater;
    impl EffectExecutor for FailLater {
        fn execute(&self,game:&mut GameState,_ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError> {
            if game.player(PlayerId::from_index(0)).unwrap().life!=21 {return Err(ExecutionError::InternalError("earlier replacement was reordered behind later failure".into()));}
            Err(ExecutionError::InternalError("injected wide removal failure".into()))
        }
    }
    fn transaction(pause:bool) {
        let (mut game,source,other,alice)=fixture(false);let mut dm=Decisions{pause_replacement:pause,pause_selection:false,pending:false,selections:0,choices:0,source,other};let mut ctx=ExecutionContext::new(source,alice,&mut dm);seed(&mut game,source,other,alice,&mut ctx);
        let first=replacement(&mut game,source,alice,CounterType::Charge,ReplacementAction::Instead(vec![Effect::gain_life(1)]));let mut shields=vec![first];
        if pause {for modification in [EventModification::Add(0),EventModification::Subtract(1)] {shields.push(replacement(&mut game,source,alice,CounterType::PlusOnePlusOne,ReplacementAction::Modify(modification)));}}
        else{shields.push(replacement(&mut game,source,alice,CounterType::PlusOnePlusOne,ReplacementAction::Instead(vec![Effect::new(FailLater)])));}
        let effect=instruction(false);let result=execute_effect(&mut game,&effect,&mut ctx);
        if pause{assert!(ctx.decision_maker.awaiting_choice());let out=result.unwrap();assert_eq!(out.as_count(),Some(0));assert!(out.events.is_empty());}
        else{assert_eq!(result.unwrap_err(),ExecutionError::InternalError("injected wide removal failure".into()));}
        assert_eq!(game.player(alice).unwrap().life,20);assert_eq!(game.counter_count(source,CounterType::Charge),u32::MAX);assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),u32::MAX);assert!(shields.iter().all(|id|game.effect_store.replacement_effects.get_effect(*id).is_some()));assert!(game.take_pending_trigger_events().is_empty());assert_eq!(ctx.get_tagged_players("retained"),Some(&vec![alice]));assert_eq!(ctx.get_outcome(EffectId(57)).unwrap().as_count(),Some(7));for id in [31,32]{assert_eq!(ctx.get_outcome(EffectId(id)).unwrap().as_count(),Some(i64::from(u32::MAX)));}
        if pause{
            let saved=crate::effects::ExecutionContextCheckpoint::capture(&ctx);drop(ctx);let mut replay=Decisions{pause_replacement:false,pause_selection:false,pending:false,selections:0,choices:0,source,other};let mut ctx=ExecutionContext::new(source,alice,&mut replay);saved.restore(&mut ctx);let out=execute_effect(&mut game,&effect,&mut ctx).unwrap();assert!(!ctx.decision_maker.awaiting_choice());assert_eq!(out.as_count(),Some(i64::from(u32::MAX)-1));assert_eq!(game.player(alice).unwrap().life,21);assert_eq!(game.counter_count(source,CounterType::Charge),u32::MAX);assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),1);assert!(shields.iter().all(|id|game.effect_store.replacement_effects.get_effect(*id).is_none()));assert_eq!(out.events_of_type::<crate::events::LifeGainEvent>().count(),1);assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(),1);assert_eq!(ctx.get_outcome(EffectId(57)).unwrap().as_count(),Some(i64::from(u32::MAX)-1));assert!(game.take_pending_trigger_events().is_empty());
        }
    }
    #[test]fn wide_later_replacement_pause_restores_then_replays_once(){transaction(true);}
    #[test]fn wide_later_replacement_failure_restores_all_state(){transaction(false);}
    fn partial(instead:bool) {
        let (mut game,source,other,alice)=fixture(false);let following=object(&mut game,alice);let mut dm=Decisions{pause_replacement:false,pause_selection:false,pending:false,selections:0,choices:0,source,other};let mut ctx=ExecutionContext::new(source,alice,&mut dm);seed(&mut game,source,other,alice,&mut ctx);
        let action=if instead{ReplacementAction::Instead(vec![Effect::gain_life(2)])}else{ReplacementAction::Prevent};let shield=replacement(&mut game,source,alice,CounterType::Charge,action);let effect=instruction(false);let out=execute_effect(&mut game,&effect,&mut ctx).unwrap();assert_eq!(out.as_count(),Some(i64::from(u32::MAX)));assert_eq!(game.counter_count(source,CounterType::Charge),u32::MAX);assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),0);assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(),1);assert_eq!(out.events_of_type::<crate::events::LifeGainEvent>().count(),usize::from(instead));if instead{assert!(out.events[0].downcast::<crate::events::LifeGainEvent>().is_some(),"replacement payload must preserve selected kind order");}assert_eq!(game.player(alice).unwrap().life,if instead{22}else{20});assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        let follow=Effect::new(PutCountersEffect::new(CounterType::Charge,Value::EffectValue(EffectId(57)),ChooseSpec::SpecificObject(following)));assert_eq!(execute_effect(&mut game,&follow,&mut ctx).unwrap().as_count(),Some(i64::from(u32::MAX)));assert_eq!(game.counter_count(following,CounterType::Charge),u32::MAX);
    }
    #[test]fn wide_prevention_keeps_selected_budget_separate_from_actual_receipt(){partial(false);}
    #[test]fn wide_instead_keeps_payload_order_and_unaffected_group(){partial(true);}
    #[test]fn wide_second_object_selection_pause_restores_and_replays_full_budget() {
        let (mut game,source,other,alice)=fixture(true);let mut dm=Decisions{pause_replacement:false,pause_selection:true,pending:false,selections:0,choices:0,source,other};let mut ctx=ExecutionContext::new(source,alice,&mut dm);seed(&mut game,source,other,alice,&mut ctx);let effect=instruction(true);let out=execute_effect(&mut game,&effect,&mut ctx).unwrap();assert!(ctx.decision_maker.awaiting_choice());assert_eq!(out.as_count(),Some(0));assert!(out.events.is_empty());assert_eq!(game.counter_count(source,CounterType::Charge),u32::MAX);assert_eq!(game.counter_count(other,CounterType::Charge),u32::MAX);assert_eq!(ctx.get_outcome(EffectId(57)).unwrap().as_count(),Some(7));assert!(game.take_pending_trigger_events().is_empty());let saved=crate::effects::ExecutionContextCheckpoint::capture(&ctx);drop(ctx);
        let mut replay=Decisions{pause_replacement:false,pause_selection:false,pending:false,selections:0,choices:0,source,other};let mut ctx=ExecutionContext::new(source,alice,&mut replay);saved.restore(&mut ctx);let out=execute_effect(&mut game,&effect,&mut ctx).unwrap();assert_eq!(out.as_count(),Some(2*i64::from(u32::MAX)));assert_eq!(game.counter_count(source,CounterType::Charge),0);assert_eq!(game.counter_count(other,CounterType::Charge),0);assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(),2);assert!(!ctx.decision_maker.awaiting_choice());assert_eq!(ctx.get_outcome(EffectId(57)).unwrap().as_count(),Some(2*i64::from(u32::MAX)));assert!(game.take_pending_trigger_events().is_empty());
    }
}
