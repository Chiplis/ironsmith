//! Stack interaction effects.
//!
//! This module contains effects that interact with the stack,
//! such as countering spells and copying spells.

mod choose_new_targets;
mod copy_spell;
mod copy_spell_for_each_target;
mod counter;
mod retarget_stack_object;
mod variable_casualty_planeswalker_copy;

pub use choose_new_targets::ChooseNewTargetsEffect;
pub use copy_spell::CopySpellEffect;
pub use copy_spell_for_each_target::CopySpellForEachTargetEffect;
pub use counter::CounterEffect;
pub use retarget_stack_object::{NewTargetRestriction, RetargetMode, RetargetStackObjectEffect};
pub use variable_casualty_planeswalker_copy::VariableCasualtyPlaneswalkerCopyEffect;
