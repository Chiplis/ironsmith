//! Materialization of versioned compiled-card artifacts into engine values.

use ironsmith_compiled_artifact as wire;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactMaterializationError {
    UnsupportedEffect { detail: String },
    UnsupportedStaticAbility { detail: String },
    UnsupportedTrigger { detail: String },
}

impl std::fmt::Display for ArtifactMaterializationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
        "CastTaggedEffect" => decode_as::<T, ironsmith_core::CastTaggedEffect>(effect),
        "ChooseCardNameEffect" => decode_as::<T, ironsmith_core::ChooseCardNameEffect>(effect),
        "ChooseCardTypeEffect" => decode_as::<T, ironsmith_core::ChooseCardTypeEffect>(effect),
        "ChooseColorEffect" => decode_as::<T, ironsmith_core::ChooseColorEffect>(effect),
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
        "CumulativeUpkeepEffect" => {
            decode_as::<T, ironsmith_core::CumulativeUpkeepEffect<wire::WireEffect>>(effect)
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
        "DevourEffect" => decode_as::<T, ironsmith_core::DevourEffect>(effect),
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
        "ForPlayersEffect" => {
            decode_as::<T, ironsmith_core::ForPlayersEffect<wire::WireEffect>>(effect)
        }
        "GainLifeEffect" => decode_as::<T, ironsmith_core::GainLifeEffect>(effect),
        "GoadEffect" => decode_as::<T, ironsmith_core::GoadEffect>(effect),
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
        "GrantPlayTaggedEffect" => decode_as::<T, ironsmith_core::GrantPlayTaggedEffect>(effect),
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
        "PreventAllDamageEffect" => decode_as::<T, ironsmith_core::PreventAllDamageEffect>(effect),
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
        "RegisterEnterTappedReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterEnterTappedReplacementEffect>(effect)
        }
        "RegisterEnterUnderControlReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterEnterUnderControlReplacementEffect>(effect)
        }
        "RegisterFutureZoneReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterFutureZoneReplacementEffect>(effect)
        }
        "RegisterManaReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterManaReplacementEffect>(effect)
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
        "RemoveCountersEffect" => decode_as::<T, ironsmith_core::RemoveCountersEffect>(effect),
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
        "SkipMainPhasesThisTurnEffect" => {
            decode_as::<T, ironsmith_core::SkipMainPhasesThisTurnEffect>(effect)
        }
        "SkipNextCombatPhaseThisTurnEffect" => {
            decode_as::<T, ironsmith_core::SkipNextCombatPhaseThisTurnEffect>(effect)
        }
        "SkipTurnEffect" => decode_as::<T, ironsmith_core::SkipTurnEffect>(effect),
        "SneakCostEffect" => decode_as::<T, ironsmith_core::SneakCostEffect>(effect),
        "SolveCaseEffect" => decode_as::<T, ironsmith_core::SolveCaseEffect>(effect),
        "SoulbondPairEffect" => decode_as::<T, ironsmith_core::SoulbondPairEffect>(effect),
        "SupportEffect" => decode_as::<T, ironsmith_core::SupportEffect>(effect),
        "SurveilEffect" => decode_as::<T, ironsmith_core::SurveilEffect>(effect),
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

struct WireEffectModelHooks;

impl crate::effect_model_interpreter::EffectModelInterpreterHooks<WireEffectModel>
    for WireEffectModelHooks
{
    type Error = ArtifactMaterializationError;

    fn unsupported_effect(&mut self, detail: String) -> Self::Error {
        ArtifactMaterializationError::UnsupportedEffect { detail }
    }

    fn runtime_static_ability_hook(
        &mut self,
        ability: wire::WireStaticAbility,
    ) -> Result<crate::static_abilities::StaticAbility, Self::Error> {
        runtime_static_ability(ability)
    }

    fn runtime_card_definition_hook(
        &mut self,
        definition: wire::WireCardDefinition,
    ) -> Result<crate::cards::CardDefinition, Self::Error> {
        runtime_definition_from_core_model(definition)
    }

    fn runtime_ability_hook(
        &mut self,
        ability: wire::WireAbility,
    ) -> Result<crate::ability::Ability, Self::Error> {
        runtime_ability_from_core_model(ability)
    }

    fn runtime_emblem_hook(
        &mut self,
        emblem: wire::WireEmblemDescription,
    ) -> Result<crate::effect::EmblemDescription, Self::Error> {
        let mut converted = crate::effect::EmblemDescription::new(&emblem.name, &emblem.text);
        for ability in emblem.abilities {
            converted = converted.with_ability(runtime_ability_from_core_model(ability)?);
        }
        Ok(converted)
    }

    fn runtime_continuous_modification_hook(
        &mut self,
        modification: wire::WireContinuousModification,
    ) -> Result<crate::continuous::Modification, Self::Error> {
        crate::continuous::Modification::try_from_model(
            modification,
            runtime_static_ability,
            runtime_ability_from_core_model,
            runtime_ability_from_core_model,
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
                crate::grant::Grantable::Ability(runtime_static_ability(ability)?)
            }
            wire::WireGrantable::AlternativeCast(method) => {
                crate::grant::Grantable::AlternativeCast(convert_alternative_cast(method)?)
            }
            wire::WireGrantable::PlayFrom => crate::grant::Grantable::PlayFrom,
            wire::WireGrantable::DerivedAlternativeCast(spec) => {
                crate::grant::Grantable::DerivedAlternativeCast(convert_derived_alternative_cast(
                    spec,
                )?)
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
            beneficiary: spec.beneficiary,
            usage_limit: spec.usage_limit,
            cast_this_way_filter: spec.cast_this_way_filter,
            source_exiled_surface: spec.source_exiled_surface,
            cast_this_way_grants: spec
                .cast_this_way_grants
                .into_iter()
                .map(|ability| self.runtime_static_ability_hook(ability))
                .collect::<Result<Vec<_>, _>>()?,
        })
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
    crate::effect_model_interpreter::interpret_effect_model::<WireEffectModel, _>(
        effect,
        &mut WireEffectModelHooks,
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
        return Some(crate::effect::Effect::new(
            crate::effects::TaggedEffect::new(tagged.tag.clone(), inner),
        ));
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
    converted = runtime_ability_with_inherent_functional_zones(converted);
    Ok(converted)
}

fn runtime_ability_with_inherent_functional_zones(
    ability: crate::ability::Ability,
) -> crate::ability::Ability {
    let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
        return ability;
    };
    match static_ability.id() {
        crate::static_abilities::StaticAbilityId::ExileToExileInsteadOfGraveyard
        | crate::static_abilities::StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard
        | crate::static_abilities::StaticAbilityId::ExileWouldDieInstead => ability.in_zones(vec![
            crate::zone::Zone::Battlefield,
            crate::zone::Zone::Stack,
            crate::zone::Zone::Graveyard,
            crate::zone::Zone::Hand,
            crate::zone::Zone::Library,
            crate::zone::Zone::Exile,
            crate::zone::Zone::Command,
        ]),
        crate::static_abilities::StaticAbilityId::Dredge => {
            ability.in_zones(vec![crate::zone::Zone::Graveyard])
        }
        crate::static_abilities::StaticAbilityId::Grants => {
            if let Some(spec) = static_ability.grant_spec()
                && spec.filter.source
                && spec.zone != crate::zone::Zone::Battlefield
            {
                ability.in_zones(vec![spec.zone])
            } else {
                ability
            }
        }
        _ => ability,
    }
}

fn combine_level_ability_statics(
    abilities: Vec<crate::ability::Ability>,
) -> Vec<crate::ability::Ability> {
    let mut out = Vec::with_capacity(abilities.len());
    let mut levels = Vec::new();

    for ability in abilities {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            out.push(ability);
            continue;
        };
        let Some(level_abilities) = static_ability.level_abilities() else {
            out.push(ability);
            continue;
        };
        levels.extend(level_abilities.iter().cloned());
    }

    if !levels.is_empty() {
        out.push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::with_level_abilities(levels),
        ));
    }

    out
}

const CLASS_LEVEL_MARKER_PREFIX: &str = "__ironsmith_class_level:";

fn class_level_marker(ability: &crate::ability::ActivatedAbility) -> Option<u32> {
    ability
        .additional_restrictions
        .iter()
        .find_map(|restriction| restriction.strip_prefix(CLASS_LEVEL_MARKER_PREFIX))
        .and_then(|level| level.parse::<u32>().ok())
}

fn class_level_activation_condition(level: u32) -> crate::ConditionExpr {
    let required_counters = level.saturating_sub(2);
    if required_counters == 0 {
        return crate::ConditionExpr::SourceHasNoCounter(crate::CounterType::Level);
    }
    crate::ConditionExpr::And(
        Box::new(crate::ConditionExpr::SourceHasCounterAtLeast {
            counter_type: crate::CounterType::Level,
            count: required_counters,
            surface: crate::SourceCounterThresholdSurface::SourceHas,
        }),
        Box::new(crate::ConditionExpr::Not(Box::new(
            crate::ConditionExpr::SourceHasCounterAtLeast {
                counter_type: crate::CounterType::Level,
                count: required_counters + 1,
                surface: crate::SourceCounterThresholdSurface::SourceHas,
            },
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
            current_level = Some(level);
            continue;
        }

        let Some(level) = current_level else {
            continue;
        };
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

pub fn materialize_definition(
    definition: wire::WireCardDefinition,
) -> Result<crate::cards::CardDefinition, ArtifactMaterializationError> {
    runtime_definition_from_core_model(definition)
}

pub fn materialize_artifact(
    artifact: &wire::CompiledCardArtifact,
) -> Result<crate::cards::CardDefinition, ArtifactMaterializationError> {
    let mut definition = runtime_definition_from_core_model(artifact.payload.definition.clone())?;
    definition.canonical_text = artifact.payload.canonical_text.clone();
    definition.ability_labels = artifact.payload.ability_labels.clone();
    Ok(definition)
}
