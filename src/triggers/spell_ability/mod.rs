//! Spell and ability triggers.

mod ability_activated;
mod becomes_targeted;
mod becomes_targeted_by_spell;
mod spell_cast;
mod spell_copied;
mod tap_for_mana;
mod you_cast_this_spell;

pub use ability_activated::AbilityActivatedTrigger;
pub use becomes_targeted::BecomesTargetedTrigger;
pub use becomes_targeted_by_spell::BecomesTargetedBySpellTrigger;
pub use spell_cast::SpellCastTrigger;
pub use spell_copied::SpellCopiedTrigger;
pub use tap_for_mana::TapForManaTrigger;
pub use you_cast_this_spell::YouCastThisSpellTrigger;
