//! Special triggers (undying, persist, miracle, custom, combinators).

mod custom;
mod keyword_ability;
mod or_trigger;

pub use custom::CustomTrigger;
pub use keyword_ability::{KeywordAbilityTrigger, KeywordAbilityTriggerKind};
pub use or_trigger::OrTrigger;
