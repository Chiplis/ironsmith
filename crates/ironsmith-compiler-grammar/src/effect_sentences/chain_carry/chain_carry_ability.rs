use super::*;
use crate::cards::builders::CharacteristicActionAst;
use crate::cards::builders::ControlActionAst;
use crate::cards::builders::GrantActionAst;

pub(super) fn effect_duration_for_gain_followup_carry(effect: &EffectAst) -> Option<Until> {
    let duration = match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Control(ControlActionAst::GainControl { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll { duration, .. })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBaseToughness {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes {
                    duration, ..
                })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSupertypes {
                    duration, ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes {
                    duration, ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCreatureSubtypes {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless {
                    duration, ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(
                    CharacteristicActionAst::BecomeBasicLandTypeChoice { duration, .. },
                )
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(
                    CharacteristicActionAst::BecomeCreatureTypeChoice { duration, .. },
                )
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                    duration, ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { duration, .. })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
                    duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
                    duration,
                    ..
                })
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

/// Fold a bare "loses B until end of turn" arm (parsed with a synthetic "it"
/// subject) back into the preceding lose-ability effect over the shared
/// subject: "Permanents your opponents control lose hexproof and
/// indestructible until end of turn" is one removal of both abilities from
/// the same locked set, for the stated duration (CR 611.2a, 611.2c).
/// `followup` is always the synthetic implicit-"it" arm built by the chain
/// splitter. Returns false when `previous` isn't a lose-ability effect,
/// leaving both untouched.
pub(super) fn merge_shared_subject_lose_followup(
    previous: &mut EffectAst,
    followup: &EffectAst,
) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
                abilities: followup_abilities,
                duration: followup_duration,
                ..
            }),
        ..
    }) = followup
    else {
        return false;
    };
    let (abilities, duration) = match previous {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::StatChanges(
                    StatChangeActionAst::RemoveAbilitiesAll {
                        abilities,
                        duration,
                        condition: None,
                        ..
                    },
                ),
            ..
        }) => (abilities, duration),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
                    abilities,
                    duration,
                    ..
                }),
            ..
        }) => (abilities, duration),
        _ => return false,
    };
    if !matches!(duration, Until::Forever) && duration != followup_duration {
        return false;
    }
    abilities.extend(followup_abilities.iter().cloned());
    *duration = followup_duration.clone();
    true
}

