use super::*;

pub(super) fn effect_duration_for_gain_followup_carry(effect: &EffectAst) -> Option<Until> {
    let duration = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GainControl { duration, .. }
                | SubjectVerbActionAst::Pump { duration, .. }
                | SubjectVerbActionAst::PumpForEach { duration, .. }
                | SubjectVerbActionAst::PumpAll { duration, .. }
                | SubjectVerbActionAst::PumpByLastEffect { duration, .. }
                | SubjectVerbActionAst::SetBasePowerToughness { duration, .. }
                | SubjectVerbActionAst::SetBasePower { duration, .. }
                | SubjectVerbActionAst::BecomeBasePtCreature { duration, .. }
                | SubjectVerbActionAst::AddCardTypes { duration, .. }
                | SubjectVerbActionAst::SetCardTypes { duration, .. }
                | SubjectVerbActionAst::RemoveCardTypes { duration, .. }
                | SubjectVerbActionAst::AddSubtypes { duration, .. }
                | SubjectVerbActionAst::RemoveSubtypes { duration, .. }
                | SubjectVerbActionAst::SetCreatureSubtypes { duration, .. }
                | SubjectVerbActionAst::AddColors { duration, .. }
                | SubjectVerbActionAst::AddAllSubtypesOfFamily { duration, .. }
                | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { duration, .. }
                | SubjectVerbActionAst::BecomeAuraEnchantment { duration, .. }
                | SubjectVerbActionAst::SetColors { duration, .. }
                | SubjectVerbActionAst::MakeColorless { duration, .. }
                | SubjectVerbActionAst::BecomeBasicLandType { duration, .. }
                | SubjectVerbActionAst::BecomeBasicLandTypeChoice { duration, .. }
                | SubjectVerbActionAst::BecomeColorChoice { duration, .. }
                | SubjectVerbActionAst::BecomeCreatureTypeChoice { duration, .. }
                | SubjectVerbActionAst::BecomeCopy { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesToTarget { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesAll { duration, .. }
                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { duration, .. }
                | SubjectVerbActionAst::RemoveAbilitiesFromTarget { duration, .. }
                | SubjectVerbActionAst::RemoveAbilitiesAll { duration, .. }
                | SubjectVerbActionAst::Cant { duration, .. },
            ..
        }) => duration,
        _ => return None,
    };

    if matches!(duration, Until::Forever) {
        None
    } else {
        Some(duration.clone())
    }
}
