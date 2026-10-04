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
        "DealDamageEachEffect" => {
            decode_as::<ironsmith_core::DealDamageEachEffect>(payload).map(Some)
        }
        "DealDamageBySourcesEffect" => {
            decode_as::<ironsmith_core::DealDamageBySourcesEffect>(payload).map(Some)
        }
        "DealDamageToRecipientsEffect" => {
            decode_as::<ironsmith_core::DealDamageToRecipientsEffect>(payload).map(Some)
        }
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
            decode_as::<ironsmith_core::PreventAllDamageEffect<wire::WireEffect>>(payload).map(Some)
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
        "BecomeBlockedEffect" => decode_as::<ironsmith_core::BecomeBlockedEffect>(payload).map(Some),
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

pub(super) fn map_card_ids(
    kind: &str,
    payload: Value,
    context: &super::card_graph::Context<'_>,
) -> Result<Option<Value>, String> {
    match kind {
        "AssignNoCombatDamageEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AssignNoCombatDamageEffect,
        >(payload, context)
        .map(Some),
        "DealDamageEachEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::DealDamageEachEffect,
        >(payload, context)
        .map(Some),
        "DealDamageBySourcesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::DealDamageBySourcesEffect,
        >(payload, context)
        .map(Some),
        "DealDamageToRecipientsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::DealDamageToRecipientsEffect,
        >(payload, context)
        .map(Some),
        "DealDamageEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::DealDamageEffect>(payload, context)
                .map(Some)
        }
        "DealDistributedDamageEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::DealDistributedDamageEffect,
        >(payload, context)
        .map(Some),
        "ExchangeValuesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExchangeValuesEffect,
        >(payload, context)
        .map(Some),
        "FightEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::FightEffect>(payload, context)
                .map(Some)
        }
        "GoadEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::GoadEffect>(payload, context)
                .map(Some)
        }
        "ClearGoadEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ClearGoadEffect>(payload, context)
                .map(Some)
        }
        "GrantAbilitiesTargetEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantAbilitiesTargetEffect<wire::WireStaticAbility>,
        >(payload, context)
        .map(Some),
        "HealDamageEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::HealDamageEffect>(payload, context)
                .map(Some)
        }
        "ModifyPowerToughnessEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ModifyPowerToughnessEffect,
        >(payload, context)
        .map(Some),
        "ModifyPowerToughnessForEachEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ModifyPowerToughnessForEachEffect,
        >(payload, context)
        .map(Some),
        "PreventAllCombatDamageEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PreventAllCombatDamageEffect,
        >(payload, context)
        .map(Some),
        "PreventAllDamageEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PreventAllDamageEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "PreventAllDamageToTargetEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PreventAllDamageToTargetEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "PreventDamageEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PreventDamageEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "PreventNextTimeDamageEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PreventNextTimeDamageEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "RedirectAllDamageThisTurnToTargetEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RedirectAllDamageThisTurnToTargetEffect,
        >(payload, context)
        .map(Some),
        "RedirectNextDamageToTargetEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RedirectNextDamageToTargetEffect,
        >(payload, context)
        .map(Some),
        "RedirectNextTimeDamageToSourceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RedirectNextTimeDamageToSourceEffect,
        >(payload, context)
        .map(Some),
        "RemoveFromCombatEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RemoveFromCombatEffect,
        >(payload, context)
        .map(Some),
        "ReplaceNextDamageToTargetEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReplaceNextDamageToTargetEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "SetBasePowerToughnessEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SetBasePowerToughnessEffect,
        >(payload, context)
        .map(Some),
        _ => Ok(None),
    }
}
