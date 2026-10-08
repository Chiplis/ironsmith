//! Generated typed materializers for the stack-event runtime effect family.

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde_json::Value;

use super::{ErasedPayload, decode_as};

fn validated_counter_effect(payload: Value) -> Result<ironsmith_core::CounterEffect, String> {
    let effect: ironsmith_core::CounterEffect =
        serde_json::from_value(payload).map_err(|error| error.to_string())?;
    if !effect.exile_permission_target_is_supported() {
        return Err("counter exile permission requires one exact stack spell target".into());
    }
    Ok(effect)
}

pub fn decode(kind: &str, payload: Value) -> Result<Option<ErasedPayload>, String> {
    match kind {
        "CantEffect" => decode_as::<ironsmith_core::CantEffect>(payload).map(Some),
        "ChooseNewTargetsEffect" => {
            decode_as::<ironsmith_core::ChooseNewTargetsEffect>(payload).map(Some)
        }
        "CopySpellEffect" => decode_as::<ironsmith_core::CopySpellEffect>(payload).map(Some),
        "CopySpellForEachTargetEffect" => {
            decode_as::<ironsmith_core::CopySpellForEachTargetEffect>(payload).map(Some)
        }
        "CounterEffect" => validated_counter_effect(payload)
            .map(|effect| Some(Box::new(effect) as ErasedPayload)),
        "ExileTaggedWhenSourceLeavesEffect" => {
            decode_as::<ironsmith_core::ExileTaggedWhenSourceLeavesEffect>(payload).map(Some)
        }
        "RegisterDamagedBySourceZoneReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterDamagedBySourceZoneReplacementEffect>(payload)
                .map(Some)
        }
        "RegisterDrawReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterDrawReplacementEffect<wire::WireEffect>>(payload)
                .map(Some)
        }
        "RegisterEnterTappedReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterEnterTappedReplacementEffect>(payload).map(Some)
        }
        "RegisterEnterUnderControlReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterEnterUnderControlReplacementEffect>(payload)
                .map(Some)
        }
        "RegisterFutureZoneReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterFutureZoneReplacementEffect>(payload).map(Some)
        }
        "RegisterManaRewriteEffect" => decode_as::<ironsmith_core::RegisterManaRewriteEffect>(payload).map(Some),
        "RegisterManaSpendPermissionEffect" => decode_as::<ironsmith_core::RegisterManaSpendPermissionEffect>(payload).map(Some),
        "RegisterManaReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterManaReplacementEffect>(payload).map(Some)
        }
        "RegisterDamageMultiplierEffect" => {
            decode_as::<ironsmith_core::RegisterDamageMultiplierEffect>(payload).map(Some)
        }
        "RegisterDamageAdditionEffect" => {
            decode_as::<ironsmith_core::RegisterDamageAdditionEffect>(payload).map(Some)
        }
        "RegisterCounterPlacementReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterCounterPlacementReplacementEffect>(payload)
                .map(Some)
        }
        "RegisterEnterWithCountersReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterEnterWithCountersReplacementEffect>(payload)
                .map(Some)
        }
        "RegisterNextBatchEnterWithCountersEffect" => {
            decode_as::<ironsmith_core::RegisterNextBatchEnterWithCountersEffect>(payload).map(Some)
        }
        "RegisterZoneReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterZoneReplacementEffect>(payload).map(Some)
        }
        "RetargetStackObjectEffect" => {
            decode_as::<ironsmith_core::RetargetStackObjectEffect>(payload).map(Some)
        }
        "ScheduleDelayedTriggerEffect" => {
            decode_as::<ironsmith_core::ScheduleDelayedTriggerEffect<wire::WireEffect>>(payload)
                .map(Some)
        }
        "ScheduleEffectsWhenTaggedLeavesEffect" => decode_as::<
            ironsmith_core::ScheduleEffectsWhenTaggedLeavesEffect<wire::WireEffect>,
        >(payload)
        .map(Some),
        "VariableCasualtyPlaneswalkerCopyEffect" => {
            decode_as::<ironsmith_core::VariableCasualtyPlaneswalkerCopyEffect>(payload).map(Some)
        }
        "ScaleXValueEffect" => decode_as::<wire::WireScaleXValueEffect>(payload).map(Some),
        _ => Ok(None),
    }
}

pub(super) fn map_card_ids(
    kind: &str,
    payload: Value,
    context: &super::card_graph::Context<'_>,
) -> Result<Option<Value>, String> {
    match kind {
        "CantEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::CantEffect>(payload, context)
                .map(Some)
        }
        "ChooseNewTargetsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseNewTargetsEffect,
        >(payload, context)
        .map(Some),
        "CopySpellEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::CopySpellEffect>(payload, context)
                .map(Some)
        }
        "CopySpellForEachTargetEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::CopySpellForEachTargetEffect,
        >(payload, context)
        .map(Some),
        "CounterEffect" => {
            validated_counter_effect(payload.clone())?;
            super::card_graph::map_payload_as::<ironsmith_core::CounterEffect>(payload, context)
                .map(Some)
        }
        "ExileTaggedWhenSourceLeavesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExileTaggedWhenSourceLeavesEffect,
        >(payload, context)
        .map(Some),
        "RegisterDamagedBySourceZoneReplacementEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterDamagedBySourceZoneReplacementEffect,
        >(payload, context)
        .map(Some),
        "RegisterDrawReplacementEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterDrawReplacementEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "RegisterEnterTappedReplacementEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterEnterTappedReplacementEffect,
        >(payload, context)
        .map(Some),
        "RegisterEnterUnderControlReplacementEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterEnterUnderControlReplacementEffect,
        >(payload, context)
        .map(Some),
        "RegisterFutureZoneReplacementEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterFutureZoneReplacementEffect,
        >(payload, context)
        .map(Some),
        "RegisterManaRewriteEffect" => super::card_graph::map_payload_as::<ironsmith_core::RegisterManaRewriteEffect>(payload, context).map(Some),
        "RegisterManaSpendPermissionEffect" => super::card_graph::map_payload_as::<ironsmith_core::RegisterManaSpendPermissionEffect>(payload, context).map(Some),
        "RegisterManaReplacementEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterManaReplacementEffect,
        >(payload, context)
        .map(Some),
        "RegisterDamageMultiplierEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterDamageMultiplierEffect,
        >(payload, context)
        .map(Some),
        "RegisterDamageAdditionEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterDamageAdditionEffect,
        >(payload, context)
        .map(Some),
        "RegisterCounterPlacementReplacementEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterCounterPlacementReplacementEffect,
        >(payload, context)
        .map(Some),
        "RegisterEnterWithCountersReplacementEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterEnterWithCountersReplacementEffect,
        >(payload, context)
        .map(Some),
        "RegisterNextBatchEnterWithCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterNextBatchEnterWithCountersEffect,
        >(payload, context)
        .map(Some),
        "RegisterZoneReplacementEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegisterZoneReplacementEffect,
        >(payload, context)
        .map(Some),
        "RetargetStackObjectEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RetargetStackObjectEffect,
        >(payload, context)
        .map(Some),
        "ScheduleDelayedTriggerEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ScheduleDelayedTriggerEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ScheduleEffectsWhenTaggedLeavesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ScheduleEffectsWhenTaggedLeavesEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "VariableCasualtyPlaneswalkerCopyEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::VariableCasualtyPlaneswalkerCopyEffect,
        >(payload, context)
        .map(Some),
        "ScaleXValueEffect" => {
            super::card_graph::map_payload_as::<wire::WireScaleXValueEffect>(payload, context)
                .map(Some)
        }
        _ => Ok(None),
    }
}
