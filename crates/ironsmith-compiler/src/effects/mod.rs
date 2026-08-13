use crate::effect::Effect;
pub use ironsmith_core::{
    AdaptEffect, AddManaEffect, AddManaFromCommanderColorIdentityEffect, AddManaOfAnyColorEffect,
    AddManaOfAnyOneColorEffect, AddManaOfLandProducedTypesEffect, AdditionalLandPlaysEffect,
    AdditionalPhase, AdditionalPhasesEffect, AmassEffect, AmplifyEffect,
    AssignNoCombatDamageEffect, AttachObjectsEffect, AttachToEffect, AuraSwapEffect, BackupEffect,
    BattlefieldController, BecomeBasicLandTypeChoiceEffect, BecomeColorChoiceEffect,
    BecomeCreatureTypeChoiceEffect, BecomeMonarchEffect, BecomeSaddledUntilEotEffect, BeholdEffect,
    BidLifeEffect as CoreBidLifeEffect, BolsterEffect, CantEffect, CastSourceEffect,
    CastTaggedEffect, ChooseCardNameEffect, ChooseCardTypeEffect, ChooseColorEffect,
    ChooseCreatureTypeEffect, ChooseLandTypeEffect, ChooseModeEffect as CoreChooseModeEffect,
    ChooseNamedOptionEffect, ChooseNewTargetsEffect, ChooseObjectsEffect, ChoosePlayerEffect,
    ChooseSpellCastHistoryEffect, CipherEffect, ClashEffect, ClearSuspectedEffect,
    CombatDamagePreventionTarget, ConditionalEffect as CoreConditionalEffect, ConniveEffect,
    ConspireCostEffect, ConsultTopOfLibraryEffect, ConsultTopOfLibraryStopRule,
    ControlCombatChoicesThisTurnEffect, ControlPlayerEffect, ConvertEffect, CopySpellEffect,
    CopySpellForEachTargetEffect, CounterEffect, CreateEmblemEffect as CoreCreateEmblemEffect,
    CreateTokenEffect as CoreCreateTokenEffect, CrewCostEffect,
    CumulativeUpkeepEffect as CoreCumulativeUpkeepEffect, DealDamageEffect,
    DealDistributedDamageEffect, DelayedTriggerSpec, DestroyEffect, DestroyNoRegenerationEffect,
    DetainEffect, DevourEffect, DirectionalAdjacentPlayerControlEffect, DiscardEffect,
    DiscardHandEffect, DiscoverEffect, DoubleCountersEffect, DoubleManaPoolEffect, DrawCardsEffect,
    DrawForEachTaggedMatchingEffect, EachPlayerScryEffect, EarthbendEffect, EmitGiftGivenEffect,
    EmitKeywordActionEffect, EmptyManaPoolEffect, EndCombatPhaseEffect, EndTurnEffect,
    EnergyCountersEffect, EvolveEffect, ExchangeControlEffect, ExchangeLifeTotalsEffect,
    ExchangeTextBoxesEffect, ExchangeValueOperand, ExchangeValuesEffect, ExchangeZonesEffect,
    ExecuteWithSourceEffect as CoreExecuteWithSourceEffect, ExertCostEffect, ExileEffect,
    ExileInsteadOfGraveyardEffect, ExileTaggedWhenSourceLeavesEffect, ExileTopOfLibraryEffect,
    ExileUntilDuration, ExileUntilEffect, ExperienceCountersEffect, ExploreEffect,
    ExtraTurnAfterNextTurnEffect, ExtraTurnEffect, FatesealEffect, FightEffect, FlipCoinEffect,
    FlipEffect, ForEachControllerOfTaggedEffect, ForEachCounterKindPutOrRemoveEffect,
    ForEachObject as CoreForEachObject,
    ForEachObjectCorrelatedResultEffect as CoreForEachObjectCorrelatedResultEffect,
    ForEachTaggedEffect, ForEachTaggedPlayerEffect, ForPlayersEffect, GainLifeEffect, GoadEffect,
    GrantAbilitiesTargetEffect as CoreGrantAbilitiesTargetEffect,
    GrantBySpecEffect as CoreGrantBySpecEffect, GrantEffect as CoreGrantEffect,
    GrantNextSpellCostReductionEffect, GrantPlayTaggedDuration, GrantPlayTaggedEffect,
    GrantRepeatableManaPaymentActionUntilEndOfTurnEffect as CoreGrantRepeatableManaPaymentActionUntilEndOfTurnEffect,
    GrantTaggedSpellFreeCastUntilEndOfTurnEffect, GrantTaggedSpellLifeCostByManaValueEffect,
    HauntExileEffect as CoreHauntExileEffect, HealDamageEffect, IfEffect as CoreIfEffect,
    IncubateEffect, InvestigateEffect, LearnEffect, LibraryBottomOrder, LibraryConsultMode,
    LibraryPlacementOrder, LifeBidStart, LocalRewriteEffect as CoreLocalRewriteEffect,
    LookAtHandEffect, LookAtObjectsEffect, LookAtTopCardsEffect, LoseLifeEffect, LoseTheGameEffect,
    ManaRestrictedEffect as CoreManaRestrictedEffect, ManaRetainedEffect as CoreManaRetainedEffect,
    ManaRetentionDuration, ManaTypeSource, ManifestCardFromHandEffect, ManifestDreadEffect,
    ManifestObjectsEffect, ManifestTopCardOfLibraryEffect,
    MayCastMatchingSpellWithoutPayingManaCostEffect, MayEffect, MayMoveToZoneEffect, MeldEffect,
    MillEffect, ModifyPowerToughnessEffect, ModifyPowerToughnessForEachEffect, MonstrosityEffect,
    MoveAllCountersEffect, MoveCountersEffect, MoveOneCounterEffect, MoveToLibraryNthFromTopEffect,
    MoveToLibraryTopOrBottomChoiceEffect, MoveToZoneAttackTargetMode, MoveToZoneEffect,
    NewTargetRestriction, NinjutsuCostEffect, NinjutsuEffect, NoteLifeTotalEffect,
    OpenAttractionEffect, PayAnyEnergyEffect, PayAnyLifeEffect, PayEnergyEffect, PayLifeEffect,
    PayManaEffect, PhaseInEffect, PhaseOutDuration, PhaseOutEffect, PlaySubgameEffect,
    PoisonCountersEffect, PopulateEffect, PreventAllCombatDamageEffect, PreventAllDamageEffect,
    PreventAllDamageToTargetEffect as CorePreventAllDamageToTargetEffect,
    PreventDamageEffect as CorePreventDamageEffect, PreventNextTimeDamageEffect,
    PreventNextTimeDamageSource, PreventNextTimeDamageTarget, ProliferateEffect,
    PutCounterOfChosenKindEffect, PutCountersEffect, PutOntoBattlefieldEffect, PutStickerEffect,
    PutTaggedRemainderOnLibraryBottomEffect, RearrangeLookedCardsInLibraryEffect,
    ReconfigureEffect, RedirectAllDamageThisTurnToTargetEffect, RedirectNextDamageToTargetEffect,
    RedirectNextTimeDamageDestination, RedirectNextTimeDamageSource,
    RedirectNextTimeDamageToSourceEffect, ReflexiveTriggerEffect as CoreReflexiveTriggerEffect,
    RegenerateEffect as CoreRegenerateEffect, RegisterDamagedBySourceZoneReplacementEffect,
    RegisterDrawReplacementEffect, RegisterEnterTappedReplacementEffect,
    RegisterEnterUnderControlReplacementEffect, RegisterFutureZoneReplacementEffect,
    RegisterManaReplacementEffect, RegisterNextBatchEnterWithCountersEffect,
    RegisterZoneReplacementEffect, RemoveAnyCountersAmongEffect, RemoveAnyCountersFromSourceEffect,
    RemoveCountersEffect, RemoveFromCombatEffect, RemoveUpToAnyCountersEffect,
    RemoveUpToCountersEffect, RenownEffect, ReorderGraveyardEffect, ReorderLibraryTopEffect,
    ReorderTopPlanarDeckEffect, RepeatProcessPromptEffect,
    ReplaceNextDamageToTargetEffect as CoreReplaceNextDamageToTargetEffect, ReplacementApplyMode,
    RestartGameEffect, RetainManaUntilEndOfTurnEffect, RetargetMode, RetargetStackObjectEffect,
    ReturnAllToBattlefieldEffect, ReturnAsAuraOptions,
    ReturnFromGraveyardOrExileToBattlefieldEffect, ReturnFromGraveyardToBattlefieldEffect,
    ReturnFromGraveyardToHandEffect, ReturnToHandEffect, RevealFromHandEffect,
    RevealSourceFromHandDuration, RevealSourceFromHandEffect, RevealTaggedEffect, RevealTopEffect,
    ReverseTurnOrderEffect, RingTemptsYouEffect, RollDiceChooseResultEffect, RollDieEffect,
    SacrificeEffect, SacrificePlayerEffect, SacrificeTargetEffect,
    ScheduleEffectsWhenTaggedLeavesEffect as CoreScheduleEffectsWhenTaggedLeavesEffect, ScryEffect,
    SearchLibraryEffect as CoreSearchLibraryEffect, SearchLibrarySlot,
    SearchLibrarySlotsEffect as CoreSearchLibrarySlotsEffect, SecretChoiceEffect,
    SecretObjectChoice, SequenceEffect as CoreSequenceEffect, SetBasePowerToughnessEffect,
    SetLifeTotalEffect, SharedTypeConstraint, ShuffleGraveyardIntoLibraryEffect,
    ShuffleHandAndGraveyardIntoLibraryEffect, ShuffleLibraryEffect,
    ShuffleObjectsIntoLibraryEffect, SkipCombatPhasesEffect, SkipCombatPhasesThisTurnEffect,
    SkipDrawStepEffect, SkipMainPhasesThisTurnEffect, SkipNextCombatPhaseThisTurnEffect,
    SkipTurnEffect, SneakCostEffect, SolveCaseEffect, SoulbondPairEffect, SupportEffect,
    SurveilEffect, SuspectEffect, TagAttachedToSourceEffect, TagMatchingObjectsEffect,
    TagOtherBlockParticipantEffect, TagTriggeringAttackerEffect, TagTriggeringBlockersEffect,
    TagTriggeringDamageTargetEffect, TagTriggeringObjectEffect, TagTriggeringSourceEffect,
    TaggedEffect as CoreTaggedEffect, TaggedLeavesAbilitySource, TakeInitiativeEffect, TapEffect,
    TargetOnlyEffect, TicketCountersEffect, TransformEffect, UnattachObjectsEffect, UnearthEffect,
    UnlessActionEffect, UnlessPaysEffect, UnlockRoomDoorEffect, UntapEffect,
    VariableCasualtyPlaneswalkerCopyEffect, VentureIntoDungeonEffect,
    VillainousChoiceEffect as CoreVillainousChoiceEffect, WinTheGameEffect,
    WithIdEffect as CoreWithIdEffect,
};

pub type ChooseModeEffect = CoreChooseModeEffect<Effect>;
pub type CreateEmblemEffect = CoreCreateEmblemEffect<crate::effect::EmblemDescription>;
pub type CreateTokenEffect = CoreCreateTokenEffect<crate::cards::CardDefinition>;
pub type ConditionalEffect = CoreConditionalEffect<Effect>;
pub type CumulativeUpkeepEffect = CoreCumulativeUpkeepEffect<Effect>;
pub type ExecuteWithSourceEffect = CoreExecuteWithSourceEffect<Effect>;
pub type ForEachObject = CoreForEachObject<Effect>;
pub type ForEachObjectCorrelatedResultEffect = CoreForEachObjectCorrelatedResultEffect<Effect>;
pub type HauntExileEffect = CoreHauntExileEffect<Effect>;
pub type IfEffect = CoreIfEffect<Effect>;
pub type LocalRewriteEffect = CoreLocalRewriteEffect<Effect>;
pub type ManaRestrictedEffect = CoreManaRestrictedEffect<Effect>;
pub type ManaRetainedEffect = CoreManaRetainedEffect<Effect>;
pub type PreventDamageEffect = CorePreventDamageEffect<Effect>;
pub type PreventAllDamageToTargetEffect = CorePreventAllDamageToTargetEffect<Effect>;
pub type ReplaceNextDamageToTargetEffect = CoreReplaceNextDamageToTargetEffect<Effect>;
pub type RegenerateEffect = CoreRegenerateEffect<Effect>;
pub type ScheduleEffectsWhenTaggedLeavesEffect = CoreScheduleEffectsWhenTaggedLeavesEffect<Effect>;
pub type SequenceEffect = CoreSequenceEffect<Effect>;
pub type WithIdEffect = CoreWithIdEffect<Effect>;
pub type TaggedEffect = CoreTaggedEffect<Effect>;
pub type ReflexiveTriggerEffect = CoreReflexiveTriggerEffect<Effect>;
pub type RepeatEffectsEffect = ironsmith_core::RepeatEffectsEffect<Effect>;
pub type RepeatProcessEffect = ironsmith_core::RepeatProcessEffect<Effect>;
pub type GrantRepeatableManaPaymentActionUntilEndOfTurnEffect =
    CoreGrantRepeatableManaPaymentActionUntilEndOfTurnEffect<Effect>;
pub type BidLifeEffect = CoreBidLifeEffect<Effect>;
pub type VoteChoice = ironsmith_core::VoteChoice<Effect>;
pub type VoteEffect = ironsmith_core::VoteEffect<Effect>;
pub type VillainousChoiceEffect = CoreVillainousChoiceEffect<Effect>;
pub type GrantEffect = CoreGrantEffect<crate::grant::Grantable, crate::grant::GrantDuration>;
pub type GrantBySpecEffect =
    CoreGrantBySpecEffect<crate::grant::GrantSpec, crate::grant::GrantDuration>;

pub const VOTE_WINNERS_TAG: &str = "__vote_winners__";
pub const VOTED_OBJECTS_TAG: &str = "__voted_objects__";

pub type SearchLibraryEffect = CoreSearchLibraryEffect;
pub type SearchLibrarySlotsEffect = CoreSearchLibrarySlotsEffect;

pub type CopyPtAdjustment = ironsmith_core::CopyPtAdjustment;
pub type CopyAttackTargetMode = ironsmith_core::CopyAttackTargetMode;
pub type CreateTokenCopyEffect =
    ironsmith_core::CreateTokenCopyEffect<crate::static_abilities::StaticAbility>;
pub type ScheduleDelayedTriggerEffect = ironsmith_core::ScheduleDelayedTriggerEffect<Effect>;

pub type GrantAbilitiesTargetEffect =
    CoreGrantAbilitiesTargetEffect<crate::static_abilities::StaticAbility>;
pub type ApplyContinuousEffect = ironsmith_core::ApplyContinuousEffect<
    crate::continuous::EffectTarget,
    crate::continuous::Modification,
    continuous::RuntimeModification,
    crate::ConditionExpr,
>;

pub type GrantNextSpellAbilityEffect =
    ironsmith_core::GrantNextSpellAbilityEffect<crate::ability::Ability>;

#[derive(Debug, Clone, PartialEq)]
pub struct ScaleXValueEffect {
    pub target: crate::target::ChooseSpec,
    pub multiplier: u32,
}

impl ScaleXValueEffect {
    pub fn new(target: crate::target::ChooseSpec, multiplier: u32) -> Self {
        Self { target, multiplier }
    }
}

pub mod cards {
    #[derive(Debug, Clone, PartialEq)]
    pub struct ImprintFromHandEffect {
        pub filter: crate::target::ObjectFilter,
    }

    impl ImprintFromHandEffect {
        pub fn new(filter: crate::target::ObjectFilter) -> Self {
            Self { filter }
        }
    }
}

pub mod continuous {
    #[derive(Debug, Clone, PartialEq)]
    pub enum RuntimeModification {
        ModifyPowerToughness {
            power: crate::effect::Value,
            toughness: crate::effect::Value,
        },
        ChangeControllerToEffectController,
        ChangeControllerToPlayer(crate::target::PlayerFilter),
        CopyOf {
            source: crate::target::ChooseSpec,
            preserve_source_abilities: bool,
            name_override: Option<String>,
            name_override_surface: Option<crate::target::SourceReferenceSurface>,
            add_supertypes: Vec<crate::types::Supertype>,
            copy_exception_surface: Option<String>,
        },
        RemoveAllAbilities,
        RemoveThisAbility,
        SetAuraAttachmentFilter(crate::AuraAttachmentFilter),
    }
}

pub mod composition {
    pub type VoteOption = ironsmith_core::VoteOption<crate::effect::Effect>;
}

pub mod consult_helpers {
    pub use ironsmith_core::{LibraryBottomOrder, LibraryConsultMode};
}

pub mod mana {
    pub use ironsmith_core::{
        AddManaOfChosenColorEffect, AddManaOfColorsAmongEffect, AddManaOfImprintedColorsEffect,
        AddOneManaOfAnyColorAmongEffect, AddScaledManaEffect,
    };
}
