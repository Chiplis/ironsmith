use crate::runtime_backend::token_definition::{
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
pub(crate) fn merge_token_equipment_reminder_definition(
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

fn merge_creature_rules(base: &mut CreatureTokenRulesShape, reminder: &CreatureTokenRulesShape) {
    merge_token_rules(&mut base.token_rules, &reminder.token_rules);
    macro_rules! merge_option {
        ($field:ident) => {
            if reminder.$field.is_some() {
                base.$field = reminder.$field.clone();
            }
        };
    }
    macro_rules! merge_bool {
        ($($field:ident),+ $(,)?) => {
            $(base.$field |= reminder.$field;)+
        };
    }
    merge_option!(cumulative_upkeep_mana_symbols);
    merge_option!(tap_mana_ability);
    merge_option!(saddle_crew_power_bonus);
    merge_option!(toxic_amount);
    merge_option!(sacrifice_return);
    merge_option!(upkeep_return_name);
    merge_option!(dies_damage_any_target);
    merge_option!(leaves_damage_you_and_creatures);
    merge_option!(noncreature_spell_each_opponent_damage);
    merge_option!(becomes_tapped_damage_player);
    merge_option!(leaves_return_named_to_hand);
    merge_option!(combat_restriction);
    merge_option!(graveyard_anthem_card_name);
    merge_bool!(
        banding,
        hexproof,
        indestructible,
        copies_exiled_triggered_abilities,
        upkeep_return_grants_haste,
        dies_create_firebreathing_dragon,
        dies_minus_one_target_creature,
        bands_with_wolves,
        red_pump,
        white_tap_target_creature,
        combat_damage_poison,
        combat_damage_gain_artifact,
        pest_dies_gain_life,
        first_strike,
        double_strike,
        mercenary_pump,
        can_block_only_flying,
        counter_noncreature_unless_pays,
        changeling,
        landfall_pump,
    );
}

pub(crate) fn merge_token_reminder_definition(
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
