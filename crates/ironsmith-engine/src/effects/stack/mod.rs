//! Stack interaction effects.
//!
//! This module contains effects that interact with the stack,
//! such as countering spells and copying spells.

mod choose_new_targets;
mod copy_source_selection;
pub(crate) use copy_source_selection::{select_copy_source_definition,CopySourceSelection,ForcedCopySourceDecision};
mod copy_from_definition;
pub use copy_from_definition::CopyFromDefinitionEffect;
pub(crate) use copy_from_definition::definition_copy_source;
mod copy_spell;
mod copy_spell_for_each_target;
mod counter;
mod epic_spell_copy;
mod retained_spell_copy;
mod retarget_stack_object;
mod scale_x_value;
mod variable_casualty_planeswalker_copy;

pub use choose_new_targets::ChooseNewTargetsEffect;
pub use copy_spell::CopySpellEffect;
pub(crate) use copy_spell::discard_departed_ability_copy_object;
pub use copy_spell_for_each_target::CopySpellForEachTargetEffect;
pub use counter::CounterEffect;
pub(crate) use counter::{counter_stack_entry_at_with_outputs};
pub(crate) use epic_spell_copy::EpicSpellCopyEffect;
pub use retained_spell_copy::{PutRetainedSpellCopyOntoStackEffect, RetainSpellChoicesEffect};
pub use retarget_stack_object::{NewTargetRestriction, RetargetMode, RetargetStackObjectEffect};
pub use scale_x_value::ScaleXValueEffect;
pub use variable_casualty_planeswalker_copy::VariableCasualtyPlaneswalkerCopyEffect;

mod cast_stored_card_copy;
pub(crate) use cast_stored_card_copy::CastStoredCardCopyEffect;

mod trigger_by_class;
pub use trigger_by_class::TriggerAbilitiesByClassEffect;
