//! Retarget stack object effect implementation.
//!
//! Supports text like "Change the target of target spell" and
//! "Choose new targets for target spell or ability".

use crate::decisions::context::{
    SelectObjectsContext, SelectOptionsContext, SelectableObject, SelectableOption,
    TargetRequirementContext, TargetsContext,
};
use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_objects_from_spec, resolve_player_filter, resolve_players_from_spec,
};
use crate::events::spells::BecomesTargetedEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry, Target};
use crate::ids::PlayerId;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::targeting::compute_legal_targets;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub enum NewTargetRestriction {
    Player(PlayerFilter),
    Object(ObjectFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RetargetMode {
    /// Choose new targets for all targets of the spell/ability.
    All,
    /// Change a single target to a fixed target.
    OneToFixed(ChooseSpec),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetargetStackObjectEffect {
    pub target: ChooseSpec,
    pub mode: RetargetMode,
    pub chooser: PlayerFilter,
    pub require_change: bool,
    pub new_target_restriction: Option<NewTargetRestriction>,
}

impl RetargetStackObjectEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            mode: RetargetMode::All,
            chooser: PlayerFilter::You,
            require_change: false,
            new_target_restriction: None,
        }
    }

    pub fn with_mode(mut self, mode: RetargetMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }

    pub fn require_change(mut self) -> Self {
        self.require_change = true;
        self
    }

    pub fn with_restriction(mut self, restriction: NewTargetRestriction) -> Self {
        self.new_target_restriction = Some(restriction);
        self
    }
}

fn requires_target_selection(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) => requires_target_selection(inner),
        ChooseSpec::AnyTarget | ChooseSpec::Player(_) | ChooseSpec::Object(_) => true,
        _ => false,
    }
}

fn effects_for_stack_entry(game: &GameState, entry: &StackEntry) -> Vec<crate::effect::Effect> {
    if let Some(ref effects) = entry.ability_effects {
        return effects.clone();
    }

    let Some(obj) = game.object(entry.object_id) else {
        return Vec::new();
    };

    if let Some(ref effects) = obj.spell_effect {
        return effects.clone();
    }

    Vec::new()
}

fn extract_requirements(
    game: &GameState,
    entry: &StackEntry,
) -> Option<Vec<TargetRequirementContext>> {
    let effects = effects_for_stack_entry(game, entry);
    let mut requirements = Vec::new();

    for effect in &effects {
        let Some(spec) = effect.0.get_target_spec() else {
            continue;
        };
        if !requires_target_selection(spec) {
            continue;
        }

        let count: ChoiceCount = effect.0.get_target_count().unwrap_or_default();
        let legal_targets =
            compute_legal_targets(game, spec, entry.controller, Some(entry.object_id));
        let has_enough = count.min == 0 || legal_targets.len() >= count.min;
        if !has_enough {
            return None;
        }

        requirements.push(TargetRequirementContext {
            description: effect.0.target_description().to_string(),
            legal_targets,
            min_targets: count.min,
            max_targets: count.max,
        });
    }

    Some(requirements)
}

fn normalize_target_choice(
    requirements: &[TargetRequirementContext],
    proposed: Vec<Target>,
) -> Option<Vec<Target>> {
    let mut out = Vec::new();
    let mut cursor = 0usize;

    for req in requirements {
        let mut selected = Vec::new();
        let max_for_req = req.max_targets.unwrap_or(req.min_targets.max(1));
        let end = (cursor + max_for_req).min(proposed.len());

        for target in &proposed[cursor..end] {
            if req.legal_targets.contains(target) {
                selected.push(*target);
            }
        }
        cursor = end;

        if selected.len() < req.min_targets {
            for legal in &req.legal_targets {
                if selected.len() >= req.min_targets {
                    break;
                }
                if !selected.contains(legal) {
                    selected.push(*legal);
                }
            }
        }

        if selected.len() < req.min_targets {
            return None;
        }

        out.extend(selected);
    }

    Some(out)
}

fn target_slices_for_requirements(
    requirements: &[TargetRequirementContext],
    total_targets: usize,
) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut cursor = 0usize;
    for req in requirements {
        let max_for_req = req.max_targets.unwrap_or(req.min_targets.max(1));
        let end = (cursor + max_for_req).min(total_targets);
        ranges.push(cursor..end);
        cursor = end;
    }
    ranges
}

fn filter_targets_with_restriction(
    targets: Vec<Target>,
    restriction: Option<&NewTargetRestriction>,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Vec<Target> {
    let Some(restriction) = restriction else {
        return targets;
    };

    let filter_ctx = ctx.filter_context(game);
    targets
        .into_iter()
        .filter(|target| match (restriction, target) {
            (NewTargetRestriction::Player(player_filter), Target::Player(pid)) => {
                player_filter.matches_player(*pid, &filter_ctx)
            }
            (NewTargetRestriction::Object(object_filter), Target::Object(obj_id)) => game
                .object(*obj_id)
                .is_some_and(|obj| object_filter.matches(obj, &filter_ctx, game)),
            _ => false,
        })
        .collect()
}

fn resolve_fixed_target(
    game: &GameState,
    ctx: &ExecutionContext,
    spec: &ChooseSpec,
) -> Result<Target, ExecutionError> {
    if let Ok(objects) = resolve_objects_from_spec(game, spec, ctx) {
        if let Some(id) = objects.first() {
            return Ok(Target::Object(*id));
        }
    }

    if let Ok(players) = resolve_players_from_spec(game, spec, ctx) {
        if let Some(id) = players.first() {
            return Ok(Target::Player(*id));
        }
    }

    Err(ExecutionError::InvalidTarget)
}

fn resolve_retarget_objects(
    game: &GameState,
    ctx: &mut ExecutionContext,
    chooser: PlayerId,
    spec: &ChooseSpec,
) -> Result<Vec<crate::ids::ObjectId>, ExecutionError> {
    if spec.is_target() {
        return resolve_objects_from_spec(game, spec, ctx);
    }

    match spec.base() {
        ChooseSpec::Object(filter) => {
            let count = spec.count();
            let candidate_ids: Vec<crate::ids::ObjectId> = match filter.zone {
                Some(Zone::Stack) | None => game.stack.iter().map(|e| e.object_id).collect(),
                Some(Zone::Battlefield) => game.battlefield.clone(),
                Some(Zone::Graveyard) => game
                    .players
                    .iter()
                    .flat_map(|p| p.graveyard.iter().copied())
                    .collect(),
                Some(Zone::Exile) => game.exile.clone(),
                Some(Zone::Command) => game.command_zone.clone(),
                Some(Zone::Library) => game
                    .players
                    .iter()
                    .flat_map(|p| p.library.iter().copied())
                    .collect(),
                Some(Zone::Hand) => game
                    .players
                    .iter()
                    .flat_map(|p| p.hand.iter().copied())
                    .collect(),
            };

            let filter_ctx = ctx.filter_context(game);
            let mut candidates: Vec<SelectableObject> = candidate_ids
                .iter()
                .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                .map(|(id, obj)| SelectableObject::new(id, obj.name.clone()))
                .collect();

            if candidates.is_empty() {
                return Ok(Vec::new());
            }

            let min = count.min;
            let max = count.max;
            if min == 0 && max == Some(0) {
                return Ok(Vec::new());
            }

            let description = format!("Choose {} to retarget", filter.description());
            let select_ctx = SelectObjectsContext::new(
                chooser,
                Some(ctx.source),
                description,
                candidates.drain(..).collect(),
                min,
                max,
            );
            let chosen = ctx
                .decision_maker
                .decide_objects(game, &select_ctx)
                .into_iter()
                .collect();
            Ok(chosen)
        }
        ChooseSpec::Tagged(_) | ChooseSpec::SpecificObject(_) | ChooseSpec::Source => {
            resolve_objects_from_spec(game, spec, ctx)
        }
        _ => resolve_objects_from_spec(game, spec, ctx),
    }
}

impl EffectExecutor for RetargetStackObjectEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser_id = resolve_player_filter(game, &self.chooser, ctx)?;
        let object_ids = resolve_retarget_objects(game, ctx, chooser_id, &self.target)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let mut changed = 0;
        let mut events = Vec::new();

        for object_id in object_ids {
            let Some(stack_idx) = game.stack.iter().position(|e| e.object_id == object_id) else {
                continue;
            };

            if game
                .object(object_id)
                .is_none_or(|obj| obj.zone != Zone::Stack && !game.stack[stack_idx].is_ability)
            {
                continue;
            }

            let entry = game.stack[stack_idx].clone();
            let Some(requirements) = extract_requirements(game, &entry) else {
                continue;
            };

            if requirements.is_empty() {
                continue;
            }

            match &self.mode {
                RetargetMode::All => {
                    let mut adjusted = requirements.clone();
                    let slices = target_slices_for_requirements(&adjusted, entry.targets.len());

                    for (req, range) in adjusted.iter_mut().zip(slices.iter()) {
                        let existing_targets = entry.targets.get(range.clone()).unwrap_or(&[]);
                        let mut legal = req.legal_targets.clone();
                        legal = filter_targets_with_restriction(
                            legal,
                            self.new_target_restriction.as_ref(),
                            game,
                            ctx,
                        );

                        if self.require_change {
                            let filtered: Vec<Target> = legal
                                .iter()
                                .copied()
                                .filter(|t| !existing_targets.contains(t))
                                .collect();
                            if filtered.len() >= req.min_targets {
                                legal = filtered;
                            }
                        }

                        if legal.len() < req.min_targets {
                            legal.clear();
                        }

                        req.legal_targets = legal;
                    }

                    if adjusted
                        .iter()
                        .any(|req| req.min_targets > 0 && req.legal_targets.is_empty())
                    {
                        continue;
                    }

                    if adjusted.iter().all(|req| req.legal_targets.is_empty()) {
                        continue;
                    }

                    let source_name = game
                        .object(object_id)
                        .map(|o| o.name.clone())
                        .unwrap_or_else(|| "spell".to_string());

                    let targets_ctx =
                        TargetsContext::new(chooser_id, object_id, source_name, adjusted.clone());
                    let proposed = ctx.decision_maker.decide_targets(game, &targets_ctx);
                    let Some(new_targets) = normalize_target_choice(&adjusted, proposed) else {
                        continue;
                    };

                    if game.stack[stack_idx].targets != new_targets {
                        game.stack[stack_idx].targets = new_targets;
                        changed += 1;
                        for target in &game.stack[stack_idx].targets {
                            if let Target::Object(target_id) = target {
                                events.push(TriggerEvent::new(BecomesTargetedEvent::new(
                                    *target_id,
                                    object_id,
                                    entry.controller,
                                    entry.is_ability,
                                )));
                            }
                        }
                    }
                }
                RetargetMode::OneToFixed(spec) => {
                    let fixed_target = match resolve_fixed_target(game, ctx, spec) {
                        Ok(target) => target,
                        Err(_) => continue,
                    };

                    if let Some(restriction) = &self.new_target_restriction {
                        let filtered = filter_targets_with_restriction(
                            vec![fixed_target],
                            Some(restriction),
                            game,
                            ctx,
                        );
                        if filtered.is_empty() {
                            continue;
                        }
                    }

                    let mut eligible_indices = Vec::new();
                    let slices = target_slices_for_requirements(&requirements, entry.targets.len());
                    for (req, range) in requirements.iter().zip(slices.iter()) {
                        let legal = filter_targets_with_restriction(
                            req.legal_targets.clone(),
                            self.new_target_restriction.as_ref(),
                            game,
                            ctx,
                        );
                        if !legal.contains(&fixed_target) {
                            continue;
                        }
                        for idx in range.clone() {
                            if entry.targets.get(idx).is_some_and(|t| *t == fixed_target) {
                                continue;
                            }
                            eligible_indices.push(idx);
                        }
                    }

                    if eligible_indices.is_empty() {
                        continue;
                    }

                    let chosen_idx = if eligible_indices.len() == 1 {
                        eligible_indices[0]
                    } else {
                        let mut options = Vec::new();
                        for (opt_idx, target_idx) in eligible_indices.iter().enumerate() {
                            let target = entry.targets.get(*target_idx).copied();
                            let description = match target {
                                Some(Target::Player(pid)) => game
                                    .player(pid)
                                    .map(|p| format!("target player {}", p.name))
                                    .unwrap_or_else(|| "target player".to_string()),
                                Some(Target::Object(obj_id)) => game
                                    .object(obj_id)
                                    .map(|o| format!("target {}", o.name))
                                    .unwrap_or_else(|| "target object".to_string()),
                                _ => "target".to_string(),
                            };
                            options.push(SelectableOption::new(opt_idx, description));
                        }
                        let select_ctx = SelectOptionsContext::new(
                            chooser_id,
                            Some(ctx.source),
                            "Choose target to change",
                            options,
                            1,
                            1,
                        );
                        let choice = ctx.decision_maker.decide_options(game, &select_ctx);
                        let idx = choice.first().copied().unwrap_or(0);
                        let selected = eligible_indices
                            .get(idx)
                            .copied()
                            .unwrap_or_else(|| eligible_indices[0]);
                        selected
                    };

                    if let Some(entry_target) = game.stack[stack_idx].targets.get_mut(chosen_idx) {
                        if *entry_target != fixed_target {
                            *entry_target = fixed_target;
                            changed += 1;
                            if let Target::Object(target_id) = fixed_target {
                                events.push(TriggerEvent::new(BecomesTargetedEvent::new(
                                    target_id,
                                    object_id,
                                    entry.controller,
                                    entry.is_ability,
                                )));
                            }
                        }
                    }
                }
            }
        }

        Ok(EffectOutcome::from_result(EffectResult::Count(changed)).with_events(events))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        if self.target.is_target() {
            Some(self.target.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "spell or ability to retarget"
    }
}
