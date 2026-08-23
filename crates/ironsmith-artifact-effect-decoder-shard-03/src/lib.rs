//! Generated typed materializers for the permanent runtime effect family.

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
        "AmassEffect" => decode_as::<ironsmith_core::AmassEffect>(payload).map(Some),
        "ApplyContinuousEffect" => decode_as::<
            ironsmith_core::ApplyContinuousEffect<
                wire::WireContinuousTarget,
                wire::WireContinuousModification,
                wire::WireRuntimeModification,
                ironsmith_core::Condition,
            >,
        >(payload)
        .map(Some),
        "AttachObjectsEffect" => {
            decode_as::<ironsmith_core::AttachObjectsEffect>(payload).map(Some)
        }
        "AttachToEffect" => decode_as::<ironsmith_core::AttachToEffect>(payload).map(Some),
        "BecomeBasicLandTypeChoiceEffect" => {
            decode_as::<ironsmith_core::BecomeBasicLandTypeChoiceEffect>(payload).map(Some)
        }
        "BecomeColorChoiceEffect" => {
            decode_as::<ironsmith_core::BecomeColorChoiceEffect>(payload).map(Some)
        }
        "BecomeCreatureTypeChoiceEffect" => {
            decode_as::<ironsmith_core::BecomeCreatureTypeChoiceEffect>(payload).map(Some)
        }
        "BecomeSaddledUntilEotEffect" => {
            decode_as::<ironsmith_core::BecomeSaddledUntilEotEffect>(payload).map(Some)
        }
        "ClearSuspectedEffect" => {
            decode_as::<ironsmith_core::ClearSuspectedEffect>(payload).map(Some)
        }
        "ConspireCostEffect" => decode_as::<ironsmith_core::ConspireCostEffect>(payload).map(Some),
        "ConvertEffect" => decode_as::<ironsmith_core::ConvertEffect>(payload).map(Some),
        "CreateTokenCopyEffect" => {
            decode_as::<ironsmith_core::CreateTokenCopyEffect<wire::WireStaticAbility>>(payload)
                .map(Some)
        }
        "CreateTokenEffect" => {
            decode_as::<ironsmith_core::CreateTokenEffect<wire::WireCardDefinition>>(payload)
                .map(Some)
        }
        "CrewCostEffect" => decode_as::<ironsmith_core::CrewCostEffect>(payload).map(Some),
        "DetainEffect" => decode_as::<ironsmith_core::DetainEffect>(payload).map(Some),
        "DirectionalAdjacentPlayerControlEffect" => {
            decode_as::<ironsmith_core::DirectionalAdjacentPlayerControlEffect>(payload).map(Some)
        }
        "EarthbendEffect" => decode_as::<ironsmith_core::EarthbendEffect>(payload).map(Some),
        "EvolveEffect" => decode_as::<ironsmith_core::EvolveEffect>(payload).map(Some),
        "ExchangeControlEffect" => {
            decode_as::<ironsmith_core::ExchangeControlEffect>(payload).map(Some)
        }
        "ExchangeTextBoxesEffect" => {
            decode_as::<ironsmith_core::ExchangeTextBoxesEffect>(payload).map(Some)
        }
        "ExertCostEffect" => decode_as::<ironsmith_core::ExertCostEffect>(payload).map(Some),
        "FlipEffect" => decode_as::<ironsmith_core::FlipEffect>(payload).map(Some),
        "IncubateEffect" => decode_as::<ironsmith_core::IncubateEffect>(payload).map(Some),
        "InvestigateEffect" => decode_as::<ironsmith_core::InvestigateEffect>(payload).map(Some),
        "MeldEffect" => decode_as::<ironsmith_core::MeldEffect>(payload).map(Some),
        "MonstrosityEffect" => decode_as::<ironsmith_core::MonstrosityEffect>(payload).map(Some),
        "NinjutsuCostEffect" => decode_as::<ironsmith_core::NinjutsuCostEffect>(payload).map(Some),
        "NinjutsuEffect" => decode_as::<ironsmith_core::NinjutsuEffect>(payload).map(Some),
        "PhaseInEffect" => decode_as::<ironsmith_core::PhaseInEffect>(payload).map(Some),
        "PhaseOutEffect" => decode_as::<ironsmith_core::PhaseOutEffect>(payload).map(Some),
        "PutStickerEffect" => decode_as::<ironsmith_core::PutStickerEffect>(payload).map(Some),
        "ReconfigureEffect" => decode_as::<ironsmith_core::ReconfigureEffect>(payload).map(Some),
        "RegenerateEffect" => {
            decode_as::<ironsmith_core::RegenerateEffect<wire::WireEffect>>(payload).map(Some)
        }
        "RenownEffect" => decode_as::<ironsmith_core::RenownEffect>(payload).map(Some),
        "SneakCostEffect" => decode_as::<ironsmith_core::SneakCostEffect>(payload).map(Some),
        "SolveCaseEffect" => decode_as::<ironsmith_core::SolveCaseEffect>(payload).map(Some),
        "SoulbondPairEffect" => decode_as::<ironsmith_core::SoulbondPairEffect>(payload).map(Some),
        "SuspectEffect" => decode_as::<ironsmith_core::SuspectEffect>(payload).map(Some),
        "TapEffect" => decode_as::<ironsmith_core::TapEffect>(payload).map(Some),
        "TransformEffect" => decode_as::<ironsmith_core::TransformEffect>(payload).map(Some),
        "TurnFaceUpEffect" => decode_as::<ironsmith_core::TurnFaceUpEffect>(payload).map(Some),
        "UnattachObjectsEffect" => {
            decode_as::<ironsmith_core::UnattachObjectsEffect>(payload).map(Some)
        }
        "UnearthEffect" => decode_as::<ironsmith_core::UnearthEffect>(payload).map(Some),
        "UnlockRoomDoorEffect" => {
            decode_as::<ironsmith_core::UnlockRoomDoorEffect>(payload).map(Some)
        }
        "UntapEffect" => decode_as::<ironsmith_core::UntapEffect>(payload).map(Some),
        _ => Ok(None),
    }
}
