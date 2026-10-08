//! Fail closed before dispatch, including durations copied by chain readers.
use super::*;
use crate::cards::builders::{
    DamagePreventionActionAst as D, ExchangeActionAst as E,
    PermanentStateActionAst as P, ReplacementActionAst as R, StackActionAst,
    TurnStructureActionAst as T,
};
use crate::effect::Until;

pub(super) fn validate(action: &SubjectVerbActionAst) -> Result<(), CardTextError> {
    // Inventory of all direct Until-bearing subject-verb actions. Other
    // duration types (GrantDuration, ControlDurationAst, PhaseOutDuration,
    // ZoneReplacementDurationAst, ExileUntilDuration) cannot carry these variants.
    let duration = match action {
        SubjectVerbActionAst::Cant { duration, .. }
        | SubjectVerbActionAst::Control(ControlActionAst::GainControl { duration, .. })
        | SubjectVerbActionAst::Characteristics(
            CharacteristicActionAst::ChangeText { duration, .. }
            | CharacteristicActionAst::SetBasePowerToughness { duration, .. }
            | CharacteristicActionAst::BecomeBasePtCreature { duration, .. }
            | CharacteristicActionAst::SetBasePower { duration, .. }
            | CharacteristicActionAst::SetBaseToughness { duration, .. }
            | CharacteristicActionAst::AddCardTypes { duration, .. }
            | CharacteristicActionAst::SetCardTypes { duration, .. }
            | CharacteristicActionAst::AddSubtypes { duration, .. }
            | CharacteristicActionAst::SetCreatureSubtypes { duration, .. }
            | CharacteristicActionAst::AddColors { duration, .. }
            | CharacteristicActionAst::AddAllSubtypesOfFamily { duration, .. }
            | CharacteristicActionAst::BecomeAuraEnchantment { duration, .. }
            | CharacteristicActionAst::BecomeBasicLandType { duration, .. }
            | CharacteristicActionAst::SetColors { duration, .. }
            | CharacteristicActionAst::BecomeBasicLandTypeChoice { duration, .. }
            | CharacteristicActionAst::BecomeCreatureTypeChoice { duration, .. }
            | CharacteristicActionAst::BecomeColorChoice { duration, .. }
            | CharacteristicActionAst::BecomeCopy { duration, .. }
        )
        | SubjectVerbActionAst::StatChanges(
            StatChangeActionAst::Pump { duration, .. }
            | StatChangeActionAst::PumpForEach { duration, .. }
            | StatChangeActionAst::PumpAll { duration, .. }
            | StatChangeActionAst::PumpByLastEffect { duration, .. }
            | StatChangeActionAst::RemoveCardTypes { duration, .. }
            | StatChangeActionAst::RemoveSubtypes { duration, .. }
            | StatChangeActionAst::RemoveAllSubtypesOfFamily { duration, .. }
            | StatChangeActionAst::MakeColorless { duration, .. }
            | StatChangeActionAst::RemoveAbilitiesAll { duration, .. }
            | StatChangeActionAst::RemoveAbilitiesFromTarget { duration, .. }
            | StatChangeActionAst::RemoveSupertypes { duration, .. }
        )
        | SubjectVerbActionAst::Grants(
            GrantActionAst::GrantAbilitiesAll { duration, .. }
            | GrantActionAst::GrantAbilitiesChoiceAll { duration, .. }
            | GrantActionAst::GrantAbilitiesToTarget { duration, .. }
            | GrantActionAst::GrantAbilitiesChoiceToTarget { duration, .. }
            | GrantActionAst::GrantAbilityToSource { duration, .. }
        )
        | SubjectVerbActionAst::DamagePrevention(
            D::PreventAllCombatDamage { duration }
            | D::AssignNoCombatDamage { duration, .. }
            | D::PreventAllCombatDamageFromSource { duration, .. }
            | D::PreventAllCombatDamageFromSourceFilter { duration, .. }
            | D::PreventAllCombatDamageToPlayers { duration }
            | D::PreventAllCombatDamageToYou { duration, .. }
            | D::PreventDamage { duration, .. }
            | D::PreventAllDamageToTarget { duration, .. }
            | D::PreventAllDamageToTargetFromSourceFilter { duration, .. }
            | D::PreventAllDamageFromSourceFilter { duration, .. }
            | D::PreventDamageToTargetPutCounters { duration, .. }
            | D::PreventDamageEach { duration, .. }
        )
        | SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn { duration, .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad { duration, .. })
        | SubjectVerbActionAst::TurnStructure(T::AdditionalLandPlays { duration, .. })
        | SubjectVerbActionAst::Exchanges(E::ExchangeValues { duration, .. })
        | SubjectVerbActionAst::PermanentState(
            P::SwitchPowerToughness { duration, .. } | P::ScalePowerToughnessAll { duration, .. }
        )
        | SubjectVerbActionAst::Replacements(R::RegisterManaSpendPermission { until: duration, .. }) => duration,
        _ => return Ok(()),
    };
    let supported = match duration {
        Until::PlayersNextUntapStep { .. } => matches!(action, SubjectVerbActionAst::Cant { .. }),
        Until::UntilControllersNextUntapStep { .. } => matches!(action,
            SubjectVerbActionAst::Characteristics(
                CharacteristicActionAst::BecomeBasicLandType { .. }
                | CharacteristicActionAst::BecomeBasicLandTypeChoice { .. }
            )),
        _ => true,
    };
    if supported {
        Ok(())
    } else {
        Err(CardTextError::ParseError("next-untap duration has no supported lifetime owner for this action".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carried_or_direct_prevention_and_other_owners_reject_both_new_durations() {
        for duration in [
            Until::PlayersNextUntapStep { player: PlayerFilter::Specific(ironsmith_core::PlayerId(1)) },
            Until::UntilControllersNextUntapStep { object: ironsmith_core::ContinuousDurationObject::AffectedObject },
        ] {
            for action in [
                SubjectVerbActionAst::DamagePrevention(D::PreventAllCombatDamage { duration: duration.clone() }),
                SubjectVerbActionAst::DamagePrevention(D::PreventAllDamageFromSourceFilter {
                    duration: duration.clone(), source_filter: ObjectFilter::creature(), of_chosen_color: false,
                    source_would_deal_surface: false, follow_up_effects: Vec::new(),
                }),
                SubjectVerbActionAst::DamagePrevention(D::AssignNoCombatDamage {
                    duration: duration.clone(), source: TargetAst::Source(None),
                }),
                SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower {
                    duration: duration.clone(), target: TargetAst::Source(None), power: Value::Fixed(5),
                }),
                SubjectVerbActionAst::TurnStructure(T::AdditionalLandPlays { duration: duration.clone(), count: Value::Fixed(1) }),
                SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad {
                    duration: duration.clone(), target: TargetAst::Source(None), spelled_out_requirement: false,
                }),
            ] {
                assert!(validate(&action).is_err());
            }
        }
    }
}
