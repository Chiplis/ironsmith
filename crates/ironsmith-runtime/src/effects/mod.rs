//! Modular effect system for MTG.
//!
//! This module provides a trait-based architecture for effect execution.
//! Each effect type implements the `EffectExecutor` trait, allowing for:
//! - Co-located tests with each effect implementation
//! - Self-contained effect logic
//! - Easy addition of new effects without modifying central dispatcher
//!
//! # Module Structure
//!
//! ```text
//! effects/
//!   mod.rs              - This file, module organization
//!   executor_trait.rs   - EffectExecutor trait definition
//!   helpers.rs          - Shared utilities (resolve_value, etc.)
//!   damage/
//!     mod.rs
//!     deal_damage.rs    - DealDamageEffect implementation + tests
//! ```
//!
//! # Usage
//!
//! Effects can be executed through the `EffectExecutor` trait:
//!
//! ```ignore
//! use ironsmith::effects::{EffectExecutor, DealDamageEffect};
//!
//! let effect = DealDamageEffect::new(3, ChooseSpec::AnyTarget);
//! let result = effect.execute(&mut game, &mut ctx)?;
//! ```
//!
//! # Runtime Categories
//!
//! The supported runtime extension categories are:
//! - `Standard`: ordinary resolving effects
//! - `CostExecutable`: effects that can participate in cost payment
//! - `DelayedTriggerRegistration`: effects that register delayed trigger state
//! - `ReplacementRegistration`: effects that register replacement state
//!
//! New runtime work should fit one of those categories. When in doubt, prefer
//! building a reusable `Standard` effect first and only opt into the more
//! specialized categories when the effect's main job is registration or cost handling.
//!
//! # Ownership
//!
//! Effect execution routes through the runtime harness in this module. Public
//! effect execution entry points live here, while effect model data remains in
//! the shared compiled-card domain.

pub mod cards;
pub mod combat;
pub mod composition;
pub(crate) mod consult_helpers;
mod context;
pub mod continuous;
pub mod control;
pub mod counters;
pub mod damage;
pub mod delayed;
mod executor_trait;
pub mod helpers;
pub mod life;
pub mod mana;
pub mod permanents;
pub mod player;
pub mod replacement;
pub mod restrictions;
mod runtime;
pub mod stack;
pub mod tokens;
pub mod zones;

/// Reserved tag used to carry public reveal visibility across stack lifetime.
pub const PUBLIC_REVEALED_TAG: &str = "__public_revealed";

// Re-export the traits, modal spec, and cost validation error
pub use context::{ExecutionError, ResolvedTarget, TargetError, rebase_target_scope};
pub use executor_trait::{
    CostExecutableEffect, CostValidationError, EffectExecutionCategory, EffectExecutor,
    ModalEffectSpec, ModalSpec, TargetReusePolicy, TargetSelectionProfile,
};
pub type EffectContext<'a> = context::ExecutionContext<'a>;
pub(crate) use context::ExecutionContext;
pub use runtime::{execute_effect, resolve_value, validate_target};

// Re-export effect implementations
pub use cards::{
    ClashEffect, ClashOpponentMode, ConniveEffect, ConsultTopOfLibraryEffect,
    ConsultTopOfLibraryStopRule, DiscardEffect, DiscardHandEffect, DrawCardsEffect,
    DrawForEachTaggedMatchingEffect, EachPlayerScryEffect, ExileTopOfLibraryEffect,
    ExileUntilMatchEffect, FatesealEffect, LearnEffect, LookAtHandEffect, LookAtObjectsEffect,
    LookAtTopCardsEffect, MillEffect, PutTaggedRemainderOnLibraryBottomEffect,
    RearrangeLookedCardsInLibraryEffect, RevealFromHandEffect, RevealSourceFromHandEffect,
    RevealTaggedEffect, RevealTopEffect, ScryEffect, SearchLibraryEffect, SearchLibrarySlot,
    SearchLibrarySlotsEffect, ShuffleGraveyardIntoLibraryEffect,
    ShuffleHandAndGraveyardIntoLibraryEffect, ShuffleLibraryEffect, SurveilEffect,
};
pub use combat::{
    CombatDamagePreventionTarget, EnterAttackingEffect, ExchangeValueKind, ExchangeValueOperand,
    ExchangeValuesEffect, FightEffect, GoadEffect, GrantAbilitiesAllEffect,
    GrantAbilitiesTargetEffect, MeleeEffect, ModifyPowerToughnessAllEffect,
    ModifyPowerToughnessEffect, ModifyPowerToughnessForEachEffect, PreventAllCombatDamageEffect,
    PreventAllCombatDamageFromEffect, PreventAllDamageEffect, PreventAllDamageToTargetEffect,
    PreventDamageEffect, RemoveFromCombatEffect, SetBasePowerToughnessEffect,
};
pub use composition::{
    AdaptEffect, AmplifyEffect, AuraSwapEffect, BackupEffect, BeholdEffect, BidLifeEffect,
    BolsterEffect, CastEncodedCardCopyEffect, ChooseModeEffect, ChooseObjectsEffect,
    ChooseSpellCastHistoryEffect, CipherEffect, ConditionalEffect, CounterAbilityEffect,
    CumulativeUpkeepEffect, DevourEffect, EmitGiftGivenEffect, EmitKeywordActionEffect,
    ExecuteWithSourceEffect, ExploreEffect, ForEachControllerOfTaggedEffect, ForEachObject,
    ForEachTaggedEffect, ForEachTaggedPlayerEffect, ForPlayersEffect, IfEffect, LifeBidStart,
    LocalRewriteEffect, ManaRestrictedEffect, ManifestCardFromHandEffect, ManifestDreadEffect,
    ManifestTopCardOfLibraryEffect, MayEffect, OpenAttractionEffect, PopulateEffect,
    ReflexiveTriggerEffect, RepeatEffectsEffect, RepeatProcessEffect, RepeatProcessPromptEffect,
    SequenceEffect, SupportEffect, TagAllEffect, TagAttachedToSourceEffect,
    TagMatchingObjectsEffect, TagTriggeringDamageTargetEffect, TagTriggeringObjectEffect,
    TagTriggeringSourceEffect, TaggedEffect, TargetOnlyEffect, UnlessActionEffect,
    UnlessPaysEffect, VOTE_WINNERS_TAG, VOTED_OBJECTS_TAG, VoteChoice, VoteEffect, VoteOption,
    VoteResult, WithIdEffect,
};
pub use continuous::{ApplyContinuousEffect, ExchangeTextBoxesEffect, RuntimeModification};
pub use control::{
    DirectionalAdjacentPlayerControlEffect, ExchangeControlEffect, GainControlEffect,
    SharedTypeConstraint,
};
pub use counters::{
    DoubleCountersEffect, ForEachCounterKindPutOrRemoveEffect, MoveAllCountersEffect,
    MoveCountersEffect, MoveOneCounterEffect, ProliferateEffect, PutCountersEffect,
    RemoveAnyCountersAmongEffect, RemoveAnyCountersFromSourceEffect, RemoveCountersEffect,
    RemoveUpToAnyCountersEffect, RemoveUpToCountersEffect,
};
pub(crate) use counters::{
    remove_any_counters_among_cost_display, remove_any_counters_among_valid_targets_with_tags,
};
pub use damage::{
    ClearDamageEffect, DealDamageEffect, DealDistributedDamageEffect, PreventNextTimeDamageEffect,
    PreventNextTimeDamageSource, PreventNextTimeDamageTarget,
    RedirectAllDamageThisTurnToTargetEffect, RedirectNextDamageToTargetEffect,
    RedirectNextTimeDamageDestination, RedirectNextTimeDamageSource,
    RedirectNextTimeDamageToSourceEffect,
};
pub use delayed::{
    ExileTaggedWhenSourceLeavesEffect, SacrificeSourceWhenTaggedLeavesEffect,
    ScheduleDelayedTriggerEffect, ScheduleEffectsWhenTaggedLeavesEffect, TaggedLeavesAbilitySource,
};
pub use life::{ExchangeLifeTotalsEffect, GainLifeEffect, LoseLifeEffect, SetLifeTotalEffect};
pub use mana::{
    AddColorlessManaEffect, AddManaEffect, AddManaFromCommanderColorIdentityEffect,
    AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect, AddManaOfChosenColorEffect,
    AddManaOfColorsAmongEffect, AddManaOfLandProducedTypesEffect, AddScaledManaEffect,
    DoubleManaPoolEffect, EmptyManaPoolEffect, GrantManaAbilityUntilEotEffect, PayManaEffect,
    RetainManaUntilEndOfTurnEffect,
};
pub use permanents::{
    AttachObjectsEffect, AttachToEffect, BecomeBasicLandTypeChoiceEffect, BecomeColorChoiceEffect,
    BecomeCreatureTypeChoiceEffect, BecomeSaddledUntilEotEffect, ClearSuspectedEffect,
    ConspireCostEffect, ConvertEffect, CrewCostEffect, DetainEffect, EarthbendEffect, EvolveEffect,
    ExertCostEffect, FlipEffect, GrantObjectAbilityEffect, MeldEffect, MonstrosityEffect,
    NinjutsuCostEffect, NinjutsuEffect, PhaseInEffect, PhaseOutEffect, PutStickerEffect,
    ReconfigureEffect, RegenerateEffect, RenownEffect, SaddleCostEffect, SneakCostEffect,
    SoulbondPairEffect, SuspectEffect, TapEffect, TransformEffect, UmbraArmorEffect, UnearthEffect,
    UntapEffect,
};
pub use player::{
    AdditionalLandPlaysEffect, AdditionalPhase, AdditionalPhasesEffect, BecomeMonarchEffect,
    CascadeEffect, CastSourceEffect, CastTaggedEffect, ChooseCardNameEffect, ChooseCardTypeEffect,
    ChooseColorEffect, ChooseCreatureTypeEffect, ChooseNamedOptionEffect, ChoosePlayerEffect,
    ControlCombatChoicesThisTurnEffect, ControlPlayerEffect, CreateEmblemEffect, DiscoverEffect,
    EndTurnEffect, EnergyCountersEffect, ExileInsteadOfGraveyardEffect, ExileThenGrantPlayEffect,
    ExileUntilMatchCastEffect, ExileUntilMatchGrantPlayEffect, ExperienceCountersEffect,
    ExtraTurnAfterNextTurnEffect, ExtraTurnEffect, FlipCoinEffect, GrantBySpecEffect, GrantEffect,
    GrantNextSpellAbilityEffect, GrantNextSpellCostReductionEffect, GrantPlayTaggedDuration,
    GrantPlayTaggedEffect, GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
    GrantTaggedSpellLifeCostByManaValueEffect, IncreaseSpeedEffect, LoseTheGameEffect,
    MayCastMatchingSpellWithoutPayingManaCostEffect, PayAnyEnergyEffect, PayEnergyEffect,
    PoisonCountersEffect, ReduceSpeedEffect, RingTemptsYouEffect, RollDiceChooseResultEffect,
    RollDieEffect, SkipCombatPhasesEffect, SkipCombatPhasesThisTurnEffect, SkipDrawStepEffect,
    SkipMainPhasesThisTurnEffect, SkipNextCombatPhaseThisTurnEffect, SkipTurnEffect,
    TakeInitiativeEffect, TicketCountersEffect, VentureIntoDungeonEffect, WinTheGameEffect,
};
pub use replacement::{
    ApplyReplacementEffect, RegisterDamagedBySourceZoneReplacementEffect,
    RegisterEnterUnderControlReplacementEffect, RegisterFutureZoneReplacementEffect,
    RegisterZoneReplacementEffect, ReplacementApplyMode,
};
pub use restrictions::CantEffect;
pub(crate) use stack::EpicSpellCopyEffect;
pub use stack::{
    ChooseNewTargetsEffect, CopySpellEffect, CopySpellForEachTargetEffect, CounterEffect,
    NewTargetRestriction, RetargetMode, RetargetStackObjectEffect,
    VariableCasualtyPlaneswalkerCopyEffect,
};
pub use tokens::{
    AmassEffect, CopyAttackTargetMode, CreateTokenCopyEffect, CreateTokenEffect, IncubateEffect,
    InvestigateEffect,
};
pub use zones::{
    BattlefieldController, DestroyEffect, DestroyNoRegenerationEffect, EachPlayerSacrificesEffect,
    ExchangeZonesEffect, ExileEffect, ExileUntilDuration, ExileUntilEffect, HauntExileEffect,
    MayMoveToZoneEffect, MoveToLibraryNthFromTopEffect, MoveToLibraryTopOrBottomChoiceEffect,
    MoveToZoneEffect, PutOntoBattlefieldEffect, ReorderGraveyardEffect, ReorderLibraryTopEffect,
    ReturnAllToBattlefieldEffect, ReturnAsAuraOptions,
    ReturnFromGraveyardOrExileToBattlefieldEffect, ReturnFromGraveyardToBattlefieldEffect,
    ReturnFromGraveyardToHandEffect, ReturnToHandEffect, SacrificeEffect, SacrificeTargetEffect,
    ShuffleObjectsIntoLibraryEffect,
};
