//! Generated typed materializers for the resources runtime effect family.

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde_json::Value;

use super::{ErasedPayload, decode_as};

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
        "PutCounterOfKindChosenFromEffect" => {
            decode_as::<ironsmith_core::PutCounterOfKindChosenFromEffect>(payload).map(Some)
        }
        "PutCountersEffect" => decode_as::<ironsmith_core::PutCountersEffect>(payload).map(Some),
        "RemoveAnyCountersAmongEffect" => {
            decode_as::<ironsmith_core::RemoveAnyCountersAmongEffect>(payload).map(Some)
        }
        "RemoveAnyCountersFromSourceEffect" => {
            decode_as::<ironsmith_core::RemoveAnyCountersFromSourceEffect>(payload).map(Some)
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
        "AddManaOfNotedTypeEffect" => {
            decode_as::<ironsmith_core::AddManaOfNotedTypeEffect>(payload).map(Some)
        }
        "NoteActivationManaTypeEffect" => {
            decode_as::<ironsmith_core::NoteActivationManaTypeEffect>(payload).map(Some)
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
        "AddManaEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::AddManaEffect>(payload, context)
                .map(Some)
        }
        "AddManaFromCommanderColorIdentityEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddManaFromCommanderColorIdentityEffect,
        >(payload, context)
        .map(Some),
        "AddManaOfAnyColorEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddManaOfAnyColorEffect,
        >(payload, context)
        .map(Some),
        "AddManaOfAnyOneColorEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddManaOfAnyOneColorEffect,
        >(payload, context)
        .map(Some),
        "AddManaOfChosenColorEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddManaOfChosenColorEffect,
        >(payload, context)
        .map(Some),
        "AddManaOfColorsAmongEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddManaOfColorsAmongEffect,
        >(payload, context)
        .map(Some),
        "AddManaOfImprintedColorsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddManaOfImprintedColorsEffect,
        >(payload, context)
        .map(Some),
        "AddManaOfLandProducedTypesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddManaOfLandProducedTypesEffect,
        >(payload, context)
        .map(Some),
        "AddOneManaOfAnyColorAmongEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddOneManaOfAnyColorAmongEffect,
        >(payload, context)
        .map(Some),
        "AddScaledManaEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddScaledManaEffect,
        >(payload, context)
        .map(Some),
        "DoubleCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::DoubleCountersEffect,
        >(payload, context)
        .map(Some),
        "DoubleManaPoolEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::DoubleManaPoolEffect,
        >(payload, context)
        .map(Some),
        "EmptyManaPoolEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::EmptyManaPoolEffect,
        >(payload, context)
        .map(Some),
        "ExchangeLifeTotalsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExchangeLifeTotalsEffect,
        >(payload, context)
        .map(Some),
        "ForEachCounterKindPutOrRemoveEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ForEachCounterKindPutOrRemoveEffect,
        >(payload, context)
        .map(Some),
        "GainLifeEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::GainLifeEffect>(payload, context)
                .map(Some)
        }
        "LoseLifeEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::LoseLifeEffect>(payload, context)
                .map(Some)
        }
        "MoveAllCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::MoveAllCountersEffect,
        >(payload, context)
        .map(Some),
        "MoveCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::MoveCountersEffect,
        >(payload, context)
        .map(Some),
        "MoveOneCounterEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::MoveOneCounterEffect,
        >(payload, context)
        .map(Some),
        "NoteLifeTotalEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::NoteLifeTotalEffect,
        >(payload, context)
        .map(Some),
        "PayLifeEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PayLifeEffect>(payload, context)
                .map(Some)
        }
        "PayManaEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PayManaEffect>(payload, context)
                .map(Some)
        }
        "ProliferateEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ProliferateEffect>(payload, context)
                .map(Some)
        }
        "PutCounterOfChosenKindEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PutCounterOfChosenKindEffect,
        >(payload, context)
        .map(Some),
        "PutCounterOfKindChosenFromEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PutCounterOfKindChosenFromEffect,
        >(payload, context)
        .map(Some),
        "PutCountersEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PutCountersEffect>(payload, context)
                .map(Some)
        }
        "RemoveAnyCountersAmongEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RemoveAnyCountersAmongEffect,
        >(payload, context)
        .map(Some),
        "RemoveAnyCountersFromSourceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RemoveAnyCountersFromSourceEffect,
        >(payload, context)
        .map(Some),
        "RemoveCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RemoveCountersEffect,
        >(payload, context)
        .map(Some),
        "RemoveUpToAnyCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RemoveUpToAnyCountersEffect,
        >(payload, context)
        .map(Some),
        "RemoveUpToCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RemoveUpToCountersEffect,
        >(payload, context)
        .map(Some),
        "RetainManaUntilEndOfTurnEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RetainManaUntilEndOfTurnEffect,
        >(payload, context)
        .map(Some),
        "SetLifeTotalEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SetLifeTotalEffect,
        >(payload, context)
        .map(Some),
        "AddManaOfNotedTypeEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AddManaOfNotedTypeEffect,
        >(payload, context)
        .map(Some),
        "NoteActivationManaTypeEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::NoteActivationManaTypeEffect,
        >(payload, context)
        .map(Some),
        _ => Ok(None),
    }
}
