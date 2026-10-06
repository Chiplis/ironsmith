//! Generated typed materializers for the composition-a-l runtime effect family.

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde_json::Value;

use super::{ErasedPayload, decode_as};

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
        "CollectManaPaymentsEffect" => {
            decode_as::<ironsmith_core::CollectManaPaymentsEffect<wire::WireEffect>>(payload).map(Some)
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

pub(super) fn map_card_ids(
    kind: &str,
    payload: Value,
    context: &super::card_graph::Context<'_>,
) -> Result<Option<Value>, String> {
    match kind {
        "AdaptEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::AdaptEffect>(payload, context)
                .map(Some)
        }
        "AmplifyEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::AmplifyEffect>(payload, context)
                .map(Some)
        }
        "AuraSwapEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::AuraSwapEffect>(payload, context)
                .map(Some)
        }
        "BackupEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::BackupEffect<wire::WireAbility>,
        >(payload, context)
        .map(Some),
        "BeholdEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::BeholdEffect>(payload, context)
                .map(Some)
        }
        "BidLifeEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::BidLifeEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "BolsterEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::BolsterEffect>(payload, context)
                .map(Some)
        }
        "ChooseModeEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseModeEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ChooseObjectsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseObjectsEffect,
        >(payload, context)
        .map(Some),
        "ChooseSpellCastHistoryEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseSpellCastHistoryEffect,
        >(payload, context)
        .map(Some),
        "CipherEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::CipherEffect>(payload, context)
                .map(Some)
        }
        "ConditionalEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ConditionalEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "CumulativeUpkeepEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::CumulativeUpkeepEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "DevourEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::DevourEffect>(payload, context)
                .map(Some)
        }
        "EmitGiftGivenEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::EmitGiftGivenEffect,
        >(payload, context)
        .map(Some),
        "EmitKeywordActionEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::EmitKeywordActionEffect,
        >(payload, context)
        .map(Some),
        "ExecuteWithSourceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExecuteWithSourceEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ExploreEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ExploreEffect>(payload, context)
                .map(Some)
        }
        "ForEachControllerOfTaggedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ForEachControllerOfTaggedEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ForEachObject" => super::card_graph::map_payload_as::<
            ironsmith_core::ForEachObject<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ForEachObjectCorrelatedResultEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ForEachObjectCorrelatedResultEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ForEachTaggedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ForEachTaggedEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "ForEachTaggedPlayerEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ForEachTaggedPlayerEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "CollectManaPaymentsEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::CollectManaPaymentsEffect<wire::WireEffect>,
        >(payload, context).map(Some),
        "ForPlayersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ForPlayersEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "GrantRepeatableManaPaymentActionUntilEndOfTurnEffect" => {
            super::card_graph::map_payload_as::<
                ironsmith_core::GrantRepeatableManaPaymentActionUntilEndOfTurnEffect<
                    wire::WireEffect,
                >,
            >(payload, context)
            .map(Some)
        }
        "IfEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::IfEffect<wire::WireEffect>>(
                payload, context,
            )
            .map(Some)
        }
        "LocalRewriteEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::LocalRewriteEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        _ => Ok(None),
    }
}
