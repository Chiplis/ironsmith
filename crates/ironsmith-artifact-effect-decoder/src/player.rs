//! Generated typed materializers for the player runtime effect family.

#[allow(unused_imports)]
use ironsmith_compiled_artifact as wire;
use serde_json::Value;

use super::{ErasedPayload, decode_as};

pub fn decode(kind: &str, payload: Value) -> Result<Option<ErasedPayload>, String> {
    match kind {
        "AdditionalLandPlaysEffect" => {
            decode_as::<ironsmith_core::AdditionalLandPlaysEffect>(payload).map(Some)
        }
        "AdditionalPhasesEffect" => {
            decode_as::<ironsmith_core::AdditionalPhasesEffect>(payload).map(Some)
        }
        "AscendEffect" => decode_as::<ironsmith_core::AscendEffect>(payload).map(Some),
        "BecomeMonarchEffect" => {
            decode_as::<ironsmith_core::BecomeMonarchEffect>(payload).map(Some)
        }
        "CastSourceEffect" => decode_as::<ironsmith_core::CastSourceEffect>(payload).map(Some),
        "CastTaggedEffect" => decode_as::<ironsmith_core::CastTaggedEffect<wire::WireCost>>(payload).map(Some),
        "ChooseCardNameEffect" => {
            decode_as::<ironsmith_core::ChooseCardNameEffect>(payload).map(Some)
        }
        "ChooseCardTypeEffect" => {
            decode_as::<ironsmith_core::ChooseCardTypeEffect>(payload).map(Some)
        }
        "ChooseColorEffect" => decode_as::<ironsmith_core::ChooseColorEffect>(payload).map(Some),
        "ChooseCreatureTypeEffect" => {
            decode_as::<ironsmith_core::ChooseCreatureTypeEffect>(payload).map(Some)
        }
        "ChooseLandTypeEffect" => {
            decode_as::<ironsmith_core::ChooseLandTypeEffect>(payload).map(Some)
        }
        "RippleEffect" => decode_as::<ironsmith_core::RippleEffect>(payload).map(Some),
        "ChooseNumberEffect" => decode_as::<ironsmith_core::ChooseNumberEffect>(payload).map(Some),
        "ChooseNamedOptionEffect" => {
            decode_as::<ironsmith_core::ChooseNamedOptionEffect>(payload).map(Some)
        }
        "ChoosePlayerEffect" => decode_as::<ironsmith_core::ChoosePlayerEffect>(payload).map(Some),
        "ChooseFriendsOrFoesEffect" => {
            decode_as::<ironsmith_core::ChooseFriendsOrFoesEffect>(payload).map(Some)
        }
        "ControlCombatChoicesThisTurnEffect" => {
            decode_as::<ironsmith_core::ControlCombatChoicesThisTurnEffect>(payload).map(Some)
        }
        "ControlPlayerEffect" => {
            decode_as::<ironsmith_core::ControlPlayerEffect>(payload).map(Some)
        }
        "CreateEmblemEffect" => {
            decode_as::<ironsmith_core::CreateEmblemEffect<wire::WireEmblemDescription>>(payload)
                .map(Some)
        }
        "DiscoverEffect" => decode_as::<ironsmith_core::DiscoverEffect>(payload).map(Some),
        "EndCombatPhaseEffect" => {
            decode_as::<ironsmith_core::EndCombatPhaseEffect>(payload).map(Some)
        }
        "EndTurnEffect" => decode_as::<ironsmith_core::EndTurnEffect>(payload).map(Some),
        "EnergyCountersEffect" => {
            decode_as::<ironsmith_core::EnergyCountersEffect>(payload).map(Some)
        }
        "ExileInsteadOfGraveyardEffect" => {
            decode_as::<ironsmith_core::ExileInsteadOfGraveyardEffect>(payload).map(Some)
        }
        "ExperienceCountersEffect" => {
            decode_as::<ironsmith_core::ExperienceCountersEffect>(payload).map(Some)
        }
        "ExtraTurnAfterNextTurnEffect" => {
            decode_as::<ironsmith_core::ExtraTurnAfterNextTurnEffect>(payload).map(Some)
        }
        "ExtraTurnEffect" => decode_as::<ironsmith_core::ExtraTurnEffect>(payload).map(Some),
        "FlipCoinEffect" => decode_as::<ironsmith_core::FlipCoinEffect>(payload).map(Some),
        "GivePlayerCountersEffect" => {
            decode_as::<ironsmith_core::GivePlayerCountersEffect>(payload).map(Some)
        }
        "GrantBySpecEffect" => decode_as::<
            ironsmith_core::GrantBySpecEffect<wire::WireGrantSpec, wire::WireGrantDuration>,
        >(payload)
        .map(Some),
        "GrantEffect" => decode_as::<
            ironsmith_core::GrantEffect<wire::WireGrantable, wire::WireGrantDuration>,
        >(payload)
        .map(Some),
        "GrantNextSpellAbilityEffect" => {
            decode_as::<ironsmith_core::GrantNextSpellAbilityEffect<wire::WireAbility>>(payload)
                .map(Some)
        }
        "GrantNextSpellCostReductionEffect" => {
            decode_as::<ironsmith_core::GrantNextSpellCostReductionEffect>(payload).map(Some)
        }
        "GrantPlayTaggedEffect" => {
            decode_as::<ironsmith_core::GrantPlayTaggedEffect<wire::WireCost>>(payload).map(Some)
        }
        "GrantTaggedSpellFreeCastUntilEndOfTurnEffect" => {
            decode_as::<ironsmith_core::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>(payload)
                .map(Some)
        }
        "GrantTaggedSpellLifeCostByManaValueEffect" => {
            decode_as::<ironsmith_core::GrantTaggedSpellLifeCostByManaValueEffect>(payload)
                .map(Some)
        }
        "IncreaseSpeedEffect" => {
            decode_as::<ironsmith_core::IncreaseSpeedEffect>(payload).map(Some)
        }
        "LoseTheGameEffect" => decode_as::<ironsmith_core::LoseTheGameEffect>(payload).map(Some),
        "MayCastMatchingSpellWithoutPayingManaCostEffect" => {
            decode_as::<ironsmith_core::MayCastMatchingSpellWithoutPayingManaCostEffect>(payload)
                .map(Some)
        }
        "PayAnyEnergyEffect" => decode_as::<ironsmith_core::PayAnyEnergyEffect>(payload).map(Some),
        "PayAnyLifeEffect" => decode_as::<ironsmith_core::PayAnyLifeEffect>(payload).map(Some),
        "PayEnergyEffect" => decode_as::<ironsmith_core::PayEnergyEffect>(payload).map(Some),
        "PlaySubgameEffect" => {
            decode_as::<ironsmith_core::PlaySubgameEffect<wire::WireEffect>>(payload).map(Some)
        }
        "PoisonCountersEffect" => {
            decode_as::<ironsmith_core::PoisonCountersEffect>(payload).map(Some)
        }
        "ReduceSpeedEffect" => decode_as::<ironsmith_core::ReduceSpeedEffect>(payload).map(Some),
        "RestartGameEffect" => decode_as::<ironsmith_core::RestartGameEffect>(payload).map(Some),
        "ReverseTurnOrderEffect" => {
            decode_as::<ironsmith_core::ReverseTurnOrderEffect>(payload).map(Some)
        }
        "RingTemptsYouEffect" => {
            decode_as::<ironsmith_core::RingTemptsYouEffect>(payload).map(Some)
        }
        "RollDiceChooseResultEffect" => {
            decode_as::<ironsmith_core::RollDiceChooseResultEffect>(payload).map(Some)
        }
        "RollDieEffect" => decode_as::<ironsmith_core::RollDieEffect>(payload).map(Some),
        "SkipCombatPhasesEffect" => {
            decode_as::<ironsmith_core::SkipCombatPhasesEffect>(payload).map(Some)
        }
        "SkipCombatPhasesThisTurnEffect" => {
            decode_as::<ironsmith_core::SkipCombatPhasesThisTurnEffect>(payload).map(Some)
        }
        "SkipDrawStepEffect" => decode_as::<ironsmith_core::SkipDrawStepEffect>(payload).map(Some),
        "SkipScheduledEffect" => decode_as::<ironsmith_core::SkipScheduledEffect>(payload).map(Some),
        "SkipMainPhasesThisTurnEffect" => {
            decode_as::<ironsmith_core::SkipMainPhasesThisTurnEffect>(payload).map(Some)
        }
        "SkipNextCombatPhaseThisTurnEffect" => {
            decode_as::<ironsmith_core::SkipNextCombatPhaseThisTurnEffect>(payload).map(Some)
        }
        "SkipTurnEffect" => decode_as::<ironsmith_core::SkipTurnEffect>(payload).map(Some),
        "TakeInitiativeEffect" => {
            decode_as::<ironsmith_core::TakeInitiativeEffect>(payload).map(Some)
        }
        "TicketCountersEffect" => {
            decode_as::<ironsmith_core::TicketCountersEffect>(payload).map(Some)
        }
        "VentureIntoDungeonEffect" => {
            decode_as::<ironsmith_core::VentureIntoDungeonEffect>(payload).map(Some)
        }
        "WinTheGameEffect" => decode_as::<ironsmith_core::WinTheGameEffect>(payload).map(Some),
        "RevealChosenSubtypeEffect" => {
            decode_as::<ironsmith_core::RevealChosenSubtypeEffect>(payload).map(Some)
        }
        "GrantLoyaltyActivationAllowanceEffect" => {
            decode_as::<ironsmith_core::GrantLoyaltyActivationAllowanceEffect>(payload).map(Some)
        }
        "GrantEndThisEffectPaymentEffect" => {
            decode_as::<ironsmith_core::GrantEndThisEffectPaymentEffect>(payload).map(Some)
        }
        "MayCastForMiracleCostEffect" => {
            decode_as::<ironsmith_core::MayCastForMiracleCostEffect>(payload).map(Some)
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
        "AdditionalLandPlaysEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AdditionalLandPlaysEffect,
        >(payload, context)
        .map(Some),
        "AdditionalPhasesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::AdditionalPhasesEffect,
        >(payload, context)
        .map(Some),
        "AscendEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::AscendEffect>(payload, context)
                .map(Some)
        }
        "BecomeMonarchEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::BecomeMonarchEffect,
        >(payload, context)
        .map(Some),
        "CastSourceEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::CastSourceEffect>(payload, context)
                .map(Some)
        }
        "CastTaggedEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::CastTaggedEffect<wire::WireCost>>(payload, context)
                .map(Some)
        }
        "ChooseCardNameEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseCardNameEffect,
        >(payload, context)
        .map(Some),
        "ChooseCardTypeEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseCardTypeEffect,
        >(payload, context)
        .map(Some),
        "ChooseColorEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ChooseColorEffect>(payload, context)
                .map(Some)
        }
        "ChooseCreatureTypeEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseCreatureTypeEffect,
        >(payload, context)
        .map(Some),
        "ChooseLandTypeEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseLandTypeEffect,
        >(payload, context)
        .map(Some),
        "RippleEffect" => super::card_graph::map_payload_as::<ironsmith_core::RippleEffect>(payload, context).map(Some),
        "ChooseNumberEffect" => super::card_graph::map_payload_as::<ironsmith_core::ChooseNumberEffect>(payload, context).map(Some),
        "ChooseNamedOptionEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseNamedOptionEffect,
        >(payload, context)
        .map(Some),
        "ChoosePlayerEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChoosePlayerEffect,
        >(payload, context)
        .map(Some),
        "ChooseFriendsOrFoesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ChooseFriendsOrFoesEffect,
        >(payload, context)
        .map(Some),
        "ControlCombatChoicesThisTurnEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ControlCombatChoicesThisTurnEffect,
        >(payload, context)
        .map(Some),
        "ControlPlayerEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ControlPlayerEffect,
        >(payload, context)
        .map(Some),
        "CreateEmblemEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::CreateEmblemEffect<wire::WireEmblemDescription>,
        >(payload, context)
        .map(Some),
        "DiscoverEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::DiscoverEffect>(payload, context)
                .map(Some)
        }
        "EndCombatPhaseEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::EndCombatPhaseEffect,
        >(payload, context)
        .map(Some),
        "EndTurnEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::EndTurnEffect>(payload, context)
                .map(Some)
        }
        "EnergyCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::EnergyCountersEffect,
        >(payload, context)
        .map(Some),
        "ExileInsteadOfGraveyardEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExileInsteadOfGraveyardEffect,
        >(payload, context)
        .map(Some),
        "ExperienceCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExperienceCountersEffect,
        >(payload, context)
        .map(Some),
        "ExtraTurnAfterNextTurnEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ExtraTurnAfterNextTurnEffect,
        >(payload, context)
        .map(Some),
        "ExtraTurnEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ExtraTurnEffect>(payload, context)
                .map(Some)
        }
        "FlipCoinEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::FlipCoinEffect>(payload, context)
                .map(Some)
        }
        "GivePlayerCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GivePlayerCountersEffect,
        >(payload, context)
        .map(Some),
        "GrantBySpecEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantBySpecEffect<wire::WireGrantSpec, wire::WireGrantDuration>,
        >(payload, context)
        .map(Some),
        "GrantEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantEffect<wire::WireGrantable, wire::WireGrantDuration>,
        >(payload, context)
        .map(Some),
        "GrantNextSpellAbilityEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantNextSpellAbilityEffect<wire::WireAbility>,
        >(payload, context)
        .map(Some),
        "GrantNextSpellCostReductionEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantNextSpellCostReductionEffect,
        >(payload, context)
        .map(Some),
        "GrantPlayTaggedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantPlayTaggedEffect<wire::WireCost>,
        >(payload, context)
        .map(Some),
        "GrantTaggedSpellFreeCastUntilEndOfTurnEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
        >(payload, context)
        .map(Some),
        "GrantTaggedSpellLifeCostByManaValueEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantTaggedSpellLifeCostByManaValueEffect,
        >(payload, context)
        .map(Some),
        "IncreaseSpeedEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::IncreaseSpeedEffect,
        >(payload, context)
        .map(Some),
        "LoseTheGameEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::LoseTheGameEffect>(payload, context)
                .map(Some)
        }
        "MayCastMatchingSpellWithoutPayingManaCostEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::MayCastMatchingSpellWithoutPayingManaCostEffect,
        >(payload, context)
        .map(Some),
        "PayAnyEnergyEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PayAnyEnergyEffect,
        >(payload, context)
        .map(Some),
        "PayAnyLifeEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PayAnyLifeEffect>(payload, context)
                .map(Some)
        }
        "PayEnergyEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::PayEnergyEffect>(payload, context)
                .map(Some)
        }
        "PlaySubgameEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PlaySubgameEffect<wire::WireEffect>,
        >(payload, context)
        .map(Some),
        "PoisonCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::PoisonCountersEffect,
        >(payload, context)
        .map(Some),
        "ReduceSpeedEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::ReduceSpeedEffect>(payload, context)
                .map(Some)
        }
        "RestartGameEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::RestartGameEffect>(payload, context)
                .map(Some)
        }
        "ReverseTurnOrderEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::ReverseTurnOrderEffect,
        >(payload, context)
        .map(Some),
        "RingTemptsYouEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RingTemptsYouEffect,
        >(payload, context)
        .map(Some),
        "RollDiceChooseResultEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RollDiceChooseResultEffect,
        >(payload, context)
        .map(Some),
        "RollDieEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::RollDieEffect>(payload, context)
                .map(Some)
        }
        "SkipCombatPhasesEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SkipCombatPhasesEffect,
        >(payload, context)
        .map(Some),
        "SkipCombatPhasesThisTurnEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SkipCombatPhasesThisTurnEffect,
        >(payload, context)
        .map(Some),
        "SkipScheduledEffect" => super::card_graph::map_payload_as::<ironsmith_core::SkipScheduledEffect>(payload, context).map(Some),
        "SkipDrawStepEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SkipDrawStepEffect,
        >(payload, context)
        .map(Some),
        "SkipMainPhasesThisTurnEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SkipMainPhasesThisTurnEffect,
        >(payload, context)
        .map(Some),
        "SkipNextCombatPhaseThisTurnEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::SkipNextCombatPhaseThisTurnEffect,
        >(payload, context)
        .map(Some),
        "SkipTurnEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::SkipTurnEffect>(payload, context)
                .map(Some)
        }
        "TakeInitiativeEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TakeInitiativeEffect,
        >(payload, context)
        .map(Some),
        "TicketCountersEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::TicketCountersEffect,
        >(payload, context)
        .map(Some),
        "VentureIntoDungeonEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::VentureIntoDungeonEffect,
        >(payload, context)
        .map(Some),
        "WinTheGameEffect" => {
            super::card_graph::map_payload_as::<ironsmith_core::WinTheGameEffect>(payload, context)
                .map(Some)
        }
        "RevealChosenSubtypeEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::RevealChosenSubtypeEffect,
        >(payload, context)
        .map(Some),
        "GrantLoyaltyActivationAllowanceEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantLoyaltyActivationAllowanceEffect,
        >(payload, context)
        .map(Some),
        "GrantEndThisEffectPaymentEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::GrantEndThisEffectPaymentEffect,
        >(payload, context)
        .map(Some),
        "MayCastForMiracleCostEffect" => super::card_graph::map_payload_as::<
            ironsmith_core::MayCastForMiracleCostEffect,
        >(payload, context)
        .map(Some),
        _ => Ok(None),
    }
}
