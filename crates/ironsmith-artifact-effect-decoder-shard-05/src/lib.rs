//! Generated typed materializers for the stack-event runtime effect family.

use std::any::Any;

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub type ErasedPayload = Box<dyn Any + Send + Sync>;

fn decode_as<D>(payload: Value) -> Result<ErasedPayload, String>
where
    D: DeserializeOwned + Send + Sync + 'static,
{
    serde_json::from_value::<D>(payload)
        .map(|value| Box::new(value) as ErasedPayload)
        .map_err(|error| error.to_string())
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
        "CounterEffect" => decode_as::<ironsmith_core::CounterEffect>(payload).map(Some),
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
        "RegisterManaReplacementEffect" => {
            decode_as::<ironsmith_core::RegisterManaReplacementEffect>(payload).map(Some)
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
