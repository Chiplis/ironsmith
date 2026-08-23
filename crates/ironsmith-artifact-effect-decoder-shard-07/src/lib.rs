//! Generated typed materializers for the composition-m-z runtime effect family.

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
        _ => Ok(None),
    }
}
