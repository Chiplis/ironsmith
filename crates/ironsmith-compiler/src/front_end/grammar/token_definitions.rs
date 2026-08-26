#[path = "token_definitions/common.rs"]
mod common;
#[path = "token_definitions/equipment.rs"]
mod equipment;
#[path = "token_definitions/names.rs"]
mod names;
#[path = "token_definitions/reminder.rs"]
mod reminder;
#[path = "token_definitions/reminder_merge.rs"]
mod reminder_merge;
#[path = "token_definitions/rules.rs"]
mod rules;
#[path = "token_definitions/surface.rs"]
mod surface;

pub use crate::model::token_definition::*;
#[cfg(test)]
pub use equipment::parse_equipment_rules_tokens;
pub use reminder::*;
pub use reminder_merge::*;
pub use rules::*;
pub use surface::*;

/// Preserve and title-case a proper token name that precedes its appositive
/// definition (`Name, Epithet, a legendary ... token`).
pub fn leading_appositive_token_name(tokens: &[crate::lexer::OwnedLexToken]) -> Option<String> {
    names::leading_comma_name(tokens)
}
