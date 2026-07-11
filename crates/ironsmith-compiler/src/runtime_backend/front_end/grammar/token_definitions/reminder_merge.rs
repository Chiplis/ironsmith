use crate::runtime_backend::token_definition::{
    ArtifactTokenShape, CreatureTokenRulesShape, TokenDefinitionSpec, TokenRulesSurfaces,
    VehicleTokenShape,
};

use super::reminder::TokenReminderFacts;

fn merge_token_rules(base: &mut TokenRulesSurfaces, reminder: &TokenRulesSurfaces) {
    for rule in &reminder.embedded_rules {
        if !base.embedded_rules.contains(rule) {
            base.embedded_rules.push(rule.clone());
        }
    }
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
) {
    match definition {
        TokenDefinitionSpec::Vehicle(VehicleTokenShape {
            flying,
            crew_amount,
            ..
        }) => {
            *flying |= reminder.definition.vehicle_flying;
            if reminder.definition.vehicle_crew_amount.is_some() {
                *crew_amount = reminder.definition.vehicle_crew_amount;
            }
        }
        TokenDefinitionSpec::Artifact(ArtifactTokenShape {
            equipment_rules,
            token_rules,
            leaves_damage_any_target,
            ..
        }) => {
            if reminder.definition.equipment_rules.is_some() {
                *equipment_rules = reminder.definition.equipment_rules.clone();
            }
            merge_token_rules(token_rules, &reminder.definition.creature_rules.token_rules);
            if reminder
                .definition
                .artifact_leaves_damage_any_target
                .is_some()
            {
                *leaves_damage_any_target = reminder.definition.artifact_leaves_damage_any_target;
            }
        }
        TokenDefinitionSpec::Creature(creature) => {
            for keyword in &reminder.definition.keywords {
                if !creature.keywords.contains(keyword) {
                    creature.keywords.push(*keyword);
                }
            }
            merge_creature_rules(&mut creature.rules, &reminder.definition.creature_rules);
        }
        _ => {}
    }
}
