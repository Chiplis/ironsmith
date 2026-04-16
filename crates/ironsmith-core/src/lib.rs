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
    Ability, AbilityKind, ActivatedAbility, ActivationTiming, CoreCostComponent, LevelAbility,
    ManaUsageRestriction, ManaUsageSubtypeRequirement, ProtectionFrom, RestrictedManaUnit,
    TriggeredAbility,
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
pub use cost_model::{CostComponent, OptionalCost, OptionalCostsPaid, TotalCost};
pub use counter::CounterType;
pub use definition_model::CardDefinition;
pub use effect::{
    AddManaOfChosenColorEffect, AddManaOfImprintedColorsEffect, AddScaledManaEffect, AmassEffect,
    ApplyContinuousEffect, BattlefieldController, BecomeBasicLandTypeChoiceEffect,
    BecomeColorChoiceEffect, BecomeCreatureTypeChoiceEffect, ChoiceCount, ChooseModeEffect,
    ChooseObjectsEffect, ChoosePlayerEffect, ChooseSpellCastHistoryEffect, ClashEffect,
    ClashOpponentMode, ConditionalEffect, ConsultTopOfLibraryStopRule, CopyAttackTargetMode,
    CopyPtAdjustment, CopySpellEffect, CounterEffect, CreateTokenCopyEffect, CreateTokenEffect,
    CrewCostEffect, DealDamageEffect, DealDistributedDamageEffect, DelayedTriggerSpec,
    DestroyEffect, DestroyNoRegenerationEffect, DiscardEffect, DrawCardsEffect,
    DrawForEachTaggedMatchingEffect, EachPlayerScryEffect, EarthbendEffect, EffectId, EffectMode,
    EffectPredicate, ExchangeControlEffect, ExchangeValueOperand, ExecuteWithSourceEffect,
    ExertCostEffect, ExileEffect, ExileTaggedWhenSourceLeavesEffect, ExileTopOfLibraryEffect,
    ExileUntilDuration, ExileUntilEffect, ForEachControllerOfTaggedEffect,
    ForEachCounterKindPutOrRemoveEffect, ForEachObject, ForEachTaggedEffect,
    ForEachTaggedPlayerEffect, ForPlayersEffect, GrantAbilitiesTargetEffect,
    GrantNextSpellAbilityEffect, GrantNextSpellCostReductionEffect, GrantPlayTaggedDuration,
    GrantPlayTaggedEffect, GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
    GrantTaggedSpellLifeCostByManaValueEffect, HauntExileEffect, IfEffect, InvestigateEffect,
    LocalRewriteEffect, LookAtHandEffect, LoseLifeEffect, MayEffect, MeldEffect, MillEffect,
    ModifyPowerToughnessEffect, MoveToLibraryNthFromTopEffect, MoveToZoneEffect,
    NewTargetRestriction, PayEnergyEffect, PayManaEffect, PhaseOutEffect, PlaceholderEffect,
    PopulateEffect, PreventAllDamageToTargetEffect, PreventDamageEffect,
    PreventNextTimeDamageEffect, PreventNextTimeDamageSource, PreventNextTimeDamageTarget,
    PutCountersEffect, RedirectNextDamageToTargetEffect, RedirectNextTimeDamageSource,
    RedirectNextTimeDamageToSourceEffect, ReflexiveTriggerEffect, RegisterZoneReplacementEffect,
    RemoveAnyCountersFromSourceEffect, RemoveCountersEffect, RemoveFromCombatEffect,
    ReorderLibraryTopEffect, RepeatProcessPromptEffect, ReplacementApplyMode,
    RetainManaUntilEndOfTurnEffect, RetargetMode, RetargetStackObjectEffect,
    ReturnAllToBattlefieldEffect, ReturnToHandEffect, RevealTaggedEffect, SacrificeEffect,
    SacrificeTargetEffect, ScheduleDelayedTriggerEffect, ScheduleEffectsWhenTaggedLeavesEffect,
    ScryEffect, SearchLibraryEffect, SearchLibrarySlot, SearchLibrarySlotsEffect,
    SearchSelectionMode, SequenceEffect, SharedTypeConstraint, TagMatchingObjectsEffect,
    TaggedEffect, TaggedLeavesAbilitySource, TapEffect, TargetOnlyEffect, UnlessActionEffect,
    UnlessPaysEffect, UntapEffect, Until, VoteChoice, VoteEffect, VoteOption, WithIdEffect,
};
pub use effect_model::{Comparison, EventValueSpec, ValueComparisonOperator};
pub use event_model::KeywordActionKind;
pub use filter_model::{
    AlternativeCastKind, Comparison as FilterComparison, CounterConstraint, ObjectFilter,
    ObjectRef, ParityRequirement, PlayerFilter, PtReference, SourcePowerRelation, StackObjectKind,
    TaggedObjectConstraint, TaggedOpbjectRelation,
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
    ConditionalSpellKeywordKind, ConditionalSpellKeywordSpec, GraveyardCountMetric,
    PregameActionKind, PregameBeginOnBattlefieldSpec,
};
pub use tag::{SOURCE_EXILED_TAG, TagKey};
pub use target_model::ChooseSpec;
pub use trigger_model::{
    CountMode as CompilerTriggerCountMode, CounterPutOnTrigger as CompilerCounterPutOnTrigger,
    DamagedBySource, Trigger as CompilerTrigger, TriggerKind,
    ZoneChangeTrigger as CompilerZoneChangeTrigger,
};
pub use types::{CardType, Subtype, SubtypeFamily, Supertype};
pub use value_model::{Condition, ManaSpendPermission, ManaSpendScope, Restriction, Value};
pub use zone::Zone;
