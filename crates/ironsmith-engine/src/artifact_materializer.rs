//! Materialization of versioned compiled-card artifacts into engine values.

use ironsmith_compiled_artifact as wire;
#[path = "artifact_continuous_word_codec.rs"]
mod continuous_word_codec;
#[path = "artifact_text_program_codec.rs"]
mod text_program_codec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactMaterializationError {
    InvalidArtifact(wire::ArtifactValidationError),
    UnsupportedEffect { detail: String },
    UnsupportedStaticAbility { detail: String },
    UnsupportedTrigger { detail: String },
}

impl std::fmt::Display for ArtifactMaterializationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArtifact(error) => error.fmt(formatter),
            Self::UnsupportedEffect { detail } => {
                write!(formatter, "artifact effect is unsupported: {detail}")
            }
            Self::UnsupportedStaticAbility { detail } => {
                write!(
                    formatter,
                    "artifact static ability is unsupported: {detail}"
                )
            }
            Self::UnsupportedTrigger { detail } => {
                write!(formatter, "artifact trigger is unsupported: {detail}")
            }
        }
    }
}

impl std::error::Error for ArtifactMaterializationError {}

struct WireEffectModel;

#[cfg(any())]
fn decode_as<T, D>(effect: &wire::WireEffect) -> Option<&T>
where
    T: 'static,
    D: serde::de::DeserializeOwned + Send + Sync + 'static,
{
    effect.downcast_with::<T, _>(|payload| {
        serde_json::from_value::<D>(payload)
            .map(|value| Box::new(value) as Box<dyn std::any::Any + Send + Sync>)
            .map_err(|error| error.to_string())
    })
}

#[cfg(any())]
fn decode_wire_effect_monolithic_reference<T: 'static>(effect: &wire::WireEffect) -> Option<&T> {
    match effect.kind() {
        "AdaptEffect" => decode_as::<T, ironsmith_core::AdaptEffect>(effect),
        "AddManaEffect" => decode_as::<T, ironsmith_core::AddManaEffect>(effect),
        "AddManaFromCommanderColorIdentityEffect" => {
            decode_as::<T, ironsmith_core::AddManaFromCommanderColorIdentityEffect>(effect)
        }
        "AddManaOfAnyColorEffect" => {
            decode_as::<T, ironsmith_core::AddManaOfAnyColorEffect>(effect)
        }
        "AddManaOfAnyOneColorEffect" => {
            decode_as::<T, ironsmith_core::AddManaOfAnyOneColorEffect>(effect)
        }
        "AddManaOfNotedTypeEffect" => {
            decode_as::<T, ironsmith_core::AddManaOfNotedTypeEffect>(effect)
        }
        "NoteActivationManaTypeEffect" => {
            decode_as::<T, ironsmith_core::NoteActivationManaTypeEffect>(effect)
        }
        "AddManaOfChosenColorEffect" => {
            decode_as::<T, ironsmith_core::AddManaOfChosenColorEffect>(effect)
        }
        "AddManaOfColorsAmongEffect" => {
            decode_as::<T, ironsmith_core::AddManaOfColorsAmongEffect>(effect)
        }
        "AddManaOfImprintedColorsEffect" => {
            decode_as::<T, ironsmith_core::AddManaOfImprintedColorsEffect>(effect)
        }
        "AddManaOfLandProducedTypesEffect" => {
            decode_as::<T, ironsmith_core::AddManaOfLandProducedTypesEffect>(effect)
        }
        "AddOneManaOfAnyColorAmongEffect" => {
            decode_as::<T, ironsmith_core::AddOneManaOfAnyColorAmongEffect>(effect)
        }
        "AddScaledManaEffect" => decode_as::<T, ironsmith_core::AddScaledManaEffect>(effect),
        "AdditionalLandPlaysEffect" => {
            decode_as::<T, ironsmith_core::AdditionalLandPlaysEffect>(effect)
        }
        "AdditionalPhasesEffect" => decode_as::<T, ironsmith_core::AdditionalPhasesEffect>(effect),
        "AmassEffect" => decode_as::<T, ironsmith_core::AmassEffect>(effect),
        "CollectEvidenceEffect" => decode_as::<T, ironsmith_core::CollectEvidenceEffect>(effect),
        "EmpowerJaceEffect" => decode_as::<T, ironsmith_core::EmpowerJaceEffect>(effect),
        "AmplifyEffect" => decode_as::<T, ironsmith_core::AmplifyEffect>(effect),
        "ApplyContinuousEffect" => decode_as::<
            T,
            ironsmith_core::ApplyContinuousEffect<
                wire::WireContinuousTarget,
                wire::WireContinuousModification,
                wire::WireRuntimeModification,
                ironsmith_core::Condition,
            >,
        >(effect),
        "AscendEffect" => decode_as::<T, ironsmith_core::AscendEffect>(effect),
        "AssignNoCombatDamageEffect" => {
            decode_as::<T, ironsmith_core::AssignNoCombatDamageEffect>(effect)
        }
        "AttachObjectsEffect" => decode_as::<T, ironsmith_core::AttachObjectsEffect>(effect),
        "AttachToEffect" => decode_as::<T, ironsmith_core::AttachToEffect>(effect),
        "AuraSwapEffect" => decode_as::<T, ironsmith_core::AuraSwapEffect>(effect),
        "BackupEffect" => decode_as::<T, ironsmith_core::BackupEffect<wire::WireAbility>>(effect),
        "BecomeBasicLandTypeChoiceEffect" => {
            decode_as::<T, ironsmith_core::BecomeBasicLandTypeChoiceEffect>(effect)
        }
        "BecomeColorChoiceEffect" => {
            decode_as::<T, ironsmith_core::BecomeColorChoiceEffect>(effect)
        }
        "BecomeCreatureTypeChoiceEffect" => {
            decode_as::<T, ironsmith_core::BecomeCreatureTypeChoiceEffect>(effect)
        }
        "BecomeMonarchEffect" => decode_as::<T, ironsmith_core::BecomeMonarchEffect>(effect),
        "BecomeSaddledUntilEotEffect" => {
            decode_as::<T, ironsmith_core::BecomeSaddledUntilEotEffect>(effect)
        }
        "BeholdEffect" => decode_as::<T, ironsmith_core::BeholdEffect>(effect),
        "BidLifeEffect" => decode_as::<T, ironsmith_core::BidLifeEffect<wire::WireEffect>>(effect),
        "BolsterEffect" => decode_as::<T, ironsmith_core::BolsterEffect>(effect),
        "CantEffect" => decode_as::<T, ironsmith_core::CantEffect>(effect),
        "CastSourceEffect" => decode_as::<T, ironsmith_core::CastSourceEffect>(effect),
        "CastTaggedEffect" => decode_as::<T, ironsmith_core::CastTaggedEffect<wire::WireCost>>(effect),
        "ChooseCardNameEffect" => decode_as::<T, ironsmith_core::ChooseCardNameEffect>(effect),
        "ChooseCardTypeEffect" => decode_as::<T, ironsmith_core::ChooseCardTypeEffect>(effect),
        "ChangeTextEffect" => decode_as::<T, ironsmith_core::ChangeTextEffect>(effect),
        "ChooseColorEffect" => decode_as::<T, ironsmith_core::ChooseColorEffect>(effect),
        "RevealChosenSubtypeEffect" => {
            decode_as::<T, ironsmith_core::RevealChosenSubtypeEffect>(effect)
        }
        "ChooseCreatureTypeEffect" => {
            decode_as::<T, ironsmith_core::ChooseCreatureTypeEffect>(effect)
        }
        "ChooseLandTypeEffect" => decode_as::<T, ironsmith_core::ChooseLandTypeEffect>(effect),
        "ChooseModeEffect" => {
            decode_as::<T, ironsmith_core::ChooseModeEffect<wire::WireEffect>>(effect)
        }
        "ChooseNamedOptionEffect" => {
            decode_as::<T, ironsmith_core::ChooseNamedOptionEffect>(effect)
        }
        "ChooseNewTargetsEffect" => decode_as::<T, ironsmith_core::ChooseNewTargetsEffect>(effect),
        "ChooseObjectsEffect" => decode_as::<T, ironsmith_core::ChooseObjectsEffect>(effect),
        "ChoosePlayerEffect" => decode_as::<T, ironsmith_core::ChoosePlayerEffect>(effect),
        "ChooseSpellCastHistoryEffect" => {
            decode_as::<T, ironsmith_core::ChooseSpellCastHistoryEffect>(effect)
        }
        "CipherEffect" => decode_as::<T, ironsmith_core::CipherEffect>(effect),
        "ClashEffect" => decode_as::<T, ironsmith_core::ClashEffect>(effect),
        "ClearSuspectedEffect" => decode_as::<T, ironsmith_core::ClearSuspectedEffect>(effect),
        "ConditionalEffect" => {
            decode_as::<T, ironsmith_core::ConditionalEffect<wire::WireEffect>>(effect)
        }
        "ConniveEffect" => decode_as::<T, ironsmith_core::ConniveEffect>(effect),
        "ConspireCostEffect" => decode_as::<T, ironsmith_core::ConspireCostEffect>(effect),
        "ConsultTopOfLibraryEffect" => {
            decode_as::<T, ironsmith_core::ConsultTopOfLibraryEffect>(effect)
        }
        "ControlCombatChoicesThisTurnEffect" => {
            decode_as::<T, ironsmith_core::ControlCombatChoicesThisTurnEffect>(effect)
        }
        "ControlPlayerEffect" => decode_as::<T, ironsmith_core::ControlPlayerEffect>(effect),
        "ConvertEffect" => decode_as::<T, ironsmith_core::ConvertEffect>(effect),
        "CopySpellEffect" => decode_as::<T, ironsmith_core::CopySpellEffect>(effect),
        "CopySpellForEachTargetEffect" => {
            decode_as::<T, ironsmith_core::CopySpellForEachTargetEffect>(effect)
        }
        "CounterEffect" => decode_as::<T, ironsmith_core::CounterEffect>(effect),
        "CreateEmblemEffect" => {
            decode_as::<T, ironsmith_core::CreateEmblemEffect<wire::WireEmblemDescription>>(effect)
        }
        "CreateTokenCopyEffect" => {
            decode_as::<T, ironsmith_core::CreateTokenCopyEffect<wire::WireStaticAbility>>(effect)
        }
        "CreateTokenEffect" => {
            decode_as::<T, ironsmith_core::CreateTokenEffect<wire::WireCardDefinition>>(effect)
        }
        "CrewCostEffect" => decode_as::<T, ironsmith_core::CrewCostEffect>(effect),
        "SaddleCostEffect" => decode_as::<T, ironsmith_core::SaddleCostEffect>(effect),
        "CumulativeUpkeepEffect" => {
            decode_as::<T, ironsmith_core::CumulativeUpkeepEffect<wire::WireEffect>>(effect)
        }
        "DealDamageEachEffect" => decode_as::<T, ironsmith_core::DealDamageEachEffect>(effect),
        "DealDamageBySourcesEffect" => {
            decode_as::<T, ironsmith_core::DealDamageBySourcesEffect>(effect)
        }
        "DealDamageToRecipientsEffect" => {
            decode_as::<T, ironsmith_core::DealDamageToRecipientsEffect>(effect)
        }
        "DealDamageEffect" => decode_as::<T, ironsmith_core::DealDamageEffect>(effect),
        "DealDistributedDamageEffect" => {
            decode_as::<T, ironsmith_core::DealDistributedDamageEffect>(effect)
        }
        "DestroyEffect" => decode_as::<T, ironsmith_core::DestroyEffect>(effect),
        "DestroyNoRegenerationEffect" => {
            decode_as::<T, ironsmith_core::DestroyNoRegenerationEffect>(effect)
        }
        "DetainEffect" => decode_as::<T, ironsmith_core::DetainEffect>(effect),
        "RippleEffect" => decode_as::<T, ironsmith_core::RippleEffect>(effect),
        "ChooseNumberEffect" => decode_as::<T, ironsmith_core::ChooseNumberEffect>(effect),
        "ChooseNumberAtRandomEffect" => {
            decode_as::<T, ironsmith_core::ChooseNumberAtRandomEffect>(effect)
        }
        "NextAdaptIgnoresCountersEffect" => {
            decode_as::<T, ironsmith_core::NextAdaptIgnoresCountersEffect>(effect)
        }
        "DevourEffect" => decode_as::<T, ironsmith_core::DevourEffect>(effect),
        "ResolvesDespiteIllegalTargetsEffect" => {
            decode_as::<T, ironsmith_core::ResolvesDespiteIllegalTargetsEffect>(effect)
        }
        "MayCastForMiracleCostEffect" => {
            decode_as::<T, ironsmith_core::MayCastForMiracleCostEffect>(effect)
        }
        "DirectionalAdjacentPlayerControlEffect" => {
            decode_as::<T, ironsmith_core::DirectionalAdjacentPlayerControlEffect>(effect)
        }
        "DiscardEffect" => decode_as::<T, ironsmith_core::DiscardEffect>(effect),
        "DiscardHandEffect" => decode_as::<T, ironsmith_core::DiscardHandEffect>(effect),
        "DiscoverEffect" => decode_as::<T, ironsmith_core::DiscoverEffect>(effect),
        "DoubleCountersEffect" => decode_as::<T, ironsmith_core::DoubleCountersEffect>(effect),
        "DoubleManaPoolEffect" => decode_as::<T, ironsmith_core::DoubleManaPoolEffect>(effect),
        "DrawCardsEffect" => decode_as::<T, ironsmith_core::DrawCardsEffect>(effect),
        "DrawForEachTaggedMatchingEffect" => {
            decode_as::<T, ironsmith_core::DrawForEachTaggedMatchingEffect>(effect)
        }
        "EachPlayerScryEffect" => decode_as::<T, ironsmith_core::EachPlayerScryEffect>(effect),
        "EarthbendEffect" => decode_as::<T, ironsmith_core::EarthbendEffect>(effect),
        "EmitGiftGivenEffect" => decode_as::<T, ironsmith_core::EmitGiftGivenEffect>(effect),
        "EmitKeywordActionEffect" => {
            decode_as::<T, ironsmith_core::EmitKeywordActionEffect>(effect)
        }
        "EmptyManaPoolEffect" => decode_as::<T, ironsmith_core::EmptyManaPoolEffect>(effect),
        "EndCombatPhaseEffect" => decode_as::<T, ironsmith_core::EndCombatPhaseEffect>(effect),
        "EndTurnEffect" => decode_as::<T, ironsmith_core::EndTurnEffect>(effect),
        "EnergyCountersEffect" => decode_as::<T, ironsmith_core::EnergyCountersEffect>(effect),
        "EvolveEffect" => decode_as::<T, ironsmith_core::EvolveEffect>(effect),
        "ExchangeControlEffect" => decode_as::<T, ironsmith_core::ExchangeControlEffect>(effect),
        "ExchangeLifeTotalsEffect" => {
            decode_as::<T, ironsmith_core::ExchangeLifeTotalsEffect>(effect)
        }
        "ExchangeTextBoxesEffect" => {
            decode_as::<T, ironsmith_core::ExchangeTextBoxesEffect>(effect)
        }
        "ExchangeValuesEffect" => decode_as::<T, ironsmith_core::ExchangeValuesEffect>(effect),
        "ExchangeZonesEffect" => decode_as::<T, ironsmith_core::ExchangeZonesEffect>(effect),
        "ExecuteWithSourceEffect" => {
            decode_as::<T, ironsmith_core::ExecuteWithSourceEffect<wire::WireEffect>>(effect)
        }
        "ExertCostEffect" => decode_as::<T, ironsmith_core::ExertCostEffect>(effect),
        "ExileEffect" => decode_as::<T, ironsmith_core::ExileEffect>(effect),
        "ExileInsteadOfGraveyardEffect" => {
            decode_as::<T, ironsmith_core::ExileInsteadOfGraveyardEffect>(effect)
        }
        "ExileTaggedWhenSourceLeavesEffect" => {
            decode_as::<T, ironsmith_core::ExileTaggedWhenSourceLeavesEffect>(effect)
        }
        "ExileTopOfLibraryEffect" => {
            decode_as::<T, ironsmith_core::ExileTopOfLibraryEffect>(effect)
        }
        "ExileUntilEffect" => decode_as::<T, ironsmith_core::ExileUntilEffect>(effect),
        "ExperienceCountersEffect" => {
            decode_as::<T, ironsmith_core::ExperienceCountersEffect>(effect)
        }
        "GivePlayerCountersEffect" => {
            decode_as::<T, ironsmith_core::GivePlayerCountersEffect>(effect)
        }
        "ExploreEffect" => decode_as::<T, ironsmith_core::ExploreEffect>(effect),
        "ExtraTurnAfterNextTurnEffect" => {
            decode_as::<T, ironsmith_core::ExtraTurnAfterNextTurnEffect>(effect)
        }
        "ExtraTurnEffect" => decode_as::<T, ironsmith_core::ExtraTurnEffect>(effect),
        "FatesealEffect" => decode_as::<T, ironsmith_core::FatesealEffect>(effect),
        "FightEffect" => decode_as::<T, ironsmith_core::FightEffect>(effect),
        "FlipCoinEffect" => decode_as::<T, ironsmith_core::FlipCoinEffect>(effect),
        "FlipEffect" => decode_as::<T, ironsmith_core::FlipEffect>(effect),
        "ForEachControllerOfTaggedEffect" => decode_as::<
            T,
            ironsmith_core::ForEachControllerOfTaggedEffect<wire::WireEffect>,
        >(effect),
        "ForEachCounterKindPutOrRemoveEffect" => {
            decode_as::<T, ironsmith_core::ForEachCounterKindPutOrRemoveEffect>(effect)
        }
        "ForEachObject" => decode_as::<T, ironsmith_core::ForEachObject<wire::WireEffect>>(effect),
        "ForEachObjectCorrelatedResultEffect" => decode_as::<
            T,
            ironsmith_core::ForEachObjectCorrelatedResultEffect<wire::WireEffect>,
        >(effect),
        "ForEachTaggedEffect" => {
            decode_as::<T, ironsmith_core::ForEachTaggedEffect<wire::WireEffect>>(effect)
        }
        "ForEachTaggedPlayerEffect" => {
            decode_as::<T, ironsmith_core::ForEachTaggedPlayerEffect<wire::WireEffect>>(effect)
        }
        "CollectManaPaymentsEffect" => {
            decode_as::<T, ironsmith_core::CollectManaPaymentsEffect<wire::WireEffect>>(effect)
        }
        "ForPlayersEffect" => {
            decode_as::<T, ironsmith_core::ForPlayersEffect<wire::WireEffect>>(effect)
        }
        "GainLifeEffect" => decode_as::<T, ironsmith_core::GainLifeEffect>(effect),
        "GoadEffect" => decode_as::<T, ironsmith_core::GoadEffect>(effect),
        "ClearGoadEffect" => decode_as::<T, ironsmith_core::ClearGoadEffect>(effect),
        "GrantAbilitiesTargetEffect" => decode_as::<
            T,
            ironsmith_core::GrantAbilitiesTargetEffect<wire::WireStaticAbility>,
        >(effect),
        "GrantBySpecEffect" => decode_as::<
            T,
            ironsmith_core::GrantBySpecEffect<wire::WireGrantSpec, wire::WireGrantDuration>,
        >(effect),
        "GrantEffect" => decode_as::<
            T,
            ironsmith_core::GrantEffect<wire::WireGrantable, wire::WireGrantDuration>,
        >(effect),
        "GrantNextSpellAbilityEffect" => {
            decode_as::<T, ironsmith_core::GrantNextSpellAbilityEffect<wire::WireAbility>>(effect)
        }
        "GrantNextSpellCostReductionEffect" => {
            decode_as::<T, ironsmith_core::GrantNextSpellCostReductionEffect>(effect)
        }
        "GrantPlayTaggedEffect" => decode_as::<T, ironsmith_core::GrantPlayTaggedEffect<wire::WireCost>>(effect),
        "GrantEndThisEffectPaymentEffect" => {
            decode_as::<T, ironsmith_core::GrantEndThisEffectPaymentEffect>(effect)
        }
        "GrantRepeatableManaPaymentActionUntilEndOfTurnEffect" => decode_as::<
            T,
            ironsmith_core::GrantRepeatableManaPaymentActionUntilEndOfTurnEffect<wire::WireEffect>,
        >(effect),
        "GrantTaggedSpellFreeCastUntilEndOfTurnEffect" => {
            decode_as::<T, ironsmith_core::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>(effect)
        }
        "GrantTaggedSpellLifeCostByManaValueEffect" => {
            decode_as::<T, ironsmith_core::GrantTaggedSpellLifeCostByManaValueEffect>(effect)
        }
        "HauntExileEffect" => {
            decode_as::<T, ironsmith_core::HauntExileEffect<wire::WireEffect>>(effect)
        }
        "HealDamageEffect" => decode_as::<T, ironsmith_core::HealDamageEffect>(effect),
        "IfEffect" => decode_as::<T, ironsmith_core::IfEffect<wire::WireEffect>>(effect),
        "IncreaseSpeedEffect" => decode_as::<T, ironsmith_core::IncreaseSpeedEffect>(effect),
        "IncubateEffect" => decode_as::<T, ironsmith_core::IncubateEffect>(effect),
        "InvestigateEffect" => decode_as::<T, ironsmith_core::InvestigateEffect>(effect),
        "LearnEffect" => decode_as::<T, ironsmith_core::LearnEffect>(effect),
        "LocalRewriteEffect" => {
            decode_as::<T, ironsmith_core::LocalRewriteEffect<wire::WireEffect>>(effect)
        }
        "LookAtHandEffect" => decode_as::<T, ironsmith_core::LookAtHandEffect>(effect),
        "LookAtObjectsEffect" => decode_as::<T, ironsmith_core::LookAtObjectsEffect>(effect),
        "LookAtTopCardsEffect" => decode_as::<T, ironsmith_core::LookAtTopCardsEffect>(effect),
        "LoseLifeEffect" => decode_as::<T, ironsmith_core::LoseLifeEffect>(effect),
        "LoseTheGameEffect" => decode_as::<T, ironsmith_core::LoseTheGameEffect>(effect),
        "ManaRestrictedEffect" => {
            decode_as::<T, ironsmith_core::ManaRestrictedEffect<wire::WireEffect>>(effect)
        }
        "ManaRetainedEffect" => {
            decode_as::<T, ironsmith_core::ManaRetainedEffect<wire::WireEffect>>(effect)
        }
        "ManifestCardFromHandEffect" => {
            decode_as::<T, ironsmith_core::ManifestCardFromHandEffect>(effect)
        }
        "ManifestDreadEffect" => decode_as::<T, ironsmith_core::ManifestDreadEffect>(effect),
        "ManifestObjectsEffect" => decode_as::<T, ironsmith_core::ManifestObjectsEffect>(effect),
        "ManifestTopCardOfLibraryEffect" => {
            decode_as::<T, ironsmith_core::ManifestTopCardOfLibraryEffect>(effect)
        }
        "MayCastMatchingSpellWithoutPayingManaCostEffect" => {
            decode_as::<T, ironsmith_core::MayCastMatchingSpellWithoutPayingManaCostEffect>(effect)
        }
        "MayEffect" => decode_as::<T, ironsmith_core::MayEffect<wire::WireEffect>>(effect),
        "MayMoveToZoneEffect" => decode_as::<T, ironsmith_core::MayMoveToZoneEffect>(effect),
        "MeldEffect" => decode_as::<T, ironsmith_core::MeldEffect>(effect),
        "MillEffect" => decode_as::<T, ironsmith_core::MillEffect>(effect),
        "ModifyPowerToughnessEffect" => {
            decode_as::<T, ironsmith_core::ModifyPowerToughnessEffect>(effect)
        }
        "ModifyPowerToughnessForEachEffect" => {
            decode_as::<T, ironsmith_core::ModifyPowerToughnessForEachEffect>(effect)
        }
        "MonstrosityEffect" => decode_as::<T, ironsmith_core::MonstrosityEffect>(effect),
        "MoveAllCountersEffect" => decode_as::<T, ironsmith_core::MoveAllCountersEffect>(effect),
        "MoveCountersEffect" => decode_as::<T, ironsmith_core::MoveCountersEffect>(effect),
        "MoveOneCounterEffect" => decode_as::<T, ironsmith_core::MoveOneCounterEffect>(effect),
        "MoveToLibraryNthFromTopEffect" => {
            decode_as::<T, ironsmith_core::MoveToLibraryNthFromTopEffect>(effect)
        }
        "MoveToLibraryTopOrBottomChoiceEffect" => {
            decode_as::<T, ironsmith_core::MoveToLibraryTopOrBottomChoiceEffect>(effect)
        }
        "MoveToZoneEffect" => decode_as::<T, ironsmith_core::MoveToZoneEffect>(effect),
        "NinjutsuCostEffect" => decode_as::<T, ironsmith_core::NinjutsuCostEffect>(effect),
        "NinjutsuEffect" => decode_as::<T, ironsmith_core::NinjutsuEffect>(effect),
        "NoteLifeTotalEffect" => decode_as::<T, ironsmith_core::NoteLifeTotalEffect>(effect),
        "OpenAttractionEffect" => decode_as::<T, ironsmith_core::OpenAttractionEffect>(effect),
        "PayAnyEnergyEffect" => decode_as::<T, ironsmith_core::PayAnyEnergyEffect>(effect),
        "PayAnyLifeEffect" => decode_as::<T, ironsmith_core::PayAnyLifeEffect>(effect),
        "PayEnergyEffect" => decode_as::<T, ironsmith_core::PayEnergyEffect>(effect),
        "PayLifeEffect" => decode_as::<T, ironsmith_core::PayLifeEffect>(effect),
        "PayManaEffect" => decode_as::<T, ironsmith_core::PayManaEffect>(effect),
        "PhaseInEffect" => decode_as::<T, ironsmith_core::PhaseInEffect>(effect),
        "PhaseOutEffect" => decode_as::<T, ironsmith_core::PhaseOutEffect>(effect),
        "PlaySubgameEffect" => {
            decode_as::<T, ironsmith_core::PlaySubgameEffect<wire::WireEffect>>(effect)
        }
        "PoisonCountersEffect" => decode_as::<T, ironsmith_core::PoisonCountersEffect>(effect),
        "PopulateEffect" => decode_as::<T, ironsmith_core::PopulateEffect>(effect),
        "PreventAllCombatDamageEffect" => {
            decode_as::<T, ironsmith_core::PreventAllCombatDamageEffect>(effect)
        }
        "PreventAllDamageEffect" => {
            decode_as::<T, ironsmith_core::PreventAllDamageEffect<wire::WireEffect>>(effect)
        }
        "PreventAllDamageToTargetEffect" => {
            decode_as::<T, ironsmith_core::PreventAllDamageToTargetEffect<wire::WireEffect>>(effect)
        }
        "PreventDamageEffect" => {
            decode_as::<T, ironsmith_core::PreventDamageEffect<wire::WireEffect>>(effect)
        }
        "PreventNextTimeDamageEffect" => {
            decode_as::<T, ironsmith_core::PreventNextTimeDamageEffect<wire::WireEffect>>(effect)
        }
        "ProliferateEffect" => decode_as::<T, ironsmith_core::ProliferateEffect>(effect),
        "PutCounterOfChosenKindEffect" => {
            decode_as::<T, ironsmith_core::PutCounterOfChosenKindEffect>(effect)
        }
        "PutCountersEffect" => decode_as::<T, ironsmith_core::PutCountersEffect>(effect),
        "PutOntoBattlefieldEffect" => {
            decode_as::<T, ironsmith_core::PutOntoBattlefieldEffect>(effect)
        }
        "PutStickerEffect" => decode_as::<T, ironsmith_core::PutStickerEffect>(effect),
        "PutTaggedRemainderOnLibraryBottomEffect" => {
            decode_as::<T, ironsmith_core::PutTaggedRemainderOnLibraryBottomEffect>(effect)
        }
        "RearrangeLookedCardsInLibraryEffect" => {
            decode_as::<T, ironsmith_core::RearrangeLookedCardsInLibraryEffect>(effect)
        }
        "ReconfigureEffect" => decode_as::<T, ironsmith_core::ReconfigureEffect>(effect),
        "RedirectAllDamageThisTurnToTargetEffect" => {
            decode_as::<T, ironsmith_core::RedirectAllDamageThisTurnToTargetEffect>(effect)
        }
        "RedirectNextDamageToTargetEffect" => {
            decode_as::<T, ironsmith_core::RedirectNextDamageToTargetEffect>(effect)
        }
        "RedirectNextTimeDamageToSourceEffect" => {
            decode_as::<T, ironsmith_core::RedirectNextTimeDamageToSourceEffect>(effect)
        }
        "ReduceSpeedEffect" => decode_as::<T, ironsmith_core::ReduceSpeedEffect>(effect),
        "ReflexiveTriggerEffect" => {
            decode_as::<T, ironsmith_core::ReflexiveTriggerEffect<wire::WireEffect>>(effect)
        }
        "RegenerateEffect" => {
            decode_as::<T, ironsmith_core::RegenerateEffect<wire::WireEffect>>(effect)
        }
        "RegisterDamagedBySourceZoneReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterDamagedBySourceZoneReplacementEffect>(effect)
        }
        "RegisterDrawReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterDrawReplacementEffect<wire::WireEffect>>(effect)
        }
        "RegisterEnterWithCountersReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterEnterWithCountersReplacementEffect>(effect)
        }
        "RegisterEnterTappedReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterEnterTappedReplacementEffect>(effect)
        }
        "RegisterEnterUnderControlReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterEnterUnderControlReplacementEffect>(effect)
        }
        "RegisterFutureZoneReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterFutureZoneReplacementEffect>(effect)
        }
        "RegisterManaRewriteEffect" => decode_as::<T, ironsmith_core::RegisterManaRewriteEffect>(effect),
        "RegisterManaSpendPermissionEffect" => decode_as::<T, ironsmith_core::RegisterManaSpendPermissionEffect>(effect),
        "RegisterManaReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterManaReplacementEffect>(effect)
        }
        "RegisterDamageMultiplierEffect" => {
            decode_as::<T, ironsmith_core::RegisterDamageMultiplierEffect>(effect)
        }
        "RegisterDamageAdditionEffect" => {
            decode_as::<T, ironsmith_core::RegisterDamageAdditionEffect>(effect)
        }
        "RegisterCounterPlacementReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterCounterPlacementReplacementEffect>(effect)
        }
        "RegisterNextBatchEnterWithCountersEffect" => {
            decode_as::<T, ironsmith_core::RegisterNextBatchEnterWithCountersEffect>(effect)
        }
        "RegisterZoneReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterZoneReplacementEffect>(effect)
        }
        "RemoveAnyCountersAmongEffect" => {
            decode_as::<T, ironsmith_core::RemoveAnyCountersAmongEffect>(effect)
        }
        "RemoveAnyCountersFromSourceEffect" => {
            decode_as::<T, ironsmith_core::RemoveAnyCountersFromSourceEffect>(effect)
        }
        "RemoveCountersEffect" => decode_as::<T, ironsmith_core::RemoveCountersEffect>(effect),
        "BecomeBlockedEffect" => decode_as::<T, ironsmith_core::BecomeBlockedEffect>(effect),
        "RemoveFromCombatEffect" => decode_as::<T, ironsmith_core::RemoveFromCombatEffect>(effect),
        "RemoveUpToAnyCountersEffect" => {
            decode_as::<T, ironsmith_core::RemoveUpToAnyCountersEffect>(effect)
        }
        "RemoveUpToCountersEffect" => {
            decode_as::<T, ironsmith_core::RemoveUpToCountersEffect>(effect)
        }
        "RenownEffect" => decode_as::<T, ironsmith_core::RenownEffect>(effect),
        "ReorderGraveyardEffect" => decode_as::<T, ironsmith_core::ReorderGraveyardEffect>(effect),
        "ReorderLibraryTopEffect" => {
            decode_as::<T, ironsmith_core::ReorderLibraryTopEffect>(effect)
        }
        "ReorderTopPlanarDeckEffect" => {
            decode_as::<T, ironsmith_core::ReorderTopPlanarDeckEffect>(effect)
        }
        "RepeatEffectsEffect" => {
            decode_as::<T, ironsmith_core::RepeatEffectsEffect<wire::WireEffect>>(effect)
        }
        "RepeatProcessEffect" => {
            decode_as::<T, ironsmith_core::RepeatProcessEffect<wire::WireEffect>>(effect)
        }
        "RepeatProcessPromptEffect" => {
            decode_as::<T, ironsmith_core::RepeatProcessPromptEffect>(effect)
        }
        "ReplaceNextDamageToTargetEffect" => decode_as::<
            T,
            ironsmith_core::ReplaceNextDamageToTargetEffect<wire::WireEffect>,
        >(effect),
        "RestartGameEffect" => decode_as::<T, ironsmith_core::RestartGameEffect>(effect),
        "RetainManaUntilEndOfTurnEffect" => {
            decode_as::<T, ironsmith_core::RetainManaUntilEndOfTurnEffect>(effect)
        }
        "RetargetStackObjectEffect" => {
            decode_as::<T, ironsmith_core::RetargetStackObjectEffect>(effect)
        }
        "ReturnAllToBattlefieldEffect" => {
            decode_as::<T, ironsmith_core::ReturnAllToBattlefieldEffect>(effect)
        }
        "ReturnFromGraveyardOrExileToBattlefieldEffect" => {
            decode_as::<T, ironsmith_core::ReturnFromGraveyardOrExileToBattlefieldEffect>(effect)
        }
        "ReturnFromGraveyardToBattlefieldEffect" => {
            decode_as::<T, ironsmith_core::ReturnFromGraveyardToBattlefieldEffect>(effect)
        }
        "ReturnFromGraveyardToHandEffect" => {
            decode_as::<T, ironsmith_core::ReturnFromGraveyardToHandEffect>(effect)
        }
        "ReturnToHandEffect" => decode_as::<T, ironsmith_core::ReturnToHandEffect>(effect),
        "RevealFromHandEffect" => decode_as::<T, ironsmith_core::RevealFromHandEffect>(effect),
        "RevealSourceFromHandEffect" => {
            decode_as::<T, ironsmith_core::RevealSourceFromHandEffect>(effect)
        }
        "RevealTaggedEffect" => decode_as::<T, ironsmith_core::RevealTaggedEffect>(effect),
        "RevealTopEffect" => decode_as::<T, ironsmith_core::RevealTopEffect>(effect),
        "ReverseTurnOrderEffect" => decode_as::<T, ironsmith_core::ReverseTurnOrderEffect>(effect),
        "RingTemptsYouEffect" => decode_as::<T, ironsmith_core::RingTemptsYouEffect>(effect),
        "RollDiceChooseResultEffect" => {
            decode_as::<T, ironsmith_core::RollDiceChooseResultEffect>(effect)
        }
        "RollDieEffect" => decode_as::<T, ironsmith_core::RollDieEffect>(effect),
        "SacrificeEffect" => decode_as::<T, ironsmith_core::SacrificeEffect>(effect),
        "SacrificePlayerEffect" => decode_as::<T, ironsmith_core::SacrificePlayerEffect>(effect),
        "SacrificeTargetEffect" => decode_as::<T, ironsmith_core::SacrificeTargetEffect>(effect),
        "ScheduleDelayedTriggerEffect" => {
            decode_as::<T, ironsmith_core::ScheduleDelayedTriggerEffect<wire::WireEffect>>(effect)
        }
        "ScheduleEffectsWhenTaggedLeavesEffect" => decode_as::<
            T,
            ironsmith_core::ScheduleEffectsWhenTaggedLeavesEffect<wire::WireEffect>,
        >(effect),
        "ScryEffect" => decode_as::<T, ironsmith_core::ScryEffect>(effect),
        "SearchLibraryEffect" => decode_as::<T, ironsmith_core::SearchLibraryEffect>(effect),
        "SearchLibrarySlotsEffect" => {
            decode_as::<T, ironsmith_core::SearchLibrarySlotsEffect>(effect)
        }
        "SecretChoiceEffect" => decode_as::<T, ironsmith_core::SecretChoiceEffect>(effect),
        "SequenceEffect" => {
            decode_as::<T, ironsmith_core::SequenceEffect<wire::WireEffect>>(effect)
        }
        "SetBasePowerToughnessEffect" => {
            decode_as::<T, ironsmith_core::SetBasePowerToughnessEffect>(effect)
        }
        "SetLifeTotalEffect" => decode_as::<T, ironsmith_core::SetLifeTotalEffect>(effect),
        "ShuffleGraveyardIntoLibraryEffect" => {
            decode_as::<T, ironsmith_core::ShuffleGraveyardIntoLibraryEffect>(effect)
        }
        "ShuffleHandAndGraveyardIntoLibraryEffect" => {
            decode_as::<T, ironsmith_core::ShuffleHandAndGraveyardIntoLibraryEffect>(effect)
        }
        "ShuffleLibraryEffect" => decode_as::<T, ironsmith_core::ShuffleLibraryEffect>(effect),
        "ShuffleObjectsIntoLibraryEffect" => {
            decode_as::<T, ironsmith_core::ShuffleObjectsIntoLibraryEffect>(effect)
        }
        "SkipCombatPhasesEffect" => decode_as::<T, ironsmith_core::SkipCombatPhasesEffect>(effect),
        "SkipCombatPhasesThisTurnEffect" => {
            decode_as::<T, ironsmith_core::SkipCombatPhasesThisTurnEffect>(effect)
        }
        "SkipDrawStepEffect" => decode_as::<T, ironsmith_core::SkipDrawStepEffect>(effect),
        "SkipScheduledEffect" => decode_as::<T, ironsmith_core::SkipScheduledEffect>(effect),
        "SkipMainPhasesThisTurnEffect" => {
            decode_as::<T, ironsmith_core::SkipMainPhasesThisTurnEffect>(effect)
        }
        "SkipNextCombatPhaseThisTurnEffect" => {
            decode_as::<T, ironsmith_core::SkipNextCombatPhaseThisTurnEffect>(effect)
        }
        "SkipTurnEffect" => decode_as::<T, ironsmith_core::SkipTurnEffect>(effect),
        "SneakCostEffect" => decode_as::<T, ironsmith_core::SneakCostEffect>(effect),
        "SolveCaseEffect" => decode_as::<T, ironsmith_core::SolveCaseEffect>(effect),
        "SetClassLevelEffect" => decode_as::<T, ironsmith_core::SetClassLevelEffect>(effect),
        "SetDayNightEffect" => decode_as::<T, ironsmith_core::SetDayNightEffect>(effect),
        "SoulbondPairEffect" => decode_as::<T, ironsmith_core::SoulbondPairEffect>(effect),
        "SupportEffect" => decode_as::<T, ironsmith_core::SupportEffect>(effect),
        "SurveilEffect" => decode_as::<T, ironsmith_core::SurveilEffect>(effect),
        "BecomePlottedEffect" => decode_as::<T, ironsmith_core::BecomePlottedEffect>(effect),
        "GrantLoyaltyActivationAllowanceEffect" => {
            decode_as::<T, ironsmith_core::GrantLoyaltyActivationAllowanceEffect>(effect)
        }
        "MustAttackPlayerThisTurnEffect" => {
            decode_as::<T, ironsmith_core::MustAttackPlayerThisTurnEffect>(effect)
        }
        "PrepareEffect" => decode_as::<T, ironsmith_core::PrepareEffect>(effect),
        "SuspectEffect" => decode_as::<T, ironsmith_core::SuspectEffect>(effect),
        "TagAttachedToSourceEffect" => {
            decode_as::<T, ironsmith_core::TagAttachedToSourceEffect>(effect)
        }
        "TagMatchingObjectsEffect" => {
            decode_as::<T, ironsmith_core::TagMatchingObjectsEffect>(effect)
        }
        "TagOtherBlockParticipantEffect" => {
            decode_as::<T, ironsmith_core::TagOtherBlockParticipantEffect>(effect)
        }
        "TagTriggeringAttackerEffect" => {
            decode_as::<T, ironsmith_core::TagTriggeringAttackerEffect>(effect)
        }
        "TagTriggeringBlockersEffect" => {
            decode_as::<T, ironsmith_core::TagTriggeringBlockersEffect>(effect)
        }
        "TagTriggeringDamageTargetEffect" => {
            decode_as::<T, ironsmith_core::TagTriggeringDamageTargetEffect>(effect)
        }
        "TagTriggeringObjectEffect" => {
            decode_as::<T, ironsmith_core::TagTriggeringObjectEffect>(effect)
        }
        "TagTriggeringSourceEffect" => {
            decode_as::<T, ironsmith_core::TagTriggeringSourceEffect>(effect)
        }
        "TaggedEffect" => decode_as::<T, ironsmith_core::TaggedEffect<wire::WireEffect>>(effect),
        "TakeInitiativeEffect" => decode_as::<T, ironsmith_core::TakeInitiativeEffect>(effect),
        "TapEffect" => decode_as::<T, ironsmith_core::TapEffect>(effect),
        "TargetOnlyEffect" => decode_as::<T, ironsmith_core::TargetOnlyEffect>(effect),
        "TicketCountersEffect" => decode_as::<T, ironsmith_core::TicketCountersEffect>(effect),
        "TransformEffect" => decode_as::<T, ironsmith_core::TransformEffect>(effect),
        "TurnFaceDownEffect" => decode_as::<T, ironsmith_core::TurnFaceDownEffect>(effect),
        "ReselectAttackTargetEffect" => {
            decode_as::<T, ironsmith_core::ReselectAttackTargetEffect>(effect)
        }
        "TurnFaceUpEffect" => decode_as::<T, ironsmith_core::TurnFaceUpEffect>(effect),
        "UnattachObjectsEffect" => decode_as::<T, ironsmith_core::UnattachObjectsEffect>(effect),
        "UnearthEffect" => decode_as::<T, ironsmith_core::UnearthEffect>(effect),
        "UnlessActionEffect" => {
            decode_as::<T, ironsmith_core::UnlessActionEffect<wire::WireEffect>>(effect)
        }
        "UnlessPaysEffect" => {
            decode_as::<T, ironsmith_core::UnlessPaysEffect<wire::WireEffect>>(effect)
        }
        "UnlockRoomDoorEffect" => decode_as::<T, ironsmith_core::UnlockRoomDoorEffect>(effect),
        "UntapEffect" => decode_as::<T, ironsmith_core::UntapEffect>(effect),
        "VariableCasualtyPlaneswalkerCopyEffect" => {
            decode_as::<T, ironsmith_core::VariableCasualtyPlaneswalkerCopyEffect>(effect)
        }
        "VentureIntoDungeonEffect" => {
            decode_as::<T, ironsmith_core::VentureIntoDungeonEffect>(effect)
        }
        "VillainousChoiceEffect" => {
            decode_as::<T, ironsmith_core::VillainousChoiceEffect<wire::WireEffect>>(effect)
        }
        "VoteEffect" => decode_as::<T, ironsmith_core::VoteEffect<wire::WireEffect>>(effect),
        "WinTheGameEffect" => decode_as::<T, ironsmith_core::WinTheGameEffect>(effect),
        "WithIdEffect" => decode_as::<T, ironsmith_core::WithIdEffect<wire::WireEffect>>(effect),
        "ImprintFromHandEffect" => decode_as::<T, wire::WireImprintFromHandEffect>(effect),
        "ScaleXValueEffect" => decode_as::<T, wire::WireScaleXValueEffect>(effect),
        _ => None,
    }
}

fn decode_wire_effect<T: 'static>(effect: &wire::WireEffect) -> Option<&T> {
    let kind = effect.kind().to_string();
    effect
        .downcast_with::<T, _>(|payload| ironsmith_artifact_effect_decoder::decode(&kind, payload))
}

impl crate::effect_model_interpreter::EffectModel for WireEffectModel {
    type Effect = wire::WireEffect;
    type StaticAbility = wire::WireStaticAbility;
    type CardDefinition = wire::WireCardDefinition;
    type Ability = wire::WireAbility;
    type EmblemDescription = wire::WireEmblemDescription;
    type ContinuousTarget = wire::WireContinuousTarget;
    type ContinuousModification = wire::WireContinuousModification;
    type RuntimeModification = wire::WireRuntimeModification;
    type Grantable = wire::WireGrantable;
    type GrantDuration = wire::WireGrantDuration;
    type GrantSpec = wire::WireGrantSpec;

    fn downcast_ref<T: 'static>(effect: &Self::Effect) -> Option<&T> {
        decode_wire_effect(effect)
    }

    fn payload_type_name(effect: &Self::Effect) -> &str {
        effect.kind()
    }
}

// The owning restore codec supplies complete retained template snapshots. Keep
// the resolver scoped to this materialization; no global identity/model cache.
struct WireEffectModelHooks<'a> {
    card_definition: &'a mut dyn FnMut(wire::WireCardDefinition) -> Result<crate::cards::CardDefinition, ArtifactMaterializationError>,
}

impl WireEffectModelHooks<'_> {
    fn effect(
        &mut self,
        effect: wire::WireEffect,
    ) -> Result<crate::effect::Effect, ArtifactMaterializationError> {
        crate::effect_model_interpreter::interpret_effect_model::<WireEffectModel, _>(effect, self)
    }

    fn cost(
        &mut self,
        cost: wire::WireCost,
    ) -> Result<crate::costs::Cost, ArtifactMaterializationError> {
        let model = cost.try_map_effect(|effect| self.effect(effect))?;
        crate::costs::Cost::from_model(model)
            .map_err(|detail| ArtifactMaterializationError::UnsupportedEffect { detail })
    }

    fn static_ability(
        &mut self,
        ability: wire::WireStaticAbility,
    ) -> Result<crate::static_abilities::StaticAbility, ArtifactMaterializationError> {
        let hooks = std::cell::RefCell::new(self);
        let model = ability.try_map(
            runtime_trigger_from_core_model,
            |effect| hooks.borrow_mut().effect(effect),
            |cost| hooks.borrow_mut().cost(cost),
            Ok,
        )?;
        Ok(crate::static_abilities::StaticAbility::from_model(model))
    }

    fn ability(
        &mut self,
        ability: wire::WireAbility,
    ) -> Result<crate::ability::Ability, ArtifactMaterializationError> {
        let hooks = std::cell::RefCell::new(self);
        let mut converted = ability.try_map(
            |ability| hooks.borrow_mut().static_ability(ability),
            runtime_trigger_from_core_model,
            |effect| hooks.borrow_mut().effect(effect),
            |cost| hooks.borrow_mut().cost(cost),
            Ok,
        )?;
        match &mut converted.kind {
            crate::ability::AbilityKind::Triggered(triggered) => {
                remove_redundant_target_only_effects_in_program(&mut triggered.effects)
            }
            crate::ability::AbilityKind::Activated(activated) => {
                remove_redundant_target_only_effects_in_program(&mut activated.effects)
            }
            crate::ability::AbilityKind::Static(_) => {}
        }
        Ok(converted)
    }

    fn alternative_cast(
        &mut self,
        method: wire::WireAlternativeCastingMethod,
    ) -> Result<crate::alternative_cast::AlternativeCastingMethod, ArtifactMaterializationError>
    {
        let hooks = std::cell::RefCell::new(self);
        let mut method = method.try_map(
            |effect| hooks.borrow_mut().effect(effect),
            |cost| hooks.borrow_mut().cost(cost),
        )?;
        if let crate::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
            &mut method
        {
            *effects = effects
                .drain(..)
                .filter_map(detarget_overload_effect)
                .collect();
        }
        Ok(method)
    }
}

impl crate::effect_model_interpreter::EffectModelInterpreterHooks<WireEffectModel>
    for WireEffectModelHooks<'_>
{
    type Error = ArtifactMaterializationError;

    fn unsupported_effect(&mut self, detail: String) -> Self::Error {
        ArtifactMaterializationError::UnsupportedEffect { detail }
    }

    fn runtime_static_ability_hook(
        &mut self,
        ability: wire::WireStaticAbility,
    ) -> Result<crate::static_abilities::StaticAbility, Self::Error> {
        self.static_ability(ability)
    }

    fn runtime_card_definition_hook(
        &mut self,
        definition: wire::WireCardDefinition,
    ) -> Result<crate::cards::CardDefinition, Self::Error> {
        (self.card_definition)(definition)
    }

    fn runtime_ability_hook(
        &mut self,
        ability: wire::WireAbility,
    ) -> Result<crate::ability::Ability, Self::Error> {
        self.ability(ability)
    }

    fn runtime_emblem_hook(
        &mut self,
        emblem: wire::WireEmblemDescription,
    ) -> Result<crate::effect::EmblemDescription, Self::Error> {
        let mut converted = crate::effect::EmblemDescription::new(&emblem.name, &emblem.text);
        for ability in emblem.abilities {
            converted = converted.with_ability(self.ability(ability)?);
        }
        Ok(converted)
    }

    fn runtime_continuous_modification_hook(
        &mut self,
        modification: wire::WireContinuousModification,
    ) -> Result<crate::continuous::Modification, Self::Error> {
        let hooks = std::cell::RefCell::new(self);
        crate::continuous::Modification::try_from_model(
            modification,
            |ability| hooks.borrow_mut().static_ability(ability),
            |ability| hooks.borrow_mut().ability(ability),
            |ability| hooks.borrow_mut().ability(ability),
        )
    }

    fn runtime_continuous_runtime_modification_hook(
        &mut self,
        modification: wire::WireRuntimeModification,
    ) -> Result<crate::effects::continuous::RuntimeModification, Self::Error> {
        Ok(match modification {
            wire::WireRuntimeModification::ModifyPowerToughness { power, toughness } => {
                crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                    power,
                    toughness,
                }
            }
            wire::WireRuntimeModification::ChangeControllerToEffectController => {
                crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController
            }
            wire::WireRuntimeModification::ChangeControllerToPlayer(player) => {
                crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)
            }
            wire::WireRuntimeModification::CopyOf {
                source,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                copy_exception_surface,
            } => crate::effects::continuous::RuntimeModification::CopyOf {
                source,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                copy_exception_surface,
            },
            wire::WireRuntimeModification::CopyOfWithAbilities {
                source,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                copy_exception_surface,
                abilities,
            } => crate::effects::continuous::RuntimeModification::CopyOfWithAbilities {
                source,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                copy_exception_surface,
                abilities: abilities
                    .into_iter()
                    .map(|ability| self.ability(ability))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            wire::WireRuntimeModification::RemoveAllAbilities => {
                crate::effects::continuous::RuntimeModification::RemoveAllAbilities
            }
            wire::WireRuntimeModification::RemoveThisAbility => {
                crate::effects::continuous::RuntimeModification::RemoveThisAbility
            }
            wire::WireRuntimeModification::SetAuraAttachmentFilter(filter) => {
                crate::effects::continuous::RuntimeModification::SetAuraAttachmentFilter(filter)
            }
        })
    }

    fn runtime_grantable_hook(
        &mut self,
        grantable: wire::WireGrantable,
    ) -> Result<crate::grant::Grantable, Self::Error> {
        Ok(match grantable {
            wire::WireGrantable::Ability(ability) => {
                crate::grant::Grantable::Ability(self.static_ability(ability)?)
            }
            wire::WireGrantable::AlternativeCast(method) => {
                crate::grant::Grantable::AlternativeCast(self.alternative_cast(method)?)
            }
            wire::WireGrantable::PlayFrom => crate::grant::Grantable::PlayFrom,
            wire::WireGrantable::AlternativePrice { costs, origin } => crate::grant::Grantable::AlternativePrice {
                costs: costs.into_iter().map(|cost| self.cost(cost)).collect::<Result<_, _>>()?, origin,
            },
            wire::WireGrantable::DerivedAlternativeCast(spec) => {
                crate::grant::Grantable::DerivedAlternativeCast(spec.try_map(|cost| self.cost(cost))?)
            }
        })
    }

    fn runtime_grant_duration_hook(
        &mut self,
        duration: wire::WireGrantDuration,
    ) -> Result<crate::grant::GrantDuration, Self::Error> {
        match duration {
            wire::WireGrantDuration::Forever => Ok(crate::grant::GrantDuration::Forever),
            wire::WireGrantDuration::UntilEndOfTurn => {
                Ok(crate::grant::GrantDuration::UntilEndOfTurn)
            }
            wire::WireGrantDuration::UntilYourNextTurn => {
                Ok(crate::grant::GrantDuration::UntilYourNextTurn)
            }
            wire::WireGrantDuration::UntilYourNextTurnEnd => {
                Ok(crate::grant::GrantDuration::UntilYourNextTurnEnd)
            }
        }
    }

    fn runtime_grant_spec_hook(
        &mut self,
        spec: wire::WireGrantSpec,
    ) -> Result<crate::grant::GrantSpec, Self::Error> {
        Ok(crate::grant::GrantSpec {
            grantable: self.runtime_grantable_hook(spec.grantable)?,
            filter: spec.filter,
            zone: spec.zone,
            additional_zones: spec.additional_zones,
            beneficiary: spec.beneficiary,
            usage_limit: spec.usage_limit,
            max_plays: spec.max_plays,
            cast_this_way_filter: spec.cast_this_way_filter,
            on_use_effects: spec.on_use_effects.into_iter().map(|effect|
                runtime_effect_from_core_model_with_card_definitions(effect, self.card_definition)).collect::<Result<_, _>>()?,
            requires_linked_exile_pair: spec.requires_linked_exile_pair,
            may_look_at_linked_exile: spec.may_look_at_linked_exile,
            cast_mana_spend_mode: spec.cast_mana_spend_mode,
            linked_exile_pair: spec.linked_exile_pair,
            linked_exile_class_level: spec.linked_exile_class_level,
            source_exiled_surface: spec.source_exiled_surface,
            filtered_zone_surface: spec.filtered_zone_surface,
            top_card_only: spec.top_card_only,
            instant_timing: spec.instant_timing,
            may_look_at_top: spec.may_look_at_top,
            cast_this_way_grants: spec
                .cast_this_way_grants
                .into_iter()
                .map(|ability| self.runtime_static_ability_hook(ability))
                .collect::<Result<Vec<_>, _>>()?,
            permanent_this_way_grants: spec
                .permanent_this_way_grants
                .into_iter()
                .map(|ability| self.runtime_static_ability_hook(ability))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    fn retain_runtime_effect_model_hook(
        &mut self,
        model: &wire::WireEffect,
        effect: crate::effect::Effect,
    ) -> Result<crate::effect::Effect, Self::Error> {
        let json = serde_json::to_string(model).map_err(|error| {
            ArtifactMaterializationError::UnsupportedEffect {
                detail: format!("canonical effect model cannot be encoded: {error}"),
            }
        })?;
        Ok(effect.with_serialized_model(json))
    }

    fn runtime_external_model_effect_hook(
        &mut self,
        effect: &wire::WireEffect,
    ) -> Result<Option<crate::effect::Effect>, Self::Error> {
        if let Some(payload) = decode_wire_effect::<wire::WireImprintFromHandEffect>(effect) {
            return Ok(Some(crate::effect::Effect::new(
                crate::effects::cards::ImprintFromHandEffect::new(payload.filter.clone()),
            )));
        }
        if let Some(payload) = decode_wire_effect::<wire::WireScaleXValueEffect>(effect) {
            return Ok(Some(crate::effect::Effect::scale_x_value(
                payload.target.clone(),
                payload.multiplier,
            )));
        }
        Ok(None)
    }
}

fn runtime_effect_from_core_model(
    effect: wire::WireEffect,
) -> Result<crate::effect::Effect, ArtifactMaterializationError> {
    runtime_effect_from_core_model_with_card_definitions(effect, &mut runtime_definition_from_core_model)
}

/// Composition nodes retain the same scoped resolver while converting their
/// child effects. The caller validates/consumes explicit retained associations;
/// this boundary does not infer identity from CardId or model equality.
fn runtime_effect_from_core_model_with_card_definitions(
    effect: wire::WireEffect,
    card_definition: &mut dyn FnMut(wire::WireCardDefinition) -> Result<crate::cards::CardDefinition, ArtifactMaterializationError>,
) -> Result<crate::effect::Effect, ArtifactMaterializationError> {
    crate::effect_model_interpreter::interpret_effect_model::<WireEffectModel, _>(
        effect,
        &mut WireEffectModelHooks { card_definition },
    )
}

fn remove_redundant_target_only_effects_in_program(
    program: &mut crate::resolution::ResolutionProgram,
) {
    crate::effect_model_interpreter::prune_redundant_target_only_effects_in_program(program);
}

fn runtime_cost_from_core_model(
    cost: wire::WireCost,
) -> Result<crate::costs::Cost, ArtifactMaterializationError> {
    let model = cost.try_map_effect(runtime_effect_from_core_model)?;
    crate::costs::Cost::from_model(model)
        .map_err(|detail| ArtifactMaterializationError::UnsupportedEffect { detail })
}

fn runtime_optional_cost_from_core_model(
    cost: wire::WireOptionalCost,
) -> Result<crate::cost::OptionalCost, ArtifactMaterializationError> {
    cost.try_map(runtime_cost_from_core_model)
}

fn convert_alternative_cast(
    method: wire::WireAlternativeCastingMethod,
) -> Result<crate::alternative_cast::AlternativeCastingMethod, ArtifactMaterializationError> {
    let mut method =
        method.try_map(runtime_effect_from_core_model, runtime_cost_from_core_model)?;
    if let crate::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } = &mut method
    {
        *effects = effects
            .drain(..)
            .filter_map(detarget_overload_effect)
            .collect();
    }
    Ok(method)
}

fn detarget_overload_effect(effect: crate::effect::Effect) -> Option<crate::effect::Effect> {
    if effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .is_some()
    {
        return None;
    }

    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let inner = detarget_overload_effect((*tagged.effect).clone())?;
        return Some(crate::effect::Effect::new(tagged.with_effect(inner)));
    }

    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
        && let Some(crate::target::ChooseSpec::Target(inner)) = &apply.target_spec
        && let crate::target::ChooseSpec::Object(filter) = inner.as_ref()
    {
        let mut detargeted = apply.clone();
        detargeted.target = crate::continuous::EffectTarget::Filter(filter.clone());
        detargeted.target_spec = Some(crate::target::ChooseSpec::Object(filter.clone()));
        detargeted.require_creature_target = false;
        return Some(crate::effect::Effect::new(detargeted));
    }

    Some(effect)
}

fn convert_derived_alternative_cast(
    spec: wire::WireDerivedAlternativeCast,
) -> Result<crate::grant::DerivedAlternativeCast, ArtifactMaterializationError> {
    Ok(match spec {
        wire::WireDerivedAlternativeCast::FlashbackFromCardManaCost { additional_costs } => {
            crate::grant::DerivedAlternativeCast::FlashbackFromCardManaCost {
                additional_costs: additional_costs
                    .into_iter()
                    .map(runtime_cost_from_core_model)
                    .collect::<Result<Vec<_>, _>>()?,
            }
        }
        wire::WireDerivedAlternativeCast::EscapeFromCardManaCost { exile_count } => {
            crate::grant::DerivedAlternativeCast::EscapeFromCardManaCost { exile_count }
        }
        wire::WireDerivedAlternativeCast::RetraceFromCardManaCost => {
            crate::grant::DerivedAlternativeCast::RetraceFromCardManaCost
        }
        wire::WireDerivedAlternativeCast::BlitzFromCardManaCost => {
            crate::grant::DerivedAlternativeCast::BlitzFromCardManaCost
        }
        wire::WireDerivedAlternativeCast::EmergeFromCardManaCost => {
            crate::grant::DerivedAlternativeCast::EmergeFromCardManaCost
        }
        wire::WireDerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction } => {
            crate::grant::DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction }
        }
        wire::WireDerivedAlternativeCast::ManaValueAsGenericFromHand => {
            crate::grant::DerivedAlternativeCast::ManaValueAsGenericFromHand
        }
        wire::WireDerivedAlternativeCast::MadnessFromCardManaCost => {
            crate::grant::DerivedAlternativeCast::MadnessFromCardManaCost
        }
        wire::WireDerivedAlternativeCast::LifeEqualManaValueFromHand { usage_limit } => {
            crate::grant::DerivedAlternativeCast::LifeEqualManaValueFromHand { usage_limit }
        }
        wire::WireDerivedAlternativeCast::LifeEqualManaValueFromZone { zone, usage_limit } => {
            crate::grant::DerivedAlternativeCast::LifeEqualManaValueFromZone { zone, usage_limit }
        }
        wire::WireDerivedAlternativeCast::GraveyardCastFromCardManaCost {
            additional_costs,
            usage_limit,
            condition,
            exiles_after_resolution,
        } => crate::grant::DerivedAlternativeCast::GraveyardCastFromCardManaCost {
            additional_costs: additional_costs
                .into_iter()
                .map(runtime_cost_from_core_model)
                .collect::<Result<Vec<_>, _>>()?,
            usage_limit,
            condition,
            exiles_after_resolution,
        },
    })
}

fn runtime_static_ability_model(
    ability: wire::WireStaticAbility,
) -> Result<crate::static_abilities::CompiledStaticAbility, ArtifactMaterializationError> {
    ability.try_map(
        runtime_trigger_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
        // The wire form already carries resolved conditions.
        Ok,
    )
}

fn runtime_static_ability(
    ability: wire::WireStaticAbility,
) -> Result<crate::static_abilities::StaticAbility, ArtifactMaterializationError> {
    Ok(crate::static_abilities::StaticAbility::from_model(
        runtime_static_ability_model(ability)?,
    ))
}

fn runtime_trigger_from_core_model(
    trigger: wire::WireTrigger,
) -> Result<crate::triggers::Trigger, ArtifactMaterializationError> {
    crate::triggers::Trigger::from_model(trigger)
        .map_err(|err| ArtifactMaterializationError::UnsupportedTrigger { detail: err.detail })
}

fn runtime_ability_from_core_model(
    ability: wire::WireAbility,
) -> Result<crate::ability::Ability, ArtifactMaterializationError> {
    let mut converted = ability.try_map(
        runtime_static_ability,
        runtime_trigger_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
        // The wire form already carries resolved conditions.
        Ok,
    )?;
    match &mut converted.kind {
        crate::ability::AbilityKind::Triggered(triggered) => {
            remove_redundant_target_only_effects_in_program(&mut triggered.effects);
        }
        crate::ability::AbilityKind::Activated(activated) => {
            remove_redundant_target_only_effects_in_program(&mut activated.effects);
        }
        crate::ability::AbilityKind::Static(_) => {}
    }
    Ok(converted)
}

fn combine_level_ability_statics(
    abilities: Vec<crate::ability::Ability>,
) -> Vec<crate::ability::Ability> {
    let mut out = Vec::with_capacity(abilities.len());
    let mut groups: Vec<(Vec<crate::zone::Zone>, Vec<crate::ability::LevelAbility>)> = Vec::new();

    for ability in abilities {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            out.push(ability);
            continue;
        };
        let Some(level_abilities) = static_ability.level_abilities() else {
            out.push(ability);
            continue;
        };
        // Combining level rows must not broaden or discard their source zones.
        if let Some((_, levels)) = groups
            .iter_mut()
            .find(|(zones, _)| *zones == ability.functional_zones)
        {
            levels.extend(level_abilities.iter().cloned());
        } else {
            groups.push((ability.functional_zones, level_abilities.to_vec()));
        }
    }

    for (zones, levels) in groups {
        out.push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::with_level_abilities(levels),
            )
            .in_zones(zones),
        );
    }
    out
}

const CLASS_LEVEL_MARKER_PREFIX: &str = "__ironsmith_class_level:";

fn class_level_marker(ability: &crate::ability::ActivatedAbility) -> Option<u32> {
    if let Some(ironsmith_core::ActivatedAbilityKeyword::ClassLevel(level)) = ability.keyword {
        return Some(level);
    }
    // Previously admitted definitions retain their legacy runtime route. New
    // definition-local pairing consumes only the typed keyword above.
    ability
        .additional_restrictions
        .iter()
        .find_map(|restriction| restriction.strip_prefix(CLASS_LEVEL_MARKER_PREFIX))
        .and_then(|level| level.parse::<u32>().ok())
}

fn class_level_activation_condition(level: u32) -> crate::ConditionExpr {
    // CR 716.2a: "Level N" can be activated only while the Class is level
    // N-1. Levels are a designation, not level counters (CR 716.4).
    let previous = level.saturating_sub(1).max(1);
    crate::ConditionExpr::And(
        Box::new(crate::ConditionExpr::SourceClassLevelAtLeast(previous)),
        Box::new(crate::ConditionExpr::Not(Box::new(
            crate::ConditionExpr::SourceClassLevelAtLeast(previous + 1),
        ))),
    )
}

fn and_condition(
    left: Option<crate::ConditionExpr>,
    right: crate::ConditionExpr,
) -> crate::ConditionExpr {
    left.map(|left| crate::ConditionExpr::And(Box::new(left), Box::new(right.clone())))
        .unwrap_or(right)
}

fn apply_class_level_runtime_gates(definition: &mut crate::cards::CardDefinition) {
    if !definition.card.subtypes.contains(&crate::Subtype::Class) {
        return;
    }

    let mut current_level = None;
    for ability in &mut definition.abilities {
        if let crate::ability::AbilityKind::Activated(activated) = &mut ability.kind
            && let Some(level) = class_level_marker(activated)
        {
            activated.activation_condition = Some(and_condition(
                activated.activation_condition.take(),
                class_level_activation_condition(level),
            ));
            // The level ability sets the Class's level designation instead of
            // putting a level counter on it (CR 716.2b).
            activated.effects = vec![crate::effect::Effect::new(
                crate::effects::SetClassLevelEffect::new(level),
            )]
            .into();
            current_level = Some(level);
            continue;
        }

        let Some(level) = current_level else {
            continue;
        };
        if let crate::ability::AbilityKind::Static(static_ability) = &mut ability.kind {
            // Classes start at level 1. Grant the entire static ability so its
            // existing conditions stay intact.
            *static_ability = crate::static_abilities::StaticAbility::new(
                crate::static_abilities::GrantAbility::source(static_ability.clone())
                    .with_condition(crate::ConditionExpr::SourceClassLevelAtLeast(level)),
            );
        }
        if let crate::ability::AbilityKind::Triggered(triggered) = &mut ability.kind
            && triggered.presentation_label.is_none()
        {
            triggered.presentation_label =
                Some(crate::ability::PresentationLabel::from_ability_word(
                    format!("{CLASS_LEVEL_MARKER_PREFIX}{level}"),
                ));
        }
    }
}

fn runtime_definition_from_core_model(
    definition: wire::WireCardDefinition,
) -> Result<crate::cards::CardDefinition, ArtifactMaterializationError> {
    let mut definition = definition.try_map(
        runtime_ability_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
        convert_alternative_cast,
        runtime_optional_cost_from_core_model,
    )?;
    definition.abilities = combine_level_ability_statics(definition.abilities);
    apply_class_level_runtime_gates(&mut definition);
    if let Some(spell_effect) = &mut definition.spell_effect {
        remove_redundant_target_only_effects_in_program(spell_effect);
    }
    Ok(definition)
}

/// Restore one canonical executable effect using the artifact interpreter.
pub fn materialize_effect(
    effect: wire::WireEffect,
) -> Result<crate::effect::Effect, ArtifactMaterializationError> {
    runtime_effect_from_core_model(effect)
}

/// Failure to encode a native executable payload without losing semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimePayloadEncodingError {
    MissingModel { component: &'static str },
    InvalidEffectModel { detail: String },
}

impl std::fmt::Display for RuntimePayloadEncodingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingModel { component } => {
                write!(
                    formatter,
                    "native {component} has no retained canonical model"
                )
            }
            Self::InvalidEffectModel { detail } => {
                write!(formatter, "native effect model cannot be decoded: {detail}")
            }
        }
    }
}

impl std::error::Error for RuntimePayloadEncodingError {}

/// Project the complete native definition into canonical executable models.
/// This preserves every printed field and payload without interpreting or
/// normalizing it again. Occurrence identities stay in the owning codec.
pub fn encode_runtime_definition(
    definition: crate::cards::CardDefinition,
) -> Result<wire::WireCardDefinition, RuntimePayloadEncodingError> {
    definition.try_map(
        encode_runtime_ability,
        encode_runtime_effect,
        encode_runtime_cost,
        encode_runtime_alternative_cast,
        encode_runtime_optional_cost,
    )
}

macro_rules! with_native_direct_effect_types {
    ($consumer:ident) => {
        $consumer!(
            crate::effects::AddManaEffect,
            crate::effects::AddManaFromCommanderColorIdentityEffect,
            crate::effects::AddManaOfAnyColorEffect,
            crate::effects::AddManaOfAnyOneColorEffect,
            crate::effects::AddManaOfChosenColorEffect,
            crate::effects::AddManaOfColorsAmongEffect,
            crate::effects::AddManaOfNotedTypeEffect,
            crate::effects::AddOneManaOfAnyColorAmongEffect,
            crate::effects::AddScaledManaEffect,
            crate::effects::AdditionalPhasesEffect,
            crate::effects::AmassEffect,
            crate::effects::CollectEvidenceEffect,
            crate::effects::EmpowerJaceEffect,
            crate::effects::AmplifyEffect,
            crate::effects::AscendEffect,
            crate::effects::AssignNoCombatDamageEffect,
            crate::effects::AttachObjectsEffect,
            crate::effects::AttachToEffect,
            crate::effects::AuraSwapEffect,
            crate::effects::BecomeBasicLandTypeChoiceEffect,
            crate::effects::BecomeColorChoiceEffect,
            crate::effects::BecomeCreatureTypeChoiceEffect,
            crate::effects::BolsterEffect,
            crate::effects::CantEffect,
            crate::effects::CastSourceEffect,
            crate::effects::ChooseCardNameEffect,
            crate::effects::ChooseCardTypeEffect,
            crate::effects::RippleEffect,
            crate::effects::ChooseNumberEffect,
            crate::effects::ChooseNewTargetsEffect,
            crate::effects::ChooseObjectsEffect,
            crate::effects::ChooseSpellCastHistoryEffect,
            crate::effects::CipherEffect,
            crate::effects::ClashEffect,
            crate::effects::ConspireCostEffect,
            crate::effects::ConsultTopOfLibraryEffect,
            crate::effects::ControlPlayerEffect,
            crate::effects::CopySpellEffect,
            crate::effects::CopySpellForEachTargetEffect,
            crate::effects::CounterEffect,
            crate::effects::CrewCostEffect,
            crate::effects::DealDamageEffect,
            crate::effects::DealDamageToRecipientsEffect,
            crate::effects::DealDamageBySourcesEffect,
            crate::effects::DealDamageEachEffect,
            crate::effects::DevourEffect,
            crate::effects::DirectionalAdjacentPlayerControlEffect,
            crate::effects::DiscardHandEffect,
            crate::effects::DiscoverEffect,
            crate::effects::DrawCardsEffect,
            crate::effects::EachPlayerScryEffect,
            crate::effects::EarthbendEffect,
            crate::effects::EmitKeywordActionEffect,
            crate::effects::EnergyCountersEffect,
            crate::effects::EvolveEffect,
            crate::effects::ExchangeControlEffect,
            crate::effects::ExchangeTextBoxesEffect,
            crate::effects::ExertCostEffect,
            crate::effects::ExileEffect,
            crate::effects::ExileInsteadOfGraveyardEffect,
            crate::effects::ExileUntilEffect,
            crate::effects::ExtraTurnAfterNextTurnEffect,
            crate::effects::ExtraTurnEffect,
            crate::effects::FlipEffect,
            crate::effects::GainLifeEffect,
            crate::effects::GrantEndThisEffectPaymentEffect,
            crate::effects::GrantNextSpellCostReductionEffect,
            crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
            crate::effects::GrantTaggedSpellLifeCostByManaValueEffect,
            crate::effects::HealDamageEffect,
            crate::effects::IncreaseSpeedEffect,
            crate::effects::IncubateEffect,
            crate::effects::LearnEffect,
            crate::effects::LookAtHandEffect,
            crate::effects::LookAtObjectsEffect,
            crate::effects::LookAtTopCardsEffect,
            crate::effects::LoseTheGameEffect,
            crate::effects::ManifestObjectsEffect,
            crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect,
            crate::effects::MayMoveToZoneEffect,
            crate::effects::MeldEffect,
            crate::effects::MillEffect,
            crate::effects::ModifyPowerToughnessEffect,
            crate::effects::ModifyPowerToughnessForEachEffect,
            crate::effects::MonstrosityEffect,
            crate::effects::MoveAllCountersEffect,
            crate::effects::MoveCountersEffect,
            crate::effects::MoveOneCounterEffect,
            crate::effects::MoveToLibraryNthFromTopEffect,
            crate::effects::MoveToLibraryTopOrBottomChoiceEffect,
            crate::effects::MoveToZoneEffect,
            crate::effects::NinjutsuCostEffect,
            crate::effects::NinjutsuEffect,
            crate::effects::NoteActivationManaTypeEffect,
            crate::effects::PayAnyEnergyEffect,
            crate::effects::PayAnyLifeEffect,
            crate::effects::PayEnergyEffect,
            crate::effects::PayLifeEffect,
            crate::effects::PayManaEffect,
            crate::effects::PopulateEffect,
            crate::effects::PreventAllCombatDamageEffect,
            crate::effects::ProliferateEffect,
            crate::effects::PutCountersEffect,
            crate::effects::PutOntoBattlefieldEffect,
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
            crate::effects::RearrangeLookedCardsInLibraryEffect,
            crate::effects::ReconfigureEffect,
            crate::effects::ReduceSpeedEffect,
            crate::effects::RegisterCounterPlacementReplacementEffect,
            crate::effects::RegisterDamageMultiplierEffect,
            crate::effects::RegisterDamageAdditionEffect,
            crate::effects::RegisterEnterTappedReplacementEffect,
            crate::effects::RegisterFutureZoneReplacementEffect,
            crate::effects::RegisterManaReplacementEffect,
            crate::effects::RegisterManaRewriteEffect,
            crate::effects::RegisterManaSpendPermissionEffect,
            crate::effects::RegisterNextBatchEnterWithCountersEffect,
            crate::effects::RemoveAnyCountersAmongEffect,
            crate::effects::RemoveAnyCountersFromSourceEffect,
            crate::effects::RemoveCountersEffect,
            crate::effects::RemoveUpToAnyCountersEffect,
            crate::effects::RenownEffect,
            crate::effects::ReorderLibraryTopEffect,
            crate::effects::ReorderTopPlanarDeckEffect,
            crate::effects::RestartGameEffect,
            crate::effects::RetainManaUntilEndOfTurnEffect,
            crate::effects::RetargetStackObjectEffect,
            crate::effects::ReturnAllToBattlefieldEffect,
            crate::effects::ReturnFromGraveyardToBattlefieldEffect,
            crate::effects::ReturnFromGraveyardToHandEffect,
            crate::effects::ReturnToHandEffect,
            crate::effects::RevealFromHandEffect,
            crate::effects::RevealSourceFromHandEffect,
            crate::effects::RevealTaggedEffect,
            crate::effects::RevealTopEffect,
            crate::effects::ReverseTurnOrderEffect,
            crate::effects::SacrificeTargetEffect,
            crate::effects::ScryEffect,
            crate::effects::SearchLibraryEffect,
            crate::effects::SearchLibrarySlotsEffect,
            crate::effects::SetBasePowerToughnessEffect,
            crate::effects::SetClassLevelEffect,
            crate::effects::SetDayNightEffect,
            crate::effects::ShuffleLibraryEffect,
            crate::effects::ShuffleObjectsIntoLibraryEffect,
            crate::effects::SneakCostEffect,
            crate::effects::SolveCaseEffect,
            crate::effects::SoulbondPairEffect,
            crate::effects::TagAttachedToSourceEffect,
            crate::effects::TagMatchingObjectsEffect,
            crate::effects::TagOtherBlockParticipantEffect,
            crate::effects::TagTriggeringAttackerEffect,
            crate::effects::TagTriggeringBlockersEffect,
            crate::effects::TagTriggeringObjectEffect,
            crate::effects::TagTriggeringSourceEffect,
            crate::effects::TapEffect,
            crate::effects::TargetOnlyEffect,
            crate::effects::TicketCountersEffect,
            crate::effects::TransformEffect,
            crate::effects::TurnFaceDownEffect,
            crate::effects::TurnFaceUpEffect,
            crate::effects::ReselectAttackTargetEffect,
            crate::effects::UnattachObjectsEffect,
            crate::effects::UnearthEffect,
            crate::effects::UntapEffect,
            crate::effects::VariableCasualtyPlaneswalkerCopyEffect,
            crate::effects::WinTheGameEffect,
            crate::effects::composition::ResolvesDespiteIllegalTargetsEffect,
            crate::effects::mana::AddManaOfImprintedColorsEffect,
            crate::effects::player::MayCastForMiracleCostEffect,
            crate::effects::zones::SacrificePlayerEffect,
        );
    };
}

pub fn encode_runtime_effect(
    effect: crate::effect::Effect,
) -> Result<wire::WireEffect, RuntimePayloadEncodingError> {
    if let Some(json) = effect.serialized_model() {
        return serde_json::from_str(json).map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel {
            detail: error.to_string(),
        });
    }
    // Native producers use the same typed executable vocabulary as compiled
    // producers. Preserve every field, including nested definition payloads.
    macro_rules! encode_direct {
        ($($ty:path),* $(,)?) => {
            $(
                if let Some(payload) = effect.downcast_ref::<$ty>() {
                    let kind = wire::stable_payload_kind(std::any::type_name::<$ty>());
                    return serde_json::to_value(payload)
                        .map(|payload| wire::WireEffect::new(kind, payload))
                        .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
                }
            )*
        };
    }
    with_native_direct_effect_types!(encode_direct);
    if let Some(payload) = effect.downcast_ref::<crate::effects::RedirectNextTimeDamageToSourceEffect>() {
        let source = match &payload.source {
            crate::effects::RedirectNextTimeDamageSource::Choice => ironsmith_core::RedirectNextTimeDamageSource::Choice,
            crate::effects::RedirectNextTimeDamageSource::Filter(filter) => ironsmith_core::RedirectNextTimeDamageSource::Filter(filter.clone()),
            crate::effects::RedirectNextTimeDamageSource::Target(target) => ironsmith_core::RedirectNextTimeDamageSource::Target(target.clone()),
        };
        let destination = match payload.destination {
            crate::effects::RedirectNextTimeDamageDestination::DamageSource => ironsmith_core::RedirectNextTimeDamageDestination::DamageSource,
            crate::effects::RedirectNextTimeDamageDestination::SourceObject => ironsmith_core::RedirectNextTimeDamageDestination::SourceObject,
            crate::effects::RedirectNextTimeDamageDestination::Controller => ironsmith_core::RedirectNextTimeDamageDestination::Controller,
            crate::effects::RedirectNextTimeDamageDestination::SourceController => ironsmith_core::RedirectNextTimeDamageDestination::SourceController,
            crate::effects::RedirectNextTimeDamageDestination::TargetObject => ironsmith_core::RedirectNextTimeDamageDestination::TargetObject,
        };
        let converted = ironsmith_core::RedirectNextTimeDamageToSourceEffect {
            source,
            combat_only: payload.combat_only,
            target: payload.target.clone(),
            destination,
            destination_target: payload.destination_target.clone(),
            all_this_turn: payload.all_this_turn,
        };
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("RedirectNextTimeDamageToSourceEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }
    if let Some(payload) = effect.downcast_ref::<crate::effects::GrantNextSpellAbilityEffect>() {
        let converted = ironsmith_core::GrantNextSpellAbilityEffect::new(
            payload.player.clone(), payload.filter.clone(),
            encode_runtime_ability(payload.ability.clone())?,
        ).with_mode(payload.mode);
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("GrantNextSpellAbilityEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }
    if let Some(payload) = effect.downcast_ref::<crate::effects::PreventAllDamageEffect>() {
        let converted = payload.clone().try_map_effects(encode_runtime_effect)?;
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("PreventAllDamageEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel {
                detail: error.to_string(),
            });
    }
    if let Some(payload) = effect.downcast_ref::<crate::effects::PreventDamageEffect>() {
        let converted = ironsmith_core::PreventDamageEffect {
            amount: payload.amount.clone(), target: payload.target.clone(),
            until: payload.duration.clone(), damage_filter: payload.damage_filter.clone(),
            source_of_your_choice: payload.source_of_your_choice,
            protect_you_and_permanents_you_control: payload.protect_you_and_permanents_you_control,
            divided: payload.divided,
            follow_up_effects: payload.follow_up_effects.iter().cloned()
                .map(encode_runtime_effect).collect::<Result<Vec<_>, _>>()?,
        };
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("PreventDamageEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }
    if let Some(payload) = effect.downcast_ref::<crate::effects::PreventAllDamageToTargetEffect>() {
        let converted = ironsmith_core::PreventAllDamageToTargetEffect {
            target: payload.target.clone(), until: payload.duration.clone(),
            combat_only: payload.damage_filter.combat_only,
            damage_filter: payload.damage_filter.clone(),
            source_color_of_your_choice: payload.source_color_of_your_choice,
            follow_up_effects: payload.follow_up_effects.iter().cloned()
                .map(encode_runtime_effect).collect::<Result<Vec<_>, _>>()?,
        };
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("PreventAllDamageToTargetEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }
    if let Some(payload) = effect.downcast_ref::<crate::effects::CastTaggedEffect>() {
        let converted = payload.clone().try_map_cost(encode_runtime_cost)?;
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("CastTaggedEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }

    if let Some(payload) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        let ironsmith_core::CreateTokenEffect {
            token, text_roles, count, controller, controller_target, use_source_chosen_color,
            use_source_chosen_creature_type, actor_surface_explicit,
            suppress_aura_attachment_choice, ability_presentation, enters_tapped,
            enters_attacking, attack_target_mode, enters_blocking,
            exile_at_end_of_combat, sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step, exile_at_next_end_step,
            next_end_step_player, link_source_exiled_this_resolution,
        } = payload.clone();
        let converted = ironsmith_core::CreateTokenEffect {
            token: encode_runtime_definition(token)?, text_roles, count, controller,
            controller_target, use_source_chosen_color,
            use_source_chosen_creature_type, actor_surface_explicit,
            suppress_aura_attachment_choice, ability_presentation, enters_tapped,
            enters_attacking, attack_target_mode, enters_blocking,
            exile_at_end_of_combat, sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step, exile_at_next_end_step,
            next_end_step_player, link_source_exiled_this_resolution,
        };
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("CreateTokenEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }
    // These native composition owners preserve exactly the same recursive
    // typed payload as compilation, including order flags and scoped tags.
    if let Some(payload) = effect.downcast_ref::<crate::effects::CollectManaPaymentsEffect>() {
        let converted = ironsmith_core::CollectManaPaymentsEffect::new(
            payload.effects.iter().cloned().map(encode_runtime_effect).collect::<Result<Vec<_>, _>>()?,
        );
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("CollectManaPaymentsEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }
    if let Some(payload) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let converted = ironsmith_core::SequenceEffect {
            effects: payload.effects.iter().cloned().map(encode_runtime_effect).collect::<Result<Vec<_>, _>>()?,
            surface: payload.surface, result_label: payload.result_label.clone(),
        };
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("SequenceEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }
    if let Some(payload) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        let converted = ironsmith_core::ForPlayersEffect {
            effects: payload.effects.iter().cloned().map(encode_runtime_effect).collect::<Result<Vec<_>, _>>()?,
            filter: payload.filter.clone(), sequential: payload.sequential,
            starting_with_controller: payload.starting_with_controller,
            stop_after_first_happened: payload.stop_after_first_happened,
        };
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("ForPlayersEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }
    if let Some(payload) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        let converted = ironsmith_core::ForEachTaggedEffect {
            effects: payload.effects.iter().cloned().map(encode_runtime_effect).collect::<Result<Vec<_>, _>>()?,
            tag: payload.tag.clone(), controller_at_last_blocked_by: payload.controller_at_last_blocked_by.clone(),
        };
        return serde_json::to_value(converted)
            .map(|payload| wire::WireEffect::new("ForEachTaggedEffect", payload))
            .map_err(|error| RuntimePayloadEncodingError::InvalidEffectModel { detail: error.to_string() });
    }
    if let Some(model) = text_program_codec::encode_text_changed_native_effect(&effect)? { return Ok(model); }
    if let Some(model) = continuous_word_codec::encode_continuous_native_effect(&effect)? { return Ok(model); }
    Err(RuntimePayloadEncodingError::MissingModel { component: "effect" })
}

pub fn encode_runtime_cost(
    cost: crate::costs::Cost,
) -> Result<wire::WireCost, RuntimePayloadEncodingError> {
    cost.compiled_model()
        .ok_or(RuntimePayloadEncodingError::MissingModel { component: "cost" })?
        .clone()
        .try_map_effect(encode_runtime_effect)
}

pub fn encode_runtime_trigger(
    trigger: crate::triggers::Trigger,
) -> Result<wire::WireTrigger, RuntimePayloadEncodingError> {
    trigger
        .compiled_model()
        .cloned()
        .ok_or(RuntimePayloadEncodingError::MissingModel {
            component: "trigger",
        })
}

/// Encode the complete static model, including nested executable payloads.
/// Occurrence identity belongs to the enclosing descriptor's provenance codec.
pub fn encode_runtime_static_ability(
    ability: crate::static_abilities::StaticAbility,
) -> Result<wire::WireStaticAbility, RuntimePayloadEncodingError> {
    ability
        .canonical_model()
        .ok_or(RuntimePayloadEncodingError::MissingModel {
            component: "static ability",
        })?
        .try_map(
            encode_runtime_trigger,
            encode_runtime_effect,
            encode_runtime_cost,
            Ok,
        )
}

pub fn encode_runtime_ability(
    ability: crate::ability::Ability,
) -> Result<wire::WireAbility, RuntimePayloadEncodingError> {
    ability.try_map(
        encode_runtime_static_ability,
        encode_runtime_trigger,
        encode_runtime_effect,
        encode_runtime_cost,
        Ok,
    )
}

/// Restore a retained ability exactly, without card-definition normalization.
/// Static occurrence identity must be restored separately by the owning codec.
pub fn restore_runtime_ability(
    ability: wire::WireAbility,
) -> Result<crate::ability::Ability, ArtifactMaterializationError> {
    ability.try_map(
        runtime_static_ability,
        runtime_trigger_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
        Ok,
    )
}

/// Encode every copy characteristic and executable ability model.
pub fn encode_runtime_copy_values(
    values: crate::snapshot::CopiableValues,
) -> Result<crate::snapshot::RetainedCopiableValues<wire::WireAbility, wire::WireEffect>, RuntimePayloadEncodingError>
{
    crate::snapshot::RetainedCopiableValues::from(values).try_map_abilities(encode_runtime_ability)?
        .try_map_effects(encode_runtime_effect)
}

pub fn restore_runtime_copy_values(
    values: crate::snapshot::RetainedCopiableValues<wire::WireAbility, wire::WireEffect>,
) -> Result<crate::snapshot::CopiableValues, ArtifactMaterializationError> {
    values
        .try_map_abilities(restore_runtime_ability)?
        .try_map_effects(materialize_effect)
        .map(Into::into)
}

/// Encode complete rules text, ability labels and executable ability models.
pub fn encode_runtime_text_overlay(
    overlay: crate::continuous::TextBoxOverlay,
) -> Result<crate::continuous::RetainedTextBoxOverlay<wire::WireAbility>, RuntimePayloadEncodingError>
{
    crate::continuous::RetainedTextBoxOverlay::from(overlay)
        .try_map_abilities(encode_runtime_ability)
}

pub fn restore_runtime_text_overlay(
    overlay: crate::continuous::RetainedTextBoxOverlay<wire::WireAbility>,
) -> Result<crate::continuous::TextBoxOverlay, ArtifactMaterializationError> {
    overlay
        .try_map_abilities(restore_runtime_ability)
        .map(Into::into)
}

/// Encode the full restriction; occurrence aliases are restored by the owner.
pub fn encode_runtime_restriction(
    value: crate::continuous::RegisteredRestriction,
) -> Result<
    crate::continuous::RetainedRestriction<wire::WireStaticAbility>,
    RuntimePayloadEncodingError,
> {
    crate::continuous::RetainedRestriction::from(value)
        .try_map_ability(encode_runtime_static_ability)
}

pub fn restore_runtime_restriction(
    value: crate::continuous::RetainedRestriction<wire::WireStaticAbility>,
) -> Result<crate::continuous::RegisteredRestriction, ArtifactMaterializationError> {
    value
        .try_map_ability(runtime_static_ability)?
        .try_into()
        .map_err(|error: crate::continuous::RestrictionAbilityMismatch| {
            ArtifactMaterializationError::UnsupportedStaticAbility {
                detail: error.to_string(),
            }
        })
}

pub fn encode_runtime_aura_metadata(
    value: crate::object::AuraAttachmentMetadata,
) -> Result<
    crate::object::RetainedAuraAttachmentMetadata<wire::WireStaticAbility>,
    RuntimePayloadEncodingError,
> {
    crate::object::RetainedAuraAttachmentMetadata::from(value)
        .try_map_ability(encode_runtime_static_ability)
}

pub fn restore_runtime_aura_metadata(
    value: crate::object::RetainedAuraAttachmentMetadata<wire::WireStaticAbility>,
) -> Result<crate::object::AuraAttachmentMetadata, ArtifactMaterializationError> {
    value
        .try_map_ability(runtime_static_ability)?
        .try_into()
        .map_err(|error: crate::object::AuraAttachmentAbilityMismatch| {
            ArtifactMaterializationError::UnsupportedStaticAbility {
                detail: error.to_string(),
            }
        })
}
/// Decode an already trusted current model; this raw type has no release or
/// compiler-provenance envelope. Never use it as fallback for a rejected cached
/// artifact. Cached compiled-card definitions must pass materialize_artifact
/// admission, or be regenerated from complete source by the current compiler.
/// Existing trusted native retention remains owned by its savepoint/build gate.
pub fn materialize_definition(
    definition: wire::WireCardDefinition,
) -> Result<crate::cards::CardDefinition, ArtifactMaterializationError> {
    runtime_definition_from_core_model(definition)
}

pub fn materialize_artifact(
    artifact: &wire::CompiledCardArtifact,
) -> Result<crate::cards::CardDefinition, ArtifactMaterializationError> {
    // Direct callers need the same version/schema/checksum gate as registry
    // admission. In particular, pre-provenance definitions are not migratable
    // merely because their executable abilities still decode structurally.
    artifact.validate().map_err(ArtifactMaterializationError::InvalidArtifact)?;
    let mut definition = runtime_definition_from_core_model(artifact.payload.definition.clone())?;
    definition.canonical_text = artifact.payload.canonical_text.clone();
    definition.ability_labels = artifact.payload.ability_labels.clone();
    Ok(definition)
}

/// Retain the complete cost algebra, including nested alternative branches.
pub fn encode_runtime_total_cost(
    value: crate::cost::TotalCost,
) -> Result<ironsmith_core::TotalCost<wire::WireCost>, RuntimePayloadEncodingError> {
    value.try_map(encode_runtime_cost)
}
pub fn restore_runtime_total_cost(
    value: ironsmith_core::TotalCost<wire::WireCost>,
) -> Result<crate::cost::TotalCost, ArtifactMaterializationError> {
    value.try_map(runtime_cost_from_core_model)
}
pub fn encode_runtime_optional_cost(
    value: crate::cost::OptionalCost,
) -> Result<wire::WireOptionalCost, RuntimePayloadEncodingError> {
    value.try_map(encode_runtime_cost)
}
pub fn restore_runtime_optional_cost(
    value: wire::WireOptionalCost,
) -> Result<crate::cost::OptionalCost, ArtifactMaterializationError> {
    value.try_map(runtime_cost_from_core_model)
}
pub fn encode_runtime_alternative_cast(
    value: crate::alternative_cast::AlternativeCastingMethod,
) -> Result<wire::WireAlternativeCastingMethod, RuntimePayloadEncodingError> {
    value.try_map(encode_runtime_effect, encode_runtime_cost)
}
/// Restore exact retained runtime state. Definition-time Overload detargeting
/// already happened when the source definition was compiled/materialized.
pub fn restore_runtime_alternative_cast(
    value: wire::WireAlternativeCastingMethod,
) -> Result<crate::alternative_cast::AlternativeCastingMethod, ArtifactMaterializationError> {
    value.try_map(runtime_effect_from_core_model, runtime_cost_from_core_model)
}

#[cfg(test)]
mod scoped_template_materialization_tests {
    use super::*;

    fn fixture() -> (
        wire::WireEffect,
        crate::cards::CardDefinition,
        crate::static_abilities::StaticAbilityInstanceId,
    ) {
        let native = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Scoped flying template",
        )
        .token()
        .card_types(vec![crate::types::CardType::Creature])
        .flying()
        .build();
        let crate::ability::AbilityKind::Static(flying) = &native.abilities[0].kind else {
            panic!("fixture must contain flying")
        };
        let identity = flying.instance_id();
        let mut model = wire::WireCardDefinition::new(native.card.clone());
        model.abilities = native
            .abilities
            .clone()
            .into_iter()
            .map(encode_runtime_ability)
            .collect::<Result<_, _>>()
            .unwrap();
        let token = wire::WireEffect::new(
            "CreateTokenEffect",
            serde_json::to_value(ironsmith_core::CreateTokenEffect::one(model)).unwrap(),
        );
        let wrapped = wire::WireEffect::new(
            "TaggedEffect",
            serde_json::to_value(ironsmith_core::TaggedEffect::new("scoped_template", token))
                .unwrap(),
        );
        (wrapped, native, identity)
    }

    fn template(effect: &crate::effect::Effect) -> &crate::cards::CardDefinition {
        let tagged = effect
            .downcast_ref::<crate::effects::TaggedEffect>()
            .unwrap();
        &tagged
            .effect
            .downcast_ref::<crate::effects::CreateTokenEffect>()
            .unwrap()
            .token
    }

    #[test]
    fn nested_materialization_uses_scoped_snapshot_and_created_tokens_share_occurrence() {
        let (wire, snapshot, identity) = fixture();
        let card = snapshot.card.id;
        let mut calls = 0;
        let effect =
            runtime_effect_from_core_model_with_card_definitions(wire.clone(), &mut |model| {
                assert_eq!(model.card.id, card);
                calls += 1;
                Ok(snapshot.clone())
            })
            .unwrap();
        assert_eq!(calls, 1);
        let crate::ability::AbilityKind::Static(flying) = &template(&effect).abilities[0].kind
        else {
            panic!("missing static")
        };
        assert_eq!(flying.instance_id(), identity);
        assert_eq!(encode_runtime_effect(effect.clone()).unwrap(), wire);
        let alice = crate::ids::PlayerId::from_index(0);
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&snapshot, alice, crate::zone::Zone::Stack);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.battlefield.len(), 1);
        assert!(game.object(game.battlefield[0]).unwrap().abilities.iter().any(|ability| matches!(&ability.kind, crate::ability::AbilityKind::Static(value) if value.instance_id() == identity)));
    }

    #[test]
    fn scoped_template_rejection_propagates_without_default_materialization() {
        let (wire, _, _) = fixture();
        let error = ArtifactMaterializationError::UnsupportedEffect {
            detail: "unknown retained template association".into(),
        };
        let mut calls = 0;
        let result = runtime_effect_from_core_model_with_card_definitions(wire, &mut |_| {
            calls += 1;
            Err(error.clone())
        });
        assert_eq!(calls, 1);
        assert!(matches!(result, Err(actual) if actual == error));
    }

    #[test]
    fn scoped_template_resolvers_do_not_merge_equal_independent_occurrences_or_leak() {
        let (wire, first, original) = fixture();
        let second = runtime_definition_from_core_model({
            let token = wire
                .downcast_with::<ironsmith_core::TaggedEffect<wire::WireEffect>, _>(|payload| {
                    ironsmith_artifact_effect_decoder::decode("TaggedEffect", payload)
                })
                .unwrap();
            token
                .effect
                .downcast_with::<ironsmith_core::CreateTokenEffect<wire::WireCardDefinition>, _>(
                    |payload| {
                        ironsmith_artifact_effect_decoder::decode("CreateTokenEffect", payload)
                    },
                )
                .unwrap()
                .token
                .clone()
        })
        .unwrap();
        let crate::ability::AbilityKind::Static(second_static) = &second.abilities[0].kind else {
            panic!("missing static")
        };
        let second_identity = second_static.instance_id();
        assert_ne!(original, second_identity);
        let first_effect =
            runtime_effect_from_core_model_with_card_definitions(wire.clone(), &mut |_| {
                Ok(first.clone())
            })
            .unwrap();
        let second_effect =
            runtime_effect_from_core_model_with_card_definitions(wire.clone(), &mut |_| {
                Ok(second.clone())
            })
            .unwrap();
        let default_effect = runtime_effect_from_core_model(wire).unwrap();
        for (effect, expected) in [
            (&first_effect, Some(original)),
            (&second_effect, Some(second_identity)),
            (&default_effect, None),
        ] {
            let crate::ability::AbilityKind::Static(value) = &template(effect).abilities[0].kind
            else {
                panic!("missing static")
            };
            if let Some(expected) = expected {
                assert_eq!(value.instance_id(), expected);
            } else {
                assert_ne!(value.instance_id(), original);
                assert_ne!(value.instance_id(), second_identity);
            }
        }
    }
}

#[cfg(test)]
mod native_effect_payload_codec_tests {
    use super::*;
    #[test]
    fn native_effect_payload_codec_preserves_token_fields_from_actual_executor() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Native token payload")
            .token().card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1)).flying().build();
        let mut token = crate::effects::CreateTokenEffect::new(definition, 3, crate::target::PlayerFilter::You);
        token.text_roles = Some(ironsmith_core::TokenTextRoles::authored(ironsmith_core::TokenNameTextRole::Explicit, 1));
        token.controller_target = Some(crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Any));
        token.use_source_chosen_color = true;
        token.use_source_chosen_creature_type = true;
        token.actor_surface_explicit = true;
        token.suppress_aura_attachment_choice = true;
        token.enters_tapped = true;
        token.enters_attacking = true;
        token.enters_blocking = Some(crate::target::ChooseSpec::Source);
        token.exile_at_end_of_combat = true;
        token.sacrifice_at_end_of_combat = true;
        token.sacrifice_at_next_end_step = true;
        token.exile_at_next_end_step = true;
        token.link_source_exiled_this_resolution = true;
        let encoded = encode_runtime_effect(crate::effect::Effect::new(token)).expect("native token encodes");
        let restored = materialize_effect(encoded.clone()).expect("typed model materializes");
        let actual = restored.downcast_ref::<crate::effects::CreateTokenEffect>().expect("native token executor");
        let from_actual = encode_runtime_effect(crate::effect::Effect::new(actual.clone())).expect("actual executor reencodes");
        assert_eq!(from_actual, encoded, "retained JSON must not mask field loss in materialization");
    }
}

#[cfg(test)]
mod native_direct_payload_codec_tests {
    use super::*;
    #[test]
    fn native_direct_payload_codec_inventory_matches_interpreter() {
        macro_rules! inventory {
            ($($ty:path),* $(,)?) => { const TYPES: &[&str] = &[$(stringify!($ty)),*]; };
        }
        with_native_direct_effect_types!(inventory);
        let normalize = |value: &str| value.chars().filter(|character| !character.is_whitespace()).collect::<String>();
        let actual: std::collections::BTreeSet<_> = TYPES.iter().map(|value| normalize(value)).collect();
        let source = include_str!("effect_model_interpreter.rs");
        let mut expected = std::collections::BTreeSet::new();
        for suffix in source.split("clone_direct_effect::<M,").skip(1) {
            let value = suffix.split('>').next().unwrap().trim();
            if value != "$ty" { expected.insert(normalize(value)); }
        }
        let start = source.find("    clone_direct!(\n").expect("direct-clone table");
        let block = source[start..].split("\n    );").next().unwrap();
        for value in block.lines().skip(1).map(|line| line.trim().trim_end_matches(',')) {
            if value.starts_with("crate::") { expected.insert(normalize(value)); }
        }
        assert_eq!(actual, expected, "direct native schemas and interpreter vocabulary must stay synchronized");
    }

    fn check<E: crate::effects::EffectExecutor + serde::Serialize + Clone>(payload: E) {
        let kind = wire::stable_payload_kind(std::any::type_name::<E>());
        let encoded = wire::WireEffect::new(kind, serde_json::to_value(&payload).expect("typed native schema"));
        let restored = materialize_effect(encoded.clone()).expect("existing interpreter direct-clone type must materialize through active decoder");
        assert!(restored.downcast_ref::<E>().is_some(), "restored native executor has exact type");
        let original_native = encode_runtime_effect(crate::effect::Effect::new(payload)).expect("native direct producer must encode");
        assert_eq!(original_native, encoded);
        let actual = restored.downcast_ref::<E>().unwrap().clone();
        assert_eq!(encode_runtime_effect(crate::effect::Effect::new(actual)).unwrap(), encoded,
            "actual executor encoding must agree, not just retained metadata");
    }
    #[test]
    fn native_direct_payload_codec_bounded_number_preserves_bounds_and_chooser() {
        check(ironsmith_core::ChooseNumberEffect::new(crate::target::PlayerFilter::Specific(crate::ids::PlayerId::from_index(1)), 0, 13));
    }
    #[test]
    fn native_direct_payload_codec_source_owned_number_keeps_unbounded_domain() {
        check(ironsmith_core::ChooseNumberEffect::unbounded(crate::target::PlayerFilter::You).with_source_retention());
        check(ironsmith_core::ChooseNumberEffect::new(crate::target::PlayerFilter::You, 0, 7).with_source_retention());
    }
    #[test]
    fn native_direct_payload_codec_note_activation_mana() {
        check(ironsmith_core::NoteActivationManaTypeEffect::new());
    }
    #[test]
    fn native_direct_payload_codec_add_noted_mana() {
        check(ironsmith_core::AddManaOfNotedTypeEffect::new(3, crate::target::PlayerFilter::You));
    }
    #[test]
    fn native_direct_payload_codec_miracle_permission() {
        check(ironsmith_core::MayCastForMiracleCostEffect::new());
    }
    #[test]
    fn native_direct_payload_codec_resolve_illegal_targets() {
        check(ironsmith_core::ResolvesDespiteIllegalTargetsEffect::new());
    }
}

#[cfg(test)]
mod native_cost_producer_codec_tests {
    use super::*;
    #[test]
    fn native_cost_producer_codec_restored_payments_use_final_replaced_event() {
        for restore in [false, true] {
            let alice = crate::PlayerId::from_index(0);
            let mut game = crate::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition = crate::cards::CardDefinition::new(crate::CardBuilder::new(crate::CardId::new(), "Native cost source")
                .card_types(vec![crate::CardType::Artifact]).build());
            let source = game.create_object_from_definition(&definition, alice, crate::Zone::Battlefield);
            game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                source, alice, crate::events::life::matchers::WouldLoseLifeMatcher::you(),
                crate::replacement::ReplacementAction::Double));
            let costs = [crate::costs::Cost::tap(), crate::costs::Cost::life(2), crate::costs::Cost::untap()];
            for (index, cost) in costs.into_iter().enumerate() {
                let cost = if restore { runtime_cost_from_core_model(encode_runtime_cost(cost).unwrap()).unwrap() } else { cost };
                let mut dm = crate::decision::AutoPassDecisionMaker;
                let mut ctx = crate::costs::CostContext::new(source, alice, &mut dm);
                ctx.reason = crate::costs::PaymentReason::ActivateAbility;
                cost.can_pay(&game, &ctx).expect("native/restored payment legal");
                assert!(matches!(cost.pay(&mut game, &mut ctx).expect("payment completes"), crate::costs::CostPaymentResult::Paid));
                assert_eq!(game.is_tapped(source), index != 2);
                assert_eq!(game.player(alice).unwrap().life, if index == 0 {20} else {16},
                    "original cost amount must not overwrite doubled final loss, restored={restore}");
            }
        }
    }

    #[test]
    fn native_cost_producer_codec_builtin_factories_keep_complete_models() {
        use crate::costs::Cost;
        let costs = vec![
            ("tap", Cost::tap()), ("untap", Cost::untap()), ("life", Cost::life(2)),
            ("mana", Cost::mana(crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(1)]))),
            ("dynamic mana", Cost::dynamic_mana(ironsmith_core::DynamicManaCost::from_x(
                crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::X]), crate::effect::Value::Fixed(3)))),
            ("discard source", Cost::discard_source()), ("exile source", Cost::exile_self()),
            ("remove variable counters", Cost::remove_any_counters_from_source(None, true)),
            ("remove all counters", Cost::remove_all_counters_from_source(Some(crate::CounterType::Charge))),
            ("reveal matching", Cost::reveal_from_hand_with_color_filter(crate::effect::Value::Fixed(2),
                Some(crate::CardType::Creature), Some(crate::color::ColorSet::RED))),
            ("return matching", Cost::return_to_hand(crate::filter::ObjectFilter::creature())),

            ("sacrifice self", Cost::sacrifice_self()),
            ("sacrifice matching", Cost::sacrifice(crate::filter::ObjectFilter::creature())),
            ("discard matching", Cost::discard_types(2, vec![crate::CardType::Creature])),
            ("discard hand", Cost::discard_hand()),
            ("exile graveyard", Cost::exile_from_graveyard_types(2, vec![crate::CardType::Creature])),
            ("exile hand", Cost::exile_from_hand(2, Some(crate::color::ColorSet::RED))),
            ("remove counters", Cost::remove_counters(crate::CounterType::Charge, 2)),
            ("add counters", Cost::add_counters(crate::CounterType::Charge, 2)),
            ("energy", Cost::energy(2)), ("return self", Cost::return_self_to_hand()),
            ("mill", Cost::mill(2)),
            ("typed effect", Cost::effect(crate::effects::TapEffect::source())),
            ("erased effect", Cost::try_effect(crate::effect::Effect::new(crate::effects::TapEffect::source())).unwrap()),
        ];
        for (name, cost) in costs {
            let wire = encode_runtime_cost(cost).unwrap_or_else(|error| panic!("{name}: {error}"));
            let restored = runtime_cost_from_core_model(wire.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(serde_json::to_value(encode_runtime_cost(restored).unwrap()).unwrap(),
                serde_json::to_value(wire).unwrap(), "complete model survives native {name}");
        }
    }
}
