//! Explicit mechanic effects used by parser/rendering for supported wording.
//!
//! These mechanics are represented as first-class effects so parser output does
//! not depend on raw oracle text passthrough for rendering.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct ExploreEffect {
    pub target: ChooseSpec,
}

impl ExploreEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for ExploreEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Runtime explore behavior is handled separately; this preserves
        // parser/render semantics without oracle-text fallback.
        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenAttractionEffect;

impl OpenAttractionEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for OpenAttractionEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestDreadEffect;

impl ManifestDreadEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for ManifestDreadEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BolsterEffect {
    pub amount: u32,
}

impl BolsterEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl EffectExecutor for BolsterEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SupportEffect {
    pub amount: u32,
}

impl SupportEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl EffectExecutor for SupportEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdaptEffect {
    pub amount: u32,
}

impl AdaptEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl EffectExecutor for AdaptEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CounterAbilityEffect;

impl CounterAbilityEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for CounterAbilityEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        Ok(EffectOutcome::from_result(EffectResult::Resolved))
    }
}
