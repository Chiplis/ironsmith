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
    Ability, AbilityKind, ActivatedAbility, ActivationTiming, LevelAbility, ManaUsageRestriction,
    ManaUsageSubtypeRequirement, ProtectionFrom, RestrictedManaUnit, TriggeredAbility,
};
pub use alternative_cast_model::{
    AlternativeCastRequirements, AlternativeCastingMethod, TrapCondition,
};
pub use anthem_model::{AnthemCountExpression, AnthemValue};
pub use attachment_model::AuraAttachmentFilter;
pub use card::{Card, CardBuilder, LinkedFaceLayout, PowerToughness, PtValue};
pub use cause_model::{CauseFilter, CauseType, CauseTypeFilter, ControllerFilter, EventCause};
pub use color::{Color, ColorSet};
pub use continuous_model::{
    CompiledContinuousEffectTarget, CompiledContinuousModification, CompiledPtSublayer,
};
pub use cost_model::{
    CoreCostComponent, Cost, CostComponent, DynamicManaCost, DynamicManaDisplayHint, OptionalCost,
    OptionalCostsPaid, TotalCost, TotalCostKind,
};
pub use counter::CounterType;
pub use definition_model::CardDefinition;
pub use effect::{
    AdaptEffect, AddManaEffect, AddManaFromCommanderColorIdentityEffect, AddManaOfAnyColorEffect,
    AddManaOfAnyOneColorEffect, AddManaOfChosenColorEffect, AddManaOfImprintedColorsEffect,
    AddManaOfLandProducedTypesEffect, AddScaledManaEffect, AdditionalLandPlaysEffect, AmassEffect,
    ApplyContinuousEffect, AttachObjectsEffect, AttachToEffect, BackupEffect,
    BattlefieldController, BecomeBasicLandTypeChoiceEffect, BecomeColorChoiceEffect,
    BecomeCreatureTypeChoiceEffect, BecomeMonarchEffect, BeholdEffect, BolsterEffect, CantEffect,
    CastSourceEffect, CastTaggedEffect, ChoiceCount, ChooseCardNameEffect, ChooseCardTypeEffect,
    ChooseColorEffect, ChooseCreatureTypeEffect, ChooseModeEffect, ChooseNamedOptionEffect,
    ChooseNewTargetsEffect, ChooseObjectsEffect, ChoosePlayerEffect, ChooseSpellCastHistoryEffect,
    CipherEffect, ClashEffect, ClashOpponentMode, CombatDamagePreventionTarget, ConditionalEffect,
    ConniveEffect, ConspireCostEffect, ConsultTopOfLibraryEffect, ConsultTopOfLibraryStopRule,
    ControlCombatChoicesThisTurnEffect, ControlPlayerEffect, ConvertEffect, CopyAttackTargetMode,
    CopyPtAdjustment, CopySpellEffect, CopySpellForEachTargetEffect, CounterEffect,
    CreateEmblemEffect, CreateTokenCopyEffect, CreateTokenEffect, CrewCostEffect,
    CumulativeUpkeepEffect, DamageFilter, DealDamageEffect, DealDistributedDamageEffect,
    DelayedTriggerSpec, DestroyEffect, DestroyNoRegenerationEffect, DetainEffect, DiscardEffect,
    DiscardHandEffect, DiscoverEffect, DoubleManaPoolEffect, DrawCardsEffect,
    DrawForEachTaggedMatchingEffect, EachPlayerScryEffect, EarthbendEffect, EffectId, EffectMode,
    EffectPredicate, EmitGiftGivenEffect, EmitKeywordActionEffect, EmptyManaPoolEffect,
    EnergyCountersEffect, EvolveEffect, ExchangeControlEffect, ExchangeLifeTotalsEffect,
    ExchangeTextBoxesEffect, ExchangeValueOperand, ExchangeValuesEffect, ExchangeZonesEffect,
    ExecuteWithSourceEffect, ExertCostEffect, ExileEffect, ExileInsteadOfGraveyardEffect,
    ExileTaggedWhenSourceLeavesEffect, ExileTopOfLibraryEffect, ExileUntilDuration,
    ExileUntilEffect, ExploreEffect, ExtraTurnAfterNextTurnEffect, ExtraTurnEffect, FatesealEffect,
    FightEffect, FlipCoinEffect, FlipEffect, ForEachControllerOfTaggedEffect,
    ForEachCounterKindPutOrRemoveEffect, ForEachObject, ForEachTaggedEffect,
    ForEachTaggedPlayerEffect, ForPlayersEffect, GainLifeEffect, GoadEffect,
    GrantAbilitiesTargetEffect, GrantBySpecEffect, GrantEffect, GrantNextSpellAbilityEffect,
    GrantNextSpellCostReductionEffect, GrantPlayTaggedDuration, GrantPlayTaggedEffect,
    GrantTaggedSpellFreeCastUntilEndOfTurnEffect, GrantTaggedSpellLifeCostByManaValueEffect,
    HauntExileEffect, IfEffect, IncreaseSpeedEffect, IncubateEffect, InvestigateEffect,
    LibraryBottomOrder, LibraryConsultMode, LocalRewriteEffect, LookAtHandEffect,
    LookAtTopCardsEffect, LoseLifeEffect, LoseTheGameEffect, ManifestDreadEffect,
    ManifestTopCardOfLibraryEffect, MayEffect, MayMoveToZoneEffect, MeldEffect, MillEffect,
    ModifyPowerToughnessEffect, ModifyPowerToughnessForEachEffect, MonstrosityEffect,
    MoveAllCountersEffect, MoveCountersEffect, MoveToLibraryNthFromTopEffect, MoveToZoneEffect,
    NewTargetRestriction, NinjutsuCostEffect, NinjutsuEffect, OpenAttractionEffect,
    PayAnyEnergyEffect, PayEnergyEffect, PayManaEffect, PhaseInEffect, PhaseOutEffect,
    PlayerControlDuration, PlayerControlStart, PoisonCountersEffect, PopulateEffect,
    PreventAllCombatDamageEffect, PreventAllDamageEffect, PreventAllDamageToTargetEffect,
    PreventDamageEffect, PreventNextTimeDamageEffect, PreventNextTimeDamageSource,
    PreventNextTimeDamageTarget, PreventionTarget, ProliferateEffect, PutCountersEffect,
    PutOntoBattlefieldEffect, PutStickerEffect, PutTaggedRemainderOnLibraryBottomEffect,
    RearrangeLookedCardsInLibraryEffect, RedirectNextDamageToTargetEffect,
    RedirectNextTimeDamageSource, RedirectNextTimeDamageToSourceEffect, ReduceSpeedEffect,
    ReflexiveTriggerEffect, RegenerateEffect, RegisterZoneReplacementEffect,
    RemoveAnyCountersAmongEffect, RemoveAnyCountersFromSourceEffect, RemoveCountersEffect,
    RemoveFromCombatEffect, RemoveUpToAnyCountersEffect, RemoveUpToCountersEffect, RenownEffect,
    ReorderGraveyardEffect, ReorderLibraryTopEffect, RepeatEffectsEffect, RepeatProcessEffect,
    RepeatProcessPromptEffect, ReplacementApplyMode, RetainManaUntilEndOfTurnEffect, RetargetMode,
    RetargetStackObjectEffect, ReturnAllToBattlefieldEffect,
    ReturnFromGraveyardToBattlefieldEffect, ReturnFromGraveyardToHandEffect, ReturnToHandEffect,
    RevealTaggedEffect, RevealTopEffect, RingTemptsYouEffect, RollDieEffect, SacrificeEffect,
    SacrificePlayerEffect, SacrificeTargetEffect, ScheduleDelayedTriggerEffect,
    ScheduleEffectsWhenTaggedLeavesEffect, ScryEffect, SearchLibraryEffect, SearchLibrarySlot,
    SearchLibrarySlotsEffect, SearchSelectionMode, SequenceEffect, SetBasePowerToughnessEffect,
    SetLifeTotalEffect, SharedTypeConstraint, ShuffleGraveyardIntoLibraryEffect,
    ShuffleHandAndGraveyardIntoLibraryEffect, ShuffleLibraryEffect,
    ShuffleObjectsIntoLibraryEffect, SkipCombatPhasesEffect, SkipDrawStepEffect,
    SkipNextCombatPhaseThisTurnEffect, SkipTurnEffect, SoulbondPairEffect, SupportEffect,
    SurveilEffect, TagAttachedToSourceEffect, TagMatchingObjectsEffect,
    TagTriggeringDamageTargetEffect, TagTriggeringObjectEffect, TaggedEffect,
    TaggedLeavesAbilitySource, TakeInitiativeEffect, TapEffect, TargetOnlyEffect,
    TicketCountersEffect, TransformEffect, UnearthEffect, UnlessActionEffect, UnlessPaysEffect,
    UntapEffect, Until, VentureIntoDungeonEffect, VoteChoice, VoteEffect, VoteOption,
    WinTheGameEffect, WithIdEffect,
};
pub use effect_model::{Comparison, EventValueSpec, ValueComparisonOperator};
pub use event_model::KeywordActionKind;
pub use filter_model::{
    AlternativeCastKind, Comparison as FilterComparison, CounterConstraint, ObjectFilter,
    ObjectRef, ParityRequirement, PlayerFilter, PtReference, SourcePowerRelation, StackObjectKind,
    TaggedObjectConstraint, TaggedOpbjectRelation, TargetabilityConstraint,
};
pub use grant_model::{
    DerivedAlternativeCast, GrantDuration, GrantSpec, GrantStaticAbility, Grantable,
};
pub use ids::{
    CardId, IdCountersSnapshot, ObjectId, PlayerId, StableId, reset_runtime_id_counters,
    restore_id_counters, snapshot_id_counters,
};
pub use mana::{ManaCost, ManaSymbol};
pub use resolution_model::{ResolutionProgram, ResolutionSegment, SelfReplacementBranch};
pub use spell_cost_condition_model::ThisSpellCostCondition;
pub use spell_timing_model::ThisSpellCastTiming;
pub use static_ability_id::StaticAbilityId;
pub use static_ability_model::{
    ActivatedAbilityCostCondition, Anthem, AttachedAbilityGrant, AttachedChosenLandwalkGrant,
    AttackCostCondition, AttackingGroupAttackCondition, CantAttackUnlessConditionSpec,
    ConditionalSpellKeywordKind, ConditionalSpellKeywordSpec, CopyActivatedAbilities,
    CopyTriggeredAbilities, CostIncrease, CostIncreaseManaCost, CostReduction,
    CostReductionManaCost, DefendingPlayerAttackCondition, EnterAsCopyAsEntersSpec, GrantAbility,
    GrantObjectAbilityForFilter, GraveyardCountMetric, LandwalkKind, PregameActionKind,
    PregameBeginOnBattlefieldSpec, RemoveCardTypesForFilter, SetColorsForFilter, StaticAbility,
    StaticAbilityPayload, ThisSpellCastRestrictionKind, ThisSpellCostReduction,
    ThisSpellCostReductionManaCost,
};
pub use tag::{SOURCE_EXILED_TAG, TagKey};
pub use target_model::{ChooseSpec, ChooseSpecSurfaceHint, SourceReferenceSurface};
pub use trigger_model::{
    CountMode as CompilerTriggerCountMode, CounterPutOnTrigger as CompilerCounterPutOnTrigger,
    DamagedBySource, Trigger as CompilerTrigger, TriggerKind,
    ZoneChangeTrigger as CompilerZoneChangeTrigger,
};
pub use types::{CardType, Subtype, SubtypeFamily, Supertype};
pub use value_model::{
    Condition, EffectMetric, EffectMetricSource, ManaSpendPermission, ManaSpendScope, Restriction,
    Value, ValueSurfaceHint,
};
pub use zone::Zone;
