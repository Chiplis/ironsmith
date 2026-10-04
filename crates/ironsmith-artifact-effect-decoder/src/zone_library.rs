//! Generated typed materializers for the zone-library runtime effect family.

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde_json::Value;

use super::{ErasedPayload, decode_as};

pub fn decode(kind: &str, payload: Value) -> Result<Option<ErasedPayload>, String> {
    match kind {
        "CollectEvidenceEffect" => decode_as::<ironsmith_core::CollectEvidenceEffect>(payload).map(Some),
        "ClashEffect" => decode_as::<ironsmith_core::ClashEffect>(payload).map(Some),
        "ConniveEffect" => decode_as::<ironsmith_core::ConniveEffect>(payload).map(Some),
        "ConsultTopOfLibraryEffect" => {
            decode_as::<ironsmith_core::ConsultTopOfLibraryEffect>(payload).map(Some)
        }
        "DestroyEffect" => decode_as::<ironsmith_core::DestroyEffect>(payload).map(Some),
        "DestroyNoRegenerationEffect" => {
            decode_as::<ironsmith_core::DestroyNoRegenerationEffect>(payload).map(Some)
        }
        "DiscardEffect" => decode_as::<ironsmith_core::DiscardEffect>(payload).map(Some),
        "DiscardHandEffect" => decode_as::<ironsmith_core::DiscardHandEffect>(payload).map(Some),
        "DrawCardsEffect" => decode_as::<ironsmith_core::DrawCardsEffect>(payload).map(Some),
        "DrawForEachTaggedMatchingEffect" => {
            decode_as::<ironsmith_core::DrawForEachTaggedMatchingEffect>(payload).map(Some)
        }
        "EachPlayerScryEffect" => {
            decode_as::<ironsmith_core::EachPlayerScryEffect>(payload).map(Some)
        }
        "ExchangeZonesEffect" => {
            decode_as::<ironsmith_core::ExchangeZonesEffect>(payload).map(Some)
        }
        "ExileEffect" => decode_as::<ironsmith_core::ExileEffect>(payload).map(Some),
        "ExileTopOfLibraryEffect" => {
            decode_as::<ironsmith_core::ExileTopOfLibraryEffect>(payload).map(Some)
        }
        "ExileUntilEffect" => decode_as::<ironsmith_core::ExileUntilEffect>(payload).map(Some),
        "FatesealEffect" => decode_as::<ironsmith_core::FatesealEffect>(payload).map(Some),
        "HauntExileEffect" => {
            decode_as::<ironsmith_core::HauntExileEffect<wire::WireEffect>>(payload).map(Some)
        }
        "LearnEffect" => decode_as::<ironsmith_core::LearnEffect>(payload).map(Some),
        "LookAtHandEffect" => decode_as::<ironsmith_core::LookAtHandEffect>(payload).map(Some),
        "LookAtObjectsEffect" => {
            decode_as::<ironsmith_core::LookAtObjectsEffect>(payload).map(Some)
        }
        "LookAtTopCardsEffect" => {
            decode_as::<ironsmith_core::LookAtTopCardsEffect>(payload).map(Some)
        }
        "MayMoveToZoneEffect" => {
            decode_as::<ironsmith_core::MayMoveToZoneEffect>(payload).map(Some)
        }
        "MillEffect" => decode_as::<ironsmith_core::MillEffect>(payload).map(Some),
        "MoveToLibraryNthFromTopEffect" => {
            decode_as::<ironsmith_core::MoveToLibraryNthFromTopEffect>(payload).map(Some)
        }
        "MoveToLibraryTopOrBottomChoiceEffect" => {
            decode_as::<ironsmith_core::MoveToLibraryTopOrBottomChoiceEffect>(payload).map(Some)
        }
        "MoveToZoneEffect" => decode_as::<ironsmith_core::MoveToZoneEffect>(payload).map(Some),
        "PutOntoBattlefieldEffect" => {
            decode_as::<ironsmith_core::PutOntoBattlefieldEffect>(payload).map(Some)
        }
        "PutTaggedRemainderOnLibraryBottomEffect" => {
            decode_as::<ironsmith_core::PutTaggedRemainderOnLibraryBottomEffect>(payload).map(Some)
        }
        "RearrangeLookedCardsInLibraryEffect" => {
            decode_as::<ironsmith_core::RearrangeLookedCardsInLibraryEffect>(payload).map(Some)
        }
        "ReorderGraveyardEffect" => {
            decode_as::<ironsmith_core::ReorderGraveyardEffect>(payload).map(Some)
        }
        "ReorderLibraryTopEffect" => {
            decode_as::<ironsmith_core::ReorderLibraryTopEffect>(payload).map(Some)
        }
        "ReorderTopPlanarDeckEffect" => {
            decode_as::<ironsmith_core::ReorderTopPlanarDeckEffect>(payload).map(Some)
        }
        "ReturnAllToBattlefieldEffect" => {
            decode_as::<ironsmith_core::ReturnAllToBattlefieldEffect>(payload).map(Some)
        }
        "ReturnFromGraveyardOrExileToBattlefieldEffect" => {
            decode_as::<ironsmith_core::ReturnFromGraveyardOrExileToBattlefieldEffect>(payload)
                .map(Some)
        }
        "ReturnFromGraveyardToBattlefieldEffect" => {
            decode_as::<ironsmith_core::ReturnFromGraveyardToBattlefieldEffect>(payload).map(Some)
        }
        "ReturnFromGraveyardToHandEffect" => {
            decode_as::<ironsmith_core::ReturnFromGraveyardToHandEffect>(payload).map(Some)
        }
        "ReturnToHandEffect" => decode_as::<ironsmith_core::ReturnToHandEffect>(payload).map(Some),
        "RevealFromHandEffect" => {
            decode_as::<ironsmith_core::RevealFromHandEffect>(payload).map(Some)
        }
        "RevealSourceFromHandEffect" => {
            decode_as::<ironsmith_core::RevealSourceFromHandEffect>(payload).map(Some)
        }
        "RevealTaggedEffect" => decode_as::<ironsmith_core::RevealTaggedEffect>(payload).map(Some),
        "RevealTopEffect" => decode_as::<ironsmith_core::RevealTopEffect>(payload).map(Some),
        "SacrificeEffect" => decode_as::<ironsmith_core::SacrificeEffect>(payload).map(Some),
        "SacrificePlayerEffect" => {
            decode_as::<ironsmith_core::SacrificePlayerEffect>(payload).map(Some)
        }
        "SacrificeTargetEffect" => {
            decode_as::<ironsmith_core::SacrificeTargetEffect>(payload).map(Some)
        }
        "ScryEffect" => decode_as::<ironsmith_core::ScryEffect>(payload).map(Some),
        "SearchLibraryEffect" => {
            decode_as::<ironsmith_core::SearchLibraryEffect>(payload).map(Some)
        }
        "SearchLibrarySlotsEffect" => {
            decode_as::<ironsmith_core::SearchLibrarySlotsEffect>(payload).map(Some)
        }
        "ShuffleGraveyardIntoLibraryEffect" => {
            decode_as::<ironsmith_core::ShuffleGraveyardIntoLibraryEffect>(payload).map(Some)
        }
        "ShuffleHandAndGraveyardIntoLibraryEffect" => {
            decode_as::<ironsmith_core::ShuffleHandAndGraveyardIntoLibraryEffect>(payload).map(Some)
        }
        "ShuffleLibraryEffect" => {
            decode_as::<ironsmith_core::ShuffleLibraryEffect>(payload).map(Some)
        }
        "ShuffleObjectsIntoLibraryEffect" => {
            decode_as::<ironsmith_core::ShuffleObjectsIntoLibraryEffect>(payload).map(Some)
        }
        "SurveilEffect" => decode_as::<ironsmith_core::SurveilEffect>(payload).map(Some),
        "ImprintFromHandEffect" => decode_as::<wire::WireImprintFromHandEffect>(payload).map(Some),
        "BecomePlottedEffect" => {
            decode_as::<ironsmith_core::BecomePlottedEffect>(payload).map(Some)
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
        "CollectEvidenceEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::CollectEvidenceEffect>(payload, context).map(Some)
        }
        "ClashEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ClashEffect>(payload, context)
                .map(Some)
        }
        "ConniveEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ConniveEffect>(payload, context)
                .map(Some)
        }
        "ConsultTopOfLibraryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ConsultTopOfLibraryEffect,
        >(payload, context)
        .map(Some),
        "DestroyEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::DestroyEffect>(payload, context)
                .map(Some)
        }
        "DestroyNoRegenerationEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::DestroyNoRegenerationEffect,
        >(payload, context)
        .map(Some),
        "DiscardEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::DiscardEffect>(payload, context)
                .map(Some)
        }
        "DiscardHandEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::DiscardHandEffect>(payload, context)
                .map(Some)
        }
        "DrawCardsEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::DrawCardsEffect>(payload, context)
                .map(Some)
        }
        "DrawForEachTaggedMatchingEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::DrawForEachTaggedMatchingEffect,
        >(payload, context)
        .map(Some),
        "EachPlayerScryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::EachPlayerScryEffect,
        >(payload, context)
        .map(Some),
        "ExchangeZonesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExchangeZonesEffect,
        >(payload, context)
        .map(Some),
        "ExileEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ExileEffect>(payload, context)
                .map(Some)
        }
        "ExileTopOfLibraryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExileTopOfLibraryEffect,
        >(payload, context)
        .map(Some),
        "ExileUntilEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ExileUntilEffect>(payload, context)
                .map(Some)
        }
        "FatesealEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::FatesealEffect>(payload, context)
                .map(Some)
        }
        "HauntExileEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::HauntExileEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "LearnEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::LearnEffect>(payload, context)
                .map(Some)
        }
        "LookAtHandEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::LookAtHandEffect>(payload, context)
                .map(Some)
        }
        "LookAtObjectsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::LookAtObjectsEffect,
        >(payload, context)
        .map(Some),
        "LookAtTopCardsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::LookAtTopCardsEffect,
        >(payload, context)
        .map(Some),
        "MayMoveToZoneEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::MayMoveToZoneEffect,
        >(payload, context)
        .map(Some),
        "MillEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::MillEffect>(payload, context)
                .map(Some)
        }
        "MoveToLibraryNthFromTopEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::MoveToLibraryNthFromTopEffect,
        >(payload, context)
        .map(Some),
        "MoveToLibraryTopOrBottomChoiceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::MoveToLibraryTopOrBottomChoiceEffect,
        >(payload, context)
        .map(Some),
        "MoveToZoneEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::MoveToZoneEffect>(payload, context)
                .map(Some)
        }
        "PutOntoBattlefieldEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PutOntoBattlefieldEffect,
        >(payload, context)
        .map(Some),
        "PutTaggedRemainderOnLibraryBottomEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PutTaggedRemainderOnLibraryBottomEffect,
        >(payload, context)
        .map(Some),
        "RearrangeLookedCardsInLibraryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RearrangeLookedCardsInLibraryEffect,
        >(payload, context)
        .map(Some),
        "ReorderGraveyardEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReorderGraveyardEffect,
        >(payload, context)
        .map(Some),
        "ReorderLibraryTopEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReorderLibraryTopEffect,
        >(payload, context)
        .map(Some),
        "ReorderTopPlanarDeckEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReorderTopPlanarDeckEffect,
        >(payload, context)
        .map(Some),
        "ReturnAllToBattlefieldEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReturnAllToBattlefieldEffect,
        >(payload, context)
        .map(Some),
        "ReturnFromGraveyardOrExileToBattlefieldEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReturnFromGraveyardOrExileToBattlefieldEffect,
        >(payload, context)
        .map(Some),
        "ReturnFromGraveyardToBattlefieldEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReturnFromGraveyardToBattlefieldEffect,
        >(payload, context)
        .map(Some),
        "ReturnFromGraveyardToHandEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReturnFromGraveyardToHandEffect,
        >(payload, context)
        .map(Some),
        "ReturnToHandEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReturnToHandEffect,
        >(payload, context)
        .map(Some),
        "RevealFromHandEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RevealFromHandEffect,
        >(payload, context)
        .map(Some),
        "RevealSourceFromHandEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RevealSourceFromHandEffect,
        >(payload, context)
        .map(Some),
        "RevealTaggedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RevealTaggedEffect,
        >(payload, context)
        .map(Some),
        "RevealTopEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::RevealTopEffect>(payload, context)
                .map(Some)
        }
        "SacrificeEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::SacrificeEffect>(payload, context)
                .map(Some)
        }
        "SacrificePlayerEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SacrificePlayerEffect,
        >(payload, context)
        .map(Some),
        "SacrificeTargetEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SacrificeTargetEffect,
        >(payload, context)
        .map(Some),
        "ScryEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ScryEffect>(payload, context)
                .map(Some)
        }
        "SearchLibraryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SearchLibraryEffect,
        >(payload, context)
        .map(Some),
        "SearchLibrarySlotsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SearchLibrarySlotsEffect,
        >(payload, context)
        .map(Some),
        "ShuffleGraveyardIntoLibraryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ShuffleGraveyardIntoLibraryEffect,
        >(payload, context)
        .map(Some),
        "ShuffleHandAndGraveyardIntoLibraryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ShuffleHandAndGraveyardIntoLibraryEffect,
        >(payload, context)
        .map(Some),
        "ShuffleLibraryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ShuffleLibraryEffect,
        >(payload, context)
        .map(Some),
        "ShuffleObjectsIntoLibraryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ShuffleObjectsIntoLibraryEffect,
        >(payload, context)
        .map(Some),
        "SurveilEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::SurveilEffect>(payload, context)
                .map(Some)
        }
        "ImprintFromHandEffect" => {
            super::card_graph::map_payload_as::<wire::WireImprintFromHandEffect>(payload, context)
                .map(Some)
        }
        "BecomePlottedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::BecomePlottedEffect,
        >(payload, context)
        .map(Some),
        _ => Ok(None),
    }
}
