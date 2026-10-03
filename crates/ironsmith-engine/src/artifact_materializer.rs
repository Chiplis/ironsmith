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
        "DealDamageEffect" => decode_as::<T, ironsmith_core::DealDamageEffect>(effect),
        "DealDistributedDamageEffect" => {
            decode_as::<T, ironsmith_core::DealDistributedDamageEffect>(effect)
        }
        "DestroyEffect" => decode_as::<T, ironsmith_core::DestroyEffect>(effect),
        "DestroyNoRegenerationEffect" => {
            decode_as::<T, ironsmith_core::DestroyNoRegenerationEffect>(effect)
        }
        "DetainEffect" => decode_as::<T, ironsmith_core::DetainEffect>(effect),
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
        "GrantPlayTaggedEffect" => decode_as::<T, ironsmith_core::GrantPlayTaggedEffect>(effect),
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
        "RegisterManaReplacementEffect" => {
            decode_as::<T, ironsmith_core::RegisterManaReplacementEffect>(effect)
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
        "SetClassLevelEffect" => decode_as::<T, ironsmith_core::SetClassLevelEffect>(effect),
        "SoulbondPairEffect" => decode_as::<T, ironsmith_core::SoulbondPairEffect>(effect),
        "SupportEffect" => decode_as::<T, ironsmith_core::SupportEffect>(effect),
        "SurveilEffect" => decode_as::<T, ironsmith_core::SurveilEffect>(effect),
        "BecomePlottedEffect" => decode_as::<T, ironsmith_core::BecomePlottedEffect>(effect),
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
            beneficiary: spec.beneficiary,
            usage_limit: spec.usage_limit,
            max_plays: spec.max_plays,
            cast_this_way_filter: spec.cast_this_way_filter,
            source_exiled_surface: spec.source_exiled_surface,
            cast_this_way_grants: spec
                .cast_this_way_grants
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

/// Restore one canonical executable effect, retaining the same transport model
/// for subsequent checkpoints. This shares the ordinary artifact interpreter.
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
            crate::effects::CastTaggedEffect,
            crate::effects::ChooseCardNameEffect,
            crate::effects::ChooseCardTypeEffect,
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
            crate::effects::PreventAllDamageEffect,
            crate::effects::ProliferateEffect,
            crate::effects::PutCountersEffect,
            crate::effects::PutOntoBattlefieldEffect,
            crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
            crate::effects::RearrangeLookedCardsInLibraryEffect,
            crate::effects::ReconfigureEffect,
            crate::effects::ReduceSpeedEffect,
            crate::effects::RegisterCounterPlacementReplacementEffect,
            crate::effects::RegisterEnterTappedReplacementEffect,
            crate::effects::RegisterFutureZoneReplacementEffect,
            crate::effects::RegisterManaReplacementEffect,
            crate::effects::RegisterNextBatchEnterWithCountersEffect,
            crate::effects::RemoveAnyCountersAmongEffect,
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
            crate::effects::TurnFaceUpEffect,
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
    if let Some(payload) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        let ironsmith_core::CreateTokenEffect {
            token, count, controller, controller_target, use_source_chosen_color,
            use_source_chosen_creature_type, actor_surface_explicit,
            suppress_aura_attachment_choice, ability_presentation, enters_tapped,
            enters_attacking, attack_target_mode, enters_blocking,
            exile_at_end_of_combat, sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step, exile_at_next_end_step,
            next_end_step_player, link_source_exiled_this_resolution,
        } = payload.clone();
        let converted = ironsmith_core::CreateTokenEffect {
            token: encode_runtime_definition(token)?, count, controller,
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
        .compiled_model()
        .ok_or(RuntimePayloadEncodingError::MissingModel {
            component: "static ability",
        })?
        .clone()
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
/// Native occurrence aliases are bound separately by the owning checkpoint table.
pub fn encode_runtime_copy_values(
    values: crate::snapshot::CopiableValues,
) -> Result<crate::snapshot::RetainedCopiableValues<wire::WireAbility>, RuntimePayloadEncodingError>
{
    crate::snapshot::RetainedCopiableValues::from(values).try_map_abilities(encode_runtime_ability)
}

pub fn restore_runtime_copy_values(
    values: crate::snapshot::RetainedCopiableValues<wire::WireAbility>,
) -> Result<crate::snapshot::CopiableValues, ArtifactMaterializationError> {
    values
        .try_map_abilities(restore_runtime_ability)
        .map(Into::into)
}

/// Encode complete rules text, ability labels and executable ability models.
/// Native occurrence aliases are bound separately by the owning checkpoint table.
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
/// A checkpoint-local reference, never a process-global native instance ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct StaticAbilityOccurrenceRef(pub u32);

/// Card references in a shared model have an explicit namespace. A native
/// payload is never interpreted as framed merely because its number fits a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RetainedModelCardReferences {
    Native,
    Bound,
}

/// Every entry is an independent occurrence, even when its model equals another.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RetainedStaticAbilityTable {
    pub models: Vec<wire::WireStaticAbility>,
    /// Required per-occurrence template associations; equal model entries
    /// remain independent and may reference later declared occurrences.
    pub embedded_definitions: Vec<Vec<RetainedEmbeddedCardDefinition>>,
    pub model_card_references: Vec<RetainedModelCardReferences>,
    /// Payload-local card slots refer to the owning definition graph, never
    /// native CardIds. Values retain the owner's typed reference representation.
    pub payload_card_references: Vec<serde_json::Value>,
}

pub type RetainedOccurrenceAbilityModel = ironsmith_core::Ability<
    StaticAbilityOccurrenceRef,
    wire::WireTrigger,
    wire::WireEffect,
    wire::WireCost,
>;

/// Inline executable payloads carry their own reference namespace. A native
/// CardId is never a payload-local slot just because the numbers coincide.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RetainedCardPayload<T> {
    pub card_references: RetainedModelCardReferences,
    pub model: T,
    /// Ordered, explicit associations for definitions reached by this payload's
    /// effect interpreter. Required even when empty; never infer occurrences
    /// from equality of models or process-local card IDs.
    pub embedded_definitions: Vec<RetainedEmbeddedCardDefinition>,
}
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RetainedEmbeddedCardDefinition {
    pub model: wire::WireCardDefinition,
    pub snapshot: RetainedOccurrenceCardDefinition<crate::ids::CardId>,
}
pub type RetainedOccurrenceAbility = RetainedCardPayload<RetainedOccurrenceAbilityModel>;
pub type RetainedOccurrenceProgram =
    RetainedCardPayload<ironsmith_core::ResolutionProgram<wire::WireEffect>>;

pub type RetainedOccurrenceAlternativeCast =
    RetainedCardPayload<wire::WireAlternativeCastingMethod>;
pub type RetainedOccurrenceOptionalCost = RetainedCardPayload<wire::WireOptionalCost>;
pub type RetainedOccurrenceTotalCost =
    RetainedCardPayload<ironsmith_core::TotalCost<wire::WireCost>>;


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OccurrenceBindingError {
    Encoding(RuntimePayloadEncodingError),
    Materialization(ArtifactMaterializationError),
    InvalidModel {
        detail: String,
    },
    InconsistentOccurrence {
        reference: StaticAbilityOccurrenceRef,
    },
    UnboundNativeOccurrence,
    UnknownReference {
        reference: StaticAbilityOccurrenceRef,
    },
    TooManyOccurrences,
}
impl std::fmt::Display for OccurrenceBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encoding(error) => error.fmt(f),
            Self::Materialization(error) => error.fmt(f),
            Self::InvalidModel { detail } => {
                write!(f, "invalid retained occurrence model: {detail}")
            }
            Self::InconsistentOccurrence { reference } => {
                write!(f, "conflicting models for occurrence {}", reference.0)
            }
            Self::UnboundNativeOccurrence => {
                f.write_str("native occurrence has no model in the owning table")
            }
            Self::UnknownReference { reference } => {
                write!(f, "unknown retained occurrence {}", reference.0)
            }
            Self::TooManyOccurrences => f.write_str("retained occurrence table is too large"),
        }
    }
}
impl std::error::Error for OccurrenceBindingError {}
impl From<RuntimePayloadEncodingError> for OccurrenceBindingError {
    fn from(error: RuntimePayloadEncodingError) -> Self {
        Self::Encoding(error)
    }
}
impl From<ArtifactMaterializationError> for OccurrenceBindingError {
    fn from(error: ArtifactMaterializationError) -> Self {
        Self::Materialization(error)
    }
}

fn remap_payload_card_ids<T: serde::Serialize + serde::de::DeserializeOwned>(
    value: T,
    card: &mut impl FnMut(u32) -> Result<u32, OccurrenceBindingError>,
) -> Result<T, OccurrenceBindingError> {
    let mut binding_error = None;
    let result = ironsmith_artifact_effect_decoder::remap_card_ids(&value, &mut |id| {
        card(id).map_err(|error| {
            let detail = error.to_string();
            binding_error = Some(error);
            detail
        })
    });
    if let Some(error) = binding_error {
        return Err(error);
    }
    let json = result.map_err(|detail| OccurrenceBindingError::InvalidModel { detail })?;
    serde_json::from_value(json).map_err(|error| OccurrenceBindingError::InvalidModel {
        detail: error.to_string(),
    })
}
/// Wire specialization of every supported continuous modification variant.
pub type RetainedOccurrenceModification = crate::continuous::ContinuousModification<
    StaticAbilityOccurrenceRef,
    RetainedOccurrenceAbility,
    crate::snapshot::RetainedCopiableValues<RetainedOccurrenceAbility>,
    crate::continuous::RetainedTextBoxOverlay<RetainedOccurrenceAbility>,
    crate::continuous::RetainedRestriction<StaticAbilityOccurrenceRef>,
    crate::object::RetainedAuraAttachmentMetadata<StaticAbilityOccurrenceRef>,
>;
pub type RetainedOccurrenceContinuousEffect = crate::continuous::ContinuousEffect<
    RetainedOccurrenceModification,
    StaticAbilityOccurrenceRef,
    crate::continuous::ContinuousAbilityOrigin<StaticAbilityOccurrenceRef>,
>;
pub type RetainedOccurrenceRegisteredState =
    crate::continuous::RegisteredContinuousEffectState<RetainedOccurrenceContinuousEffect>;

pub type RetainedOccurrenceFaceDownCastState = crate::object::RetainedFaceDownCastState<
    RetainedOccurrenceAbility,
    RetainedOccurrenceProgram,
    crate::object::RetainedAuraAttachmentMetadata<StaticAbilityOccurrenceRef>,
>;
pub type RetainedOccurrenceEntersAsCopyRestoreState<I> =
    crate::object::RetainedEntersAsCopyRestoreState<
        RetainedOccurrenceAbility,
        RetainedOccurrenceProgram,
        crate::object::RetainedAuraAttachmentMetadata<StaticAbilityOccurrenceRef>,
        I,
    >;

pub type RetainedOccurrenceBestowCastState = crate::object::RetainedBestowCastState<
    RetainedOccurrenceProgram,
    crate::object::RetainedAuraAttachmentMetadata<StaticAbilityOccurrenceRef>,
>;
pub type RetainedOccurrenceSpliceCastState =
    crate::object::RetainedSpliceCastState<RetainedOccurrenceProgram>;

/// One exporter owns this table for all carriers and historical references.
#[derive(Debug, Clone, Default)]
pub struct StaticAbilityOccurrenceEncoder {
    bindings: std::collections::HashMap<
        crate::static_abilities::StaticAbilityInstanceId,
        StaticAbilityOccurrenceRef,
    >,
    models: Vec<wire::WireStaticAbility>,
    embedded_definitions: Vec<Vec<RetainedEmbeddedCardDefinition>>,
    canonical_embedded_definitions: Vec<serde_json::Value>,
    model_card_references: Vec<RetainedModelCardReferences>,
    canonical: Vec<serde_json::Value>,
    graph_bindings: std::collections::HashMap<crate::ids::CardId, (usize, serde_json::Value)>,
    payload_bindings: std::collections::HashMap<crate::ids::CardId, u32>,
    payload_card_references: Vec<serde_json::Value>,
}
impl StaticAbilityOccurrenceEncoder {
    fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, OccurrenceBindingError>,
    ) -> Result<T, OccurrenceBindingError> {
        // Nested carriers can bind an existing shared model before a later
        // sibling fails. Preserve the old model payloads and reference modes,
        // as well as truncating newly retained graph/occurrence prefixes.
        let previous_models = self.models.clone();
        let previous_modes = self.model_card_references.clone();
        let previous_embedded = self.embedded_definitions.clone();
        let retained = self.models.len();
        let retained_graph = self.graph_bindings.len();
        let retained_payload = self.payload_card_references.len();
        match operation(self) {
            Ok(result) => Ok(result),
            Err(error) => {
                self.models = previous_models;
                self.model_card_references = previous_modes;
                self.embedded_definitions = previous_embedded;
                self.canonical.truncate(retained);
                self.canonical_embedded_definitions.truncate(retained);
                self.graph_bindings
                    .retain(|_, (index, _)| *index < retained_graph);
                self.payload_card_references.truncate(retained_payload);
                self.payload_bindings
                    .retain(|_, index| (*index as usize) < retained_payload);
                self.bindings
                    .retain(|_, reference| (reference.0 as usize) < retained);
                Err(error)
            }
        }
    }
    pub fn retain(
        &mut self,
        ability: crate::static_abilities::StaticAbility,
    ) -> Result<StaticAbilityOccurrenceRef, OccurrenceBindingError> {
        self.transaction(|table| table.retain_inner(ability))
    }

    fn retain_inner(
        &mut self,
        ability: crate::static_abilities::StaticAbility,
    ) -> Result<StaticAbilityOccurrenceRef, OccurrenceBindingError> {
        let instance = ability.instance_id();
        let model = encode_runtime_static_ability(ability.clone())?;
        let canonical =
            serde_json::to_value(&model).map_err(|error| OccurrenceBindingError::InvalidModel {
                detail: error.to_string(),
            })?;
        if let Some(reference) = self.bindings.get(&instance).copied() {
            if self.canonical[reference.0 as usize] != canonical {
                return Err(OccurrenceBindingError::InconsistentOccurrence { reference });
            }
            return Ok(reference);
        }
        if self.models.len() >= u32::MAX as usize {
            return Err(OccurrenceBindingError::TooManyOccurrences);
        }
        let reference = StaticAbilityOccurrenceRef(self.models.len() as u32);
        self.models.push(model);
        self.model_card_references
            .push(RetainedModelCardReferences::Native);
        self.canonical.push(canonical);
        self.bindings.insert(instance, reference);
        // Declare the parent before walking child templates. Its index stays
        // stable while complete nested native occurrences join this table.
        self.embedded_definitions.push(Vec::new());
        self.canonical_embedded_definitions.push(serde_json::json!([]));
        let native = ability.compiled_model().ok_or(RuntimePayloadEncodingError::MissingModel { component: "static ability" })?.clone();
        let table = std::cell::RefCell::new(&mut *self);
        let embedded = std::cell::RefCell::new(Vec::new());
        native.try_map(
            |trigger| encode_runtime_trigger(trigger).map_err(OccurrenceBindingError::from),
            |effect| table.borrow_mut().encode_effect_with_occurrences(effect, &mut embedded.borrow_mut()),
            |cost| table.borrow_mut().encode_cost_with_occurrences(cost, &mut embedded.borrow_mut()),
            Ok,
        )?;
        let embedded = embedded.into_inner();
        self.canonical_embedded_definitions[reference.0 as usize] = serde_json::to_value(&embedded)
            .map_err(|error| OccurrenceBindingError::InvalidModel { detail: error.to_string() })?;
        self.embedded_definitions[reference.0 as usize] = embedded;
        Ok(reference)
    }

    /// Resolve provenance only after its full native payload has been retained.
    pub fn reference(
        &self,
        instance: crate::static_abilities::StaticAbilityInstanceId,
    ) -> Result<StaticAbilityOccurrenceRef, OccurrenceBindingError> {
        self.bindings
            .get(&instance)
            .copied()
            .ok_or(OccurrenceBindingError::UnboundNativeOccurrence)
    }

    pub fn into_table(self) -> RetainedStaticAbilityTable {
        RetainedStaticAbilityTable {
            models: self.models,
            embedded_definitions: self.embedded_definitions,
            model_card_references: self.model_card_references,
            payload_card_references: self.payload_card_references,
        }
    }

    fn bind_graph_reference<I: serde::Serialize>(
        &mut self,
        id: crate::ids::CardId,
        card: &mut impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<I, OccurrenceBindingError> {
        let bound = card(id)?;
        let wire =
            serde_json::to_value(&bound).map_err(|error| OccurrenceBindingError::InvalidModel {
                detail: error.to_string(),
            })?;
        if wire.is_null() {
            return Err(OccurrenceBindingError::InvalidModel {
                detail: "null owning card graph reference".into(),
            });
        }
        if let Some((_, original)) = self.graph_bindings.get(&id) {
            if original != &wire {
                return Err(OccurrenceBindingError::InvalidModel {
                    detail: "card graph binding changed between retained roots".into(),
                });
            }
        } else {
            if self
                .graph_bindings
                .values()
                .any(|(_, value)| value == &wire)
            {
                return Err(OccurrenceBindingError::InvalidModel {
                    detail: "distinct native card nodes share an owning graph reference".into(),
                });
            }
            self.graph_bindings
                .insert(id, (self.graph_bindings.len(), wire));
        }
        Ok(bound)
    }

    fn bind_payload<T: serde::Serialize + serde::de::DeserializeOwned, I: serde::Serialize>(
        &mut self,
        value: T,
        card: &mut impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<T, OccurrenceBindingError> {
        remap_payload_card_ids(value, &mut |raw| {
            let native = crate::ids::CardId::from_raw(raw);
            let owner = self.bind_graph_reference(native, card)?;
            if let Some(slot) = self.payload_bindings.get(&native) {
                return Ok(*slot);
            }
            let slot = u32::try_from(self.payload_card_references.len()).map_err(|_| {
                OccurrenceBindingError::InvalidModel {
                    detail: "too many payload card references".into(),
                }
            })?;
            self.payload_card_references
                .push(serde_json::to_value(owner).map_err(|error| {
                    OccurrenceBindingError::InvalidModel {
                        detail: error.to_string(),
                    }
                })?);
            self.payload_bindings.insert(native, slot);
            Ok(slot)
        })
    }
    fn bind_shared_models<I: serde::Serialize>(
        &mut self,
        card: &mut impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<(), OccurrenceBindingError> {
        // Always traverse the original native model, never an already-framed
        // payload. Publish the entire replacement vector only after success.
        let mut bound_models = Vec::with_capacity(self.models.len());
        let mut bound_embedded = Vec::with_capacity(self.models.len());
        for (canonical, embedded) in self.canonical.clone().into_iter().zip(self.canonical_embedded_definitions.clone()) {
            let native: wire::WireStaticAbility = serde_json::from_value(canonical)
                .map_err(|error| OccurrenceBindingError::InvalidModel { detail: error.to_string() })?;
            let embedded: Vec<RetainedEmbeddedCardDefinition> = serde_json::from_value(embedded)
                .map_err(|error| OccurrenceBindingError::InvalidModel { detail: error.to_string() })?;
            let (model, embedded) = self.bind_payload((native, embedded), card)?;
            bound_models.push(model);
            bound_embedded.push(embedded);
        }
        self.models = bound_models;
        self.embedded_definitions = bound_embedded;
        self.model_card_references
            .fill(RetainedModelCardReferences::Bound);
        Ok(())
    }

    fn encode_embedded_definition(
        &mut self,
        definition: crate::cards::CardDefinition,
    ) -> Result<RetainedOccurrenceCardDefinition<crate::ids::CardId>, OccurrenceBindingError> {
        // The enclosing typed payload traversal binds these native CardIds.
        // Do not bind a second owning graph with a different reference shape.
        let table = std::cell::RefCell::new(self);
        NativeRetainedCardDefinition::from(definition).try_map_payloads(
            Ok,
            |ability| table.borrow_mut().encode_ability(ability),
            |program| table.borrow_mut().encode_program(program),
            |method| table.borrow_mut().encode_alternative_cast(method),
            |optional| table.borrow_mut().encode_optional_cost(optional),
            |cost| table.borrow_mut().encode_total_cost(cost),
        )
    }

    fn encode_effect_with_occurrences(
        &mut self,
        effect: crate::effect::Effect,
        embedded: &mut Vec<RetainedEmbeddedCardDefinition>,
    ) -> Result<wire::WireEffect, OccurrenceBindingError> {
        fn collect(
            effect: &crate::effect::Effect,
            definitions: &mut Vec<crate::cards::CardDefinition>,
        ) {
            effect.visit_card_definitions(&mut |definition| definitions.push(definition.clone()));
            effect.visit_child_effects(&mut |child| collect(child, definitions));
        }
        let model = encode_runtime_effect(effect.clone())?;
        let mut native = Vec::new();
        collect(&effect, &mut native);
        let mut native = native.into_iter();
        let mut error = None;
        let result = runtime_effect_from_core_model_with_card_definitions(
            model.clone(),
            &mut |definition| {
                let association = (|| {
                    let snapshot =
                        native
                            .next()
                            .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                                detail: "canonical effect has no corresponding native template"
                                    .into(),
                            })?;
                    if encode_runtime_definition(snapshot.clone())? != definition {
                        return Err(OccurrenceBindingError::InvalidModel {
                            detail: "native embedded snapshot contradicts canonical definition".into(),
                        });
                    }
                    let retained = self.encode_embedded_definition(snapshot.clone())?;
                    embedded.push(RetainedEmbeddedCardDefinition {
                        model: definition,
                        snapshot: retained,
                    });
                    Ok(snapshot)
                })();
                association.map_err(|failure| {
                    let detail = failure.to_string();
                    error = Some(failure);
                    ArtifactMaterializationError::UnsupportedEffect { detail }
                })
            },
        );
        if let Some(error) = error {
            return Err(error);
        }
        result?;
        if native.next().is_some() {
            return Err(OccurrenceBindingError::InvalidModel {
                detail: "native template has no corresponding canonical definition".into(),
            });
        }
        Ok(model)
    }


    fn encode_cost_with_occurrences(
        &mut self,
        cost: crate::costs::Cost,
        embedded: &mut Vec<RetainedEmbeddedCardDefinition>,
    ) -> Result<wire::WireCost, OccurrenceBindingError> {
        // compiled_model is bound to the immutable payer. Its native effect
        // clones retain the actual runtime template occurrences, not models
        // rematerialized solely to infer identity.
        cost.compiled_model()
            .ok_or(RuntimePayloadEncodingError::MissingModel { component: "cost" })?
            .clone()
            .try_map_effect(|effect| self.encode_effect_with_occurrences(effect, embedded))
    }

    fn encode_alternative_cast(
        &mut self,
        method: crate::alternative_cast::AlternativeCastingMethod,
    ) -> Result<RetainedOccurrenceAlternativeCast, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            let embedded = std::cell::RefCell::new(Vec::new());
            let model = method.try_map(
                |effect| {
                    table
                        .borrow_mut()
                        .encode_effect_with_occurrences(effect, &mut embedded.borrow_mut())
                },
                |cost| {
                    table
                        .borrow_mut()
                        .encode_cost_with_occurrences(cost, &mut embedded.borrow_mut())
                },
            )?;
            Ok(RetainedCardPayload {
                card_references: RetainedModelCardReferences::Native,
                model,
                embedded_definitions: embedded.into_inner(),
            })
        })
    }

    fn encode_optional_cost(
        &mut self,
        cost: crate::cost::OptionalCost,
    ) -> Result<RetainedOccurrenceOptionalCost, OccurrenceBindingError> {
        self.transaction(|table| {
            let mut embedded_definitions = Vec::new();
            let model = cost.try_map(|cost| {
                table.encode_cost_with_occurrences(cost, &mut embedded_definitions)
            })?;
            Ok(RetainedCardPayload {
                card_references: RetainedModelCardReferences::Native,
                model,
                embedded_definitions,
            })
        })
    }

    fn encode_total_cost(
        &mut self,
        cost: crate::cost::TotalCost,
    ) -> Result<RetainedOccurrenceTotalCost, OccurrenceBindingError> {
        self.transaction(|table| {
            let mut embedded_definitions = Vec::new();
            let model = cost.try_map(|cost| {
                table.encode_cost_with_occurrences(cost, &mut embedded_definitions)
            })?;
            Ok(RetainedCardPayload {
                card_references: RetainedModelCardReferences::Native,
                model,
                embedded_definitions,
            })
        })
    }
    fn encode_program(
        &mut self,
        value: crate::resolution::ResolutionProgram,
    ) -> Result<RetainedOccurrenceProgram, OccurrenceBindingError> {
        self.transaction(|table| {
            let mut embedded_definitions = Vec::new();
            let model = value.try_map_effects(|effect| {
                table.encode_effect_with_occurrences(effect, &mut embedded_definitions)
            })?;
            Ok(RetainedCardPayload {
                card_references: RetainedModelCardReferences::Native,
                model,
                embedded_definitions,
            })
        })
    }

    pub fn encode_ability(
        &mut self,
        ability: crate::ability::Ability,
    ) -> Result<RetainedOccurrenceAbility, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            let embedded_definitions = std::cell::RefCell::new(Vec::new());
            let model = ability.try_map(
                |ability| table.borrow_mut().retain(ability),
                |trigger| encode_runtime_trigger(trigger).map_err(Into::into),
                |effect| {
                    table
                        .borrow_mut()
                        .encode_effect_with_occurrences(effect, &mut embedded_definitions.borrow_mut())
                },
                |cost| table.borrow_mut().encode_cost_with_occurrences(cost, &mut embedded_definitions.borrow_mut()),
                Ok,
            )?;
            Ok(RetainedOccurrenceAbility {
                card_references: RetainedModelCardReferences::Native,
                model,
                embedded_definitions: embedded_definitions.into_inner(),
            })
        })
    }
    /// Export one standalone ability with its owning card graph.
    pub fn encode_ability_with_card_graph<I: serde::Serialize>(
        &mut self,
        ability: crate::ability::Ability,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceAbility, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_ability(ability)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }
    pub fn encode_program_with_card_graph<I: serde::Serialize>(
        &mut self,
        value: crate::resolution::ResolutionProgram,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceProgram, OccurrenceBindingError> {
        self.transaction(|table| {
            let bound = { let encoded = table.encode_program(value)?; table.bind_payload(encoded, &mut card)? };
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }
    pub fn encode_alternative_cast_with_card_graph<I: serde::Serialize>(
        &mut self,
        value: crate::alternative_cast::AlternativeCastingMethod,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceAlternativeCast, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_alternative_cast(value)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }
    pub fn encode_optional_cost_with_card_graph<I: serde::Serialize>(
        &mut self,
        value: crate::cost::OptionalCost,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceOptionalCost, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_optional_cost(value)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }
    pub fn encode_total_cost_with_card_graph<I: serde::Serialize>(
        &mut self,
        value: crate::cost::TotalCost,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceTotalCost, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_total_cost(value)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }
    pub fn encode_restriction(
        &mut self,
        value: crate::continuous::RegisteredRestriction,
    ) -> Result<
        crate::continuous::RetainedRestriction<StaticAbilityOccurrenceRef>,
        OccurrenceBindingError,
    > {
        self.transaction(|table| {
            crate::continuous::RetainedRestriction::from(value)
                .try_map_ability(|ability| table.retain(ability))
        })
    }

    pub fn encode_aura_metadata(
        &mut self,
        value: crate::object::AuraAttachmentMetadata,
    ) -> Result<
        crate::object::RetainedAuraAttachmentMetadata<StaticAbilityOccurrenceRef>,
        OccurrenceBindingError,
    > {
        self.transaction(|table| {
            crate::object::RetainedAuraAttachmentMetadata::from(value)
                .try_map_ability(|ability| table.retain(ability))
        })
    }

    pub fn encode_face_down_state_with_card_graph<I: serde::Serialize>(
        &mut self,
        value: crate::object::FaceDownCastState,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceFaceDownCastState, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_face_down_state(value)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }
    pub fn encode_face_down_state(
        &mut self,
        value: crate::object::FaceDownCastState,
    ) -> Result<RetainedOccurrenceFaceDownCastState, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            crate::object::RetainedFaceDownCastState::from(value).try_map_payloads(
                |ability| table.borrow_mut().encode_ability(ability),
                |program| table.borrow_mut().encode_program(program),
                |attachment| table.borrow_mut().encode_aura_metadata(attachment),
            )
        })
    }

    /// Linked card identity belongs to the owning checkpoint's definition graph.
    /// The mandatory binder never treats a sender's allocation as a receiver id.
    pub fn encode_enters_as_copy_restore_state<I: serde::Serialize>(
        &mut self,
        value: crate::object::EntersAsCopyRestoreState,
        face: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceEntersAsCopyRestoreState<I>, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            let face = std::cell::RefCell::new(face);
            let result = crate::object::RetainedEntersAsCopyRestoreState::from(value)
                .try_map_payloads(
                    |ability| {
                        let encoded = table.borrow_mut().encode_ability(ability)?;
                        table
                            .borrow_mut()
                            .bind_payload(encoded, &mut *face.borrow_mut())
                    },
                    |program| {
                        let encoded = table.borrow_mut().encode_program(program)?;
                        table.borrow_mut().bind_payload(encoded, &mut *face.borrow_mut())
                    },
                    |attachment| table.borrow_mut().encode_aura_metadata(attachment),
                    |id| {
                        table
                            .borrow_mut()
                            .bind_graph_reference(id, &mut *face.borrow_mut())
                    },
                )?;
            table
                .borrow_mut()
                .bind_shared_models(&mut *face.borrow_mut())?;
            Ok(result)
        })
    }

    /// Export the complete standalone payload through its owning card graph.
    pub fn encode_copy_values_with_card_graph<I: serde::Serialize>(
        &mut self, value: crate::snapshot::CopiableValues,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<crate::snapshot::RetainedCopiableValues<RetainedOccurrenceAbility>, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_copy_values(value)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }

    /// Export the complete standalone payload through its owning card graph.
    pub fn encode_text_overlay_with_card_graph<I: serde::Serialize>(
        &mut self, value: crate::continuous::TextBoxOverlay,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<crate::continuous::RetainedTextBoxOverlay<RetainedOccurrenceAbility>, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_text_overlay(value)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }

    /// Export the complete standalone payload through its owning card graph.
    pub fn encode_modification_with_card_graph<I: serde::Serialize>(
        &mut self, value: crate::continuous::Modification,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceModification, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_modification(value)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }

    pub fn encode_copy_values(
        &mut self,
        value: crate::snapshot::CopiableValues,
    ) -> Result<
        crate::snapshot::RetainedCopiableValues<RetainedOccurrenceAbility>,
        OccurrenceBindingError,
    > {
        self.transaction(|table| {
            crate::snapshot::RetainedCopiableValues::from(value)
                .try_map_abilities(|ability| table.encode_ability(ability))
        })
    }

    pub fn encode_text_overlay(
        &mut self,
        value: crate::continuous::TextBoxOverlay,
    ) -> Result<
        crate::continuous::RetainedTextBoxOverlay<RetainedOccurrenceAbility>,
        OccurrenceBindingError,
    > {
        self.transaction(|table| {
            crate::continuous::RetainedTextBoxOverlay::from(value)
                .try_map_abilities(|ability| table.encode_ability(ability))
        })
    }
    pub fn encode_modification(
        &mut self,
        value: crate::continuous::Modification,
    ) -> Result<RetainedOccurrenceModification, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            value.try_map_payloads(
                |ability| table.borrow_mut().retain(ability),
                |ability| table.borrow_mut().encode_ability(ability),
                |copy| table.borrow_mut().encode_copy_values(copy),
                |text| table.borrow_mut().encode_text_overlay(text),
                |restriction| table.borrow_mut().encode_restriction(restriction),
                |attachment| table.borrow_mut().encode_aura_metadata(attachment),
            )
        })
    }

    /// Retain all descriptor payloads before binding provenance, including
    /// references to occurrences carried by descriptors later in the vector.
    pub fn encode_registered_state(
        &mut self,
        state: crate::continuous::RegisteredContinuousEffectState,
    ) -> Result<RetainedOccurrenceRegisteredState, OccurrenceBindingError> {
        self.transaction(|table| {
            let retained = state.try_map_effects(|effect| {
                let table = std::cell::RefCell::new(&mut *table);
                effect.try_map_payloads(
                    |value| table.borrow_mut().encode_modification(value),
                    |ability| table.borrow_mut().retain(ability),
                    Ok::<_, OccurrenceBindingError>,
                )
            })?;
            retained.try_map_effects(|effect| {
                effect.try_map_payloads(
                    Ok::<_, OccurrenceBindingError>,
                    Ok::<_, OccurrenceBindingError>,
                    |origin| {
                        origin.try_map_static_instances(&mut |instance| table.reference(instance))
                    },
                )
            })
        })
    }
    pub fn encode_counter_store(
        &mut self,
        value: &crate::object::ObjectCounters,
    ) -> Result<
        crate::object::RetainedObjectCounters<StaticAbilityOccurrenceRef>,
        OccurrenceBindingError,
    > {
        self.transaction(|table| {
            value
                .retain_with_static_occurrences(|ability| {
                    table
                        .retain(ability.clone())
                        .map_err(|error| error.to_string())
                })
                .map_err(|detail| OccurrenceBindingError::InvalidModel { detail })
        })
    }
    pub fn encode_temporary_grants(
        &mut self,
        value: crate::object::TemporaryStaticAbilityGrants,
    ) -> Result<
        crate::object::RetainedTemporaryStaticAbilityGrants<StaticAbilityOccurrenceRef>,
        OccurrenceBindingError,
    > {
        self.transaction(|table| {
            crate::object::RetainedTemporaryStaticAbilityGrants::from(value)
                .try_map_abilities(|ability| table.retain(ability))
        })
    }
}

/// A fresh receiver constructs each occurrence once, then shares only clones.
#[derive(Debug, Clone)]
pub struct StaticAbilityOccurrenceDecoder {
    abilities: Vec<Option<crate::static_abilities::StaticAbility>>,
    payload_cards: Vec<crate::ids::CardId>,
}
impl StaticAbilityOccurrenceDecoder {
    pub fn restore(table: RetainedStaticAbilityTable) -> Result<Self, OccurrenceBindingError> {
        if !table.payload_card_references.is_empty() {
            return Err(OccurrenceBindingError::InvalidModel {
                detail: "owning card graph binder required for payload references".into(),
            });
        }
        Self::restore_with_card_graph::<serde_json::Value>(table, |_| {
            Err(OccurrenceBindingError::InvalidModel {
                detail: "undeclared card graph reference".into(),
            })
        })
    }

    pub fn restore_with_card_graph<I: serde::de::DeserializeOwned>(
        table: RetainedStaticAbilityTable,
        mut card: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<Self, OccurrenceBindingError> {
        if table.models.len() > u32::MAX as usize
            || table.payload_card_references.len() > u32::MAX as usize
        {
            return Err(OccurrenceBindingError::TooManyOccurrences);
        }
        if table.model_card_references.len() != table.models.len()
            || table.embedded_definitions.len() != table.models.len()
        {
            return Err(OccurrenceBindingError::InvalidModel {
                detail: "shared model reference modes do not match model count".into(),
            });
        }
        let mut payload_cards = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut wire_seen = std::collections::HashSet::new();
        for reference in table.payload_card_references {
            if reference.is_null() || !wire_seen.insert(reference.clone()) {
                return Err(OccurrenceBindingError::InvalidModel {
                    detail: "null or duplicate owning card graph reference".into(),
                });
            }
            let reference = serde_json::from_value(reference).map_err(|error| {
                OccurrenceBindingError::InvalidModel {
                    detail: error.to_string(),
                }
            })?;
            let native = card(reference)?;
            if !seen.insert(native) {
                return Err(OccurrenceBindingError::InvalidModel {
                    detail: "distinct graph references bind the same receiver card".into(),
                });
            }
            payload_cards.push(native);
        }

    let count = table.models.len();
    let mut decoder = Self {
        abilities: vec![None; count],
        payload_cards,
    };
    let mut pending = Vec::with_capacity(count);
    let mut dependencies = Vec::with_capacity(count);
    for ((model, embedded_definitions), card_references) in table
        .models
        .into_iter()
        .zip(table.embedded_definitions)
        .zip(table.model_card_references)
    {
        let payload = decoder.bind_retained_payload_with_definitions(RetainedCardPayload {
            card_references,
            model,
            embedded_definitions,
        })?;
        let mut required = std::collections::HashSet::new();
        Self::embedded_occurrence_dependencies(&payload.embedded_definitions, &mut required);
        for reference in &required {
            if reference.0 as usize >= count {
                return Err(OccurrenceBindingError::UnknownReference {
                    reference: *reference,
                });
            }
        }
        dependencies.push(required);
        pending.push(Some(payload));
    }
    let mut remaining = count;
    while remaining != 0 {
        let mut progress = false;
        for index in 0..count {
            if pending[index].is_none()
                || !dependencies[index]
                    .iter()
                    .all(|reference| decoder.abilities[reference.0 as usize].is_some())
            {
                continue;
            }
            let payload =
                pending[index]
                    .take()
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "missing ready shared occurrence payload".into(),
                    })?;
            let embedded = std::cell::RefCell::new(payload.embedded_definitions.into());
            let model = payload.model.try_map(
                |trigger| {
                    runtime_trigger_from_core_model(trigger).map_err(OccurrenceBindingError::from)
                },
                |effect| {
                    decoder.restore_effect_with_embedded_definitions(
                        effect,
                        &mut embedded.borrow_mut(),
                    )
                },
                |cost| {
                    decoder.restore_cost_with_embedded_definitions(cost, &mut embedded.borrow_mut())
                },
                Ok,
            )?;
            Self::finish_embedded_definitions(&embedded.into_inner())?;
            decoder.abilities[index] =
                Some(crate::static_abilities::StaticAbility::from_model(model));
            remaining -= 1;
            progress = true;
        }
        if !progress {
            let blocked: Vec<_> = pending
                .iter()
                .enumerate()
                .filter_map(|(index, value)| value.as_ref().map(|_| index))
                .collect();
            return Err(OccurrenceBindingError::InvalidModel {
                detail: format!("cyclic shared occurrence dependencies at slots {blocked:?}"),
            });
        }
    }
        Ok(decoder)
    }


    fn embedded_occurrence_dependencies(
        embedded: &[RetainedEmbeddedCardDefinition],
        dependencies: &mut std::collections::HashSet<StaticAbilityOccurrenceRef>,
    ) {
        for association in embedded {
            let snapshot = &association.snapshot;
            for ability in &snapshot.abilities {
                if let ironsmith_core::AbilityKind::Static(reference) = &ability.model.kind {
                    dependencies.insert(*reference);
                }
                Self::embedded_occurrence_dependencies(&ability.embedded_definitions, dependencies);
            }
            if let Some(program) = &snapshot.spell_effect {
                Self::embedded_occurrence_dependencies(&program.embedded_definitions, dependencies);
            }
            for method in &snapshot.alternative_casts {
                Self::embedded_occurrence_dependencies(&method.embedded_definitions, dependencies);
            }
            for optional in &snapshot.optional_costs {
                Self::embedded_occurrence_dependencies(
                    &optional.embedded_definitions,
                    dependencies,
                );
            }
            Self::embedded_occurrence_dependencies(
                &snapshot.additional_cost.embedded_definitions,
                dependencies,
            );
        }
    }
    fn bind_payload<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        value: T,
    ) -> Result<T, OccurrenceBindingError> {
        remap_payload_card_ids(value, &mut |slot| {
            self.payload_cards
                .get(slot as usize)
                .map(|card| card.0)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: format!("unknown payload card reference {slot}"),
                })
        })
    }
    pub fn ability(
        &self,
        reference: StaticAbilityOccurrenceRef,
    ) -> Result<crate::static_abilities::StaticAbility, OccurrenceBindingError> {
        self.abilities
            .get(reference.0 as usize)
            .and_then(Clone::clone)
            .ok_or(OccurrenceBindingError::UnknownReference { reference })
    }

    pub fn instance_id(
        &self,
        reference: StaticAbilityOccurrenceRef,
    ) -> Result<crate::static_abilities::StaticAbilityInstanceId, OccurrenceBindingError> {
        Ok(self.ability(reference)?.instance_id())
    }


    fn bind_retained_payload_with_definitions<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        value: RetainedCardPayload<T>,
    ) -> Result<RetainedCardPayload<T>, OccurrenceBindingError> {
        match value.card_references {
            RetainedModelCardReferences::Bound => {
                // Each nested retained payload owns its own framed namespace.
                // Resolve only this payload and the snapshot's direct card
                // fields; child restore calls resolve their payloads once.
                let mut embedded_definitions = Vec::new();
                for association in value.embedded_definitions {
                    embedded_definitions.push(RetainedEmbeddedCardDefinition {
                        model: self.bind_payload(association.model)?,
                        snapshot: association.snapshot.try_map_payloads(
                            |id| self.bind_payload(id), Ok, Ok, Ok, Ok, Ok,
                        )?,
                    });
                }
                Ok(RetainedCardPayload {
                    card_references: RetainedModelCardReferences::Bound,
                    model: self.bind_payload(value.model)?,
                    embedded_definitions,
                })
            },
            RetainedModelCardReferences::Native => remap_payload_card_ids(value, &mut |id| {
                Err(OccurrenceBindingError::InvalidModel {
                    detail: format!("native card reference {id} in unbound retained payload"),
                })
            }),
        }
    }

    fn restore_effect_with_embedded_definitions(
        &self,
        effect: wire::WireEffect,
        embedded: &mut std::collections::VecDeque<RetainedEmbeddedCardDefinition>,
    ) -> Result<crate::effect::Effect, OccurrenceBindingError> {
        let mut error = None;
        let result = runtime_effect_from_core_model_with_card_definitions(effect, &mut |model| {
            let snapshot = (|| {
                let association =
                    embedded
                        .pop_front()
                        .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                            detail: "missing retained embedded definition association".into(),
                        })?;
                if association.model != model {
                    return Err(OccurrenceBindingError::InvalidModel {
                        detail: "retained embedded definition does not match executable model"
                            .into(),
                    });
                }
                let snapshot = self.restore_card_definition(association.snapshot, Ok)?;
                if encode_runtime_definition(snapshot.clone())? != model {
                    return Err(OccurrenceBindingError::InvalidModel {
                        detail: "retained embedded snapshot contradicts canonical definition".into(),
                    });
                }
                Ok(snapshot)
            })();
            snapshot.map_err(|failure| {
                let detail = failure.to_string();
                error = Some(failure);
                ArtifactMaterializationError::UnsupportedEffect { detail }
            })
        });
        if let Some(error) = error {
            return Err(error);
        }
        result.map_err(Into::into)
    }

    fn finish_embedded_definitions<T>(remaining: &T) -> Result<(), OccurrenceBindingError>
    where
        for<'a> &'a T: IntoIterator,
    {
        if remaining.into_iter().next().is_some() {
            return Err(OccurrenceBindingError::InvalidModel {
                detail: "unused retained embedded definition association".into(),
            });
        }
        Ok(())
    }


    fn restore_cost_with_embedded_definitions(
        &self,
        cost: wire::WireCost,
        embedded: &mut std::collections::VecDeque<RetainedEmbeddedCardDefinition>,
    ) -> Result<crate::costs::Cost, OccurrenceBindingError> {
        let model = cost.try_map_effect(|effect| {
            self.restore_effect_with_embedded_definitions(effect, embedded)
        })?;
        crate::costs::Cost::from_model(model)
            .map_err(|detail| ArtifactMaterializationError::UnsupportedEffect { detail }.into())
    }
    pub fn restore_program(
        &self,
        value: RetainedOccurrenceProgram,
    ) -> Result<crate::resolution::ResolutionProgram, OccurrenceBindingError> {
        let value = self.bind_retained_payload_with_definitions(value)?;
        let mut embedded = value.embedded_definitions.into();
        let result = value.model.try_map_effects(|effect| {
            self.restore_effect_with_embedded_definitions(effect, &mut embedded)
        })?;
        Self::finish_embedded_definitions(&embedded)?;
        Ok(result)
    }

    pub fn restore_ability(
        &self,
        ability: RetainedOccurrenceAbility,
    ) -> Result<crate::ability::Ability, OccurrenceBindingError> {
        let value = self.bind_retained_payload_with_definitions(ability)?;
        let embedded = std::cell::RefCell::new(value.embedded_definitions.into());
        let result = value.model.try_map(
            |reference| self.ability(reference),
            |trigger| runtime_trigger_from_core_model(trigger).map_err(Into::into),
            |effect| {
                self.restore_effect_with_embedded_definitions(effect, &mut embedded.borrow_mut())
            },
            |cost| self.restore_cost_with_embedded_definitions(cost, &mut embedded.borrow_mut()),
            Ok,
        )?;
        Self::finish_embedded_definitions(&embedded.into_inner())?;
        Ok(result)
    }
    pub fn restore_alternative_cast(
        &self,
        value: RetainedOccurrenceAlternativeCast,
    ) -> Result<crate::alternative_cast::AlternativeCastingMethod, OccurrenceBindingError> {
        let value = self.bind_retained_payload_with_definitions(value)?;
        let embedded = std::cell::RefCell::new(value.embedded_definitions.into());
        // Exact native retained state was already normalized/detargeted by
        // definition compilation. Restore does not normalize it a second time.
        let result = value.model.try_map(
            |effect| {
                self.restore_effect_with_embedded_definitions(effect, &mut embedded.borrow_mut())
            },
            |cost| self.restore_cost_with_embedded_definitions(cost, &mut embedded.borrow_mut()),
        )?;
        Self::finish_embedded_definitions(&embedded.into_inner())?;
        Ok(result)
    }
    pub fn restore_optional_cost(
        &self,
        value: RetainedOccurrenceOptionalCost,
    ) -> Result<crate::cost::OptionalCost, OccurrenceBindingError> {
        let value = self.bind_retained_payload_with_definitions(value)?;
        let mut embedded = value.embedded_definitions.into();
        let result = value
            .model
            .try_map(|cost| self.restore_cost_with_embedded_definitions(cost, &mut embedded))?;
        Self::finish_embedded_definitions(&embedded)?;
        Ok(result)
    }
    pub fn restore_total_cost(
        &self,
        value: RetainedOccurrenceTotalCost,
    ) -> Result<crate::cost::TotalCost, OccurrenceBindingError> {
        let value = self.bind_retained_payload_with_definitions(value)?;
        let mut embedded = value.embedded_definitions.into();
        let result = value
            .model
            .try_map(|cost| self.restore_cost_with_embedded_definitions(cost, &mut embedded))?;
        Self::finish_embedded_definitions(&embedded)?;
        Ok(result)
    }
    pub fn restore_restriction(
        &self,
        value: crate::continuous::RetainedRestriction<StaticAbilityOccurrenceRef>,
    ) -> Result<crate::continuous::RegisteredRestriction, OccurrenceBindingError> {
        value
            .try_map_ability(|reference| self.ability(reference))?
            .try_into()
            .map_err(|error: crate::continuous::RestrictionAbilityMismatch| {
                ArtifactMaterializationError::UnsupportedStaticAbility {
                    detail: error.to_string(),
                }
                .into()
            })
    }

    pub fn restore_aura_metadata(
        &self,
        value: crate::object::RetainedAuraAttachmentMetadata<StaticAbilityOccurrenceRef>,
    ) -> Result<crate::object::AuraAttachmentMetadata, OccurrenceBindingError> {
        value
            .try_map_ability(|reference| self.ability(reference))?
            .try_into()
            .map_err(|error: crate::object::AuraAttachmentAbilityMismatch| {
                ArtifactMaterializationError::UnsupportedStaticAbility {
                    detail: error.to_string(),
                }
                .into()
            })
    }

    pub fn restore_face_down_state(
        &self,
        value: RetainedOccurrenceFaceDownCastState,
    ) -> Result<crate::object::FaceDownCastState, OccurrenceBindingError> {
        Ok(value
            .try_map_payloads(
                |ability| self.restore_ability(ability),
                |program| self.restore_program(program),
                |attachment| self.restore_aura_metadata(attachment),
            )?
            .into())
    }

    pub fn restore_enters_as_copy_restore_state<I>(
        &self,
        value: RetainedOccurrenceEntersAsCopyRestoreState<I>,
        face: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<crate::object::EntersAsCopyRestoreState, OccurrenceBindingError> {
        Ok(value
            .try_map_payloads(
                |ability| self.restore_ability(ability),
                |program| self.restore_program(program),
                |attachment| self.restore_aura_metadata(attachment),
                face,
            )?
            .into())
    }

    pub fn restore_copy_values(
        &self,
        value: crate::snapshot::RetainedCopiableValues<RetainedOccurrenceAbility>,
    ) -> Result<crate::snapshot::CopiableValues, OccurrenceBindingError> {
        value
            .try_map_abilities(|ability| self.restore_ability(ability))
            .map(Into::into)
    }

    pub fn restore_text_overlay(
        &self,
        value: crate::continuous::RetainedTextBoxOverlay<RetainedOccurrenceAbility>,
    ) -> Result<crate::continuous::TextBoxOverlay, OccurrenceBindingError> {
        value
            .try_map_abilities(|ability| self.restore_ability(ability))
            .map(Into::into)
    }
    pub fn restore_modification(
        &self,
        value: RetainedOccurrenceModification,
    ) -> Result<crate::continuous::Modification, OccurrenceBindingError> {
        value.try_map_payloads(
            |reference| self.ability(reference),
            |ability| self.restore_ability(ability),
            |copy| self.restore_copy_values(copy),
            |text| self.restore_text_overlay(text),
            |restriction| self.restore_restriction(restriction),
            |attachment| self.restore_aura_metadata(attachment),
        )
    }

    /// Produces complete native state; the owning importer still validates
    /// game references and atomically publishes it through the manager API.
    pub fn restore_registered_state(
        &self,
        state: RetainedOccurrenceRegisteredState,
    ) -> Result<crate::continuous::RegisteredContinuousEffectState, OccurrenceBindingError> {
        state.try_map_effects(|effect| {
            effect.try_map_payloads(
                |value| self.restore_modification(value),
                |reference| self.ability(reference),
                |origin| {
                    origin.try_map_static_instances(&mut |reference| self.instance_id(reference))
                },
            )
        })
    }
    pub fn restore_counter_store(
        &self,
        value: crate::object::RetainedObjectCounters<StaticAbilityOccurrenceRef>,
    ) -> Result<crate::object::ObjectCounters, OccurrenceBindingError> {
        crate::object::ObjectCounters::restore_with_static_occurrences(value, |reference| {
            self.ability(reference).map_err(|error| error.to_string())
        })
        .map_err(|detail| OccurrenceBindingError::InvalidModel { detail })
    }
    pub fn restore_temporary_grants(
        &self,
        value: crate::object::RetainedTemporaryStaticAbilityGrants<StaticAbilityOccurrenceRef>,
    ) -> Result<crate::object::TemporaryStaticAbilityGrants, OccurrenceBindingError> {
        value
            .try_map_abilities(|reference| self.ability(reference))?
            .try_into()
            .map_err(|detail| OccurrenceBindingError::InvalidModel { detail })
    }
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

impl StaticAbilityOccurrenceEncoder {
    pub fn encode_bestow_state_with_card_graph<I: serde::Serialize>(
        &mut self,
        value: crate::object::BestowCastState,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceBestowCastState, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_bestow_state(value)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }
    pub fn encode_bestow_state(
        &mut self,
        value: crate::object::BestowCastState,
    ) -> Result<RetainedOccurrenceBestowCastState, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            crate::object::RetainedBestowCastState::from(value)
                .try_map_payloads(|program| table.borrow_mut().encode_program(program), |attachment| {
                    table.borrow_mut().encode_aura_metadata(attachment)
                })
        })
    }
    pub fn encode_splice_state_with_card_graph<I: serde::Serialize>(
        &mut self,
        value: crate::object::SpliceCastState,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceSpliceCastState, OccurrenceBindingError> {
        self.transaction(|table| {
            let encoded = table.encode_splice_state(value)?;
            let bound = table.bind_payload(encoded, &mut card)?;
            table.bind_shared_models(&mut card)?;
            Ok(bound)
        })
    }
    pub fn encode_splice_state(
        &mut self,
        value: crate::object::SpliceCastState,
    ) -> Result<RetainedOccurrenceSpliceCastState, OccurrenceBindingError> {
        self.transaction(|table| {
            crate::object::RetainedSpliceCastState::from(value)
                .try_map_payloads(|program| table.encode_program(program))
        })
    }
}
impl StaticAbilityOccurrenceDecoder {
    pub fn restore_bestow_state(
        &self,
        value: RetainedOccurrenceBestowCastState,
    ) -> Result<crate::object::BestowCastState, OccurrenceBindingError> {
        Ok(value
            .try_map_payloads(
                |program| self.restore_program(program),
                |attachment| self.restore_aura_metadata(attachment),
            )?
            .into())
    }
    pub fn restore_splice_state(
        &self,
        value: RetainedOccurrenceSpliceCastState,
    ) -> Result<crate::object::SpliceCastState, OccurrenceBindingError> {
        Ok(value
            .try_map_payloads(|program| self.restore_program(program))?
            .into())
    }
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

pub type RetainedOccurrenceObjectSnapshot<I> =
    crate::snapshot::RetainedObjectSnapshot<RetainedOccurrenceAbility, I>;
impl StaticAbilityOccurrenceEncoder {
    /// Historical references use the same occurrence table as current objects.
    /// Card ids always pass through the owning definition-graph binder.
    pub fn encode_snapshot<I: serde::Serialize>(
        &mut self,
        value: crate::snapshot::ObjectSnapshot,
        card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceObjectSnapshot<I>, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            let card = std::cell::RefCell::new(card);
            let result = crate::snapshot::RetainedObjectSnapshot::from(value).try_map_payloads(
                |ability| {
                    let encoded = table.borrow_mut().encode_ability(ability)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |face| {
                    table
                        .borrow_mut()
                        .bind_graph_reference(face, &mut *card.borrow_mut())
                },
            )?;
            table
                .borrow_mut()
                .bind_shared_models(&mut *card.borrow_mut())?;
            Ok(result)
        })
    }
}
impl StaticAbilityOccurrenceDecoder {
    pub fn restore_snapshot<I>(
        &self,
        value: RetainedOccurrenceObjectSnapshot<I>,
        card: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<crate::snapshot::ObjectSnapshot, OccurrenceBindingError> {
        Ok(value
            .try_map_payloads(|ability| self.restore_ability(ability), card)?
            .into())
    }
}

pub type RetainedGraphRegisteredState<I> = crate::continuous::RegisteredContinuousEffectState<
    crate::continuous::ContinuousEffect<
        RetainedOccurrenceModification,
        StaticAbilityOccurrenceRef,
        crate::continuous::ContinuousAbilityOrigin<StaticAbilityOccurrenceRef, I>,
    >,
>;
impl StaticAbilityOccurrenceEncoder {
    /// Retain descriptors and bind all nested printed faces atomically. The
    /// owning checkpoint supplies its card-definition graph, never raw ids.
    pub fn encode_registered_state_with_card_graph<I: serde::Serialize>(
        &mut self,
        value: crate::continuous::RegisteredContinuousEffectState,
        card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedGraphRegisteredState<I>, OccurrenceBindingError> {
        self.transaction(|table| {
            let retained = table.encode_registered_state(value)?;
            let table = std::cell::RefCell::new(table);
            let card = std::cell::RefCell::new(card);
            let result = retained.try_map_effects(|effect| {
                effect.try_map_payloads(
                    |value| {
                        table
                            .borrow_mut()
                            .bind_payload(value, &mut *card.borrow_mut())
                    },
                    Ok::<_, OccurrenceBindingError>,
                    |origin| {
                        origin.try_map_card_ids(&mut |face| {
                            table
                                .borrow_mut()
                                .bind_graph_reference(face, &mut *card.borrow_mut())
                        })
                    },
                )
            })?;
            table
                .borrow_mut()
                .bind_shared_models(&mut *card.borrow_mut())?;
            Ok(result)
        })
    }
}
impl StaticAbilityOccurrenceDecoder {
    pub fn restore_registered_state_with_card_graph<I>(
        &self,
        value: RetainedGraphRegisteredState<I>,
        mut card: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<crate::continuous::RegisteredContinuousEffectState, OccurrenceBindingError> {
        let bound = value.try_map_effects(|effect| {
            effect.try_map_payloads(
                Ok::<_, OccurrenceBindingError>,
                Ok::<_, OccurrenceBindingError>,
                |origin| origin.try_map_card_ids(&mut card),
            )
        })?;
        self.restore_registered_state(bound)
    }
}

/// Replacement identities share the native occurrence and definition tables
/// with live objects, continuous effects, grants and historical snapshots.
pub type RetainedOccurrenceReplacementOrigin<I> =
    crate::replacement::ReplacementAbilityOrigin<StaticAbilityOccurrenceRef, I>;
pub type RetainedOccurrenceReplacementKey<I> =
    crate::replacement::ReplacementEffectKey<StaticAbilityOccurrenceRef, I>;

impl StaticAbilityOccurrenceEncoder {
    /// Origin payloads must already be retained from visibility-approved roots.
    /// All nested references and shared models bind in one atomic transaction.
    pub fn encode_replacement_origin<I: serde::Serialize>(
        &mut self,
        value: crate::replacement::ReplacementAbilityOrigin,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceReplacementOrigin<I>, OccurrenceBindingError> {
        self.transaction(|table| {
            let value = value.try_map_static_instances(&mut |id| table.reference(id))?
                .try_map_card_ids(&mut |id| table.bind_graph_reference(id, &mut card))?;
            table.bind_shared_models(&mut card)?;
            Ok(value)
        })
    }

    /// Retain application history and decline-parent identity in the same tables
    /// as the descriptor. Legacy structural strings remain identity data; this
    /// API does not establish their portability after executable/world rebinding.
    pub fn encode_replacement_key<I: serde::Serialize>(
        &mut self,
        value: crate::replacement::ReplacementEffectKey,
        mut card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceReplacementKey<I>, OccurrenceBindingError> {
        self.transaction(|table| {
            let value = value.try_map_static_instances(&mut |id| table.reference(id))?
                .try_map_card_ids(&mut |id| table.bind_graph_reference(id, &mut card))?;
            table.bind_shared_models(&mut card)?;
            Ok(value)
        })
    }
}

impl StaticAbilityOccurrenceDecoder {
    pub fn restore_replacement_origin<I>(
        &self,
        value: RetainedOccurrenceReplacementOrigin<I>,
        mut card: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<crate::replacement::ReplacementAbilityOrigin, OccurrenceBindingError> {
        value.try_map_static_instances(&mut |reference| self.instance_id(reference))?
            .try_map_card_ids(&mut card)
    }

    pub fn restore_replacement_key<I>(
        &self,
        value: RetainedOccurrenceReplacementKey<I>,
        mut card: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<crate::replacement::ReplacementEffectKey, OccurrenceBindingError> {
        value.try_map_static_instances(&mut |reference| self.instance_id(reference))?
            .try_map_card_ids(&mut card)
    }
}

pub type RetainedOccurrenceGrantPermission<I> =
    crate::grant_registry::RetainedGrantPermissionIdentity<
        crate::continuous::AbilityOrigin<StaticAbilityOccurrenceRef, I>,
        I,
    >;
impl StaticAbilityOccurrenceEncoder {
    /// Referenced origin payloads must already belong to the shared table.
    pub fn encode_permission_identity<I: serde::Serialize>(
        &mut self,
        value: crate::grant_registry::GrantPermissionIdentity,
        card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceGrantPermission<I>, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            let card = std::cell::RefCell::new(card);
            let result = crate::grant_registry::RetainedGrantPermissionIdentity::from(value)
                .try_map_payloads(
                    |origin| {
                        let retained = origin.try_map_static_instances(&mut |instance| {
                            table.borrow().reference(instance)
                        })?;
                        retained.try_map_card_ids(&mut |id| {
                            table
                                .borrow_mut()
                                .bind_graph_reference(id, &mut *card.borrow_mut())
                        })
                    },
                    |id| {
                        table
                            .borrow_mut()
                            .bind_graph_reference(id, &mut *card.borrow_mut())
                    },
                )?;
            table
                .borrow_mut()
                .bind_shared_models(&mut *card.borrow_mut())?;
            Ok(result)
        })
    }
}
impl StaticAbilityOccurrenceDecoder {
    pub fn restore_permission_identity<I>(
        &self,
        value: RetainedOccurrenceGrantPermission<I>,
        card: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<crate::grant_registry::GrantPermissionIdentity, OccurrenceBindingError> {
        let card = std::cell::RefCell::new(card);
        Ok(value
            .try_map_payloads(
                |origin| {
                    origin
                        .try_map_static_instances(&mut |reference| self.instance_id(reference))?
                        .try_map_card_ids(&mut |face| (*card.borrow_mut())(face))
                },
                |face| (*card.borrow_mut())(face),
            )?
            .into())
    }
}

/// Captured cast facts use the same occurrence and definition graph as live
/// objects, registered descriptors, and their historical source snapshots.
pub type RetainedOccurrenceCastPaymentState<I> = crate::object::RetainedCastPaymentState<
    RetainedOccurrenceAlternativeCast,
    RetainedOccurrenceOptionalCost,
    RetainedOccurrenceTotalCost,
    RetainedOccurrenceGrantPermission<I>,
    RetainedOccurrenceObjectSnapshot<I>,
>;
impl StaticAbilityOccurrenceEncoder {
    pub fn encode_cast_payment_state<I: serde::Serialize>(
        &mut self,
        value: crate::object::NativeCastPaymentState,
        card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceCastPaymentState<I>, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            let card = std::cell::RefCell::new(card);
            let result = value.try_map_payloads(
                |method| {
                    let encoded = table.borrow_mut().encode_alternative_cast(method)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |optional| {
                    let encoded = table.borrow_mut().encode_optional_cost(optional)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |cost| {
                    let encoded = table.borrow_mut().encode_total_cost(cost)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |permission| {
                    table
                        .borrow_mut()
                        .encode_permission_identity(permission, |face| (*card.borrow_mut())(face))
                },
                |snapshot| {
                    table
                        .borrow_mut()
                        .encode_snapshot(snapshot, |face| (*card.borrow_mut())(face))
                },
            )?;
            table
                .borrow_mut()
                .bind_shared_models(&mut *card.borrow_mut())?;
            Ok(result)
        })
    }
}
impl StaticAbilityOccurrenceDecoder {
    pub fn restore_cast_payment_state<I>(
        &self,
        value: RetainedOccurrenceCastPaymentState<I>,
        card: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<crate::object::NativeCastPaymentState, OccurrenceBindingError> {
        let card = std::cell::RefCell::new(card);
        value.try_map_payloads(
            |method| self.restore_alternative_cast(method),
            |optional| self.restore_optional_cost(optional),
            |cost| self.restore_total_cost(cost),
            |permission| {
                self.restore_permission_identity(permission, |face| (*card.borrow_mut())(face))
            },
            |snapshot| self.restore_snapshot(snapshot, |face| (*card.borrow_mut())(face)),
        )
    }
}

/// Complete live object state uses one shared occurrence and definition graph.
/// The game importer still owns extension maps and object/player validation.
pub type RetainedOccurrenceLiveObject<I> = crate::object::RetainedLiveObject<
    RetainedOccurrenceAbility,
    RetainedOccurrenceProgram,
    crate::object::RetainedAuraAttachmentMetadata<StaticAbilityOccurrenceRef>,
    I,
    crate::object::RetainedObjectCounters<StaticAbilityOccurrenceRef>,
    crate::object::RetainedTemporaryStaticAbilityGrants<StaticAbilityOccurrenceRef>,
    RetainedOccurrenceCastPaymentState<I>,
>;
impl StaticAbilityOccurrenceEncoder {
    pub fn encode_live_object<I: serde::Serialize>(
        &mut self,
        value: crate::object::Object,
        card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceLiveObject<I>, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            let card = std::cell::RefCell::new(card);
            let result = crate::object::NativeRetainedLiveObject::from(value).try_map_payloads(
                |ability| {
                    let encoded = table.borrow_mut().encode_ability(ability)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |program| {
                    let encoded = table.borrow_mut().encode_program(program)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |attachment| table.borrow_mut().encode_aura_metadata(attachment),
                |face| {
                    table
                        .borrow_mut()
                        .bind_graph_reference(face, &mut *card.borrow_mut())
                },
                |counters| table.borrow_mut().encode_counter_store(&counters),
                |grants| table.borrow_mut().encode_temporary_grants(grants),
                |capture| {
                    table
                        .borrow_mut()
                        .encode_cast_payment_state(capture, |face| (*card.borrow_mut())(face))
                },
            )?;
            table
                .borrow_mut()
                .bind_shared_models(&mut *card.borrow_mut())?;
            Ok(result)
        })
    }
}
impl StaticAbilityOccurrenceDecoder {
    pub fn restore_live_object<I>(
        &self,
        value: RetainedOccurrenceLiveObject<I>,
        card: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<crate::object::Object, OccurrenceBindingError> {
        let card = std::cell::RefCell::new(card);
        value
            .try_map_payloads(
                |ability| self.restore_ability(ability),
                |program| self.restore_program(program),
                |attachment| self.restore_aura_metadata(attachment),
                |face| (*card.borrow_mut())(face),
                |counters| self.restore_counter_store(counters),
                |grants| self.restore_temporary_grants(grants),
                |capture| {
                    self.restore_cast_payment_state(capture, |face| (*card.borrow_mut())(face))
                },
            )?
            .try_into()
            .map_err(|detail| OccurrenceBindingError::InvalidModel { detail })
    }
}

fn deserialize_present_definition_field<
    'de,
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    <Option<T> as serde::Deserialize>::deserialize(deserializer)
}

/// Complete printed card facts with explicit owning definition-graph IDs.
/// Legacy artifact defaults are deliberately not inherited by checkpoints.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "I: serde::Deserialize<'de>"))]
pub struct RetainedCard<I> {
    pub id: I,
    pub name: String,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub first_printed_set_name: Option<String>,
    pub attraction_lights: Vec<u8>,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub mana_cost: Option<crate::mana::ManaCost>,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub color_indicator: Option<crate::color::ColorSet>,
    pub supertypes: Vec<crate::types::Supertype>,
    pub card_types: Vec<crate::types::CardType>,
    pub subtypes: Vec<crate::types::Subtype>,
    pub rules_text_color_identity: crate::color::ColorSet,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub power_toughness: Option<crate::card::PowerToughness>,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub loyalty: Option<u32>,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub defense: Option<u32>,
    pub hand_modifier: i32,
    pub life_modifier: i32,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub other_face: Option<I>,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub other_face_name: Option<String>,
    pub linked_face_layout: crate::card::LinkedFaceLayout,
    pub transforming_dfc: bool,
    pub is_token: bool,
}
impl From<crate::card::Card> for RetainedCard<crate::ids::CardId> {
    fn from(card: crate::card::Card) -> Self {
        let crate::card::Card {
            id,
            name,
            first_printed_set_name,
            attraction_lights,
            mana_cost,
            color_indicator,
            supertypes,
            card_types,
            subtypes,
            rules_text_color_identity,
            power_toughness,
            loyalty,
            defense,
            hand_modifier,
            life_modifier,
            other_face,
            other_face_name,
            linked_face_layout,
            transforming_dfc,
            is_token,
        } = card;
        Self {
            id,
            name,
            first_printed_set_name,
            attraction_lights,
            mana_cost,
            color_indicator,
            supertypes,
            card_types,
            subtypes,
            rules_text_color_identity,
            power_toughness,
            loyalty,
            defense,
            hand_modifier,
            life_modifier,
            other_face,
            other_face_name,
            linked_face_layout,
            transforming_dfc,
            is_token,
        }
    }
}
impl From<RetainedCard<crate::ids::CardId>> for crate::card::Card {
    fn from(card: RetainedCard<crate::ids::CardId>) -> Self {
        let RetainedCard {
            id,
            name,
            first_printed_set_name,
            attraction_lights,
            mana_cost,
            color_indicator,
            supertypes,
            card_types,
            subtypes,
            rules_text_color_identity,
            power_toughness,
            loyalty,
            defense,
            hand_modifier,
            life_modifier,
            other_face,
            other_face_name,
            linked_face_layout,
            transforming_dfc,
            is_token,
        } = card;
        Self {
            id,
            name,
            first_printed_set_name,
            attraction_lights,
            mana_cost,
            color_indicator,
            supertypes,
            card_types,
            subtypes,
            rules_text_color_identity,
            power_toughness,
            loyalty,
            defense,
            hand_modifier,
            life_modifier,
            other_face,
            other_face_name,
            linked_face_layout,
            transforming_dfc,
            is_token,
        }
    }
}
impl<I> RetainedCard<I> {
    pub fn try_map_ids<J, E>(
        self,
        mut bind: impl FnMut(I) -> Result<J, E>,
    ) -> Result<RetainedCard<J>, E> {
        let Self {
            id,
            name,
            first_printed_set_name,
            attraction_lights,
            mana_cost,
            color_indicator,
            supertypes,
            card_types,
            subtypes,
            rules_text_color_identity,
            power_toughness,
            loyalty,
            defense,
            hand_modifier,
            life_modifier,
            other_face,
            other_face_name,
            linked_face_layout,
            transforming_dfc,
            is_token,
        } = self;
        Ok(RetainedCard {
            id: bind(id)?,
            name,
            first_printed_set_name,
            attraction_lights,
            mana_cost,
            color_indicator,
            supertypes,
            card_types,
            subtypes,
            rules_text_color_identity,
            power_toughness,
            loyalty,
            defense,
            hand_modifier,
            life_modifier,
            other_face: other_face.map(&mut bind).transpose()?,
            other_face_name,
            linked_face_layout,
            transforming_dfc,
            is_token,
        })
    }
}
/// Exact runtime definition. Restoring does not rerun definition-time
/// level/class merging, target normalization, or program cleanup.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound(
    deserialize = "I: serde::Deserialize<'de>, A: serde::Deserialize<'de>, P: serde::Deserialize<'de>, M: serde::Deserialize<'de>, K: serde::Deserialize<'de>, C: serde::Deserialize<'de>"
))]
pub struct RetainedCardDefinition<I, A, P, M, K, C> {
    pub card: RetainedCard<I>,
    pub canonical_text: String,
    pub ability_labels: Vec<String>,
    pub abilities: Vec<A>,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub spell_effect: Option<P>,
    #[serde(deserialize_with = "deserialize_present_definition_field")]
    pub aura_attach_filter: Option<crate::object::AuraAttachmentFilter>,
    pub alternative_casts: Vec<M>,
    pub has_fuse: bool,
    pub optional_costs: Vec<K>,
    pub additional_cost: C,
    pub refers_to_ante: bool,
}
pub type NativeRetainedCardDefinition = RetainedCardDefinition<
    crate::ids::CardId,
    crate::ability::Ability,
    crate::resolution::ResolutionProgram,
    crate::alternative_cast::AlternativeCastingMethod,
    crate::cost::OptionalCost,
    crate::cost::TotalCost,
>;
impl From<crate::cards::CardDefinition> for NativeRetainedCardDefinition {
    fn from(value: crate::cards::CardDefinition) -> Self {
        let crate::cards::CardDefinition {
            card,
            canonical_text,
            ability_labels,
            abilities,
            spell_effect,
            aura_attach_filter,
            alternative_casts,
            has_fuse,
            optional_costs,
            additional_cost,
            refers_to_ante,
        } = value;
        Self {
            card: card.into(),
            canonical_text,
            ability_labels,
            abilities,
            spell_effect,
            aura_attach_filter,
            alternative_casts,
            has_fuse,
            optional_costs,
            additional_cost,
            refers_to_ante,
        }
    }
}
impl From<NativeRetainedCardDefinition> for crate::cards::CardDefinition {
    fn from(value: NativeRetainedCardDefinition) -> Self {
        let NativeRetainedCardDefinition {
            card,
            canonical_text,
            ability_labels,
            abilities,
            spell_effect,
            aura_attach_filter,
            alternative_casts,
            has_fuse,
            optional_costs,
            additional_cost,
            refers_to_ante,
        } = value;
        Self {
            card: card.into(),
            canonical_text,
            ability_labels,
            abilities,
            spell_effect,
            aura_attach_filter,
            alternative_casts,
            has_fuse,
            optional_costs,
            additional_cost,
            refers_to_ante,
        }
    }
}
impl<I, A, P, M, K, C> RetainedCardDefinition<I, A, P, M, K, C> {
    pub fn try_map_payloads<J, B, Q, M2, K2, C2, E>(
        self,
        mut card_id: impl FnMut(I) -> Result<J, E>,
        mut ability: impl FnMut(A) -> Result<B, E>,
        mut program: impl FnMut(P) -> Result<Q, E>,
        mut alternative: impl FnMut(M) -> Result<M2, E>,
        mut optional: impl FnMut(K) -> Result<K2, E>,
        mut cost: impl FnMut(C) -> Result<C2, E>,
    ) -> Result<RetainedCardDefinition<J, B, Q, M2, K2, C2>, E> {
        let Self {
            card,
            canonical_text,
            ability_labels,
            abilities,
            spell_effect,
            aura_attach_filter,
            alternative_casts,
            has_fuse,
            optional_costs,
            additional_cost,
            refers_to_ante,
        } = self;
        Ok(RetainedCardDefinition {
            card: card.try_map_ids(&mut card_id)?,
            canonical_text,
            ability_labels,
            abilities: abilities
                .into_iter()
                .map(&mut ability)
                .collect::<Result<_, E>>()?,
            spell_effect: spell_effect.map(&mut program).transpose()?,
            aura_attach_filter,
            alternative_casts: alternative_casts
                .into_iter()
                .map(&mut alternative)
                .collect::<Result<_, E>>()?,
            has_fuse,
            optional_costs: optional_costs
                .into_iter()
                .map(&mut optional)
                .collect::<Result<_, E>>()?,
            additional_cost: cost(additional_cost)?,
            refers_to_ante,
        })
    }
}
pub type RetainedOccurrenceCardDefinition<I> = RetainedCardDefinition<
    I,
    RetainedOccurrenceAbility,
    RetainedOccurrenceProgram,
    RetainedOccurrenceAlternativeCast,
    RetainedOccurrenceOptionalCost,
    RetainedOccurrenceTotalCost,
>;
impl StaticAbilityOccurrenceEncoder {
    pub fn encode_card_definition<I: serde::Serialize>(
        &mut self,
        value: crate::cards::CardDefinition,
        card: impl FnMut(crate::ids::CardId) -> Result<I, OccurrenceBindingError>,
    ) -> Result<RetainedOccurrenceCardDefinition<I>, OccurrenceBindingError> {
        self.transaction(|table| {
            let table = std::cell::RefCell::new(table);
            let card = std::cell::RefCell::new(card);
            let result = NativeRetainedCardDefinition::from(value).try_map_payloads(
                |id| {
                    table
                        .borrow_mut()
                        .bind_graph_reference(id, &mut *card.borrow_mut())
                },
                |ability| {
                    let encoded = table.borrow_mut().encode_ability(ability)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |program| {
                    let encoded = table.borrow_mut().encode_program(program)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |method| {
                    let encoded = table.borrow_mut().encode_alternative_cast(method)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |optional| {
                    let encoded = table.borrow_mut().encode_optional_cost(optional)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
                |cost| {
                    let encoded = table.borrow_mut().encode_total_cost(cost)?;
                    table
                        .borrow_mut()
                        .bind_payload(encoded, &mut *card.borrow_mut())
                },
            )?;
            let mut table = table.borrow_mut();
            table.bind_shared_models(&mut *card.borrow_mut())?;
            Ok(result)
        })
    }
}
impl StaticAbilityOccurrenceDecoder {
    pub fn restore_card_definition<I>(
        &self,
        value: RetainedOccurrenceCardDefinition<I>,
        card: impl FnMut(I) -> Result<crate::ids::CardId, OccurrenceBindingError>,
    ) -> Result<crate::cards::CardDefinition, OccurrenceBindingError> {
        Ok(value
            .try_map_payloads(
                card,
                |ability| self.restore_ability(ability),
                |program| self.restore_program(program),
                |method| self.restore_alternative_cast(method),
                |optional| self.restore_optional_cost(optional),
                |cost| self.restore_total_cost(cost),
            )?
            .into())
    }
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
