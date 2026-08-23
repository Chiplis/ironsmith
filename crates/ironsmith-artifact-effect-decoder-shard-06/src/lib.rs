//! Generated typed materializers for the composition-a-l runtime effect family.

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
        "AdaptEffect" => decode_as::<ironsmith_core::AdaptEffect>(payload).map(Some),
        "AmplifyEffect" => decode_as::<ironsmith_core::AmplifyEffect>(payload).map(Some),
        "AuraSwapEffect" => decode_as::<ironsmith_core::AuraSwapEffect>(payload).map(Some),
        "BackupEffect" => {
            decode_as::<ironsmith_core::BackupEffect<wire::WireAbility>>(payload).map(Some)
        }
        "BeholdEffect" => decode_as::<ironsmith_core::BeholdEffect>(payload).map(Some),
        "BidLifeEffect" => {
            decode_as::<ironsmith_core::BidLifeEffect<wire::WireEffect>>(payload).map(Some)
        }
        "BolsterEffect" => decode_as::<ironsmith_core::BolsterEffect>(payload).map(Some),
        "ChooseModeEffect" => {
            decode_as::<ironsmith_core::ChooseModeEffect<wire::WireEffect>>(payload).map(Some)
        }
        "ChooseObjectsEffect" => {
            decode_as::<ironsmith_core::ChooseObjectsEffect>(payload).map(Some)
        }
        "ChooseSpellCastHistoryEffect" => {
            decode_as::<ironsmith_core::ChooseSpellCastHistoryEffect>(payload).map(Some)
        }
        "CipherEffect" => decode_as::<ironsmith_core::CipherEffect>(payload).map(Some),
        "ConditionalEffect" => {
            decode_as::<ironsmith_core::ConditionalEffect<wire::WireEffect>>(payload).map(Some)
        }
        "CumulativeUpkeepEffect" => {
            decode_as::<ironsmith_core::CumulativeUpkeepEffect<wire::WireEffect>>(payload).map(Some)
        }
        "DevourEffect" => decode_as::<ironsmith_core::DevourEffect>(payload).map(Some),
        "EmitGiftGivenEffect" => {
            decode_as::<ironsmith_core::EmitGiftGivenEffect>(payload).map(Some)
        }
        "EmitKeywordActionEffect" => {
            decode_as::<ironsmith_core::EmitKeywordActionEffect>(payload).map(Some)
        }
        "ExecuteWithSourceEffect" => {
            decode_as::<ironsmith_core::ExecuteWithSourceEffect<wire::WireEffect>>(payload)
                .map(Some)
        }
        "ExploreEffect" => decode_as::<ironsmith_core::ExploreEffect>(payload).map(Some),
        "ForEachControllerOfTaggedEffect" => {
            decode_as::<ironsmith_core::ForEachControllerOfTaggedEffect<wire::WireEffect>>(payload)
                .map(Some)
        }
        "ForEachObject" => {
            decode_as::<ironsmith_core::ForEachObject<wire::WireEffect>>(payload).map(Some)
        }
        "ForEachObjectCorrelatedResultEffect" => decode_as::<
            ironsmith_core::ForEachObjectCorrelatedResultEffect<wire::WireEffect>,
        >(payload)
        .map(Some),
        "ForEachTaggedEffect" => {
            decode_as::<ironsmith_core::ForEachTaggedEffect<wire::WireEffect>>(payload).map(Some)
        }
        "ForEachTaggedPlayerEffect" => {
            decode_as::<ironsmith_core::ForEachTaggedPlayerEffect<wire::WireEffect>>(payload)
                .map(Some)
        }
        "ForPlayersEffect" => {
            decode_as::<ironsmith_core::ForPlayersEffect<wire::WireEffect>>(payload).map(Some)
        }
        "GrantRepeatableManaPaymentActionUntilEndOfTurnEffect" => decode_as::<
            ironsmith_core::GrantRepeatableManaPaymentActionUntilEndOfTurnEffect<wire::WireEffect>,
        >(payload)
        .map(Some),
        "IfEffect" => decode_as::<ironsmith_core::IfEffect<wire::WireEffect>>(payload).map(Some),
        "LocalRewriteEffect" => {
            decode_as::<ironsmith_core::LocalRewriteEffect<wire::WireEffect>>(payload).map(Some)
        }
        _ => Ok(None),
    }
}
