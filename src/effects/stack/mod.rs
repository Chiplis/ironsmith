//! Stack interaction effects.
//!
//! This module contains effects that interact with the stack,
//! such as countering spells and copying spells.

mod choose_new_targets;
mod copy_spell;
mod counter;
mod counter_unless_pays;

pub use choose_new_targets::ChooseNewTargetsEffect;
pub use copy_spell::CopySpellEffect;
pub use counter::CounterEffect;
pub use counter_unless_pays::CounterUnlessPaysEffect;
