//! Reflexive trigger effect implementation.

use std::collections::HashMap;

use crate::card::LinkedFaceLayout;
use crate::decisions::context::{TargetRequirementContext, TargetsContext};
use crate::effect::{
    Effect, EffectId, EffectOutcome, EffectPredicate, EffectPredicateRuntimeExt,
    OutcomeObjectMemory,
};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::object::ObjectKind;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::targeting::normalize_targets_for_requirements;

/// Effect that creates a reflexive triggered ability from a prior effect result.
///
/// This models clauses like "When you do, ..." where the follow-up trigger is
/// created only if an earlier effect satisfied a result predicate, and targets
/// are chosen when that new ability is put onto the stack.
#[derive(Debug, Clone, PartialEq)]
pub struct ReflexiveTriggerEffect {
    /// The prior effect result to inspect.
    pub condition: EffectId,
    /// How to evaluate the prior effect result.
    pub predicate: EffectPredicate,
    /// Effects for the reflexive triggered ability.
    pub effects: Vec<Effect>,
    /// Target choices that must be made when the reflexive ability is created.
    pub choices: Vec<ChooseSpec>,
    /// Intervening-if of the reflexive ability ("When you do, if ...",
    /// CR 603.4), checked as the ability would trigger. The resolution-time
    /// recheck is part of `effects`.
    pub intervening_if: Option<crate::effect::Condition>,
}

impl ReflexiveTriggerEffect {
    pub fn new(
        condition: EffectId,
        predicate: EffectPredicate,
        effects: Vec<Effect>,
        choices: Vec<ChooseSpec>,
    ) -> Self {
        Self {
            condition,
            predicate,
            effects,
            choices,
            intervening_if: None,
        }
    }

    pub fn with_intervening_if(mut self, condition: Option<crate::effect::Condition>) -> Self {
        self.intervening_if = condition;
        self
    }
}

fn describe_choice(spec: &ChooseSpec) -> String {
    match spec.base() {
        ChooseSpec::Player(_) => "target player".to_string(),
        ChooseSpec::Object(_) => "target object".to_string(),
        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => "target".to_string(),
        ChooseSpec::PlayerOrPlaneswalker(_) => "target player or planeswalker".to_string(),
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            "target attacked player or planeswalker".to_string()
        }
        _ => "target".to_string(),
    }
}

fn resolve_reflexive_choice_spec(
    game: &GameState,
    ctx: &ExecutionContext,
    spec: &ChooseSpec,
) -> Option<ChooseSpec> {
    let specialized;
    let spec = if let Some(player) = ctx.iteration.iterated_player {
        specialized = crate::game_loop::specialize_iterated_player_choose_spec(spec, player);
        &specialized
    } else {
        spec
    };
    let mut count = spec.count();
    if !count.is_dynamic_x() {
        return Some(spec.clone());
    }
    let resolved = if let Some(count_value) = spec.count_value() {
        crate::effects::helpers::resolve_value(game, count_value, ctx)
            .ok()?
            .max(0) as usize
    } else {
        ctx.x_value? as usize
    };
    if count.is_up_to_dynamic_x() {
        count.min = 0;
    } else {
        count.min = resolved;
    }
    count.max = Some(resolved);
    count.dynamic_x = false;
    count.up_to_x = false;
    Some(spec.clone().with_count(count))
}

fn choose_reflexive_targets(
    game: &GameState,
    ctx: &mut ExecutionContext,
    choices: &[ChooseSpec],
) -> Option<(
    Vec<crate::game_state::Target>,
    Vec<crate::game_state::TargetAssignment>,
)> {
    let mut chosen_targets = Vec::new();
    let mut assignments = Vec::new();

    for spec in choices {
        let resolved_spec = resolve_reflexive_choice_spec(game, ctx, spec)?;
        let count = resolved_spec.count();
        let legal_targets = crate::targeting::compute_legal_targets_with_execution_context(
            game,
            &resolved_spec,
            ctx,
        );
        let legal_target_sets =
            crate::targeting::legal_target_sets_for_spec(game, &resolved_spec, &legal_targets);
        let aggregate_constraint =
            crate::targeting::resolved_target_aggregate_constraint_with_context(
                game,
                &resolved_spec,
                ctx,
                &legal_targets,
            )
            .ok()?;
        if !crate::targeting::has_enough_legal_targets_for_spec(
            game,
            &resolved_spec,
            &legal_targets,
            count.min,
        ) || aggregate_constraint
            .as_ref()
            .is_some_and(|constraint| !constraint.supports_minimum(count.min))
        {
            return None;
        }

        let targets_ctx = TargetsContext::new(
            ctx.controller,
            ctx.source,
            "reflexive triggered ability",
            vec![TargetRequirementContext {
                description: describe_choice(spec),
                legal_targets: legal_targets.clone(),
                legal_target_sets,
                aggregate_constraint,
                min_targets: count.min,
                max_targets: count.max,
                distinct_player_group: None,
                shared_player_group: None,
            }],
        );
        let selected = ctx.decision_maker.decide_targets(game, &targets_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return None;
        }
        let selected = normalize_targets_for_requirements(&targets_ctx.requirements, selected)?;

        let start = chosen_targets.len();
        chosen_targets.extend(selected);
        assignments.push(crate::game_state::TargetAssignment {
            spec: resolved_spec,
            range: start..chosen_targets.len(),
        });
    }

    Some((chosen_targets, assignments))
}

fn snapshot_from_memory(game: &GameState, memory: &OutcomeObjectMemory) -> ObjectSnapshot {
    let mut snapshot = game
        .object(memory.object_id)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        .unwrap_or_else(|| ObjectSnapshot {
            chosen_subtype: None,
            secret_chosen_subtype: None,
            noted_life_total: None,
            chosen_object: None,
            object_id: memory.object_id,
            stable_id: memory.stable_id,
            kind: if memory.is_token {
                ObjectKind::Token
            } else {
                ObjectKind::Card
            },
            card: None,
            controller: memory.controller,
            owner: memory.owner,
            name: String::new(),
            first_printed_set_name: None,
            mana_cost: None,
            colors: memory.colors,
            supertypes: Vec::new(),
            card_types: memory.card_types.clone(),
            subtypes: memory.subtypes.clone(),
            compiled_card_text: String::new(),
            ability_labels: Vec::new(),
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            linked_face_mana_value: None,
            power: memory.power,
            toughness: memory.toughness,
            base_power: memory.power,
            base_toughness: memory.toughness,
            loyalty: None,
            defense: None,
            abilities: std::sync::Arc::new(Vec::new()),
            aura_attach_filter: None,
            copiable_values: crate::snapshot::CopiableValues::default(),
            x_value: None,
            cast_order_this_turn: None,
            mana_spent_to_cast: crate::player::ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: None,
            snow_mana_spent_to_cast: crate::player::ManaPool::default(),
            mana_sources_spent_to_cast: Vec::new(),
            optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
            counters: std::collections::BTreeMap::new(),
            is_token: memory.is_token,
            tapped: false,
            attacking: false,
            goaded: None,
            ring_bearer: None,
            flipped: false,
            face_down: false,
            transform_count: 0,
            attached_to: None,
            attachments: Vec::new(),
            attachment_snapshots: Vec::new(),
            was_enchanted: false,
            is_monstrous: false,
            is_prepared: false,
            is_commander: false,
            zone: memory.zone,
        });

    snapshot.stable_id = memory.stable_id;
    snapshot.controller = memory.controller;
    snapshot.owner = memory.owner;
    snapshot.zone = memory.zone;
    snapshot.power = memory.power;
    snapshot.toughness = memory.toughness;
    snapshot.card_types = memory.card_types.clone();
    snapshot.colors = memory.colors;
    snapshot.subtypes = memory.subtypes.clone();
    snapshot.is_token = memory.is_token;
    snapshot
}

fn snapshots_from_object_ids(
    game: &GameState,
    ids: &[crate::ids::ObjectId],
) -> Vec<ObjectSnapshot> {
    ids.iter()
        .filter_map(|id| game.object(*id))
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        .collect()
}

fn reflexive_it_snapshots(game: &GameState, outcome: &EffectOutcome) -> Vec<ObjectSnapshot> {
    if let Some(memory) = outcome.affected_object_memory()
        && !memory.is_empty()
    {
        return memory
            .iter()
            .map(|memory| snapshot_from_memory(game, memory))
            .collect();
    }
    if let Some(memory) = outcome.chosen_object_memory()
        && !memory.is_empty()
    {
        return memory
            .iter()
            .map(|memory| snapshot_from_memory(game, memory))
            .collect();
    }
    if let Some(ids) = outcome.affected_objects()
        && !ids.is_empty()
    {
        return snapshots_from_object_ids(game, ids);
    }
    if let Some(ids) = outcome.chosen_objects()
        && !ids.is_empty()
    {
        return snapshots_from_object_ids(game, ids);
    }
    Vec::new()
}

impl EffectExecutor for ReflexiveTriggerEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // An antecedent that never ran left no result: it didn't happen, so
        // the reflexive ability doesn't trigger (CR 603.12).
        let outcome = ctx
            .get_outcome(self.condition)
            .cloned()
            .unwrap_or_else(EffectOutcome::impossible);
        if !self.predicate.evaluate_outcome(&outcome) {
            return Ok(EffectOutcome::resolved());
        }
        if let Some(condition) = &self.intervening_if
            && !crate::condition_eval::evaluate_condition_resolution(game, condition, ctx)?
        {
            return Ok(EffectOutcome::resolved());
        }
        let fallback_it_snapshots = reflexive_it_snapshots(game, &outcome);

        // X chosen while paying for the antecedent belongs to this follow-up,
        // even when the enclosing spell/ability had no X (or a different X).
        let parent_x = ctx.x_value;
        let reflexive_x = outcome
            .execution_facts()
            .iter()
            .rev()
            .find_map(|fact| match fact {
                crate::effect::ExecutionFact::ManaPaid { x_value } => Some(*x_value),
                _ => None,
            })
            .or(parent_x);
        let mut tagged_objects = ctx.tagged_objects.clone();
        let it_tag = TagKey::from("__it__");
        if !tagged_objects.contains_key(&it_tag) && !fallback_it_snapshots.is_empty() {
            tagged_objects.insert(it_tag, fallback_it_snapshots);
        }
        // The reflexive ability resolves later, as its own resolution: carry
        // the objects this resolution moved as the objects they became
        // (CR 400.7j), and nothing it couldn't follow itself.
        crate::effects::helpers::pin_tagged_objects_to_current(game, ctx, &mut tagged_objects);

        // CR 603.12: a reflexive triggered ability triggers now but, like any
        // triggered ability, is put on the stack the next time a player would
        // receive priority (after state-based actions, CR 603.3), in APNAP
        // order alongside its controller's other triggers; its targets are
        // chosen then (CR 603.3d). Remember what it needs from this
        // resolution and queue it.
        let id = game.effect_store.next_reflexive_trigger_id;
        game.effect_store.next_reflexive_trigger_id += 1;
        let trigger_identity = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            "reflexive_trigger".hash(&mut hasher);
            id.hash(&mut hasher);
            crate::triggers::TriggerIdentity(hasher.finish())
        };
        let (source_stable_id, source_name, source_snapshot) =
            if let Some(source) = game.object(ctx.source) {
                (
                    source.stable_id,
                    source.name.to_string(),
                    ctx.source_snapshot.clone().or_else(|| {
                        Some(ObjectSnapshot::from_object_with_calculated_characteristics(
                            source, game,
                        ))
                    }),
                )
            } else if let Some(snapshot) = ctx.source_snapshot.clone() {
                (
                    snapshot.stable_id,
                    snapshot.name.to_string(),
                    Some(snapshot),
                )
            } else {
                (
                    crate::ids::StableId::from(ctx.source),
                    "Reflexive trigger".to_string(),
                    None,
                )
            };
        let pending = PendingReflexiveTrigger {
            trigger_identity,
            source: ctx.source,
            controller: ctx.controller,
            effects: self.effects.clone(),
            choices: self.choices.clone(),
            tagged_objects: tagged_objects.clone(),
            tagged_players: ctx.tagged_players.clone(),
            effect_outcomes: ctx.effect_outcomes.clone(),
            targets: ctx.targets.clone(),
            x_value: reflexive_x,
            iteration: ctx.iteration,
            combat: ctx.combat,
            triggering_event: ctx.triggering_event.clone(),
            event_value_amount: ctx.event_value_amount,
            optional_costs_paid: ctx.optional_costs_paid.clone(),
            source_snapshot: source_snapshot.clone(),
        };
        game.effect_store.pending_reflexive_triggers.push(pending);

        let triggering_event = ctx.triggering_event.clone().unwrap_or_else(|| {
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::StateTriggerEvent::new(ctx.source),
                ctx.provenance,
            )
        });
        game.defer_trigger_entries([crate::triggers::TriggeredAbilityEntry {
            source: ctx.source,
            controller: ctx.controller,
            x_value: reflexive_x,
            event_value_amount: ctx.event_value_amount,
            ability: crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::custom(
                    REFLEXIVE_TRIGGER_ID,
                    "When you do".to_string(),
                ),
                effects: crate::resolution::ResolutionProgram::from_effects(self.effects.clone()),
                choices: Vec::new(),
                intervening_if: None,
                presentation_label: None,
            },
            triggering_event,
            source_stable_id,
            source_name,
            source_snapshot,
            tagged_objects,
            source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
            trigger_identity,
        }]);
        Ok(EffectOutcome::count(1))
    }
}

/// Queue a reflexive triggered ability that a replacement effect's choice
/// created rather than a resolving instruction ("You may have this enter as a
/// copy of ... . When you do, exile that card.", CR 603.12). It triggers now
/// and is put on the stack the next time a player would receive priority.
pub(crate) fn queue_reflexive_trigger(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    effects: Vec<Effect>,
    tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
) {
    queue_reflexive_trigger_with_source_snapshot(
        game,
        source,
        controller,
        effects,
        tagged_objects,
        None,
    );
}

pub(crate) fn queue_reflexive_trigger_with_source_snapshot(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    effects: Vec<Effect>,
    tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    fallback_snapshot: Option<ObjectSnapshot>,
) {
    let id = game.effect_store.next_reflexive_trigger_id;
    game.effect_store.next_reflexive_trigger_id += 1;
    let trigger_identity = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        "reflexive_trigger".hash(&mut hasher);
        id.hash(&mut hasher);
        crate::triggers::TriggerIdentity(hasher.finish())
    };
    let fallback_snapshot = fallback_snapshot.or_else(|| {
        game.turn_store
            .turn_history
            .source_last_known_snapshot(source)
            .cloned()
    });
    let (source_stable_id, source_name, source_snapshot) =
        match game.object(source).filter(|_| !game.is_phased_out(source)) {
            Some(object) => (
                object.stable_id,
                object.name.to_string(),
                Some(ObjectSnapshot::from_object_with_calculated_characteristics(
                    object, game,
                )),
            ),
            None => match fallback_snapshot {
                Some(snapshot) => (
                    snapshot.stable_id,
                    snapshot.name.to_string(),
                    Some(snapshot),
                ),
                None => (
                    crate::ids::StableId::from(source),
                    "Reflexive trigger".to_string(),
                    None,
                ),
            },
        };
    game.effect_store
        .pending_reflexive_triggers
        .push(PendingReflexiveTrigger {
            trigger_identity,
            source,
            controller,
            effects: effects.clone(),
            choices: Vec::new(),
            tagged_objects: tagged_objects.clone(),
            tagged_players: HashMap::new(),
            effect_outcomes: HashMap::new(),
            targets: Vec::new(),
            x_value: None,
            iteration: Default::default(),
            combat: Default::default(),
            triggering_event: None,
            event_value_amount: None,
            optional_costs_paid: Default::default(),
            source_snapshot: source_snapshot.clone(),
        });
    let provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::StateTrigger);
    let triggering_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::StateTriggerEvent::new(source),
        provenance,
    );
    game.defer_trigger_entries([crate::triggers::TriggeredAbilityEntry {
        source,
        controller,
        x_value: None,
        event_value_amount: None,
        ability: crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::custom(
                REFLEXIVE_TRIGGER_ID,
                "When you do".to_string(),
            ),
            effects: crate::resolution::ResolutionProgram::from_effects(effects),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        },
        triggering_event,
        source_stable_id,
        source_name,
        source_snapshot,
        tagged_objects,
        source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
        trigger_identity,
    }]);
}

/// Custom trigger id for queued reflexive triggered abilities.
pub(crate) const REFLEXIVE_TRIGGER_ID: &str = "reflexive_trigger";

/// Resolution context a queued reflexive triggered ability carries from the
/// resolution that triggered it.
#[derive(Debug, Clone)]
pub(crate) struct PendingReflexiveTrigger {
    pub trigger_identity: crate::triggers::TriggerIdentity,
    pub source: crate::ids::ObjectId,
    pub controller: crate::ids::PlayerId,
    pub effects: Vec<Effect>,
    pub choices: Vec<ChooseSpec>,
    pub tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    pub tagged_players: HashMap<TagKey, Vec<crate::ids::PlayerId>>,
    pub effect_outcomes: HashMap<EffectId, EffectOutcome>,
    pub targets: Vec<crate::effects::ResolvedTarget>,
    pub x_value: Option<u32>,
    pub iteration: crate::effects::context::IterationContext,
    pub combat: crate::effects::context::CombatExecutionContext,
    pub triggering_event: Option<crate::triggers::TriggerEvent>,
    pub event_value_amount: Option<i32>,
    pub optional_costs_paid: crate::cost::OptionalCostsPaid,
    pub source_snapshot: Option<ObjectSnapshot>,
}

/// Build the stack entry for a queued reflexive triggered ability as it is put
/// on the stack, choosing its targets now.
///
/// Returns `None` for an entry that isn't a reflexive trigger. `Some(None)`
/// means the ability is removed from the stack for lack of legal targets
/// (CR 603.3d) or is waiting on a target choice.
pub(crate) fn reflexive_trigger_stack_entry(
    game: &mut GameState,
    trigger: &crate::triggers::TriggeredAbilityEntry,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> Option<Option<StackEntry>> {
    let index = game
        .effect_store
        .pending_reflexive_triggers
        .iter()
        .position(|pending| pending.trigger_identity == trigger.trigger_identity)?;
    let pending = game.effect_store.pending_reflexive_triggers[index].clone();

    let mut ctx = ExecutionContext::new(pending.source, pending.controller, decision_maker);
    ctx.tagged_objects = pending.tagged_objects.clone();
    ctx.tagged_players = pending.tagged_players.clone();
    ctx.effect_outcomes = pending.effect_outcomes.clone();
    ctx.targets = pending.targets.clone();
    ctx.x_value = pending.x_value;
    ctx.iteration = pending.iteration;
    ctx.combat = pending.combat;
    ctx.triggering_event = pending.triggering_event.clone();
    ctx.event_value_amount = pending.event_value_amount;
    ctx.optional_costs_paid = pending.optional_costs_paid.clone();
    ctx.source_snapshot = pending.source_snapshot.clone();
    let selection = choose_reflexive_targets(game, &mut ctx, &pending.choices);
    if ctx.decision_maker.awaiting_choice() {
        return Some(None);
    }
    drop(ctx);
    let Some((targets, assignments)) = selection else {
        game.effect_store.pending_reflexive_triggers.remove(index);
        return Some(None);
    };

    // "Divided as you choose" / "distribute ... among" in the reflexive
    // ability is announced with its targets (CR 603.3d, 601.2d).
    let program_requirements = crate::game_loop::extract_target_requirements_with_modes(
        game,
        &pending.effects,
        pending.controller,
        Some(pending.source),
        None,
    );
    let distribution_requirements = program_requirements
        .iter()
        .any(|requirement| requirement.distribution_value.is_some())
        .then(|| {
            assignments
                .iter()
                .filter_map(|assignment| {
                    program_requirements
                        .iter()
                        .find(|requirement| {
                            requirement.distribution_value.is_some()
                                && requirement.spec == assignment.spec
                        })
                        .or_else(|| program_requirements.first())
                        .map(|requirement| {
                            let mut requirement = requirement.clone();
                            if requirement.spec != assignment.spec {
                                requirement.distribution_value = None;
                            }
                            requirement
                        })
                })
                .collect::<Vec<_>>()
        });
    let mut entry = StackEntry::ability(pending.source, pending.controller, pending.effects)
        .with_targets(targets)
        .with_target_assignments(assignments)
        .with_optional_costs_paid(pending.optional_costs_paid)
        .with_tagged_objects(pending.tagged_objects)
        .with_effect_outcomes(pending.effect_outcomes)
        .with_provenance(trigger.triggering_event.provenance())
        .with_trigger_identity(pending.trigger_identity);
    // References such as "that player" in the follow-up still refer to
    // the event that supplied the enclosing ability's context.
    entry.iteration = pending.iteration;
    entry.triggering_event = pending.triggering_event;
    entry.event_value_amount = pending.event_value_amount;

    if let Some(x) = pending.x_value {
        entry = entry.with_x(x);
    }
    if let Some(requirements) = distribution_requirements
        && requirements.len() == entry.target_assignments.len()
        && !crate::game_loop::announce_trigger_target_distributions(
            game,
            pending.source,
            pending.controller,
            pending.x_value,
            &mut entry,
            &requirements,
            decision_maker,
        )
    {
        // Awaiting the division: the reflexive trigger stays pending.
        return Some(None);
    }
    game.effect_store.pending_reflexive_triggers.remove(index);
    if let Some(defending_player) = pending.combat.defending_player {
        entry = entry.with_defending_player(defending_player);
    }
    if let Some(source) = game.object(pending.source) {
        entry = entry.with_source_info(source.stable_id, source.name.to_string());
    } else if let Some(snapshot) = pending.source_snapshot {
        entry = entry
            .with_source_info(snapshot.stable_id, snapshot.name.to_string())
            .with_source_snapshot(snapshot);
    }
    Some(Some(entry))
}

#[cfg(test)]
mod tests {
    #[cfg(ironsmith_runtime_parser_tests)]
    use super::ReflexiveTriggerEffect;
    use super::choose_reflexive_targets;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::cards::definitions::grizzly_bears;
    use crate::cards::definitions::lightning_bolt;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::TargetsContext;
    use crate::effect::{
        ChoiceCount, EffectId, EffectMetric, EffectMetricSource, EffectOutcome, Value,
    };
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::effect::{Effect, EffectPredicate, OutcomeObjectMemory};
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::effects::EffectExecutor;
    use crate::effects::ExecutionContext;
    use crate::game_state::{GameState, Target};
    use crate::ids::PlayerId;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::snapshot::ObjectSnapshot;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::tag::TagKey;
    use crate::target::ChooseSpec;
    use crate::zone::Zone;

    struct DuplicateTargetDecisionMaker {
        target: Target,
    }

    impl DecisionMaker for DuplicateTargetDecisionMaker {
        fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
            vec![self.target, self.target]
        }
    }

    #[derive(Default)]
    struct CaptureTargetBounds {
        min: Option<usize>,
        max: Option<Option<usize>>,
    }

    impl DecisionMaker for CaptureTargetBounds {
        fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
            self.min = ctx
                .requirements
                .first()
                .map(|requirement| requirement.min_targets);
            self.max = ctx
                .requirements
                .first()
                .map(|requirement| requirement.max_targets);
            Vec::new()
        }
    }

    #[test]
    fn reflexive_dynamic_target_count_resolves_from_the_antecedent_outcome() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source =
            game.create_object_from_definition(&lightning_bolt(), alice, Zone::Battlefield);
        let condition = EffectId(77);
        let mut dm = CaptureTargetBounds::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.store_outcome(condition, EffectOutcome::count(2));
        let choices = vec![ChooseSpec::target(ChooseSpec::creature()).with_count_value(
            ChoiceCount::up_to_dynamic_x(),
            Value::EffectMetric {
                effect_id: condition,
                source: EffectMetricSource::Outcome,
                metric: EffectMetric::Count,
            },
        )];

        let (selected, assignments) =
            choose_reflexive_targets(&game, &mut ctx, &choices).expect("up-to-zero is legal");

        assert!(selected.is_empty());
        assert_eq!(assignments[0].spec.count().max, Some(2));
        drop(ctx);
        assert_eq!(dm.min, Some(0));
        assert_eq!(dm.max, Some(Some(2)));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn reflexive_targets_are_normalized_per_requirement() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&lightning_bolt(), alice, Zone::Stack);
        let first = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let second = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);

        let mut dm = DuplicateTargetDecisionMaker {
            target: Target::Object(first),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let choices =
            vec![ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2))];

        let (selected, _) =
            choose_reflexive_targets(&game, &mut ctx, &choices).expect("valid targets");

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0], Target::Object(first));
        assert_eq!(selected[1], Target::Object(second));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn reflexive_trigger_pushes_stack_ability_with_captured_context() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source =
            game.create_object_from_definition(&lightning_bolt(), alice, Zone::Battlefield);
        let tagged = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let tagged_snapshot =
            ObjectSnapshot::from_object(game.object(tagged).expect("tagged object"), &game);

        let condition = EffectId(77);
        let reflexive = ReflexiveTriggerEffect::new(
            condition,
            EffectPredicate::Happened,
            vec![Effect::draw(1)],
            Vec::new(),
        );

        let mut dm = DuplicateTargetDecisionMaker {
            target: Target::Object(tagged),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.store_outcome(
            condition,
            EffectOutcome::count(1).with_affected_object_memory(vec![
                OutcomeObjectMemory::from_snapshot(&tagged_snapshot),
            ]),
        );
        ctx.x_value = Some(7);
        ctx.combat.defending_player = Some(bob);
        ctx.set_tagged_objects(TagKey::from("sacrificed"), vec![tagged_snapshot.clone()]);

        reflexive
            .execute(&mut game, &mut ctx)
            .expect("reflexive trigger should push a stack ability");

        drop(ctx);
        assert!(
            game.stack.is_empty(),
            "the scheduled reflexive ability awaits priority placement"
        );
        let mut queue = crate::triggers::TriggerQueue::new();
        crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        let entry = game.stack.last().expect("reflexive ability on stack");
        assert!(entry.is_ability);
        assert_eq!(entry.object_id, source);
        assert_eq!(entry.controller, alice);
        assert_eq!(entry.x_value, Some(7));
        assert_eq!(entry.defending_player, Some(bob));
        let captured = entry
            .tagged_objects
            .get(&TagKey::from("sacrificed"))
            .expect("tagged object snapshots carried to stack entry");
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].object_id, tagged_snapshot.object_id);
        let it = entry
            .tagged_objects
            .get(&TagKey::from("__it__"))
            .expect("condition object memory exposed as reflexive it tag");
        assert_eq!(it.len(), 1);
        assert_eq!(it[0].object_id, tagged_snapshot.object_id);
        assert_eq!(
            entry
                .effect_outcomes
                .get(&condition)
                .and_then(EffectOutcome::as_count),
            Some(1),
            "the reflexive ability must retain parent outcomes used by its child effects"
        );
        assert_eq!(
            entry.source_name.as_deref(),
            Some(game.object(source).expect("source object").name.as_str())
        );
        assert!(entry.source_snapshot.is_some());
    }
}

#[cfg(test)]
mod pending_reflexive_context_contract_tests {
    use super::*;
    use crate::{PlayerId, Zone};
    #[test]
    fn antecedent_context_is_retained_until_priority_placement_and_resolution() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let card = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Pending trigger source",
        )
        .card_types(vec![crate::types::CardType::Creature])
        .build();
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let condition = EffectId(77);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.x_value = Some(7);
        ctx.set_tagged_objects("paid", vec![snapshot.clone()]);
        ctx.store_outcome(
            condition,
            EffectOutcome::count(1).with_affected_object_memory(vec![
                crate::effect::OutcomeObjectMemory::from_snapshot(&snapshot),
            ]),
        );
        let effect = ReflexiveTriggerEffect::new(
            condition,
            crate::effect::EffectPredicate::Happened,
            vec![Effect::gain_life(2)],
            Vec::new(),
        );
        assert_eq!(
            effect.execute(&mut game, &mut ctx).unwrap().count_or_zero(),
            1
        );
        drop(ctx);
        assert!(game.stack.is_empty());
        assert_eq!(game.player(alice).unwrap().life, 20);
        let mut queue = crate::triggers::TriggerQueue::new();
        crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        let entry = &game.stack[0];
        assert_eq!(entry.x_value, Some(7));
        assert_eq!(entry.controller, alice);
        assert_eq!(
            entry
                .effect_outcomes
                .get(&condition)
                .unwrap()
                .count_or_zero(),
            1
        );
        assert_eq!(
            entry
                .tagged_objects
                .get(&crate::tag::TagKey::from("paid"))
                .unwrap()[0]
                .object_id,
            source
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 22);
        assert!(game.stack.is_empty());
    }
}
