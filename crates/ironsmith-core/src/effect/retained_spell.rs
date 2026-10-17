//! Spells put back onto the stack as copies of themselves (Ertai's Meddling:
//! "Target spell's controller exiles it with X delay counters on it. ... the
//! player puts it onto the stack as a copy of the original spell").
//!
//! `RetainSpellChoicesEffect` keeps a spell's announced choices (targets,
//! X, modes, optional costs; CR 601.2b-d) with its card, by stable identity,
//! while the spell is still on the stack. `PutRetainedSpellCopyOntoStackEffect`
//! later puts that card onto the stack (it is not cast) as a copy of the
//! original spell: the copy keeps those choices (CR 707.10) and is controlled
//! by the original spell's controller.
use crate::tag::TagKeyWalk;
use crate::target_model::ChooseSpec;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct RetainSpellChoicesEffect {
    pub target: ChooseSpec,
    /// The spell's controller is tagged under this player tag ("that
    /// player" for later instructions and delayed triggers).
    pub controller_tag: crate::tag::TagKey,
}

impl RetainSpellChoicesEffect {
    pub fn new(target: ChooseSpec, controller_tag: impl Into<crate::tag::TagKey>) -> Self {
        Self {
            target,
            controller_tag: controller_tag.into(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct PutRetainedSpellCopyOntoStackEffect {
    pub target: ChooseSpec,
}

impl PutRetainedSpellCopyOntoStackEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}
