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

// Re-export the trait, modal spec, and cost validation error
pub use executor_trait::{CostValidationError, EffectExecutor, ModalSpec};

// Re-export effect implementations
pub use cards::{
    ConniveEffect, DiscardEffect, DiscardHandEffect, DrawCardsEffect, ExileTopOfLibraryEffect,
    LookAtHandEffect, LookAtTopCardsEffect, MillEffect, RevealTopEffect, ScryEffect,
    SearchLibraryEffect, ShuffleGraveyardIntoLibraryEffect, ShuffleLibraryEffect, SurveilEffect,
};
pub use combat::{
    EnterAttackingEffect, FightEffect, GoadEffect, GrantAbilitiesAllEffect,
    GrantAbilitiesTargetEffect, ModifyPowerToughnessAllEffect, ModifyPowerToughnessEffect,
    ModifyPowerToughnessForEachEffect, PreventAllCombatDamageFromEffect, PreventAllDamageEffect,
    PreventAllDamageToTargetEffect, PreventDamageEffect, SetBasePowerToughnessEffect,
};
pub use composition::{
    AdaptEffect, BeholdEffect, BolsterEffect, ChooseModeEffect, ChooseObjectsEffect,
    ConditionalEffect, CounterAbilityEffect, EmitKeywordActionEffect, ExploreEffect,
    ForEachControllerOfTaggedEffect, ForEachObject, ForEachTaggedEffect, ForEachTaggedPlayerEffect,
    ForPlayersEffect, IfEffect, ManifestDreadEffect, MayEffect, OpenAttractionEffect,
    SequenceEffect, SupportEffect, TagAllEffect, TagAttachedToSourceEffect,
    TagTriggeringDamageTargetEffect, TagTriggeringObjectEffect, TaggedEffect, TargetOnlyEffect,
    UnlessActionEffect, UnlessPaysEffect, VoteEffect, VoteOption, WithIdEffect,
};
pub use continuous::ApplyContinuousEffect;
pub use control::{ExchangeControlEffect, GainControlEffect, SharedTypeConstraint};
pub use counters::{
    MoveAllCountersEffect, MoveCountersEffect, ProliferateEffect, PutCountersEffect,
    RemoveCountersEffect, RemoveUpToAnyCountersEffect, RemoveUpToCountersEffect,
};
pub use damage::{ClearDamageEffect, DealDamageEffect};
pub use delayed::{
    ExileTaggedWhenSourceLeavesEffect, SacrificeSourceWhenTaggedLeavesEffect,
    ScheduleDelayedTriggerEffect, ScheduleEffectsWhenTaggedLeavesEffect, TaggedLeavesAbilitySource,
};
pub use life::{ExchangeLifeTotalsEffect, GainLifeEffect, LoseLifeEffect, SetLifeTotalEffect};
pub use mana::{
    AddColorlessManaEffect, AddManaEffect, AddManaFromCommanderColorIdentityEffect,
    AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect, AddManaOfChosenColorEffect,
    AddManaOfLandProducedTypesEffect, AddScaledManaEffect, GrantManaAbilityUntilEotEffect,
    PayManaEffect,
};
pub use permanents::{
    AttachObjectsEffect, AttachToEffect, BecomeSaddledUntilEotEffect, CrewCostEffect,
    EarthbendEffect, EvolveEffect, FlipEffect, GrantObjectAbilityEffect, MonstrosityEffect,
    NinjutsuCostEffect, NinjutsuEffect, RegenerateEffect, RenownEffect, SaddleCostEffect,
    TapEffect, TrainingEffect, TransformEffect, UnearthEffect, UntapEffect,
};
pub use player::{
    CascadeEffect, CastSourceEffect, CastTaggedEffect, ControlPlayerEffect, CreateEmblemEffect,
    EnergyCountersEffect, ExileInsteadOfGraveyardEffect, ExperienceCountersEffect, ExtraTurnEffect,
    GrantEffect, GrantPlayFromGraveyardEffect, LoseTheGameEffect, PayEnergyEffect,
    PoisonCountersEffect, SkipCombatPhasesEffect, SkipDrawStepEffect,
    SkipNextCombatPhaseThisTurnEffect, SkipTurnEffect, WinTheGameEffect,
};
pub use replacement::{ApplyReplacementEffect, ReplacementApplyMode};
pub use restrictions::CantEffect;
pub use stack::{
    ChooseNewTargetsEffect, CopySpellEffect, CounterEffect, NewTargetRestriction, RetargetMode,
    RetargetStackObjectEffect,
};
pub use tokens::{CreateTokenCopyEffect, CreateTokenEffect, InvestigateEffect};
pub use zones::{
    BattlefieldController, DestroyEffect, ExileEffect, ExileFromHandAsCostEffect,
    ExileUntilDuration, ExileUntilEffect, HauntExileEffect, MoveToZoneEffect,
    PutOntoBattlefieldEffect,
    ReorderGraveyardEffect, ReorderLibraryTopEffect, ReturnAllToBattlefieldEffect,
    ReturnFromGraveyardOrExileToBattlefieldEffect, ReturnFromGraveyardToBattlefieldEffect,
    ReturnFromGraveyardToHandEffect, ReturnToHandEffect, SacrificeEffect, SacrificeTargetEffect,
};
