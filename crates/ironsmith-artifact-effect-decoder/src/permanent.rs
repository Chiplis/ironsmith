//! Generated typed materializers for the permanent runtime effect family.

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde_json::Value;

use super::{ErasedPayload, decode_as};

pub fn decode(kind: &str, payload: Value) -> Result<Option<ErasedPayload>, String> {
    match kind {
        "AmassEffect" => decode_as::<ironsmith_core::AmassEffect>(payload).map(Some),
        "EmpowerJaceEffect" => decode_as::<ironsmith_core::EmpowerJaceEffect>(payload).map(Some),
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
        "PrepareEffect" => decode_as::<ironsmith_core::PrepareEffect>(payload).map(Some),
        "PutStickerEffect" => decode_as::<ironsmith_core::PutStickerEffect>(payload).map(Some),
        "ReconfigureEffect" => decode_as::<ironsmith_core::ReconfigureEffect>(payload).map(Some),
        "RegenerateEffect" => {
            decode_as::<ironsmith_core::RegenerateEffect<wire::WireEffect>>(payload).map(Some)
        }
        "RenownEffect" => decode_as::<ironsmith_core::RenownEffect>(payload).map(Some),
        "SneakCostEffect" => decode_as::<ironsmith_core::SneakCostEffect>(payload).map(Some),
        "SolveCaseEffect" => decode_as::<ironsmith_core::SolveCaseEffect>(payload).map(Some),
        "SetClassLevelEffect" => {
            decode_as::<ironsmith_core::SetClassLevelEffect>(payload).map(Some)
        }
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
        "SaddleCostEffect" => decode_as::<ironsmith_core::SaddleCostEffect>(payload).map(Some),
        _ => Ok(None),
    }
}

pub(super) fn map_card_ids(
    kind: &str,
    payload: Value,
    context: &super::card_graph::Context<'_>,
) -> Result<Option<Value>, String> {
    match kind {
        "EmpowerJaceEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::EmpowerJaceEffect>(payload, context)
                .map(Some)
        }
        "AmassEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::AmassEffect>(payload, context)
                .map(Some)
        }
        "ApplyContinuousEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ApplyContinuousEffect<
                wire::WireContinuousTarget,
                wire::WireContinuousModification,
                wire::WireRuntimeModification,
                ironsmith_core::Condition,
            >,
        >(payload, context)
        .map(Some),
        "AttachObjectsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AttachObjectsEffect,
        >(payload, context)
        .map(Some),
        "AttachToEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::AttachToEffect>(payload, context)
                .map(Some)
        }
        "BecomeBasicLandTypeChoiceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::BecomeBasicLandTypeChoiceEffect,
        >(payload, context)
        .map(Some),
        "BecomeColorChoiceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::BecomeColorChoiceEffect,
        >(payload, context)
        .map(Some),
        "BecomeCreatureTypeChoiceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::BecomeCreatureTypeChoiceEffect,
        >(payload, context)
        .map(Some),
        "BecomeSaddledUntilEotEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::BecomeSaddledUntilEotEffect,
        >(payload, context)
        .map(Some),
        "ClearSuspectedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ClearSuspectedEffect,
        >(payload, context)
        .map(Some),
        "ConspireCostEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ConspireCostEffect,
        >(payload, context)
        .map(Some),
        "ConvertEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ConvertEffect>(payload, context)
                .map(Some)
        }
        "CreateTokenCopyEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::CreateTokenCopyEffect<wire::WireStaticAbility>,
        >(payload, context)
        .map(Some),
        "CreateTokenEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::CreateTokenEffect<wire::WireCardDefinition>,
        >(payload, context)
        .map(Some),
        "CrewCostEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::CrewCostEffect>(payload, context)
                .map(Some)
        }
        "DetainEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::DetainEffect>(payload, context)
                .map(Some)
        }
        "DirectionalAdjacentPlayerControlEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::DirectionalAdjacentPlayerControlEffect,
        >(payload, context)
        .map(Some),
        "EarthbendEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::EarthbendEffect>(payload, context)
                .map(Some)
        }
        "EvolveEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::EvolveEffect>(payload, context)
                .map(Some)
        }
        "ExchangeControlEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExchangeControlEffect,
        >(payload, context)
        .map(Some),
        "ExchangeTextBoxesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExchangeTextBoxesEffect,
        >(payload, context)
        .map(Some),
        "ExertCostEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ExertCostEffect>(payload, context)
                .map(Some)
        }
        "FlipEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::FlipEffect>(payload, context)
                .map(Some)
        }
        "IncubateEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::IncubateEffect>(payload, context)
                .map(Some)
        }
        "InvestigateEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::InvestigateEffect>(payload, context)
                .map(Some)
        }
        "MeldEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::MeldEffect>(payload, context)
                .map(Some)
        }
        "MonstrosityEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::MonstrosityEffect>(payload, context)
                .map(Some)
        }
        "NinjutsuCostEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::NinjutsuCostEffect,
        >(payload, context)
        .map(Some),
        "NinjutsuEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::NinjutsuEffect>(payload, context)
                .map(Some)
        }
        "PhaseInEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PhaseInEffect>(payload, context)
                .map(Some)
        }
        "PhaseOutEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PhaseOutEffect>(payload, context)
                .map(Some)
        }
        "PrepareEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PrepareEffect>(payload, context)
                .map(Some)
        }
        "PutStickerEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PutStickerEffect>(payload, context)
                .map(Some)
        }
        "ReconfigureEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ReconfigureEffect>(payload, context)
                .map(Some)
        }
        "RegenerateEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RegenerateEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "RenownEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::RenownEffect>(payload, context)
                .map(Some)
        }
        "SneakCostEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::SneakCostEffect>(payload, context)
                .map(Some)
        }
        "SolveCaseEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::SolveCaseEffect>(payload, context)
                .map(Some)
        }
        "SetClassLevelEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SetClassLevelEffect,
        >(payload, context)
        .map(Some),
        "SoulbondPairEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SoulbondPairEffect,
        >(payload, context)
        .map(Some),
        "SuspectEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::SuspectEffect>(payload, context)
                .map(Some)
        }
        "TapEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::TapEffect>(payload, context)
                .map(Some)
        }
        "TransformEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::TransformEffect>(payload, context)
                .map(Some)
        }
        "TurnFaceUpEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::TurnFaceUpEffect>(payload, context)
                .map(Some)
        }
        "UnattachObjectsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::UnattachObjectsEffect,
        >(payload, context)
        .map(Some),
        "UnearthEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::UnearthEffect>(payload, context)
                .map(Some)
        }
        "UnlockRoomDoorEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::UnlockRoomDoorEffect,
        >(payload, context)
        .map(Some),
        "UntapEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::UntapEffect>(payload, context)
                .map(Some)
        }
        "SaddleCostEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::SaddleCostEffect>(payload, context)
                .map(Some)
        }
        _ => Ok(None),
    }
}
