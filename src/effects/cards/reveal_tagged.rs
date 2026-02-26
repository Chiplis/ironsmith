//! Reveal tagged cards effect implementation.
//!
//! The engine does not fully model hidden information; "reveal" is treated as a
//! semantic no-op that can still be referenced by compiled text and auditing.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;

/// Effect that reveals the objects currently tagged under `tag`.
///
/// This is mainly used to support clauses like "reveal it" where "it" refers to
/// a card found/drawn earlier in the same effect chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealTaggedEffect {
    pub tag: TagKey,
}

impl RevealTaggedEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

impl EffectExecutor for RevealTaggedEffect {
    fn execute(
        &self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = ctx
            .get_tagged_all(self.tag.clone())
            .map(|objs| objs.len())
            .unwrap_or(0);
        Ok(EffectOutcome::from_result(EffectResult::Count(
            count as i32,
        )))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

