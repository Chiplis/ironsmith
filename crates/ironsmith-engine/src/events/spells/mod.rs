//! Spell-related events.

mod ability_activated;
mod ability_triggered;
mod becomes_targeted;
mod spell_cast;
mod spell_copied;
mod spell_resolved;

pub use ability_activated::AbilityActivatedEvent;
pub(crate) use ability_activated::ActivationDeclaration;
pub use ability_triggered::{AbilityTriggerZoneChangeCause, AbilityTriggeredEvent};
pub use becomes_targeted::BecomesTargetedEvent;
pub use spell_cast::SpellCastEvent;
pub use spell_copied::SpellCopiedEvent;
pub use spell_resolved::SpellResolvedEvent;

mod stack_object_added;
pub use stack_object_added::StackObjectAddedEvent;
