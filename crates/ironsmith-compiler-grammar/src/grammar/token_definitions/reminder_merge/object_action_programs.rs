use super::*;

pub fn merge_token_reminder_definition(
    definition: &mut TokenDefinitionSpec,
    reminder: &TokenReminderFacts,
) -> bool {
    match definition {
        TokenDefinitionSpec::Vehicle(VehicleTokenShape {
            flying,
            crew_amount,
            ..
        }) => {
            let has_definition_facts = reminder.definition.vehicle_flying
                || reminder.definition.vehicle_crew_amount.is_some();
            *flying |= reminder.definition.vehicle_flying;
            if reminder.definition.vehicle_crew_amount.is_some() {
                *crew_amount = reminder.definition.vehicle_crew_amount;
            }
            has_definition_facts
        }
        TokenDefinitionSpec::Artifact(ArtifactTokenShape {
            equipment_rules,
            token_rules,
            leaves_damage_any_target,
            ..
        }) => {
            let has_definition_facts = reminder.definition.equipment_rules.is_some()
                || !reminder
                    .definition
                    .creature_rules
                    .token_rules
                    .embedded_rules
                    .is_empty()
                || reminder
                    .definition
                    .artifact_leaves_damage_any_target
                    .is_some();
            // A quoted equipped-creature grant and an unquoted trailing
            // `and equip {N}` are parsed independently. Merge their typed
            // lines instead of replacing the complete definition with the
            // quoted subset and silently deleting the executable equip
            // ability.
            merge_equipment_rules(equipment_rules, &reminder.definition.equipment_rules);
            merge_token_rules(token_rules, &reminder.definition.creature_rules.token_rules);
            if reminder
                .definition
                .artifact_leaves_damage_any_target
                .is_some()
            {
                *leaves_damage_any_target = reminder.definition.artifact_leaves_damage_any_target;
            }
            has_definition_facts
        }
        TokenDefinitionSpec::Creature(creature) => {
            let has_definition_facts = !reminder.definition.keywords.is_empty()
                || reminder.definition.creature_rules != CreatureTokenRulesShape::default();
            for keyword in &reminder.definition.keywords {
                if !creature.keywords.contains(keyword) {
                    creature.keywords.push(*keyword);
                }
            }
            merge_creature_rules(&mut creature.rules, &reminder.definition.creature_rules);
            has_definition_facts
        }
        _ => false,
    }
}
