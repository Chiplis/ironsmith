//! Spell-related events.

mod ability_activated;
mod becomes_targeted;
mod spell_cast;
mod spell_copied;

pub use ability_activated::AbilityActivatedEvent;
pub use becomes_targeted::BecomesTargetedEvent;
pub use spell_cast::SpellCastEvent;
pub use spell_copied::SpellCopiedEvent;
