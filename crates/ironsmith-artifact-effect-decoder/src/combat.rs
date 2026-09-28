//! Generated typed materializers for the combat runtime effect family.

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde_json::Value;

use super::{ErasedPayload, decode_as};

pub fn decode(kind: &str, payload: Value) -> Result<Option<ErasedPayload>, String> {
    match kind {
        "AssignNoCombatDamageEffect" => {
            decode_as::<ironsmith_core::AssignNoCombatDamageEffect>(payload).map(Some)
        }
        "DealDamageEffect" => decode_as::<ironsmith_core::DealDamageEffect>(payload).map(Some),
        "DealDistributedDamageEffect" => {
            decode_as::<ironsmith_core::DealDistributedDamageEffect>(payload).map(Some)
        }
        "ExchangeValuesEffect" => {
            decode_as::<ironsmith_core::ExchangeValuesEffect>(payload).map(Some)
        }
        "FightEffect" => decode_as::<ironsmith_core::FightEffect>(payload).map(Some),
        "GoadEffect" => decode_as::<ironsmith_core::GoadEffect>(payload).map(Some),
        "ClearGoadEffect" => decode_as::<ironsmith_core::ClearGoadEffect>(payload).map(Some),
        "GrantAbilitiesTargetEffect" => decode_as::<
            ironsmith_core::GrantAbilitiesTargetEffect<wire::WireStaticAbility>,
        >(payload)
        .map(Some),
        "HealDamageEffect" => decode_as::<ironsmith_core::HealDamageEffect>(payload).map(Some),
        "ModifyPowerToughnessEffect" => {
            decode_as::<ironsmith_core::ModifyPowerToughnessEffect>(payload).map(Some)
        }
        "ModifyPowerToughnessForEachEffect" => {
            decode_as::<ironsmith_core::ModifyPowerToughnessForEachEffect>(payload).map(Some)
        }
        "PreventAllCombatDamageEffect" => {
            decode_as::<ironsmith_core::PreventAllCombatDamageEffect>(payload).map(Some)
        }
        "PreventAllDamageEffect" => {
            decode_as::<ironsmith_core::PreventAllDamageEffect>(payload).map(Some)
        }
        "PreventAllDamageToTargetEffect" => {
            decode_as::<ironsmith_core::PreventAllDamageToTargetEffect<wire::WireEffect>>(payload)
                .map(Some)
        }
        "PreventDamageEffect" => {
            decode_as::<ironsmith_core::PreventDamageEffect<wire::WireEffect>>(payload).map(Some)
        }
        "PreventNextTimeDamageEffect" => {
            decode_as::<ironsmith_core::PreventNextTimeDamageEffect<wire::WireEffect>>(payload)
                .map(Some)
        }
        "RedirectAllDamageThisTurnToTargetEffect" => {
            decode_as::<ironsmith_core::RedirectAllDamageThisTurnToTargetEffect>(payload).map(Some)
        }
        "RedirectNextDamageToTargetEffect" => {
            decode_as::<ironsmith_core::RedirectNextDamageToTargetEffect>(payload).map(Some)
        }
        "RedirectNextTimeDamageToSourceEffect" => {
            decode_as::<ironsmith_core::RedirectNextTimeDamageToSourceEffect>(payload).map(Some)
        }
        "RemoveFromCombatEffect" => {
            decode_as::<ironsmith_core::RemoveFromCombatEffect>(payload).map(Some)
        }
        "ReplaceNextDamageToTargetEffect" => {
            decode_as::<ironsmith_core::ReplaceNextDamageToTargetEffect<wire::WireEffect>>(payload)
                .map(Some)
        }
        "SetBasePowerToughnessEffect" => {
            decode_as::<ironsmith_core::SetBasePowerToughnessEffect>(payload).map(Some)
        }
        _ => Ok(None),
    }
}
