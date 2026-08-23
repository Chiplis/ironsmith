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
pub mod consult_helpers;
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
/// Cards revealed by an earlier effect in the same execution ("revealed this way").
pub const REVEALED_THIS_WAY_TAG: &str = crate::tag::REVEALED_THIS_WAY_TAG;

// Re-export the traits, modal spec, and cost validation error
pub use context::{ExecutionError, ResolvedTarget, TargetError, rebase_target_scope};
pub use executor_trait::{
    CostExecutableEffect, CostValidationError, DeferredPlayerActionProposal,
    EffectExecutionCategory, EffectExecutor, ModalEffectSpec, ModalSpec,
    SimultaneousEffectProposal, TargetReusePolicy, TargetSelectionProfile,
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
    RearrangeLookedCardsInLibraryEffect, ReorderTopPlanarDeckEffect, RevealFromHandEffect,
    RevealSourceFromHandEffect, RevealTaggedEffect, RevealTopEffect, ScryEffect,
    SearchLibraryEffect, SearchLibrarySlot, SearchLibrarySlotsEffect,
    ShuffleGraveyardIntoLibraryEffect, ShuffleHandAndGraveyardIntoLibraryEffect,
    ShuffleLibraryEffect, SurveilEffect,
};
pub use combat::{
    AssignNoCombatDamageEffect, CombatDamagePreventionTarget, EnterAttackingEffect,
    ExchangeValueKind, ExchangeValueOperand, ExchangeValuesEffect, FightEffect, GoadEffect,
    GrantAbilitiesAllEffect, GrantAbilitiesTargetEffect, MeleeEffect,
    ModifyPowerToughnessAllEffect, ModifyPowerToughnessEffect, ModifyPowerToughnessForEachEffect,
    PreventAllCombatDamageEffect, PreventAllCombatDamageFromEffect, PreventAllDamageEffect,
    PreventAllDamageToTargetEffect, PreventDamageEffect, RemoveFromCombatEffect,
    SetBasePowerToughnessEffect,
};
pub use composition::{
    AdaptEffect, AmplifyEffect, AuraSwapEffect, BackupEffect, BeholdEffect, BidLifeEffect,
    BolsterEffect, CastEncodedCardCopyEffect, ChooseModeEffect, ChooseObjectsEffect,
    ChooseSpellCastHistoryEffect, CipherEffect, ConditionalEffect, CounterAbilityEffect,
    CumulativeUpkeepEffect, DevourEffect, EmitGiftGivenEffect, EmitKeywordActionEffect,
    ExecuteWithSourceEffect, ExploreEffect, ForEachControllerOfTaggedEffect, ForEachObject,
    ForEachObjectCorrelatedResultEffect, ForEachTaggedEffect, ForEachTaggedPlayerEffect,
    ForPlayersEffect, GrantRepeatableManaPaymentActionUntilEndOfTurnEffect, IfEffect, LifeBidStart,
    LocalRewriteEffect, ManaRestrictedEffect, ManaRetainedEffect, ManifestCardFromHandEffect,
    ManifestDreadEffect, ManifestObjectsEffect, ManifestTopCardOfLibraryEffect, MayEffect,
    OpenAttractionEffect, PopulateEffect, ReflexiveTriggerEffect, RepeatEffectsEffect,
    RepeatProcessEffect, RepeatProcessPromptEffect, SecretChoiceEffect, SecretChoiceResult,
    SequenceEffect, SupportEffect, TagAllEffect, TagAttachedToSourceEffect,
    TagMatchingObjectsEffect, TagOtherBlockParticipantEffect, TagTriggeringAttackerEffect,
    TagTriggeringBlockersEffect, TagTriggeringDamageTargetEffect, TagTriggeringObjectEffect,
    TagTriggeringSourceEffect, TaggedEffect, TargetOnlyEffect, UnlessActionEffect,
    UnlessPaysEffect, VOTE_WINNERS_TAG, VOTED_OBJECTS_TAG, VillainousChoiceEffect, VoteChoice,
    VoteEffect, VoteOption, VoteResult, WithIdEffect,
};
pub use continuous::{ApplyContinuousEffect, ExchangeTextBoxesEffect, RuntimeModification};
pub use control::{
    DirectionalAdjacentPlayerControlEffect, ExchangeControlEffect, GainControlEffect,
    SharedTypeConstraint,
};
pub use counters::remove_any_counters_among_cost_display;
pub(crate) use counters::remove_any_counters_among_valid_targets_with_tags;
pub use counters::{
    DoubleCountersEffect, ForEachCounterKindPutOrRemoveEffect, MoveAllCountersEffect,
    MoveCountersEffect, MoveOneCounterEffect, ProliferateEffect, PutCounterOfChosenKindEffect,
    PutCountersEffect, RemoveAnyCountersAmongEffect, RemoveAnyCountersFromSourceEffect,
    RemoveCountersEffect, RemoveUpToAnyCountersEffect, RemoveUpToCountersEffect,
};
pub use damage::{
    ClearDamageEffect, DamageDistributionMode, DealDamageEffect, DealDistributedDamageEffect,
    HealDamageEffect, PreventNextTimeDamageEffect, PreventNextTimeDamageSource,
    PreventNextTimeDamageTarget, RedirectAllDamageThisTurnToTargetEffect,
    RedirectNextDamageDestination, RedirectNextDamageToTargetEffect,
    RedirectNextTimeDamageDestination, RedirectNextTimeDamageSource,
    RedirectNextTimeDamageToSourceEffect, ReplaceNextDamageToTargetEffect,
};
pub use delayed::{
    DelayedTriggerPrepayment, ExileTaggedWhenSourceLeavesEffect,
    SacrificeSourceWhenTaggedLeavesEffect, ScheduleDelayedTriggerEffect,
    ScheduleEffectsWhenTaggedLeavesEffect, TaggedLeavesAbilitySource,
};
pub use life::{
    ExchangeLifeTotalsEffect, GainLifeEffect, LoseLifeEffect, NoteLifeTotalEffect, PayLifeEffect,
    SetLifeTotalEffect,
};
pub use mana::{
    AddColorlessManaEffect, AddManaEffect, AddManaFromCommanderColorIdentityEffect,
    AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect, AddManaOfChosenColorEffect,
    AddManaOfColorsAmongEffect, AddManaOfLandProducedTypesEffect, AddOneManaOfAnyColorAmongEffect,
    AddScaledManaEffect, DoubleManaPoolEffect, EmptyManaPoolEffect, GrantManaAbilityUntilEotEffect,
    ManaTypeSource, PayManaEffect, RetainManaUntilEndOfTurnEffect,
};
pub use permanents::{
    AttachObjectsEffect, AttachToEffect, BecomeBasicLandTypeChoiceEffect, BecomeColorChoiceEffect,
    BecomeCreatureTypeChoiceEffect, BecomeSaddledUntilEotEffect, ClearSuspectedEffect,
    ConspireCostEffect, ConvertEffect, CrewCostEffect, DetainEffect, EarthbendEffect, EvolveEffect,
    ExertCostEffect, FlipEffect, GrantObjectAbilityEffect, MeldEffect, MonstrosityEffect,
    NinjutsuCostEffect, NinjutsuEffect, PhaseInEffect, PhaseOutDuration, PhaseOutEffect,
    PutStickerEffect, ReconfigureEffect, RegenerateEffect, RenownEffect, SaddleCostEffect,
    SneakCostEffect, SolveCaseEffect, SoulbondPairEffect, SuspectEffect, TapEffect,
    TransformEffect, TurnFaceUpEffect, UmbraArmorEffect, UnattachObjectsEffect, UnearthEffect,
    UnlockRoomDoorEffect, UntapEffect,
};
pub use player::{
    AdditionalLandPlaysEffect, AdditionalPhase, AdditionalPhasesEffect, AscendEffect,
    BecomeMonarchEffect, CascadeEffect, CastSourceEffect, CastTaggedEffect, ChooseCardNameEffect,
    ChooseCardTypeEffect, ChooseColorEffect, ChooseCreatureTypeEffect, ChooseLandTypeEffect,
    ChooseNamedOptionEffect, ChoosePlayerEffect, ControlCombatChoicesThisTurnEffect,
    ControlPlayerEffect, CreateEmblemEffect, DiscoverEffect, DrawTheGameEffect,
    EndCombatPhaseEffect, EndTurnEffect, EnergyCountersEffect, ExileInsteadOfGraveyardEffect,
    ExileThenGrantPlayEffect, ExileUntilMatchCastEffect, ExileUntilMatchGrantPlayEffect,
    ExperienceCountersEffect, ExtraTurnAfterNextTurnEffect, ExtraTurnEffect, FlipCoinEffect,
    GrantBySpecEffect, GrantEffect, GrantNextSpellAbilityEffect, GrantNextSpellCostReductionEffect,
    GrantPlayTaggedDuration, GrantPlayTaggedEffect, GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
    GrantTaggedSpellLifeCostByManaValueEffect, IncreaseSpeedEffect, LoseTheGameEffect,
    MayCastMatchingSpellWithoutPayingManaCostEffect, PayAnyEnergyEffect, PayAnyLifeEffect,
    PayEnergyEffect, PlaySubgameEffect, PlayerCountersEffect, PoisonCountersEffect,
    RadiationEffect, ReduceSpeedEffect, RestartGameEffect, ReverseTurnOrderEffect,
    RingTemptsYouEffect, RollDiceChooseResultEffect, RollDieEffect, SkipCombatPhasesEffect,
    SkipCombatPhasesThisTurnEffect, SkipDrawStepEffect, SkipMainPhasesThisTurnEffect,
    SkipNextCombatPhaseThisTurnEffect, SkipTurnEffect, TakeInitiativeEffect, TicketCountersEffect,
    VentureIntoDungeonEffect, WinTheGameEffect,
};
pub use replacement::{
    ApplyReplacementEffect, RegisterDamagedBySourceZoneReplacementEffect,
    RegisterDrawReplacementEffect, RegisterEnterTappedReplacementEffect,
    RegisterEnterUnderControlReplacementEffect, RegisterFutureZoneReplacementEffect,
    RegisterManaReplacementEffect, RegisterNextBatchEnterWithCountersEffect,
    RegisterZoneReplacementEffect, ReplacementApplyMode,
};
pub use restrictions::CantEffect;
pub(crate) use stack::EpicSpellCopyEffect;
pub use stack::{
    ChooseNewTargetsEffect, CopySpellEffect, CopySpellForEachTargetEffect, CounterEffect,
    NewTargetRestriction, RetargetMode, RetargetStackObjectEffect, ScaleXValueEffect,
    VariableCasualtyPlaneswalkerCopyEffect,
};
pub use tokens::{
    AmassEffect, CopyAttackTargetMode, CreateTokenCopyEffect, CreateTokenEffect, IncubateEffect,
    InvestigateEffect, TokenCopyReferenceSurface,
};
pub use zones::{
    BattlefieldController, DestroyEffect, DestroyNoRegenerationEffect, EachPlayerSacrificesEffect,
    ExchangeZonesEffect, ExileEffect, ExileUntilDuration, ExileUntilEffect, HauntExileEffect,
    LibraryPlacementOrder, MayMoveToZoneEffect, MoveToLibraryNthFromTopEffect,
    MoveToLibraryTopOrBottomChoiceEffect, MoveToZoneAttackTargetMode, MoveToZoneEffect,
    PutOntoBattlefieldEffect, ReorderGraveyardEffect, ReorderLibraryTopEffect,
    ReturnAllToBattlefieldEffect, ReturnAsAuraOptions,
    ReturnFromGraveyardOrExileToBattlefieldEffect, ReturnFromGraveyardToBattlefieldEffect,
    ReturnFromGraveyardToHandEffect, ReturnToHandEffect, SacrificeEffect, SacrificeTargetEffect,
    ShuffleObjectsIntoLibraryEffect,
};
