use crate::{
    CauseFilter, ChooseSpec, CounterType, KeywordActionKind, ObjectFilter, PlayerFilter,
    SourceReferenceSurface, TagKey, Zone, filter_model::Comparison,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountMode {
    One,
    OneOrMore,
}

/// Oracle surface for an end-step trigger whose runtime player filter is Any.
///
/// Both forms fire at every end step; this distinction only preserves whether
/// the source says "the end step" or "each end step" for compiled text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EndStepSurface {
    #[default]
    Each,
    Definite,
}

/// Authored wording for a zone change into a graveyard.
///
/// This is presentation metadata only. It distinguishes authored "dies" text
/// from explicit "is put into a graveyard" text without changing which zone
/// change events match the trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraveyardTriggerSurface {
    Dies,
    PutIntoGraveyard,
}

/// Authored wording for a trigger that observes the winner of a clash.
///
/// Both surfaces subscribe to the same winner-aware event. This distinction
/// exists only so compiled text can retain whether the source said "win a
/// clash" or "clash and win".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClashWinTriggerSurface {
    #[default]
    WinAClash,
    ClashAndWin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamagedBySource {
    ThisCreature,
    EquippedCreature,
    EnchantedCreature,
}

/// A game-state boundary that is part of a trigger event's qualification.
///
/// This is evaluated only when the event occurs. It is intentionally distinct
/// from an intervening-if condition, which is checked again on resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerTimingRestriction {
    DuringCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageSourceSurface {
    Filter,
    Source,
    /// Passive Oracle wording: "[recipient] is dealt damage by [source]."
    ///
    /// Runtime matching remains source-to-recipient damage; this only
    /// preserves the authored direction for compiled text.
    PassiveBy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerKind {
    StateBased {
        display: String,
    },
    /// "Whenever A or B" — fires when any branch's event occurs.
    AnyOf(Vec<Trigger>),
    ThisAttacks,
    ThisAttacksPlayerWhoControlsAtLeast {
        count: usize,
        filter: ObjectFilter,
    },
    ThisAttacksPlayerWithMostLife,
    ThisAttacksWithGreaterPower,
    ThisAttacksWithNOthers {
        count: usize,
        display_subject: Option<String>,
        other_filter: Option<ObjectFilter>,
        other_surface: bool,
    },
    ThisAttacksWithExactNOthers {
        count: usize,
    },
    ThisAttacksAndIsntBlocked,
    ThisAttacksWhileSaddled,
    Attacks {
        filter: ObjectFilter,
    },
    AttacksAndIsntBlocked {
        filter: ObjectFilter,
    },
    AttacksWhileSaddled {
        filter: ObjectFilter,
    },
    AttacksOneOrMore {
        filter: ObjectFilter,
    },
    PlayersAttackedOneOrMore {
        player_filter: PlayerFilter,
    },
    AttacksOneOrMoreWithMinTotal {
        filter: ObjectFilter,
        min_total_attackers: usize,
    },
    AttacksOneOrMoreWithExactTotal {
        filter: ObjectFilter,
        total_attackers: usize,
    },
    AttacksAlone {
        filter: ObjectFilter,
    },
    AttacksYou {
        filter: ObjectFilter,
    },
    AttacksYouOneOrMore {
        filter: ObjectFilter,
    },
    ThisBlocks,
    ThisBlocksObject {
        filter: ObjectFilter,
        /// `None` is the ordinary per-object trigger. `Some(N)` is the
        /// aggregated "N or more" event and fires once for the declaration.
        min_blocked_objects: Option<usize>,
    },
    Blocks {
        filter: ObjectFilter,
    },
    BlocksOneOrMore {
        filter: ObjectFilter,
    },
    /// A blocking relationship whose blocked object has strictly less power
    /// than the blocker. Both filters are evaluated against the objects as
    /// they existed when blockers were declared.
    BlocksObjectWithLesserPower {
        blocker: ObjectFilter,
        blocked: ObjectFilter,
    },
    ThisBecomesBlocked,
    BecomesBlocked {
        filter: ObjectFilter,
    },
    ThisBecomesBlockedByObject {
        filter: ObjectFilter,
    },
    /// A blocking relationship whose blocker has strictly less power than the
    /// object it blocked. This is a per-blocker event, not the aggregate
    /// "becomes blocked" event.
    BecomesBlockedByObjectWithLesserPower {
        blocked: ObjectFilter,
        blocker: ObjectFilter,
    },
    ThisDies,
    ThisDiesOrIsExiled,
    ThisDiesOrIsExiledWithSurface {
        surface: SourceReferenceSurface,
    },
    ThisLeavesBattlefield,
    ThisPhasesOut,
    ThisMutates,
    LeavesBattlefield {
        filter: ObjectFilter,
    },
    ThisBecomesMonstrous,
    BecomesTapped,
    PermanentBecomesTapped {
        filter: ObjectFilter,
    },
    BecomesUntapped,
    ThisIsTurnedFaceUp,
    TurnedFaceUp {
        filter: ObjectFilter,
    },
    BecomesTargeted,
    BecomesTargetedObject {
        filter: ObjectFilter,
    },
    BecomesTargetedBySpell {
        filter: ObjectFilter,
    },
    BecomesTargetedByStackObject {
        filter: ObjectFilter,
    },
    BecomesTargetedObjectByStackObject {
        target: ObjectFilter,
        source: ObjectFilter,
    },
    BecomesTargetedBySourceController {
        target: ObjectFilter,
        controller: PlayerFilter,
    },
    PlayerOrObjectBecomesTargetedBySourceController {
        player: PlayerFilter,
        object: ObjectFilter,
        controller: PlayerFilter,
    },
    ThisDealsDamage,
    ThisDealsDamageToPlayer {
        player: PlayerFilter,
        amount: Option<Comparison>,
    },
    ThisDealsDamageTo {
        filter: ObjectFilter,
    },
    ThisDealsCombatDamage,
    ThisDealsCombatDamageTo {
        filter: ObjectFilter,
    },
    ThisDealsCombatDamageToPlayer {
        player: PlayerFilter,
        source_surface: Option<SourceReferenceSurface>,
    },
    DealsDamage {
        filter: ObjectFilter,
        source_surface: DamageSourceSurface,
    },
    DealsDamageTo {
        source: ObjectFilter,
        target: ObjectFilter,
        source_surface: DamageSourceSurface,
    },
    DealsDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
        source_surface: DamageSourceSurface,
    },
    DealsNoncombatDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
        source_surface: DamageSourceSurface,
    },
    DealsCombatDamage {
        filter: ObjectFilter,
    },
    DealsCombatDamageTo {
        source: ObjectFilter,
        target: ObjectFilter,
    },
    DealsCombatDamageToPlayer {
        source: ObjectFilter,
        player: PlayerFilter,
        one_or_more: bool,
    },
    PlayerPlaysLand {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    PlayerGivesGift {
        player: PlayerFilter,
    },
    PlayerSearchesLibrary {
        player: PlayerFilter,
    },
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
    AbilityActivatedQualified {
        activator: PlayerFilter,
        filter: ObjectFilter,
        non_mana_only: bool,
        loyalty_only: bool,
        activation_cost_has_tap: Option<bool>,
    },
    AbilityTriggered {
        another: bool,
    },
    IsDealtDamage {
        target: ChooseSpec,
        combat_only: bool,
    },
    YouGainLife,
    YouGainLifeCausedBy {
        source: ObjectFilter,
    },
    YouGainLifeDuringTurn {
        during_turn: PlayerFilter,
    },
    PlayerLosesLife {
        player: PlayerFilter,
    },
    PlayersLoseLifeOneOrMore {
        player: PlayerFilter,
    },
    /// "Whenever one or more opponents each lose exactly N life"
    OpponentsEachLoseExactLife {
        amount: u32,
    },
    PlayerLosesGame {
        player: PlayerFilter,
    },
    PlayerLosesLifeDuringTurn {
        player: PlayerFilter,
        during_turn: PlayerFilter,
    },
    SpellCountered {
        filter: Option<ObjectFilter>,
        controller: PlayerFilter,
    },
    YouDrawCard,
    PlayerDrawsCard {
        player: PlayerFilter,
    },
    PlayerDrawsCardNotDuringTurn {
        player: PlayerFilter,
        during_turn: PlayerFilter,
    },
    PlayerDrawsCardExceptFirstInDrawStep {
        player: PlayerFilter,
    },
    PlayerDrawsNthCardEachTurn {
        player: PlayerFilter,
        card_number: u32,
    },
    PlayerDrawsNumberedCardsEachTurn {
        player: PlayerFilter,
        card_numbers: Vec<u32>,
    },
    PlayerDiscardsCardCausedByController {
        player: PlayerFilter,
        filter: Option<ObjectFilter>,
        controller: PlayerFilter,
        effect_like_only: bool,
    },
    PlayerDiscardsCard {
        player: PlayerFilter,
        filter: Option<ObjectFilter>,
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
        one_or_more_surface: bool,
    },
    TokensCreated {
        player: PlayerFilter,
        filter: ObjectFilter,
        one_or_more: bool,
    },
    Dies {
        filter: ObjectFilter,
    },
    PutIntoGraveyard {
        filter: ObjectFilter,
    },
    CardsLeaveYourGraveyard {
        filter: ObjectFilter,
        one_or_more: bool,
        during_your_turn: bool,
    },
    DiesCreatureDealtDamageByThisTurn {
        victim: ObjectFilter,
        damager: DamagedBySource,
    },
    SpellCastQualified {
        filter: Option<ObjectFilter>,
        mana_source_filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        timing: Option<TriggerTimingRestriction>,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    },
    SpellCast {
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
    },
    /// The Nth spell cast during the turn, counted across every player.
    NthSpellOfTurnCast {
        spell_number: u32,
    },
    SpellCopied {
        filter: Option<ObjectFilter>,
        copier: PlayerFilter,
    },
    EntersBattlefield {
        filter: ObjectFilter,
        cause_filter: Option<CauseFilter>,
        count: CountMode,
        tapped: Option<bool>,
    },
    BeginningOfUpkeep {
        player: PlayerFilter,
    },
    BeginningOfDrawStep {
        player: PlayerFilter,
    },
    BeginningOfCombat {
        player: PlayerFilter,
    },
    EndOfCombat,
    BeginningOfEndStep {
        player: PlayerFilter,
        surface: EndStepSurface,
    },
    BeginningOfPrecombatMainPhase {
        player: PlayerFilter,
    },
    BeginningOfPostcombatMainPhase {
        player: PlayerFilter,
    },
    DayNightChanged,
    ThisEntersBattlefield,
    ThisTransforms {
        destination_name: Option<String>,
    },
    ThisTransformsWithSurface {
        surface: SourceReferenceSurface,
        destination_name: Option<String>,
    },
    YouCastThisSpell,
    KeywordActionMatchingObject {
        action: KeywordActionKind,
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    KeywordActionMatchingTaggedObject {
        action: KeywordActionKind,
        player: PlayerFilter,
        source_filter: ObjectFilter,
        object_tag: TagKey,
        object_filter: ObjectFilter,
        during_your_main_phase: bool,
    },
    KeywordAction {
        action: KeywordActionKind,
        player: PlayerFilter,
    },
    KeywordActionFromSource {
        action: KeywordActionKind,
        player: PlayerFilter,
    },
    WinsClash {
        player: PlayerFilter,
        surface: ClashWinTriggerSurface,
    },
    Expend {
        amount: u32,
        player: PlayerFilter,
    },
    SagaChapter {
        chapters: Vec<u32>,
    },
    FinalChapterAbilityResolved {
        filter: ObjectFilter,
    },
    Custom {
        id: String,
        label: String,
    },
    Either {
        left: Box<Trigger>,
        right: Box<Trigger>,
    },
    ZoneChange(ZoneChangeTrigger),
    PlayerGetsCounters(PlayerGetsCountersTrigger),
    CounterPutOn(CounterPutOnTrigger),
    NthCounterPutOn {
        filter: ObjectFilter,
        counter_type: CounterType,
        counter_number: u32,
    },
    CounterRemovedFrom(CounterRemovedFromTrigger),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerIntroSurface {
    When,
    Whenever,
    At,
}

impl TriggerIntroSurface {
    fn as_str(self) -> &'static str {
        match self {
            Self::When => "When",
            Self::Whenever => "Whenever",
            Self::At => "At",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    pub label: String,
    pub kind: TriggerKind,
    pub intro_surface: Option<TriggerIntroSurface>,
}

impl Trigger {
    pub fn new<T: CompilerTriggerMatcher>(matcher: T) -> Self {
        matcher.into_trigger()
    }

    pub fn state_based(display: impl Into<String>) -> Self {
        let display = display.into();
        Self::typed(display.clone(), TriggerKind::StateBased { display })
    }

    /// A union of alternative trigger events ("Whenever A or B").
    pub fn any_of(branches: Vec<Trigger>) -> Self {
        let label = branches
            .iter()
            .enumerate()
            .map(|(idx, branch)| {
                if idx == 0 {
                    branch.label.clone()
                } else {
                    ["Whenever ", "When ", "At "]
                        .into_iter()
                        .find_map(|prefix| branch.label.strip_prefix(prefix).map(str::to_string))
                        .unwrap_or_else(|| branch.label.clone())
                }
            })
            .collect::<Vec<_>>()
            .join(" or ");
        Self::typed(label, TriggerKind::AnyOf(branches))
    }

    fn typed(label: impl Into<String>, kind: TriggerKind) -> Self {
        Self {
            label: label.into(),
            kind,
            intro_surface: None,
        }
    }

    pub fn with_intro_surface(mut self, intro: TriggerIntroSurface) -> Self {
        self.intro_surface = Some(intro);
        self
    }

    /// Preserve the authored trigger wording while retaining the typed matcher.
    ///
    /// Most trigger constructors use an internal label because their display
    /// is reconstructed elsewhere.  Front-end lowering sometimes has a more
    /// precise authored surface (for example, a zone-change provenance clause
    /// or an object/player damage union), so let it attach that presentation
    /// without replacing the runtime trigger kind.
    pub fn with_display_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn this_attacks() -> Self {
        Self::typed("this_attacks", TriggerKind::ThisAttacks)
    }
    pub fn this_attacks_player_who_controls_at_least(count: usize, filter: ObjectFilter) -> Self {
        Self::typed(
            "this_attacks_player_who_controls_at_least",
            TriggerKind::ThisAttacksPlayerWhoControlsAtLeast { count, filter },
        )
    }
    pub fn this_attacks_player_with_most_life() -> Self {
        Self::typed(
            "this_attacks_player_with_most_life",
            TriggerKind::ThisAttacksPlayerWithMostLife,
        )
    }
    pub fn this_attacks_with_greater_power() -> Self {
        Self::typed(
            "this_attacks_with_greater_power",
            TriggerKind::ThisAttacksWithGreaterPower,
        )
    }
    pub fn this_attacks_with_n_others_display_subject(
        count: usize,
        display_subject: Option<String>,
    ) -> Self {
        Self::this_attacks_with_n_others_display_subject_and_filter(count, display_subject, None)
    }

    pub fn this_attacks_with_n_others_display_subject_and_filter(
        count: usize,
        display_subject: Option<String>,
        other_filter: Option<ObjectFilter>,
    ) -> Self {
        Self::this_attacks_with_n_others_display_subject_filter_and_other_surface(
            count,
            display_subject,
            other_filter,
            true,
        )
    }

    pub fn this_attacks_with_n_others_display_subject_filter_and_other_surface(
        count: usize,
        display_subject: Option<String>,
        other_filter: Option<ObjectFilter>,
        other_surface: bool,
    ) -> Self {
        Self::typed(
            "this_attacks_with_n_others",
            TriggerKind::ThisAttacksWithNOthers {
                count,
                display_subject,
                other_filter,
                other_surface,
            },
        )
    }
    pub fn this_attacks_with_exact_n_others(count: usize) -> Self {
        Self::typed(
            "this_attacks_with_exact_n_others",
            TriggerKind::ThisAttacksWithExactNOthers { count },
        )
    }
    pub fn this_attacks_and_isnt_blocked() -> Self {
        Self::typed(
            "this_attacks_and_isnt_blocked",
            TriggerKind::ThisAttacksAndIsntBlocked,
        )
    }
    pub fn this_attacks_while_saddled() -> Self {
        Self::typed(
            "this_attacks_while_saddled",
            TriggerKind::ThisAttacksWhileSaddled,
        )
    }
    pub fn attacks(filter: ObjectFilter) -> Self {
        Self::typed("attacks", TriggerKind::Attacks { filter })
    }
    pub fn attacks_and_isnt_blocked(filter: ObjectFilter) -> Self {
        Self::typed(
            "attacks_and_isnt_blocked",
            TriggerKind::AttacksAndIsntBlocked { filter },
        )
    }
    pub fn attacks_while_saddled(filter: ObjectFilter) -> Self {
        Self::typed(
            "attacks_while_saddled",
            TriggerKind::AttacksWhileSaddled { filter },
        )
    }
    pub fn attacks_one_or_more(filter: ObjectFilter) -> Self {
        Self::typed(
            "attacks_one_or_more",
            TriggerKind::AttacksOneOrMore { filter },
        )
    }
    pub fn players_attacked_one_or_more(player_filter: PlayerFilter) -> Self {
        Self::typed(
            "players_attacked_one_or_more",
            TriggerKind::PlayersAttackedOneOrMore { player_filter },
        )
    }
    pub fn attacks_one_or_more_with_min_total(
        filter: ObjectFilter,
        min_total_attackers: usize,
    ) -> Self {
        Self::typed(
            "attacks_one_or_more_with_min_total",
            TriggerKind::AttacksOneOrMoreWithMinTotal {
                filter,
                min_total_attackers,
            },
        )
    }
    pub fn attacks_one_or_more_with_exact_total(
        filter: ObjectFilter,
        total_attackers: usize,
    ) -> Self {
        Self::typed(
            "attacks_one_or_more_with_exact_total",
            TriggerKind::AttacksOneOrMoreWithExactTotal {
                filter,
                total_attackers,
            },
        )
    }
    pub fn attacks_alone(filter: ObjectFilter) -> Self {
        Self::typed("attacks_alone", TriggerKind::AttacksAlone { filter })
    }
    pub fn attacks_you(filter: ObjectFilter) -> Self {
        Self::typed("attacks_you", TriggerKind::AttacksYou { filter })
    }
    pub fn attacks_you_one_or_more(filter: ObjectFilter) -> Self {
        Self::typed(
            "attacks_you_one_or_more",
            TriggerKind::AttacksYouOneOrMore { filter },
        )
    }
    pub fn this_blocks() -> Self {
        Self::typed("this_blocks", TriggerKind::ThisBlocks)
    }
    pub fn this_blocks_object(filter: ObjectFilter) -> Self {
        Self::typed(
            "this_blocks_object",
            TriggerKind::ThisBlocksObject {
                filter,
                min_blocked_objects: None,
            },
        )
    }
    pub fn this_blocks_objects_with_minimum(
        filter: ObjectFilter,
        min_blocked_objects: usize,
    ) -> Self {
        Self::typed(
            "this_blocks_objects_with_minimum",
            TriggerKind::ThisBlocksObject {
                filter,
                min_blocked_objects: Some(min_blocked_objects.max(1)),
            },
        )
    }
    pub fn blocks(filter: ObjectFilter) -> Self {
        Self::typed("blocks", TriggerKind::Blocks { filter })
    }
    pub fn blocks_one_or_more(filter: ObjectFilter) -> Self {
        Self::typed(
            "blocks_one_or_more",
            TriggerKind::BlocksOneOrMore { filter },
        )
    }
    pub fn blocks_object_with_lesser_power(blocker: ObjectFilter, blocked: ObjectFilter) -> Self {
        Self::typed(
            "blocks_object_with_lesser_power",
            TriggerKind::BlocksObjectWithLesserPower { blocker, blocked },
        )
    }
    pub fn this_becomes_blocked() -> Self {
        Self::typed("this_becomes_blocked", TriggerKind::ThisBecomesBlocked)
    }
    pub fn becomes_blocked(filter: ObjectFilter) -> Self {
        Self::typed("becomes_blocked", TriggerKind::BecomesBlocked { filter })
    }
    pub fn this_becomes_blocked_by_object(filter: ObjectFilter) -> Self {
        Self::typed(
            "this_becomes_blocked_by_object",
            TriggerKind::ThisBecomesBlockedByObject { filter },
        )
    }
    pub fn becomes_blocked_by_object_with_lesser_power(
        blocked: ObjectFilter,
        blocker: ObjectFilter,
    ) -> Self {
        Self::typed(
            "becomes_blocked_by_object_with_lesser_power",
            TriggerKind::BecomesBlockedByObjectWithLesserPower { blocked, blocker },
        )
    }
    pub fn this_dies() -> Self {
        Self::typed("this_dies", TriggerKind::ThisDies)
    }
    pub fn this_dies_or_is_exiled() -> Self {
        Self::typed("this_dies_or_is_exiled", TriggerKind::ThisDiesOrIsExiled)
    }
    pub fn this_dies_or_is_exiled_with_surface(surface: SourceReferenceSurface) -> Self {
        Self::typed(
            "this_dies_or_is_exiled",
            TriggerKind::ThisDiesOrIsExiledWithSurface { surface },
        )
    }
    pub fn this_leaves_battlefield() -> Self {
        Self::typed(
            "this_leaves_battlefield",
            TriggerKind::ThisLeavesBattlefield,
        )
    }
    pub fn this_phases_out() -> Self {
        Self::typed("When this phases out", TriggerKind::ThisPhasesOut)
    }
    pub fn this_mutates() -> Self {
        Self::typed("this_mutates", TriggerKind::ThisMutates)
    }
    pub fn leaves_battlefield(filter: ObjectFilter) -> Self {
        Self::typed(
            "leaves_battlefield",
            TriggerKind::LeavesBattlefield { filter },
        )
    }
    pub fn this_becomes_monstrous() -> Self {
        Self::typed("this_becomes_monstrous", TriggerKind::ThisBecomesMonstrous)
    }
    pub fn becomes_tapped() -> Self {
        Self::typed("becomes_tapped", TriggerKind::BecomesTapped)
    }
    pub fn permanent_becomes_tapped(filter: ObjectFilter) -> Self {
        Self::typed(
            "permanent_becomes_tapped",
            TriggerKind::PermanentBecomesTapped { filter },
        )
    }
    pub fn becomes_untapped() -> Self {
        Self::typed("becomes_untapped", TriggerKind::BecomesUntapped)
    }
    pub fn this_is_turned_face_up() -> Self {
        Self::typed("this_is_turned_face_up", TriggerKind::ThisIsTurnedFaceUp)
    }
    pub fn turned_face_up(filter: ObjectFilter) -> Self {
        Self::typed("turned_face_up", TriggerKind::TurnedFaceUp { filter })
    }
    pub fn becomes_targeted() -> Self {
        Self::typed("becomes_targeted", TriggerKind::BecomesTargeted)
    }
    pub fn becomes_targeted_object(filter: ObjectFilter) -> Self {
        Self::typed(
            "becomes_targeted_object",
            TriggerKind::BecomesTargetedObject { filter },
        )
    }
    pub fn becomes_targeted_by_spell(filter: ObjectFilter) -> Self {
        Self::typed(
            "becomes_targeted_by_spell",
            TriggerKind::BecomesTargetedBySpell { filter },
        )
    }
    pub fn becomes_targeted_by_stack_object(filter: ObjectFilter) -> Self {
        Self::typed(
            "becomes_targeted_by_stack_object",
            TriggerKind::BecomesTargetedByStackObject { filter },
        )
    }
    pub fn becomes_targeted_object_by_stack_object(
        target: ObjectFilter,
        source: ObjectFilter,
    ) -> Self {
        Self::typed(
            "becomes_targeted_object_by_stack_object",
            TriggerKind::BecomesTargetedObjectByStackObject { target, source },
        )
    }
    pub fn becomes_targeted_by_source_controller(
        target: ObjectFilter,
        controller: PlayerFilter,
    ) -> Self {
        Self::typed(
            "becomes_targeted_by_source_controller",
            TriggerKind::BecomesTargetedBySourceController { target, controller },
        )
    }
    pub fn player_or_object_becomes_targeted_by_source_controller(
        player: PlayerFilter,
        object: ObjectFilter,
        controller: PlayerFilter,
    ) -> Self {
        let controller_text = match controller {
            PlayerFilter::You => "you",
            PlayerFilter::Opponent => "an opponent",
            PlayerFilter::Any => "a player",
            _ => "a player",
        };
        Self::typed(
            format!(
                "Whenever {} or {} becomes the target of a spell or ability {} controls",
                crate::filter_model::describe_player_filter(&player),
                object.description(),
                controller_text
            ),
            TriggerKind::PlayerOrObjectBecomesTargetedBySourceController {
                player,
                object,
                controller,
            },
        )
    }
    pub fn this_deals_damage() -> Self {
        Self::typed("this_deals_damage", TriggerKind::ThisDealsDamage)
    }
    pub fn this_deals_damage_to_player(player: PlayerFilter, amount: Option<Comparison>) -> Self {
        Self::typed(
            "this_deals_damage_to_player",
            TriggerKind::ThisDealsDamageToPlayer { player, amount },
        )
    }
    pub fn this_deals_damage_to(filter: ObjectFilter) -> Self {
        Self::typed(
            "this_deals_damage_to",
            TriggerKind::ThisDealsDamageTo { filter },
        )
    }
    pub fn this_deals_combat_damage() -> Self {
        Self::typed(
            "this_deals_combat_damage",
            TriggerKind::ThisDealsCombatDamage,
        )
    }
    pub fn this_deals_combat_damage_to(filter: ObjectFilter) -> Self {
        Self::typed(
            "this_deals_combat_damage_to",
            TriggerKind::ThisDealsCombatDamageTo { filter },
        )
    }
    pub fn this_deals_combat_damage_to_player(player: PlayerFilter) -> Self {
        Self::typed(
            "this_deals_combat_damage_to_player",
            TriggerKind::ThisDealsCombatDamageToPlayer {
                player,
                source_surface: None,
            },
        )
    }
    pub fn this_deals_combat_damage_to_player_with_surface(
        player: PlayerFilter,
        source_surface: SourceReferenceSurface,
    ) -> Self {
        Self::typed(
            "this_deals_combat_damage_to_player",
            TriggerKind::ThisDealsCombatDamageToPlayer {
                player,
                source_surface: Some(source_surface),
            },
        )
    }
    pub fn deals_damage(filter: ObjectFilter) -> Self {
        Self::deals_damage_with_source_surface(filter, DamageSourceSurface::Filter)
    }
    pub fn deals_damage_with_source_surface(
        filter: ObjectFilter,
        source_surface: DamageSourceSurface,
    ) -> Self {
        Self::typed(
            "deals_damage",
            TriggerKind::DealsDamage {
                filter,
                source_surface,
            },
        )
    }
    pub fn deals_damage_to(source: ObjectFilter, target: ObjectFilter) -> Self {
        Self::deals_damage_to_with_source_surface(source, target, DamageSourceSurface::Filter)
    }
    pub fn deals_damage_to_with_source_surface(
        source: ObjectFilter,
        target: ObjectFilter,
        source_surface: DamageSourceSurface,
    ) -> Self {
        Self::typed(
            "deals_damage_to",
            TriggerKind::DealsDamageTo {
                source,
                target,
                source_surface,
            },
        )
    }
    pub fn deals_damage_to_player(source: ObjectFilter, player: PlayerFilter) -> Self {
        Self::deals_damage_to_player_with_source_surface(
            source,
            player,
            DamageSourceSurface::Filter,
        )
    }
    pub fn deals_damage_to_player_with_source_surface(
        source: ObjectFilter,
        player: PlayerFilter,
        source_surface: DamageSourceSurface,
    ) -> Self {
        let source_description = match source_surface {
            DamageSourceSurface::Source => "this source".to_string(),
            DamageSourceSurface::Filter | DamageSourceSurface::PassiveBy => {
                if source == ObjectFilter::default() {
                    "a source".to_string()
                } else {
                    source.description()
                }
            }
        };
        let label = if source_surface == DamageSourceSurface::PassiveBy {
            format!(
                "Whenever {} is dealt damage by {source_description}",
                player.description()
            )
        } else {
            format!(
                "Whenever {source_description} deals damage to {}",
                player.description()
            )
        };
        Self::typed(
            label,
            TriggerKind::DealsDamageToPlayer {
                source,
                player,
                source_surface,
            },
        )
    }
    pub fn deals_noncombat_damage_to_player(source: ObjectFilter, player: PlayerFilter) -> Self {
        Self::deals_noncombat_damage_to_player_with_source_surface(
            source,
            player,
            DamageSourceSurface::Filter,
        )
    }
    pub fn deals_noncombat_damage_to_player_with_source_surface(
        source: ObjectFilter,
        player: PlayerFilter,
        source_surface: DamageSourceSurface,
    ) -> Self {
        Self::typed(
            "deals_noncombat_damage_to_player",
            TriggerKind::DealsNoncombatDamageToPlayer {
                source,
                player,
                source_surface,
            },
        )
    }
    pub fn deals_combat_damage(filter: ObjectFilter) -> Self {
        Self::typed(
            "deals_combat_damage",
            TriggerKind::DealsCombatDamage { filter },
        )
    }
    pub fn deals_combat_damage_to(source: ObjectFilter, target: ObjectFilter) -> Self {
        Self::typed(
            "deals_combat_damage_to",
            TriggerKind::DealsCombatDamageTo { source, target },
        )
    }
    pub fn deals_combat_damage_to_player(source: ObjectFilter, player: PlayerFilter) -> Self {
        Self::typed(
            "deals_combat_damage_to_player",
            TriggerKind::DealsCombatDamageToPlayer {
                source,
                player,
                one_or_more: false,
            },
        )
    }
    pub fn deals_combat_damage_to_player_one_or_more(
        source: ObjectFilter,
        player: PlayerFilter,
    ) -> Self {
        Self::typed(
            "deals_combat_damage_to_player_one_or_more",
            TriggerKind::DealsCombatDamageToPlayer {
                source,
                player,
                one_or_more: true,
            },
        )
    }
    pub fn player_plays_land(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self::typed(
            "player_plays_land",
            TriggerKind::PlayerPlaysLand { player, filter },
        )
    }
    pub fn player_gives_gift(player: PlayerFilter) -> Self {
        Self::typed("player_gives_gift", TriggerKind::PlayerGivesGift { player })
    }
    pub fn player_searches_library(player: PlayerFilter) -> Self {
        Self::typed(
            "player_searches_library",
            TriggerKind::PlayerSearchesLibrary { player },
        )
    }
    pub fn player_shuffles_library(
        player: PlayerFilter,
        caused_by_effect: bool,
        source_controller_shuffles: bool,
    ) -> Self {
        Self::typed(
            "player_shuffles_library",
            TriggerKind::PlayerShufflesLibrary {
                player,
                caused_by_effect,
                source_controller_shuffles,
            },
        )
    }
    pub fn player_taps_for_mana(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self::typed(
            "player_taps_for_mana",
            TriggerKind::PlayerTapsForMana { player, filter },
        )
    }
    pub fn player_rolls_result(player: PlayerFilter, result: u32) -> Self {
        Self::typed(
            "player_rolls_result",
            TriggerKind::PlayerRollsResult { player, result },
        )
    }
    pub fn player_rolls_highest_natural_result(player: PlayerFilter) -> Self {
        Self::typed(
            "player_rolls_highest_natural_result",
            TriggerKind::PlayerRollsHighestNaturalResult { player },
        )
    }
    pub fn player_rolls_die(player: PlayerFilter) -> Self {
        Self::player_rolls_die_with_surface(player, false)
    }
    pub fn player_rolls_die_with_surface(player: PlayerFilter, one_or_more: bool) -> Self {
        Self::typed(
            "player_rolls_die",
            TriggerKind::PlayerRollsDie {
                player,
                one_or_more,
            },
        )
    }
    pub fn player_coin_flip_result(player: PlayerFilter, won: bool) -> Self {
        Self::typed(
            "player_coin_flip_result",
            TriggerKind::PlayerCoinFlipResult { player, won },
        )
    }
    pub fn ability_activated_qualified(
        activator: PlayerFilter,
        filter: ObjectFilter,
        non_mana_only: bool,
        loyalty_only: bool,
    ) -> Self {
        Self::ability_activated_qualified_with_activation_cost_tap(
            activator,
            filter,
            non_mana_only,
            loyalty_only,
            None,
        )
    }

    pub fn ability_activated_qualified_with_activation_cost_tap(
        activator: PlayerFilter,
        filter: ObjectFilter,
        non_mana_only: bool,
        loyalty_only: bool,
        activation_cost_has_tap: Option<bool>,
    ) -> Self {
        Self::typed(
            "ability_activated_qualified",
            TriggerKind::AbilityActivatedQualified {
                activator,
                filter,
                non_mana_only,
                loyalty_only,
                activation_cost_has_tap,
            },
        )
    }
    pub fn ability_triggered(another: bool) -> Self {
        Self::typed(
            if another {
                "Whenever another ability triggers"
            } else {
                "Whenever an ability triggers"
            },
            TriggerKind::AbilityTriggered { another },
        )
    }
    pub fn is_dealt_damage(target: ChooseSpec) -> Self {
        Self::typed(
            "is_dealt_damage",
            TriggerKind::IsDealtDamage {
                target,
                combat_only: false,
            },
        )
    }
    pub fn is_dealt_combat_damage(target: ChooseSpec) -> Self {
        Self::typed(
            "is_dealt_damage",
            TriggerKind::IsDealtDamage {
                target,
                combat_only: true,
            },
        )
    }
    pub fn you_gain_life() -> Self {
        Self::typed("you_gain_life", TriggerKind::YouGainLife)
    }
    pub fn you_gain_life_caused_by(source: ObjectFilter) -> Self {
        let description = source.description();
        let source_description = if description.starts_with("a ")
            || description.starts_with("an ")
            || description.starts_with("the ")
        {
            description
        } else {
            let article = if matches!(
                description
                    .chars()
                    .next()
                    .map(|character| character.to_ascii_lowercase()),
                Some('a' | 'e' | 'i' | 'o' | 'u')
            ) {
                "an"
            } else {
                "a"
            };
            format!("{article} {description}")
        };
        Self::typed(
            format!("Whenever {source_description} causes you to gain life"),
            TriggerKind::YouGainLifeCausedBy { source },
        )
    }
    pub fn you_gain_life_during_turn(during_turn: PlayerFilter) -> Self {
        Self::typed(
            "you_gain_life_during_turn",
            TriggerKind::YouGainLifeDuringTurn { during_turn },
        )
    }
    pub fn player_loses_life(player: PlayerFilter) -> Self {
        Self::typed("player_loses_life", TriggerKind::PlayerLosesLife { player })
    }
    pub fn players_lose_life_one_or_more(player: PlayerFilter) -> Self {
        Self::typed(
            "players_lose_life_one_or_more",
            TriggerKind::PlayersLoseLifeOneOrMore { player },
        )
    }
    pub fn opponents_each_lose_exact_life(amount: u32) -> Self {
        Self::typed(
            "opponents_each_lose_exact_life",
            TriggerKind::OpponentsEachLoseExactLife { amount },
        )
    }
    pub fn player_loses_game(player: PlayerFilter) -> Self {
        Self::typed("player_loses_game", TriggerKind::PlayerLosesGame { player })
    }
    pub fn player_loses_life_during_turn(player: PlayerFilter, during_turn: PlayerFilter) -> Self {
        Self::typed(
            "player_loses_life_during_turn",
            TriggerKind::PlayerLosesLifeDuringTurn {
                player,
                during_turn,
            },
        )
    }
    pub fn spell_countered(filter: Option<ObjectFilter>, controller: PlayerFilter) -> Self {
        Self::typed(
            "spell_countered",
            TriggerKind::SpellCountered { filter, controller },
        )
    }
    pub fn you_draw_card() -> Self {
        Self::typed("you_draw_card", TriggerKind::YouDrawCard)
    }
    pub fn player_draws_card(player: PlayerFilter) -> Self {
        Self::typed("player_draws_card", TriggerKind::PlayerDrawsCard { player })
    }
    pub fn player_draws_card_not_during_turn(
        player: PlayerFilter,
        during_turn: PlayerFilter,
    ) -> Self {
        Self::typed(
            "player_draws_card_not_during_turn",
            TriggerKind::PlayerDrawsCardNotDuringTurn {
                player,
                during_turn,
            },
        )
    }
    pub fn player_draws_card_except_first_in_draw_step(player: PlayerFilter) -> Self {
        Self::typed(
            "player_draws_card_except_first_in_draw_step",
            TriggerKind::PlayerDrawsCardExceptFirstInDrawStep { player },
        )
    }
    pub fn player_draws_nth_card_each_turn(player: PlayerFilter, card_number: u32) -> Self {
        Self::typed(
            "player_draws_nth_card_each_turn",
            TriggerKind::PlayerDrawsNthCardEachTurn {
                player,
                card_number,
            },
        )
    }
    pub fn player_draws_numbered_cards_each_turn(
        player: PlayerFilter,
        card_numbers: impl IntoIterator<Item = u32>,
    ) -> Self {
        let mut card_numbers = card_numbers
            .into_iter()
            .filter(|number| *number > 0)
            .collect::<Vec<_>>();
        card_numbers.sort_unstable();
        card_numbers.dedup();
        Self::typed(
            "player_draws_numbered_cards_each_turn",
            TriggerKind::PlayerDrawsNumberedCardsEachTurn {
                player,
                card_numbers,
            },
        )
    }
    pub fn player_discards_card_caused_by_controller(
        player: PlayerFilter,
        filter: Option<ObjectFilter>,
        controller: PlayerFilter,
        effect_like_only: bool,
    ) -> Self {
        Self::typed(
            "player_discards_card_caused_by_controller",
            TriggerKind::PlayerDiscardsCardCausedByController {
                player,
                filter,
                controller,
                effect_like_only,
            },
        )
    }
    pub fn player_discards_card(player: PlayerFilter, filter: Option<ObjectFilter>) -> Self {
        Self::typed(
            "player_discards_card",
            TriggerKind::PlayerDiscardsCard {
                player,
                filter,
                one_or_more: false,
            },
        )
    }
    pub fn player_discards_cards(player: PlayerFilter, filter: Option<ObjectFilter>) -> Self {
        Self::typed(
            "player_discards_cards",
            TriggerKind::PlayerDiscardsCard {
                player,
                filter,
                one_or_more: true,
            },
        )
    }
    pub fn player_reveals_card(
        player: PlayerFilter,
        filter: ObjectFilter,
        from_source: bool,
    ) -> Self {
        Self::typed(
            "player_reveals_card",
            TriggerKind::PlayerRevealsCard {
                player,
                filter,
                from_source,
            },
        )
    }
    pub fn player_sacrifices(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self::player_sacrifices_with_surface(player, filter, false)
    }
    pub fn player_sacrifices_with_surface(
        player: PlayerFilter,
        filter: ObjectFilter,
        one_or_more_surface: bool,
    ) -> Self {
        Self::typed(
            "player_sacrifices",
            TriggerKind::PlayerSacrifices {
                player,
                filter,
                one_or_more_surface,
            },
        )
    }
    pub fn tokens_created(player: PlayerFilter, filter: ObjectFilter, one_or_more: bool) -> Self {
        Self::typed(
            "tokens_created",
            TriggerKind::TokensCreated {
                player,
                filter,
                one_or_more,
            },
        )
    }
    pub fn dies(filter: ObjectFilter) -> Self {
        Self::typed("dies", TriggerKind::Dies { filter })
    }
    pub fn put_into_graveyard(filter: ObjectFilter) -> Self {
        Self::typed(
            "put_into_graveyard",
            TriggerKind::PutIntoGraveyard { filter },
        )
    }
    pub fn cards_leave_your_graveyard(
        filter: ObjectFilter,
        one_or_more: bool,
        during_your_turn: bool,
    ) -> Self {
        Self::typed(
            "cards_leave_your_graveyard",
            TriggerKind::CardsLeaveYourGraveyard {
                filter,
                one_or_more,
                during_your_turn,
            },
        )
    }
    pub fn creature_dealt_damage_by_this_creature_this_turn_dies(victim: ObjectFilter) -> Self {
        Self::typed(
            "creature_dealt_damage_by_this_creature_this_turn_dies",
            TriggerKind::DiesCreatureDealtDamageByThisTurn {
                victim,
                damager: DamagedBySource::ThisCreature,
            },
        )
    }
    pub fn creature_dealt_damage_by_equipped_creature_this_turn_dies(victim: ObjectFilter) -> Self {
        Self::typed(
            "creature_dealt_damage_by_equipped_creature_this_turn_dies",
            TriggerKind::DiesCreatureDealtDamageByThisTurn {
                victim,
                damager: DamagedBySource::EquippedCreature,
            },
        )
    }
    pub fn creature_dealt_damage_by_enchanted_creature_this_turn_dies(
        victim: ObjectFilter,
    ) -> Self {
        Self::typed(
            "creature_dealt_damage_by_enchanted_creature_this_turn_dies",
            TriggerKind::DiesCreatureDealtDamageByThisTurn {
                victim,
                damager: DamagedBySource::EnchantedCreature,
            },
        )
    }
    pub fn spell_cast_qualified(
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        timing: Option<TriggerTimingRestriction>,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    ) -> Self {
        Self::spell_cast_qualified_with_mana_source(
            filter,
            None,
            caster,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        )
    }
    pub fn spell_cast_qualified_with_mana_source(
        filter: Option<ObjectFilter>,
        mana_source_filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        timing: Option<TriggerTimingRestriction>,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    ) -> Self {
        Self::typed(
            "spell_cast_qualified",
            TriggerKind::SpellCastQualified {
                filter,
                mana_source_filter,
                caster,
                timing,
                during_turn,
                min_spells_this_turn,
                exact_spells_this_turn,
                from_not_hand,
            },
        )
    }
    pub fn spell_cast(filter: Option<ObjectFilter>, caster: PlayerFilter) -> Self {
        Self::typed("spell_cast", TriggerKind::SpellCast { filter, caster })
    }
    pub fn nth_spell_of_turn_cast(spell_number: u32) -> Self {
        Self::typed(
            "nth_spell_of_turn_cast",
            TriggerKind::NthSpellOfTurnCast { spell_number },
        )
    }
    pub fn nth_counter_put_on(
        filter: ObjectFilter,
        counter_type: CounterType,
        counter_number: u32,
    ) -> Self {
        Self::typed(
            "nth_counter_put_on",
            TriggerKind::NthCounterPutOn {
                filter,
                counter_type,
                counter_number,
            },
        )
    }
    pub fn spell_copied(filter: Option<ObjectFilter>, copier: PlayerFilter) -> Self {
        Self::typed("spell_copied", TriggerKind::SpellCopied { filter, copier })
    }
    pub fn enters_battlefield(filter: ObjectFilter, cause_filter: Option<CauseFilter>) -> Self {
        Self::typed(
            "enters_battlefield",
            TriggerKind::EntersBattlefield {
                filter,
                cause_filter,
                count: CountMode::One,
                tapped: None,
            },
        )
    }
    pub fn enters_battlefield_one_or_more(
        filter: ObjectFilter,
        cause_filter: Option<CauseFilter>,
    ) -> Self {
        Self::typed(
            "enters_battlefield_one_or_more",
            TriggerKind::EntersBattlefield {
                filter,
                cause_filter,
                count: CountMode::OneOrMore,
                tapped: None,
            },
        )
    }
    pub fn enters_battlefield_tapped(
        filter: ObjectFilter,
        cause_filter: Option<CauseFilter>,
    ) -> Self {
        Self::typed(
            "enters_battlefield_tapped",
            TriggerKind::EntersBattlefield {
                filter,
                cause_filter,
                count: CountMode::One,
                tapped: Some(true),
            },
        )
    }
    pub fn enters_battlefield_untapped(
        filter: ObjectFilter,
        cause_filter: Option<CauseFilter>,
    ) -> Self {
        Self::typed(
            "enters_battlefield_untapped",
            TriggerKind::EntersBattlefield {
                filter,
                cause_filter,
                count: CountMode::One,
                tapped: Some(false),
            },
        )
    }
    pub fn beginning_of_upkeep(player: PlayerFilter) -> Self {
        Self::typed(
            "beginning_of_upkeep",
            TriggerKind::BeginningOfUpkeep { player },
        )
    }
    pub fn beginning_of_draw_step(player: PlayerFilter) -> Self {
        Self::typed(
            "beginning_of_draw_step",
            TriggerKind::BeginningOfDrawStep { player },
        )
    }
    pub fn beginning_of_combat(player: PlayerFilter) -> Self {
        Self::typed(
            "beginning_of_combat",
            TriggerKind::BeginningOfCombat { player },
        )
    }
    pub fn end_of_combat() -> Self {
        Self::typed("end_of_combat", TriggerKind::EndOfCombat)
    }
    pub fn beginning_of_end_step(player: PlayerFilter) -> Self {
        Self::typed(
            "beginning_of_end_step",
            TriggerKind::BeginningOfEndStep {
                player,
                surface: EndStepSurface::Each,
            },
        )
    }
    pub fn beginning_of_the_end_step() -> Self {
        Self::typed(
            "beginning_of_end_step",
            TriggerKind::BeginningOfEndStep {
                player: PlayerFilter::Any,
                surface: EndStepSurface::Definite,
            },
        )
    }
    pub fn beginning_of_precombat_main_phase(player: PlayerFilter) -> Self {
        Self::typed(
            "beginning_of_precombat_main_phase",
            TriggerKind::BeginningOfPrecombatMainPhase { player },
        )
    }
    pub fn beginning_of_postcombat_main_phase(player: PlayerFilter) -> Self {
        Self::typed(
            "beginning_of_postcombat_main_phase",
            TriggerKind::BeginningOfPostcombatMainPhase { player },
        )
    }
    pub fn day_night_changed() -> Self {
        Self::typed("day_night_changed", TriggerKind::DayNightChanged)
    }
    pub fn this_enters_battlefield() -> Self {
        Self::typed(
            "this_enters_battlefield",
            TriggerKind::ThisEntersBattlefield,
        )
    }
    pub fn transforms() -> Self {
        Self::transforms_with_destination(None)
    }
    pub fn transforms_with_destination(destination_name: Option<String>) -> Self {
        Self::typed(
            "this_transforms",
            TriggerKind::ThisTransforms { destination_name },
        )
    }
    pub fn transforms_with_surface(surface: SourceReferenceSurface) -> Self {
        Self::transforms_with_surface_and_destination(surface, None)
    }
    pub fn transforms_with_surface_and_destination(
        surface: SourceReferenceSurface,
        destination_name: Option<String>,
    ) -> Self {
        Self::typed(
            "this_transforms",
            TriggerKind::ThisTransformsWithSurface {
                surface,
                destination_name,
            },
        )
    }
    pub fn you_cast_this_spell() -> Self {
        Self::typed("you_cast_this_spell", TriggerKind::YouCastThisSpell)
    }
    pub fn keyword_action_matching_object(
        action: KeywordActionKind,
        player: PlayerFilter,
        filter: ObjectFilter,
    ) -> Self {
        Self::typed(
            "keyword_action_matching_object",
            TriggerKind::KeywordActionMatchingObject {
                action,
                player,
                filter,
            },
        )
    }
    pub fn keyword_action_matching_source_and_tagged_object(
        action: KeywordActionKind,
        player: PlayerFilter,
        source_filter: ObjectFilter,
        object_tag: TagKey,
        object_filter: ObjectFilter,
    ) -> Self {
        Self::typed(
            "keyword_action_matching_source_and_tagged_object",
            TriggerKind::KeywordActionMatchingTaggedObject {
                action,
                player,
                source_filter,
                object_tag,
                object_filter,
                during_your_main_phase: false,
            },
        )
    }
    pub fn keyword_action_matching_source_and_tagged_object_during_your_main_phase(
        action: KeywordActionKind,
        player: PlayerFilter,
        source_filter: ObjectFilter,
        object_tag: TagKey,
        object_filter: ObjectFilter,
    ) -> Self {
        Self::typed(
            "keyword_action_matching_source_and_tagged_object",
            TriggerKind::KeywordActionMatchingTaggedObject {
                action,
                player,
                source_filter,
                object_tag,
                object_filter,
                during_your_main_phase: true,
            },
        )
    }
    pub fn keyword_action(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self::typed(
            "keyword_action",
            TriggerKind::KeywordAction { action, player },
        )
    }
    pub fn keyword_action_from_source(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self::typed(
            "keyword_action_from_source",
            TriggerKind::KeywordActionFromSource { action, player },
        )
    }
    pub fn wins_clash(player: PlayerFilter) -> Self {
        Self::wins_clash_with_surface(player, ClashWinTriggerSurface::WinAClash)
    }
    pub fn wins_clash_with_surface(player: PlayerFilter, surface: ClashWinTriggerSurface) -> Self {
        let label = match (&player, surface) {
            (PlayerFilter::You, ClashWinTriggerSurface::ClashAndWin) => {
                "Whenever you clash and win"
            }
            (PlayerFilter::Opponent, ClashWinTriggerSurface::ClashAndWin) => {
                "Whenever an opponent clashes and wins"
            }
            (PlayerFilter::Any, ClashWinTriggerSurface::ClashAndWin) => {
                "Whenever a player clashes and wins"
            }
            (PlayerFilter::You, ClashWinTriggerSurface::WinAClash) => "Whenever you win a clash",
            (PlayerFilter::Opponent, ClashWinTriggerSurface::WinAClash) => {
                "Whenever an opponent wins a clash"
            }
            (_, ClashWinTriggerSurface::ClashAndWin) => "Whenever a player clashes and wins",
            (_, ClashWinTriggerSurface::WinAClash) => "Whenever a player wins a clash",
        };
        Self::typed(label, TriggerKind::WinsClash { player, surface })
    }
    pub fn expend(amount: u32, player: PlayerFilter) -> Self {
        Self::typed("expend", TriggerKind::Expend { amount, player })
    }
    pub fn saga_chapter(chapters: Vec<u32>) -> Self {
        Self::typed("saga_chapter", TriggerKind::SagaChapter { chapters })
    }
    pub fn final_chapter_ability_resolved(filter: ObjectFilter) -> Self {
        Self::typed(
            "final_chapter_ability_resolved",
            TriggerKind::FinalChapterAbilityResolved { filter },
        )
    }
    pub fn custom(id: impl Into<String>, label: String) -> Self {
        let id = id.into();
        Self::typed(label.clone(), TriggerKind::Custom { id, label })
    }
    pub fn either(left: Trigger, right: Trigger) -> Self {
        Self::typed(
            "either",
            TriggerKind::Either {
                left: Box::new(left),
                right: Box::new(right),
            },
        )
    }
    pub fn display(&self) -> String {
        let Some(intro) = self.intro_surface else {
            return self.label.clone();
        };
        ["Whenever ", "When ", "At "]
            .into_iter()
            .find_map(|prefix| self.label.strip_prefix(prefix))
            .map(|rest| format!("{} {rest}", intro.as_str()))
            .unwrap_or_else(|| self.label.clone())
    }
}

pub trait CompilerTriggerMatcher {
    fn into_trigger(self) -> Trigger;
}

/// Authored surface for the entered-object subject of a zone-change origin
/// condition ("if it entered from ..." versus "if that creature entered
/// from ..."). Presentation-only; never read while matching.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OriginConditionSubjectSurface {
    /// "it" (or "one or more of them" for batch triggers).
    #[default]
    It,
    /// A demonstrative subject such as "that creature".
    That(String),
}

/// Additional provenance required for a zone-change trigger to match.
#[derive(Debug, Clone, PartialEq)]
pub enum ZoneChangeOriginCondition {
    /// The object either moved directly from this zone or was cast from this
    /// zone before entering the destination zone from the stack.
    MovedFromOrCastFrom {
        /// The origin zone.
        zone: Zone,
        /// Required owner of the origin zone (for player-owned zones such as
        /// graveyards and hands). `None` accepts any owner.
        zone_owner: Option<PlayerFilter>,
        /// Required caster for the "was cast from" branch. `None` accepts any
        /// caster.
        caster: Option<PlayerFilter>,
        /// Authored subject wording. Presentation-only.
        subject_surface: OriginConditionSubjectSurface,
    },
}

impl ZoneChangeOriginCondition {
    /// The unscoped form: moved from `zone` or cast from `zone`, by anyone.
    pub fn moved_from_or_cast_from(zone: Zone) -> Self {
        Self::MovedFromOrCastFrom {
            zone,
            zone_owner: None,
            caster: None,
            subject_surface: OriginConditionSubjectSurface::It,
        }
    }

    /// The ", if it entered from X or was cast from X" display suffix.
    pub fn display_suffix(&self, plural: bool) -> String {
        fn origin_zone_phrase(zone: Zone, owner: Option<&PlayerFilter>) -> String {
            let owned = |noun: &str| match owner {
                Some(PlayerFilter::You) => format!("your {noun}"),
                Some(PlayerFilter::Opponent) => format!("an opponent's {noun}"),
                _ => format!("a {noun}"),
            };
            match zone {
                Zone::Graveyard => owned("graveyard"),
                Zone::Hand => owned("hand"),
                Zone::Library => owned("library"),
                Zone::Battlefield => "the battlefield".to_string(),
                Zone::Stack => "the stack".to_string(),
                Zone::Exile => "exile".to_string(),
                Zone::Command => "the command zone".to_string(),
                Zone::Ante => "ante".to_string(),
                Zone::OutsideGame => "outside the game".to_string(),
            }
        }

        let Self::MovedFromOrCastFrom {
            zone,
            zone_owner,
            caster,
            subject_surface,
        } = self;
        let zone_phrase = origin_zone_phrase(*zone, zone_owner.as_ref());
        let entered_subject = if plural {
            "one or more of them".to_string()
        } else {
            match subject_surface {
                OriginConditionSubjectSurface::It => "it".to_string(),
                OriginConditionSubjectSurface::That(subject) => subject.clone(),
            }
        };
        let cast_clause = match caster {
            Some(PlayerFilter::You) => {
                let object = if plural { "them" } else { "it" };
                format!("you cast {object} from {zone_phrase}")
            }
            _ => format!("was cast from {zone_phrase}"),
        };
        format!(", if {entered_subject} entered from {zone_phrase} or {cast_clause}")
    }
}

/// Grammatical number of an explicitly authored source-object trigger subject.
///
/// This is presentation metadata only: it preserves distinctions such as
/// "When Ran and Shaw enter" versus "When Hidetsugu and Kairi enters" without
/// guessing from the card name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriggerSubjectNumber {
    #[default]
    Singular,
    Plural,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneChangeTrigger {
    pub from: Option<Zone>,
    pub from_zones: Option<Vec<Zone>>,
    pub to: Option<Zone>,
    pub filter: Option<ObjectFilter>,
    pub this: bool,
    pub this_surface: Option<SourceReferenceSurface>,
    pub this_subject_number: TriggerSubjectNumber,
    pub count: CountMode,
    pub cause_filter: Option<CauseFilter>,
    pub during_turn: Option<PlayerFilter>,
    pub origin_condition: Option<ZoneChangeOriginCondition>,
    pub graveyard_surface: Option<GraveyardTriggerSurface>,
}

impl ZoneChangeTrigger {
    pub fn new() -> Self {
        Self {
            from: None,
            from_zones: None,
            to: None,
            filter: None,
            this: false,
            this_surface: None,
            this_subject_number: TriggerSubjectNumber::Singular,
            count: CountMode::One,
            cause_filter: None,
            during_turn: None,
            origin_condition: None,
            graveyard_surface: None,
        }
    }

    pub fn count(mut self, mode: CountMode) -> Self {
        self.count = mode;
        self
    }

    pub fn from(mut self, zone: Zone) -> Self {
        self.from = Some(zone);
        self.from_zones = None;
        self
    }

    pub fn from_any_of(mut self, zones: Vec<Zone>) -> Self {
        if zones.len() == 1 {
            self.from = zones.first().copied();
            self.from_zones = None;
        } else {
            self.from = None;
            self.from_zones = Some(zones);
        }
        self
    }

    pub fn to(mut self, zone: Zone) -> Self {
        self.to = Some(zone);
        self
    }

    pub fn filter(mut self, filter: ObjectFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn this(mut self) -> Self {
        self.this = true;
        self
    }

    pub fn this_surface(mut self, surface: SourceReferenceSurface) -> Self {
        self.this_surface = Some(surface);
        self
    }

    pub fn this_subject_number(mut self, number: TriggerSubjectNumber) -> Self {
        self.this_subject_number = number;
        self
    }

    pub fn cause_filter(mut self, filter: Option<CauseFilter>) -> Self {
        self.cause_filter = filter;
        self
    }

    pub fn during_turn(mut self, player: PlayerFilter) -> Self {
        self.during_turn = Some(player);
        self
    }

    pub fn origin_condition(mut self, condition: ZoneChangeOriginCondition) -> Self {
        self.origin_condition = Some(condition);
        self
    }

    pub fn graveyard_surface(mut self, surface: GraveyardTriggerSurface) -> Self {
        self.graveyard_surface = Some(surface);
        self
    }
}

impl Default for ZoneChangeTrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerTriggerMatcher for ZoneChangeTrigger {
    fn into_trigger(self) -> Trigger {
        Trigger::typed("zone_change", TriggerKind::ZoneChange(self))
    }
}

pub mod zone_changes {
    pub use super::ZoneChangeTrigger;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerGetsCountersTrigger {
    pub player: PlayerFilter,
    pub counter_type: Option<CounterType>,
    pub count: CountMode,
}

impl PlayerGetsCountersTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            counter_type: None,
            count: CountMode::One,
        }
    }

    pub fn counter_type(mut self, counter_type: CounterType) -> Self {
        self.counter_type = Some(counter_type);
        self
    }

    pub fn count(mut self, mode: CountMode) -> Self {
        self.count = mode;
        self
    }
}

impl CompilerTriggerMatcher for PlayerGetsCountersTrigger {
    fn into_trigger(self) -> Trigger {
        Trigger::typed(
            "player_gets_counters",
            TriggerKind::PlayerGetsCounters(self),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CounterPutOnTrigger {
    pub filter: ObjectFilter,
    pub counter_type: Option<CounterType>,
    pub source_controller: Option<PlayerFilter>,
    pub count: CountMode,
    /// "on a permanent or player" — counters placed on players match too.
    pub include_players: bool,
}

impl CounterPutOnTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter_type: None,
            source_controller: None,
            count: CountMode::One,
            include_players: false,
        }
    }

    pub fn include_players(mut self) -> Self {
        self.include_players = true;
        self
    }

    pub fn counter_type(mut self, counter_type: CounterType) -> Self {
        self.counter_type = Some(counter_type);
        self
    }

    pub fn source_controller(mut self, controller: PlayerFilter) -> Self {
        self.source_controller = Some(controller);
        self
    }

    pub fn count(mut self, mode: CountMode) -> Self {
        self.count = mode;
        self
    }
}

impl CompilerTriggerMatcher for CounterPutOnTrigger {
    fn into_trigger(self) -> Trigger {
        Trigger::typed("counter_put_on", TriggerKind::CounterPutOn(self))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CounterRemovedFromTrigger {
    pub filter: ObjectFilter,
    /// Preserve the grouped Oracle surface "one or more counters". A grouped
    /// marker-change event still queues this trigger exactly once.
    pub one_or_more: bool,
    /// Require the counter removal event to have been caused by this trigger's
    /// source. This is the event-provenance meaning of Oracle's "this way".
    pub caused_by_source: bool,
}

impl CounterRemovedFromTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: false,
            caused_by_source: false,
        }
    }

    pub fn one_or_more(mut self) -> Self {
        self.one_or_more = true;
        self
    }

    pub fn caused_by_source(mut self) -> Self {
        self.caused_by_source = true;
        self
    }
}

impl CompilerTriggerMatcher for CounterRemovedFromTrigger {
    fn into_trigger(self) -> Trigger {
        Trigger::typed(
            "counter_removed_from",
            TriggerKind::CounterRemovedFrom(self),
        )
    }
}
