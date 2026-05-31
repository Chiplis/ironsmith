use crate::ConditionExpr;
use crate::ability::Ability;
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::effect::{ChoiceCount, EffectId, Until, Value};
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::{AuraAttachmentFilter, CounterType};
use crate::static_abilities::StaticAbility;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, SubtypeFamily, Supertype};
use crate::zone::Zone;

use super::super::{
    ClashOpponentAst, ControlDurationAst, DamageBySpec, ExchangeValueAst, ExtraTurnAnchorAst,
    IfResultPredicate, KeywordAction, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, RetargetModeAst, ReturnControllerAst, SearchLibrarySlotAst,
    SharedTypeConstraintAst, TargetAst, ZoneReplacementDurationAst,
};
use super::semantic::ParsedAbility;
use crate::runtime_backend::GrantedAbilityAst;

#[derive(Debug, Clone)]
pub(crate) enum StaticAbilityAst {
    Static(StaticAbility),
    KeywordAction(KeywordAction),
    ConditionalStaticAbility {
        ability: Box<StaticAbilityAst>,
        condition: ConditionExpr,
    },
    ConditionalKeywordAction {
        action: KeywordAction,
        condition: ConditionExpr,
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TriggerSpec {
    StateBased {
        condition: PredicateAst,
        display: String,
    },
    ThisAttacks,
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
    AttacksAlone(ObjectFilter),
    AttacksYouOrPlaneswalkerYouControl(ObjectFilter),
    AttacksYouOrPlaneswalkerYouControlOneOrMore(ObjectFilter),
    ThisBlocks,
    ThisBlocksObject(ObjectFilter),
    Blocks(ObjectFilter),
    ThisBecomesBlocked,
    ThisBecomesBlockedByObject(ObjectFilter),
    ThisDies,
    ThisDiesOrIsExiled,
    ThisExiledFromBattlefieldDuringCostOfAbilityWithMarker {
        marker: String,
    },
    ThisLeavesBattlefield,
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
    ThisDealsDamage,
    ThisDealsDamageToPlayer {
        player: PlayerFilter,
        amount: Option<crate::filter::Comparison>,
    },
    ThisDealsDamageTo(ObjectFilter),
    ThisDealsCombatDamage,
    ThisDealsCombatDamageTo(ObjectFilter),
    DealsDamage(ObjectFilter),
    DealsDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
    },
    DealsNoncombatDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
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
    AbilityActivated {
        activator: PlayerFilter,
        filter: ObjectFilter,
        non_mana_only: bool,
    },
    ThisIsDealtDamage,
    ThisIsDealtCombatDamage,
    IsDealtDamage(ObjectFilter),
    IsDealtCombatDamage(ObjectFilter),
    YouGainLife,
    YouGainLifeDuringTurn(PlayerFilter),
    PlayerLosesLife(PlayerFilter),
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
    PlayerDiscardsCard {
        player: PlayerFilter,
        filter: Option<ObjectFilter>,
        cause_controller: Option<PlayerFilter>,
        effect_like_only: bool,
    },
    PlayerRevealsCard {
        player: PlayerFilter,
        filter: ObjectFilter,
        from_source: bool,
    },
    PlayerSacrifices {
        player: PlayerFilter,
        filter: ObjectFilter,
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
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    },
    SpellCopied {
        filter: Option<ObjectFilter>,
        copier: PlayerFilter,
    },
    EntersBattlefield {
        filter: ObjectFilter,
        cause_filter: Option<crate::events::cause::CauseFilter>,
    },
    EntersBattlefieldOneOrMore {
        filter: ObjectFilter,
        cause_filter: Option<crate::events::cause::CauseFilter>,
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
    BeginningOfPrecombatMain(PlayerFilter),
    BeginningOfPostcombatMain(PlayerFilter),
    DayNightChanged,
    ThisEntersBattlefield,
    ThisEntersBattlefieldWithSurface(crate::target::SourceReferenceSurface),
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
    ThisDealsCombatDamageToPlayer,
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
    ItIsLandCard,
    ItIsSoulbondPaired,
    SourceChosenOption(String),
    ItMatches(ObjectFilter),
    TargetMatches(ObjectFilter),
    TaggedMatches(TagKey, ObjectFilter),
    TaggedWasCast(TagKey),
    EnchantedPermanentAttackedThisTurn,
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
    PlayerControlsAtLeast {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsExactly {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsAtLeastWithDifferentPowers {
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
    AnOpponentControlsMoreThanPlayer {
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
    PlayerIsMonarch {
        player: PlayerAst,
    },
    PlayerHasInitiative {
        player: PlayerAst,
    },
    PlayerHasCitysBlessing {
        player: PlayerAst,
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
    SourceIsSaddled,
    SourceMatches(ObjectFilter),
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
    },
    SourceHasCountersAtLeast(u32),
    SourceHasAttachmentsMatching {
        filter: ObjectFilter,
        comparison: crate::effect::Comparison,
        display: String,
    },
    SourcePowerAtLeast(u32),
    SourceDealtCombatDamageToPlayerThisTurn,
    SourceAttackedThisTurn,
    SourceCameUnderYourControlThisTurn,
    SourceAttackedOrBlockedThisTurn,
    SourceIsInZone(Zone),
    YourTurn,
    YouAttackedWithExactlyNOtherCreaturesThisCombat(u32),
    CreatureDiedThisTurn,
    CreatureDiedThisTurnOrMore(u32),
    CreatureCardPutIntoYourGraveyardThisTurn,
    PermanentLeftBattlefieldThisTurn,
    PermanentLeftBattlefieldUnderYourControlThisTurn,
    ObjectEnteredBattlefieldThisTurn(ObjectFilter),
    ObjectEnteredBattlefieldLastTurn(ObjectFilter),
    ObjectPutIntoGraveyardFromBattlefieldThisTurn(ObjectFilter),
    YouHaveFullParty,
    YouAttackedThisTurn,
    SourceWasCast,
    ThisSpellEscaped,
    NoSpellsWereCastLastTurn,
    ThisSpellWasKicked,
    ThisSpellPaidLabel(String),
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
    SameColorManaSpentToCastThisSpellAtLeast(u32),
    ThisSpellWasCastFromZone(Zone),
    ValueComparison {
        left: Value,
        operator: crate::effect::ValueComparisonOperator,
        right: Value,
    },
    Not(Box<PredicateAst>),
    And(Box<PredicateAst>, Box<PredicateAst>),
    Or(Box<PredicateAst>, Box<PredicateAst>),
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
            | PredicateAst::SourceIsSaddled
            | PredicateAst::SourceMatches(_)
            | PredicateAst::SourceHasNoCounter(_)
            | PredicateAst::SourceHasCounterAtLeast { .. }
            | PredicateAst::SourceHasCountersAtLeast(_)
            | PredicateAst::SourceHasAttachmentsMatching { .. }
            | PredicateAst::SourcePowerAtLeast(_)
            | PredicateAst::SourceAttackedThisTurn
            | PredicateAst::SourceCameUnderYourControlThisTurn
            | PredicateAst::SourceAttackedOrBlockedThisTurn
            | PredicateAst::SourceIsInZone(_)
            | PredicateAst::SourceWasCast
            | PredicateAst::ThisSpellEscaped
            | PredicateAst::ThisSpellWasKicked
            | PredicateAst::ThisSpellPaidLabel(_)
            | PredicateAst::ThisSpellWasCastFromZone(_) => {
                Some(PredicateReferenceAntecedent::SourceObject)
            }
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
#[allow(dead_code)]
pub(crate) enum SubjectVerbRoleAst {
    Actor,
    AffectedPlayer,
    Chooser,
    LibraryOwner,
    ZoneOwner,
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
        duration: ZoneReplacementDurationAst,
        optional: bool,
        choice_description: Option<String>,
    },
    RegisterFutureZoneReplacement {
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
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
    RegisterEnterWithCountersReplacement {
        filter: ObjectFilter,
        counter_type: CounterType,
        count: Value,
        duration: ZoneReplacementDurationAst,
    },
    ExileInsteadOfGraveyardThisTurn,
    ControlCombatChoicesThisTurn {
        attackers: bool,
        blockers: bool,
    },
    GainControl {
        target: TargetAst,
        duration: Until,
    },
    RevealTop,
    ExileTopOfLibrary {
        count: Value,
        tags: Vec<TagKey>,
        accumulated_tags: Vec<TagKey>,
    },
    RevealTagged {
        tag: TagKey,
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
    PutIntoHand {
        object: ObjectRefAst,
    },
    MayMoveToZone {
        target: TargetAst,
        zone: Zone,
    },
    PutSomeIntoHandRestIntoGraveyard {
        count: ChoiceCount,
    },
    PutSomeIntoHandRestOnBottomOfLibrary {
        count: ChoiceCount,
    },
    AdditionalLandPlays {
        count: Value,
        duration: Until,
    },
    ExtraTurnAfterTurn {
        anchor: ExtraTurnAnchorAst,
    },
    RearrangeLookedCardsInLibrary {
        tag: TagKey,
        count: ChoiceCount,
    },
    ReorderTopOfLibrary {
        tag: TagKey,
    },
    AddManaImprintedColors,
    ShuffleLibrary,
    ShuffleObjectsIntoLibrary {
        target: TargetAst,
    },
    GrantProtectionChoice {
        target: TargetAst,
        allow_colorless: bool,
    },
    PreventAllCombatDamage {
        duration: Until,
    },
    PreventAllCombatDamageFromSource {
        duration: Until,
        source: TargetAst,
    },
    PreventAllCombatDamageFromSourceFilter {
        duration: Until,
        source_filter: ObjectFilter,
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
    },
    PreventAllDamageToTarget {
        target: TargetAst,
        duration: Until,
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
        count: Value,
        player: PlayerAst,
        may_choose_new_targets: bool,
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
    PutTaggedRemainderOnBottomOfLibrary {
        tag: TagKey,
        keep_tagged: Option<TagKey>,
        order: LibraryBottomOrderAst,
        player: PlayerAst,
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
        allow_any_color_for_cast: bool,
    },
    GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
        tag: TagKey,
        player: PlayerAst,
    },
    GrantPlayTaggedUntilYourNextTurn {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: bool,
    },
    GrantPlayTaggedForAsLongAsExiled {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: bool,
    },
    GrantPlayTaggedForAsLongAsYouControlSource {
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: bool,
    },
    ReturnToBattlefield {
        target: TargetAst,
        tapped: bool,
        transformed: bool,
        converted: bool,
        controller: ReturnControllerAst,
        count_value: Option<Value>,
        as_aura: Option<ReturnAsAuraAst>,
    },
    ReturnAllToBattlefield {
        filter: ObjectFilter,
        tapped: bool,
        controller: ReturnControllerAst,
    },
    ExileUntilSourceLeaves {
        target: TargetAst,
        face_down: bool,
    },
    MoveToZone {
        target: TargetAst,
        zone: Zone,
        to_top: bool,
        battlefield_controller: ReturnControllerAst,
        battlefield_tapped: bool,
        attached_to: Option<TargetAst>,
    },
    MoveToLibraryTopOrBottomChoice {
        target: TargetAst,
    },
    TargetOnly {
        target: TargetAst,
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
    },
    SetBasePowerToughness {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
    },
    BecomeBasePtCreature {
        power: Value,
        toughness: Value,
        target: TargetAst,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
        colors: Option<ColorSet>,
        abilities: Vec<StaticAbility>,
        granted_abilities: Vec<GrantedAbilityAst>,
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
    },
    GrantAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    RemoveAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
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
        exile_at_next_end_step: bool,
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
        exile_at_next_end_step: bool,
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
        count: Value,
        dynamic_power_toughness: Option<(Value, Value)>,
        player: PlayerAst,
        attached_to: Option<TargetAst>,
        tapped: bool,
        attacking: bool,
        exile_at_end_of_combat: bool,
        sacrifice_at_end_of_combat: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        granted_abilities: Vec<GrantedAbilityAst>,
    },
    RedirectNextDamageFromSourceToTarget {
        amount: Value,
        target: TargetAst,
    },
    RedirectNextTimeDamageToSource {
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
        all_this_turn: bool,
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
        reveal: bool,
        progress_tag: TagKey,
    },
    RevealTopChooseCardTypePutToHandRestBottom {
        count: u32,
    },
    RevealTopPutMatchingIntoHandRestIntoGraveyard {
        count: u32,
        filter: ObjectFilter,
    },
    RevealTopPutMatchingIntoHandRestOnBottomOfLibrary {
        count: u32,
        filter: ObjectFilter,
        order: LibraryBottomOrderAst,
    },
    ChooseFromLookedCardsIntoHandRestIntoGraveyard {
        filter: ObjectFilter,
        reveal: bool,
        if_not_chosen: Vec<EffectAst>,
    },
    ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary {
        spell_filter: ObjectFilter,
        order: LibraryBottomOrderAst,
    },
    ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary {
        order: LibraryBottomOrderAst,
    },
    ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
        battlefield_filter: ObjectFilter,
        tapped: bool,
    },
    ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
        battlefield_filter: ObjectFilter,
        hand_filter: ObjectFilter,
        tapped: bool,
        order: LibraryBottomOrderAst,
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
    },
    DealDamageEach {
        amount: Value,
        filter: ObjectFilter,
    },
    DealDamageEqualToPower {
        source: TargetAst,
        target: TargetAst,
    },
    DealDistributedDamage {
        amount: Value,
        target: TargetAst,
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
    },
    PhaseOutAll {
        filter: ObjectFilter,
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
    Exile {
        target: TargetAst,
        face_down: bool,
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
    },
    ReturnToHand {
        target: TargetAst,
        random: bool,
    },
    ReturnAllToHand {
        filter: ObjectFilter,
    },
    ReturnAllToHandOfChosenColor {
        filter: ObjectFilter,
    },
    MoveToLibraryNthFromTop {
        target: TargetAst,
        position: Value,
    },
    DoubleCountersOnEach {
        counter_type: CounterType,
        filter: ObjectFilter,
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
    TicketCounters {
        count: Value,
    },
    PayEnergy {
        amount: Value,
    },
    PayAnyEnergy {
        min_amount: u32,
    },
    PayMana {
        cost: ManaCost,
    },
    DoubleManaPool,
    EmptyManaPool,
    SetLifeTotal {
        amount: Value,
    },
    EndTurn,
    SkipTurn,
    SkipCombatPhases,
    SkipNextCombatPhaseThisTurn,
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
        text: String,
    },
    LoseGame,
    WinGame,
    Detain {
        target: TargetAst,
    },
    Goad {
        target: TargetAst,
    },
    Suspect {
        target: TargetAst,
    },
    ClearSuspected {
        target: Option<TargetAst>,
    },
    RemoveFromCombat {
        target: TargetAst,
    },
    Flip {
        target: TargetAst,
    },
    Regenerate {
        target: TargetAst,
    },
    RegenerateAll {
        filter: ObjectFilter,
    },
    Sacrifice {
        filter: ObjectFilter,
        count: u32,
        target: Option<TargetAst>,
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
            Self::ZoneOwner => "ZoneOwner",
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
            } => f
                .debug_struct("AddManaAnyColor")
                .field("amount", amount)
                .field("available_colors", available_colors)
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
            } => f
                .debug_struct("AddManaFromLandCouldProduce")
                .field("amount", amount)
                .field("land_filter", land_filter)
                .field("allow_colorless", allow_colorless)
                .field("same_type", same_type)
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
                duration,
                optional,
                choice_description,
            } => f
                .debug_struct("RegisterZoneReplacement")
                .field("target", target)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("duration", duration)
                .field("optional", optional)
                .field("choice_description", choice_description)
                .finish(),
            Self::RegisterFutureZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
            } => f
                .debug_struct("RegisterFutureZoneReplacement")
                .field("filter", filter)
                .field("from_zone", from_zone)
                .field("to_zone", to_zone)
                .field("replacement_zone", replacement_zone)
                .field("duration", duration)
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
            Self::RegisterEnterWithCountersReplacement {
                filter,
                counter_type,
                count,
                duration,
            } => f
                .debug_struct("RegisterEnterWithCountersReplacement")
                .field("filter", filter)
                .field("counter_type", counter_type)
                .field("count", count)
                .field("duration", duration)
                .finish(),
            Self::ExileInsteadOfGraveyardThisTurn => f.write_str("ExileInsteadOfGraveyardThisTurn"),
            Self::ControlCombatChoicesThisTurn {
                attackers,
                blockers,
            } => f
                .debug_struct("ControlCombatChoicesThisTurn")
                .field("attackers", attackers)
                .field("blockers", blockers)
                .finish(),
            Self::GainControl { target, duration } => f
                .debug_struct("GainControl")
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::RevealTop => f.write_str("RevealTop"),
            Self::ExileTopOfLibrary {
                count,
                tags,
                accumulated_tags,
            } => f
                .debug_struct("ExileTopOfLibrary")
                .field("count", count)
                .field("tags", tags)
                .field("accumulated_tags", accumulated_tags)
                .finish(),
            Self::RevealTagged { tag } => f.debug_tuple("RevealTagged").field(tag).finish(),
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
            Self::PutIntoHand { object } => f.debug_tuple("PutIntoHand").field(object).finish(),
            Self::MayMoveToZone { target, zone } => f
                .debug_struct("MayMoveToZone")
                .field("target", target)
                .field("zone", zone)
                .finish(),
            Self::PutSomeIntoHandRestIntoGraveyard { count } => f
                .debug_tuple("PutSomeIntoHandRestIntoGraveyard")
                .field(count)
                .finish(),
            Self::PutSomeIntoHandRestOnBottomOfLibrary { count } => f
                .debug_tuple("PutSomeIntoHandRestOnBottomOfLibrary")
                .field(count)
                .finish(),
            Self::AdditionalLandPlays { count, duration } => f
                .debug_struct("AdditionalLandPlays")
                .field("count", count)
                .field("duration", duration)
                .finish(),
            Self::ExtraTurnAfterTurn { anchor } => {
                f.debug_tuple("ExtraTurnAfterTurn").field(anchor).finish()
            }
            Self::RearrangeLookedCardsInLibrary { tag, count } => f
                .debug_struct("RearrangeLookedCardsInLibrary")
                .field("tag", tag)
                .field("count", count)
                .finish(),
            Self::ReorderTopOfLibrary { tag } => {
                f.debug_tuple("ReorderTopOfLibrary").field(tag).finish()
            }
            Self::AddManaImprintedColors => f.write_str("AddManaImprintedColors"),
            Self::ShuffleLibrary => f.write_str("ShuffleLibrary"),
            Self::ShuffleObjectsIntoLibrary { target } => f
                .debug_tuple("ShuffleObjectsIntoLibrary")
                .field(target)
                .finish(),
            Self::GrantProtectionChoice {
                target,
                allow_colorless,
            } => f
                .debug_struct("GrantProtectionChoice")
                .field("target", target)
                .field("allow_colorless", allow_colorless)
                .finish(),
            Self::PreventAllCombatDamage { duration } => f
                .debug_struct("PreventAllCombatDamage")
                .field("duration", duration)
                .finish(),
            Self::PreventAllCombatDamageFromSource { duration, source } => f
                .debug_struct("PreventAllCombatDamageFromSource")
                .field("duration", duration)
                .field("source", source)
                .finish(),
            Self::PreventAllCombatDamageFromSourceFilter {
                duration,
                source_filter,
            } => f
                .debug_struct("PreventAllCombatDamageFromSourceFilter")
                .field("duration", duration)
                .field("source_filter", source_filter)
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
                ..
            } => f
                .debug_struct("PreventDamage")
                .field("amount", amount)
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::PreventAllDamageToTarget { target, duration } => f
                .debug_struct("PreventAllDamageToTarget")
                .field("target", target)
                .field("duration", duration)
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
                count,
                player,
                may_choose_new_targets,
                removed_supertypes,
            } => f
                .debug_struct("CopySpell")
                .field("target", target)
                .field("count", count)
                .field("player", player)
                .field("may_choose_new_targets", may_choose_new_targets)
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
            Self::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged,
                order,
                player,
            } => f
                .debug_struct("PutTaggedRemainderOnBottomOfLibrary")
                .field("tag", tag)
                .field("keep_tagged", keep_tagged)
                .field("order", order)
                .field("player", player)
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
            } => f
                .debug_struct("GrantPlayTaggedUntilEndOfTurn")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("without_paying_mana_cost", without_paying_mana_cost)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
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
            } => f
                .debug_struct("GrantPlayTaggedUntilYourNextTurn")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
                .finish(),
            Self::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
            } => f
                .debug_struct("GrantPlayTaggedForAsLongAsExiled")
                .field("tag", tag)
                .field("player", player)
                .field("allow_land", allow_land)
                .field("allow_any_color_for_cast", allow_any_color_for_cast)
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
            } => f
                .debug_struct("ReturnToBattlefield")
                .field("target", target)
                .field("tapped", tapped)
                .field("transformed", transformed)
                .field("converted", converted)
                .field("controller", controller)
                .field("count_value", count_value)
                .field("as_aura", as_aura)
                .finish(),
            Self::ReturnAllToBattlefield {
                filter,
                tapped,
                controller,
            } => f
                .debug_struct("ReturnAllToBattlefield")
                .field("filter", filter)
                .field("tapped", tapped)
                .field("controller", controller)
                .finish(),
            Self::ExileUntilSourceLeaves { target, face_down } => f
                .debug_struct("ExileUntilSourceLeaves")
                .field("target", target)
                .field("face_down", face_down)
                .finish(),
            Self::MoveToZone {
                target,
                zone,
                to_top,
                battlefield_controller,
                battlefield_tapped,
                attached_to,
            } => f
                .debug_struct("MoveToZone")
                .field("target", target)
                .field("zone", zone)
                .field("to_top", to_top)
                .field("battlefield_controller", battlefield_controller)
                .field("battlefield_tapped", battlefield_tapped)
                .field("attached_to", attached_to)
                .finish(),
            Self::MoveToLibraryTopOrBottomChoice { target } => f
                .debug_struct("MoveToLibraryTopOrBottomChoice")
                .field("target", target)
                .finish(),
            Self::TargetOnly { target } => f.debug_tuple("TargetOnly").field(target).finish(),
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
            } => f
                .debug_struct("Pump")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .field("condition", condition)
                .finish(),
            Self::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
            } => f
                .debug_struct("SetBasePowerToughness")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("duration", duration)
                .finish(),
            Self::BecomeBasePtCreature {
                power,
                toughness,
                target,
                card_types,
                subtypes,
                colors,
                abilities,
                granted_abilities,
                duration,
            } => f
                .debug_struct("BecomeBasePtCreature")
                .field("power", power)
                .field("toughness", toughness)
                .field("target", target)
                .field("card_types", card_types)
                .field("subtypes", subtypes)
                .field("colors", colors)
                .field("abilities", abilities)
                .field("granted_abilities", granted_abilities)
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
            } => f
                .debug_struct("PumpAll")
                .field("filter", filter)
                .field("power", power)
                .field("toughness", toughness)
                .field("duration", duration)
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
                duration,
            } => f
                .debug_struct("BecomeAuraEnchantment")
                .field("target", target)
                .field("attachment_filter", attachment_filter)
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
            } => f
                .debug_struct("BecomeCopy")
                .field("target", target)
                .field("source", source)
                .field("duration", duration)
                .field("preserve_source_abilities", preserve_source_abilities)
                .finish(),
            Self::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
            } => f
                .debug_struct("GrantAbilitiesAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
                .finish(),
            Self::RemoveAbilitiesAll {
                filter,
                abilities,
                duration,
            } => f
                .debug_struct("RemoveAbilitiesAll")
                .field("filter", filter)
                .field("abilities", abilities)
                .field("duration", duration)
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
            } => f
                .debug_struct("GrantAbilitiesToTarget")
                .field("target", target)
                .field("abilities", abilities)
                .field("duration", duration)
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
                all_tag,
                match_tag,
            } => f
                .debug_struct("ConsultTopOfLibrary")
                .field("player", player)
                .field("mode", mode)
                .field("filter", filter)
                .field("stop_rule", stop_rule)
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
            Self::CreateTokenWithMods { name, count, player, .. } => f
                .debug_struct("CreateTokenWithMods")
                .field("name", name)
                .field("count", count)
                .field("player", player)
                .finish(),
            Self::RedirectNextDamageFromSourceToTarget { amount, target } => f
                .debug_struct("RedirectNextDamageFromSourceToTarget")
                .field("amount", amount)
                .field("target", target)
                .finish(),
            Self::RedirectNextTimeDamageToSource {
                source,
                target,
                all_this_turn,
            } => f
                .debug_struct("RedirectNextTimeDamageToSource")
                .field("source", source)
                .field("target", target)
                .field("all_this_turn", all_this_turn)
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
                reveal,
                progress_tag,
            } => f
                .debug_struct("SearchLibrarySlotsToHand")
                .field("slots", slots)
                .field("reveal", reveal)
                .field("progress_tag", progress_tag)
                .finish(),
            Self::RevealTopChooseCardTypePutToHandRestBottom { count } => f
                .debug_tuple("RevealTopChooseCardTypePutToHandRestBottom")
                .field(count)
                .finish(),
            Self::RevealTopPutMatchingIntoHandRestIntoGraveyard { count, filter } => f
                .debug_struct("RevealTopPutMatchingIntoHandRestIntoGraveyard")
                .field("count", count)
                .field("filter", filter)
                .finish(),
            Self::RevealTopPutMatchingIntoHandRestOnBottomOfLibrary {
                count,
                filter,
                order,
            } => f
                .debug_struct("RevealTopPutMatchingIntoHandRestOnBottomOfLibrary")
                .field("count", count)
                .field("filter", filter)
                .field("order", order)
                .finish(),
            Self::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
                filter,
                reveal,
                if_not_chosen,
            } => f
                .debug_struct("ChooseFromLookedCardsIntoHandRestIntoGraveyard")
                .field("filter", filter)
                .field("reveal", reveal)
                .field("if_not_chosen", if_not_chosen)
                .finish(),
            Self::ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary {
                spell_filter,
                order,
            } => f
                .debug_struct("ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary")
                .field("spell_filter", spell_filter)
                .field("order", order)
                .finish(),
            Self::ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary { order } => f
                .debug_struct("ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary")
                .field("order", order)
                .finish(),
            Self::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
                battlefield_filter,
                tapped,
            } => f
                .debug_struct("ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary")
                .field("battlefield_filter", battlefield_filter)
                .field("tapped", tapped)
                .finish(),
            Self::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
                battlefield_filter,
                hand_filter,
                tapped,
                order,
            } => f
                .debug_struct("ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary")
                .field("battlefield_filter", battlefield_filter)
                .field("hand_filter", hand_filter)
                .field("tapped", tapped)
                .field("order", order)
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
            Self::DealDamage { amount, target } => f
                .debug_struct("DealDamage")
                .field("amount", amount)
                .field("target", target)
                .finish(),
            Self::DealDamageEach { amount, filter } => f
                .debug_struct("DealDamageEach")
                .field("amount", amount)
                .field("filter", filter)
                .finish(),
            Self::DealDamageEqualToPower { source, target } => f
                .debug_struct("DealDamageEqualToPower")
                .field("source", source)
                .field("target", target)
                .finish(),
            Self::DealDistributedDamage { amount, target } => f
                .debug_struct("DealDistributedDamage")
                .field("amount", amount)
                .field("target", target)
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
            Self::PhaseOut { target } => f.debug_tuple("PhaseOut").field(target).finish(),
            Self::PhaseOutAll { filter } => f.debug_tuple("PhaseOutAll").field(filter).finish(),
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
            Self::Exile { target, face_down } => f
                .debug_struct("Exile")
                .field("target", target)
                .field("face_down", face_down)
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
            } => f
                .debug_struct("RemoveUpToAnyCounters")
                .field("amount", amount)
                .field("target", target)
                .field("counter_type", counter_type)
                .field("up_to", up_to)
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
            Self::ForEachCounterKindPutOrRemove { target } => f
                .debug_struct("ForEachCounterKindPutOrRemove")
                .field("target", target)
                .finish(),
            Self::ReturnToHand { target, random } => f
                .debug_struct("ReturnToHand")
                .field("target", target)
                .field("random", random)
                .finish(),
            Self::ReturnAllToHand { filter } => f
                .debug_struct("ReturnAllToHand")
                .field("filter", filter)
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
            Self::TicketCounters { count } => f.debug_tuple("TicketCounters").field(count).finish(),
            Self::PayEnergy { amount } => f.debug_tuple("PayEnergy").field(amount).finish(),
            Self::PayAnyEnergy { min_amount } => f
                .debug_struct("PayAnyEnergy")
                .field("min_amount", min_amount)
                .finish(),
            Self::PayMana { cost } => f.debug_tuple("PayMana").field(cost).finish(),
            Self::DoubleManaPool => f.write_str("DoubleManaPool"),
            Self::EmptyManaPool => f.write_str("EmptyManaPool"),
            Self::SetLifeTotal { amount } => f.debug_tuple("SetLifeTotal").field(amount).finish(),
            Self::EndTurn => f.write_str("EndTurn"),
            Self::SkipTurn => f.write_str("SkipTurn"),
            Self::SkipCombatPhases => f.write_str("SkipCombatPhases"),
            Self::SkipNextCombatPhaseThisTurn => f.write_str("SkipNextCombatPhaseThisTurn"),
            Self::SkipDrawStep => f.write_str("SkipDrawStep"),
            Self::AdditionalPhases { phases } => f
                .debug_tuple("AdditionalPhases")
                .field(phases)
                .finish(),
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
            Self::CreateEmblem { text } => f.debug_tuple("CreateEmblem").field(text).finish(),
            Self::LoseGame => f.write_str("LoseGame"),
            Self::WinGame => f.write_str("WinGame"),
            Self::Detain { target } => f.debug_tuple("Detain").field(target).finish(),
            Self::Goad { target } => f.debug_tuple("Goad").field(target).finish(),
            Self::Suspect { target } => f.debug_tuple("Suspect").field(target).finish(),
            Self::ClearSuspected { target } => f
                .debug_tuple("ClearSuspected")
                .field(target)
                .finish(),
            Self::RemoveFromCombat { target } => {
                f.debug_tuple("RemoveFromCombat").field(target).finish()
            }
            Self::Flip { target } => f.debug_tuple("Flip").field(target).finish(),
            Self::Regenerate { target } => f.debug_tuple("Regenerate").field(target).finish(),
            Self::RegenerateAll { filter } => f.debug_tuple("RegenerateAll").field(filter).finish(),
            Self::Sacrifice {
                filter,
                count,
                target,
            } => f
                .debug_struct("Sacrifice")
                .field("filter", filter)
                .field("count", count)
                .field("target", target)
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EffectAst {
    SubjectVerb(SubjectVerbEffectAst),
    Sequence {
        effects: Vec<EffectAst>,
    },
    UnlessPays {
        effects: Vec<EffectAst>,
        player: PlayerAst,
        cost: TotalCost,
    },
    UnlessAction {
        effects: Vec<EffectAst>,
        alternative: Vec<EffectAst>,
        player: PlayerAst,
    },
    DelayedUntilNextEndStep {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextUpkeep {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextDrawStep {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilEndStepOfExtraTurn {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilEndOfCombat {
        effects: Vec<EffectAst>,
    },
    DelayedTriggerThisTurn {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
    },
    DelayedWhenLastObjectDiesThisTurn {
        filter: Option<ObjectFilter>,
        effects: Vec<EffectAst>,
    },
    Conditional {
        predicate: PredicateAst,
        if_true: Vec<EffectAst>,
        if_false: Vec<EffectAst>,
    },
    ManaRestricted {
        effects: Vec<EffectAst>,
        restrictions: Vec<crate::ability::ManaUsageRestriction>,
    },
    SelfReplacement {
        predicate: PredicateAst,
        if_true: Vec<EffectAst>,
        if_false: Vec<EffectAst>,
    },
    ChooseObjects {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagKey,
    },
    ChooseObjectsAcrossZones {
        filter: ObjectFilter,
        count: ChoiceCount,
        count_value: Option<Value>,
        player: PlayerAst,
        tag: TagKey,
        zones: Vec<Zone>,
        search_mode: Option<crate::effect::SearchSelectionMode>,
    },
    MayCastMatchingSpellWithoutPayingManaCost {
        player: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
        payment: ironsmith_core::MayCastMatchingSpellPayment,
    },
    RepeatThisProcess,
    RepeatThisProcessMay,
    RepeatThisProcessOnce,
    RepeatEffects {
        count: Value,
        effects: Vec<EffectAst>,
    },
    May {
        effects: Vec<EffectAst>,
    },
    MayByPlayer {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    ResolvedIfResult {
        condition: EffectId,
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    ResolvedWhenResult {
        condition: EffectId,
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    IfResult {
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    WhenResult {
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    ForEachOpponent {
        effects: Vec<EffectAst>,
    },
    ForEachPlayersFiltered {
        filter: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    ForEachPlayer {
        effects: Vec<EffectAst>,
    },
    ForEachTargetPlayers {
        count: ChoiceCount,
        effects: Vec<EffectAst>,
    },
    ForEachObject {
        filter: ObjectFilter,
        effects: Vec<EffectAst>,
    },
    ForEachTagged {
        tag: TagKey,
        effects: Vec<EffectAst>,
    },
    ForEachOpponentDoesNot {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
    },
    ForEachPlayerDoesNot {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
    },
    ForEachOpponentDid {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
    },
    ForEachPlayerDid {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
    },
    ForEachTaggedPlayer {
        tag: TagKey,
        effects: Vec<EffectAst>,
    },
    RepeatProcess {
        effects: Vec<EffectAst>,
        continue_effect_index: usize,
        continue_predicate: IfResultPredicate,
    },
    VoteStart {
        options: Vec<String>,
        secret: bool,
    },
    VoteStartObjects {
        filter: ObjectFilter,
        count: ChoiceCount,
        secret: bool,
    },
    VoteOption {
        option: String,
        effects: Vec<EffectAst>,
    },
    VoteExtra {
        count: u32,
        optional: bool,
    },
}

impl EffectAst {
    pub(crate) fn subject_verb(
        role: SubjectVerbRoleAst,
        player: PlayerAst,
        action: SubjectVerbActionAst,
    ) -> Self {
        Self::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { role, player },
            action,
        })
    }

    pub(crate) fn subject_verb_draw_for_each_tagged_matching(
        player: PlayerAst,
        tag: TagKey,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::DrawForEachTaggedMatching { tag, filter },
        )
    }

    pub(crate) fn subject_verb_grant_next_spell_ability_this_turn(
        player: PlayerAst,
        filter: ObjectFilter,
        ability: GrantedAbilityAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::GrantNextSpellAbilityThisTurn { filter, ability },
        )
    }

    pub(crate) fn subject_verb_put_some_into_hand_rest_into_graveyard(
        player: PlayerAst,
        count: u32,
    ) -> Self {
        Self::subject_verb_put_some_into_hand_rest_into_graveyard_with_count(
            player,
            ChoiceCount::exactly(count as usize),
        )
    }

    pub(crate) fn subject_verb_put_some_into_hand_rest_into_graveyard_with_count(
        player: PlayerAst,
        count: ChoiceCount,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::PutSomeIntoHandRestIntoGraveyard { count },
        )
    }

    pub(crate) fn subject_verb_may_move_to_zone(
        player: PlayerAst,
        target: TargetAst,
        zone: Zone,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::MayMoveToZone { target, zone },
        )
    }

    pub(crate) fn subject_verb_put_some_into_hand_rest_on_bottom_of_library(
        player: PlayerAst,
        count: u32,
    ) -> Self {
        Self::subject_verb_put_some_into_hand_rest_on_bottom_of_library_with_count(
            player,
            ChoiceCount::exactly(count as usize),
        )
    }

    pub(crate) fn subject_verb_put_some_into_hand_rest_on_bottom_of_library_with_count(
        player: PlayerAst,
        count: ChoiceCount,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::PutSomeIntoHandRestOnBottomOfLibrary { count },
        )
    }

    pub(crate) fn subject_verb_grant_protection_choice(
        target: TargetAst,
        allow_colorless: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantProtectionChoice {
                target,
                allow_colorless,
            },
        )
    }

    pub(crate) fn subject_verb_prevent_all_combat_damage(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllCombatDamage { duration },
        )
    }

    pub(crate) fn subject_verb_prevent_all_combat_damage_from_source(
        source: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllCombatDamageFromSource { duration, source },
        )
    }

    pub(crate) fn subject_verb_prevent_all_combat_damage_from_source_filter(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter {
                duration,
                source_filter,
            },
        )
    }

    pub(crate) fn subject_verb_prevent_all_combat_damage_to_players(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllCombatDamageToPlayers { duration },
        )
    }

    pub(crate) fn subject_verb_prevent_all_combat_damage_to_you(duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllCombatDamageToYou { duration },
        )
    }

    pub(crate) fn subject_verb_prevent_next_time_damage(
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
    ) -> Self {
        Self::subject_verb_prevent_next_time_damage_with_reflection(source, target, false)
    }

    pub(crate) fn subject_verb_prevent_next_time_damage_with_reflection(
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
        reflect_damage_to_source_controller: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventNextTimeDamage {
                source,
                target,
                reflect_damage_to_source_controller,
                follow_up_effects: Vec::new(),
            },
        )
    }

    pub(crate) fn subject_verb_prevent_damage(
        amount: Value,
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb_prevent_damage_with_source_choice(amount, target, duration, false)
    }

    pub(crate) fn subject_verb_prevent_damage_with_source_choice(
        amount: Value,
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventDamage {
                amount,
                target,
                duration,
                source_of_your_choice,
            },
        )
    }

    pub(crate) fn subject_verb_prevent_all_damage_to_target(
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllDamageToTarget { target, duration },
        )
    }

    pub(crate) fn subject_verb_prevent_all_damage_from_source_filter(
        source_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventAllDamageFromSourceFilter {
                duration,
                source_filter,
            },
        )
    }

    pub(crate) fn subject_verb_prevent_damage_to_target_put_counters(
        amount: Option<Value>,
        target: TargetAst,
        duration: Until,
        counter_type: CounterType,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventDamageToTargetPutCounters {
                amount,
                target,
                duration,
                counter_type,
            },
        )
    }

    pub(crate) fn subject_verb_prevent_damage_each(
        amount: Value,
        filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PreventDamageEach {
                amount,
                filter,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_copy_spell(
        target: TargetAst,
        count: Value,
        player: PlayerAst,
        may_choose_new_targets: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::CopySpell {
                target,
                count,
                player,
                may_choose_new_targets,
                removed_supertypes,
            },
        )
    }

    pub(crate) fn subject_verb_copy_spell_for_each_target(
        target: TargetAst,
        object_filter: Option<ObjectFilter>,
        player_filter: Option<PlayerFilter>,
        player: PlayerAst,
        exclude_current_targets: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::CopySpellForEachTarget {
                target,
                object_filter,
                player_filter,
                player,
                exclude_current_targets,
                removed_supertypes,
            },
        )
    }

    pub(crate) fn subject_verb_put_tagged_remainder_on_bottom_of_library(
        tag: TagKey,
        keep_tagged: Option<TagKey>,
        order: LibraryBottomOrderAst,
        player: PlayerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag,
                keep_tagged,
                order,
                player,
            },
        )
    }

    pub(crate) fn subject_verb_cast_tagged(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        as_copy: bool,
        without_paying_mana_cost: bool,
        cost_reduction: Option<ManaCost>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::CastTagged {
                tag,
                player,
                allow_land,
                as_copy,
                without_paying_mana_cost,
                cost_reduction,
            },
        )
    }

    pub(crate) fn may_cast_matching_spell_without_paying_mana_cost(
        player: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
    ) -> Self {
        Self::MayCastMatchingSpellWithoutPayingManaCost {
            player,
            filter,
            zone,
            payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
        }
    }

    pub(crate) fn may_cast_matching_spell_with_alternative_cost(
        player: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
        kind: crate::filter::AlternativeCastKind,
    ) -> Self {
        Self::MayCastMatchingSpellWithoutPayingManaCost {
            player,
            filter,
            zone,
            payment: ironsmith_core::MayCastMatchingSpellPayment::AlternativeCost(kind),
        }
    }

    pub(crate) fn subject_verb_grant_play_tagged_until_end_of_turn(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
            },
        )
    }

    pub(crate) fn subject_verb_grant_tagged_spell_alternative_cost_pay_life_by_mana_value_until_end_of_turn(
        tag: TagKey,
        player: PlayerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                player,
            },
        )
    }

    pub(crate) fn subject_verb_grant_play_tagged_until_your_next_turn(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
            },
        )
    }

    pub(crate) fn subject_verb_grant_play_tagged_for_as_long_as_exiled(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
            },
        )
    }

    pub(crate) fn subject_verb_grant_play_tagged_for_as_long_as_you_control_source(
        tag: TagKey,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
                tag,
                player,
                allow_land,
                allow_any_color_for_cast,
            },
        )
    }

    pub(crate) fn subject_verb_return_to_battlefield(
        target: TargetAst,
        tapped: bool,
        transformed: bool,
        converted: bool,
        controller: ReturnControllerAst,
        count_value: Option<Value>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReturnToBattlefield {
                target,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura: None,
            },
        )
    }

    pub(crate) fn subject_verb_return_all_to_battlefield(
        filter: ObjectFilter,
        tapped: bool,
        controller: ReturnControllerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                controller,
            },
        )
    }

    pub(crate) fn subject_verb_exile_until_source_leaves(
        target: TargetAst,
        face_down: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExileUntilSourceLeaves { target, face_down },
        )
    }

    pub(crate) fn subject_verb_move_to_zone(
        target: TargetAst,
        zone: Zone,
        to_top: bool,
        battlefield_controller: ReturnControllerAst,
        battlefield_tapped: bool,
        attached_to: Option<TargetAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MoveToZone {
                target,
                zone,
                to_top,
                battlefield_controller,
                battlefield_tapped,
                attached_to,
            },
        )
    }

    pub(crate) fn subject_verb_move_to_library_top_or_bottom_choice(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target },
        )
    }

    pub(crate) fn subject_verb_target_only(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TargetOnly { target },
        )
    }

    pub(crate) fn subject_verb_tag_matching_objects(
        filter: ObjectFilter,
        zones: Vec<Zone>,
        tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TagMatchingObjects { filter, zones, tag },
        )
    }

    pub(crate) fn subject_verb_pump(
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Pump {
                power,
                toughness,
                target,
                duration,
                condition,
            },
        )
    }

    pub(crate) fn subject_verb_set_base_power_toughness(
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn subject_verb_become_base_pt_creature(
        power: Value,
        toughness: Value,
        target: TargetAst,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
        colors: Option<ColorSet>,
        abilities: Vec<StaticAbility>,
        granted_abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::BecomeBasePtCreature {
                power,
                toughness,
                target,
                card_types,
                subtypes,
                colors,
                abilities,
                granted_abilities,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_set_base_power(
        power: Value,
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::SetBasePower {
                power,
                target,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_pump_for_each(
        power_per: i32,
        toughness_per: i32,
        target: TargetAst,
        count: Value,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PumpForEach {
                power_per,
                toughness_per,
                target,
                count,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_pump_all(
        filter: ObjectFilter,
        power: Value,
        toughness: Value,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PumpAll {
                filter,
                power,
                toughness,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_pump_by_last_effect(
        power: i32,
        toughness: i32,
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PumpByLastEffect {
                power,
                toughness,
                target,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_add_card_types(
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AddCardTypes {
                target,
                card_types,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_remove_card_types(
        target: TargetAst,
        card_types: Vec<CardType>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RemoveCardTypes {
                target,
                card_types,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_add_subtypes(
        target: TargetAst,
        subtypes: Vec<Subtype>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AddSubtypes {
                target,
                subtypes,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_add_colors(
        target: TargetAst,
        colors: ColorSet,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AddColors {
                target,
                colors,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_add_all_subtypes_of_family(
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AddAllSubtypesOfFamily {
                target,
                family,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_remove_all_subtypes_of_family(
        target: TargetAst,
        family: SubtypeFamily,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
                target,
                family,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_become_aura_enchantment(
        target: TargetAst,
        attachment_filter: ObjectFilter,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::BecomeAuraEnchantment {
                target,
                attachment_filter,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_become_basic_land_type(
        target: TargetAst,
        subtype: Subtype,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::BecomeBasicLandType {
                target,
                subtype,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_set_colors(
        target: TargetAst,
        colors: ColorSet,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::SetColors {
                target,
                colors,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_make_colorless(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MakeColorless { target, duration },
        )
    }

    pub(crate) fn subject_verb_become_basic_land_type_choice(
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::BecomeBasicLandTypeChoice { target, duration },
        )
    }

    pub(crate) fn subject_verb_become_creature_type_choice(
        target: TargetAst,
        duration: Until,
        excluded_subtypes: Vec<Subtype>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::BecomeCreatureTypeChoice {
                target,
                duration,
                excluded_subtypes,
            },
        )
    }

    pub(crate) fn subject_verb_become_color_choice(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::BecomeColorChoice { target, duration },
        )
    }

    pub(crate) fn subject_verb_become_copy(
        target: TargetAst,
        source: TargetAst,
        duration: Until,
        preserve_source_abilities: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::BecomeCopy {
                target,
                source,
                duration,
                preserve_source_abilities,
            },
        )
    }

    pub(crate) fn subject_verb_grant_abilities_all(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantAbilitiesAll {
                filter,
                abilities,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_remove_abilities_all(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RemoveAbilitiesAll {
                filter,
                abilities,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_grant_abilities_choice_all(
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantAbilitiesChoiceAll {
                filter,
                abilities,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_grant_abilities_to_target(
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_grant_to_target(
        target: TargetAst,
        grantable: crate::grant::Grantable,
        duration: crate::grant::GrantDuration,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantToTarget {
                target,
                grantable,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_grant_by_spec(
        spec: crate::grant::GrantSpec,
        player: PlayerAst,
        duration: crate::grant::GrantDuration,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::GrantBySpec {
                spec,
                player,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_remove_abilities_from_target(
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RemoveAbilitiesFromTarget {
                target,
                abilities,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_grant_abilities_choice_to_target(
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
                target,
                abilities,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_consult_top_of_library(
        player: PlayerAst,
        mode: LibraryConsultModeAst,
        filter: ObjectFilter,
        stop_rule: LibraryConsultStopRuleAst,
        all_tag: TagKey,
        match_tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ConsultTopOfLibrary {
                player,
                mode,
                filter,
                stop_rule,
                all_tag,
                match_tag,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn subject_verb_search_library(
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
        tapped: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            chooser,
            SubjectVerbActionAst::SearchLibrary {
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
                tapped,
            },
        )
    }

    pub(crate) fn subject_verb_cant(
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        condition: Option<crate::ConditionExpr>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Cant {
                restriction,
                duration,
                condition,
            },
        )
    }

    pub(crate) fn subject_verb_redirect_next_damage_from_source_to_target(
        amount: Value,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget { amount, target },
        )
    }

    pub(crate) fn subject_verb_redirect_next_time_damage_to_source(
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RedirectNextTimeDamageToSource {
                source,
                target,
                all_this_turn: false,
            },
        )
    }

    pub(crate) fn subject_verb_redirect_all_damage_this_turn_to_source(
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RedirectNextTimeDamageToSource {
                source,
                target,
                all_this_turn: true,
            },
        )
    }

    pub(crate) fn subject_verb_redirect_all_damage_this_turn_to_target(
        player_filter: PlayerFilter,
        object_filter: ObjectFilter,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RedirectAllDamageThisTurnToTarget {
                player_filter,
                object_filter,
                target,
            },
        )
    }

    pub(crate) fn subject_verb_meld(
        result_name: impl Into<String>,
        enters_tapped: bool,
        enters_attacking: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Meld {
                result_name: result_name.into(),
                enters_tapped,
                enters_attacking,
            },
        )
    }

    pub(crate) fn subject_verb_search_library_slots_to_hand(
        player: PlayerAst,
        slots: Vec<SearchLibrarySlotAst>,
        reveal: bool,
        progress_tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::SearchLibrarySlotsToHand {
                slots,
                reveal,
                progress_tag,
            },
        )
    }

    pub(crate) fn subject_verb_reveal_top_choose_card_type_put_to_hand_rest_bottom(
        player: PlayerAst,
        count: u32,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::RevealTopChooseCardTypePutToHandRestBottom { count },
        )
    }

    pub(crate) fn subject_verb_reveal_top_put_matching_into_hand_rest_into_graveyard(
        player: PlayerAst,
        count: u32,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestIntoGraveyard { count, filter },
        )
    }

    pub(crate) fn subject_verb_reveal_top_put_matching_into_hand_rest_on_bottom_of_library(
        player: PlayerAst,
        count: u32,
        filter: ObjectFilter,
        order: LibraryBottomOrderAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestOnBottomOfLibrary {
                count,
                filter,
                order,
            },
        )
    }

    pub(crate) fn subject_verb_choose_from_looked_cards_into_hand_rest_into_graveyard(
        player: PlayerAst,
        filter: ObjectFilter,
        reveal: bool,
        if_not_chosen: Vec<EffectAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
                filter,
                reveal,
                if_not_chosen,
            },
        )
    }

    pub(crate) fn subject_verb_choose_from_looked_cards_for_each_card_type_among_spells_cast_this_turn_into_hand_rest_on_bottom_of_library(
        player: PlayerAst,
        spell_filter: ObjectFilter,
        order: LibraryBottomOrderAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary {
                spell_filter,
                order,
            },
        )
    }

    pub(crate) fn subject_verb_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom_of_library(
        player: PlayerAst,
        order: LibraryBottomOrderAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary {
                order,
            },
        )
    }

    pub(crate) fn subject_verb_choose_from_looked_cards_onto_battlefield_or_into_hand_rest_on_bottom_of_library(
        player: PlayerAst,
        battlefield_filter: ObjectFilter,
        tapped: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
                battlefield_filter,
                tapped,
            },
        )
    }

    pub(crate) fn subject_verb_choose_from_looked_cards_onto_battlefield_and_into_hand_rest_on_bottom_of_library(
        player: PlayerAst,
        battlefield_filter: ObjectFilter,
        hand_filter: ObjectFilter,
        tapped: bool,
        order: LibraryBottomOrderAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
                battlefield_filter,
                hand_filter,
                tapped,
                order,
            },
        )
    }

    pub(crate) fn subject_verb_retarget_stack_object(
        chooser: PlayerAst,
        target: TargetAst,
        mode: RetargetModeAst,
        require_change: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            chooser,
            SubjectVerbActionAst::RetargetStackObject {
                target,
                mode,
                require_change,
            },
        )
    }

    pub(crate) fn subject_verb_grant_ability_to_source(
        ability: ParsedAbility,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantAbilityToSource { ability, duration },
        )
    }

    pub(crate) fn subject_verb_exchange_control(
        filter: ObjectFilter,
        count: u32,
        shared_type: Option<SharedTypeConstraintAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExchangeControl {
                filter,
                count,
                shared_type,
            },
        )
    }

    pub(crate) fn subject_verb_exchange_control_heterogeneous(
        permanent1: TargetAst,
        permanent2: TargetAst,
        shared_type: Option<SharedTypeConstraintAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExchangeControlHeterogeneous {
                permanent1,
                permanent2,
                shared_type,
            },
        )
    }

    pub(crate) fn subject_verb_destroy_all_attached_to(
        filter: ObjectFilter,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DestroyAllAttachedTo { filter, target },
        )
    }

    pub(crate) fn subject_verb_attach(object: TargetAst, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Attach { object, target },
        )
    }

    pub(crate) fn subject_verb_enchant(filter: AuraAttachmentFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Enchant { filter },
        )
    }

    pub(crate) fn subject_verb_exile_when_source_leaves(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExileWhenSourceLeaves { target },
        )
    }

    pub(crate) fn subject_verb_sacrifice_source_when_leaves(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::SacrificeSourceWhenLeaves { target },
        )
    }

    pub(crate) fn subject_verb_register_zone_replacement(
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RegisterZoneReplacement {
                target,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
                optional: false,
                choice_description: None,
            },
        )
    }

    pub(crate) fn subject_verb_register_future_zone_replacement(
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RegisterFutureZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_register_damaged_by_source_zone_replacement(
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RegisterDamagedBySourceZoneReplacement {
                filter,
                from_zone,
                to_zone,
                replacement_zone,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_register_enter_under_control_replacement(
        filter: ObjectFilter,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RegisterEnterUnderControlReplacement { filter, duration },
        )
    }

    pub(crate) fn subject_verb_register_enter_with_counters_replacement(
        filter: ObjectFilter,
        counter_type: CounterType,
        count: Value,
        duration: ZoneReplacementDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RegisterEnterWithCountersReplacement {
                filter,
                counter_type,
                count,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_choose_spell_cast_history(
        chooser: PlayerAst,
        cast_by: PlayerAst,
        filter: ObjectFilter,
        tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            chooser,
            SubjectVerbActionAst::ChooseSpellCastHistory {
                cast_by,
                filter,
                tag,
            },
        )
    }

    pub(crate) fn subject_verb_damage(amount: Value, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DealDamage { amount, target },
        )
    }

    pub(crate) fn subject_verb_damage_each(amount: Value, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DealDamageEach { amount, filter },
        )
    }

    pub(crate) fn subject_verb_damage_equal_to_power(source: TargetAst, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DealDamageEqualToPower { source, target },
        )
    }

    pub(crate) fn subject_verb_distributed_damage(amount: Value, target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DealDistributedDamage { amount, target },
        )
    }

    pub(crate) fn subject_verb_proliferate(count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Proliferate { count },
        )
    }

    pub(crate) fn subject_verb_investigate(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Investigate { count },
        )
    }

    pub(crate) fn subject_verb_incubate(player: PlayerAst, amount: Value, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Incubate { amount, count },
        )
    }

    pub(crate) fn subject_verb_learn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Learn,
        )
    }

    pub(crate) fn subject_verb_emit_keyword_action(
        action: crate::events::KeywordActionKind,
        amount: u32,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::EmitKeywordAction { action, amount },
        )
    }

    pub(crate) fn subject_verb_amass(subtype: Option<Subtype>, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Amass { subtype, amount },
        )
    }

    pub(crate) fn subject_verb_bolster(amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Bolster { amount },
        )
    }

    pub(crate) fn subject_verb_support(amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Support { amount },
        )
    }

    pub(crate) fn subject_verb_adapt(amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Adapt { amount },
        )
    }

    pub(crate) fn subject_verb_monstrosity(amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Monstrosity { amount },
        )
    }

    pub(crate) fn subject_verb_discover(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Discover { count },
        )
    }

    pub(crate) fn subject_verb_fateseal(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Fateseal { count },
        )
    }

    pub(crate) fn subject_verb_populate(count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Populate {
                count,
                enters_tapped: false,
                enters_attacking: false,
                has_haste: false,
                sacrifice_at_next_end_step: false,
                exile_at_next_end_step: false,
                exile_at_end_of_combat: false,
                sacrifice_at_end_of_combat: false,
            },
        )
    }

    pub(crate) fn subject_verb_explore(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Explore { target },
        )
    }

    pub(crate) fn subject_verb_endure(target: TargetAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Endure { target, amount },
        )
    }

    pub(crate) fn subject_verb_exploit() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exploit,
        )
    }

    pub(crate) fn subject_verb_connive(target: TargetAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Connive { target, count },
        )
    }

    pub(crate) fn subject_verb_connive_iterated() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ConniveIterated,
        )
    }

    pub(crate) fn subject_verb_put_rest_on_bottom_of_library() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutRestOnBottomOfLibrary,
        )
    }

    pub(crate) fn subject_verb_dont_lose_this_mana_as_steps_and_phases_end_this_turn() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn,
        )
    }

    pub(crate) fn subject_verb_open_attraction(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::OpenAttraction,
        )
    }

    pub(crate) fn subject_verb_manifest_top_card(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ManifestTopCardOfLibrary,
        )
    }

    pub(crate) fn subject_verb_manifest_from_hand(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ManifestCardFromHand,
        )
    }

    pub(crate) fn subject_verb_manifest_dread(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ManifestDread,
        )
    }

    pub(crate) fn subject_verb_earthbend(counters: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Earthbend { counters },
        )
    }

    pub(crate) fn subject_verb_behold(subtype: Subtype, count: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Behold { subtype, count },
        )
    }

    pub(crate) fn subject_verb_fight(creature1: TargetAst, creature2: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Fight {
                creature1,
                creature2,
            },
        )
    }

    pub(crate) fn subject_verb_fight_iterated(creature2: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::FightIterated { creature2 },
        )
    }

    pub(crate) fn subject_verb_clash(opponent: ClashOpponentAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Clash { opponent },
        )
    }

    pub(crate) fn subject_verb_add_mana(player: PlayerAst, mana: Vec<ManaSymbol>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddMana { mana },
        )
    }

    pub(crate) fn subject_verb_add_mana_scaled(
        player: PlayerAst,
        mana: Vec<ManaSymbol>,
        amount: Value,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaScaled { mana, amount },
        )
    }

    pub(crate) fn subject_verb_add_mana_any_color(
        player: PlayerAst,
        amount: Value,
        available_colors: Option<Vec<crate::color::Color>>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaAnyColor {
                amount,
                available_colors,
            },
        )
    }

    pub(crate) fn subject_verb_add_mana_any_one_color(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaAnyOneColor { amount },
        )
    }

    pub(crate) fn subject_verb_add_mana_chosen_color(
        player: PlayerAst,
        amount: Value,
        fixed_option: Option<crate::color::Color>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaChosenColor {
                amount,
                fixed_option,
            },
        )
    }

    pub(crate) fn subject_verb_add_mana_from_land_could_produce(
        player: PlayerAst,
        amount: Value,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaFromLandCouldProduce {
                amount,
                land_filter,
                allow_colorless,
                same_type,
            },
        )
    }

    pub(crate) fn subject_verb_add_mana_colors_among(
        player: PlayerAst,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaColorsAmong { filter },
        )
    }

    pub(crate) fn subject_verb_add_mana_commander_identity(
        player: PlayerAst,
        amount: Value,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AddManaCommanderIdentity { amount },
        )
    }

    pub(crate) fn subject_verb_exchange_life_totals(
        player1: PlayerAst,
        player2: PlayerAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player1,
            SubjectVerbActionAst::ExchangeLifeTotals { player2 },
        )
    }

    pub(crate) fn subject_verb_exchange_text_boxes(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExchangeTextBoxes { target },
        )
    }

    pub(crate) fn subject_verb_exchange_zones(player: PlayerAst, zone1: Zone, zone2: Zone) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ExchangeZones { zone1, zone2 },
        )
    }

    pub(crate) fn subject_verb_exchange_values(
        left: ExchangeValueAst,
        right: ExchangeValueAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExchangeValues {
                left,
                right,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_exile_instead_of_graveyard_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ExileInsteadOfGraveyardThisTurn,
        )
    }

    pub(crate) fn subject_verb_control_combat_choices_this_turn(
        attackers: bool,
        blockers: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ControlCombatChoicesThisTurn {
                attackers,
                blockers,
            },
        )
    }

    pub(crate) fn subject_verb_control_player(
        player: PlayerAst,
        target: PlayerFilter,
        duration: ControlDurationAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::ControlPlayer {
                player: target,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_reduce_next_spell_cost_this_turn(
        player: PlayerAst,
        filter: ObjectFilter,
        reduction: ManaCost,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ReduceNextSpellCostThisTurn { filter, reduction },
        )
    }

    pub(crate) fn subject_verb_gain_control(
        player: PlayerAst,
        target: TargetAst,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::GainControl { target, duration },
        )
    }

    pub(crate) fn subject_verb_reveal_top(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::RevealTop,
        )
    }

    pub(crate) fn subject_verb_exile_top_of_library(
        player: PlayerAst,
        count: Value,
        tags: Vec<TagKey>,
        accumulated_tags: Vec<TagKey>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ExileTopOfLibrary {
                count,
                tags,
                accumulated_tags,
            },
        )
    }

    pub(crate) fn subject_verb_reveal_tagged(tag: TagKey) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RevealTagged { tag },
        )
    }

    pub(crate) fn subject_verb_reveal_cards_from_hand(
        player: PlayerAst,
        count: ChoiceCount,
        count_value: Option<Value>,
        tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RevealCardsFromHand {
                count,
                count_value,
                tag,
            },
        )
    }

    pub(crate) fn subject_verb_look_at_top_cards(
        player: PlayerAst,
        count: Value,
        tag: TagKey,
    ) -> Self {
        Self::subject_verb_top_library_cards(player, count, tag, false)
    }

    pub(crate) fn subject_verb_reveal_top_cards(
        player: PlayerAst,
        count: Value,
        tag: TagKey,
    ) -> Self {
        Self::subject_verb_top_library_cards(player, count, tag, true)
    }

    pub(crate) fn subject_verb_look_at_objects(player: PlayerAst, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LookAtObjects { filter },
        )
    }

    fn subject_verb_top_library_cards(
        player: PlayerAst,
        count: Value,
        tag: TagKey,
        reveal: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::LookAtTopCards { count, tag, reveal },
        )
    }

    pub(crate) fn subject_verb_put_into_hand(player: PlayerAst, object: ObjectRefAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PutIntoHand { object },
        )
    }

    pub(crate) fn subject_verb_additional_land_plays(
        player: PlayerAst,
        count: Value,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::AdditionalLandPlays { count, duration },
        )
    }

    pub(crate) fn subject_verb_extra_turn_after_turn(
        player: PlayerAst,
        anchor: ExtraTurnAnchorAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ExtraTurnAfterTurn { anchor },
        )
    }

    pub(crate) fn subject_verb_rearrange_looked_cards_in_library(
        player: PlayerAst,
        tag: TagKey,
        count: ChoiceCount,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::RearrangeLookedCardsInLibrary { tag, count },
        )
    }

    pub(crate) fn subject_verb_reorder_top_of_library(tag: TagKey) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReorderTopOfLibrary { tag },
        )
    }

    pub(crate) fn subject_verb_shuffle_objects_into_library(
        player: PlayerAst,
        target: TargetAst,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleObjectsIntoLibrary { target },
        )
    }

    pub(crate) fn subject_verb_add_mana_imprinted_colors() -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AddManaImprintedColors,
        )
    }

    pub(crate) fn subject_verb_flip_coin(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::FlipCoin,
        )
    }

    pub(crate) fn subject_verb_roll_die(player: PlayerAst, sides: u32) -> Self {
        Self::subject_verb_roll_die_with_die_text(player, sides, None)
    }

    pub(crate) fn subject_verb_roll_die_with_die_text(
        player: PlayerAst,
        sides: u32,
        die_text: Option<String>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RollDie { sides, die_text },
        )
    }

    pub(crate) fn subject_verb_roll_dice_choose_result_with_die_text(
        player: PlayerAst,
        count: u32,
        sides: u32,
        die_text: Option<String>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RollDiceChooseResult {
                count,
                sides,
                die_text,
            },
        )
    }

    pub(crate) fn subject_verb_shuffle_hand_and_graveyard_into_library(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary,
        )
    }

    pub(crate) fn subject_verb_shuffle_graveyard_into_library(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ShuffleGraveyardIntoLibrary,
        )
    }

    pub(crate) fn subject_verb_reorder_graveyard(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::ReorderGraveyard,
        )
    }

    pub(crate) fn subject_verb_choose_color(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseColor,
        )
    }

    pub(crate) fn subject_verb_choose_card_type(player: PlayerAst, options: Vec<CardType>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseCardType { options },
        )
    }

    pub(crate) fn subject_verb_choose_named_option(
        player: PlayerAst,
        options: Vec<String>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseNamedOption { options },
        )
    }

    pub(crate) fn subject_verb_choose_creature_type(
        player: PlayerAst,
        excluded_subtypes: Vec<Subtype>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseCreatureType { excluded_subtypes },
        )
    }

    pub(crate) fn subject_verb_choose_card_name(
        player: PlayerAst,
        filter: Option<ObjectFilter>,
        tag: TagKey,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            player,
            SubjectVerbActionAst::ChooseCardName { filter, tag },
        )
    }

    pub(crate) fn subject_verb_choose_player(
        chooser: PlayerAst,
        filter: PlayerFilter,
        tag: TagKey,
        random: bool,
        exclude_previous_choices: usize,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Chooser,
            chooser,
            SubjectVerbActionAst::ChoosePlayer {
                filter,
                tag,
                random,
                exclude_previous_choices,
            },
        )
    }

    pub(crate) fn subject_verb_tap(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Tap { target },
        )
    }

    pub(crate) fn subject_verb_untap(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Untap { target },
        )
    }

    pub(crate) fn subject_verb_tap_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TapAll { filter },
        )
    }

    pub(crate) fn subject_verb_untap_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::UntapAll { filter },
        )
    }

    pub(crate) fn subject_verb_tap_or_untap(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TapOrUntap { target },
        )
    }

    pub(crate) fn subject_verb_tap_or_untap_all(
        tap_filter: ObjectFilter,
        untap_filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TapOrUntapAll {
                tap_filter,
                untap_filter,
            },
        )
    }

    pub(crate) fn subject_verb_phase_out(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PhaseOut { target },
        )
    }

    pub(crate) fn subject_verb_phase_out_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PhaseOutAll { filter },
        )
    }

    pub(crate) fn subject_verb_phase_in(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PhaseIn { target },
        )
    }

    pub(crate) fn subject_verb_phase_in_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PhaseInAll { filter },
        )
    }

    pub(crate) fn subject_verb_transform(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Transform { target },
        )
    }

    pub(crate) fn subject_verb_convert(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Convert { target },
        )
    }

    pub(crate) fn subject_verb_destroy(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Destroy {
                target,
                no_regeneration: false,
            },
        )
    }

    pub(crate) fn subject_verb_destroy_no_regeneration(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Destroy {
                target,
                no_regeneration: true,
            },
        )
    }

    pub(crate) fn subject_verb_destroy_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DestroyAll {
                filter,
                no_regeneration: false,
            },
        )
    }

    pub(crate) fn subject_verb_destroy_all_of_chosen_color(
        filter: ObjectFilter,
        no_regeneration: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DestroyAllOfChosenColor {
                filter,
                no_regeneration,
            },
        )
    }

    pub(crate) fn subject_verb_exile(target: TargetAst, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Exile { target, face_down },
        )
    }

    pub(crate) fn subject_verb_exile_all(filter: ObjectFilter, face_down: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ExileAll { filter, face_down },
        )
    }

    pub(crate) fn subject_verb_look_at_hand(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::LookAtHand { target },
        )
    }

    pub(crate) fn subject_verb_counter(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Counter { target },
        )
    }

    pub(crate) fn subject_verb_counter_unless_pays(target: TargetAst, cost: TotalCost) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::CounterUnlessPays { target, cost },
        )
    }

    pub(crate) fn subject_verb_put_counters(
        counter_type: CounterType,
        count: Value,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
        distributed: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutCounters {
                counter_type,
                count,
                target,
                target_count,
                distributed,
            },
        )
    }

    pub(crate) fn subject_verb_put_or_remove_counters(
        put_counter_type: CounterType,
        put_count: Value,
        remove_counter_type: CounterType,
        remove_count: Value,
        put_mode_text: impl Into<String>,
        remove_mode_text: impl Into<String>,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutOrRemoveCounters {
                put_counter_type,
                put_count,
                remove_counter_type,
                remove_count,
                put_mode_text: put_mode_text.into(),
                remove_mode_text: remove_mode_text.into(),
                target,
                target_count,
            },
        )
    }

    pub(crate) fn subject_verb_put_counters_all(
        counter_type: CounterType,
        count: Value,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutCountersAll {
                counter_type,
                count,
                filter,
            },
        )
    }

    pub(crate) fn subject_verb_remove_up_to_any_counters(
        amount: Value,
        target: TargetAst,
        counter_type: Option<CounterType>,
        up_to: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RemoveUpToAnyCounters {
                amount,
                target,
                counter_type,
                up_to,
            },
        )
    }

    pub(crate) fn subject_verb_move_all_counters(from: TargetAst, to: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MoveAllCounters { from, to },
        )
    }

    pub(crate) fn subject_verb_move_one_counter(from: TargetAst, to: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MoveOneCounter { from, to },
        )
    }

    pub(crate) fn subject_verb_for_each_counter_kind_put_or_remove(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ForEachCounterKindPutOrRemove { target },
        )
    }

    pub(crate) fn subject_verb_return_to_hand(target: TargetAst, random: bool) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReturnToHand { target, random },
        )
    }

    pub(crate) fn subject_verb_return_all_to_hand(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReturnAllToHand { filter },
        )
    }

    pub(crate) fn subject_verb_return_all_to_hand_of_chosen_color(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter },
        )
    }

    pub(crate) fn subject_verb_move_to_library_nth_from_top(
        target: TargetAst,
        position: Value,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::MoveToLibraryNthFromTop { target, position },
        )
    }

    pub(crate) fn subject_verb_double_counters_on_each(
        counter_type: CounterType,
        filter: ObjectFilter,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::DoubleCountersOnEach {
                counter_type,
                filter,
            },
        )
    }

    pub(crate) fn subject_verb_remove_counters_all(
        amount: Value,
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        up_to: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RemoveCountersAll {
                amount,
                filter,
                counter_type,
                up_to,
            },
        )
    }

    pub(crate) fn subject_verb_put_sticker(
        target: TargetAst,
        action: crate::events::KeywordActionKind,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutSticker { target, action },
        )
    }

    pub(crate) fn subject_verb_switch_power_toughness(target: TargetAst, duration: Until) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::SwitchPowerToughness { target, duration },
        )
    }

    pub(crate) fn subject_verb_scale_power_toughness_all(
        filter: ObjectFilter,
        power: bool,
        toughness: bool,
        multiplier: i32,
        duration: Until,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ScalePowerToughnessAll {
                filter,
                power,
                toughness,
                multiplier,
                duration,
            },
        )
    }

    pub(crate) fn subject_verb_reveal_hand(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RevealHand,
        )
    }

    pub(crate) fn subject_verb_discard(
        player: PlayerAst,
        count: Value,
        random: bool,
        any_number: bool,
        filter: Option<ObjectFilter>,
        tag: Option<TagKey>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::Discard {
                count,
                random,
                any_number,
                filter,
                tag,
            },
        )
    }

    pub(crate) fn subject_verb_discard_hand(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::DiscardHand,
        )
    }

    pub(crate) fn subject_verb_poison_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PoisonCounters { count },
        )
    }

    pub(crate) fn subject_verb_energy_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::EnergyCounters { count },
        )
    }

    pub(crate) fn subject_verb_ticket_counters(player: PlayerAst, count: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TicketCounters { count },
        )
    }

    pub(crate) fn subject_verb_pay_energy(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PayEnergy { amount },
        )
    }

    pub(crate) fn subject_verb_pay_any_energy(player: PlayerAst, min_amount: u32) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PayAnyEnergy { min_amount },
        )
    }

    pub(crate) fn subject_verb_pay_mana(player: PlayerAst, cost: ManaCost) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PayMana { cost },
        )
    }

    pub(crate) fn subject_verb_double_mana_pool(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::DoubleManaPool,
        )
    }

    pub(crate) fn subject_verb_empty_mana_pool(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::EmptyManaPool,
        )
    }

    pub(crate) fn subject_verb_set_life_total(player: PlayerAst, amount: Value) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SetLifeTotal { amount },
        )
    }

    pub(crate) fn subject_verb_skip_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipTurn,
        )
    }

    pub(crate) fn subject_verb_end_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::EndTurn,
        )
    }

    pub(crate) fn subject_verb_skip_combat_phases(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipCombatPhases,
        )
    }

    pub(crate) fn subject_verb_skip_next_combat_phase_this_turn(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipNextCombatPhaseThisTurn,
        )
    }

    pub(crate) fn subject_verb_skip_draw_step(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::SkipDrawStep,
        )
    }

    pub(crate) fn subject_verb_additional_phases(
        phases: Vec<crate::effects::AdditionalPhase>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Implicit,
            SubjectVerbActionAst::AdditionalPhases { phases },
        )
    }

    pub(crate) fn subject_verb_play_from_graveyard_until_eot(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::PlayFromGraveyardUntilEot,
        )
    }

    pub(crate) fn subject_verb_ring_tempts_you(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::RingTemptsYou,
        )
    }

    pub(crate) fn subject_verb_venture_into_dungeon(
        player: PlayerAst,
        undercity_if_no_active: bool,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::VentureIntoDungeon {
                undercity_if_no_active,
            },
        )
    }

    pub(crate) fn subject_verb_become_monarch(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::BecomeMonarch,
        )
    }

    pub(crate) fn subject_verb_take_initiative(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::TakeInitiative,
        )
    }

    pub(crate) fn subject_verb_create_emblem(player: PlayerAst, text: String) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::CreateEmblem { text },
        )
    }

    pub(crate) fn subject_verb_lose_game(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::LoseGame,
        )
    }

    pub(crate) fn subject_verb_win_game(player: PlayerAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            player,
            SubjectVerbActionAst::WinGame,
        )
    }

    pub(crate) fn subject_verb_detain(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Detain { target },
        )
    }

    pub(crate) fn subject_verb_goad(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Goad { target },
        )
    }

    pub(crate) fn subject_verb_suspect(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Suspect { target },
        )
    }

    pub(crate) fn subject_verb_clear_suspected(target: Option<TargetAst>) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ClearSuspected { target },
        )
    }

    pub(crate) fn subject_verb_remove_from_combat(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RemoveFromCombat { target },
        )
    }

    pub(crate) fn subject_verb_flip(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Flip { target },
        )
    }

    pub(crate) fn subject_verb_regenerate(target: TargetAst) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Regenerate { target },
        )
    }

    pub(crate) fn subject_verb_regenerate_all(filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::RegenerateAll { filter },
        )
    }

    pub(crate) fn subject_verb_sacrifice(
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
        target: Option<TargetAst>,
    ) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::Sacrifice {
                filter,
                count,
                target,
            },
        )
    }

    pub(crate) fn subject_verb_sacrifice_all(player: PlayerAst, filter: ObjectFilter) -> Self {
        Self::subject_verb(
            SubjectVerbRoleAst::Actor,
            player,
            SubjectVerbActionAst::SacrificeAll { filter },
        )
    }
}
