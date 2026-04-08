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
//! # Migration Status
//!
//! Effects are being migrated incrementally from the monolithic `execute_effect()`
//! function in `executor.rs`. During migration:
//! - The `Effect` enum remains unchanged while modular execution lands
//! - `execute_effect()` delegates to modular implementations via bridges
//! - New effects can be added directly to this module

pub mod cards;
pub mod combat;
pub mod composition;
pub(crate) mod consult_helpers;
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
pub mod stack;
pub mod tokens;
pub mod zones;

/// Reserved tag used to carry public reveal visibility across stack lifetime.
pub const PUBLIC_REVEALED_TAG: &str = "__public_revealed";

// Re-export the traits, modal spec, and cost validation error
pub use executor_trait::{CostExecutableEffect, CostValidationError, EffectExecutor, ModalSpec};

// Re-export effect implementations
pub use cards::{
    ClashEffect, ClashOpponentMode, ConniveEffect, ConsultTopOfLibraryEffect,
    ConsultTopOfLibraryStopRule, DiscardEffect, DiscardHandEffect, DrawCardsEffect,
    DrawForEachTaggedMatchingEffect, EachPlayerScryEffect, ExileTopOfLibraryEffect,
    ExileUntilMatchEffect, FatesealEffect, LookAtHandEffect, LookAtTopCardsEffect, MillEffect,
    PutTaggedRemainderOnLibraryBottomEffect, RearrangeLookedCardsInLibraryEffect,
    RevealFromHandEffect, RevealTaggedEffect, RevealTopEffect, ScryEffect, SearchLibraryEffect,
    SearchLibrarySlot, SearchLibrarySlotsEffect, ShuffleGraveyardIntoLibraryEffect,
    ShuffleLibraryEffect, SurveilEffect,
};
pub use combat::{
    EnterAttackingEffect, ExchangeValueKind, ExchangeValueOperand, ExchangeValuesEffect,
    FightEffect, GoadEffect, GrantAbilitiesAllEffect, GrantAbilitiesTargetEffect, MeleeEffect,
    ModifyPowerToughnessAllEffect, ModifyPowerToughnessEffect, ModifyPowerToughnessForEachEffect,
    PreventAllCombatDamageFromEffect, PreventAllDamageEffect, PreventAllDamageToTargetEffect,
    PreventDamageEffect, RemoveFromCombatEffect, SetBasePowerToughnessEffect,
};
pub use composition::{
    AdaptEffect, BackupEffect, BeholdEffect, BolsterEffect, CastEncodedCardCopyEffect,
    ChooseModeEffect, ChooseObjectsEffect, ChooseSpellCastHistoryEffect, CipherEffect,
    ConditionalEffect, CounterAbilityEffect, DevourEffect, EmitGiftGivenEffect,
    EmitKeywordActionEffect, ExecuteWithSourceEffect, ExploreEffect,
    ForEachControllerOfTaggedEffect, ForEachObject, ForEachTaggedEffect, ForEachTaggedPlayerEffect,
    ForPlayersEffect, IfEffect, LocalRewriteEffect, ManifestDreadEffect,
    ManifestTopCardOfLibraryEffect, MayEffect, OpenAttractionEffect, PopulateEffect,
    ReflexiveTriggerEffect, RepeatEffectsEffect, RepeatProcessEffect, RepeatProcessPromptEffect,
    SequenceEffect, SupportEffect, TagAllEffect, TagAttachedToSourceEffect,
    TagMatchingObjectsEffect, TagTriggeringDamageTargetEffect, TagTriggeringObjectEffect,
    TaggedEffect, TargetOnlyEffect, UnlessActionEffect, UnlessPaysEffect, VOTE_WINNERS_TAG,
    VOTED_OBJECTS_TAG, VoteChoice, VoteEffect, VoteOption, VoteResult, WithIdEffect,
};
pub use continuous::{ApplyContinuousEffect, ExchangeTextBoxesEffect};
pub use control::{ExchangeControlEffect, GainControlEffect, SharedTypeConstraint};
pub use counters::{
    ForEachCounterKindPutOrRemoveEffect, MoveAllCountersEffect, MoveCountersEffect,
    ProliferateEffect, PutCountersEffect, RemoveAnyCountersAmongEffect,
    RemoveAnyCountersFromSourceEffect, RemoveCountersEffect, RemoveUpToAnyCountersEffect,
    RemoveUpToCountersEffect,
};
pub use damage::{
    ClearDamageEffect, DealDamageEffect, DealDistributedDamageEffect, PreventNextTimeDamageEffect,
    PreventNextTimeDamageSource, PreventNextTimeDamageTarget, RedirectNextDamageToTargetEffect,
    RedirectNextTimeDamageSource, RedirectNextTimeDamageToSourceEffect,
};
pub use delayed::{
    ExileTaggedWhenSourceLeavesEffect, SacrificeSourceWhenTaggedLeavesEffect,
    ScheduleDelayedTriggerEffect, ScheduleEffectsWhenTaggedLeavesEffect, TaggedLeavesAbilitySource,
};
pub use life::{ExchangeLifeTotalsEffect, GainLifeEffect, LoseLifeEffect, SetLifeTotalEffect};
pub use mana::{
    AddColorlessManaEffect, AddManaEffect, AddManaFromCommanderColorIdentityEffect,
    AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect, AddManaOfChosenColorEffect,
    AddManaOfLandProducedTypesEffect, AddScaledManaEffect, DoubleManaPoolEffect,
    GrantManaAbilityUntilEotEffect, PayManaEffect, RetainManaUntilEndOfTurnEffect,
};
pub use permanents::{
    AttachObjectsEffect, AttachToEffect, BecomeBasicLandTypeChoiceEffect, BecomeColorChoiceEffect,
    BecomeCreatureTypeChoiceEffect, BecomeSaddledUntilEotEffect, ConspireCostEffect, ConvertEffect,
    CrewCostEffect, DetainEffect, EarthbendEffect, EvolveEffect, ExertCostEffect, FlipEffect,
    GrantObjectAbilityEffect, MeldEffect, MonstrosityEffect, NinjutsuCostEffect, NinjutsuEffect,
    PhaseOutEffect, PutStickerEffect, RegenerateEffect, RenownEffect, SaddleCostEffect,
    SoulbondPairEffect, TapEffect, TransformEffect, UmbraArmorEffect, UnearthEffect, UntapEffect,
};
pub use player::{
    AdditionalLandPlaysEffect, BecomeMonarchEffect, CascadeEffect, CastSourceEffect,
    CastTaggedEffect, ChooseCardNameEffect, ChooseColorEffect, ChooseCreatureTypeEffect,
    ChooseNamedOptionEffect, ChoosePlayerEffect, ControlPlayerEffect, CreateEmblemEffect,
    DiscoverEffect, EnergyCountersEffect, ExileInsteadOfGraveyardEffect, ExileThenGrantPlayEffect,
    ExileUntilMatchCastEffect, ExileUntilMatchGrantPlayEffect, ExperienceCountersEffect,
    ExtraTurnAfterNextTurnEffect, ExtraTurnEffect, GrantBySpecEffect, GrantEffect,
    GrantNextSpellAbilityEffect, GrantNextSpellCostReductionEffect, GrantPlayTaggedDuration,
    GrantPlayTaggedEffect, GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
    GrantTaggedSpellLifeCostByManaValueEffect, LoseTheGameEffect, PayEnergyEffect,
    PoisonCountersEffect, RingTemptsYouEffect, SkipCombatPhasesEffect, SkipDrawStepEffect,
    SkipNextCombatPhaseThisTurnEffect, SkipTurnEffect, TakeInitiativeEffect,
    VentureIntoDungeonEffect, WinTheGameEffect,
};
pub use replacement::{
    ApplyReplacementEffect, RegisterZoneReplacementEffect, ReplacementApplyMode,
};
pub use restrictions::CantEffect;
pub use stack::{
    ChooseNewTargetsEffect, CopySpellEffect, CounterEffect, NewTargetRestriction, RetargetMode,
    RetargetStackObjectEffect,
};
pub use tokens::{
    AmassEffect, CopyAttackTargetMode, CreateTokenCopyEffect, CreateTokenEffect, InvestigateEffect,
};
pub use zones::{
    BattlefieldController, DestroyEffect, DestroyNoRegenerationEffect, EachPlayerSacrificesEffect,
    ExchangeZonesEffect, ExileEffect, ExileUntilDuration, ExileUntilEffect, HauntExileEffect,
    MayMoveToZoneEffect, MoveToLibraryNthFromTopEffect, MoveToZoneEffect, PutOntoBattlefieldEffect,
    ReorderGraveyardEffect, ReorderLibraryTopEffect, ReturnAllToBattlefieldEffect,
    ReturnFromGraveyardOrExileToBattlefieldEffect, ReturnFromGraveyardToBattlefieldEffect,
    ReturnFromGraveyardToHandEffect, ReturnToHandEffect, SacrificeEffect, SacrificeTargetEffect,
    ShuffleObjectsIntoLibraryEffect,
};
