//! Apply continuous effect implementation.

use crate::continuous::{ContinuousEffect, EffectSourceType, EffectTarget, Modification};
use crate::effect::{ChoiceCount, EffectOutcome, EffectResult, Until, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_objects_from_spec, resolve_player_filter, resolve_value, validate_target,
};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;

/// Runtime-resolved continuous modification templates.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeModification {
    /// Change controller to the executing effect's controller.
    ChangeControllerToEffectController,
    /// Change controller to a resolved player filter.
    ChangeControllerToPlayer(crate::target::PlayerFilter),
    /// Resolve power/toughness deltas at execution, then apply layer 7c modification.
    ModifyPowerToughness { power: Value, toughness: Value },
    /// Resolve power delta at execution, then apply layer 7c modification.
    ModifyPower { value: Value },
    /// Resolve toughness delta at execution, then apply layer 7c modification.
    ModifyToughness { value: Value },
}

/// Effect that registers a continuous effect with the game state.
///
/// This is a low-level primitive used by other effects to compose
/// continuous effects without duplicating registration logic.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyContinuousEffect {
    /// Which objects the continuous effect applies to.
    pub target: EffectTarget,
    /// Optional ChooseSpec that will be resolved at execution time.
    /// When present, this takes precedence over `target`.
    pub target_spec: Option<ChooseSpec>,
    /// The modification to apply.
    pub modification: Option<Modification>,
    /// Additional modifications that share target/duration/source metadata.
    pub additional_modifications: Vec<Modification>,
    /// Runtime-resolved modifications that are materialized at execution.
    pub runtime_modifications: Vec<RuntimeModification>,
    /// How long the effect lasts.
    pub until: Until,
    /// Optional source type (e.g., resolution lock).
    pub source_type: Option<EffectSourceType>,
    /// For filter targets created by resolving spells/abilities, lock matching
    /// battlefield objects at resolution time (Rule 611.2c).
    pub lock_filter_at_resolution: bool,
    /// Resolve set-P/T Value expressions at resolution and store fixed values.
    pub resolve_set_pt_values_at_resolution: bool,
    /// Require resolved object targets to currently be creatures.
    pub require_creature_target: bool,
}

impl ApplyContinuousEffect {
    /// Create a new apply continuous effect.
    pub fn new(target: EffectTarget, modification: Modification, until: Until) -> Self {
        Self {
            target,
            target_spec: None,
            modification: Some(modification),
            additional_modifications: Vec::new(),
            runtime_modifications: Vec::new(),
            until,
            source_type: None,
            lock_filter_at_resolution: false,
            resolve_set_pt_values_at_resolution: false,
            require_creature_target: false,
        }
    }

    /// Create a new apply continuous effect that resolves a ChooseSpec at execution.
    pub fn with_spec(spec: ChooseSpec, modification: Modification, until: Until) -> Self {
        Self {
            target: EffectTarget::AllPermanents,
            target_spec: Some(spec),
            modification: Some(modification),
            additional_modifications: Vec::new(),
            runtime_modifications: Vec::new(),
            until,
            source_type: None,
            lock_filter_at_resolution: false,
            resolve_set_pt_values_at_resolution: false,
            require_creature_target: false,
        }
    }

    /// Create an effect with a runtime-resolved modification.
    pub fn with_spec_runtime(
        spec: ChooseSpec,
        runtime_modification: RuntimeModification,
        until: Until,
    ) -> Self {
        Self {
            target: EffectTarget::AllPermanents,
            target_spec: Some(spec),
            modification: None,
            additional_modifications: Vec::new(),
            runtime_modifications: vec![runtime_modification],
            until,
            source_type: None,
            lock_filter_at_resolution: false,
            resolve_set_pt_values_at_resolution: false,
            require_creature_target: false,
        }
    }

    /// Create an effect with a runtime-resolved modification for an explicit target.
    pub fn new_runtime(
        target: EffectTarget,
        runtime_modification: RuntimeModification,
        until: Until,
    ) -> Self {
        Self {
            target,
            target_spec: None,
            modification: None,
            additional_modifications: Vec::new(),
            runtime_modifications: vec![runtime_modification],
            until,
            source_type: None,
            lock_filter_at_resolution: false,
            resolve_set_pt_values_at_resolution: false,
            require_creature_target: false,
        }
    }

    /// Add another modification sharing the same metadata.
    pub fn with_additional_modification(mut self, modification: Modification) -> Self {
        self.additional_modifications.push(modification);
        self
    }

    /// Add another runtime modification sharing the same metadata.
    pub fn with_additional_runtime_modification(
        mut self,
        modification: RuntimeModification,
    ) -> Self {
        self.runtime_modifications.push(modification);
        self
    }

    /// Set the source type for the continuous effect.
    pub fn with_source_type(mut self, source_type: EffectSourceType) -> Self {
        self.source_type = Some(source_type);
        self
    }

    /// Lock filtered targets at resolution time.
    pub fn lock_filter_at_resolution(mut self) -> Self {
        self.lock_filter_at_resolution = true;
        self
    }

    /// Resolve set-P/T Value expressions to fixed values at resolution.
    pub fn resolve_set_pt_values_at_resolution(mut self) -> Self {
        self.resolve_set_pt_values_at_resolution = true;
        self
    }

    /// Require object targets to currently be creatures.
    pub fn require_creature_target(mut self) -> Self {
        self.require_creature_target = true;
        self
    }

    fn resolve_target(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<(EffectTarget, Option<Vec<ObjectId>>, bool), ExecutionError> {
        let Some(spec) = &self.target_spec else {
            return Ok((self.target.clone(), None, false));
        };

        let mut objects = resolve_objects_from_spec(game, spec, ctx)?;
        if spec.is_target() {
            objects.retain(|id| validate_target(game, &ResolvedTarget::Object(*id), spec, ctx));
        }
        if objects.is_empty() {
            if spec.is_target() {
                return Ok((EffectTarget::AllPermanents, Some(Vec::new()), true));
            }
            if !matches!(spec.base(), ChooseSpec::All(_)) {
                return Err(ExecutionError::InvalidTarget);
            }
            return Ok((EffectTarget::AllPermanents, Some(Vec::new()), false));
        }

        if objects.len() == 1 {
            return Ok((EffectTarget::Specific(objects[0]), None, false));
        }

        Ok((EffectTarget::AllPermanents, Some(objects), false))
    }

    fn lock_targets_for_filter(
        filter: &crate::target::ObjectFilter,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Vec<ObjectId> {
        let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
        game.battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.zone == Zone::Battlefield)
            .filter(|obj| filter.matches(obj, &filter_ctx, game))
            .map(|obj| obj.id)
            .collect()
    }

    fn resolve_set_pt_modification(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
        modification: &Modification,
    ) -> Result<Modification, ExecutionError> {
        if !self.resolve_set_pt_values_at_resolution {
            return Ok(modification.clone());
        }

        match modification {
            Modification::SetPower { value, sublayer } => Ok(Modification::SetPower {
                value: Value::Fixed(resolve_value(game, value, ctx)?),
                sublayer: *sublayer,
            }),
            Modification::SetToughness { value, sublayer } => Ok(Modification::SetToughness {
                value: Value::Fixed(resolve_value(game, value, ctx)?),
                sublayer: *sublayer,
            }),
            Modification::SetPowerToughness {
                power,
                toughness,
                sublayer,
            } => Ok(Modification::SetPowerToughness {
                power: Value::Fixed(resolve_value(game, power, ctx)?),
                toughness: Value::Fixed(resolve_value(game, toughness, ctx)?),
                sublayer: *sublayer,
            }),
            _ => Ok(modification.clone()),
        }
    }

    fn resolve_runtime_modification(
        game: &GameState,
        ctx: &ExecutionContext,
        modification: &RuntimeModification,
    ) -> Result<Modification, ExecutionError> {
        match modification {
            RuntimeModification::ChangeControllerToEffectController => {
                Ok(Modification::ChangeController(ctx.controller))
            }
            RuntimeModification::ChangeControllerToPlayer(player) => Ok(
                Modification::ChangeController(resolve_player_filter(game, player, ctx)?),
            ),
            RuntimeModification::ModifyPowerToughness { power, toughness } => {
                Ok(Modification::ModifyPowerToughness {
                    power: resolve_value(game, power, ctx)?,
                    toughness: resolve_value(game, toughness, ctx)?,
                })
            }
            RuntimeModification::ModifyPower { value } => {
                Ok(Modification::ModifyPower(resolve_value(game, value, ctx)?))
            }
            RuntimeModification::ModifyToughness { value } => Ok(Modification::ModifyToughness(
                resolve_value(game, value, ctx)?,
            )),
        }
    }

    fn target_object_ids(
        target: &EffectTarget,
        source_type: &Option<EffectSourceType>,
    ) -> Vec<ObjectId> {
        if let Some(EffectSourceType::Resolution { locked_targets }) = source_type {
            return locked_targets.clone();
        }
        match target {
            EffectTarget::Specific(id) => vec![*id],
            _ => Vec::new(),
        }
    }
}

impl EffectExecutor for ApplyContinuousEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (target, spec_locked_targets, target_invalid) = self.resolve_target(game, ctx)?;
        if target_invalid {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }
        let mut source_type = self.source_type.clone();

        let filter_locked_targets = if self.lock_filter_at_resolution {
            if let EffectTarget::Filter(filter) = &target {
                Some(Self::lock_targets_for_filter(filter, game, ctx))
            } else {
                None
            }
        } else {
            None
        };

        let locked_targets = filter_locked_targets.or(spec_locked_targets);
        if let Some(locked_targets) = locked_targets {
            source_type = Some(EffectSourceType::Resolution { locked_targets });
        }

        let mut mods = Vec::with_capacity(
            self.additional_modifications.len() + self.runtime_modifications.len() + 1,
        );
        if let Some(modification) = &self.modification {
            mods.push(modification.clone());
        }
        mods.extend(self.additional_modifications.iter().cloned());
        for runtime_modification in &self.runtime_modifications {
            mods.push(Self::resolve_runtime_modification(
                game,
                ctx,
                runtime_modification,
            )?);
        }

        if self.require_creature_target {
            for id in Self::target_object_ids(&target, &source_type) {
                let Some(obj) = game.object(id) else {
                    return Err(ExecutionError::ObjectNotFound(id));
                };
                if !obj.has_card_type(CardType::Creature) {
                    return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
                }
            }
        }

        for modification in mods {
            let resolved_modification =
                self.resolve_set_pt_modification(game, ctx, &modification)?;
            let mut effect = ContinuousEffect::new(
                ctx.source,
                ctx.controller,
                target.clone(),
                resolved_modification,
            )
            .until(self.until.clone());

            if let Some(source_type) = &source_type {
                effect = effect.with_source_type(source_type.clone());
            }

            game.continuous_effects.add_effect(effect);
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target_spec.as_ref()
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        self.target_spec.as_ref().map(ChooseSpec::count)
    }

    fn target_description(&self) -> &'static str {
        "target"
    }
}
