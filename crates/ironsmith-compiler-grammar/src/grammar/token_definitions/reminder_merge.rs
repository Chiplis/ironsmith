use crate::model::token_definition::{
    ArtifactTokenShape, CreatureTokenRulesShape, EquipmentRuleLineShape, EquipmentRulesShape,
    TokenDefinitionSpec, TokenRulesSurfaces, VehicleTokenShape,
};

use super::reminder::TokenReminderFacts;

fn merge_token_rules(base: &mut TokenRulesSurfaces, reminder: &TokenRulesSurfaces) {
    for rule in &reminder.embedded_rules {
        if !base.embedded_rules.contains(rule) {
            base.embedded_rules.push(rule.clone());
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
    let TokenDefinitionSpec::Artifact(ArtifactTokenShape {
        equipment_rules, ..
    }) = definition
    else {
        return false;
    };
    let has_equipment_rules = reminder.definition.equipment_rules.is_some();
    merge_equipment_rules(equipment_rules, &reminder.definition.equipment_rules);
    has_equipment_rules
}

#[path = "reminder_merge/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::merge_token_reminder_definition;
#[path = "reminder_merge/core.rs"]
mod core_programs;
use core_programs::merge_creature_rules;
