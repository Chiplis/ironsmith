use crate::{
    CauseFilter, ChooseSpec, CounterType, KeywordActionKind, ObjectFilter, PlayerFilter,
    SourceReferenceSurface, Zone, filter_model::Comparison,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountMode {
    One,
    OneOrMore,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DamagedBySource {
    ThisCreature,
    EquippedCreature,
    EnchantedCreature,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerKind {
    StateBased {
        display: String,
    },
    ThisAttacks,
    ThisAttacksPlayerWithMostLife,
    ThisAttacksWithGreaterPower,
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
    AttacksOneOrMoreWithMinTotal {
        filter: ObjectFilter,
        min_total_attackers: usize,
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
    },
    Blocks {
        filter: ObjectFilter,
    },
    ThisBecomesBlocked,
    ThisDies,
    ThisDiesOrIsExiled,
    ThisLeavesBattlefield,
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
    BecomesTargetedBySourceController {
        target: ObjectFilter,
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
    ThisDealsCombatDamageToPlayer,
    DealsDamage {
        filter: ObjectFilter,
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
    AbilityActivatedQualified {
        activator: PlayerFilter,
        filter: ObjectFilter,
        non_mana_only: bool,
    },
    IsDealtDamage {
        target: ChooseSpec,
    },
    YouGainLife,
    YouGainLifeDuringTurn {
        during_turn: PlayerFilter,
    },
    PlayerLosesLife {
        player: PlayerFilter,
    },
    PlayerLosesLifeDuringTurn {
        player: PlayerFilter,
        during_turn: PlayerFilter,
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
    PlayerDiscardsCardCausedByController {
        player: PlayerFilter,
        filter: Option<ObjectFilter>,
        controller: PlayerFilter,
        effect_like_only: bool,
    },
    PlayerDiscardsCard {
        player: PlayerFilter,
        filter: Option<ObjectFilter>,
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
    Dies {
        filter: ObjectFilter,
    },
    PutIntoGraveyard {
        filter: ObjectFilter,
    },
    DiesCreatureDealtDamageByThisTurn {
        victim: ObjectFilter,
        damager: DamagedBySource,
    },
    SpellCastQualified {
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    },
    SpellCast {
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
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
    },
    BeginningOfPrecombatMainPhase {
        player: PlayerFilter,
    },
    BeginningOfPostcombatMainPhase {
        player: PlayerFilter,
    },
    ThisEntersBattlefield,
    YouCastThisSpell,
    KeywordActionMatchingObject {
        action: KeywordActionKind,
        player: PlayerFilter,
        filter: ObjectFilter,
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
    },
    Expend {
        amount: u32,
        player: PlayerFilter,
    },
    SagaChapter {
        chapters: Vec<u32>,
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
    CounterPutOn(CounterPutOnTrigger),
    CounterRemovedFrom(CounterRemovedFromTrigger),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    pub label: String,
    pub kind: TriggerKind,
}

impl Trigger {
    pub fn new<T: CompilerTriggerMatcher>(matcher: T) -> Self {
        matcher.into_trigger()
    }

    pub fn state_based(display: impl Into<String>) -> Self {
        let display = display.into();
        Self::typed(display.clone(), TriggerKind::StateBased { display })
    }

    fn typed(label: impl Into<String>, kind: TriggerKind) -> Self {
        Self {
            label: label.into(),
            kind,
        }
    }

    pub fn this_attacks() -> Self {
        Self::typed("this_attacks", TriggerKind::ThisAttacks)
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
            TriggerKind::ThisBlocksObject { filter },
        )
    }
    pub fn blocks(filter: ObjectFilter) -> Self {
        Self::typed("blocks", TriggerKind::Blocks { filter })
    }
    pub fn this_becomes_blocked() -> Self {
        Self::typed("this_becomes_blocked", TriggerKind::ThisBecomesBlocked)
    }
    pub fn this_dies() -> Self {
        Self::typed("this_dies", TriggerKind::ThisDies)
    }
    pub fn this_dies_or_is_exiled() -> Self {
        Self::typed("this_dies_or_is_exiled", TriggerKind::ThisDiesOrIsExiled)
    }
    pub fn this_leaves_battlefield() -> Self {
        Self::typed(
            "this_leaves_battlefield",
            TriggerKind::ThisLeavesBattlefield,
        )
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
    pub fn becomes_targeted_by_source_controller(
        target: ObjectFilter,
        controller: PlayerFilter,
    ) -> Self {
        Self::typed(
            "becomes_targeted_by_source_controller",
            TriggerKind::BecomesTargetedBySourceController { target, controller },
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
    pub fn this_deals_combat_damage_to_player() -> Self {
        Self::typed(
            "this_deals_combat_damage_to_player",
            TriggerKind::ThisDealsCombatDamageToPlayer,
        )
    }
    pub fn deals_damage(filter: ObjectFilter) -> Self {
        Self::typed("deals_damage", TriggerKind::DealsDamage { filter })
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
    pub fn ability_activated_qualified(
        activator: PlayerFilter,
        filter: ObjectFilter,
        non_mana_only: bool,
    ) -> Self {
        Self::typed(
            "ability_activated_qualified",
            TriggerKind::AbilityActivatedQualified {
                activator,
                filter,
                non_mana_only,
            },
        )
    }
    pub fn is_dealt_damage(target: ChooseSpec) -> Self {
        Self::typed("is_dealt_damage", TriggerKind::IsDealtDamage { target })
    }
    pub fn you_gain_life() -> Self {
        Self::typed("you_gain_life", TriggerKind::YouGainLife)
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
    pub fn player_loses_life_during_turn(player: PlayerFilter, during_turn: PlayerFilter) -> Self {
        Self::typed(
            "player_loses_life_during_turn",
            TriggerKind::PlayerLosesLifeDuringTurn {
                player,
                during_turn,
            },
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
            TriggerKind::PlayerDiscardsCard { player, filter },
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
        Self::typed(
            "player_sacrifices",
            TriggerKind::PlayerSacrifices { player, filter },
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
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        exact_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    ) -> Self {
        Self::typed(
            "spell_cast_qualified",
            TriggerKind::SpellCastQualified {
                filter,
                caster,
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
            TriggerKind::BeginningOfEndStep { player },
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
    pub fn this_enters_battlefield() -> Self {
        Self::typed(
            "this_enters_battlefield",
            TriggerKind::ThisEntersBattlefield,
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
        Self::typed("wins_clash", TriggerKind::WinsClash { player })
    }
    pub fn expend(amount: u32, player: PlayerFilter) -> Self {
        Self::typed("expend", TriggerKind::Expend { amount, player })
    }
    pub fn saga_chapter(chapters: Vec<u32>) -> Self {
        Self::typed("saga_chapter", TriggerKind::SagaChapter { chapters })
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
        self.label.clone()
    }
}

pub trait CompilerTriggerMatcher {
    fn into_trigger(self) -> Trigger;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneChangeTrigger {
    pub from: Option<Zone>,
    pub to: Option<Zone>,
    pub filter: Option<ObjectFilter>,
    pub this: bool,
    pub this_surface: Option<SourceReferenceSurface>,
    pub count: CountMode,
    pub cause_filter: Option<CauseFilter>,
}

impl ZoneChangeTrigger {
    pub fn new() -> Self {
        Self {
            from: None,
            to: None,
            filter: None,
            this: false,
            this_surface: None,
            count: CountMode::One,
            cause_filter: None,
        }
    }

    pub fn count(mut self, mode: CountMode) -> Self {
        self.count = mode;
        self
    }

    pub fn from(mut self, zone: Zone) -> Self {
        self.from = Some(zone);
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

    pub fn cause_filter(mut self, filter: Option<CauseFilter>) -> Self {
        self.cause_filter = filter;
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
pub struct CounterPutOnTrigger {
    pub filter: ObjectFilter,
    pub counter_type: Option<CounterType>,
    pub source_controller: Option<PlayerFilter>,
    pub count: CountMode,
}

impl CounterPutOnTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter_type: None,
            source_controller: None,
            count: CountMode::One,
        }
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
}

impl CounterRemovedFromTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
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
