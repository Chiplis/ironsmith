//! Generated typed materializers for the resources runtime effect family.

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
        "AddManaEffect" => decode_as::<ironsmith_core::AddManaEffect>(payload).map(Some),
        "AddManaFromCommanderColorIdentityEffect" => {
            decode_as::<ironsmith_core::AddManaFromCommanderColorIdentityEffect>(payload).map(Some)
        }
        "AddManaOfAnyColorEffect" => {
            decode_as::<ironsmith_core::AddManaOfAnyColorEffect>(payload).map(Some)
        }
        "AddManaOfAnyOneColorEffect" => {
            decode_as::<ironsmith_core::AddManaOfAnyOneColorEffect>(payload).map(Some)
        }
        "AddManaOfChosenColorEffect" => {
            decode_as::<ironsmith_core::AddManaOfChosenColorEffect>(payload).map(Some)
        }
        "AddManaOfColorsAmongEffect" => {
            decode_as::<ironsmith_core::AddManaOfColorsAmongEffect>(payload).map(Some)
        }
        "AddManaOfImprintedColorsEffect" => {
            decode_as::<ironsmith_core::AddManaOfImprintedColorsEffect>(payload).map(Some)
        }
        "AddManaOfLandProducedTypesEffect" => {
            decode_as::<ironsmith_core::AddManaOfLandProducedTypesEffect>(payload).map(Some)
        }
        "AddOneManaOfAnyColorAmongEffect" => {
            decode_as::<ironsmith_core::AddOneManaOfAnyColorAmongEffect>(payload).map(Some)
        }
        "AddScaledManaEffect" => {
            decode_as::<ironsmith_core::AddScaledManaEffect>(payload).map(Some)
        }
        "DoubleCountersEffect" => {
            decode_as::<ironsmith_core::DoubleCountersEffect>(payload).map(Some)
        }
        "DoubleManaPoolEffect" => {
            decode_as::<ironsmith_core::DoubleManaPoolEffect>(payload).map(Some)
        }
        "EmptyManaPoolEffect" => {
            decode_as::<ironsmith_core::EmptyManaPoolEffect>(payload).map(Some)
        }
        "ExchangeLifeTotalsEffect" => {
            decode_as::<ironsmith_core::ExchangeLifeTotalsEffect>(payload).map(Some)
        }
        "ForEachCounterKindPutOrRemoveEffect" => {
            decode_as::<ironsmith_core::ForEachCounterKindPutOrRemoveEffect>(payload).map(Some)
        }
        "GainLifeEffect" => decode_as::<ironsmith_core::GainLifeEffect>(payload).map(Some),
        "LoseLifeEffect" => decode_as::<ironsmith_core::LoseLifeEffect>(payload).map(Some),
        "MoveAllCountersEffect" => {
            decode_as::<ironsmith_core::MoveAllCountersEffect>(payload).map(Some)
        }
        "MoveCountersEffect" => decode_as::<ironsmith_core::MoveCountersEffect>(payload).map(Some),
        "MoveOneCounterEffect" => {
            decode_as::<ironsmith_core::MoveOneCounterEffect>(payload).map(Some)
        }
        "NoteLifeTotalEffect" => {
            decode_as::<ironsmith_core::NoteLifeTotalEffect>(payload).map(Some)
        }
        "PayLifeEffect" => decode_as::<ironsmith_core::PayLifeEffect>(payload).map(Some),
        "PayManaEffect" => decode_as::<ironsmith_core::PayManaEffect>(payload).map(Some),
        "ProliferateEffect" => decode_as::<ironsmith_core::ProliferateEffect>(payload).map(Some),
        "PutCounterOfChosenKindEffect" => {
            decode_as::<ironsmith_core::PutCounterOfChosenKindEffect>(payload).map(Some)
        }
        "PutCountersEffect" => decode_as::<ironsmith_core::PutCountersEffect>(payload).map(Some),
        "RemoveAnyCountersAmongEffect" => {
            decode_as::<ironsmith_core::RemoveAnyCountersAmongEffect>(payload).map(Some)
        }
        "RemoveCountersEffect" => {
            decode_as::<ironsmith_core::RemoveCountersEffect>(payload).map(Some)
        }
        "RemoveUpToAnyCountersEffect" => {
            decode_as::<ironsmith_core::RemoveUpToAnyCountersEffect>(payload).map(Some)
        }
        "RemoveUpToCountersEffect" => {
            decode_as::<ironsmith_core::RemoveUpToCountersEffect>(payload).map(Some)
        }
        "RetainManaUntilEndOfTurnEffect" => {
            decode_as::<ironsmith_core::RetainManaUntilEndOfTurnEffect>(payload).map(Some)
        }
        "SetLifeTotalEffect" => decode_as::<ironsmith_core::SetLifeTotalEffect>(payload).map(Some),
        _ => Ok(None),
    }
}
