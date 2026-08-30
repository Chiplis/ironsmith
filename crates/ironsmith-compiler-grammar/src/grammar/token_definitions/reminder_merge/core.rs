use super::*;

pub(super) fn merge_creature_rules(
    base: &mut CreatureTokenRulesShape,
    reminder: &CreatureTokenRulesShape,
) {
    merge_token_rules(&mut base.token_rules, &reminder.token_rules);
    for presentation in &reminder.authored_inline_rules {
        if !base.authored_inline_rules.contains(presentation) {
            base.authored_inline_rules.push(presentation.clone());
        }
    }
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
