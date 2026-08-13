//! Shared domain-model crate for the workspace refactor.
//!
//! Shared domain-model crate for the workspace refactor.
//!
//! Low-level card model and value types live here so runtime, compiler, and
//! registry layers can share them without introducing forbidden dependency
//! edges.

pub mod ability_model;
pub mod alternative_cast_model;
pub mod anthem_model;
pub mod attachment_model;
pub mod card;
pub mod cardinal;
pub mod cause_model;
pub mod color;
pub mod continuous_model;
pub mod cost_model;
pub mod counter;
pub mod definition_model;
pub mod effect;
pub mod effect_model;
pub mod event_model;
pub mod filter_model;
pub mod grant_model;
pub mod ids;
pub mod mana;
pub mod ordinal;
pub mod resolution_model;
pub mod spell_cost_condition_model;
pub mod spell_timing_model;
pub mod static_ability_id;
pub mod static_ability_model;
pub mod tag;
pub mod target_model;
pub mod trigger_model;
pub mod types;
pub mod value_model;
pub mod zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceSplitMarker;

pub use ability_model::{
    Ability, AbilityKind, ActivatedAbility, ActivatedPresentationLabel, ActivationTiming,
    LevelAbility, ManaPaymentPredicate, ManaPaymentPurpose, ManaSpendAbilityGrantDuration,
    ManaSpendBonusCondition, ManaSpendGrantedKeyword, ManaSpendPayload, ManaUsageRestriction,
    ManaUsageSubtypeRequirement, PresentationKeyword, PresentationLabel, ProtectionFrom,
    RestrictedManaUnit, TriggeredAbility,
};
pub use alternative_cast_model::{
    AlternativeCastRequirements, AlternativeCastingMethod, TrapCondition,
};
pub use anthem_model::{AnthemCountExpression, AnthemValue, SourceCounterPronounSurface};
pub use attachment_model::AuraAttachmentFilter;
pub use card::{Card, CardBuilder, LinkedFaceLayout, PowerToughness, PtValue};
pub use cardinal::{cardinal_word, parse_cardinal_word, parse_cardinal_words};
pub use cause_model::{CauseFilter, CauseType, CauseTypeFilter, ControllerFilter, EventCause};
pub use color::{Color, ColorSet};
pub use continuous_model::{
    CompiledContinuousEffectTarget, CompiledContinuousModification, CompiledPtSublayer,
};
pub use cost_model::{
    AlternativeCostReference, AlternativeCostReferenceSurface, CoreCostComponent, Cost,
    CostComponent, DynamicManaCost, DynamicManaDisplayHint, OptionalCost, OptionalCostKind,
    OptionalCostRef, OptionalCostsPaid, TotalCost, TotalCostKind,
};
pub use counter::CounterType;
pub use definition_model::CardDefinition;
pub use effect::{
    AdaptEffect, AddManaEffect, AddManaFromCommanderColorIdentityEffect, AddManaOfAnyColorEffect,
    AddManaOfAnyOneColorEffect, AddManaOfChosenColorEffect, AddManaOfColorsAmongEffect,
    AddManaOfImprintedColorsEffect, AddManaOfLandProducedTypesEffect,
    AddOneManaOfAnyColorAmongEffect, AddScaledManaEffect, AdditionalLandPlaysEffect,
    AdditionalPhase, AdditionalPhasesEffect, AmassEffect, AmplifyEffect, AnimationDurationSurface,
    AnimationPtSurface, ApplyContinuousEffect, AscendEffect, AssignNoCombatDamageEffect,
    AttachObjectsEffect, AttachToEffect, AuraSwapEffect, BackupEffect, BattlefieldController,
    BattlefieldEntryCounterSpec, BattlefieldEntryCounterSurface, BecomeBasicLandTypeChoiceEffect,
    BecomeColorChoiceEffect, BecomeCreatureTypeChoiceEffect, BecomeForetoldEffect,
    BecomeMonarchEffect, BecomeSaddledUntilEotEffect, BeholdEffect, BidLifeEffect, BolsterEffect,
    CantEffect, CastSourceEffect, CastTaggedEffect, ChoiceAggregateConstraint,
    ChoiceAggregateMetric, ChoiceCount, ChooseCardNameEffect, ChooseCardTypeEffect,
    ChooseColorEffect, ChooseCreatureTypeEffect, ChooseLandTypeEffect, ChooseModeEffect,
    ChooseNamedOptionEffect, ChooseNewTargetsEffect, ChooseObjectsEffect, ChoosePlayerEffect,
    ChooseSpellCastHistoryEffect, CipherEffect, ClashEffect, ClashOpponentMode,
    ClearSuspectedEffect, CoinFace, CoinFlipKind, CombatDamagePreventionTarget, ConditionalEffect,
    ConditionalModeRange, ConditionalSurface, ConniveEffect, ConspireCostEffect,
    ConsultTopOfLibraryEffect, ConsultTopOfLibraryStopRule, ContinuousDurationObject,
    ContinuousDurationPlayer, ContinuousDurationPredicate, ControlCombatChoicesThisTurnEffect,
    ControlPlayerEffect, ConvertEffect, CopyAttackTargetMode, CopyPtAdjustment, CopySpellEffect,
    CopySpellForEachTargetEffect, CounterEffect, CreateEmblemEffect, CreateTokenCopyEffect,
    CreateTokenEffect, CrewCostEffect, CumulativeUpkeepEffect, DamageDistributionMode,
    DamageFilter, DealDamageEffect, DealDistributedDamageEffect, DelayedTriggerDuration,
    DelayedTriggerPrepayment, DelayedTriggerSpec, DestinationPlayerReferenceSurface, DestroyEffect,
    DestroyNoRegenerationEffect, DetainEffect, DevourEffect,
    DirectionalAdjacentPlayerControlEffect, DiscardEffect, DiscardHandEffect, DiscoverEffect,
    DoubleCountersEffect, DoubleManaPoolEffect, DrawCardsEffect, DrawForEachTaggedMatchingEffect,
    DrawTheGameEffect, EachPlayerScryEffect, EarthbendEffect, EffectId, EffectMode,
    EffectPredicate, EmitGiftGivenEffect, EmitKeywordActionEffect, EmitKeywordActionObjectTag,
    EmptyManaPoolEffect, EndCombatPhaseEffect, EndTurnEffect, EnergyCountersEffect, EvolveEffect,
    ExchangeControlEffect, ExchangeLifeTotalsEffect, ExchangeTextBoxesEffect, ExchangeValueOperand,
    ExchangeValuesEffect, ExchangeZonesEffect, ExecuteWithSourceEffect, ExertCostEffect,
    ExileEffect, ExileInsteadOfGraveyardEffect, ExileTaggedWhenSourceLeavesEffect,
    ExileTopLibrarySurface, ExileTopOfLibraryEffect, ExileUntilDuration, ExileUntilEffect,
    ExiledWithSourceDestinationSurface, ExiledWithSourceMoveSurface,
    ExiledWithSourceMoveVerbSurface, ExiledWithSourceReferenceSurface,
    ExiledWithSourceSubjectSurface, ExperienceCountersEffect, ExploreEffect,
    ExtraTurnAfterNextTurnEffect, ExtraTurnEffect, FatesealEffect, FightEffect, FlipCoinEffect,
    FlipEffect, ForEachControllerOfTaggedEffect, ForEachCounterKindPutOrRemoveEffect,
    ForEachObject, ForEachObjectCorrelatedResultEffect, ForEachTaggedEffect,
    ForEachTaggedPlayerEffect, ForPlayersEffect, GainLifeEffect, GoadEffect,
    GrantAbilitiesTargetEffect, GrantBySpecEffect, GrantEffect, GrantNextSpellAbilityEffect,
    GrantNextSpellCostReductionEffect, GrantPlayTaggedDuration, GrantPlayTaggedEffect,
    GrantPlayTaggedManaReferenceSurface, GrantPlayTaggedObjectSurface, GrantPlayTaggedSurface,
    GrantRepeatableManaPaymentActionUntilEndOfTurnEffect,
    GrantTaggedSpellFreeCastUntilEndOfTurnEffect, GrantTaggedSpellLifeCostByManaValueEffect,
    HauntExileEffect, HealDamageEffect, IfEffect, IncreaseSpeedEffect, IncubateEffect,
    InvestigateEffect, LearnEffect, LibraryBottomOrder, LibraryConsultMode, LibraryPlacementOrder,
    LibraryRemainderSurface, LifeBidStart, LinkedExileFollowUp, LocalRewriteEffect,
    LookAtHandEffect, LookAtObjectsEffect, LookAtTopCardsEffect, LoseLifeEffect, LoseTheGameEffect,
    ManaRestrictedEffect, ManaRetainedEffect, ManaRetentionDuration, ManaTypeSource,
    ManifestCardFromHandEffect, ManifestDreadEffect, ManifestObjectsEffect,
    ManifestTopCardOfLibraryEffect, MayCastMatchingSpellPayment,
    MayCastMatchingSpellWithoutPayingManaCostEffect, MayEffect, MayMoveToZoneEffect, MeldEffect,
    MillEffect, ModifyPowerToughnessEffect, ModifyPowerToughnessForEachEffect, MonstrosityEffect,
    MoveAllCountersEffect, MoveCountersEffect, MoveOneCounterEffect, MoveToLibraryNthFromTopEffect,
    MoveToLibraryTopOrBottomChoiceEffect, MoveToZoneAttackTargetMode, MoveToZoneEffect,
    MoveToZoneVerbSurface, NewTargetRestriction, NinjutsuCostEffect, NinjutsuEffect,
    NoteLifeTotalEffect, OpenAttractionEffect, PayAnyEnergyEffect, PayAnyLifeEffect,
    PayEnergyEffect, PayLifeEffect, PayManaEffect, PhaseInEffect, PhaseOutDuration, PhaseOutEffect,
    PlaySubgameEffect, PlayerControlDuration, PlayerControlStart, PoisonCountersEffect,
    PopulateEffect, PreventAllCombatDamageEffect, PreventAllDamageEffect,
    PreventAllDamageToTargetEffect, PreventDamageEffect, PreventNextTimeDamageEffect,
    PreventNextTimeDamageSource, PreventNextTimeDamageTarget, PreventionTarget,
    PriorEffectResultActor, PriorEffectResultQuantifier, PriorEffectResultSurface,
    ProliferateEffect, PutCounterOfChosenKindEffect, PutCountersEffect, PutOntoBattlefieldEffect,
    PutStickerEffect, PutTaggedRemainderOnLibraryBottomEffect, RearrangeLookedCardsInLibraryEffect,
    ReconfigureEffect, RedirectAllDamageThisTurnToTargetEffect, RedirectNextDamageDestination,
    RedirectNextDamageToTargetEffect, RedirectNextTimeDamageDestination,
    RedirectNextTimeDamageSource, RedirectNextTimeDamageToSourceEffect, ReduceSpeedEffect,
    ReflexiveTriggerEffect, RegenerateEffect, RegisterDamagedBySourceZoneReplacementEffect,
    RegisterDrawReplacementEffect, RegisterEnterTappedReplacementEffect,
    RegisterEnterUnderControlReplacementEffect, RegisterFutureZoneReplacementEffect,
    RegisterManaReplacementEffect, RegisterNextBatchEnterWithCountersEffect,
    RegisterZoneReplacementEffect, RemoveAnyCountersAmongEffect, RemoveAnyCountersFromSourceEffect,
    RemoveCountersEffect, RemoveFromCombatEffect, RemoveUpToAnyCountersEffect,
    RemoveUpToCountersEffect, RenownEffect, ReorderGraveyardEffect, ReorderLibraryTopEffect,
    ReorderTopPlanarDeckEffect, RepeatEffectsEffect, RepeatProcessEffect,
    RepeatProcessPromptEffect, RepeatProcessPromptKind, ReplaceNextDamageToTargetEffect,
    ReplacementApplyMode, RestartGameEffect, RestrictionDurationSurface, RestrictionStart,
    RetainManaUntilEndOfTurnEffect, RetargetMode, RetargetStackObjectEffect,
    ReturnAllToBattlefieldEffect, ReturnAsAuraOptions,
    ReturnFromGraveyardOrExileToBattlefieldEffect, ReturnFromGraveyardToBattlefieldEffect,
    ReturnFromGraveyardToHandEffect, ReturnToHandEffect, RevealFromHandEffect,
    RevealSourceFromHandDuration, RevealSourceFromHandEffect, RevealTaggedEffect, RevealTopEffect,
    ReverseTurnOrderEffect, RingTemptsYouEffect, RollDiceChooseResultEffect, RollDieEffect,
    SacrificeEffect, SacrificePlayerEffect, SacrificeTargetEffect, ScheduleDelayedTriggerEffect,
    ScheduleEffectsWhenTaggedLeavesEffect, ScryEffect, SearchLibraryEffect, SearchLibrarySlot,
    SearchLibrarySlotsEffect, SearchResultReferenceSurface, SearchSelectionMode,
    SecretChoiceEffect, SecretObjectChoice, SequenceEffect, SequenceSurface,
    SetBasePowerToughnessEffect, SetLifeTotalEffect, SetQuantifierSurface, SharedTypeConstraint,
    ShuffleGraveyardIntoLibraryEffect, ShuffleHandAndGraveyardIntoLibraryEffect,
    ShuffleLibraryEffect, ShuffleObjectsIntoLibraryEffect, SkipCombatPhasesEffect,
    SkipCombatPhasesThisTurnEffect, SkipDrawStepEffect, SkipMainPhasesThisTurnEffect,
    SkipNextCombatPhaseThisTurnEffect, SkipTurnEffect, SneakCostEffect, SolveCaseEffect,
    SoulbondPairEffect, SupportEffect, SurveilEffect, SuspectEffect, TagAttachedToSourceEffect,
    TagMatchingObjectsEffect, TagOtherBlockParticipantEffect, TagTriggeringAttackerEffect,
    TagTriggeringBlockersEffect, TagTriggeringDamageTargetEffect, TagTriggeringObjectEffect,
    TagTriggeringSourceEffect, TaggedEffect, TaggedLeavesAbilitySource, TakeInitiativeEffect,
    TapEffect, TargetOnlyEffect, TicketCountersEffect, TokenAbilityPresentation,
    TokenCopyReferenceSurface, TransformEffect, TurnFaceUpEffect, TypeRetentionSurface,
    UnattachObjectsEffect, UnearthEffect, UnlessActionEffect, UnlessPaysEffect,
    UnlockRoomDoorEffect, UntapEffect, Until, VariableCasualtyPlaneswalkerCopyEffect,
    VentureIntoDungeonEffect, VillainousChoiceEffect, VoteChoice, VoteEffect, VoteOption,
    WinTheGameEffect, WithIdEffect, ZoneReplacementLibraryPlacement,
};
pub use effect_model::{Comparison, EventValueSpec, ValueComparisonOperator};
pub use event_model::KeywordActionKind;
pub use filter_model::{
    AdditionalCostObjectAction, AdditionalCostObjectSurface, AlternativeCastKind,
    ChosenNameSourceSurface, Comparison as FilterComparison, CounterConstraint,
    CountersPutOnThisTurnConstraint, DemonstrativeAntecedentSurface, ExcludedNameSurface,
    GlobalCharacteristicDomainSurface, GraveyardEntryHistorySurface, ObjectCharacteristic,
    ObjectCharacteristicRelation, ObjectCharacteristicRelationKind, ObjectFilter,
    ObjectFilterUnionConnective, ObjectFilterUnionSurface, ObjectRef, ParityRequirement,
    PlayedByOpponentSurface, PlayerFilter, PowerToughnessRelation, PtReference,
    SameNameAntecedentSurface, SourcePowerRelation, StackObjectKind, TaggedObjectConstraint,
    TaggedOpbjectRelation, TargetabilityConstraint,
};
pub use grant_model::{
    DerivedAlternativeCast, GrantDuration, GrantSpec, GrantStaticAbility, GrantUsageLimit,
    Grantable, SourceExiledGrantSurface,
};
pub use ids::{
    CardId, IdCountersSnapshot, ObjectId, PlayerId, StableId, reset_runtime_id_counters,
    restore_id_counters, snapshot_id_counters,
};
pub use mana::{ManaCost, ManaSymbol};
pub use ordinal::{ordinal_word, parse_ordinal_word, parse_ordinal_words};
pub use resolution_model::{ResolutionProgram, ResolutionSegment, SelfReplacementBranch};
pub use spell_cost_condition_model::ThisSpellCostCondition;
pub use spell_timing_model::ThisSpellCastTiming;
pub use static_ability_id::StaticAbilityId;
pub use static_ability_model::{
    AbilityLossMode, ActivatedAbilityCostCondition, AdditionalTokenKind, Anthem,
    AnthemReplacementSurface, AttachedAbilityGrant, AttachedChosenLandwalkGrant,
    AttackCostCondition, AttackingGroupAttackCondition, CantAttackUnlessConditionSpec,
    CompanionDeckCardFacts, CompanionDeckCondition, ConditionalSpellKeywordKind,
    ConditionalSpellKeywordSpec, CopyActivatedAbilities, CopyStaticAbilityVariants,
    CopyTriggeredAbilities, CostIncrease, CostIncreaseManaCost, CostReduction,
    CostReductionCharacteristicIntersection, CostReductionManaCost, CounterRemovalFollowUp,
    CounterRemovalPreventionSurface, DefendingPlayerAttackCondition, EnterAsCopyAsEntersSpec,
    EnterAsCopyLinkedExilePairSpec, EscalateSpec, GrantAbility, GrantObjectAbilityForFilter,
    GraveyardCountMetric, LandwalkKind, OptionalLifeAdditionalCost, PowerToughnessChoiceOption,
    PregameActionKind, PregameBeginOnBattlefieldSpec, PregameRevealFromOpeningHandSpec,
    PreventAllDamageToSelfFromSourcesMatchingSpec, RemoveCardTypesForFilter, SetColorsForFilter,
    SpliceQuality, SpliceSpec, StaticAbility, StaticAbilityPayload, StaticAbilityVariantSelector,
    StaticDamageSourceRelation, ThisSpellCastRestrictionKind, ThisSpellCostReduction,
    ThisSpellCostReductionManaCost,
};
pub use tag::{
    ATTACKING_GROUP_TAG, CAST_CONTROLLED_OBJECTS_TAG, CAST_MODIFIED_CREATURES_TAG,
    CHOSEN_OBJECTS_TAG, COMBAT_DAMAGE_GROUP_TAG, EXPLOITED_TAG, EXPLOITER_TAG,
    INITIATIVE_HOLDER_TAG, MANA_PAID_OBJECT_TAG, MANA_SOURCES_SPENT_TO_CAST_TAG,
    MANIFEST_DREAD_GRAVEYARD_TAG, PREVIOUS_ITERATED_OBJECTS_TAG, PRIOR_EXILED_CARD_TAG,
    REVEALED_THIS_WAY_TAG, SOURCE_EXILED_TAG, SOURCE_OBJECT_TAG, TagKey, ZONE_CHANGE_GROUP_TAG,
};
pub use target_model::{
    ChooseSpec, ChooseSpecSurfaceHint, SacrificedObjectKind, SourceReferenceSurface,
};
pub use trigger_model::{
    AttackTargetRestriction, ClashWinTriggerSurface, CountMode as CompilerTriggerCountMode,
    CounterPutOnTrigger as CompilerCounterPutOnTrigger, DamagedBySource, GraveyardTriggerSurface,
    Trigger as CompilerTrigger, TriggerIntroSurface as CompilerTriggerIntroSurface, TriggerKind,
    TriggerTimingRestriction, ZoneChangeTrigger as CompilerZoneChangeTrigger,
};
pub use types::{CardType, Subtype, SubtypeFamily, Supertype};
pub use value_model::{
    AttachmentConditionHost, Condition, DeathHistoryControllerSurface, EffectMetric,
    EffectMetricSource, ManaSpendPermission, ManaSpendScope, ManaSpentCastReferenceSurface,
    PermanentLeftBattlefieldControlSurface, PriorEffectAction, PriorEffectMetricQuery, Restriction,
    SourceCounterThresholdSurface, TaggedObjectMatchMode, TurnHistoryCondition, TurnHistoryCount,
    Value, ValueSurfaceHint,
};
pub use zone::Zone;
