use crate::effect::Effect;
pub use ironsmith_core::{
    AmassEffect, BattlefieldController, BecomeBasicLandTypeChoiceEffect, BecomeColorChoiceEffect,
    BecomeCreatureTypeChoiceEffect, ChooseModeEffect as CoreChooseModeEffect, ChooseObjectsEffect,
    ChoosePlayerEffect, ChooseSpellCastHistoryEffect, ClashEffect,
    ConditionalEffect as CoreConditionalEffect, ConsultTopOfLibraryStopRule, CopySpellEffect,
    CounterEffect, CreateTokenEffect as CoreCreateTokenEffect, CrewCostEffect, DealDamageEffect,
    DealDistributedDamageEffect, DelayedTriggerSpec, DestroyEffect, DestroyNoRegenerationEffect,
    DiscardEffect, DrawCardsEffect, DrawForEachTaggedMatchingEffect, EachPlayerScryEffect,
    EarthbendEffect, ExchangeControlEffect, ExchangeValueOperand,
    ExecuteWithSourceEffect as CoreExecuteWithSourceEffect, ExertCostEffect, ExileEffect,
    ExileTaggedWhenSourceLeavesEffect, ExileTopOfLibraryEffect, ExileUntilDuration,
    ExileUntilEffect, ForEachControllerOfTaggedEffect, ForEachCounterKindPutOrRemoveEffect,
    ForEachObject as CoreForEachObject, ForEachTaggedEffect, ForEachTaggedPlayerEffect,
    ForPlayersEffect, GrantAbilitiesTargetEffect as CoreGrantAbilitiesTargetEffect,
    GrantNextSpellCostReductionEffect, GrantPlayTaggedDuration, GrantPlayTaggedEffect,
    GrantTaggedSpellFreeCastUntilEndOfTurnEffect, GrantTaggedSpellLifeCostByManaValueEffect,
    HauntExileEffect as CoreHauntExileEffect, IfEffect as CoreIfEffect, InvestigateEffect,
    LocalRewriteEffect as CoreLocalRewriteEffect, LookAtHandEffect, LoseLifeEffect, MayEffect,
    MeldEffect, MillEffect, ModifyPowerToughnessEffect, MoveToLibraryNthFromTopEffect,
    MoveToZoneEffect, NewTargetRestriction, PayEnergyEffect, PayManaEffect, PhaseOutEffect,
    PlaceholderEffect, PopulateEffect,
    PreventAllDamageToTargetEffect as CorePreventAllDamageToTargetEffect,
    PreventDamageEffect as CorePreventDamageEffect, PreventNextTimeDamageEffect,
    PreventNextTimeDamageSource, PreventNextTimeDamageTarget, PutCountersEffect,
    RedirectNextDamageToTargetEffect, RedirectNextTimeDamageSource,
    RedirectNextTimeDamageToSourceEffect, ReflexiveTriggerEffect as CoreReflexiveTriggerEffect,
    RegisterZoneReplacementEffect, RemoveAnyCountersFromSourceEffect, RemoveCountersEffect,
    RemoveFromCombatEffect, ReorderLibraryTopEffect, RepeatProcessPromptEffect,
    ReplacementApplyMode, RetainManaUntilEndOfTurnEffect, RetargetMode, RetargetStackObjectEffect,
    ReturnAllToBattlefieldEffect, ReturnToHandEffect, RevealTaggedEffect, SacrificeEffect,
    SacrificeTargetEffect,
    ScheduleEffectsWhenTaggedLeavesEffect as CoreScheduleEffectsWhenTaggedLeavesEffect, ScryEffect,
    SearchLibraryEffect as CoreSearchLibraryEffect, SearchLibrarySlot,
    SearchLibrarySlotsEffect as CoreSearchLibrarySlotsEffect, SequenceEffect as CoreSequenceEffect,
    SharedTypeConstraint, TagMatchingObjectsEffect, TaggedEffect as CoreTaggedEffect,
    TaggedLeavesAbilitySource, TapEffect, TargetOnlyEffect, UnlessActionEffect, UnlessPaysEffect,
    UntapEffect, WithIdEffect as CoreWithIdEffect,
};

pub type ChooseModeEffect = CoreChooseModeEffect<Effect>;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastSourceEffect {
    pub without_paying_mana_cost: bool,
    pub require_exile: bool,
}

impl CastSourceEffect {
    pub fn new() -> Self {
        Self {
            without_paying_mana_cost: false,
            require_exile: false,
        }
    }

    pub fn without_paying_mana_cost(mut self) -> Self {
        self.without_paying_mana_cost = true;
        self
    }

    pub fn require_exile(mut self) -> Self {
        self.require_exile = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CipherEffect;

impl CipherEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UnearthEffect;

impl UnearthEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NinjutsuCostEffect;

impl NinjutsuCostEffect {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NinjutsuEffect;

impl NinjutsuEffect {
    pub fn new() -> Self {
        Self
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

#[derive(Debug, Clone, PartialEq)]
pub enum VoteChoice {
    NamedOptions(Vec<composition::VoteOption>),
    Objects {
        filter: crate::target::ObjectFilter,
        count: crate::effect::ChoiceCount,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoteEffect {
    pub choice: VoteChoice,
    pub controller_extra_votes: u32,
    pub controller_optional_extra_votes: u32,
}

impl VoteEffect {
    pub fn named(
        options: Vec<composition::VoteOption>,
        controller_extra_votes: u32,
        controller_optional_extra_votes: u32,
    ) -> Self {
        Self {
            choice: VoteChoice::NamedOptions(options),
            controller_extra_votes,
            controller_optional_extra_votes,
        }
    }

    pub fn objects(
        filter: crate::target::ObjectFilter,
        count: crate::effect::ChoiceCount,
        controller_extra_votes: u32,
        controller_optional_extra_votes: u32,
    ) -> Self {
        Self {
            choice: VoteChoice::Objects { filter, count },
            controller_extra_votes,
            controller_optional_extra_votes,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GainLifeEffect {
    pub amount: crate::effect::Value,
    pub player: crate::target::ChooseSpec,
}

impl GainLifeEffect {
    pub fn new(amount: impl Into<crate::effect::Value>, player: crate::target::ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegenerateEffect {
    pub target: crate::target::ChooseSpec,
    pub duration: crate::effect::Until,
}

impl RegenerateEffect {
    pub fn new(target: crate::target::ChooseSpec, duration: crate::effect::Until) -> Self {
        Self { target, duration }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaFromCommanderColorIdentityEffect {
    pub amount: crate::effect::Value,
    pub player: crate::target::PlayerFilter,
}

impl AddManaFromCommanderColorIdentityEffect {
    pub fn new(
        amount: impl Into<crate::effect::Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<crate::effect::Value>) -> Self {
        Self::new(amount, crate::target::PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfAnyColorEffect {
    pub amount: crate::effect::Value,
    pub player: crate::target::PlayerFilter,
    pub available_colors: Option<Vec<crate::color::Color>>,
}

impl AddManaOfAnyColorEffect {
    pub fn new(
        amount: impl Into<crate::effect::Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            available_colors: None,
        }
    }

    pub fn restricted(
        amount: impl Into<crate::effect::Value>,
        player: crate::target::PlayerFilter,
        available_colors: Vec<crate::color::Color>,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
            available_colors: Some(available_colors),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaOfAnyOneColorEffect {
    pub amount: crate::effect::Value,
    pub player: crate::target::PlayerFilter,
}

impl AddManaOfAnyOneColorEffect {
    pub fn new(
        amount: impl Into<crate::effect::Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    pub fn you(amount: impl Into<crate::effect::Value>) -> Self {
        Self::new(amount, crate::target::PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddManaEffect {
    pub mana: Vec<crate::mana::ManaSymbol>,
    pub player: crate::target::PlayerFilter,
}

impl AddManaEffect {
    pub fn new(mana: Vec<crate::mana::ManaSymbol>, player: crate::target::PlayerFilter) -> Self {
        Self { mana, player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CombatDamagePreventionTarget {
    All,
    Players,
    You,
    From(crate::target::ChooseSpec),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllCombatDamageEffect {
    pub target: CombatDamagePreventionTarget,
    pub until: crate::effect::Until,
}

impl PreventAllCombatDamageEffect {
    pub fn new(target: CombatDamagePreventionTarget, until: crate::effect::Until) -> Self {
        Self { target, until }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmitKeywordActionEffect {
    pub action: crate::events::KeywordActionKind,
    pub amount: u32,
}

impl EmitKeywordActionEffect {
    pub fn new(action: crate::events::KeywordActionKind, amount: u32) -> Self {
        Self { action, amount }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenownEffect {
    pub amount: u32,
}

impl RenownEffect {
    pub const fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BolsterEffect {
    pub amount: u32,
}

impl BolsterEffect {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlipEffect {
    pub target: crate::target::ChooseSpec,
}

impl FlipEffect {
    pub fn new(target: crate::target::ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllDamageEffect {
    pub filter: crate::target::ObjectFilter,
    pub until: crate::effect::Until,
}

impl PreventAllDamageEffect {
    pub fn matching(filter: crate::target::ObjectFilter, until: crate::effect::Until) -> Self {
        Self { filter, until }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoseTheGameEffect {
    pub player: crate::target::PlayerFilter,
}

impl LoseTheGameEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(crate::target::PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateEmblemEffect {
    pub emblem: crate::effect::EmblemDescription,
}

impl CreateEmblemEffect {
    pub fn new(emblem: crate::effect::EmblemDescription) -> Self {
        Self { emblem }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscardHandEffect {
    pub player: crate::target::PlayerFilter,
}

impl DiscardHandEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(crate::target::PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAnyCountersAmongEffect {
    pub count: u32,
    pub filter: crate::target::ObjectFilter,
    pub counter_type: Option<crate::object::CounterType>,
}

impl RemoveAnyCountersAmongEffect {
    pub fn new(count: u32, filter: crate::target::ObjectFilter) -> Self {
        Self {
            count,
            filter,
            counter_type: None,
        }
    }

    pub fn with_counter_type(mut self, counter_type: Option<crate::object::CounterType>) -> Self {
        self.counter_type = counter_type;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCardTypeEffect {
    pub chooser: crate::target::PlayerFilter,
    pub options: Vec<crate::types::CardType>,
}

impl ChooseCardTypeEffect {
    pub fn new(chooser: crate::target::PlayerFilter, options: Vec<crate::types::CardType>) -> Self {
        Self { chooser, options }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleObjectsIntoLibraryEffect {
    pub target: crate::target::ChooseSpec,
    pub player: crate::target::PlayerFilter,
}

impl ShuffleObjectsIntoLibraryEffect {
    pub fn new(target: crate::target::ChooseSpec, player: crate::target::PlayerFilter) -> Self {
        Self { target, player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeTextBoxesEffect {
    pub target: crate::target::ChooseSpec,
}

impl ExchangeTextBoxesEffect {
    pub fn new(target: crate::target::ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyCountersEffect {
    pub count: crate::effect::Value,
    pub player: crate::target::PlayerFilter,
}

impl EnergyCountersEffect {
    pub fn new(
        count: impl Into<crate::effect::Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<crate::effect::Value>) -> Self {
        Self::new(count, crate::target::PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoverEffect {
    pub count: crate::effect::Value,
    pub player: crate::target::PlayerFilter,
}

impl DiscoverEffect {
    pub fn new(
        count: impl Into<crate::effect::Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    pub fn you(count: impl Into<crate::effect::Value>) -> Self {
        Self::new(count, crate::target::PlayerFilter::You)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetBasePowerToughnessEffect {
    pub target: crate::target::ChooseSpec,
    pub power: crate::effect::Value,
    pub toughness: crate::effect::Value,
    pub duration: crate::effect::Until,
}

impl SetBasePowerToughnessEffect {
    pub fn new(
        target: crate::target::ChooseSpec,
        power: impl Into<crate::effect::Value>,
        toughness: impl Into<crate::effect::Value>,
        duration: crate::effect::Until,
    ) -> Self {
        Self {
            target,
            power: power.into(),
            toughness: toughness.into(),
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CantEffect {
    pub restriction: crate::effect::Restriction,
    pub duration: crate::effect::Until,
}

impl CantEffect {
    pub fn new(restriction: crate::effect::Restriction, duration: crate::effect::Until) -> Self {
        Self {
            restriction,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtraTurnEffect {
    pub player: crate::target::PlayerFilter,
}

impl ExtraTurnEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtraTurnAfterNextTurnEffect {
    pub player: crate::target::PlayerFilter,
}

impl ExtraTurnAfterNextTurnEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModifyPowerToughnessForEachEffect {
    pub target: crate::target::ChooseSpec,
    pub power_per: i32,
    pub toughness_per: i32,
    pub count: crate::effect::Value,
    pub duration: crate::effect::Until,
}

impl ModifyPowerToughnessForEachEffect {
    pub fn new(
        target: crate::target::ChooseSpec,
        power_per: i32,
        toughness_per: i32,
        count: crate::effect::Value,
        duration: crate::effect::Until,
    ) -> Self {
        Self {
            target,
            power_per,
            toughness_per,
            count,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastTaggedEffect {
    pub tag: crate::tag::TagKey,
    pub player: crate::target::PlayerFilter,
    pub allow_land: bool,
    pub as_copy: bool,
    pub without_paying_mana_cost: bool,
    pub cost_reduction: Option<crate::mana::ManaCost>,
}

impl CastTaggedEffect {
    pub fn new(tag: crate::tag::TagKey, player: crate::target::PlayerFilter) -> Self {
        Self {
            tag,
            player,
            allow_land: false,
            as_copy: false,
            without_paying_mana_cost: false,
            cost_reduction: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutTaggedRemainderOnLibraryBottomEffect {
    pub tag: crate::tag::TagKey,
    pub keep_tagged: Option<crate::tag::TagKey>,
    pub order: consult_helpers::LibraryBottomOrder,
    pub player: crate::target::PlayerFilter,
}

impl PutTaggedRemainderOnLibraryBottomEffect {
    pub fn new(
        tag: impl Into<crate::tag::TagKey>,
        keep_tagged: Option<crate::tag::TagKey>,
        order: consult_helpers::LibraryBottomOrder,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            tag: tag.into(),
            keep_tagged,
            order,
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveUpToAnyCountersEffect {
    pub max_count: crate::effect::Value,
    pub target: crate::target::ChooseSpec,
}

impl RemoveUpToAnyCountersEffect {
    pub fn new(
        max_count: impl Into<crate::effect::Value>,
        target: crate::target::ChooseSpec,
    ) -> Self {
        Self {
            max_count: max_count.into(),
            target,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringObjectEffect {
    pub tag: crate::tag::TagKey,
}

impl TagTriggeringObjectEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SacrificePlayerEffect {
    pub filter: crate::target::ObjectFilter,
    pub count: crate::effect::Value,
    pub player: crate::target::PlayerFilter,
}

impl SacrificePlayerEffect {
    pub fn new(
        filter: crate::target::ObjectFilter,
        count: impl Into<crate::effect::Value>,
        player: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            filter,
            count: count.into(),
            player,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachToEffect {
    pub target: crate::target::ChooseSpec,
}

impl AttachToEffect {
    pub fn new(target: crate::target::ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachObjectsEffect {
    pub objects: crate::target::ChooseSpec,
    pub target: crate::target::ChooseSpec,
}

impl AttachObjectsEffect {
    pub fn new(objects: crate::target::ChooseSpec, target: crate::target::ChooseSpec) -> Self {
        Self { objects, target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevealTopEffect {
    pub player: crate::target::PlayerFilter,
    pub tag: Option<crate::tag::TagKey>,
}

impl RevealTopEffect {
    pub fn new(player: crate::target::PlayerFilter, tag: Option<crate::tag::TagKey>) -> Self {
        Self { player, tag }
    }

    pub fn tagged(player: crate::target::PlayerFilter, tag: impl Into<crate::tag::TagKey>) -> Self {
        Self::new(player, Some(tag.into()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagAttachedToSourceEffect {
    pub tag: crate::tag::TagKey,
}

impl TagAttachedToSourceEffect {
    pub fn new(tag: impl Into<crate::tag::TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardToBattlefieldEffect {
    pub target: crate::target::ChooseSpec,
    pub tapped: bool,
}

impl ReturnFromGraveyardToBattlefieldEffect {
    pub fn new(target: crate::target::ChooseSpec, tapped: bool) -> Self {
        Self { target, tapped }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardToHandEffect {
    pub target: crate::target::ChooseSpec,
    pub random: bool,
}

impl ReturnFromGraveyardToHandEffect {
    pub fn new(target: crate::target::ChooseSpec, random: bool) -> Self {
        Self { target, random }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PutOntoBattlefieldEffect {
    pub target: crate::target::ChooseSpec,
    pub tapped: bool,
    pub controller: crate::target::PlayerFilter,
}

impl PutOntoBattlefieldEffect {
    pub fn new(
        target: crate::target::ChooseSpec,
        tapped: bool,
        controller: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            target,
            tapped,
            controller,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleLibraryEffect {
    pub player: crate::target::PlayerFilter,
}

impl ShuffleLibraryEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MayMoveToZoneEffect {
    pub target: crate::target::ChooseSpec,
    pub zone: crate::zone::Zone,
    pub decider: crate::target::PlayerFilter,
}

impl MayMoveToZoneEffect {
    pub fn new(
        target: crate::target::ChooseSpec,
        zone: crate::zone::Zone,
        decider: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            target,
            zone,
            decider,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LookAtTopCardsEffect {
    pub player: crate::target::PlayerFilter,
    pub count: crate::effect::Value,
    pub tag: crate::tag::TagKey,
}

impl LookAtTopCardsEffect {
    pub fn new(
        player: crate::target::PlayerFilter,
        count: crate::effect::Value,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            player,
            count,
            tag: tag.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCardNameEffect {
    pub chooser: crate::target::PlayerFilter,
    pub filter: Option<crate::target::ObjectFilter>,
    pub tag: crate::tag::TagKey,
}

impl ChooseCardNameEffect {
    pub fn new(
        chooser: crate::target::PlayerFilter,
        filter: Option<crate::target::ObjectFilter>,
        tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            chooser,
            filter,
            tag: tag.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlPlayerEffect {
    pub player: crate::target::PlayerFilter,
    pub start: crate::game_state::PlayerControlStart,
    pub duration: crate::game_state::PlayerControlDuration,
}

impl ControlPlayerEffect {
    pub fn new(
        player: crate::target::PlayerFilter,
        start: crate::game_state::PlayerControlStart,
        duration: crate::game_state::PlayerControlDuration,
    ) -> Self {
        Self {
            player,
            start,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantEffect {
    pub grantable: crate::grant::Grantable,
    pub target: crate::target::ChooseSpec,
    pub duration: crate::grant::GrantDuration,
}

impl GrantEffect {
    pub fn new(
        grantable: crate::grant::Grantable,
        target: crate::target::ChooseSpec,
        duration: crate::grant::GrantDuration,
    ) -> Self {
        Self {
            grantable,
            target,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantBySpecEffect {
    pub spec: crate::grant::GrantSpec,
    pub player: crate::target::PlayerFilter,
    pub duration: crate::grant::GrantDuration,
}

impl GrantBySpecEffect {
    pub fn new(
        spec: crate::grant::GrantSpec,
        player: crate::target::PlayerFilter,
        duration: crate::grant::GrantDuration,
    ) -> Self {
        Self {
            spec,
            player,
            duration,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveAllCountersEffect {
    pub from: crate::target::ChooseSpec,
    pub to: crate::target::ChooseSpec,
}

impl MoveAllCountersEffect {
    pub fn new(from: crate::target::ChooseSpec, to: crate::target::ChooseSpec) -> Self {
        Self { from, to }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProliferateEffect {
    pub count: crate::effect::Value,
}

impl ProliferateEffect {
    pub fn new(count: impl Into<crate::effect::Value>) -> Self {
        Self {
            count: count.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonstrosityEffect {
    pub n: crate::effect::Value,
}

impl MonstrosityEffect {
    pub fn new(n: impl Into<crate::effect::Value>) -> Self {
        Self { n: n.into() }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransformEffect {
    pub target: crate::target::ChooseSpec,
}

impl TransformEffect {
    pub fn new(target: crate::target::ChooseSpec) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatProcessEffect {
    pub effects: Vec<crate::effect::Effect>,
    pub condition: crate::effect::EffectId,
    pub predicate: crate::effect::EffectPredicate,
}

impl RepeatProcessEffect {
    pub fn new(
        effects: Vec<crate::effect::Effect>,
        condition: crate::effect::EffectId,
        predicate: crate::effect::EffectPredicate,
    ) -> Self {
        Self {
            effects,
            condition,
            predicate,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatEffectsEffect {
    pub count: crate::effect::Value,
    pub effects: Vec<crate::effect::Effect>,
}

impl RepeatEffectsEffect {
    pub fn new(count: crate::effect::Value, effects: Vec<crate::effect::Effect>) -> Self {
        Self { count, effects }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RearrangeLookedCardsInLibraryEffect {
    pub tag: crate::tag::TagKey,
    pub chooser: crate::target::PlayerFilter,
    pub count: crate::effect::ChoiceCount,
}

impl RearrangeLookedCardsInLibraryEffect {
    pub fn new(
        tag: impl Into<crate::tag::TagKey>,
        chooser: crate::target::PlayerFilter,
        count: crate::effect::ChoiceCount,
    ) -> Self {
        Self {
            tag: tag.into(),
            chooser,
            count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNewTargetsEffect {
    pub from_effect: crate::effect::EffectId,
    pub may: bool,
    pub chooser: Option<crate::target::PlayerFilter>,
}

impl ChooseNewTargetsEffect {
    pub fn may_for_player(
        from_effect: crate::effect::EffectId,
        chooser: crate::target::PlayerFilter,
    ) -> Self {
        Self {
            from_effect,
            may: true,
            chooser: Some(chooser),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileInsteadOfGraveyardEffect {
    pub player: crate::target::PlayerFilter,
}

impl ExileInsteadOfGraveyardEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WinTheGameEffect {
    pub player: crate::target::PlayerFilter,
}

impl WinTheGameEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsultTopOfLibraryEffect {
    pub player: crate::target::PlayerFilter,
    pub mode: consult_helpers::LibraryConsultMode,
    pub filter: crate::target::ObjectFilter,
    pub stop_rule: ConsultTopOfLibraryStopRule,
    pub all_tag: crate::tag::TagKey,
    pub match_tag: crate::tag::TagKey,
}

impl ConsultTopOfLibraryEffect {
    pub fn new(
        player: crate::target::PlayerFilter,
        mode: consult_helpers::LibraryConsultMode,
        filter: crate::target::ObjectFilter,
        stop_rule: ConsultTopOfLibraryStopRule,
        all_tag: impl Into<crate::tag::TagKey>,
        match_tag: impl Into<crate::tag::TagKey>,
    ) -> Self {
        Self {
            player,
            mode,
            filter,
            stop_rule,
            all_tag: all_tag.into(),
            match_tag: match_tag.into(),
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LibraryBottomOrder {
        Random,
        ChooserChooses,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LibraryConsultMode {
        Reveal,
        Exile,
    }
}

pub mod mana {
    pub use ironsmith_core::{
        AddManaOfChosenColorEffect, AddManaOfImprintedColorsEffect, AddScaledManaEffect,
    };
}
