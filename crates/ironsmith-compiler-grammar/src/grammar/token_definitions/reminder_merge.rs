use crate::model::token_definition::{
    ArtifactTokenShape, CreatureTokenRulesShape, EquipmentRuleLineShape, EquipmentRulesShape,
    TokenDefinitionSpec, TokenRulesSurfaces, VehicleTokenShape, ModifiedBuiltinTokenShape,
};

use super::reminder::TokenReminderFacts;

fn merge_token_rules(base: &mut TokenRulesSurfaces, reminder: &TokenRulesSurfaces) {
    for rule in &reminder.embedded_rules {
        if !base.embedded_rules.contains(rule) {
            base.embedded_rules.push(rule.clone());
        }
    }
    // Quoted rules read by the ordinary ability readers are abilities of the
    // token too; a follow-up reminder ("It has \"...\"") must not drop them.
    for ability in &reminder.parsed_abilities {
        if !base.parsed_abilities.contains(ability) {
            base.parsed_abilities.push(ability.clone());
        }
    }
    for ability in &reminder.static_abilities {
        if !base.static_abilities.contains(ability) {
            base.static_abilities.push(ability.clone());
        }
    }
}

fn equipment_rule_display(line: &EquipmentRuleLineShape) -> String {
    match line {
        EquipmentRuleLineShape::GrantedDamage { display_text, .. }
        | EquipmentRuleLineShape::StaticGrant { display_text, .. } => display_text.clone(),
        EquipmentRuleLineShape::Equip(equip) => format!("Equip {{{}}}", equip.amount),
    }
}

fn merge_equipment_rules(
    base: &mut Option<EquipmentRulesShape>,
    reminder: &Option<EquipmentRulesShape>,
) {
    let Some(reminder) = reminder else {
        return;
    };
    let Some(base) = base else {
        *base = Some(reminder.clone());
        return;
    };

    for line in &reminder.lines {
        if !base.lines.contains(line) {
            base.lines.push(line.clone());
        }
    }
    base.text = base
        .lines
        .iter()
        .map(equipment_rule_display)
        .collect::<Vec<_>>()
        .join("\n");
}

/// Merge only Equipment rule facts from a complete token clause.
///
/// Full-clause reminder parsing is needed to see an unquoted trailing
/// `and equip {N}`, but merging every reminder fact from that same clause can
/// incorrectly copy keywords from a nested token inside a quoted rule onto the
/// outer token. This narrow entry point preserves the typed equip payload
/// without reopening that broader scope leak.
pub fn merge_token_equipment_reminder_definition(
    definition: &mut TokenDefinitionSpec,
    reminder: &TokenReminderFacts,
) -> bool {
    merge_token_equipment_facts(definition, reminder, false)
}

pub fn merge_authored_token_equipment_facts(
    definition: &mut TokenDefinitionSpec,
    facts: &TokenReminderFacts,
) -> bool {
    merge_token_equipment_facts(definition, facts, true)
}

fn merge_token_equipment_facts(
    definition: &mut TokenDefinitionSpec,
    reminder: &TokenReminderFacts,
    authored: bool,
) -> bool {
    let before = definition.clone();
    let TokenDefinitionSpec::Artifact(ArtifactTokenShape {
        equipment_rules, ..
    }) = definition
    else {
        return false;
    };
    let has_equipment_rules = reminder.definition.equipment_rules.is_some();
    merge_equipment_rules(equipment_rules, &reminder.definition.equipment_rules);
    if !authored && *definition != before { definition.mark_unproven_ability_words(); }
    has_equipment_rules
}

#[path = "reminder_merge/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::{merge_authored_token_definition_facts, merge_token_reminder_definition};
#[path = "reminder_merge/core.rs"]
mod core_programs;
use core_programs::merge_creature_rules;
