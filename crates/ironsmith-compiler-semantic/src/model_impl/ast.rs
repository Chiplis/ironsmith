use ironsmith_core::tag::TagKeyWalk;

use crate::color::ColorSet;
use crate::cost::OptionalCostRef;
use crate::effect::{ChoiceAggregateMetric, ChoiceCount, EffectId, Until, Value};
use crate::filter::Comparison;
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::RedirectNextTimeDamageDestinationAst;
use crate::model::control_flow::CompilerControlFlowAst;
use crate::model::coordination::CoordinationAst;
use crate::model::resource_choice_clauses::{CompilerIterationAst, CompilerVoteAst};
use crate::object::{AuraAttachmentFilter, CounterType};
use crate::tag::TagKey;
use crate::target::{ChooseSpec, ObjectFilter, ObjectRef, PlayerFilter, SourceReferenceSurface};
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
use crate::cards::builders::GrantedAbilityAst;
use crate::model::compiler_semantic::ParsedAbility;

#[path = "ast/queries.rs"]
mod queries;
pub use queries::*;

#[path = "ast/predicates.rs"]
mod predicates;
pub use predicates::*;

#[path = "ast/actions.rs"]
mod actions;
pub use actions::*;

#[path = "ast/effects.rs"]
mod effects;
pub use effects::*;

#[path = "ast/nodes.rs"]
mod nodes;
pub use nodes::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum StaticAbilityAst {
    Static(crate::model::CompilerStaticAbilityCore),
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
        condition: PredicateAst,
    },
    LabeledConditionalStaticAbility {
        ability: Box<StaticAbilityAst>,
        condition: PredicateAst,
        label: String,
    },
    ConditionalKeywordAction {
        action: KeywordAction,
        condition: PredicateAst,
    },
    WithSetQuantifierSurface {
        ability: Box<StaticAbilityAst>,
        surface: ironsmith_core::SetQuantifierSurface,
    },
    GrantStaticAbility {
        filter: ObjectFilter,
        ability: Box<StaticAbilityAst>,
        condition: Option<PredicateAst>,
    },
    GrantKeywordAction {
        filter: ObjectFilter,
        action: KeywordAction,
        condition: Option<PredicateAst>,
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
        condition: Option<PredicateAst>,
    },
    AttachedKeywordActionGrant {
        action: KeywordAction,
        display: String,
        condition: Option<PredicateAst>,
        protection_does_not_remove_controlled_attachments: bool,
    },
    AttachedChosenLandwalkGrant {
        snow: bool,
        display: String,
        condition: Option<PredicateAst>,
    },
    EquipmentKeywordActionsGrant {
        actions: Vec<KeywordAction>,
    },
    GrantObjectAbility {
        filter: ObjectFilter,
        ability: ParsedAbility,
        display: String,
        condition: Option<PredicateAst>,
    },
    AttachedObjectAbilityGrant {
        ability: ParsedAbility,
        display: String,
        condition: Option<PredicateAst>,
    },
    SoulbondSharedObjectAbility {
        ability: ParsedAbility,
    },
    AttachmentRestriction {
        filter: AuraAttachmentFilter,
        display: String,
    },
}

impl From<crate::model::CompilerStaticAbilityCore> for StaticAbilityAst {
    fn from(ability: crate::model::CompilerStaticAbilityCore) -> Self {
        Self::Static(ability)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum TriggerIntroSurfaceAst {
    When,
    Whenever,
    At,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum TriggerSpec {
    WithIntro {
        intro: TriggerIntroSurfaceAst,
        trigger: Box<TriggerSpec>,
    },
    /// "Whenever A or B" — fires when any branch's event occurs.
    AnyOf(Vec<TriggerSpec>),
    /// Event-time `while` qualification. This is part of event matching, not
    /// an intervening-if condition that is checked again on resolution.
    ConditionQualified {
        trigger: Box<TriggerSpec>,
        condition: PredicateAst,
        surface: String,
    },
    ThisPhasesOut,
    StateBased {
        condition: PredicateAst,
        display: String,
    },
    ThisAttacks,
    /// The source attacks while its controller controls a matching permanent.
    /// This predicate is part of the attack event qualification and is not an
    /// intervening-if condition checked again on resolution.
    ThisAttacksWhileYouControl(ObjectFilter),
    ThisAndAnotherAttackDifferentPlayers,
    ThisAttacksPlayerWhoControlsAtLeast {
        count: u32,
        filter: ObjectFilter,
    },
    ThisAttacksWithNOthers {
        other_count: u32,
        display_subject: Option<String>,
        other_filter: Option<ObjectFilter>,
        other_surface: bool,
    },
    ThisAttacksWithExactlyNOthers(u32),
    ThisAttacksAndIsntBlocked,
    ThisAttacksWhileSaddled,
    Attacks(ObjectFilter),
    AttacksAndIsntBlocked(ObjectFilter),
    AttacksWhileSaddled(ObjectFilter),
    AttacksOneOrMore(ObjectFilter),
    PlayersAttackedOneOrMore(PlayerFilter),
    PlayerAttacksOneOrMore {
        attacker: PlayerFilter,
        target: ironsmith_core::AttackTargetRestriction,
    },
    /// A matching player attacks one matching defender with one or more
    /// creatures. Unlike `PlayerAttacksOneOrMore`, grouping is scoped to the
    /// attacked defender rather than the whole attack declaration.
    PlayerAttacksTargetWithOneOrMore {
        attacker: PlayerFilter,
        target: ironsmith_core::AttackTargetRestriction,
    },
    AttacksOneOrMoreWithMinTotal {
        filter: ObjectFilter,
        min_total_attackers: u32,
    },
    AttacksOneOrMoreWithExactTotal {
        filter: ObjectFilter,
        total_attackers: u32,
    },
    AttacksOneOrMoreWithAggregate {
        filter: ObjectFilter,
        metric: ChoiceAggregateMetric,
        comparison: Comparison,
    },
    AttacksAlone(ObjectFilter),
    AttacksYouOrPlaneswalkerYouControl(ObjectFilter),
    AttacksYouOrPlaneswalkerYouControlOneOrMore(ObjectFilter),
    ThisBlocks,
    ThisBlocksObject {
        filter: ObjectFilter,
        /// `None` is a per-object trigger; `Some(N)` is one aggregated
        /// declaration containing at least N matching blocked objects.
        min_blocked_objects: Option<u32>,
    },
    Blocks(ObjectFilter),
    BlocksOneOrMore(ObjectFilter),
    /// One per blocking pair where `subject` is either the blocker or the
    /// blocked creature and the participant on the other side matches
    /// `other`. This preserves authored tagged subjects such as "enchanted
    /// creature blocks or becomes blocked by a creature ..." without
    /// pretending that the Aura source is itself in combat.
    BlocksOrBecomesBlockedByObject {
        subject: ObjectFilter,
        other: ObjectFilter,
    },
    BlocksObjectWithLesserPower {
        blocker: ObjectFilter,
        blocked: ObjectFilter,
    },
    ThisBecomesBlocked,
    BecomesBlocked(ObjectFilter),
    ThisBecomesBlockedByObject(ObjectFilter),
    BecomesBlockedByObjectWithLesserPower {
        blocked: ObjectFilter,
        blocker: ObjectFilter,
    },
    ThisDies,
    ThisDiesOrIsExiled,
    ThisDiesOrIsExiledWithSurface(crate::target::SourceReferenceSurface),
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
    DealsExactDamageToObjectOrPlayer {
        source: ObjectFilter,
        object: ObjectFilter,
        player: PlayerFilter,
        player_first: bool,
        amount: u32,
        source_surface: crate::triggers::DamageSourceSurface,
    },
    DealsNoncombatDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
        source_surface: crate::triggers::DamageSourceSurface,
        damaged_player_one_or_more: bool,
        during_turn: Option<PlayerFilter>,
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
        source_filter: Option<ObjectFilter>,
        caused_by_source_entering: bool,
    },
    ThisIsDealtDamage,
    ThisIsDealtCombatDamage,
    IsDealtDamage(ObjectFilter),
    IsDealtCombatDamage(ObjectFilter),
    IsDealtExcessNoncombatDamage(ObjectFilter),
    YouGainLife,
    YouGainLifeCausedBy(ObjectFilter),
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
    PermanentSacrificed(ObjectFilter),
    PermanentDestroyed(ObjectFilter),
    TokensCreated {
        player: PlayerFilter,
        filter: ObjectFilter,
        one_or_more: bool,
    },
    LeavesBattlefield(ObjectFilter),
    LeavesBattlefieldWithoutDying {
        filter: ObjectFilter,
        one_or_more: bool,
    },
    Dies(ObjectFilter),
    DiesOneOrMore(ObjectFilter),
    DiesDuringTurn {
        filter: ObjectFilter,
        one_or_more: bool,
        during_turn: PlayerFilter,
    },
    DiesDuringCombat {
        filter: Option<ObjectFilter>,
        one_or_more: bool,
    },
    HauntedCreatureDies,
    PutIntoGraveyard(ObjectFilter),
    PutIntoGraveyardOneOrMore(ObjectFilter),
    PutIntoGraveyardFromZone {
        filter: ObjectFilter,
        from: Zone,
        one_or_more: bool,
    },
    PutIntoGraveyardFromAnyExcept {
        filter: ObjectFilter,
        excluded: Zone,
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
    NthCounterPutOn {
        filter: ObjectFilter,
        counter_type: CounterType,
        counter_number: u32,
    },
    CounterRemovedFrom {
        filter: ObjectFilter,
        counter_type: Option<CounterType>,
        last: bool,
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
    DiesCreatureDealtDamageByFilteredSourceThisTurn {
        victim: ObjectFilter,
        damager_filter: ObjectFilter,
    },
    SpellCast {
        filter: Option<ObjectFilter>,
        mana_source_filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        timing: Option<ironsmith_core::TriggerTimingRestriction>,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    },
    /// Passive "the Nth spell of a turn is cast" trigger. Unlike a player's
    /// Nth-spell trigger, this count spans spells cast by every player.
    NthSpellOfTurnCast {
        spell_number: u32,
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
        origin_condition: Option<ironsmith_core::trigger_model::ZoneChangeOriginCondition>,
        during_turn: Option<PlayerFilter>,
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
    BeginningOfMonarchEndStep,
    BeginningOfMainPhase {
        player: PlayerFilter,
        surface: ironsmith_core::trigger_model::MainPhaseSurface,
    },
    BeginningOfPrecombatMain(PlayerFilter),
    BeginningOfPostcombatMain {
        player: PlayerFilter,
        surface: ironsmith_core::trigger_model::PostcombatMainPhaseSurface,
    },
    DayNightChanged,
    ThisEntersBattlefield {
        origin_condition: Option<ironsmith_core::trigger_model::ZoneChangeOriginCondition>,
    },
    ThisEntersBattlefieldWithSurface {
        surface: crate::target::SourceReferenceSurface,
        subject_number: ironsmith_core::trigger_model::TriggerSubjectNumber,
        origin_condition: Option<ironsmith_core::trigger_model::ZoneChangeOriginCondition>,
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
        source_surface: Option<crate::target::SourceReferenceSurface>,
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
        during_your_turn: bool,
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
        surface: ironsmith_core::ClashWinTriggerSurface,
    },
    Expend {
        player: PlayerFilter,
        amount: u32,
    },
    SagaChapter(Vec<u32>),
    FinalChapterAbilityResolved(ObjectFilter),
    Either(Box<TriggerSpec>, Box<TriggerSpec>),
}
