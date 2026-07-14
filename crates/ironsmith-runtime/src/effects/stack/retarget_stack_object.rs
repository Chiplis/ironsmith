//! Retarget stack object effect implementation.
//!
//! Supports text like "Change the target of target spell" and
//! "Choose new targets for target spell or ability".

use crate::decisions::context::{
    SelectObjectsContext, SelectOptionsContext, SelectableObject, SelectableOption,
    TargetRequirementContext, TargetsContext,
};
use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_objects_from_spec, resolve_player_filter, resolve_players_from_spec,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::spells::BecomesTargetedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::game_state::{GameState, StackEntry, Target};
use crate::ids::PlayerId;
use crate::target::ChooseSpec;
use crate::targeting::{
    assigned_target_ranges, compute_legal_targets, normalize_targets_for_requirements,
};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;
pub use ironsmith_core::{NewTargetRestriction, RetargetMode, RetargetStackObjectEffect};

fn requires_target_selection(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) => requires_target_selection(inner),
        ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::Player(_)
        | ChooseSpec::Object(_) => true,
        _ => false,
    }
}

fn effects_for_stack_entry(game: &GameState, entry: &StackEntry) -> Vec<crate::effect::Effect> {
    if let Some(ref effects) = entry.ability_effects {
        return effects.to_vec();
    }

    let Some(obj) = game.object(entry.object_id) else {
        return Vec::new();
    };

    if let Some(ref effects) = obj.spell_effect {
        return effects.to_vec();
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
        let legal_target_sets =
            crate::targeting::legal_target_sets_for_spec(game, spec, &legal_targets);
        let has_enough = crate::targeting::has_enough_legal_targets_for_spec(
            game,
            spec,
            &legal_targets,
            count.min,
        );
        if !has_enough {
            return None;
        }

        requirements.push(TargetRequirementContext {
            description: effect.0.target_description().to_string(),
            legal_targets,
            legal_target_sets,
            min_targets: count.min,
            max_targets: count.max,
            distinct_player_group: None,
        });
    }

    Some(requirements)
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

fn push_becomes_targeted_event(
    events: &mut Vec<TriggerEvent>,
    target: Target,
    source: crate::ids::ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
    provenance: crate::provenance::ProvNodeId,
) {
    events.push(TriggerEvent::new_with_provenance(
        BecomesTargetedEvent::new_target(target, source, source_controller, by_ability),
        provenance,
    ));
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
            let filter_ctx = ctx.filter_context(game);
            let zone = filter.zone.unwrap_or(Zone::Stack);
            let mut candidates: Vec<SelectableObject> = game
                .zone_ids(zone)
                .filter_map(|id| game.object(id).map(|obj| (id, obj)))
                .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                .map(|(id, obj)| SelectableObject::new(id, obj.name.to_string()))
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
            if ctx.decision_maker.awaiting_choice() {
                return Ok(Vec::new());
            }
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
                    let Some(slices) = assigned_target_ranges(&adjusted, &entry.targets) else {
                        continue;
                    };

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
                        .map(|o| o.name.to_string())
                        .unwrap_or_else(|| "spell".to_string());

                    let targets_ctx =
                        TargetsContext::new(chooser_id, object_id, source_name, adjusted.clone());
                    let proposed = ctx.decision_maker.decide_targets(game, &targets_ctx);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    let Some(new_targets) = normalize_targets_for_requirements(&adjusted, proposed)
                    else {
                        continue;
                    };

                    if game.stack[stack_idx].targets != new_targets {
                        let old_targets = game.stack[stack_idx].targets.clone();
                        game.stack[stack_idx].targets = new_targets;
                        changed += 1;
                        for target in &game.stack[stack_idx].targets {
                            if !old_targets.contains(target) {
                                push_becomes_targeted_event(
                                    &mut events,
                                    *target,
                                    object_id,
                                    entry.controller,
                                    entry.is_ability,
                                    ctx.provenance,
                                );
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
                    let Some(slices) = assigned_target_ranges(&requirements, &entry.targets) else {
                        continue;
                    };
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
                        if ctx.decision_maker.awaiting_choice() {
                            continue;
                        }
                        let Some(idx) = choice.first().copied() else {
                            continue;
                        };
                        let Some(selected) = eligible_indices.get(idx).copied() else {
                            continue;
                        };
                        selected
                    };

                    if let Some(entry_target) = game.stack[stack_idx].targets.get_mut(chosen_idx) {
                        if *entry_target != fixed_target {
                            *entry_target = fixed_target;
                            changed += 1;
                            push_becomes_targeted_event(
                                &mut events,
                                fixed_target,
                                object_id,
                                entry.controller,
                                entry.is_ability,
                                ctx.provenance,
                            );
                        }
                    }
                }
            }
        }

        Ok(EffectOutcome::count(changed).with_events(events))
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::card::CardBuilder;
    use crate::effect::Effect;
    use crate::ids::{CardId, PlayerId};
    use crate::resolution::ResolutionProgram;
    use crate::types::CardType;

    #[test]
    fn retarget_to_player_emits_becomes_targeted_event_for_new_player() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell = CardBuilder::new(CardId::new(), "Retarget Test Bolt")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
        game.object_mut(spell_id)
            .expect("spell object exists")
            .spell_effect = Some(
            ResolutionProgram::from_effects(vec![Effect::deal_damage(
                1,
                ChooseSpec::target_player(),
            )])
            .into(),
        );
        game.push_to_stack(
            StackEntry::new(spell_id, alice).with_targets(vec![Target::Player(alice)]),
        );
        let retarget_source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(retarget_source, bob);
        let retarget = RetargetStackObjectEffect::new(ChooseSpec::SpecificObject(spell_id))
            .with_mode(RetargetMode::OneToFixed(ChooseSpec::SpecificPlayer(bob)))
            .with_chooser(crate::target::PlayerFilter::You);

        let outcome = retarget
            .execute(&mut game, &mut ctx)
            .expect("retarget should resolve");

        assert_eq!(game.stack[0].targets, vec![Target::Player(bob)]);
        assert!(outcome.events.iter().any(|event| {
            event
                .downcast::<BecomesTargetedEvent>()
                .is_some_and(|becomes_targeted| {
                    becomes_targeted.target_player() == Some(bob)
                        && becomes_targeted.source == spell_id
                        && becomes_targeted.source_controller == alice
                })
        }));
    }
}
