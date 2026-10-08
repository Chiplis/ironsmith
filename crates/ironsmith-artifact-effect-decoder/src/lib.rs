//! Typed compiled-effect decoding, organized by runtime effect family.
//!
//! The families are modules of one crate rather than sibling crates so that
//! each serde instantiation they share (filters, values, conditions, ...) is
//! compiled once instead of once per family.

use std::any::Any;

use serde::de::DeserializeOwned;
use serde_json::Value;

mod combat;
mod composition_a_l;
mod composition_m_z;
mod permanent;
mod player;
mod resources;
mod stack_event;
mod zone_library;
#[cfg(test)]
mod counter_exile_permission_tests;

pub type ErasedPayload = Box<dyn Any + Send + Sync>;

fn decode_as<D>(payload: Value) -> Result<ErasedPayload, String>
where
    D: DeserializeOwned + Send + Sync + 'static,
{
    serde_json::from_value::<D>(payload)
        .map(|value| Box::new(value) as ErasedPayload)
        .map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectFamily {
    ZoneLibrary,
    Player,
    Resources,
    Permanent,
    Combat,
    StackEvent,
    CompositionAL,
    CompositionMZ,
}

pub fn family_for_kind(kind: &str) -> Option<EffectFamily> {
    match kind {
        "AdaptEffect" => Some(EffectFamily::CompositionAL),
        "AddManaEffect" => Some(EffectFamily::Resources),
        "AddManaFromCommanderColorIdentityEffect" => Some(EffectFamily::Resources),
        "AddManaOfAnyColorEffect" => Some(EffectFamily::Resources),
        "AddManaOfAnyOneColorEffect" => Some(EffectFamily::Resources),
        "AddManaOfChosenColorEffect" => Some(EffectFamily::Resources),
        "AddManaOfColorsAmongEffect" => Some(EffectFamily::Resources),
        "AddManaOfImprintedColorsEffect" => Some(EffectFamily::Resources),
        "AddManaOfLandProducedTypesEffect" => Some(EffectFamily::Resources),
        "AddOneManaOfAnyColorAmongEffect" => Some(EffectFamily::Resources),
        "AddScaledManaEffect" => Some(EffectFamily::Resources),
        "AdditionalLandPlaysEffect" => Some(EffectFamily::Player),
        "AdditionalPhasesEffect" => Some(EffectFamily::Player),
        "AmassEffect" => Some(EffectFamily::Permanent),
        "CollectEvidenceEffect" => Some(EffectFamily::ZoneLibrary),
        "EmpowerJaceEffect" => Some(EffectFamily::Permanent),
        "AmplifyEffect" => Some(EffectFamily::CompositionAL),
        "ApplyContinuousEffect" => Some(EffectFamily::Permanent),
        "AscendEffect" => Some(EffectFamily::Player),
        "AssignNoCombatDamageEffect" => Some(EffectFamily::Combat),
        "AttachObjectsEffect" => Some(EffectFamily::Permanent),
        "AttachToEffect" => Some(EffectFamily::Permanent),
        "AuraSwapEffect" => Some(EffectFamily::CompositionAL),
        "BackupEffect" => Some(EffectFamily::CompositionAL),
        "BecomeBasicLandTypeChoiceEffect" => Some(EffectFamily::Permanent),
        "BecomeColorChoiceEffect" => Some(EffectFamily::Permanent),
        "BecomeCreatureTypeChoiceEffect" => Some(EffectFamily::Permanent),
        "BecomeMonarchEffect" => Some(EffectFamily::Player),
        "BecomeSaddledUntilEotEffect" => Some(EffectFamily::Permanent),
        "BeholdEffect" => Some(EffectFamily::CompositionAL),
        "BidLifeEffect" => Some(EffectFamily::CompositionAL),
        "BolsterEffect" => Some(EffectFamily::CompositionAL),
        "CantEffect" => Some(EffectFamily::StackEvent),
        "CastSourceEffect" => Some(EffectFamily::Player),
        "CastTaggedEffect" => Some(EffectFamily::Player),
        "ChooseCardNameEffect" => Some(EffectFamily::Player),
        "ChooseCardTypeEffect" => Some(EffectFamily::Player),
        "ChangeTextEffect" => Some(EffectFamily::Permanent),
        "ChooseColorEffect" => Some(EffectFamily::Player),
        "ChooseCreatureTypeEffect" => Some(EffectFamily::Player),
        "ChooseLandTypeEffect" => Some(EffectFamily::Player),
        "ChooseModeEffect" => Some(EffectFamily::CompositionAL),
        "ChooseNamedOptionEffect" => Some(EffectFamily::Player),
        "RippleEffect" => Some(EffectFamily::Player),
        "ChooseNumberEffect" => Some(EffectFamily::Player),
        "ChooseNewTargetsEffect" => Some(EffectFamily::StackEvent),
        "ChooseObjectsEffect" => Some(EffectFamily::CompositionAL),
        "ChoosePlayerEffect" => Some(EffectFamily::Player),
        "ChooseSpellCastHistoryEffect" => Some(EffectFamily::CompositionAL),
        "CipherEffect" => Some(EffectFamily::CompositionAL),
        "ClashEffect" => Some(EffectFamily::ZoneLibrary),
        "ClearSuspectedEffect" => Some(EffectFamily::Permanent),
        "ConditionalEffect" => Some(EffectFamily::CompositionAL),
        "ConniveEffect" => Some(EffectFamily::ZoneLibrary),
        "ConspireCostEffect" => Some(EffectFamily::Permanent),
        "ConsultTopOfLibraryEffect" => Some(EffectFamily::ZoneLibrary),
        "ControlCombatChoicesThisTurnEffect" => Some(EffectFamily::Player),
        "ControlPlayerEffect" => Some(EffectFamily::Player),
        "ConvertEffect" => Some(EffectFamily::Permanent),
        "CopySpellEffect" => Some(EffectFamily::StackEvent),
        "CopySpellForEachTargetEffect" => Some(EffectFamily::StackEvent),
        "CounterEffect" => Some(EffectFamily::StackEvent),
        "CreateEmblemEffect" => Some(EffectFamily::Player),
        "CreateTokenCopyEffect" => Some(EffectFamily::Permanent),
        "CreateTokenEffect" => Some(EffectFamily::Permanent),
        "CrewCostEffect" => Some(EffectFamily::Permanent),
        "CumulativeUpkeepEffect" => Some(EffectFamily::CompositionAL),
        "DealDamageEffect" => Some(EffectFamily::Combat),
        "DealDamageToRecipientsEffect" => Some(EffectFamily::Combat),
        "DealDamageBySourcesEffect" => Some(EffectFamily::Combat),
        "DealDamageEachEffect" => Some(EffectFamily::Combat),
        "DealDistributedDamageEffect" => Some(EffectFamily::Combat),
        "DestroyEffect" => Some(EffectFamily::ZoneLibrary),
        "DestroyNoRegenerationEffect" => Some(EffectFamily::ZoneLibrary),
        "DetainEffect" => Some(EffectFamily::Permanent),
        "DevourEffect" => Some(EffectFamily::CompositionAL),
        "DirectionalAdjacentPlayerControlEffect" => Some(EffectFamily::Permanent),
        "DiscardEffect" => Some(EffectFamily::ZoneLibrary),
        "DiscardHandEffect" => Some(EffectFamily::ZoneLibrary),
        "DiscoverEffect" => Some(EffectFamily::Player),
        "DoubleCountersEffect" => Some(EffectFamily::Resources),
        "DoubleManaPoolEffect" => Some(EffectFamily::Resources),
        "DrawCardsEffect" => Some(EffectFamily::ZoneLibrary),
        "DrawForEachTaggedMatchingEffect" => Some(EffectFamily::ZoneLibrary),
        "EachPlayerScryEffect" => Some(EffectFamily::ZoneLibrary),
        "EarthbendEffect" => Some(EffectFamily::Permanent),
        "EmitGiftGivenEffect" => Some(EffectFamily::CompositionAL),
        "EmitKeywordActionEffect" => Some(EffectFamily::CompositionAL),
        "EmptyManaPoolEffect" => Some(EffectFamily::Resources),
        "EndCombatPhaseEffect" => Some(EffectFamily::Player),
        "EndTurnEffect" => Some(EffectFamily::Player),
        "EnergyCountersEffect" => Some(EffectFamily::Player),
        "EvolveEffect" => Some(EffectFamily::Permanent),
        "ExchangeControlEffect" => Some(EffectFamily::Permanent),
        "ExchangeLifeTotalsEffect" => Some(EffectFamily::Resources),
        "ExchangeTextBoxesEffect" => Some(EffectFamily::Permanent),
        "ExchangeValuesEffect" => Some(EffectFamily::Combat),
        "ExchangeZonesEffect" => Some(EffectFamily::ZoneLibrary),
        "ExecuteWithSourceEffect" => Some(EffectFamily::CompositionAL),
        "ExertCostEffect" => Some(EffectFamily::Permanent),
        "ExileEffect" => Some(EffectFamily::ZoneLibrary),
        "ExileInsteadOfGraveyardEffect" => Some(EffectFamily::Player),
        "ExileTaggedWhenSourceLeavesEffect" => Some(EffectFamily::StackEvent),
        "ExileTopOfLibraryEffect" => Some(EffectFamily::ZoneLibrary),
        "ExileUntilEffect" => Some(EffectFamily::ZoneLibrary),
        "ExperienceCountersEffect" => Some(EffectFamily::Player),
        "ExploreEffect" => Some(EffectFamily::CompositionAL),
        "ExtraTurnAfterNextTurnEffect" => Some(EffectFamily::Player),
        "ExtraTurnEffect" => Some(EffectFamily::Player),
        "FatesealEffect" => Some(EffectFamily::ZoneLibrary),
        "FightEffect" => Some(EffectFamily::Combat),
        "FlipCoinEffect" => Some(EffectFamily::Player),
        "FlipEffect" => Some(EffectFamily::Permanent),
        "ForEachControllerOfTaggedEffect" => Some(EffectFamily::CompositionAL),
        "ForEachCounterKindPutOrRemoveEffect" => Some(EffectFamily::Resources),
        "ForEachObject" => Some(EffectFamily::CompositionAL),
        "ForEachObjectCorrelatedResultEffect" => Some(EffectFamily::CompositionAL),
        "ForEachTaggedEffect" => Some(EffectFamily::CompositionAL),
        "ForEachTaggedPlayerEffect" => Some(EffectFamily::CompositionAL),
        "CollectManaPaymentsEffect" => Some(EffectFamily::CompositionAL),
        "ForPlayersEffect" => Some(EffectFamily::CompositionAL),
        "GivePlayerCountersEffect" => Some(EffectFamily::Player),
        "GainLifeEffect" => Some(EffectFamily::Resources),
        "GoadEffect" => Some(EffectFamily::Combat),
        "ClearGoadEffect" => Some(EffectFamily::Combat),
        "GrantAbilitiesTargetEffect" => Some(EffectFamily::Combat),
        "GrantBySpecEffect" => Some(EffectFamily::Player),
        "GrantEffect" => Some(EffectFamily::Player),
        "GrantNextSpellAbilityEffect" => Some(EffectFamily::Player),
        "GrantNextSpellCostReductionEffect" => Some(EffectFamily::Player),
        "GrantPlayTaggedEffect" => Some(EffectFamily::Player),
        "GrantRepeatableManaPaymentActionUntilEndOfTurnEffect" => Some(EffectFamily::CompositionAL),
        "GrantTaggedSpellFreeCastUntilEndOfTurnEffect" => Some(EffectFamily::Player),
        "GrantTaggedSpellLifeCostByManaValueEffect" => Some(EffectFamily::Player),
        "HauntExileEffect" => Some(EffectFamily::ZoneLibrary),
        "HealDamageEffect" => Some(EffectFamily::Combat),
        "IfEffect" => Some(EffectFamily::CompositionAL),
        "IncreaseSpeedEffect" => Some(EffectFamily::Player),
        "IncubateEffect" => Some(EffectFamily::Permanent),
        "InvestigateEffect" => Some(EffectFamily::Permanent),
        "LearnEffect" => Some(EffectFamily::ZoneLibrary),
        "LocalRewriteEffect" => Some(EffectFamily::CompositionAL),
        "LookAtHandEffect" => Some(EffectFamily::ZoneLibrary),
        "LookAtObjectsEffect" => Some(EffectFamily::ZoneLibrary),
        "LookAtTopCardsEffect" => Some(EffectFamily::ZoneLibrary),
        "LoseLifeEffect" => Some(EffectFamily::Resources),
        "LoseTheGameEffect" => Some(EffectFamily::Player),
        "ManaRestrictedEffect" => Some(EffectFamily::CompositionMZ),
        "ManaRetainedEffect" => Some(EffectFamily::CompositionMZ),
        "ManifestCardFromHandEffect" => Some(EffectFamily::CompositionMZ),
        "ManifestDreadEffect" => Some(EffectFamily::CompositionMZ),
        "ManifestObjectsEffect" => Some(EffectFamily::CompositionMZ),
        "ManifestTopCardOfLibraryEffect" => Some(EffectFamily::CompositionMZ),
        "MayCastMatchingSpellWithoutPayingManaCostEffect" => Some(EffectFamily::Player),
        "MayEffect" => Some(EffectFamily::CompositionMZ),
        "MayMoveToZoneEffect" => Some(EffectFamily::ZoneLibrary),
        "MeldEffect" => Some(EffectFamily::Permanent),
        "MillEffect" => Some(EffectFamily::ZoneLibrary),
        "ModifyPowerToughnessEffect" => Some(EffectFamily::Combat),
        "ModifyPowerToughnessForEachEffect" => Some(EffectFamily::Combat),
        "MonstrosityEffect" => Some(EffectFamily::Permanent),
        "MoveAllCountersEffect" => Some(EffectFamily::Resources),
        "MoveCountersEffect" => Some(EffectFamily::Resources),
        "MoveOneCounterEffect" => Some(EffectFamily::Resources),
        "MoveToLibraryNthFromTopEffect" => Some(EffectFamily::ZoneLibrary),
        "MoveToLibraryTopOrBottomChoiceEffect" => Some(EffectFamily::ZoneLibrary),
        "MoveToZoneEffect" => Some(EffectFamily::ZoneLibrary),
        "NinjutsuCostEffect" => Some(EffectFamily::Permanent),
        "NinjutsuEffect" => Some(EffectFamily::Permanent),
        "NoteLifeTotalEffect" => Some(EffectFamily::Resources),
        "OpenAttractionEffect" => Some(EffectFamily::CompositionMZ),
        "PayAnyEnergyEffect" => Some(EffectFamily::Player),
        "PayAnyLifeEffect" => Some(EffectFamily::Player),
        "PayEnergyEffect" => Some(EffectFamily::Player),
        "PayLifeEffect" => Some(EffectFamily::Resources),
        "PayManaEffect" => Some(EffectFamily::Resources),
        "PhaseInEffect" => Some(EffectFamily::Permanent),
        "PhaseOutEffect" => Some(EffectFamily::Permanent),
        "PlaySubgameEffect" => Some(EffectFamily::Player),
        "PoisonCountersEffect" => Some(EffectFamily::Player),
        "PopulateEffect" => Some(EffectFamily::CompositionMZ),
        "PrepareEffect" => Some(EffectFamily::Permanent),
        "PreventAllCombatDamageEffect" => Some(EffectFamily::Combat),
        "PreventAllDamageEffect" => Some(EffectFamily::Combat),
        "PreventAllDamageToTargetEffect" => Some(EffectFamily::Combat),
        "PreventDamageEffect" => Some(EffectFamily::Combat),
        "PreventNextTimeDamageEffect" => Some(EffectFamily::Combat),
        "ProliferateEffect" => Some(EffectFamily::Resources),
        "PutCounterOfChosenKindEffect" => Some(EffectFamily::Resources),
        "PutCountersEffect" => Some(EffectFamily::Resources),
        "PutOntoBattlefieldEffect" => Some(EffectFamily::ZoneLibrary),
        "PutStickerEffect" => Some(EffectFamily::Permanent),
        "PutTaggedRemainderOnLibraryBottomEffect" => Some(EffectFamily::ZoneLibrary),
        "RearrangeLookedCardsInLibraryEffect" => Some(EffectFamily::ZoneLibrary),
        "ReconfigureEffect" => Some(EffectFamily::Permanent),
        "RedirectAllDamageThisTurnToTargetEffect" => Some(EffectFamily::Combat),
        "RedirectNextDamageToTargetEffect" => Some(EffectFamily::Combat),
        "RedirectNextTimeDamageToSourceEffect" => Some(EffectFamily::Combat),
        "ReduceSpeedEffect" => Some(EffectFamily::Player),
        "ReflexiveTriggerEffect" => Some(EffectFamily::CompositionMZ),
        "RegenerateEffect" => Some(EffectFamily::Permanent),
        "RegisterDamagedBySourceZoneReplacementEffect" => Some(EffectFamily::StackEvent),
        "RegisterDrawReplacementEffect" => Some(EffectFamily::StackEvent),
        "RegisterEnterTappedReplacementEffect" => Some(EffectFamily::StackEvent),
        "RegisterEnterUnderControlReplacementEffect" => Some(EffectFamily::StackEvent),
        "RegisterFutureZoneReplacementEffect" => Some(EffectFamily::StackEvent),
        "RegisterManaReplacementEffect" => Some(EffectFamily::StackEvent),
        "RegisterManaRewriteEffect" => Some(EffectFamily::StackEvent),
        "RegisterManaSpendPermissionEffect" => Some(EffectFamily::StackEvent),
        "RegisterCounterPlacementReplacementEffect" => Some(EffectFamily::StackEvent),
        "RegisterDamageMultiplierEffect" => Some(EffectFamily::StackEvent),
        "RegisterDamageAdditionEffect" => Some(EffectFamily::StackEvent),
        "RegisterEnterWithCountersReplacementEffect" => Some(EffectFamily::StackEvent),
        "RegisterNextBatchEnterWithCountersEffect" => Some(EffectFamily::StackEvent),
        "RegisterZoneReplacementEffect" => Some(EffectFamily::StackEvent),
        "RemoveAnyCountersAmongEffect" => Some(EffectFamily::Resources),
        "RemoveAnyCountersFromSourceEffect" => Some(EffectFamily::Resources),
        "RemoveCountersEffect" => Some(EffectFamily::Resources),
        "BecomeBlockedEffect" => Some(EffectFamily::Combat),
        "RemoveFromCombatEffect" => Some(EffectFamily::Combat),
        "RemoveUpToAnyCountersEffect" => Some(EffectFamily::Resources),
        "RemoveUpToCountersEffect" => Some(EffectFamily::Resources),
        "RenownEffect" => Some(EffectFamily::Permanent),
        "ReorderGraveyardEffect" => Some(EffectFamily::ZoneLibrary),
        "ReorderLibraryTopEffect" => Some(EffectFamily::ZoneLibrary),
        "ReorderTopPlanarDeckEffect" => Some(EffectFamily::ZoneLibrary),
        "RepeatEffectsEffect" => Some(EffectFamily::CompositionMZ),
        "RepeatProcessEffect" => Some(EffectFamily::CompositionMZ),
        "RepeatProcessPromptEffect" => Some(EffectFamily::CompositionMZ),
        "ReplaceNextDamageToTargetEffect" => Some(EffectFamily::Combat),
        "RestartGameEffect" => Some(EffectFamily::Player),
        "RetainManaUntilEndOfTurnEffect" => Some(EffectFamily::Resources),
        "RetargetStackObjectEffect" => Some(EffectFamily::StackEvent),
        "ReturnAllToBattlefieldEffect" => Some(EffectFamily::ZoneLibrary),
        "ReturnFromGraveyardOrExileToBattlefieldEffect" => Some(EffectFamily::ZoneLibrary),
        "ReturnFromGraveyardToBattlefieldEffect" => Some(EffectFamily::ZoneLibrary),
        "ReturnFromGraveyardToHandEffect" => Some(EffectFamily::ZoneLibrary),
        "ReturnToHandEffect" => Some(EffectFamily::ZoneLibrary),
        "RevealFromHandEffect" => Some(EffectFamily::ZoneLibrary),
        "RevealSourceFromHandEffect" => Some(EffectFamily::ZoneLibrary),
        "RevealTaggedEffect" => Some(EffectFamily::ZoneLibrary),
        "RevealTopEffect" => Some(EffectFamily::ZoneLibrary),
        "ReverseTurnOrderEffect" => Some(EffectFamily::Player),
        "RingTemptsYouEffect" => Some(EffectFamily::Player),
        "RollDiceChooseResultEffect" => Some(EffectFamily::Player),
        "RollDieEffect" => Some(EffectFamily::Player),
        "SacrificeEffect" => Some(EffectFamily::ZoneLibrary),
        "SacrificePlayerEffect" => Some(EffectFamily::ZoneLibrary),
        "SacrificeTargetEffect" => Some(EffectFamily::ZoneLibrary),
        "ScheduleDelayedTriggerEffect" => Some(EffectFamily::StackEvent),
        "ScheduleEffectsWhenTaggedLeavesEffect" => Some(EffectFamily::StackEvent),
        "ScryEffect" => Some(EffectFamily::ZoneLibrary),
        "SearchLibraryEffect" => Some(EffectFamily::ZoneLibrary),
        "SearchLibrarySlotsEffect" => Some(EffectFamily::ZoneLibrary),
        "SecretChoiceEffect" => Some(EffectFamily::CompositionMZ),
        "SequenceEffect" => Some(EffectFamily::CompositionMZ),
        "SetBasePowerToughnessEffect" => Some(EffectFamily::Combat),
        "SetLifeTotalEffect" => Some(EffectFamily::Resources),
        "ShuffleGraveyardIntoLibraryEffect" => Some(EffectFamily::ZoneLibrary),
        "ShuffleHandAndGraveyardIntoLibraryEffect" => Some(EffectFamily::ZoneLibrary),
        "ShuffleLibraryEffect" => Some(EffectFamily::ZoneLibrary),
        "ShuffleObjectsIntoLibraryEffect" => Some(EffectFamily::ZoneLibrary),
        "SkipCombatPhasesEffect" => Some(EffectFamily::Player),
        "SkipCombatPhasesThisTurnEffect" => Some(EffectFamily::Player),
        "SkipDrawStepEffect" => Some(EffectFamily::Player),
        "SkipScheduledEffect" => Some(EffectFamily::Player),
        "SkipMainPhasesThisTurnEffect" => Some(EffectFamily::Player),
        "SkipNextCombatPhaseThisTurnEffect" => Some(EffectFamily::Player),
        "SkipTurnEffect" => Some(EffectFamily::Player),
        "SneakCostEffect" => Some(EffectFamily::Permanent),
        "SolveCaseEffect" => Some(EffectFamily::Permanent),
        "SetClassLevelEffect" => Some(EffectFamily::Permanent),
        "SoulbondPairEffect" => Some(EffectFamily::Permanent),
        "SupportEffect" => Some(EffectFamily::CompositionMZ),
        "SurveilEffect" => Some(EffectFamily::ZoneLibrary),
        "SuspectEffect" => Some(EffectFamily::Permanent),
        "TagAttachedToSourceEffect" => Some(EffectFamily::CompositionMZ),
        "TagMatchingObjectsEffect" => Some(EffectFamily::CompositionMZ),
        "TagOtherBlockParticipantEffect" => Some(EffectFamily::CompositionMZ),
        "TagTriggeringAttackerEffect" => Some(EffectFamily::CompositionMZ),
        "TagTriggeringBlockersEffect" => Some(EffectFamily::CompositionMZ),
        "TagTriggeringDamageTargetEffect" => Some(EffectFamily::CompositionMZ),
        "TagTriggeringObjectEffect" => Some(EffectFamily::CompositionMZ),
        "TagTriggeringSourceEffect" => Some(EffectFamily::CompositionMZ),
        "TaggedEffect" => Some(EffectFamily::CompositionMZ),
        "TakeInitiativeEffect" => Some(EffectFamily::Player),
        "TapEffect" => Some(EffectFamily::Permanent),
        "TargetOnlyEffect" => Some(EffectFamily::CompositionMZ),
        "TicketCountersEffect" => Some(EffectFamily::Player),
        "TransformEffect" => Some(EffectFamily::Permanent),
        "TurnFaceDownEffect" => Some(EffectFamily::Permanent),
        "TurnFaceUpEffect" => Some(EffectFamily::Permanent),
        "UnattachObjectsEffect" => Some(EffectFamily::Permanent),
        "UnearthEffect" => Some(EffectFamily::Permanent),
        "UnlessActionEffect" => Some(EffectFamily::CompositionMZ),
        "UnlessPaysEffect" => Some(EffectFamily::CompositionMZ),
        "UnlockRoomDoorEffect" => Some(EffectFamily::Permanent),
        "UntapEffect" => Some(EffectFamily::Permanent),
        "VariableCasualtyPlaneswalkerCopyEffect" => Some(EffectFamily::StackEvent),
        "VentureIntoDungeonEffect" => Some(EffectFamily::Player),
        "VillainousChoiceEffect" => Some(EffectFamily::CompositionMZ),
        "VoteEffect" => Some(EffectFamily::CompositionMZ),
        "WinTheGameEffect" => Some(EffectFamily::Player),
        "WithIdEffect" => Some(EffectFamily::CompositionMZ),
        "ImprintFromHandEffect" => Some(EffectFamily::ZoneLibrary),
        "ScaleXValueEffect" => Some(EffectFamily::StackEvent),
        "RevealChosenSubtypeEffect" => Some(EffectFamily::Player),
        "BecomePlottedEffect" => Some(EffectFamily::ZoneLibrary),
        "GrantEndThisEffectPaymentEffect" => Some(EffectFamily::Player),
        "SaddleCostEffect" => Some(EffectFamily::Permanent),
        "AddManaOfNotedTypeEffect" => Some(EffectFamily::Resources),
        "NoteActivationManaTypeEffect" => Some(EffectFamily::Resources),
        "MayCastForMiracleCostEffect" => Some(EffectFamily::Player),
        "ResolvesDespiteIllegalTargetsEffect" => Some(EffectFamily::CompositionMZ),
        _ => None,
    }
}

pub fn decode(kind: &str, payload: Value) -> Result<ErasedPayload, String> {
    let decoded = match family_for_kind(kind) {
        Some(family) => match family {
            EffectFamily::ZoneLibrary => zone_library::decode(kind, payload),
            EffectFamily::Player => player::decode(kind, payload),
            EffectFamily::Resources => resources::decode(kind, payload),
            EffectFamily::Permanent => permanent::decode(kind, payload),
            EffectFamily::Combat => combat::decode(kind, payload),
            EffectFamily::StackEvent => stack_event::decode(kind, payload),
            EffectFamily::CompositionAL => composition_a_l::decode(kind, payload),
            EffectFamily::CompositionMZ => composition_m_z::decode(kind, payload),
        },
        None => return Err(format!("unknown compiled effect payload kind: {kind}")),
    }?;
    decoded.ok_or_else(|| format!("unknown compiled effect payload kind: {kind}"))
}

#[cfg(test)]
mod tests {
    use super::{EffectFamily, family_for_kind};

    #[test]
    fn routes_representative_effects_to_domain_families() {
        assert_eq!(
            family_for_kind("MoveToZoneEffect"),
            Some(EffectFamily::ZoneLibrary)
        );
        assert_eq!(
            family_for_kind("ChoosePlayerEffect"),
            Some(EffectFamily::Player)
        );
        assert_eq!(
            family_for_kind("AddManaEffect"),
            Some(EffectFamily::Resources)
        );
        assert_eq!(
            family_for_kind("CreateTokenEffect"),
            Some(EffectFamily::Permanent)
        );
        assert_eq!(
            family_for_kind("DealDamageEffect"),
            Some(EffectFamily::Combat)
        );
        assert_eq!(
            family_for_kind("CopySpellEffect"),
            Some(EffectFamily::StackEvent)
        );
        assert_eq!(
            family_for_kind("ChooseModeEffect"),
            Some(EffectFamily::CompositionAL)
        );
        assert_eq!(
            family_for_kind("WithIdEffect"),
            Some(EffectFamily::CompositionMZ)
        );
        assert_eq!(family_for_kind("NotAnEffect"), None);
    }

    // UNRUN: source-only regression for the measured payload registration gap.
    #[test]
    fn source_counter_payload_decodes_and_normalizes_inside_owned_cost() {
        use ironsmith_compiled_artifact::WireEffect;
        use ironsmith_core::{Cost, CounterType, EffectId, RemoveAnyCountersFromSourceEffect, WithIdEffect};

        let kind = "RemoveAnyCountersFromSourceEffect";
        assert_eq!(family_for_kind(kind), Some(EffectFamily::Resources));
        for counter_type in [None, Some(CounterType::Charge), Some(CounterType::Named("eyeball".into()))] {
            for display_x in [false, true] {
                for remove_all in [false, true] {
                    let model = RemoveAnyCountersFromSourceEffect { counter_type, display_x, remove_all };
                    let payload = serde_json::to_value(&model).unwrap();
                    let decoded = super::decode(kind, payload.clone()).unwrap();
                    assert_eq!(decoded.downcast_ref::<RemoveAnyCountersFromSourceEffect>(), Some(&model));

                    let owned = WithIdEffect::new(EffectId(73), WireEffect::new(kind, payload));
                    let cost = Cost::Effect(WireEffect::new("WithIdEffect", serde_json::to_value(owned).unwrap()));
                    let mut bind = |_: u32| -> Result<u32, String> {
                        panic!("source counter payload has no authored card IDs")
                    };
                    let (normalized, opaque) = super::authored_definition_graph(&cost, &mut bind, &[]).unwrap();
                    assert!(!opaque);
                    assert_eq!(normalized, serde_json::to_value(&cost).unwrap());
                    assert_eq!(super::remap_card_ids(&cost, &mut bind).unwrap(), normalized);
                }
            }
        }
        assert!(super::decode(kind, serde_json::json!({
            "counter_type": null, "display_x": "true", "remove_all": false,
        })).is_err(), "registration must retain typed field validation");
    }
}

/// Remap typed card references throughout a canonical payload, including opaque
/// compiled effects. The callback owns the graph namespace and failure policy.
pub fn remap_card_ids<T: serde::Serialize>(
    value: &T,
    bind: &mut dyn FnMut(u32) -> Result<u32, String>,
) -> Result<Value, String> {
    let context = card_graph::Context {
        bind: std::cell::RefCell::new(bind),
        authored_definition: false,
        retained_definition: std::cell::Cell::new(false),
        generated_definitions: &[],
        normalized_definitions: std::cell::RefCell::new(std::collections::BTreeMap::new()),
    };
    value
        .serialize(card_graph::Serializer { context: &context })
        .map_err(|error| error.to_string())
}

/// Normalize an authored definition through the same typed graph owner as
/// artifact CardId remapping. Presentation fields are not identity inputs.
/// An opaque retained definition is reported separately: callers must not
/// claim that remapping CardIds canonicalized the bytes inside an older stamp.
pub fn authored_definition_graph<T: serde::Serialize>(
    value: &T,
    bind: &mut dyn FnMut(u32) -> Result<u32, String>,
    generated_definitions: &[ironsmith_core::LinkedExileDefinition],
) -> Result<(Value, bool), String> {
    let context = card_graph::Context {
        bind: std::cell::RefCell::new(bind),
        authored_definition: true,
        retained_definition: std::cell::Cell::new(false),
        generated_definitions,
        normalized_definitions: std::cell::RefCell::new(std::collections::BTreeMap::new()),
    };
    let normalized = value.serialize(card_graph::Serializer { context: &context })
        .map_err(|error| error.to_string())?;
    Ok((normalized, context.retained_definition.get()))
}

fn remap_effect_payload(
    kind: &str,
    payload: Value,
    context: &card_graph::Context<'_>,
) -> Result<Value, String> {
    let mapped = match family_for_kind(kind) {
        Some(EffectFamily::ZoneLibrary) => zone_library::map_card_ids(kind, payload, context),
        Some(EffectFamily::Player) => player::map_card_ids(kind, payload, context),
        Some(EffectFamily::Resources) => resources::map_card_ids(kind, payload, context),
        Some(EffectFamily::Permanent) => permanent::map_card_ids(kind, payload, context),
        Some(EffectFamily::Combat) => combat::map_card_ids(kind, payload, context),
        Some(EffectFamily::StackEvent) => stack_event::map_card_ids(kind, payload, context),
        Some(EffectFamily::CompositionAL) => composition_a_l::map_card_ids(kind, payload, context),
        Some(EffectFamily::CompositionMZ) => composition_m_z::map_card_ids(kind, payload, context),
        None => return Err(format!("unknown compiled effect payload kind: {kind}")),
    }?;
    mapped.ok_or_else(|| format!("unknown compiled effect payload kind: {kind}"))
}

mod card_graph {
    use serde::ser::{self, Serialize, SerializeMap as _, Serializer as _};
    use serde_json::{Value, value};
    use std::cell::RefCell;

    pub(super) struct Context<'b> {
        pub(super) bind: RefCell<&'b mut dyn FnMut(u32) -> Result<u32, String>>,
        pub(super) authored_definition: bool,
        pub(super) retained_definition: std::cell::Cell<bool>,
        pub(super) generated_definitions: &'b [ironsmith_core::LinkedExileDefinition],
        pub(super) normalized_definitions: RefCell<std::collections::BTreeMap<[u8; 32], [u8; 32]>>,
    }
    #[derive(Clone, Copy)]
    pub(super) struct Serializer<'a, 'b> {
        pub(super) context: &'a Context<'b>,
    }
    pub(super) fn map_payload_as<D: serde::de::DeserializeOwned + Serialize>(
        payload: Value,
        context: &Context<'_>,
    ) -> Result<Value, String> {
        let decoded: D = serde_json::from_value(payload).map_err(|error| error.to_string())?;
        decoded
            .serialize(Serializer { context })
            .map_err(|error| error.to_string())
    }
    pub(super) struct Compound<'a, 'b, S> {
        inner: S,
        context: &'a Context<'b>,
        name: Option<&'static str>,
    }
    impl<'a, 'b, S> Compound<'a, 'b, S> {
        fn new(inner: S, context: &'a Context<'b>) -> Self {
            Self {
                inner,
                context,
                name: None,
            }
        }
        fn mapped<T: Serialize + ?Sized>(&self, value: &T) -> Result<Value, serde_json::Error> {
            value.serialize(Serializer {
                context: self.context,
            })
        }
    }
    macro_rules! primitive {
        ($method:ident, $ty:ty) => {
            fn $method(self, value: $ty) -> Result<Value, Self::Error> {
                value::Serializer.$method(value)
            }
        };
    }
    impl<'a, 'b> ser::Serializer for Serializer<'a, 'b> {
        type Ok = Value;
        type Error = serde_json::Error;
        type SerializeSeq = Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeSeq>;
        type SerializeTuple =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeTuple>;
        type SerializeTupleStruct =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeTupleStruct>;
        type SerializeTupleVariant =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeTupleVariant>;
        type SerializeMap = Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeMap>;
        type SerializeStruct =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeStruct>;
        type SerializeStructVariant =
            Compound<'a, 'b, <value::Serializer as ser::Serializer>::SerializeStructVariant>;
        primitive!(serialize_bool, bool);
        primitive!(serialize_i8, i8);
        primitive!(serialize_i16, i16);
        primitive!(serialize_i32, i32);
        primitive!(serialize_i64, i64);
        primitive!(serialize_i128, i128);
        primitive!(serialize_u8, u8);
        primitive!(serialize_u16, u16);
        primitive!(serialize_u32, u32);
        primitive!(serialize_u64, u64);
        primitive!(serialize_u128, u128);
        primitive!(serialize_f32, f32);
        primitive!(serialize_f64, f64);
        primitive!(serialize_char, char);
        primitive!(serialize_str, &str);
        primitive!(serialize_bytes, &[u8]);
        fn serialize_none(self) -> Result<Value, Self::Error> {
            value::Serializer.serialize_none()
        }
        fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Value, Self::Error> {
            value.serialize(self)
        }
        fn serialize_unit(self) -> Result<Value, Self::Error> {
            value::Serializer.serialize_unit()
        }
        fn serialize_unit_struct(self, name: &'static str) -> Result<Value, Self::Error> {
            value::Serializer.serialize_unit_struct(name)
        }
        fn serialize_unit_variant(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
        ) -> Result<Value, Self::Error> {
            value::Serializer.serialize_unit_variant(name, index, variant)
        }
        fn serialize_newtype_struct<T: Serialize + ?Sized>(
            self,
            name: &'static str,
            value: &T,
        ) -> Result<Value, Self::Error> {
            if self.context.authored_definition && name == "LinkedExileDefinition" {
                let raw = value.serialize(value::Serializer)?;
                let bytes: [u8; 32] = serde_json::from_value(raw)?;
                if self.context.generated_definitions.contains(&ironsmith_core::LinkedExileDefinition(bytes)) {
                    let mut definitions = self.context.normalized_definitions.borrow_mut();
                    let ordinal = definitions.len() as u64;
                    let mapped = *definitions.entry(bytes).or_insert_with(|| {
                        let mut mapped = [0; 32];
                        mapped[24..].copy_from_slice(&ordinal.to_le_bytes());
                        mapped
                    });
                    return mapped.serialize(value::Serializer);
                }
                self.context.retained_definition.set(true);
            }
            if name == "CardId" {
                let raw = value.serialize(value::Serializer)?;
                let id = raw
                    .as_u64()
                    .and_then(|number| u32::try_from(number).ok())
                    .ok_or_else(|| <Self::Error as ser::Error>::custom("invalid typed CardId"))?;
                let bound = (self.context.bind.borrow_mut())(id)
                    .map_err(<Self::Error as ser::Error>::custom)?;
                return value::Serializer.serialize_u32(bound);
            }
            let mapped = value.serialize(self)?;
            value::Serializer.serialize_newtype_struct(name, &mapped)
        }
        fn serialize_newtype_variant<T: Serialize + ?Sized>(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Value, Self::Error> {
            let mapped = value.serialize(self)?;
            value::Serializer.serialize_newtype_variant(name, index, variant, &mapped)
        }
        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_seq(len)?,
                self.context,
            ))
        }
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_tuple(len)?,
                self.context,
            ))
        }
        fn serialize_tuple_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_tuple_struct(name, len)?,
                self.context,
            ))
        }
        fn serialize_tuple_variant(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_tuple_variant(name, index, variant, len)?,
                self.context,
            ))
        }
        fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_map(len)?,
                self.context,
            ))
        }
        fn serialize_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            let mut compound =
                Compound::new(value::Serializer.serialize_struct(name, len)?, self.context);
            compound.name = Some(name);
            Ok(compound)
        }
        fn serialize_struct_variant(
            self,
            name: &'static str,
            index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Ok(Compound::new(
                value::Serializer.serialize_struct_variant(name, index, variant, len)?,
                self.context,
            ))
        }
        fn collect_str<T: std::fmt::Display + ?Sized>(
            self,
            value: &T,
        ) -> Result<Value, Self::Error> {
            value::Serializer.collect_str(value)
        }
    }
    macro_rules! positional {
        ($trait:ident, $method:ident) => {
            impl<S> ser::$trait for Compound<'_, '_, S>
            where
                S: ser::$trait<Ok = Value, Error = serde_json::Error>,
            {
                type Ok = Value;
                type Error = serde_json::Error;
                fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
                    let mapped = self.mapped(value)?;
                    self.inner.$method(&mapped)
                }
                fn end(self) -> Result<Value, Self::Error> {
                    self.inner.end()
                }
            }
        };
    }
    positional!(SerializeSeq, serialize_element);
    positional!(SerializeTuple, serialize_element);
    positional!(SerializeTupleStruct, serialize_field);
    positional!(SerializeTupleVariant, serialize_field);
    impl<S> ser::SerializeMap for Compound<'_, '_, S>
    where
        S: ser::SerializeMap<Ok = Value, Error = serde_json::Error>,
    {
        type Ok = Value;
        type Error = serde_json::Error;
        fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
            let mapped = self.mapped(key)?;
            self.inner.serialize_key(&mapped)
        }
        fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
            let mapped = self.mapped(value)?;
            self.inner.serialize_value(&mapped)
        }
        fn end(self) -> Result<Value, Self::Error> {
            self.inner.end()
        }
    }
    impl<S> ser::SerializeStruct for Compound<'_, '_, S>
    where
        S: ser::SerializeStruct<Ok = Value, Error = serde_json::Error>,
    {
        type Ok = Value;
        type Error = serde_json::Error;
        fn serialize_field<T: Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            // These are declared serde struct fields, reached after exact
            // effect payload decoding. Arbitrary JSON keys and rule-bearing
            // names/strings never select this behavior.
            if self.context.authored_definition && matches!((self.name, key),
                (Some("CardDefinition"), "canonical_text" | "ability_labels")
                    | (Some("TriggeredAbility"), "presentation_label")
                    | (Some("Trigger" | "StaticAbility"), "label"))
            {
                return Ok(());
            }
            let mapped = self.mapped(value)?;
            self.inner.serialize_field(key, &mapped)
        }
        fn end(self) -> Result<Value, Self::Error> {
            let mut result = self.inner.end()?;
            if self.name == Some("CompiledEffect") {
                let kind = result["kind"]
                    .as_str()
                    .ok_or_else(|| {
                        <Self::Error as ser::Error>::custom("missing compiled effect kind")
                    })?
                    .to_owned();
                let payload = result
                    .get_mut("payload")
                    .ok_or_else(|| {
                        <Self::Error as ser::Error>::custom("missing compiled effect payload")
                    })?
                    .take();
                result["payload"] = super::remap_effect_payload(&kind, payload, self.context)
                    .map_err(<Self::Error as ser::Error>::custom)?;
            }
            Ok(result)
        }
    }
    impl<S> ser::SerializeStructVariant for Compound<'_, '_, S>
    where
        S: ser::SerializeStructVariant<Ok = Value, Error = serde_json::Error>,
    {
        type Ok = Value;
        type Error = serde_json::Error;
        fn serialize_field<T: Serialize + ?Sized>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            let mapped = self.mapped(value)?;
            self.inner.serialize_field(key, &mapped)
        }
        fn end(self) -> Result<Value, Self::Error> {
            self.inner.end()
        }
    }
}

#[cfg(test)]
mod card_graph_tests {
    use super::*;
    use ironsmith_compiled_artifact as wire;
    use ironsmith_core::{CardBuilder, CardId, ObjectId, StableId};

    fn nested_token() -> wire::WireEffect {
        let token: wire::WireCardDefinition = ironsmith_core::CardDefinition::new(
            CardBuilder::new(CardId::from_raw(9), "Graph token")
                .token()
                .other_face(CardId::from_raw(11))
                .build(),
        );
        let effect = wire::WireEffect::new(
            "CreateTokenEffect",
            serde_json::to_value(ironsmith_core::CreateTokenEffect::one(token)).unwrap(),
        );
        let tagged = ironsmith_core::TaggedEffect {
            tag: "created".into(),
            effect: Box::new(effect),
            outcome_only: false,
        };
        wire::WireEffect::new("TaggedEffect", serde_json::to_value(tagged).unwrap())
    }
    #[test]
    fn card_graph_typed_ids_preserve_other_namespaces_and_arbitrary_json() {
        #[derive(serde::Serialize)]
        struct Carrier {
            card: CardId,
            object: ObjectId,
            stable: StableId,
            arbitrary: Value,
        }
        let value = Carrier {
            card: CardId::from_raw(9),
            object: ObjectId::from_raw(9),
            stable: StableId::from_raw(9),
            arbitrary: serde_json::json!({"id":9,"CardId":9,"kind":"CardId"}),
        };
        let mut seen = Vec::new();
        let mapped = remap_card_ids(&value, &mut |id| {
            seen.push(id);
            Ok(id + 1000)
        })
        .unwrap();
        assert_eq!(seen, [9]);
        assert_eq!(mapped["card"], 1009);
        assert_eq!(mapped["object"], 9);
        assert_eq!(mapped["stable"], 9);
        assert_eq!(mapped["arbitrary"], value.arbitrary);
    }
    #[test]
    fn card_graph_typed_traversal_rebinds_nested_token_and_linked_face_once() {
        let original = nested_token();
        let mut seen = Vec::new();
        let mapped = remap_card_ids(&original, &mut |id| {
            seen.push(id);
            Ok(id + 1000)
        })
        .unwrap();
        assert_eq!(seen, [9, 11]);
        let receiver: wire::WireEffect = serde_json::from_value(mapped).unwrap();
        let tagged: ironsmith_core::TaggedEffect<wire::WireEffect> =
            serde_json::from_value(receiver.payload().clone()).unwrap();
        let token: ironsmith_core::CreateTokenEffect<wire::WireCardDefinition> =
            serde_json::from_value(tagged.effect.payload().clone()).unwrap();
        assert_eq!(token.token.card.id, CardId::from_raw(1009));
        assert_eq!(token.token.card.other_face, Some(CardId::from_raw(1011)));
        let roundtrip = remap_card_ids(&receiver, &mut |id| Ok(id - 1000)).unwrap();
        assert_eq!(roundtrip, serde_json::to_value(original).unwrap());
    }
    #[test]
    fn card_graph_typed_traversal_propagates_unknown_graph_and_nested_model_errors() {
        let mut seen = Vec::new();
        assert!(
            remap_card_ids(&nested_token(), &mut |id| {
                seen.push(id);
                if id == 11 {
                    Err("unbound linked template".into())
                } else {
                    Ok(1)
                }
            })
            .unwrap_err()
            .contains("unbound linked template")
        );
        assert_eq!(seen, [9, 11]);
        let unknown = wire::WireEffect::new("UnknownGraphEffect", serde_json::json!({}));
        assert!(
            remap_card_ids(&unknown, &mut |id| Ok(id))
                .unwrap_err()
                .contains("unknown compiled effect")
        );
    }

    #[test]
    fn authored_graph_tracks_only_typed_opaque_definitions_and_preserves_pair_aliases() {
        #[derive(serde::Serialize)]
        struct Carrier {
            pairs: Vec<ironsmith_core::LinkedExilePair>,
            arbitrary: Value,
        }
        let first = ironsmith_core::LinkedExileDefinition([17; 32]);
        let second = ironsmith_core::LinkedExileDefinition([29; 32]);
        let value = Carrier {
            pairs: vec![
                ironsmith_core::LinkedExilePair { definition: first, pair: 3 },
                ironsmith_core::LinkedExilePair { definition: first, pair: 7 },
                ironsmith_core::LinkedExilePair { definition: second, pair: 3 },
            ],
            arbitrary: serde_json::json!({"LinkedExileDefinition": [99], "label": "semantic arbitrary value"}),
        };
        let (_, opaque) = authored_definition_graph(&value, &mut |id| Ok(id), &[first]).unwrap();
        assert!(opaque, "one proven owner never canonicalizes an independent native identity");
        let (mapped, opaque) = authored_definition_graph(&value, &mut |id| Ok(id), &[first, second]).unwrap();
        assert!(!opaque);
        assert_eq!(mapped["pairs"][0]["definition"], mapped["pairs"][1]["definition"]);
        assert_ne!(mapped["pairs"][0]["definition"], mapped["pairs"][2]["definition"]);
        assert_eq!(mapped["pairs"][1]["pair"], 7);
        assert_eq!(mapped["arbitrary"], value.arbitrary);
        let ordinary = remap_card_ids(&value, &mut |id| Ok(id)).unwrap();
        assert_eq!(ordinary, serde_json::to_value(&value).unwrap(), "artifact remapping preserves retained identities");
    }

    #[test]
    fn authored_graph_omits_nested_presentation_without_mutating_transport_data() {
        let make = |text: &str| {
            let mut token: wire::WireCardDefinition = ironsmith_core::CardDefinition::new(
                CardBuilder::new(CardId::from_raw(9), "Token name is a characteristic").token().build());
            token.canonical_text = text.into();
            token.ability_labels = vec![text.into()];
            wire::WireEffect::new("CreateTokenEffect", serde_json::to_value(ironsmith_core::CreateTokenEffect::one(token)).unwrap())
        };
        let first = make("First rendering");
        let second = make("Second rendering");
        assert_ne!(serde_json::to_value(&first).unwrap(), serde_json::to_value(&second).unwrap());
        assert_eq!(authored_definition_graph(&first, &mut |id| Ok(id), &[]).unwrap(),
            authored_definition_graph(&second, &mut |id| Ok(id), &[]).unwrap());
    }
}
