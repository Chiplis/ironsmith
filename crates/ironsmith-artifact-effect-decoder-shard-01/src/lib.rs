//! Generated typed materializers for the player runtime effect family.

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
        "CastTaggedEffect" => decode_as::<ironsmith_core::CastTaggedEffect>(payload).map(Some),
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
        "ChooseNamedOptionEffect" => {
            decode_as::<ironsmith_core::ChooseNamedOptionEffect>(payload).map(Some)
        }
        "ChoosePlayerEffect" => decode_as::<ironsmith_core::ChoosePlayerEffect>(payload).map(Some),
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
            decode_as::<ironsmith_core::GrantPlayTaggedEffect>(payload).map(Some)
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
        _ => Ok(None),
    }
}
