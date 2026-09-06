use crate::cards::builders::ControlActionAst;
use crate::cards::builders::CharacteristicActionAst;
use crate::cards::builders::GrantActionAst;
use super::*;

pub(super) fn effect_duration_for_gain_followup_carry(effect: &EffectAst) -> Option<Until> {
    let duration = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Control(ControlActionAst::GainControl { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCreatureSubtypes { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice { duration, .. })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy { duration, .. })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { duration, .. })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { duration, .. })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll { duration, .. })
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
