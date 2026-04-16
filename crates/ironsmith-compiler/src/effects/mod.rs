use crate::effect::Effect;
pub use ironsmith_core::{
    AddManaEffect, AddManaFromCommanderColorIdentityEffect, AddManaOfAnyColorEffect,
    AddManaOfAnyOneColorEffect, AmassEffect, AttachObjectsEffect, AttachToEffect,
    BattlefieldController, BecomeBasicLandTypeChoiceEffect, BecomeColorChoiceEffect,
    BecomeCreatureTypeChoiceEffect, BolsterEffect, CantEffect, CastSourceEffect, CastTaggedEffect,
    ChooseCardNameEffect, ChooseCardTypeEffect, ChooseModeEffect as CoreChooseModeEffect,
    ChooseNewTargetsEffect, ChooseObjectsEffect, ChoosePlayerEffect, ChooseSpellCastHistoryEffect,
    CipherEffect, ClashEffect, CombatDamagePreventionTarget,
    ConditionalEffect as CoreConditionalEffect, ConsultTopOfLibraryEffect,
    ConsultTopOfLibraryStopRule, ControlPlayerEffect, CopySpellEffect, CounterEffect,
    CreateEmblemEffect as CoreCreateEmblemEffect, CreateTokenEffect as CoreCreateTokenEffect,
    CrewCostEffect, DealDamageEffect, DealDistributedDamageEffect, DelayedTriggerSpec,
    DestroyEffect, DestroyNoRegenerationEffect, DiscardEffect, DiscardHandEffect, DiscoverEffect,
    DrawCardsEffect, DrawForEachTaggedMatchingEffect, EachPlayerScryEffect, EarthbendEffect,
    EmitKeywordActionEffect, EnergyCountersEffect, ExchangeControlEffect, ExchangeTextBoxesEffect,
    ExchangeValueOperand, ExecuteWithSourceEffect as CoreExecuteWithSourceEffect, ExertCostEffect,
    ExileEffect, ExileInsteadOfGraveyardEffect, ExileTaggedWhenSourceLeavesEffect,
    ExileTopOfLibraryEffect, ExileUntilDuration, ExileUntilEffect, ExtraTurnAfterNextTurnEffect,
    ExtraTurnEffect, FlipEffect, ForEachControllerOfTaggedEffect,
    ForEachCounterKindPutOrRemoveEffect, ForEachObject as CoreForEachObject, ForEachTaggedEffect,
    ForEachTaggedPlayerEffect, ForPlayersEffect, GainLifeEffect,
    GrantAbilitiesTargetEffect as CoreGrantAbilitiesTargetEffect,
    GrantBySpecEffect as CoreGrantBySpecEffect, GrantEffect as CoreGrantEffect,
    GrantNextSpellCostReductionEffect, GrantPlayTaggedDuration, GrantPlayTaggedEffect,
    GrantTaggedSpellFreeCastUntilEndOfTurnEffect, GrantTaggedSpellLifeCostByManaValueEffect,
    HauntExileEffect as CoreHauntExileEffect, IfEffect as CoreIfEffect, InvestigateEffect,
    LibraryBottomOrder, LibraryConsultMode, LocalRewriteEffect as CoreLocalRewriteEffect,
    LookAtHandEffect, LookAtTopCardsEffect, LoseLifeEffect, LoseTheGameEffect, MayEffect,
    MayMoveToZoneEffect, MeldEffect, MillEffect, ModifyPowerToughnessEffect,
    ModifyPowerToughnessForEachEffect, MonstrosityEffect, MoveAllCountersEffect,
    MoveToLibraryNthFromTopEffect, MoveToZoneEffect, NewTargetRestriction, NinjutsuCostEffect,
    NinjutsuEffect, PayEnergyEffect, PayManaEffect, PhaseOutEffect, PlaceholderEffect,
    PopulateEffect, PreventAllCombatDamageEffect, PreventAllDamageEffect,
    PreventAllDamageToTargetEffect as CorePreventAllDamageToTargetEffect,
    PreventDamageEffect as CorePreventDamageEffect, PreventNextTimeDamageEffect,
    PreventNextTimeDamageSource, PreventNextTimeDamageTarget, ProliferateEffect, PutCountersEffect,
    PutOntoBattlefieldEffect, PutTaggedRemainderOnLibraryBottomEffect,
    RearrangeLookedCardsInLibraryEffect, RedirectNextDamageToTargetEffect,
    RedirectNextTimeDamageSource, RedirectNextTimeDamageToSourceEffect,
    ReflexiveTriggerEffect as CoreReflexiveTriggerEffect, RegenerateEffect,
    RegisterZoneReplacementEffect, RemoveAnyCountersAmongEffect, RemoveAnyCountersFromSourceEffect,
    RemoveCountersEffect, RemoveFromCombatEffect, RemoveUpToAnyCountersEffect, RenownEffect,
    ReorderLibraryTopEffect, RepeatProcessPromptEffect, ReplacementApplyMode,
    RetainManaUntilEndOfTurnEffect, RetargetMode, RetargetStackObjectEffect,
    ReturnAllToBattlefieldEffect, ReturnFromGraveyardToBattlefieldEffect,
    ReturnFromGraveyardToHandEffect, ReturnToHandEffect, RevealTaggedEffect, RevealTopEffect,
    SacrificeEffect, SacrificePlayerEffect, SacrificeTargetEffect,
    ScheduleEffectsWhenTaggedLeavesEffect as CoreScheduleEffectsWhenTaggedLeavesEffect, ScryEffect,
    SearchLibraryEffect as CoreSearchLibraryEffect, SearchLibrarySlot,
    SearchLibrarySlotsEffect as CoreSearchLibrarySlotsEffect, SequenceEffect as CoreSequenceEffect,
    SetBasePowerToughnessEffect, SharedTypeConstraint, ShuffleLibraryEffect,
    ShuffleObjectsIntoLibraryEffect, TagAttachedToSourceEffect, TagMatchingObjectsEffect,
    TagTriggeringObjectEffect, TaggedEffect as CoreTaggedEffect, TaggedLeavesAbilitySource,
    TapEffect, TargetOnlyEffect, TransformEffect, UnearthEffect, UnlessActionEffect,
    UnlessPaysEffect, UntapEffect, WinTheGameEffect, WithIdEffect as CoreWithIdEffect,
};

pub type ChooseModeEffect = CoreChooseModeEffect<Effect>;
pub type CreateEmblemEffect = CoreCreateEmblemEffect<crate::effect::EmblemDescription>;
pub type CreateTokenEffect = CoreCreateTokenEffect<crate::cards::CardDefinition>;
pub type ConditionalEffect = CoreConditionalEffect<Effect>;
pub type ExecuteWithSourceEffect = CoreExecuteWithSourceEffect<Effect>;
pub type ForEachObject = CoreForEachObject<Effect>;
pub type HauntExileEffect = CoreHauntExileEffect<Effect>;
pub type IfEffect = CoreIfEffect<Effect>;
pub type LocalRewriteEffect = CoreLocalRewriteEffect<Effect>;
pub type PreventDamageEffect = CorePreventDamageEffect<Effect>;
pub type PreventAllDamageToTargetEffect = CorePreventAllDamageToTargetEffect<Effect>;
pub type ScheduleEffectsWhenTaggedLeavesEffect = CoreScheduleEffectsWhenTaggedLeavesEffect<Effect>;
pub type SequenceEffect = CoreSequenceEffect<Effect>;
pub type WithIdEffect = CoreWithIdEffect<Effect>;
pub type TaggedEffect = CoreTaggedEffect<Effect>;
pub type ReflexiveTriggerEffect = CoreReflexiveTriggerEffect<Effect>;
pub type RepeatEffectsEffect = ironsmith_core::RepeatEffectsEffect<Effect>;
pub type RepeatProcessEffect = ironsmith_core::RepeatProcessEffect<Effect>;
pub type VoteChoice = ironsmith_core::VoteChoice<Effect>;
pub type VoteEffect = ironsmith_core::VoteEffect<Effect>;
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
    ironsmith_core::GrantNextSpellAbilityEffect<crate::static_abilities::StaticAbility>;

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
        Placeholder(String),
        ModifyPowerToughness {
            power: crate::effect::Value,
            toughness: crate::effect::Value,
        },
        ChangeControllerToEffectController,
        ChangeControllerToPlayer(crate::target::PlayerFilter),
        CopyOf {
            source: crate::target::ChooseSpec,
            preserve_source_abilities: bool,
        },
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
        AddManaOfChosenColorEffect, AddManaOfImprintedColorsEffect, AddScaledManaEffect,
    };
}
