//! Generated typed materializers for the zone-library runtime effect family.

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
        _ => Ok(None),
    }
}
