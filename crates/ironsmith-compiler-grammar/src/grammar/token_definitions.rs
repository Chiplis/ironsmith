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
#[cfg(any(test, feature = "test-support"))]
pub use equipment::parse_equipment_rules_tokens;
pub use names::leading_appositive_token_name;
pub use reminder::*;
pub use reminder_merge::*;
pub use rules::*;
pub use surface::*;

pub(crate) fn fixed_creature(
    name: &str,
    subtypes: Vec<crate::types::Subtype>,
    colors: crate::color::ColorSet,
    power_toughness: (i32, i32),
    text_roles: Option<TokenDescriptionTextRoles>,
) -> TokenDefinitionSpec {
    TokenDefinitionSpec::Creature(CreatureTokenShape {
        name: name.into(), text_roles, card_types: vec![crate::types::CardType::Creature], subtypes,
        power_toughness, colors, legendary: false, snow: false, use_source_chosen_color: false,
        use_source_chosen_creature_type: false, keywords: Vec::new(), rules: Default::default(), mana_cost: None,
    })
}
