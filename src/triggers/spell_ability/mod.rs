//! Spell and ability triggers.

mod ability_activated;
mod becomes_targeted;
mod spell_cast;
mod you_cast_this_spell;

pub use ability_activated::AbilityActivatedTrigger;
pub use becomes_targeted::BecomesTargetedTrigger;
pub use spell_cast::SpellCastTrigger;
pub use you_cast_this_spell::YouCastThisSpellTrigger;
