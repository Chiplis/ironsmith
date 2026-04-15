//! "Choose new targets" effect implementation.
//!
//! This effect supports text like "You may choose new targets for the copy."
//! by re-targeting stack objects produced by a prior effect.

use crate::decisions::context::{BooleanContext, TargetRequirementContext, TargetsContext};
use crate::effect::{ChoiceCount, EffectId, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::events::spells::BecomesTargetedEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry, Target};
use crate::target::{ChooseSpec, PlayerFilter};
use crate::targeting::{compute_legal_targets, normalize_targets_for_requirements};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Effect that lets a player choose new targets for stack object(s).
///
/// The objects are read from a prior effect outcome, preferring explicit
/// object outputs and falling back to preserved chosen/affected-object facts.
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNewTargetsEffect {
    /// Effect result ID that contains stack object IDs to retarget.
    pub from_effect: EffectId,
    /// Whether this is optional ("you may choose new targets").
    pub may: bool,
    /// Optional explicit chooser for "that player may choose new targets".
    pub chooser: Option<PlayerFilter>,
}

impl ChooseNewTargetsEffect {
    /// Create a new retargeting effect.
    pub fn new(from_effect: EffectId, may: bool) -> Self {
        Self {
            from_effect,
            may,
            chooser: None,
        }
    }

    /// Create a new retargeting effect with explicit chooser.
    pub fn new_for_player(from_effect: EffectId, may: bool, chooser: PlayerFilter) -> Self {
        Self {
            from_effect,
            may,
            chooser: Some(chooser),
        }
    }

    /// Optional retargeting ("you may choose new targets").
    pub fn may(from_effect: EffectId) -> Self {
        Self::new(from_effect, true)
    }

    /// Optional retargeting where a specific player chooses.
    pub fn may_for_player(from_effect: EffectId, chooser: PlayerFilter) -> Self {
        Self::new_for_player(from_effect, true, chooser)
    }

    /// Mandatory retargeting ("choose new targets").
    pub fn must(from_effect: EffectId) -> Self {
        Self::new(from_effect, false)
    }
}

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

impl EffectExecutor for ChooseNewTargetsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_ids = match ctx.get_outcome(self.from_effect) {
            Some(outcome) => outcome.output_objects().to_vec(),
            None => return Ok(EffectOutcome::resolved()),
        };
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
                .is_none_or(|obj| obj.zone != Zone::Stack)
            {
                continue;
            }

            let entry = game.stack[stack_idx].clone();
            let Some(requirements) = extract_requirements(game, &entry) else {
                if self.may {
                    continue;
                }
                return Ok(EffectOutcome::target_invalid());
            };

            if requirements.is_empty() {
                continue;
            }

            let chooser = if let Some(filter) = &self.chooser {
                resolve_player_filter(game, filter, ctx)?
            } else {
                entry.controller
            };

            if self.may {
                let source_name = game
                    .object(object_id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "copy".to_string());
                let choose = ctx.decision_maker.decide_boolean(
                    game,
                    &BooleanContext::new(
                        chooser,
                        Some(object_id),
                        format!("Choose new targets for {source_name}?"),
                    ),
                );
                if !choose {
                    continue;
                }
            }

            let targets_ctx =
                TargetsContext::new(chooser, object_id, "copy".to_string(), requirements.clone());
            let proposed = ctx.decision_maker.decide_targets(game, &targets_ctx);
            let Some(new_targets) = normalize_targets_for_requirements(&requirements, proposed)
            else {
                if self.may {
                    continue;
                }
                return Ok(EffectOutcome::target_invalid());
            };

            if game.stack[stack_idx].targets != new_targets {
                game.stack[stack_idx].targets = new_targets;
                changed += 1;
                for target in &game.stack[stack_idx].targets {
                    if let Target::Object(target_id) = target {
                        events.push(TriggerEvent::new_with_provenance(
                            BecomesTargetedEvent::new(
                                *target_id,
                                object_id,
                                entry.controller,
                                entry.is_ability,
                            ),
                            ctx.provenance,
                        ));
                    }
                }
            }
        }

        Ok(EffectOutcome::count(changed).with_events(events))
    }
}
