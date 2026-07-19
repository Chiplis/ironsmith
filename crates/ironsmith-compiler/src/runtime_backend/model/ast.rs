use crate::ConditionExpr;
use crate::color::ColorSet;
use crate::cost::{OptionalCostRef, TotalCost};
use crate::effect::{ChoiceCount, EffectId, Until, Value};
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::RedirectNextTimeDamageDestinationAst;
use crate::object::{AuraAttachmentFilter, CounterType};
use crate::static_abilities::StaticAbility;
use crate::tag::TagKey;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter, SourceReferenceSurface};
use crate::types::{CardType, Subtype, SubtypeFamily, Supertype};
use crate::zone::Zone;

use super::super::{
    ClashOpponentAst, ControlDurationAst, DamageBySpec, ExchangeValueAst, ExtraTurnAnchorAst,
    FutureZoneReplacementCausePolicyAst, IfResultPredicate, KeywordAction, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, RetargetModeAst,
    ReturnControllerAst, SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst,
    ZoneReplacementDurationAst,
};
use super::semantic::ParsedAbility;
use crate::runtime_backend::GrantedAbilityAst;

#[path = "ast/effects.rs"]
mod effects;
pub(crate) use effects::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StaticAbilityAst {
    Static(StaticAbility),
    KeywordAction(KeywordAction),
    PregameRevealFromOpeningHand {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        one_shot: bool,
        first_spell_of_game: bool,
        effect_before_timing: bool,
        display: String,
    },
    LoseGameReplacement {
        effects: Vec<EffectAst>,
        optional: bool,
        display: String,
    },
    ConditionalStaticAbility {
        ability: Box<StaticAbilityAst>,
        condition: ConditionExpr,
    },
    LabeledConditionalStaticAbility {
        ability: Box<StaticAbilityAst>,
        condition: ConditionExpr,
        label: String,
    },
    ConditionalKeywordAction {
        action: KeywordAction,
        condition: ConditionExpr,
    },
    WithSetQuantifierSurface {
        ability: Box<StaticAbilityAst>,
        surface: ironsmith_core::SetQuantifierSurface,
    },
    GrantStaticAbility {
        filter: ObjectFilter,
        ability: Box<StaticAbilityAst>,
        condition: Option<ConditionExpr>,
    },
    GrantKeywordAction {
        filter: ObjectFilter,
        action: KeywordAction,
        condition: Option<ConditionExpr>,
    },
    RemoveStaticAbility {
        filter: ObjectFilter,
        ability: Box<StaticAbilityAst>,
    },
    RemoveKeywordAction {
        filter: ObjectFilter,
        action: KeywordAction,
        mode: ironsmith_core::AbilityLossMode,
    },
    AttachedStaticAbilityGrant {
        ability: Box<StaticAbilityAst>,
        display: String,
        condition: Option<ConditionExpr>,
    },
    AttachedKeywordActionGrant {
        action: KeywordAction,
        display: String,
        condition: Option<ConditionExpr>,
    },
    AttachedChosenLandwalkGrant {
        snow: bool,
        display: String,
        condition: Option<ConditionExpr>,
    },
    EquipmentKeywordActionsGrant {
        actions: Vec<KeywordAction>,
    },
    GrantObjectAbility {
        filter: ObjectFilter,
        ability: ParsedAbility,
        display: String,
        condition: Option<ConditionExpr>,
    },
    AttachedObjectAbilityGrant {
        ability: ParsedAbility,
        display: String,
        condition: Option<ConditionExpr>,
    },
    SoulbondSharedObjectAbility {
        ability: ParsedAbility,
    },
    AttachmentRestriction {
        filter: AuraAttachmentFilter,
        display: String,
    },
}

impl From<StaticAbility> for StaticAbilityAst {
    fn from(ability: StaticAbility) -> Self {
        Self::Static(ability)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerIntroSurfaceAst {
    When,
    Whenever,
    At,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TriggerSpec {
    WithIntro {
        intro: TriggerIntroSurfaceAst,
        trigger: Box<TriggerSpec>,
    },
    StateBased {
        condition: PredicateAst,
        display: String,
    },
    ThisAttacks,
    ThisAttacksPlayerWhoControlsAtLeast {
        count: u32,
        filter: ObjectFilter,
    },
    ThisAttacksWithNOthers {
        other_count: u32,
        display_subject: Option<String>,
        other_filter: Option<ObjectFilter>,
    },
    ThisAttacksWithExactlyNOthers(u32),
    ThisAttacksAndIsntBlocked,
    ThisAttacksWhileSaddled,
    Attacks(ObjectFilter),
    AttacksAndIsntBlocked(ObjectFilter),
    AttacksWhileSaddled(ObjectFilter),
    AttacksOneOrMore(ObjectFilter),
    PlayersAttackedOneOrMore(PlayerFilter),
    AttacksOneOrMoreWithMinTotal {
        filter: ObjectFilter,
        min_total_attackers: u32,
    },
    AttacksOneOrMoreWithExactTotal {
        filter: ObjectFilter,
        total_attackers: u32,
    },
    AttacksAlone(ObjectFilter),
    AttacksYouOrPlaneswalkerYouControl(ObjectFilter),
    AttacksYouOrPlaneswalkerYouControlOneOrMore(ObjectFilter),
    ThisBlocks,
    ThisBlocksObject(ObjectFilter),
    Blocks(ObjectFilter),
    BlocksOneOrMore(ObjectFilter),
    ThisBecomesBlocked,
    BecomesBlocked(ObjectFilter),
    ThisBecomesBlockedByObject(ObjectFilter),
    ThisDies,
    ThisDiesOrIsExiled,
    ThisExiledFromBattlefieldDuringCostOfAbilityWithMarker {
        marker: String,
    },
    ThisLeavesBattlefield,
    ThisLeavesBattlefieldWithSurface(crate::target::SourceReferenceSurface),
    ThisMutates,
    ThisBecomesMonstrous,
    ThisBecomesTapped,
    PermanentBecomesTapped(ObjectFilter),
    ThisBecomesUntapped,
    ThisTurnedFaceUp,
    TurnedFaceUp(ObjectFilter),
    ThisBecomesTargeted,
    BecomesTargeted(ObjectFilter),
    ThisBecomesTargetedBySpell(ObjectFilter),
    ThisBecomesTargetedByStackObject(ObjectFilter),
    BecomesTargetedByStackObject {
        target: ObjectFilter,
        stack_object: ObjectFilter,
    },
    BecomesTargetedBySourceController {
        target: ObjectFilter,
        source_controller: PlayerFilter,
    },
    PlayerOrObjectBecomesTargetedBySourceController {
        player: PlayerFilter,
        object: ObjectFilter,
        source_controller: PlayerFilter,
    },
    ThisDealsDamage,
    ThisDealsDamageToPlayer {
        player: PlayerFilter,
        amount: Option<crate::filter::Comparison>,
    },
    ThisDealsDamageTo(ObjectFilter),
    ThisDealsCombatDamage,
    ThisDealsCombatDamageTo(ObjectFilter),
    DealsDamage {
        source: ObjectFilter,
        source_surface: crate::triggers::DamageSourceSurface,
    },
    DealsDamageTo {
        source: ObjectFilter,
        target: ObjectFilter,
        source_surface: crate::triggers::DamageSourceSurface,
    },
    DealsDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
        source_surface: crate::triggers::DamageSourceSurface,
    },
    DealsNoncombatDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
        source_surface: crate::triggers::DamageSourceSurface,
    },
    DealsCombatDamage(ObjectFilter),
    DealsCombatDamageTo {
        source: ObjectFilter,
        target: ObjectFilter,
    },
    PlayerPlaysLand {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    PlayerGivesGift(PlayerFilter),
    PlayerSearchesLibrary(PlayerFilter),
    PlayerShufflesLibrary {
        player: PlayerFilter,
        caused_by_effect: bool,
        source_controller_shuffles: bool,
    },
    PlayerTapsForMana {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    PlayerRollsResult {
        player: PlayerFilter,
        result: u32,
    },
    PlayerRollsHighestNaturalResult {
        player: PlayerFilter,
    },
    PlayerRollsDie {
        player: PlayerFilter,
        one_or_more: bool,
    },
    PlayerCoinFlipResult {
        player: PlayerFilter,
        won: bool,
    },
    AbilityActivated {
        activator: PlayerFilter,
        filter: ObjectFilter,
        non_mana_only: bool,
        loyalty_only: bool,
        activation_cost_has_tap: Option<bool>,
    },
    AbilityTriggered {
        another: bool,
    },
    ThisIsDealtDamage,
    ThisIsDealtCombatDamage,
    IsDealtDamage(ObjectFilter),
    IsDealtCombatDamage(ObjectFilter),
    YouGainLife,
    YouGainLifeDuringTurn(PlayerFilter),
    PlayerLosesLife(PlayerFilter),
    PlayersLoseLifeOneOrMore(PlayerFilter),
    OpponentsEachLoseExactLife {
        amount: u32,
    },
    PlayerLosesGame(PlayerFilter),
    PlayerLosesLifeDuringTurn {
        player: PlayerFilter,
        during_turn: PlayerFilter,
    },
    YouDrawCard,
    PlayerDrawsCard(PlayerFilter),
    PlayerDrawsCardNotDuringTurn {
        player: PlayerFilter,
        during_turn: PlayerFilter,
    },
    PlayerDrawsCardExceptFirstInDrawStep(PlayerFilter),
    PlayerDrawsNthCardEachTurn {
        player: PlayerFilter,
        card_number: u32,
    },
    PlayerDrawsNumberedCardsEachTurn {
        player: PlayerFilter,
        card_numbers: Vec<u32>,
    },
    PlayerDiscardsCard {
        player: PlayerFilter,
        filter: Option<ObjectFilter>,
        cause_controller: Option<PlayerFilter>,
        effect_like_only: bool,
        one_or_more: bool,
    },
    PlayerRevealsCard {
        player: PlayerFilter,
        filter: ObjectFilter,
        from_source: bool,
    },
    PlayerSacrifices {
        player: PlayerFilter,
        filter: ObjectFilter,
        one_or_more: bool,
    },
    TokensCreated {
        player: PlayerFilter,
        filter: ObjectFilter,
        one_or_more: bool,
    },
    LeavesBattlefield(ObjectFilter),
    Dies(ObjectFilter),
    DiesOneOrMore(ObjectFilter),
    DiesDuringTurn {
        filter: ObjectFilter,
        one_or_more: bool,
        during_turn: PlayerFilter,
    },
    HauntedCreatureDies,
    PutIntoGraveyard(ObjectFilter),
    PutIntoGraveyardOneOrMore(ObjectFilter),
    PutIntoGraveyardFromZone {
        filter: ObjectFilter,
        from: Zone,
        one_or_more: bool,
    },
    PutIntoExileFromZones {
        filter: ObjectFilter,
        from: Vec<Zone>,
        one_or_more: bool,
        during_turn: Option<PlayerFilter>,
        cause_filter: Option<crate::events::cause::CauseFilter>,
    },
    CardsLeaveYourGraveyard {
        filter: ObjectFilter,
        one_or_more: bool,
        during_your_turn: bool,
    },
    CounterPutOn {
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        source_controller: Option<PlayerFilter>,
        one_or_more: bool,
        include_players: bool,
    },
    CounterRemovedFrom {
        filter: ObjectFilter,
        one_or_more: bool,
        caused_by_source: bool,
    },
    PlayerGetsCounters {
        player: PlayerFilter,
        counter_type: Option<CounterType>,
        one_or_more: bool,
    },
    DiesCreatureDealtDamageByThisTurn {
        victim: ObjectFilter,
        damager: DamageBySpec,
    },
    SpellCast {
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        timing: Option<ironsmith_core::TriggerTimingRestriction>,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    },
    SpellCopied {
        filter: Option<ObjectFilter>,
        copier: PlayerFilter,
    },
    SpellCountered {
        filter: Option<ObjectFilter>,
        controller: PlayerFilter,
    },
    EntersBattlefield {
        filter: ObjectFilter,
        cause_filter: Option<crate::events::cause::CauseFilter>,
    },
    EntersBattlefieldOneOrMore {
        filter: ObjectFilter,
        cause_filter: Option<crate::events::cause::CauseFilter>,
        origin_condition: Option<ironsmith_core::trigger_model::ZoneChangeOriginCondition>,
    },
    EntersBattlefieldFromZone {
        filter: ObjectFilter,
        from: Zone,
        owner: Option<PlayerFilter>,
        one_or_more: bool,
        cause_filter: Option<crate::events::cause::CauseFilter>,
    },
    EntersBattlefieldTapped {
        filter: ObjectFilter,
        cause_filter: Option<crate::events::cause::CauseFilter>,
    },
    EntersBattlefieldUntapped {
        filter: ObjectFilter,
        cause_filter: Option<crate::events::cause::CauseFilter>,
    },
    BeginningOfUpkeep(PlayerFilter),
    BeginningOfDrawStep(PlayerFilter),
    BeginningOfCombat(PlayerFilter),
    BeginningOfEndStep(PlayerFilter),
    BeginningOfTheEndStep,
    BeginningOfPrecombatMain(PlayerFilter),
    BeginningOfPostcombatMain(PlayerFilter),
    DayNightChanged,
    ThisEntersBattlefield,
    ThisEntersBattlefieldWithSurface {
        surface: crate::target::SourceReferenceSurface,
        subject_number: ironsmith_core::trigger_model::TriggerSubjectNumber,
    },
    ThisEntersBattlefieldFromZone {
        subject_filter: ObjectFilter,
        from: Zone,
        owner: Option<PlayerFilter>,
    },
    ThisTransforms {
        destination_name: Option<String>,
    },
    ThisTransformsWithSurface {
        surface: crate::target::SourceReferenceSurface,
        destination_name: Option<String>,
    },
    ThisDealsCombatDamageToPlayer {
        player: PlayerFilter,
    },
    DealsCombatDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
    },
    DealsCombatDamageToPlayerOneOrMore {
        source: ObjectFilter,
        player: PlayerFilter,
    },
    YouCastThisSpell,
    KeywordAction {
        action: crate::events::KeywordActionKind,
        player: PlayerFilter,
        source_filter: Option<ObjectFilter>,
    },
    KeywordActionTaggedObject {
        action: crate::events::KeywordActionKind,
        player: PlayerFilter,
        source_filter: ObjectFilter,
        object_tag: crate::tag::TagKey,
        object_filter: ObjectFilter,
        during_your_main_phase: bool,
    },
    KeywordActionFromSource {
        action: crate::events::KeywordActionKind,
        player: PlayerFilter,
    },
    WinsClash {
        player: PlayerFilter,
    },
    Expend {
        player: PlayerFilter,
        amount: u32,
    },
    SagaChapter(Vec<u32>),
    FinalChapterAbilityResolved(ObjectFilter),
    Either(Box<TriggerSpec>, Box<TriggerSpec>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PredicateAst {
    ItIsNight,
    FirstCombatPhaseOfTurn,
    SourceControllersMainPhase,
    ItIsLandCard,
    ItIsSoulbondPaired,
    SourceChosenOption(String),
    ItMatches(ObjectFilter),
    /// The implicit object matched this filter immediately before its zone change.
    ///
    /// This is distinct from `ItMatches`: phrases such as "if it was a creature"
    /// use last-known information and must not become true from the object's
    /// characteristics in its new zone.
    ItMatchedLastKnown(ObjectFilter),
    TargetMatches(ObjectFilter),
    TaggedMatches(TagKey, ObjectFilter),
    TaggedWasCast(TagKey),
    EnchantedPermanentAttackedThisTurn,
    TargetObjectsHaveDifferentColorSets,
    PlayerTaggedObjectMatches {
        player: PlayerAst,
        tag: TagKey,
        filter: ObjectFilter,
    },
    PlayerTaggedObjectEnteredBattlefieldThisTurn {
        player: PlayerAst,
        tag: TagKey,
    },
    PlayerControls {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerHasAtLeast {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsExactly {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerHasAtLeastWithDifferentPowers {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsOrHasCardInGraveyard {
        player: PlayerAst,
        control_filter: ObjectFilter,
        graveyard_filter: ObjectFilter,
    },
    PlayerOwnsCardNamedInZones {
        player: PlayerAst,
        name: String,
        zones: Vec<Zone>,
    },
    PlayerControlsNo {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerControlsMost {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerControlsMoreThanEachOtherPlayer {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    AnOpponentHasFewerThanPlayer {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerControlsMoreThanYou {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerLifeAtMostHalfStartingLifeTotal {
        player: PlayerAst,
    },
    PlayerLifeLessThanHalfStartingLifeTotal {
        player: PlayerAst,
    },
    PlayerHasLessLifeThanYou {
        player: PlayerAst,
    },
    PlayerHasMoreLifeThanYou {
        player: PlayerAst,
    },
    PlayerHasNoOpponentWithMoreLifeThan {
        player: PlayerAst,
    },
    PlayerHasMoreLifeThanEachOtherPlayer {
        player: PlayerAst,
    },
    CountParity {
        count: crate::static_abilities::AnthemCountExpression,
        even: bool,
        display: Option<String>,
    },
    PlayerIsMonarch {
        player: PlayerAst,
    },
    PlayerHasInitiative {
        player: PlayerAst,
    },
    PlayerHasCitysBlessing {
        player: PlayerAst,
    },
    SourceIsRingBearer {
        player: PlayerAst,
    },
    PlayerRingTemptedThisGameOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCompletedDungeon {
        player: PlayerAst,
        dungeon_name: Option<String>,
    },
    PlayerTappedLandForManaThisTurn {
        player: PlayerAst,
    },
    PlayerGainedLifeThisTurnOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerHadLandEnterBattlefieldThisTurn {
        player: PlayerAst,
    },
    PlayerDescendedThisTurn {
        player: PlayerAst,
    },
    PlayerControlsBasicLandTypesAmongLandsOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerHasCardTypesInGraveyardOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandOrFewer {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandAtTurnStartOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandAtTurnStartOrFewer {
        player: PlayerAst,
        count: u32,
    },
    PlayerHasMoreCardsInHandThanYou {
        player: PlayerAst,
    },
    PlayerHasMoreCardsInHandThanEachOtherPlayer {
        player: PlayerAst,
    },
    PlayerHasPoisonCountersOrMore {
        player: PlayerAst,
        count: u32,
    },
    VoteOptionGetsMoreVotes {
        option: String,
    },
    SecretChoicesMatch,
    VoteOptionGetsMoreVotesOrTied {
        option: String,
    },
    NoVoteObjectsMatched {
        filter: ObjectFilter,
    },
    PlayerCastSpellsThisTurnOrMore {
        player: PlayerAst,
        count: u32,
    },
    OpponentLostLifeThisTurn,
    AnyPlayerLostLifeThisTurnOrMore {
        count: u32,
    },
    OpponentWasDealtDamageThisTurn,
    YouHaveNoCardsInHand,
    PlayerWouldDrawCard {
        player: PlayerAst,
    },
    PlayerWouldProliferate {
        player: PlayerAst,
    },
    PlayerWouldBeginExtraTurn {
        player: PlayerAst,
    },
    SourceIsTapped,
    SourceIsEquipped,
    SourceIsEnchanted,
    SourceIsSaddled,
    SourceIsRenowned,
    SourceCrewedByExactly {
        count: u32,
        filter: ObjectFilter,
    },
    SourceMatches(ObjectFilter),
    /// The battlefield object this Aura or Equipment source is attached to
    /// matches the filter at the time the trigger is checked.
    AttachedToSourceMatches(ObjectFilter),
    TriggeringObjectHadToAttackThisCombat,

    SourceHasNoCounter(CounterType),
    TriggeringObjectHadNoCounter(CounterType),
    TriggeringObjectHadCounterAtLeast {
        counter_type: CounterType,
        count: u32,
    },
    SourceHasCounterAtLeast {
        counter_type: CounterType,
        count: u32,
        surface: crate::SourceCounterThresholdSurface,
    },
    SourceHasCountersAtLeast(u32),
    SourceHasAttachmentsMatching {
        filter: ObjectFilter,
        comparison: crate::effect::Comparison,
        display: String,
    },
    SourcePowerAtLeast(u32),
    SourceDealtCombatDamageToPlayerThisTurn,
    PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
        player: PlayerAst,
        subtype: Subtype,
    },
    SourceAttackedThisTurn,
    SourceSuspected,
    SourceCameUnderYourControlThisTurn,
    SourceAttackedOrBlockedThisTurn,
    SourceIsInZone(Zone),
    YourTurn,
    YouAttackedWithExactlyNOtherCreaturesThisCombat(u32),
    CreatureDiedThisTurn,
    CreatureDiedThisTurnOrMore(u32),
    CreatureDealtDamageBySourceDiedThisTurn {
        victim: ObjectFilter,
        damager: DamageBySpec,
        count: u32,
    },
    CreatureCardPutIntoYourGraveyardThisTurn,
    PermanentLeftBattlefieldThisTurn,
    NonlandPermanentLeftBattlefieldThisTurn,
    SpellWasWarpedThisTurn,
    PermanentLeftBattlefieldUnderYourControlThisTurn,
    ObjectEnteredBattlefieldThisTurn(ObjectFilter),
    ObjectEnteredBattlefieldLastTurn(ObjectFilter),
    ObjectPutIntoGraveyardFromBattlefieldThisTurn(ObjectFilter),
    YouHaveFullParty,
    YouAttackedThisTurn,
    YouAttackedWithNOrMoreCreaturesThisTurn(u32),
    SourceWasCast,
    ThisSpellEscaped,
    NoSpellsWereCastLastTurn,
    ThisSpellWasKicked,
    ThisSpellPaidLabel(OptionalCostRef),
    TargetWasKicked,
    ThisAbilityResolvedThisTurnExactly(u32),
    TargetSpellCastOrderThisTurn(u32),
    TargetSpellControllerIsPoisoned,
    TargetSpellNoManaSpentToCast,
    YouControlMoreCreaturesThanTargetSpellController,
    TargetIsBlocked,
    TargetHasGreatestPowerAmongCreatures,
    TargetManaValueLteColorsSpentToCastThisSpell,
    ManaSpentToCastThisSpellAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    TriggeringSpellManaSpentToCastAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    ColoredManaSpentToCastThisSpellAtLeast(u32),
    TriggeringSpellColoredManaSpentToCastAtLeast(u32),
    SnowManaOfAnySpellColorSpentToCastThisSpell,
    SameColorManaSpentToCastThisSpellAtLeast(u32),
    ThisSpellWasCastFromZone(Zone),
    ThisSpellWasCastFromNonHand,
    TurnHistory(TurnHistoryPredicateAst),
    ValueComparison {
        left: Value,
        operator: crate::effect::ValueComparisonOperator,
        right: Value,
    },
    Not(Box<PredicateAst>),
    And(Box<PredicateAst>, Box<PredicateAst>),
    Or(Box<PredicateAst>, Box<PredicateAst>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TurnHistoryPredicateAst {
    SpellsCastLastTurnAtLeast(u32),
    SourceCrewedByAtLeast {
        count: u32,
        filter: ObjectFilter,
    },
    SourceWasCast {
        surface: SourceReferenceSurface,
    },
    SourceWasCastByController {
        surface: SourceReferenceSurface,
    },
    SourceWasKicked {
        surface: SourceReferenceSurface,
    },
    SourceEnteredBattlefieldThisTurn {
        surface: SourceReferenceSurface,
    },
    SourceAttackedThisTurn {
        surface: SourceReferenceSurface,
    },
    TriggeringObjectEnlistedThisCombat,
    TriggeringObjectWasCast,
    TriggeringObjectWasCastFromZone(Zone),
    PlayerPlayedLandThisTurn(PlayerAst),
    TriggeringObjectDied,
    PlayerPlayedCardFromZoneThisTurn {
        player: PlayerAst,
        zone: Zone,
    },
    TriggeringPlayerAttackedControllerLastTurn,
    PlayerLostLifeLastTurn(PlayerAst),
    TriggeringPlayersTurn {
        definite_player: bool,
    },
    ControllerTeamGainedLifeThisTurn,
    TriggeringObjectsNoneWereCastOrNoManaSpent,
    ManaFromSourceSpentOnTriggeringAction {
        source_filter: ObjectFilter,
    },
    AllPlayersLifeAtMost(i32),
    AnotherOpponentControlsPotentialTarget {
        filter: ObjectFilter,
    },
    TriggeringAttackerBlockers {
        required: ObjectFilter,
        required_count: u32,
        prohibited: ObjectFilter,
    },
    TriggeringAbilityIsManaAbility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PredicateReferenceAntecedent {
    SourceObject,
}

impl PredicateAst {
    pub(crate) fn reference_antecedent(&self) -> Option<PredicateReferenceAntecedent> {
        match self {
            PredicateAst::SourceChosenOption(_)
            | PredicateAst::SourceIsTapped
            | PredicateAst::SourceIsEquipped
            | PredicateAst::SourceIsEnchanted
            | PredicateAst::SourceIsSaddled
            | PredicateAst::SourceIsRenowned
            | PredicateAst::SourceCrewedByExactly { .. }
            | PredicateAst::SourceMatches(_)
            | PredicateAst::AttachedToSourceMatches(_)
            | PredicateAst::SourceHasNoCounter(_)
            | PredicateAst::SourceHasCounterAtLeast { .. }
            | PredicateAst::SourceHasCountersAtLeast(_)
            | PredicateAst::SourceHasAttachmentsMatching { .. }
            | PredicateAst::SourcePowerAtLeast(_)
            | PredicateAst::SourceAttackedThisTurn
            | PredicateAst::SourceSuspected
            | PredicateAst::SourceCameUnderYourControlThisTurn
            | PredicateAst::SourceAttackedOrBlockedThisTurn
            | PredicateAst::SourceIsInZone(_)
            | PredicateAst::SourceWasCast
            | PredicateAst::ThisSpellEscaped
            | PredicateAst::ThisSpellWasKicked
            | PredicateAst::ThisSpellPaidLabel(_)
            | PredicateAst::ThisSpellWasCastFromZone(_)
            | PredicateAst::ThisSpellWasCastFromNonHand
            | PredicateAst::TurnHistory(
                TurnHistoryPredicateAst::SourceCrewedByAtLeast { .. }
                | TurnHistoryPredicateAst::SourceWasCast { .. }
                | TurnHistoryPredicateAst::SourceWasCastByController { .. }
                | TurnHistoryPredicateAst::SourceWasKicked { .. }
                | TurnHistoryPredicateAst::SourceEnteredBattlefieldThisTurn { .. }
                | TurnHistoryPredicateAst::SourceAttackedThisTurn { .. },
            ) => Some(PredicateReferenceAntecedent::SourceObject),
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => left
                .reference_antecedent()
                .or_else(|| right.reference_antecedent()),
            PredicateAst::Not(inner) => inner.reference_antecedent(),
            _ => None,
        }
    }

    pub(crate) fn establishes_source_object_antecedent(&self) -> bool {
        matches!(
            self.reference_antecedent(),
            Some(PredicateReferenceAntecedent::SourceObject)
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubjectVerbRoleAst {
    Actor,
    AffectedPlayer,
    Chooser,
    LibraryOwner,
}

#[derive(Clone, PartialEq)]
pub(crate) struct SubjectVerbSubjectAst {
    pub(crate) role: SubjectVerbRoleAst,
    pub(crate) player: PlayerAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReturnAsAuraAst {
    pub(crate) attachment_filter: ObjectFilter,
    pub(crate) remove_all_abilities: bool,
    pub(crate) granted_abilities: Vec<GrantedAbilityAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EmblemDescriptionAst {
    pub(crate) text: String,
    pub(crate) abilities: Vec<EmblemAbilityAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EmblemAbilityAst {
    Static(Vec<StaticAbilityAst>),
    Activated(ParsedAbility),
    Triggered {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        trigger_limit_condition: Option<ConditionExpr>,
    },
}

#[derive(Clone, PartialEq)]
pub(crate) enum SubjectVerbActionAst {
    Draw {
        count: Value,
    },
    DrawForEachTaggedMatching {
        tag: TagKey,
        filter: ObjectFilter,
    },
    LoseLife {
        amount: Value,
    },
    PayLife {
        amount: Value,
    },
    GainLife {
        amount: Value,
    },
    RevealHand,
    Mill {
        count: Value,
    },
    Scry {
        count: Value,
    },
    Surveil {
        count: Value,
    },
    Proliferate {
        count: Value,
    },
    Investigate {
        count: Value,
    },
    Incubate {
        amount: Value,
        count: Value,
    },
    Learn,
    EmitKeywordAction {
        action: crate::events::KeywordActionKind,
        amount: u32,
    },
    Amass {
        subtype: Option<Subtype>,
        amount: Value,
    },
    Bolster {
        amount: u32,
    },
    Support {
        amount: u32,
    },
    Adapt {
        amount: u32,
    },
    Monstrosity {
        amount: Value,
    },
    Discover {
        count: Value,
    },
    Fateseal {
        count: Value,
    },
    Populate {
        count: Value,
        enters_tapped: bool,
        enters_attacking: bool,
        has_haste: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        next_end_step_player: PlayerFilter,
        exile_at_end_of_combat: bool,
        sacrifice_at_end_of_combat: bool,
    },
    Explore {
        target: TargetAst,
    },
    Endure {
        target: TargetAst,
        amount: Value,
    },
    Exploit,
    Connive {
        target: TargetAst,
        count: Value,
    },
    ConniveIterated,
    OpenAttraction,
    ManifestTopCardOfLibrary,
    CloakTopCardOfLibrary,
    ManifestCardFromHand,
    ManifestDread,
    Earthbend {
        counters: u32,
    },
    Behold {
        subtype: Subtype,
        count: u32,
    },
    Fight {
        creature1: TargetAst,
        creature2: TargetAst,
    },
    FightIterated {
        creature2: TargetAst,
    },
    Clash {
        opponent: ClashOpponentAst,
    },
    FlipCoin,
    RollDie {
        sides: u32,
        die_text: Option<String>,
    },
    RollDiceChooseResult {
        count: u32,
        sides: u32,
        die_text: Option<String>,
    },
    ShuffleHandAndGraveyardIntoLibrary,
    ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary,
    ShuffleGraveyardIntoLibrary,
    ReorderGraveyard,
    ChooseColor,
    ChooseCardType {
        options: Vec<CardType>,
    },
    ChooseNamedOption {
        options: Vec<String>,
    },
    ChooseCreatureType {
        excluded_subtypes: Vec<Subtype>,
    },
    ChooseLandType {
        exclude_basic: bool,
    },
    ChooseCardName {
        filter: Option<ObjectFilter>,
        tag: TagKey,
    },
    ChoosePlayer {
        filter: PlayerFilter,
        tag: TagKey,
        random: bool,
        exclude_previous_choices: usize,
    },
    NoteLifeTotal,
    ChooseSpellCastHistory {
        cast_by: PlayerAst,
        filter: ObjectFilter,
        tag: TagKey,
    },
    AddMana {
        mana: Vec<ManaSymbol>,
    },
    AddManaScaled {
        mana: Vec<ManaSymbol>,
        amount: Value,
    },
    AddManaAnyColor {
        amount: Value,
        available_colors: Option<Vec<crate::color::Color>>,
        distinct_colors: bool,
    },
    AddManaAnyOneColor {
        amount: Value,
    },
    AddManaChosenColor {
        amount: Value,
        fixed_option: Option<crate::color::Color>,
    },
    AddManaFromLandCouldProduce {
        amount: Value,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
        mana_type_source: crate::effects::ManaTypeSource,
    },
    AddManaColorsAmong {
        filter: ObjectFilter,
    },
    AddManaCommanderIdentity {
        amount: Value,
    },
    ExchangeLifeTotals {
        player2: PlayerAst,
    },
    ExchangeTextBoxes {
        target: TargetAst,
    },
    ExchangeZones {
        zone1: Zone,
        zone2: Zone,
    },
    PutRestOnBottomOfLibrary,
    DontLoseThisManaAsStepsAndPhasesEndThisTurn,
    ExchangeValues {
        left: ExchangeValueAst,
        right: ExchangeValueAst,
        duration: Until,
    },
    ExchangeControl {
        filter: ObjectFilter,
        count: u32,
        shared_type: Option<SharedTypeConstraintAst>,
    },
    ExchangeControlHeterogeneous {
        permanent1: TargetAst,
        permanent2: TargetAst,
        shared_type: Option<SharedTypeConstraintAst>,
    },
    Attach {
        object: TargetAst,
        target: TargetAst,
    },
    Unattach {
        object: TargetAst,
    },
    Enchant {
        filter: AuraAttachmentFilter,
    },
    ExileWhenSourceLeaves {
        target: TargetAst,
    },
    SacrificeSourceWhenLeaves {
        target: TargetAst,
    },
    RegisterZoneReplacement {
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        library_placement: Option<ironsmith_core::ZoneReplacementLibraryPlacement>,
        duration: ZoneReplacementDurationAst,
        optional: bool,
        choice_description: Option<String>,
        counters: Vec<(CounterType, u32)>,
    },
    RegisterFutureZoneReplacement {
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
        cause_policy: FutureZoneReplacementCausePolicyAst,
        link_exiled_to_source: bool,
    },
    RegisterDrawReplacement {
        player: PlayerFilter,
        replacement_effects: Vec<EffectAst>,
        duration: ZoneReplacementDurationAst,
    },
    RegisterManaReplacement {
        source_filter: ObjectFilter,
        replacement_mana: Vec<ManaSymbol>,
        mode: crate::effects::ReplacementApplyMode,
    },
    RegisterDamagedBySourceZoneReplacement {
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
    },
    RegisterEnterUnderControlReplacement {
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    },
    ExileInsteadOfGraveyardThisTurn,
    ControlCombatChoicesThisTurn {
        attackers: bool,
        blockers: bool,
        this_combat: bool,
    },
    GainControl {
        target: TargetAst,
        duration: Until,
        condition: Option<ConditionExpr>,
        source_reference_surface: Option<SourceReferenceSurface>,
    },
    RevealTop,
    ExileTopOfLibrary {
        count: Value,
        tags: Vec<TagKey>,
        accumulated_tags: Vec<TagKey>,
        face_down: bool,
    },
    RevealTagged {
        tag: TagKey,
    },
    /// Put the chosen/iterated objects onto the battlefield under a resolved
    /// controller. Inside a `ForEachTagged`, `TargetAst::Tagged(IT_TAG)` lowers
    /// to `ChooseSpec::Iterated`; otherwise the tagged collection is used.
    /// Lowers to `Effect::put_onto_battlefield`.
    PutOntoBattlefield {
        target: TargetAst,
        tapped: bool,
        controller: ReturnControllerAst,
        cloak: bool,
        shuffle_before: bool,
    },
    RevealCardsFromHand {
        count: ChoiceCount,
        count_value: Option<Value>,
        tag: TagKey,
    },
    LookAtTopCards {
        count: Value,
        tag: TagKey,
        reveal: bool,
    },
    LookAtObjects {
        filter: ObjectFilter,
    },
    LookAtTarget {
        target: TargetAst,
    },
    MayMoveToZone {
        target: TargetAst,
        zone: Zone,
    },
    AdditionalLandPlays {
        count: Value,
        duration: Until,
    },
    ExtraTurnAfterTurn {
        anchor: ExtraTurnAnchorAst,
    },
    ReorderTopOfLibrary {
        tag: TagKey,
    },
    AddManaImprintedColors,
    ShuffleLibrary,
    ShuffleObjectsIntoLibrary {
        target: TargetAst,
        all: bool,
        owner_library_destination: bool,
    },
    GrantProtectionChoice {
        target: TargetAst,
        chooser: PlayerAst,
        allow_colorless: bool,
        allow_artifacts: bool,
    },
    PreventAllCombatDamage {
        duration: Until,
    },
    AssignNoCombatDamage {
        source: TargetAst,
        duration: Until,
    },
    PreventAllCombatDamageFromSource {
        duration: Until,
        source: TargetAst,
        source_would_deal_surface: bool,
    },
    PreventAllCombatDamageFromSourceFilter {
        duration: Until,
        source_filter: ObjectFilter,
        excluded_source_target: Option<TargetAst>,
    },
    PreventAllCombatDamageToPlayers {
        duration: Until,
    },
    PreventAllCombatDamageToYou {
        duration: Until,
    },
    PreventNextTimeDamage {
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
        reflect_damage_to_source_controller: bool,
        follow_up_effects: Vec<EffectAst>,
    },
    PreventDamage {
        amount: Value,
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
        protect_you_and_permanents_you_control: bool,
        follow_up_effects: Vec<EffectAst>,
    },
    PreventAllDamageToTarget {
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
        source_choice_shares_activation_mana_color: bool,
        source_target: Option<TargetAst>,
    },
    PreventAllDamageToTargetFromSourceFilter {
        target: TargetAst,
        duration: Until,
        source_filter: ObjectFilter,
    },
    PreventAllDamageFromSourceFilter {
        duration: Until,
        source_filter: ObjectFilter,
    },
    PreventDamageToTargetPutCounters {
        amount: Option<Value>,
        target: TargetAst,
        duration: Until,
        counter_type: CounterType,
    },
    PreventDamageEach {
        amount: Value,
        filter: ObjectFilter,
        duration: Until,
    },
    CopySpell {
        target: TargetAst,
        /// Copy every matching stack object instead of choosing one match.
        ///
        /// This is intentionally part of the typed action rather than inferred
        /// from the target filter: `copy target spell` and `copy all spells`
        /// may otherwise lower to the same `ObjectFilter` and lose the printed
        /// set quantifier before runtime execution.
        all_matches: bool,
        count: Value,
        player: PlayerAst,
        may_choose_new_targets: bool,
        choose_new_target_singular: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
    },
    CopySpellForEachTarget {
        target: TargetAst,
        object_filter: Option<ObjectFilter>,
        player_filter: Option<PlayerFilter>,
        player: PlayerAst,
        exclude_current_targets: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
    },
    ScaleXValue {
        target: TargetAst,
        multiplier: u32,
    },
    PutTaggedRemainderOnBottomOfLibrary {
        tag: TagKey,
        keep_tagged: Option<TagKey>,
        order: LibraryBottomOrderAst,
        player: PlayerAst,
        surface: ironsmith_core::LibraryRemainderSurface,
    },
    /// Moves every object tagged `tag` that is NOT also in the `keep_tagged`
    /// group to `zone`, preserving each object's controller. Lowers to
    /// `for_each_tagged(tag, [conditional(in keep_tagged, [], [move iterated to
    /// zone])])`, keeping the iterated reference internal to lowering (no bare
    /// `it` surfaces). The graveyard/exile analog of
    /// `PutTaggedRemainderOnBottomOfLibrary`.
    PutTaggedRemainderInZone {
        tag: TagKey,
        keep_tagged: TagKey,
        zone: Zone,
    },
    CastTagged {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        as_copy: bool,
        without_paying_mana_cost: bool,
        cost_reduction: Option<ManaCost>,
    },
    GrantPlayTaggedUntilEndOfTurn {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        while_on_top_of_library: bool,
    },
    GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
        tag: TagKey,
        player: PlayerAst,
    },
    GrantPlayTaggedUntilYourNextTurn {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        until_next_end_step: bool,
    },
    GrantPlayTaggedForAsLongAsExiled {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        filter: Option<ObjectFilter>,
        /// Restrict the persistent permission to turns in which this counter
        /// type was put on the ability source.
        during_turns_counter_put_on_source: Option<crate::object::CounterType>,
    },
    GrantPlayTaggedForAsLongAsYouControlSource {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
    },
    ReturnToBattlefield {
        target: TargetAst,
        tapped: bool,
        transformed: bool,
        converted: bool,
        controller: ReturnControllerAst,
        count_value: Option<Value>,
        as_aura: Option<ReturnAsAuraAst>,
        top_only: bool,
    },
    ReturnAllToBattlefield {
        filter: ObjectFilter,
        tapped: bool,
        face_down: bool,
        controller: ReturnControllerAst,
        verb_surface: ironsmith_core::MoveToZoneVerbSurface,
    },
    ExileUntilSourceLeaves {
        target: TargetAst,
        face_down: bool,
        all: bool,
        explicit_return_surface: bool,
    },
    MoveToZone {
        target: TargetAst,
        /// The target is selected from the first matching object in its ordered source zone.
        source_top_only: bool,
        zone: Zone,
        to_top: bool,
        library_order: Option<LibraryBottomOrderAst>,
        library_order_chooser: PlayerAst,
        verb_surface: ironsmith_core::MoveToZoneVerbSurface,
        target_plural_surface: bool,
        destination_player_surface: Option<PlayerAst>,
        destination_player_reference_surface:
            Option<ironsmith_core::DestinationPlayerReferenceSurface>,
        exiled_with_source_surface: Option<ironsmith_core::ExiledWithSourceMoveSurface>,
        battlefield_controller: ReturnControllerAst,
        battlefield_tapped: bool,
        battlefield_attacking: bool,
        battlefield_attack_target_player_or_planeswalker_controlled_by: Option<PlayerAst>,
        battlefield_face_down: bool,
        attached_to: Option<TargetAst>,
        all: bool,
    },
    MoveToLibraryTopOrBottomChoice {
        target: TargetAst,
    },
    TargetOnly {
        target: TargetAst,
        explicit_declaration: bool,
    },
    TagMatchingObjects {
        filter: ObjectFilter,
        zones: Vec<Zone>,
        tag: TagKey,
    },
    Pump {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    SetBasePowerToughness {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    BecomeBasePtCreature {
        power: Value,
        toughness: Value,
        target: TargetAst,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
        subtype_families: Vec<SubtypeFamily>,
        colors: Option<ColorSet>,
        abilities: Vec<StaticAbility>,
        granted_abilities: Vec<GrantedAbilityAst>,
        preserve_other_types: bool,
        type_retention_surface: Option<ironsmith_core::TypeRetentionSurface>,
        animation_pt_surface: Option<ironsmith_core::AnimationPtSurface>,
        animation_duration_surface: Option<ironsmith_core::AnimationDurationSurface>,
        duration: Until,
    },
    SetBasePower {
        power: Value,
        target: TargetAst,
        duration: Until,
    },
    PumpForEach {
        power_per: i32,
        toughness_per: i32,
        target: TargetAst,
        count: Value,
        duration: Until,
    },
    PumpAll {
        filter: ObjectFilter,
        power: Value,
        toughness: Value,
        duration: Until,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    PumpByLastEffect {
        power: i32,
        toughness: i32,
        target: TargetAst,
        duration: Until,
    },
    AddCardTypes {
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    },
    SetCardTypes {
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    },
    RemoveCardTypes {
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    },
    AddSubtypes {
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    },
    /// "becomes a Bird Giant" without "in addition": replaces the object's
    /// creature subtypes (CR 205.1b) instead of adding to them.
    SetCreatureSubtypes {
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    },
    BecomeSaddledUntilEndOfTurn {
        target: TargetAst,
    },
    AddColors {
        target: TargetAst,
        colors: ColorSet,
        duration: Until,
    },
    AddAllSubtypesOfFamily {
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    },
    RemoveAllSubtypesOfFamily {
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    },
    BecomeAuraEnchantment {
        target: TargetAst,
        attachment_filter: ObjectFilter,
        granted_abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    BecomeBasicLandType {
        target: TargetAst,
        subtype: Subtype,
        duration: Until,
    },
    SetColors {
        target: TargetAst,
        colors: ColorSet,
        duration: Until,
    },
    MakeColorless {
        target: TargetAst,
        duration: Until,
    },
    BecomeBasicLandTypeChoice {
        target: TargetAst,
        duration: Until,
    },
    BecomeCreatureTypeChoice {
        target: TargetAst,
        duration: Until,
        excluded_subtypes: Vec<Subtype>,
    },
    BecomeColorChoice {
        target: TargetAst,
        duration: Until,
    },
    BecomeCopy {
        target: TargetAst,
        source: TargetAst,
        duration: Until,
        preserve_source_abilities: bool,
        name_override: Option<String>,
        name_override_surface: Option<SourceReferenceSurface>,
        add_supertypes: Vec<Supertype>,
        remove_supertypes: Vec<Supertype>,
        add_card_types: Vec<CardType>,
        set_card_types: Vec<CardType>,
        add_subtypes: Vec<Subtype>,
        set_subtypes: Vec<Subtype>,
        granted_abilities: Vec<GrantedAbilityAst>,
        set_base_power_toughness: Option<(Value, Value)>,
        copy_exception_surface: Option<String>,
    },
    GrantAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
        /// CR 611.2c normally fixes the affected set when a resolving effect
        /// starts. Some rules effects instead create a continuous rule for a
        /// filter for the stated duration and must also affect later entrants.
        lock_filter_at_resolution: bool,
    },
    RemoveAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    GrantAbilitiesChoiceAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    GrantAbilitiesToTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    GrantToTarget {
        target: TargetAst,
        grantable: crate::grant::Grantable,
        duration: crate::grant::GrantDuration,
    },
    GrantBySpec {
        spec: crate::grant::GrantSpec,
        player: PlayerAst,
        duration: crate::grant::GrantDuration,
    },
    RemoveAbilitiesFromTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    GrantAbilitiesChoiceToTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    ConsultTopOfLibrary {
        player: PlayerAst,
        mode: LibraryConsultModeAst,
        filter: ObjectFilter,
        stop_rule: LibraryConsultStopRuleAst,
        max_exposed: Option<Value>,
        all_tag: TagKey,
        match_tag: TagKey,
    },
    SearchLibrary {
        filter: ObjectFilter,
        destination: Zone,
        chooser: PlayerAst,
        player: PlayerAst,
        search_mode: crate::effect::SearchSelectionMode,
        reveal: bool,
        shuffle: bool,
        count: ChoiceCount,
        count_value: Option<Value>,
        library_position_from_top: Option<Value>,
        result_reference_surface: crate::effect::SearchResultReferenceSurface,
        tapped: bool,
    },
    Cant {
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        condition: Option<crate::ConditionExpr>,
    },
    CreateTokenCopy {
        object: ObjectRefAst,
        count: Value,
        player: PlayerAst,
        enters_tapped: bool,
        enters_attacking: bool,
        attack_target_player_or_planeswalker_controlled_by: Option<PlayerAst>,
        half_power_toughness_round_up: bool,
        has_haste: bool,
        exile_at_end_of_combat: bool,
        sacrifice_at_next_end_step: bool,
        sacrifice_at_next_end_step_ability_text: Option<String>,
        exile_at_next_end_step: bool,
        next_end_step_player: PlayerFilter,
        set_colors: Option<ColorSet>,
        set_card_types: Option<Vec<CardType>>,
        set_subtypes: Option<Vec<Subtype>>,
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        removed_supertypes: Vec<Supertype>,
        set_base_power_toughness: Option<(i32, i32)>,
        granted_abilities: Vec<StaticAbility>,
    },
    CreateTokenCopyFromSource {
        source: TargetAst,
        count: Value,
        player: PlayerAst,
        enters_tapped: bool,
        enters_attacking: bool,
        attack_target_player_or_planeswalker_controlled_by: Option<PlayerAst>,
        half_power_toughness_round_up: bool,
        has_haste: bool,
        exile_at_end_of_combat: bool,
        sacrifice_at_next_end_step: bool,
        sacrifice_at_next_end_step_ability_text: Option<String>,
        exile_at_next_end_step: bool,
        next_end_step_player: PlayerFilter,
        set_colors: Option<ColorSet>,
        set_card_types: Option<Vec<CardType>>,
        set_subtypes: Option<Vec<Subtype>>,
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        removed_supertypes: Vec<Supertype>,
        set_base_power_toughness: Option<(i32, i32)>,
        granted_abilities: Vec<StaticAbility>,
    },
    CreateTokenWithMods {
        name: String,
        definition: crate::runtime_backend::token_definition::TokenDefinitionSpec,
        count: Value,
        dynamic_power_toughness: Option<(Value, Value)>,
        player: PlayerAst,
        /// The source text explicitly used `you` as the create-action actor.
        /// This does not participate in controller resolution.
        actor_surface_explicit: bool,
        attached_to: Option<TargetAst>,
        tapped: bool,
        attacking: bool,
        exile_at_end_of_combat: bool,
        sacrifice_at_end_of_combat: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        next_end_step_player: PlayerFilter,
        granted_abilities: Vec<GrantedAbilityAst>,
        ability_presentation: Option<ironsmith_core::TokenAbilityPresentation>,
    },
    RedirectNextDamageFromSourceToTarget {
        amount: Value,
        protected_target: Option<TargetAst>,
        destination: RedirectNextTimeDamageDestinationAst,
        destination_target: Option<TargetAst>,
    },
    RedirectNextTimeDamageToSource {
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
        destination: RedirectNextTimeDamageDestinationAst,
        destination_target: Option<TargetAst>,
        all_this_turn: bool,
    },
    RedirectAllDamageThisTurnBySourceToSourceController {
        source: TargetAst,
    },
    RedirectAllDamageThisTurnToTarget {
        player_filter: PlayerFilter,
        object_filter: ObjectFilter,
        target: TargetAst,
    },
    Meld {
        result_name: String,
        enters_tapped: bool,
        enters_attacking: bool,
    },
    SearchLibrarySlotsToHand {
        slots: Vec<SearchLibrarySlotAst>,
        destination: Zone,
        reveal: bool,
        progress_tag: TagKey,
    },
    RetargetStackObject {
        target: TargetAst,
        mode: RetargetModeAst,
        require_change: bool,
    },
    GrantAbilityToSource {
        ability: ParsedAbility,
        duration: Until,
    },
    DealDamage {
        amount: Value,
        target: TargetAst,
        unpreventable: bool,
    },
    TurnFaceUp {
        target: TargetAst,
    },
    DealDamageEach {
        amount: Value,
        filter: ObjectFilter,
    },
    DealDamageEqualToPower {
        source: TargetAst,
        amount: Value,
        target: TargetAst,
        unpreventable: bool,
    },
    DealDistributedDamage {
        amount: Value,
        target: TargetAst,
        source: TargetAst,
        chooser: PlayerFilter,
    },
    Tap {
        target: TargetAst,
    },
    Untap {
        target: TargetAst,
    },
    TapAll {
        filter: ObjectFilter,
    },
    UntapAll {
        filter: ObjectFilter,
    },
    TapOrUntap {
        target: TargetAst,
    },
    TapOrUntapAll {
        tap_filter: ObjectFilter,
        untap_filter: ObjectFilter,
    },
    PhaseOut {
        target: TargetAst,
        duration: crate::effects::PhaseOutDuration,
        source_surface: Option<SourceReferenceSurface>,
    },
    PhaseOutAll {
        filter: ObjectFilter,
        duration: crate::effects::PhaseOutDuration,
        source_surface: Option<SourceReferenceSurface>,
    },
    PhaseIn {
        target: TargetAst,
    },
    PhaseInAll {
        filter: ObjectFilter,
    },
    Transform {
        target: TargetAst,
    },
    Convert {
        target: TargetAst,
    },
    Destroy {
        target: TargetAst,
        no_regeneration: bool,
    },
    DestroyAll {
        filter: ObjectFilter,
        no_regeneration: bool,
    },
    DestroyAllOfChosenColor {
        filter: ObjectFilter,
        no_regeneration: bool,
    },
    DestroyAllAttachedTo {
        filter: ObjectFilter,
        target: TargetAst,
    },
    ExileAllAttachedTo {
        filter: ObjectFilter,
        target: TargetAst,
        face_down: bool,
    },
    Exile {
        target: TargetAst,
        face_down: bool,
        /// The target is selected from the first matching object in its ordered source zone.
        source_top_only: bool,
    },
    ExileAll {
        filter: ObjectFilter,
        face_down: bool,
    },
    LookAtHand {
        target: TargetAst,
    },
    Counter {
        target: TargetAst,
    },
    CounterUnlessPays {
        target: TargetAst,
        cost: TotalCost,
    },
    PutCounters {
        counter_type: CounterType,
        count: Value,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
        distributed: bool,
    },
    PutCounterChoice {
        counter_types: Vec<CounterType>,
        count: Value,
        mode_texts: Vec<String>,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
    },
    PutOrRemoveCounters {
        put_counter_type: CounterType,
        put_count: Value,
        remove_counter_type: CounterType,
        remove_count: Value,
        put_mode_text: String,
        remove_mode_text: String,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
    },
    PutCountersAll {
        counter_type: CounterType,
        count: Value,
        filter: ObjectFilter,
    },
    RemoveUpToAnyCounters {
        amount: Value,
        target: TargetAst,
        counter_type: Option<CounterType>,
        up_to: bool,
        all_of_them: bool,
    },
    MoveAllCounters {
        from: TargetAst,
        to: TargetAst,
    },
    MoveOneCounter {
        from: TargetAst,
        to: TargetAst,
    },
    ForEachCounterKindPutOrRemove {
        target: TargetAst,
        all_kinds: bool,
        fixed_counter_type: Option<CounterType>,
        optional_action: bool,
    },
    PutCounterOfChosenKind {
        target: TargetAst,
    },
    ReturnToHand {
        target: TargetAst,
        random: bool,
        destination_player_surface: Option<PlayerAst>,
        exiled_with_source_surface: Option<ironsmith_core::ExiledWithSourceMoveSurface>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
        set_reference_surface: Option<String>,
    },
    ReturnAllToHand {
        filter: ObjectFilter,
        destination_player_surface: Option<PlayerAst>,
        exiled_with_source_surface: Option<ironsmith_core::ExiledWithSourceMoveSurface>,
    },
    ReturnAllToHandOfChosenColor {
        filter: ObjectFilter,
    },
    MoveToLibraryNthFromTop {
        target: TargetAst,
        position: Value,
    },
    DoubleCountersOnEach {
        counter_type: Option<CounterType>,
        filter: ObjectFilter,
    },
    DoubleCountersOnTarget {
        counter_type: Option<CounterType>,
        target: TargetAst,
    },
    RemoveCountersAll {
        amount: Value,
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        up_to: bool,
    },
    PutSticker {
        target: TargetAst,
        action: crate::events::KeywordActionKind,
    },
    SwitchPowerToughness {
        target: TargetAst,
        duration: Until,
    },
    ScalePowerToughnessAll {
        filter: ObjectFilter,
        power: bool,
        toughness: bool,
        multiplier: i32,
        duration: Until,
    },
    Discard {
        count: Value,
        random: bool,
        any_number: bool,
        filter: Option<ObjectFilter>,
        tag: Option<TagKey>,
    },
    DiscardHand,
    PoisonCounters {
        count: Value,
    },
    EnergyCounters {
        count: Value,
    },
    ExperienceCounters {
        count: Value,
    },
    TicketCounters {
        count: Value,
    },
    PayEnergy {
        amount: Value,
    },
    PayAnyEnergy {
        min_amount: u32,
    },
    PayAnyLife {
        min_amount: u32,
    },
    PayMana {
        cost: ManaCost,
        /// Typed value for a printed `{X}` payment whose X is defined by the
        /// surrounding Oracle sentence rather than chosen by the player.
        x_value: Option<Value>,
    },
    DoubleManaPool,
    EmptyManaPool,
    SetLifeTotal {
        amount: Value,
    },
    EndTurn,
    EndCombatPhase,
    SkipTurn,
    SkipCombatPhases,
    SkipNextCombatPhaseThisTurn,
    SkipMainPhasesThisTurn,
    SkipCombatPhasesThisTurn,
    SkipDrawStep,
    AdditionalPhases {
        phases: Vec<crate::effects::AdditionalPhase>,
    },
    PlayFromGraveyardUntilEot,
    ControlPlayer {
        player: PlayerFilter,
        duration: ControlDurationAst,
    },
    ReduceNextSpellCostThisTurn {
        filter: ObjectFilter,
        reduction: ManaCost,
    },
    ReduceMatchingSpellCostThisTurn {
        filter: ObjectFilter,
        reduction: Value,
        duration: Until,
    },
    GrantNextSpellAbilityThisTurn {
        filter: ObjectFilter,
        ability: GrantedAbilityAst,
    },
    RingTemptsYou,
    VentureIntoDungeon {
        undercity_if_no_active: bool,
    },
    BecomeMonarch,
    TakeInitiative,
    CreateEmblem {
        emblem: EmblemDescriptionAst,
    },
    LoseGame,
    WinGame,
    Detain {
        target: TargetAst,
    },
    Goad {
        target: TargetAst,
        duration: Until,
    },
    Suspect {
        target: TargetAst,
    },
    ClearSuspected {
        target: Option<TargetAst>,
    },
    HealDamage {
        target: TargetAst,
        amount: Option<Value>,
    },
    RemoveFromCombat {
        target: TargetAst,
    },
    Flip {
        target: TargetAst,
    },
    Regenerate {
        target: TargetAst,
        follow_up_effects: Vec<EffectAst>,
    },
    RegenerateAll {
        filter: ObjectFilter,
    },
    Sacrifice {
        filter: ObjectFilter,
        count: u32,
        target: Option<TargetAst>,
        /// The object phrase selected one member of a referenced collection
        /// ("one of them") rather than referring to a known singleton ("it").
        one_of_referenced_set: bool,
    },
    SacrificeAll {
        filter: ObjectFilter,
    },
}

#[derive(Clone, PartialEq)]
pub(crate) struct SubjectVerbEffectAst {
    pub(crate) subject: SubjectVerbSubjectAst,
    pub(crate) action: SubjectVerbActionAst,
}

impl std::fmt::Debug for SubjectVerbRoleAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Actor => "Actor",
            Self::AffectedPlayer => "AffectedPlayer",
            Self::Chooser => "Chooser",
            Self::LibraryOwner => "LibraryOwner",
        };
        f.write_str(label)
    }
}

impl std::fmt::Debug for SubjectVerbSubjectAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubjectVerbSubject")
            .field("role", &self.role)
            .field("player", &self.player)
            .finish()
    }
}

impl std::fmt::Debug for SubjectVerbActionAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draw { count } => f.debug_tuple("Draw").field(count).finish(),
            Self::DrawForEachTaggedMatching { tag, filter } => f
                .debug_struct("DrawForEachTaggedMatching")
                .field("tag", tag)
                .field("filter", filter)
                .finish(),
            Self::LoseLife { amount } => f.debug_tuple("LoseLife").field(amount).finish(),
            Self::PayLife { amount } => f.debug_tuple("PayLife").field(amount).finish(),
            Self::GainLife { amount } => f.debug_tuple("GainLife").field(amount).finish(),
            Self::RevealHand => f.write_str("RevealHand"),
            Self::Mill { count } => f.debug_tuple("Mill").field(count).finish(),
            Self::Scry { count } => f.debug_tuple("Scry").field(count).finish(),
            Self::Surveil { count } => f.debug_tuple("Surveil").field(count).finish(),
            Self::Proliferate { count } => f.debug_tuple("Proliferate").field(count).finish(),
            Self::Investigate { count } => f.debug_tuple("Investigate").field(count).finish(),
            Self::Incubate { amount, count } => f
                .debug_struct("Incubate")
                .field("amount", amount)
                .field("count", count)
                .finish(),
            Self::Learn => f.write_str("Learn"),
            Self::EmitKeywordAction { action, amount } => f
                .debug_struct("EmitKeywordAction")
                .field("action", action)
                .field("amount", amount)
                .finish(),
            Self::Amass { subtype, amount } => f
                .debug_struct("Amass")
                .field("subtype", subtype)
                .field("amount", amount)
                .finish(),
            Self::Bolster { amount } => f.debug_tuple("Bolster").field(amount).finish(),
            Self::Support { amount } => f.debug_tuple("Support").field(amount).finish(),
            Self::Adapt { amount } => f.debug_tuple("Adapt").field(amount).finish(),
            Self::Monstrosity { amount } => f.debug_tuple("Monstrosity").field(amount).finish(),
            Self::Discover { count } => f.debug_tuple("Discover").field(count).finish(),
            Self::Fateseal { count } => f.debug_tuple("Fateseal").field(count).finish(),
            Self::Populate { count, .. } => f.debug_tuple("Populate").field(count).finish(),
            Self::Explore { target } => f.debug_tuple("Explore").field(target).finish(),
            Self::Endure { target, amount } => f
                .debug_struct("Endure")
                .field("target", target)
                .field("amount", amount)
                .finish(),
            Self::Exploit => f.write_str("Exploit"),
            Self::Connive { target, count } => f
                .debug_struct("Connive")
                .field("target", target)
                .field("count", count)
                .finish(),
            Self::ConniveIterated => f.write_str("ConniveIterated"),
            Self::OpenAttraction => f.write_str("OpenAttraction"),
            Self::ManifestTopCardOfLibrary => f.write_str("ManifestTopCardOfLibrary"),
            Self::CloakTopCardOfLibrary => f.write_str("CloakTopCardOfLibrary"),
            Self::ManifestCardFromHand => f.write_str("ManifestCardFromHand"),
            Self::ManifestDread => f.write_str("ManifestDread"),
            Self::Earthbend { counters } => f.debug_tuple("Earthbend").field(counters).finish(),
            Self::Behold { subtype, count } => f
                .debug_struct("Behold")
                .field("subtype", subtype)
                .field("count", count)
                .finish(),
            Self::Fight {
                creature1,
                creature2,
            } => f
                .debug_struct("Fight")
                .field("creature1", creature1)
                .field("creature2", creature2)
                .finish(),
            Self::FightIterated { creature2 } => {
                f.debug_tuple("FightIterated").field(creature2).finish()
            }
            Self::Clash { opponent } => f.debug_tuple("Clash").field(opponent).finish(),
            Self::FlipCoin => f.write_str("FlipCoin"),
            Self::RollDie { sides, die_text } => {
                if let Some(die_text) = die_text {
                    f.debug_struct("RollDie")
                        .field("sides", sides)
                        .field("die_text", die_text)
                        .finish()
                } else {
                    f.debug_tuple("RollDie").field(sides).finish()
                }
            }
            Self::RollDiceChooseResult {
                count,
                sides,
                die_text,
            } => f
                .debug_struct("RollDiceChooseResult")
                .field("count", count)
                .field("sides", sides)
                .field("die_text", die_text)
                .finish(),
            Self::ShuffleHandAndGraveyardIntoLibrary => {
                f.write_str("ShuffleHandAndGraveyardIntoLibrary")
            }
            Self::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary => {
                f.write_str("ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary")
            }
            Self::ShuffleGraveyardIntoLibrary => f.write_str("ShuffleGraveyardIntoLibrary"),
            Self::ReorderGraveyard => f.write_str("ReorderGraveyard"),
            Self::ChooseColor => f.write_str("ChooseColor"),
            Self::ChooseCardType { options } => {
                f.debug_tuple("ChooseCardType").field(options).finish()
            }
            Self::ChooseNamedOption { options } => {
                f.debug_tuple("ChooseNamedOption").field(options).finish()
            }
            Self::ChooseCreatureType { excluded_subtypes } => f
                .debug_struct("ChooseCreatureType")
                .field("excluded_subtypes", excluded_subtypes)
                .finish(),
            Self::ChooseLandType { exclude_basic } => f
                .debug_struct("ChooseLandType")
                .field("exclude_basic", exclude_basic)
                .finish(),
            Self::ChooseCardName { filter, tag } => f
                .debug_struct("ChooseCardName")
                .field("filter", filter)
                .field("tag", tag)
                .finish(),
            Self::ChoosePlayer {
                filter,
                tag,
                random,
                exclude_previous_choices,
            } => f
                .debug_struct("ChoosePlayer")
                .field("filter", filter)
                .field("tag", tag)
                .field("random", random)
                .field("exclude_previous_choices", exclude_previous_choices)
                .finish(),
            Self::NoteLifeTotal => f.write_str("NoteLifeTotal"),
            Self::ChooseSpellCastHistory {
                cast_by,
                filter,
                tag,
            } => f
                .debug_struct("ChooseSpellCastHistory")
                .field("cast_by", cast_by)
                .field("filter", filter)
                .field("tag", tag)
                .finish(),
            Self::AddMana { mana } => f.debug_tuple("AddMana").field(mana).finish(),
            Self::AddManaScaled { mana, amount } => f
                .debug_struct("AddManaScaled")
                .field("mana", mana)
                .field("amount", amount)
                .finish(),
            Self::AddManaAnyColor {
                amount,
                available_colors,
                distinct_colors,
            } => f
                .debug_struct("AddManaAnyColor")
                .field("amount", amount)
                .field("available_colors", available_colors)
                .field("distinct_colors", distinct_colors)
                .finish(),
            Self::AddManaAnyOneColor { amount } => {
                f.debug_tuple("AddManaAnyOneColor").field(amount).finish()
            }
            Self::AddManaChosenColor {
                amount,
                fixed_option,
            } => f
                .debug_struct("AddManaChosenColor")
                .field("amount", amount)
                .field("fixed_option", fixed_option)
                .finish(),
            Self::AddManaFromLandCouldProduce {
                amount,
                land_filter,
                allow_colorless,
                same_type,
                mana_type_source,
            } => f
                .debug_struct("AddManaFromLandCouldProduce")
                .field("amount", amount)
                .field("land_filter", land_filter)
                .field("allow_colorless", allow_colorless)
                .field("same_type", same_type)
                .field("mana_type_source", mana_type_source)
                .finish(),
            Self::AddManaColorsAmong { filter } => f
                .debug_struct("AddManaColorsAmong")
                .field("filter", filter)
                .finish(),
            Self::AddManaCommanderIdentity { amount } => f
                .debug_tuple("AddManaCommanderIdentity")
                .field(amount)
                .finish(),
            Self::ExchangeLifeTotals { player2 } => {
                f.debug_tuple("ExchangeLifeTotals").field(player2).finish()
            }
            Self::ExchangeTextBoxes { target } => {
                f.debug_tuple("ExchangeTextBoxes").field(target).finish()
            }
            Self::ExchangeZones { zone1, zone2 } => f
                .debug_struct("ExchangeZones")
                .field("zone1", zone1)
                .field("zone2", zone2)
                .finish(),
            Self::PutRestOnBottomOfLibrary => f.write_str("PutRestOnBottomOfLibrary"),
            Self::DontLoseThisManaAsStepsAndPhasesEndThisTurn => {
                f.write_str("DontLoseThisManaAsStepsAndPhasesEndThisTurn")
            }
            Self::ExchangeValues {
                left,
                right,
                duration,
            } => f
                .debug_struct("ExchangeValues")
                .field("left", left)
                .field("right", right)
                .field("duration", duration)
                .finish(),
            Self::ExchangeControl {
                filter,
                count,
                shared_type,
            } => f
                .debug_struct("ExchangeControl")
                .field("filter", filter)
                .field("count", count)
                .field("shared_type", shared_type)
                .finish(),
            Self::ExchangeControlHeterogeneous {
                permanent1,
                permanent2,
                shared_type,
            } => f
                .debug_struct("ExchangeControlHeterogeneous")
                .field("permanent1", permanent1)
                .field("permanent2", permanent2)
                .field("shared_type", shared_type)
                .finish(),
            Self::Attach { object, target } => f
                .debug_struct("Attach")
                .field("object", object)
                .field("target", target)
                .finish(),
            Self::Unattach { object } => {
                f.debug_struct("Unattach").field("object", object).finish()
            }
            Self::Enchant { filter } => f.debug_tuple("Enchant").field(filter).finish(),
            Self::ExileWhenSourceLeaves { target } => f
                .debug_tuple("ExileWhenSourceLeaves")
                .field(target)
                .finish(),
            Self::SacrificeSourceWhenLeaves { target } => f
                .debug_tuple("SacrificeSourceWhenLeaves")
                .field(target)
                .finish(),
            Self::RegisterZoneReplacement {
                target,
                from_zone,
                to_zone,
                replacement_zone,
                library_placement,
                duration,
                optional,
                choice_description,
                counters,
            } => f
                .debug_struct("RegisterZoneReplacement")
                .field("target", target)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("library_placement", library_placement)
                .field("duration", duration)
                .field("optional", optional)
                .field("choice_description", choice_description)
                .field("counters", counters)
                .finish(),
            Self::RegisterFutureZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
                cause_policy,
                link_exiled_to_source,
            } => f
                .debug_struct("RegisterFutureZoneReplacement")
                .field("filter", filter)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("duration", duration)
                .field("cause_policy", cause_policy)
                .field("link_exiled_to_source", link_exiled_to_source)
                .finish(),
            Self::RegisterDrawReplacement {
                player,
                replacement_effects,
                duration,
            } => f
                .debug_struct("RegisterDrawReplacement")
                .field("player", player)
                .field("replacement_effects", replacement_effects)
                .field("duration", duration)
                .finish(),
            Self::RegisterManaReplacement {
                source_filter,
                replacement_mana,
                mode,
            } => f
                .debug_struct("RegisterManaReplacement")
                .field("source_filter", source_filter)
                .field("replacement_mana", replacement_mana)
                .field("mode", mode)
                .finish(),
            Self::RegisterDamagedBySourceZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
            } => f
                .debug_struct("RegisterDamagedBySourceZoneReplacement")
                .field("filter", filter)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("duration", duration)
                .finish(),
            Self::RegisterEnterUnderControlReplacement { filter, duration } => f
                .debug_struct("RegisterEnterUnderControlReplacement")
                .field("filter", filter)
                .field("duration", duration)
                .finish(),
            Self::ExileInsteadOfGraveyardThisTurn => f.write_str("ExileInsteadOfGraveyardThisTurn"),
            Self::ControlCombatChoicesThisTurn {
                attackers,
                blockers,
                this_combat,
            } => f
                .debug_struct("ControlCombatChoicesThisTurn")
                .field("attackers", attackers)
                .field("blockers", blockers)
                .field("this_combat", this_combat)
                .finish(),
            Self::GainControl {
                target,
                duration,
                condition,
                source_reference_surface,
            } => f
                .debug_struct("GainControl")
                .field("target", target)
                .field("duration", duration)
                .field("condition", condition)
                .field("source_reference_surface", source_reference_surface)
                .finish(),
            Self::RevealTop => f.write_str("RevealTop"),
            Self::ExileTopOfLibrary {
                count,
                tags,
                accumulated_tags,
                face_down,
            } => f
                .debug_struct("ExileTopOfLibrary")
                .field("count", count)
                .field("tags", tags)
                .field("accumulated_tags", accumulated_tags)
                .field("face_down", face_down)
                .finish(),
            Self::RevealTagged { tag } => f.debug_tuple("RevealTagged").field(tag).finish(),
            Self::PutOntoBattlefield {
                target,
                tapped,
                controller,
                cloak,
                shuffle_before,
            } => f
                .debug_struct("PutOntoBattlefield")
                .field("target", target)
                .field("tapped", tapped)
                .field("controller", controller)
                .field("cloak", cloak)
                .field("shuffle_before", shuffle_before)
                .finish(),
            Self::RevealCardsFromHand {
                count,
                count_value,
                tag,
            } => f
                .debug_struct("RevealCardsFromHand")
                .field("count", count)
                .field("count_value", count_value)
                .field("tag", tag)
                .finish(),
            Self::LookAtTopCards { count, tag, reveal } => f
                .debug_struct("LookAtTopCards")
                .field("count", count)
                .field("tag", tag)
                .field("reveal", reveal)
                .finish(),
            Self::LookAtObjects { filter } => f
                .debug_struct("LookAtObjects")
                .field("filter", filter)
                .finish(),
            Self::LookAtTarget { target } => f.debug_tuple("LookAtTarget").field(target).finish(),
            Self::MayMoveToZone { target, zone } => f
                .debug_struct("MayMoveToZone")
                .field("target", target)
                .field("zone", zone)
                .finish(),
            Self::AdditionalLandPlays { count, duration } => f
                .debug_struct("AdditionalLandPlays")
                .field("count", count)
                .field("duration", duration)
                .finish(),
            Self::ExtraTurnAfterTurn { anchor } => {
                f.debug_tuple("ExtraTurnAfterTurn").field(anchor).finish()
            }
            Self::ReorderTopOfLibrary { tag } => {
                f.debug_tuple("ReorderTopOfLibrary").field(tag).finish()
            }
            Self::AddManaImprintedColors => f.write_str("AddManaImprintedColors"),
            Self::ShuffleLibrary => f.write_str("ShuffleLibrary"),
            Self::ShuffleObjectsIntoLibrary {
                target,
                all,
                owner_library_destination,
            } => f
                .debug_struct("ShuffleObjectsIntoLibrary")
                .field("target", target)
                .field("all", all)
                .field("owner_library_destination", owner_library_destination)
                .finish(),
            Self::GrantProtectionChoice {
                target,
                chooser,
                allow_colorless,
                allow_artifacts,
            } => f
                .debug_struct("GrantProtectionChoice")
                .field("target", target)
                .field("chooser", chooser)
                .field("allow_colorless", allow_colorless)
                .field("allow_artifacts", allow_artifacts)
                .finish(),
            Self::PreventAllCombatDamage { duration } => f
                .debug_struct("PreventAllCombatDamage")
                .field("duration", duration)
                .finish(),
            Self::AssignNoCombatDamage { source, duration } => f
                .debug_struct("AssignNoCombatDamage")
                .field("source", source)
                .field("duration", duration)
                .finish(),
            Self::PreventAllCombatDamageFromSource {
                duration,
                source,
                source_would_deal_surface,
            } => f
                .debug_struct("PreventAllCombatDamageFromSource")
                .field("duration", duration)
                .field("source", source)
                .field("source_would_deal_surface", source_would_deal_surface)
                .finish(),
            Self::PreventAllCombatDamageFromSourceFilter {
                duration,
                source_filter,
                excluded_source_target,
            } => f
                .debug_struct("PreventAllCombatDamageFromSourceFilter")
                .field("duration", duration)
                .field("source_filter", source_filter)
                .field("excluded_source_target", excluded_source_target)
                .finish(),
            Self::PreventAllCombatDamageToPlayers { duration } => f
                .debug_struct("PreventAllCombatDamageToPlayers")
                .field("duration", duration)
                .finish(),
            Self::PreventAllCombatDamageToYou { duration } => f
                .debug_struct("PreventAllCombatDamageToYou")
                .field("duration", duration)
                .finish(),
            Self::PreventNextTimeDamage {
                source,
                target,
                reflect_damage_to_source_controller,
                follow_up_effects,
            } => f
                .debug_struct("PreventNextTimeDamage")
                .field("source", source)
                .field("target", target)
                .field(
                    "reflect_damage_to_source_controller",
                    reflect_damage_to_source_controller,
                )
                .field("follow_up_effects", follow_up_effects)
                .finish(),
            Self::PreventDamage {
                amount,
                target,
                duration,
                follow_up_effects,
                ..
            } => f
                .debug_struct("PreventDamage")
                .field("amount", amount)
                .field("target", target)
                .field("duration", duration)
                .field("follow_up_effects", follow_up_effects)
                .finish(),
            Self::PreventAllDamageToTarget {
                target,
                duration,
                source_of_your_choice,
                source_choice_shares_activation_mana_color,
                source_target,
            } => f
                .debug_struct("PreventAllDamageToTarget")
                .field("target", target)
                .field("duration", duration)
                .field("source_of_your_choice", source_of_your_choice)
                .field(
                    "source_choice_shares_activation_mana_color",
                    source_choice_shares_activation_mana_color,
                )
                .field("source_target", source_target)
                .finish(),
            Self::PreventAllDamageToTargetFromSourceFilter {
                target,
                duration,
                source_filter,
            } => f
                .debug_struct("PreventAllDamageToTargetFromSourceFilter")
                .field("target", target)
                .field("duration", duration)
                .field("source_filter", source_filter)
                .finish(),
            Self::PreventAllDamageFromSourceFilter {
                duration,
                source_filter,
            } => f
                .debug_struct("PreventAllDamageFromSourceFilter")
                .field("duration", duration)
                .field("source_filter", source_filter)
                .finish(),
            Self::PreventDamageToTargetPutCounters {
                amount,
                target,
                duration,
                counter_type,
            } => f
                .debug_struct("PreventDamageToTargetPutCounters")
                .field("amount", amount)
                .field("target", target)
                .field("duration", duration)
                .field("counter_type", counter_type)
                .finish(),
            Self::PreventDamageEach {
                amount,
                filter,
                duration,
            } => f
                .debug_struct("PreventDamageEach")
                .field("amount", amount)
                .field("filter", filter)
                .field("duration", duration)
                .finish(),
            Self::CopySpell {
                target,
                all_matches,
                count,
                player,
                may_choose_new_targets,
                choose_new_target_singular,
                removed_supertypes,
            } => f
                .debug_struct("CopySpell")
                .field("target", target)
                .field("all_matches", all_matches)
                .field("count", count)
                .field("player", player)
                .field("may_choose_new_targets", may_choose_new_targets)
                .field("choose_new_target_singular", choose_new_target_singular)
                .field("removed_supertypes", removed_supertypes)
                .finish(),
            Self::CopySpellForEachTarget {
                target,
                object_filter,
                player_filter,
                player,
                exclude_current_targets,
                removed_supertypes,
            } => f
                .debug_struct("CopySpellForEachTarget")
                .field("target", target)
                .field("object_filter", object_filter)
                .field("player_filter", player_filter)
                .field("player", player)
                .field("exclude_current_targets", exclude_current_targets)
                .field("removed_supertypes", removed_supertypes)
                .finish(),
            Self::ScaleXValue { target, multiplier } => f
                .debug_struct("ScaleXValue")
                .field("target", target)
                .field("multiplier", multiplier)
                .finish(),
            Self::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged,
                order,
                player,
                surface,
            } => f
                .debug_struct("PutTaggedRemainderOnBottomOfLibrary")
                .field("tag", tag)
                .field("keep_tagged", keep_tagged)
                .field("order", order)
                .field("player", player)
                .field("surface", surface)
                .finish(),
            Self::PutTaggedRemainderInZone {
                tag,
                keep_tagged,
                zone,
            } => f
                .debug_struct("PutTaggedRemainderInZone")
                .field("tag", tag)
                .field("keep_tagged", keep_tagged)
                .field("zone", zone)
                .finish(),
            Self::CastTagged {
                tag,
                player,
                allow_land,
                as_copy,
                without_paying_mana_cost,
                cost_reduction,
            } => f
                .debug_struct("CastTagged")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("as_copy", as_copy)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("cost_reduction", cost_reduction)
                .finish(),
            Self::GrantPlayTaggedUntilEndOfTurn {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library,
            } => f
                .debug_struct("GrantPlayTaggedUntilEndOfTurn")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("while_on_top_of_library", while_on_top_of_library)
                .finish(),
            Self::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                player,
            } => f
                .debug_struct("GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn")
                .field("tag", tag)
                .field("player", player)
                .finish(),
            Self::GrantPlayTaggedUntilYourNextTurn {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
                until_next_end_step,
            } => f
                .debug_struct("GrantPlayTaggedUntilYourNextTurn")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("until_next_end_step", until_next_end_step)
                .finish(),
            Self::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                during_turns_counter_put_on_source,
            } => f
                .debug_struct("GrantPlayTaggedForAsLongAsExiled")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .field("filter", filter)
                .field(
                    "during_turns_counter_put_on_source",
                    during_turns_counter_put_on_source,
                )
                .finish(),
            Self::GrantPlayTaggedForAsLongAsYouControlSource {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
            } => f
                .debug_struct("GrantPlayTaggedForAsLongAsYouControlSource")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .finish(),
            Self::ReturnToBattlefield {
                target,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura,
                top_only,
            } => f
                .debug_struct("ReturnToBattlefield")
                .field("target", target)
                .field("tapped", tapped)
                .field("transformed", transformed)
                .field("converted", converted)
                .field("controller", controller)
                .field("count_value", count_value)
                .field("as_aura", as_aura)
                .field("top_only", top_only)
                .finish(),
            Self::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
                verb_surface,
            } => f
                .debug_struct("ReturnAllToBattlefield")
                .field("filter", filter)
                .field("tapped", tapped)
                .field("face_down", face_down)
                .field("controller", controller)
                .field("verb_surface", verb_surface)
                .finish(),
            Self::ExileUntilSourceLeaves {
                target,
                face_down,
                all,
                explicit_return_surface,
            } => f
                .debug_struct("ExileUntilSourceLeaves")
                .field("target", target)
                .field("face_down", face_down)
                .field("all", all)
                .field("explicit_return_surface", explicit_return_surface)
                .finish(),
            Self::MoveToZone {
                target,
                source_top_only,
                zone,
                to_top,
                library_order,
                library_order_chooser,
                verb_surface,
                target_plural_surface,
                destination_player_surface,
                destination_player_reference_surface,
                exiled_with_source_surface,
                battlefield_controller,
                battlefield_tapped,
                battlefield_attacking,
                battlefield_attack_target_player_or_planeswalker_controlled_by,
                battlefield_face_down,
                attached_to,
                all,
            } => f
                .debug_struct("MoveToZone")
                .field("target", target)
                .field("source_top_only", source_top_only)
                .field("zone", zone)
                .field("to_top", to_top)
                .field("library_order", library_order)
                .field("library_order_chooser", library_order_chooser)
                .field("verb_surface", verb_surface)
                .field("target_plural_surface", target_plural_surface)
                .field("destination_player_surface", destination_player_surface)
                .field(
                    "destination_player_reference_surface",
                    destination_player_reference_surface,
                )
                .field("exiled_with_source_surface", exiled_with_source_surface)
                .field("battlefield_controller", battlefield_controller)
                .field("battlefield_tapped", battlefield_tapped)
                .field("battlefield_attacking", battlefield_attacking)
                .field(
                    "battlefield_attack_target_player_or_planeswalker_controlled_by",
                    battlefield_attack_target_player_or_planeswalker_controlled_by,
                )
                .field("battlefield_face_down", battlefield_face_down)
                .field("attached_to", attached_to)
                .field("all", all)
                .finish(),
            Self::MoveToLibraryTopOrBottomChoice { target } => f
                .debug_struct("MoveToLibraryTopOrBottomChoice")
                .field("target", target)
                .finish(),
            Self::TargetOnly {
                target,
                explicit_declaration,
            } => f
                .debug_struct("TargetOnly")
                .field("target", target)
                .field("explicit_declaration", explicit_declaration)
                .finish(),
            Self::TagMatchingObjects { filter, zones, tag } => f
                .debug_struct("TagMatchingObjects")
                .field("filter", filter)
                .field("zones", zones)
                .field("tag", tag)
                .finish(),
            Self::Pump {
                power,
                toughness,
                target,
                duration,
                condition,
                set_quantifier_surface,
            } => f
                .debug_struct("Pump")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
                set_quantifier_surface,
            } => f
                .debug_struct("SetBasePowerToughness")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::BecomeBasePtCreature {
                power,
                toughness,
                target,
                card_types,
                subtypes,
                subtype_families,
                colors,
                abilities,
                granted_abilities,
                preserve_other_types,
                type_retention_surface,
                animation_pt_surface,
                animation_duration_surface,
                duration,
            } => f
                .debug_struct("BecomeBasePtCreature")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("card_types", card_types)
                .field("subtypes", subtypes)
                .field("subtype_families", subtype_families)
                .field("colors", colors)
                .field("abilities", abilities)
                .field("granted_abilities", granted_abilities)
                .field("preserve_other_types", preserve_other_types)
                .field("type_retention_surface", type_retention_surface)
                .field("animation_pt_surface", animation_pt_surface)
                .field("animation_duration_surface", animation_duration_surface)
                .field("duration", duration)
                .finish(),
            Self::SetBasePower {
                power,
                target,
                duration,
            } => f
                .debug_struct("SetBasePower")
                .field("power", power)
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::PumpForEach {
                power_per,
                toughness_per,
                target,
                count,
                duration,
            } => f
                .debug_struct("PumpForEach")
                .field("power_per", power_per)
                .field("toughness_per", toughness_per)
                .field("target", target)
                .field("count", count)
                .field("duration", duration)
                .finish(),
            Self::PumpAll {
                filter,
                power,
                toughness,
                duration,
                set_quantifier_surface,
            } => f
                .debug_struct("PumpAll")
                .field("filter", filter)
                .field("power", power)
                .field("toughness", toughness)
                .field("duration", duration)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::PumpByLastEffect {
                power,
                toughness,
                target,
                duration,
            } => f
                .debug_struct("PumpByLastEffect")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::AddCardTypes {
                target,
                card_types,
                duration,
            } => f
                .debug_struct("AddCardTypes")
                .field("target", target)
                .field("card_types", card_types)
                .field("duration", duration)
                .finish(),
            Self::SetCardTypes {
                target,
                card_types,
                duration,
            } => f
                .debug_struct("SetCardTypes")
                .field("target", target)
                .field("card_types", card_types)
                .field("duration", duration)
                .finish(),
            Self::RemoveCardTypes {
                target,
                card_types,
                duration,
            } => f
                .debug_struct("RemoveCardTypes")
                .field("target", target)
                .field("card_types", card_types)
                .field("duration", duration)
                .finish(),
            Self::AddSubtypes {
                target,
                subtypes,
                duration,
            } => f
                .debug_struct("AddSubtypes")
                .field("target", target)
                .field("subtypes", subtypes)
                .field("duration", duration)
                .finish(),
            Self::SetCreatureSubtypes {
                target,
                subtypes,
                duration,
            } => f
                .debug_struct("SetCreatureSubtypes")
                .field("target", target)
                .field("subtypes", subtypes)
                .field("duration", duration)
                .finish(),
            Self::BecomeSaddledUntilEndOfTurn { target } => f
                .debug_struct("BecomeSaddledUntilEndOfTurn")
                .field("target", target)
                .finish(),
            Self::AddColors {
                target,
                colors,
                duration,
            } => f
                .debug_struct("AddColors")
                .field("target", target)
                .field("colors", colors)
                .field("duration", duration)
                .finish(),
            Self::AddAllSubtypesOfFamily {
                target,
                family,
                duration,
            } => f
                .debug_struct("AddAllSubtypesOfFamily")
                .field("target", target)
                .field("family", family)
                .field("duration", duration)
                .finish(),
            Self::RemoveAllSubtypesOfFamily {
                target,
                family,
                duration,
            } => f
                .debug_struct("RemoveAllSubtypesOfFamily")
                .field("target", target)
                .field("family", family)
                .field("duration", duration)
                .finish(),
            Self::BecomeAuraEnchantment {
                target,
                attachment_filter,
                granted_abilities,
                duration,
            } => f
                .debug_struct("BecomeAuraEnchantment")
                .field("target", target)
                .field("attachment_filter", attachment_filter)
                .field("granted_abilities", granted_abilities)
                .field("duration", duration)
                .finish(),
            Self::BecomeBasicLandType {
                target,
                subtype,
                duration,
            } => f
                .debug_struct("BecomeBasicLandType")
                .field("target", target)
                .field("subtype", subtype)
                .field("duration", duration)
                .finish(),
            Self::SetColors {
                target,
                colors,
                duration,
            } => f
                .debug_struct("SetColors")
                .field("target", target)
                .field("colors", colors)
                .field("duration", duration)
                .finish(),
            Self::MakeColorless { target, duration } => f
                .debug_struct("MakeColorless")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::BecomeBasicLandTypeChoice { target, duration } => f
                .debug_struct("BecomeBasicLandTypeChoice")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::BecomeCreatureTypeChoice {
                target,
                duration,
                excluded_subtypes,
            } => f
                .debug_struct("BecomeCreatureTypeChoice")
                .field("target", target)
                .field("duration", duration)
                .field("excluded_subtypes", excluded_subtypes)
                .finish(),
            Self::BecomeColorChoice { target, duration } => f
                .debug_struct("BecomeColorChoice")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::BecomeCopy {
                target,
                source,
                duration,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                remove_supertypes,
                add_card_types,
                set_card_types,
                add_subtypes,
                set_subtypes,
                granted_abilities,
                set_base_power_toughness,
                copy_exception_surface,
            } => f
                .debug_struct("BecomeCopy")
                .field("target", target)
                .field("source", source)
                .field("duration", duration)
                .field("preserve_source_abilities", preserve_source_abilities)
                .field("name_override", name_override)
                .field("name_override_surface", name_override_surface)
                .field("add_supertypes", add_supertypes)
                .field("remove_supertypes", remove_supertypes)
                .field("add_card_types", add_card_types)
                .field("set_card_types", set_card_types)
                .field("add_subtypes", add_subtypes)
                .field("set_subtypes", set_subtypes)
                .field("granted_abilities", granted_abilities)
                .field("set_base_power_toughness", set_base_power_toughness)
                .field("copy_exception_surface", copy_exception_surface)
                .finish(),
            Self::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
                condition,
                set_quantifier_surface,
                lock_filter_at_resolution,
            } => f
                .debug_struct("GrantAbilitiesAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .field("lock_filter_at_resolution", lock_filter_at_resolution)
                .finish(),
            Self::RemoveAbilitiesAll {
                filter,
                abilities,
                duration,
                condition,
                set_quantifier_surface,
            } => f
                .debug_struct("RemoveAbilitiesAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::GrantAbilitiesChoiceAll {
                filter,
                abilities,
                duration,
            } => f
                .debug_struct("GrantAbilitiesChoiceAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
                condition,
                set_quantifier_surface,
            } => f
                .debug_struct("GrantAbilitiesToTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
                .field("condition", condition)
                .field("set_quantifier_surface", set_quantifier_surface)
                .finish(),
            Self::GrantToTarget {
                target,
                grantable,
                duration,
            } => f
                .debug_struct("GrantToTarget")
                .field("target", target)
                .field("grantable", grantable)
                .field("duration", duration)
                .finish(),
            Self::GrantBySpec {
                spec,
                player,
                duration,
            } => f
                .debug_struct("GrantBySpec")
                .field("spec", spec)
                .field("player", player)
                .field("duration", duration)
                .finish(),
            Self::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            } => f
                .debug_struct("RemoveAbilitiesFromTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            } => f
                .debug_struct("GrantAbilitiesChoiceToTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::ConsultTopOfLibrary {
                player,
                mode,
                filter,
                stop_rule,
                max_exposed,
                all_tag,
                match_tag,
            } => f
                .debug_struct("ConsultTopOfLibrary")
                .field("player", player)
                .field("mode", mode)
                .field("filter", filter)
                .field("stop_rule", stop_rule)
                .field("max_exposed", max_exposed)
                .field("all_tag", all_tag)
                .field("match_tag", match_tag)
                .finish(),
            Self::SearchLibrary {
                filter,
                destination,
                chooser,
                player,
                search_mode,
                reveal,
                shuffle,
                count,
                count_value,
                library_position_from_top,
                result_reference_surface,
                tapped,
            } => f
                .debug_struct("SearchLibrary")
                .field("filter", filter)
                .field("destination", destination)
                .field("chooser", chooser)
                .field("player", player)
                .field("search_mode", search_mode)
                .field("reveal", reveal)
                .field("shuffle", shuffle)
                .field("count", count)
                .field("count_value", count_value)
                .field("library_position_from_top", library_position_from_top)
                .field("result_reference_surface", result_reference_surface)
                .field("tapped", tapped)
                .finish(),
            Self::Cant {
                restriction,
                duration,
                condition,
            } => f
                .debug_struct("Cant")
                .field("restriction", restriction)
                .field("duration", duration)
                .field("condition", condition)
                .finish(),
            Self::CreateTokenCopy { .. } => f.write_str("CreateTokenCopy"),
            Self::CreateTokenCopyFromSource { .. } => f.write_str("CreateTokenCopyFromSource"),
            Self::CreateTokenWithMods {
                name,
                count,
                player,
                ..
            } => f
                .debug_struct("CreateTokenWithMods")
                .field("name", name)
                .field("count", count)
                .field("player", player)
                .finish(),
            Self::RedirectNextDamageFromSourceToTarget {
                amount,
                protected_target,
                destination,
                destination_target,
            } => f
                .debug_struct("RedirectNextDamageFromSourceToTarget")
                .field("amount", amount)
                .field("protected_target", protected_target)
                .field("destination", destination)
                .field("destination_target", destination_target)
                .finish(),
            Self::RedirectNextTimeDamageToSource {
                source,
                target,
                destination,
                destination_target,
                all_this_turn,
            } => f
                .debug_struct("RedirectNextTimeDamageToSource")
                .field("source", source)
                .field("target", target)
                .field("destination", destination)
                .field("destination_target", destination_target)
                .field("all_this_turn", all_this_turn)
                .finish(),
            Self::RedirectAllDamageThisTurnBySourceToSourceController { source } => f
                .debug_struct("RedirectAllDamageThisTurnBySourceToSourceController")
                .field("source", source)
                .finish(),
            Self::RedirectAllDamageThisTurnToTarget {
                player_filter,
                object_filter,
                target,
            } => f
                .debug_struct("RedirectAllDamageThisTurnToTarget")
                .field("player_filter", player_filter)
                .field("object_filter", object_filter)
                .field("target", target)
                .finish(),
            Self::Meld {
                result_name,
                enters_tapped,
                enters_attacking,
            } => f
                .debug_struct("Meld")
                .field("result_name", result_name)
                .field("enters_tapped", enters_tapped)
                .field("enters_attacking", enters_attacking)
                .finish(),
            Self::SearchLibrarySlotsToHand {
                slots,
                destination,
                reveal,
                progress_tag,
            } => f
                .debug_struct("SearchLibrarySlotsToHand")
                .field("slots", slots)
                .field("destination", destination)
                .field("reveal", reveal)
                .field("progress_tag", progress_tag)
                .finish(),
            Self::RetargetStackObject {
                target,
                mode,
                require_change,
            } => f
                .debug_struct("RetargetStackObject")
                .field("target", target)
                .field("mode", mode)
                .field("require_change", require_change)
                .finish(),
            Self::GrantAbilityToSource { ability, duration } => f
                .debug_struct("GrantAbilityToSource")
                .field("ability", ability)
                .field("duration", duration)
                .finish(),
            Self::TurnFaceUp { target } => f
                .debug_struct("TurnFaceUp")
                .field("target", target)
                .finish(),
            Self::DealDamage { amount, target, .. } => f
                .debug_struct("DealDamage")
                .field("amount", amount)
                .field("target", target)
                .finish(),
            Self::DealDamageEach { amount, filter } => f
                .debug_struct("DealDamageEach")
                .field("amount", amount)
                .field("filter", filter)
                .finish(),
            Self::DealDamageEqualToPower {
                source,
                amount,
                target,
                unpreventable,
            } => f
                .debug_struct("DealDamageEqualToPower")
                .field("source", source)
                .field("amount", amount)
                .field("target", target)
                .field("unpreventable", unpreventable)
                .finish(),
            Self::DealDistributedDamage {
                amount,
                target,
                source,
                chooser,
            } => f
                .debug_struct("DealDistributedDamage")
                .field("amount", amount)
                .field("target", target)
                .field("source", source)
                .field("chooser", chooser)
                .finish(),
            Self::Tap { target } => f.debug_tuple("Tap").field(target).finish(),
            Self::Untap { target } => f.debug_tuple("Untap").field(target).finish(),
            Self::TapAll { filter } => f.debug_tuple("TapAll").field(filter).finish(),
            Self::UntapAll { filter } => f.debug_tuple("UntapAll").field(filter).finish(),
            Self::TapOrUntap { target } => f.debug_tuple("TapOrUntap").field(target).finish(),
            Self::TapOrUntapAll {
                tap_filter,
                untap_filter,
            } => f
                .debug_struct("TapOrUntapAll")
                .field("tap_filter", tap_filter)
                .field("untap_filter", untap_filter)
                .finish(),
            Self::PhaseOut {
                target,
                duration,
                source_surface,
            } => f
                .debug_struct("PhaseOut")
                .field("target", target)
                .field("duration", duration)
                .field("source_surface", source_surface)
                .finish(),
            Self::PhaseOutAll {
                filter,
                duration,
                source_surface,
            } => f
                .debug_struct("PhaseOutAll")
                .field("filter", filter)
                .field("duration", duration)
                .field("source_surface", source_surface)
                .finish(),
            Self::PhaseIn { target } => f.debug_tuple("PhaseIn").field(target).finish(),
            Self::PhaseInAll { filter } => f.debug_tuple("PhaseInAll").field(filter).finish(),
            Self::Transform { target } => f.debug_tuple("Transform").field(target).finish(),
            Self::Convert { target } => f.debug_tuple("Convert").field(target).finish(),
            Self::Destroy {
                target,
                no_regeneration,
            } => f
                .debug_struct("Destroy")
                .field("target", target)
                .field("no_regeneration", no_regeneration)
                .finish(),
            Self::DestroyAll {
                filter,
                no_regeneration,
            } => f
                .debug_struct("DestroyAll")
                .field("filter", filter)
                .field("no_regeneration", no_regeneration)
                .finish(),
            Self::DestroyAllOfChosenColor {
                filter,
                no_regeneration,
            } => f
                .debug_struct("DestroyAllOfChosenColor")
                .field("filter", filter)
                .field("no_regeneration", no_regeneration)
                .finish(),
            Self::DestroyAllAttachedTo { filter, target } => f
                .debug_struct("DestroyAllAttachedTo")
                .field("filter", filter)
                .field("target", target)
                .finish(),
            Self::ExileAllAttachedTo {
                filter,
                target,
                face_down,
            } => f
                .debug_struct("ExileAllAttachedTo")
                .field("filter", filter)
                .field("target", target)
                .field("face_down", face_down)
                .finish(),
            Self::Exile {
                target,
                face_down,
                source_top_only,
            } => f
                .debug_struct("Exile")
                .field("target", target)
                .field("face_down", face_down)
                .field("source_top_only", source_top_only)
                .finish(),
            Self::ExileAll { filter, face_down } => f
                .debug_struct("ExileAll")
                .field("filter", filter)
                .field("face_down", face_down)
                .finish(),
            Self::LookAtHand { target } => f.debug_tuple("LookAtHand").field(target).finish(),
            Self::Counter { target } => f.debug_tuple("Counter").field(target).finish(),
            Self::CounterUnlessPays { target, cost } => f
                .debug_struct("CounterUnlessPays")
                .field("target", target)
                .field("cost", cost)
                .finish(),
            Self::PutCounters {
                counter_type,
                count,
                target,
                target_count,
                distributed,
            } => f
                .debug_struct("PutCounters")
                .field("counter_type", counter_type)
                .field("count", count)
                .field("target", target)
                .field("target_count", target_count)
                .field("distributed", distributed)
                .finish(),
            Self::PutCounterChoice {
                counter_types,
                count,
                mode_texts,
                target,
                target_count,
            } => f
                .debug_struct("PutCounterChoice")
                .field("counter_types", counter_types)
                .field("count", count)
                .field("mode_texts", mode_texts)
                .field("target", target)
                .field("target_count", target_count)
                .finish(),
            Self::PutOrRemoveCounters {
                put_counter_type,
                put_count,
                remove_counter_type,
                remove_count,
                put_mode_text,
                remove_mode_text,
                target,
                target_count,
            } => f
                .debug_struct("PutOrRemoveCounters")
                .field("put_counter_type", put_counter_type)
                .field("put_count", put_count)
                .field("remove_counter_type", remove_counter_type)
                .field("remove_count", remove_count)
                .field("put_mode_text", put_mode_text)
                .field("remove_mode_text", remove_mode_text)
                .field("target", target)
                .field("target_count", target_count)
                .finish(),
            Self::PutCountersAll {
                counter_type,
                count,
                filter,
            } => f
                .debug_struct("PutCountersAll")
                .field("counter_type", counter_type)
                .field("count", count)
                .field("filter", filter)
                .finish(),
            Self::RemoveUpToAnyCounters {
                amount,
                target,
                counter_type,
                up_to,
                all_of_them,
            } => f
                .debug_struct("RemoveUpToAnyCounters")
                .field("amount", amount)
                .field("target", target)
                .field("counter_type", counter_type)
                .field("up_to", up_to)
                .field("all_of_them", all_of_them)
                .finish(),
            Self::MoveAllCounters { from, to } => f
                .debug_struct("MoveAllCounters")
                .field("from", from)
                .field("to", to)
                .finish(),
            Self::MoveOneCounter { from, to } => f
                .debug_struct("MoveOneCounter")
                .field("from", from)
                .field("to", to)
                .finish(),
            Self::ForEachCounterKindPutOrRemove {
                target,
                all_kinds,
                fixed_counter_type,
                optional_action,
            } => f
                .debug_struct("ForEachCounterKindPutOrRemove")
                .field("target", target)
                .field("all_kinds", all_kinds)
                .field("fixed_counter_type", fixed_counter_type)
                .field("optional_action", optional_action)
                .finish(),
            Self::PutCounterOfChosenKind { target } => f
                .debug_struct("PutCounterOfChosenKind")
                .field("target", target)
                .finish(),
            Self::ReturnToHand {
                target,
                random,
                destination_player_surface,
                exiled_with_source_surface,
                set_quantifier_surface,
                set_reference_surface,
            } => f
                .debug_struct("ReturnToHand")
                .field("target", target)
                .field("random", random)
                .field("destination_player_surface", destination_player_surface)
                .field("exiled_with_source_surface", exiled_with_source_surface)
                .field("set_quantifier_surface", set_quantifier_surface)
                .field("set_reference_surface", set_reference_surface)
                .finish(),
            Self::ReturnAllToHand {
                filter,
                destination_player_surface,
                exiled_with_source_surface,
            } => f
                .debug_struct("ReturnAllToHand")
                .field("filter", filter)
                .field("destination_player_surface", destination_player_surface)
                .field("exiled_with_source_surface", exiled_with_source_surface)
                .finish(),
            Self::ReturnAllToHandOfChosenColor { filter } => f
                .debug_struct("ReturnAllToHandOfChosenColor")
                .field("filter", filter)
                .finish(),
            Self::MoveToLibraryNthFromTop { target, position } => f
                .debug_struct("MoveToLibraryNthFromTop")
                .field("target", target)
                .field("position", position)
                .finish(),
            Self::DoubleCountersOnEach {
                counter_type,
                filter,
            } => f
                .debug_struct("DoubleCountersOnEach")
                .field("counter_type", counter_type)
                .field("filter", filter)
                .finish(),
            Self::DoubleCountersOnTarget {
                counter_type,
                target,
            } => f
                .debug_struct("DoubleCountersOnTarget")
                .field("counter_type", counter_type)
                .field("target", target)
                .finish(),
            Self::RemoveCountersAll {
                amount,
                filter,
                counter_type,
                up_to,
            } => f
                .debug_struct("RemoveCountersAll")
                .field("amount", amount)
                .field("filter", filter)
                .field("counter_type", counter_type)
                .field("up_to", up_to)
                .finish(),
            Self::PutSticker { target, action } => f
                .debug_struct("PutSticker")
                .field("target", target)
                .field("action", action)
                .finish(),
            Self::SwitchPowerToughness { target, duration } => f
                .debug_struct("SwitchPowerToughness")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::ScalePowerToughnessAll {
                filter,
                power,
                toughness,
                multiplier,
                duration,
            } => f
                .debug_struct("ScalePowerToughnessAll")
                .field("filter", filter)
                .field("power", power)
                .field("toughness", toughness)
                .field("multiplier", multiplier)
                .field("duration", duration)
                .finish(),
            Self::Discard {
                count,
                random,
                any_number,
                filter,
                tag,
            } => f
                .debug_struct("Discard")
                .field("count", count)
                .field("random", random)
                .field("any_number", any_number)
                .field("filter", filter)
                .field("tag", tag)
                .finish(),
            Self::DiscardHand => f.write_str("DiscardHand"),
            Self::PoisonCounters { count } => f.debug_tuple("PoisonCounters").field(count).finish(),
            Self::EnergyCounters { count } => f.debug_tuple("EnergyCounters").field(count).finish(),
            Self::ExperienceCounters { count } => {
                f.debug_tuple("ExperienceCounters").field(count).finish()
            }
            Self::TicketCounters { count } => f.debug_tuple("TicketCounters").field(count).finish(),
            Self::PayEnergy { amount } => f.debug_tuple("PayEnergy").field(amount).finish(),
            Self::PayAnyEnergy { min_amount } => f
                .debug_struct("PayAnyEnergy")
                .field("min_amount", min_amount)
                .finish(),
            Self::PayAnyLife { min_amount } => f
                .debug_struct("PayAnyLife")
                .field("min_amount", min_amount)
                .finish(),
            Self::PayMana { cost, x_value } => f
                .debug_struct("PayMana")
                .field("cost", cost)
                .field("x_value", x_value)
                .finish(),
            Self::DoubleManaPool => f.write_str("DoubleManaPool"),
            Self::EmptyManaPool => f.write_str("EmptyManaPool"),
            Self::SetLifeTotal { amount } => f.debug_tuple("SetLifeTotal").field(amount).finish(),
            Self::EndTurn => f.write_str("EndTurn"),
            Self::EndCombatPhase => f.write_str("EndCombatPhase"),
            Self::SkipTurn => f.write_str("SkipTurn"),
            Self::SkipCombatPhases => f.write_str("SkipCombatPhases"),
            Self::SkipNextCombatPhaseThisTurn => f.write_str("SkipNextCombatPhaseThisTurn"),
            Self::SkipMainPhasesThisTurn => f.write_str("SkipMainPhasesThisTurn"),
            Self::SkipCombatPhasesThisTurn => f.write_str("SkipCombatPhasesThisTurn"),
            Self::SkipDrawStep => f.write_str("SkipDrawStep"),
            Self::AdditionalPhases { phases } => {
                f.debug_tuple("AdditionalPhases").field(phases).finish()
            }
            Self::PlayFromGraveyardUntilEot => f.write_str("PlayFromGraveyardUntilEot"),
            Self::ControlPlayer { player, duration } => f
                .debug_struct("ControlPlayer")
                .field("player", player)
                .field("duration", duration)
                .finish(),
            Self::ReduceNextSpellCostThisTurn { filter, reduction } => f
                .debug_struct("ReduceNextSpellCostThisTurn")
                .field("filter", filter)
                .field("reduction", reduction)
                .finish(),
            Self::ReduceMatchingSpellCostThisTurn {
                filter,
                reduction,
                duration,
            } => f
                .debug_struct("ReduceMatchingSpellCostThisTurn")
                .field("filter", filter)
                .field("reduction", reduction)
                .field("duration", duration)
                .finish(),
            Self::GrantNextSpellAbilityThisTurn { filter, ability } => f
                .debug_struct("GrantNextSpellAbilityThisTurn")
                .field("filter", filter)
                .field("ability", ability)
                .finish(),
            Self::RingTemptsYou => f.write_str("RingTemptsYou"),
            Self::VentureIntoDungeon {
                undercity_if_no_active,
            } => f
                .debug_struct("VentureIntoDungeon")
                .field("undercity_if_no_active", undercity_if_no_active)
                .finish(),
            Self::BecomeMonarch => f.write_str("BecomeMonarch"),
            Self::TakeInitiative => f.write_str("TakeInitiative"),
            Self::CreateEmblem { emblem } => f.debug_tuple("CreateEmblem").field(emblem).finish(),
            Self::LoseGame => f.write_str("LoseGame"),
            Self::WinGame => f.write_str("WinGame"),
            Self::Detain { target } => f.debug_tuple("Detain").field(target).finish(),
            Self::Goad { target, duration } => f
                .debug_struct("Goad")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::Suspect { target } => f.debug_tuple("Suspect").field(target).finish(),
            Self::ClearSuspected { target } => {
                f.debug_tuple("ClearSuspected").field(target).finish()
            }
            Self::HealDamage { target, amount } => f
                .debug_struct("HealDamage")
                .field("target", target)
                .field("amount", amount)
                .finish(),
            Self::RemoveFromCombat { target } => {
                f.debug_tuple("RemoveFromCombat").field(target).finish()
            }
            Self::Flip { target } => f.debug_tuple("Flip").field(target).finish(),
            Self::Regenerate {
                target,
                follow_up_effects,
            } => f
                .debug_struct("Regenerate")
                .field("target", target)
                .field("follow_up_effects", follow_up_effects)
                .finish(),
            Self::RegenerateAll { filter } => f.debug_tuple("RegenerateAll").field(filter).finish(),
            Self::Sacrifice {
                filter,
                count,
                target,
                one_of_referenced_set,
            } => f
                .debug_struct("Sacrifice")
                .field("filter", filter)
                .field("count", count)
                .field("target", target)
                .field("one_of_referenced_set", one_of_referenced_set)
                .finish(),
            Self::SacrificeAll { filter } => f
                .debug_struct("SacrificeAll")
                .field("filter", filter)
                .finish(),
        }
    }
}

impl std::fmt::Debug for SubjectVerbEffectAst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubjectVerb")
            .field("subject", &self.subject)
            .field("action", &self.action)
            .finish()
    }
}
