use crate::ConditionExpr;
use crate::ability::Ability;
use crate::color::ColorSet;
use crate::effect::{ChoiceCount, EffectId, Until, Value};
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::{AuraAttachmentFilter, CounterType};
use crate::static_abilities::StaticAbility;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::super::{
    ClashOpponentAst, ControlDurationAst, DamageBySpec, ExchangeValueAst, ExtraTurnAnchorAst,
    GrantedAbilityAst, IfResultPredicate, KeywordAction, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, ObjectRefAst, PlayerAst,
    PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst, RetargetModeAst,
    ReturnControllerAst, SearchLibrarySlotAst, SharedTypeConstraintAst, TargetAst,
    ZoneReplacementDurationAst,
};
use super::semantic::ParsedAbility;

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
        display: String,
    },
}

impl From<StaticAbility> for StaticAbilityAst {
    fn from(ability: StaticAbility) -> Self {
        Self::Static(ability)
    }
}

#[derive(Debug, Clone)]
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
    ThisDies,
    ThisDiesOrIsExiled,
    ThisLeavesBattlefield,
    ThisBecomesMonstrous,
    ThisBecomesTapped,
    PermanentBecomesTapped(ObjectFilter),
    ThisBecomesUntapped,
    ThisTurnedFaceUp,
    TurnedFaceUp(ObjectFilter),
    ThisBecomesTargeted,
    BecomesTargeted(ObjectFilter),
    ThisBecomesTargetedBySpell(ObjectFilter),
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
    IsDealtDamage(ObjectFilter),
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
    Dies(ObjectFilter),
    HauntedCreatureDies,
    PutIntoGraveyard(ObjectFilter),
    PutIntoGraveyardFromZone {
        filter: ObjectFilter,
        from: Zone,
    },
    CounterPutOn {
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        source_controller: Option<PlayerFilter>,
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
    ThisEntersBattlefield,
    ThisEntersBattlefieldFromZone {
        subject_filter: ObjectFilter,
        from: Zone,
        owner: Option<PlayerFilter>,
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
    Either(Box<TriggerSpec>, Box<TriggerSpec>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PredicateAst {
    ItIsLandCard,
    ItIsSoulbondPaired,
    ItMatches(ObjectFilter),
    TaggedMatches(TagKey, ObjectFilter),
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
    YouHaveNoCardsInHand,
    SourceIsTapped,
    SourceIsSaddled,

    SourceHasNoCounter(CounterType),
    TriggeringObjectHadNoCounter(CounterType),
    SourceHasCounterAtLeast {
        counter_type: CounterType,
        count: u32,
    },
    SourcePowerAtLeast(u32),
    SourceAttackedOrBlockedThisTurn,
    SourceIsInZone(Zone),
    YourTurn,
    YouAttackedWithExactlyNOtherCreaturesThisCombat(u32),
    CreatureDiedThisTurn,
    CreatureDiedThisTurnOrMore(u32),
    PermanentLeftBattlefieldUnderYourControlThisTurn,
    YouHaveFullParty,
    YouAttackedThisTurn,
    SourceWasCast,
    NoSpellsWereCastLastTurn,
    ThisSpellWasKicked,
    ThisSpellPaidLabel(String),
    TargetWasKicked,
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
    ThisSpellWasCastFromZone(Zone),
    ValueComparison {
        left: Value,
        operator: crate::effect::ValueComparisonOperator,
        right: Value,
    },
    Unmodeled(String),
    Not(Box<PredicateAst>),
    And(Box<PredicateAst>, Box<PredicateAst>),
}

#[derive(Debug, Clone)]
pub(crate) enum EffectAst {
    DealDamage {
        amount: Value,
        target: TargetAst,
    },
    DealDamageEqualToPower {
        source: TargetAst,
        target: TargetAst,
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
    DealDamageEach {
        amount: Value,
        filter: ObjectFilter,
    },
    Draw {
        count: Value,
        player: PlayerAst,
    },
    DrawForEachTaggedMatching {
        player: PlayerAst,
        tag: TagKey,
        filter: ObjectFilter,
    },
    Counter {
        target: TargetAst,
    },
    CounterUnlessPays {
        target: TargetAst,
        mana: Vec<ManaSymbol>,
        life: Option<Value>,
        additional_generic: Option<Value>,
    },
    UnlessPays {
        effects: Vec<EffectAst>,
        player: PlayerAst,
        mana: Vec<ManaSymbol>,
    },
    UnlessAction {
        effects: Vec<EffectAst>,
        alternative: Vec<EffectAst>,
        player: PlayerAst,
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
    ForEachCounterKindPutOrRemove {
        target: TargetAst,
    },
    PutCountersAll {
        counter_type: CounterType,
        count: Value,
        filter: ObjectFilter,
    },
    DoubleCountersOnEach {
        counter_type: CounterType,
        filter: ObjectFilter,
    },
    Proliferate {
        count: Value,
    },
    Tap {
        target: TargetAst,
    },
    TapAll {
        filter: ObjectFilter,
    },
    Untap {
        target: TargetAst,
    },
    TapOrUntapAll {
        tap_filter: ObjectFilter,
        untap_filter: ObjectFilter,
    },
    PhaseOut {
        target: TargetAst,
    },
    RemoveFromCombat {
        target: TargetAst,
    },
    TapOrUntap {
        target: TargetAst,
    },
    UntapAll {
        filter: ObjectFilter,
    },
    LoseLife {
        amount: Value,
        player: PlayerAst,
    },
    GainLife {
        amount: Value,
        player: PlayerAst,
    },
    LoseGame {
        player: PlayerAst,
    },
    WinGame {
        player: PlayerAst,
    },
    PreventAllCombatDamage {
        duration: Until,
    },
    PreventAllCombatDamageFromSource {
        duration: Until,
        source: TargetAst,
    },
    PreventAllCombatDamageToPlayers {
        duration: Until,
    },
    PreventAllCombatDamageToYou {
        duration: Until,
    },
    PreventDamage {
        amount: Value,
        target: TargetAst,
        duration: Until,
    },
    PreventAllDamageToTarget {
        target: TargetAst,
        duration: Until,
    },
    PreventDamageToTargetPutCounters {
        amount: Option<Value>,
        target: TargetAst,
        duration: Until,
        counter_type: CounterType,
    },
    PreventNextTimeDamage {
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
    },
    RedirectNextDamageFromSourceToTarget {
        amount: Value,
        target: TargetAst,
    },
    RedirectNextTimeDamageToSource {
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
    },
    PreventDamageEach {
        amount: Value,
        filter: ObjectFilter,
        duration: Until,
    },
    GrantProtectionChoice {
        target: TargetAst,
        allow_colorless: bool,
    },
    Earthbend {
        counters: u32,
    },
    Behold {
        subtype: Subtype,
        count: u32,
    },
    Explore {
        target: TargetAst,
    },
    OpenAttraction,
    ManifestTopCardOfLibrary {
        player: PlayerAst,
    },
    ManifestDread,
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
    Bolster {
        amount: u32,
    },
    Support {
        amount: u32,
    },
    Adapt {
        amount: u32,
    },
    AddMana {
        mana: Vec<ManaSymbol>,
        player: PlayerAst,
    },
    AddManaScaled {
        mana: Vec<ManaSymbol>,
        amount: Value,
        player: PlayerAst,
    },
    AddManaAnyColor {
        amount: Value,
        player: PlayerAst,
        available_colors: Option<Vec<crate::color::Color>>,
    },
    AddManaAnyOneColor {
        amount: Value,
        player: PlayerAst,
    },
    AddManaChosenColor {
        amount: Value,
        player: PlayerAst,
        fixed_option: Option<crate::color::Color>,
    },
    AddManaFromLandCouldProduce {
        amount: Value,
        player: PlayerAst,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    },
    AddManaCommanderIdentity {
        amount: Value,
        player: PlayerAst,
    },
    AddManaImprintedColors,
    Scry {
        count: Value,
        player: PlayerAst,
    },
    Fateseal {
        count: Value,
        player: PlayerAst,
    },
    Discover {
        count: Value,
        player: PlayerAst,
    },
    ConsultTopOfLibrary {
        player: PlayerAst,
        mode: LibraryConsultModeAst,
        filter: ObjectFilter,
        stop_rule: LibraryConsultStopRuleAst,
        all_tag: TagKey,
        match_tag: TagKey,
    },
    PutTaggedRemainderOnBottomOfLibrary {
        tag: TagKey,
        keep_tagged: Option<TagKey>,
        order: LibraryBottomOrderAst,
        player: PlayerAst,
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
    Surveil {
        count: Value,
        player: PlayerAst,
    },
    PayMana {
        cost: ManaCost,
        player: PlayerAst,
    },
    PayEnergy {
        amount: Value,
        player: PlayerAst,
    },
    Cant {
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        condition: Option<crate::ConditionExpr>,
    },
    PlayFromGraveyardUntilEot {
        player: PlayerAst,
    },
    AdditionalLandPlays {
        count: Value,
        player: PlayerAst,
        duration: Until,
    },
    ReduceNextSpellCostThisTurn {
        player: PlayerAst,
        filter: ObjectFilter,
        reduction: ManaCost,
    },
    GrantNextSpellAbilityThisTurn {
        player: PlayerAst,
        filter: ObjectFilter,
        ability: GrantedAbilityAst,
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
    },
    CastTagged {
        tag: TagKey,
        allow_land: bool,
        as_copy: bool,
        without_paying_mana_cost: bool,
        cost_reduction: Option<ManaCost>,
    },
    RegisterZoneReplacement {
        target: TargetAst,
        from_zone: Option<Zone>,
        to_zone: Option<Zone>,
        replacement_zone: Zone,
        duration: ZoneReplacementDurationAst,
    },
    ExileInsteadOfGraveyardThisTurn {
        player: PlayerAst,
    },
    GainControl {
        target: TargetAst,
        player: PlayerAst,
        duration: Until,
    },
    ControlPlayer {
        player: PlayerFilter,
        duration: ControlDurationAst,
    },
    ExtraTurnAfterTurn {
        player: PlayerAst,
        anchor: ExtraTurnAnchorAst,
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
    RevealTop {
        player: PlayerAst,
    },
    RevealTopChooseCardTypePutToHandRestBottom {
        player: PlayerAst,
        count: u32,
    },
    RevealTopPutMatchingIntoHandRestIntoGraveyard {
        player: PlayerAst,
        count: u32,
        filter: ObjectFilter,
    },
    RevealTagged {
        tag: TagKey,
    },
    ExileTopOfLibrary {
        count: Value,
        player: PlayerAst,
        tags: Vec<TagKey>,
        accumulated_tags: Vec<TagKey>,
    },
    LookAtTopCards {
        player: PlayerAst,
        count: Value,
        tag: TagKey,
    },
    RearrangeLookedCardsInLibrary {
        tag: TagKey,
        player: PlayerAst,
        count: ChoiceCount,
    },
    RevealHand {
        player: PlayerAst,
    },
    PutIntoHand {
        player: PlayerAst,
        object: ObjectRefAst,
    },
    PutSomeIntoHandRestIntoGraveyard {
        player: PlayerAst,
        count: u32,
    },
    PutSomeIntoHandRestOnBottomOfLibrary {
        player: PlayerAst,
        count: u32,
    },
    ChooseFromLookedCardsIntoHandRestIntoGraveyard {
        player: PlayerAst,
        filter: ObjectFilter,
        reveal: bool,
        if_not_chosen: Vec<EffectAst>,
    },
    ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
        player: PlayerAst,
        filter: ObjectFilter,
        reveal: bool,
        if_not_chosen: Vec<EffectAst>,
    },
    ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary {
        player: PlayerAst,
        spell_filter: ObjectFilter,
        order: LibraryBottomOrderAst,
    },
    ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
        player: PlayerAst,
        battlefield_filter: ObjectFilter,
        tapped: bool,
    },
    ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
        player: PlayerAst,
        battlefield_filter: ObjectFilter,
        hand_filter: ObjectFilter,
        tapped: bool,
        order: LibraryBottomOrderAst,
    },
    PutRestOnBottomOfLibrary,
    CopySpell {
        target: TargetAst,
        count: Value,
        player: PlayerAst,
        may_choose_new_targets: bool,
    },
    FlipCoin {
        player: PlayerAst,
    },
    RetargetStackObject {
        target: TargetAst,
        mode: RetargetModeAst,
        chooser: PlayerAst,
        require_change: bool,
    },
    Conditional {
        predicate: PredicateAst,
        if_true: Vec<EffectAst>,
        if_false: Vec<EffectAst>,
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
        player: PlayerAst,
        tag: TagKey,
        zones: Vec<Zone>,
        search_mode: Option<crate::effect::SearchSelectionMode>,
    },
    Sacrifice {
        filter: ObjectFilter,
        player: PlayerAst,
        count: u32,
        target: Option<TargetAst>,
    },
    SacrificeAll {
        filter: ObjectFilter,
        player: PlayerAst,
    },
    DiscardHand {
        player: PlayerAst,
    },
    Discard {
        count: Value,
        player: PlayerAst,
        random: bool,
        filter: Option<ObjectFilter>,
        tag: Option<TagKey>,
    },
    Connive {
        target: TargetAst,
    },
    ConniveIterated,
    Detain {
        target: TargetAst,
    },
    Goad {
        target: TargetAst,
    },
    Transform {
        target: TargetAst,
    },
    Meld {
        result_name: String,
        enters_tapped: bool,
        enters_attacking: bool,
    },
    Convert {
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
    Mill {
        count: Value,
        player: PlayerAst,
    },
    ReturnToHand {
        target: TargetAst,
        random: bool,
    },
    ReturnToBattlefield {
        target: TargetAst,
        tapped: bool,
        transformed: bool,
        converted: bool,
        controller: ReturnControllerAst,
    },
    MoveToZone {
        target: TargetAst,
        zone: Zone,
        to_top: bool,
        battlefield_controller: ReturnControllerAst,
        battlefield_tapped: bool,
        attached_to: Option<TargetAst>,
    },
    ShuffleObjectsIntoLibrary {
        target: TargetAst,
        player: PlayerAst,
    },
    MoveToLibraryNthFromTop {
        target: TargetAst,
        position: Value,
    },
    ReturnAllToHand {
        filter: ObjectFilter,
    },
    ReturnAllToHandOfChosenColor {
        filter: ObjectFilter,
    },
    ReturnAllToBattlefield {
        filter: ObjectFilter,
        tapped: bool,
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
    ExchangeLifeTotals {
        player1: PlayerAst,
        player2: PlayerAst,
    },
    ExchangeTextBoxes {
        target: TargetAst,
    },
    ExchangeZones {
        player: PlayerAst,
        zone1: Zone,
        zone2: Zone,
    },
    ExchangeValues {
        left: ExchangeValueAst,
        right: ExchangeValueAst,
        duration: Until,
    },
    RingTemptsYou {
        player: PlayerAst,
    },
    VentureIntoDungeon {
        player: PlayerAst,
        undercity_if_no_active: bool,
    },
    BecomeMonarch {
        player: PlayerAst,
    },
    TakeInitiative {
        player: PlayerAst,
    },
    DoubleManaPool {
        player: PlayerAst,
    },
    SetLifeTotal {
        amount: Value,
        player: PlayerAst,
    },
    SkipTurn {
        player: PlayerAst,
    },
    SkipCombatPhases {
        player: PlayerAst,
    },
    SkipNextCombatPhaseThisTurn {
        player: PlayerAst,
    },
    SkipDrawStep {
        player: PlayerAst,
    },
    PoisonCounters {
        count: Value,
        player: PlayerAst,
    },
    EnergyCounters {
        count: Value,
        player: PlayerAst,
    },
    CreateEmblem {
        player: PlayerAst,
        text: String,
    },
    DealDistributedDamage {
        amount: Value,
        target: TargetAst,
    },
    ChooseCardName {
        player: PlayerAst,
        filter: Option<ObjectFilter>,
        tag: TagKey,
    },
    ChoosePlayer {
        chooser: PlayerAst,
        filter: PlayerFilter,
        tag: TagKey,
        random: bool,
        exclude_previous_choices: usize,
    },
    TagMatchingObjects {
        filter: ObjectFilter,
        zones: Vec<Zone>,
        tag: TagKey,
    },
    ChooseSpellCastHistory {
        chooser: PlayerAst,
        cast_by: PlayerAst,
        filter: ObjectFilter,
        tag: TagKey,
    },
    ChooseColor {
        player: PlayerAst,
    },
    ChooseCardType {
        player: PlayerAst,
        options: Vec<CardType>,
    },
    ChooseNamedOption {
        player: PlayerAst,
        options: Vec<String>,
    },
    ChooseCreatureType {
        player: PlayerAst,
        excluded_subtypes: Vec<Subtype>,
    },
    DontLoseThisManaAsStepsAndPhasesEndThisTurn,
    RepeatThisProcess,
    RepeatThisProcessMay,
    RepeatThisProcessOnce,
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
    RollDie {
        player: PlayerAst,
        sides: u32,
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
    Enchant {
        filter: AuraAttachmentFilter,
    },
    Attach {
        object: TargetAst,
        target: TargetAst,
    },
    PutSticker {
        target: TargetAst,
        action: crate::events::KeywordActionKind,
    },
    Investigate {
        count: Value,
    },
    Amass {
        subtype: Option<Subtype>,
        amount: u32,
    },
    Destroy {
        target: TargetAst,
    },
    DestroyNoRegeneration {
        target: TargetAst,
    },
    DestroyAll {
        filter: ObjectFilter,
    },
    DestroyAllNoRegeneration {
        filter: ObjectFilter,
    },
    DestroyAllOfChosenColor {
        filter: ObjectFilter,
    },
    DestroyAllOfChosenColorNoRegeneration {
        filter: ObjectFilter,
    },
    DestroyAllAttachedTo {
        filter: ObjectFilter,
        target: TargetAst,
    },
    Exile {
        target: TargetAst,
        face_down: bool,
    },
    ExileWhenSourceLeaves {
        target: TargetAst,
    },
    SacrificeSourceWhenLeaves {
        target: TargetAst,
    },
    ExileUntilSourceLeaves {
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
    TargetOnly {
        target: TargetAst,
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
    },
    Monstrosity {
        amount: Value,
    },
    RemoveUpToAnyCounters {
        amount: Value,
        target: TargetAst,
        counter_type: Option<CounterType>,
        up_to: bool,
    },
    RemoveCountersAll {
        amount: Value,
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        up_to: bool,
    },
    MoveAllCounters {
        from: TargetAst,
        to: TargetAst,
    },
    Pump {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
        condition: Option<crate::ConditionExpr>,
    },
    SwitchPowerToughness {
        target: TargetAst,
        duration: Until,
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
    ScalePowerToughnessAll {
        filter: ObjectFilter,
        power: bool,
        toughness: bool,
        multiplier: i32,
        duration: Until,
    },
    PumpByLastEffect {
        power: i32,
        toughness: i32,
        target: TargetAst,
        duration: Until,
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
    GrantAbilityToSource {
        ability: ParsedAbility,
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
        tapped: bool,
    },
    SearchLibrarySlotsToHand {
        slots: Vec<SearchLibrarySlotAst>,
        player: PlayerAst,
        reveal: bool,
        progress_tag: TagKey,
    },
    MayMoveToZone {
        target: TargetAst,
        zone: Zone,
        player: PlayerAst,
    },
    ShuffleHandAndGraveyardIntoLibrary {
        player: PlayerAst,
    },
    ShuffleGraveyardIntoLibrary {
        player: PlayerAst,
    },
    ReorderGraveyard {
        player: PlayerAst,
    },
    ReorderTopOfLibrary {
        tag: TagKey,
    },

    ShuffleLibrary {
        player: PlayerAst,
    },
    VoteStart {
        options: Vec<String>,
    },
    VoteStartObjects {
        filter: ObjectFilter,
        count: ChoiceCount,
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
