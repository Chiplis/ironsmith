//! Generated typed materializers for the composition-m-z runtime effect family.

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde_json::Value;

use super::{ErasedPayload, decode_as};

pub fn decode(kind: &str, payload: Value) -> Result<Option<ErasedPayload>, String> {
    match kind {
        "ManaRestrictedEffect" => {
            decode_as::<ironsmith_core::ManaRestrictedEffect<wire::WireEffect>>(payload).map(Some)
        }
        "ManaRetainedEffect" => {
            decode_as::<ironsmith_core::ManaRetainedEffect<wire::WireEffect>>(payload).map(Some)
        }
        "ManifestCardFromHandEffect" => {
            decode_as::<ironsmith_core::ManifestCardFromHandEffect>(payload).map(Some)
        }
        "ManifestDreadEffect" => {
            decode_as::<ironsmith_core::ManifestDreadEffect>(payload).map(Some)
        }
        "ManifestObjectsEffect" => {
            decode_as::<ironsmith_core::ManifestObjectsEffect>(payload).map(Some)
        }
        "ManifestTopCardOfLibraryEffect" => {
            decode_as::<ironsmith_core::ManifestTopCardOfLibraryEffect>(payload).map(Some)
        }
        "MayEffect" => decode_as::<ironsmith_core::MayEffect<wire::WireEffect>>(payload).map(Some),
        "OpenAttractionEffect" => {
            decode_as::<ironsmith_core::OpenAttractionEffect>(payload).map(Some)
        }
        "RollToVisitAttractionsEffect" => {
            decode_as::<ironsmith_core::RollToVisitAttractionsEffect>(payload).map(Some)
        }
        "PopulateEffect" => decode_as::<ironsmith_core::PopulateEffect>(payload).map(Some),
        "ReflexiveTriggerEffect" => {
            decode_as::<ironsmith_core::ReflexiveTriggerEffect<wire::WireEffect>>(payload).map(Some)
        }
        "RepeatEffectsEffect" => {
            decode_as::<ironsmith_core::RepeatEffectsEffect<wire::WireEffect>>(payload).map(Some)
        }
        "RepeatProcessEffect" => {
            decode_as::<ironsmith_core::RepeatProcessEffect<wire::WireEffect>>(payload).map(Some)
        }
        "RepeatProcessPromptEffect" => {
            decode_as::<ironsmith_core::RepeatProcessPromptEffect>(payload).map(Some)
        }
        "SecretChoiceEffect" => decode_as::<ironsmith_core::SecretChoiceEffect>(payload).map(Some),
        "SequenceEffect" => {
            decode_as::<ironsmith_core::SequenceEffect<wire::WireEffect>>(payload).map(Some)
        }
        "SupportEffect" => decode_as::<ironsmith_core::SupportEffect>(payload).map(Some),
        "TagAttachedToSourceEffect" => {
            decode_as::<ironsmith_core::TagAttachedToSourceEffect>(payload).map(Some)
        }
        "TagMatchingObjectsEffect" => {
            decode_as::<ironsmith_core::TagMatchingObjectsEffect>(payload).map(Some)
        }
        "TagOtherBlockParticipantEffect" => {
            decode_as::<ironsmith_core::TagOtherBlockParticipantEffect>(payload).map(Some)
        }
        "TagTriggeringAttackerEffect" => {
            decode_as::<ironsmith_core::TagTriggeringAttackerEffect>(payload).map(Some)
        }
        "TagTriggeringBlockersEffect" => {
            decode_as::<ironsmith_core::TagTriggeringBlockersEffect>(payload).map(Some)
        }
        "TagTriggeringDamageTargetEffect" => {
            decode_as::<ironsmith_core::TagTriggeringDamageTargetEffect>(payload).map(Some)
        }
        "TagTriggeringObjectEffect" => {
            decode_as::<ironsmith_core::TagTriggeringObjectEffect>(payload).map(Some)
        }
        "TagTriggeringSourceEffect" => {
            decode_as::<ironsmith_core::TagTriggeringSourceEffect>(payload).map(Some)
        }
        "TaggedEffect" => {
            decode_as::<ironsmith_core::TaggedEffect<wire::WireEffect>>(payload).map(Some)
        }
        "TargetOnlyEffect" => decode_as::<ironsmith_core::TargetOnlyEffect>(payload).map(Some),
        "UnlessActionEffect" => {
            decode_as::<ironsmith_core::UnlessActionEffect<wire::WireEffect>>(payload).map(Some)
        }
        "UnlessPaysEffect" => {
            decode_as::<ironsmith_core::UnlessPaysEffect<wire::WireEffect>>(payload).map(Some)
        }
        "VillainousChoiceEffect" => {
            decode_as::<ironsmith_core::VillainousChoiceEffect<wire::WireEffect>>(payload).map(Some)
        }
        "VoteEffect" => {
            decode_as::<ironsmith_core::VoteEffect<wire::WireEffect>>(payload).map(Some)
        }
        "WithIdEffect" => {
            decode_as::<ironsmith_core::WithIdEffect<wire::WireEffect>>(payload).map(Some)
        }
        "ResolvesDespiteIllegalTargetsEffect" => {
            decode_as::<ironsmith_core::ResolvesDespiteIllegalTargetsEffect>(payload).map(Some)
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
        "ManaRestrictedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ManaRestrictedEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ManaRetainedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ManaRetainedEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ManifestCardFromHandEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ManifestCardFromHandEffect,
        >(payload, context)
        .map(Some),
        "ManifestDreadEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ManifestDreadEffect,
        >(payload, context)
        .map(Some),
        "ManifestObjectsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ManifestObjectsEffect,
        >(payload, context)
        .map(Some),
        "ManifestTopCardOfLibraryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ManifestTopCardOfLibraryEffect,
        >(payload, context)
        .map(Some),
        "MayEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::MayEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "OpenAttractionEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::OpenAttractionEffect,
        >(payload, context)
        .map(Some),
        "RollToVisitAttractionsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RollToVisitAttractionsEffect,
        >(payload, context)
        .map(Some),
        "PopulateEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PopulateEffect>(payload, context)
                .map(Some)
        }
        "ReflexiveTriggerEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReflexiveTriggerEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "RepeatEffectsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RepeatEffectsEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "RepeatProcessEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RepeatProcessEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "RepeatProcessPromptEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RepeatProcessPromptEffect,
        >(payload, context)
        .map(Some),
        "SecretChoiceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SecretChoiceEffect,
        >(payload, context)
        .map(Some),
        "SequenceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SequenceEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "SupportEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::SupportEffect>(payload, context)
                .map(Some)
        }
        "TagAttachedToSourceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TagAttachedToSourceEffect,
        >(payload, context)
        .map(Some),
        "TagMatchingObjectsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TagMatchingObjectsEffect,
        >(payload, context)
        .map(Some),
        "TagOtherBlockParticipantEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TagOtherBlockParticipantEffect,
        >(payload, context)
        .map(Some),
        "TagTriggeringAttackerEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TagTriggeringAttackerEffect,
        >(payload, context)
        .map(Some),
        "TagTriggeringBlockersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TagTriggeringBlockersEffect,
        >(payload, context)
        .map(Some),
        "TagTriggeringDamageTargetEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TagTriggeringDamageTargetEffect,
        >(payload, context)
        .map(Some),
        "TagTriggeringObjectEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TagTriggeringObjectEffect,
        >(payload, context)
        .map(Some),
        "TagTriggeringSourceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TagTriggeringSourceEffect,
        >(payload, context)
        .map(Some),
        "TaggedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TaggedEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "TargetOnlyEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::TargetOnlyEffect>(payload, context)
                .map(Some)
        }
        "UnlessActionEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::UnlessActionEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "UnlessPaysEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::UnlessPaysEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "VillainousChoiceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::VillainousChoiceEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "VoteEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::VoteEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "WithIdEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::WithIdEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ResolvesDespiteIllegalTargetsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ResolvesDespiteIllegalTargetsEffect,
        >(payload, context)
        .map(Some),
        _ => Ok(None),
    }
}
