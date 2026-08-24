use super::*;

pub(super) fn spell_cast_trigger_filter(
    trigger: &TriggerSpec,
) -> Option<(ObjectFilter, PlayerFilter)> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => spell_cast_trigger_filter(trigger),
        TriggerSpec::SpellCast {
            filter: Some(filter),
            mana_source_filter: None,
            caster,
            timing: None,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            from_not_hand: false,
        } => Some((filter.clone(), caster.clone())),
        _ => None,
    }
}
