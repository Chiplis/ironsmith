use crate::cards::builders::TagKey;
use crate::filter::ParityRequirement;
use crate::target::PlayerFilter;

use super::super::primitives;
use super::UnspentManaRetentionTail;

#[derive(Debug, Clone, PartialEq)]
pub enum GlobalCantRestrictionFact {
    PlayersLoseOrWin,
    OpponentsBlockManaValueParity(ParityRequirement),
    GainLife(PlayerFilter),
    SearchLibraries(PlayerFilter),
    DrawCards(PlayerFilter),
    DrawExtraCards(PlayerFilter),
    PreventDamage,
    LoseGame(PlayerFilter),
    WinGame(PlayerFilter),
    ChangeLifeTotal(PlayerFilter),
    BecomeMonarch(PlayerFilter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrWinGameTail;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerRestrictionTailKind {
    GainLife,
    SearchLibraries,
    LoseGame,
    LoseLife,
    WinGame,
    DrawCards,
    DrawExtraCards,
    PoisonCounters,
    CastMoreThanOneSpellEachTurn,
    CastSpells,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageLifeLossSubject {
    You,
    AnyPlayer,
    IteratedPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleObjectRestrictionKind {
    Attack,
    AttackAlone,
    AttackOrBlock,
    AttackOrBlockAlone,
    Block,
    BlockAlone,
    BeBlocked,
    BeDestroyed,
    BeRegenerated,
    BeSacrificed,
    BeCountered,
    Transform,
    PhaseOut,
    PhaseIn,
    BeTargeted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestrictionSubjectSurface {
    Damage,
    Source,
    TaggedObjectPronoun,
    Player,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerOrToughnessSubject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DealtDamageThisWay;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DealtDamageBySourceSubject {
    pub base_word_count: usize,
    pub damager: ironsmith_core::DamagedBySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaRetentionTailKind {
    Unspent(UnspentManaRetentionTail),
    ThisMana,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaRetentionNegatedClause {
    pub tail: ManaRetentionTailKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CantCastSubject {
    pub player: PlayerFilter,
    pub consumed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectActionRestrictionTail;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeadingIfRestrictionSubject;

pub fn parse_global_cant_restriction_words(words: &[&str]) -> Option<GlobalCantRestrictionFact> {
    if prefix(
        words,
        &[
            "players", "cant", "lose", "the", "game", "or", "win", "the", "game",
        ],
    ) {
        return Some(GlobalCantRestrictionFact::PlayersLoseOrWin);
    }
    if prefix(
        words,
        &[
            "your",
            "opponents",
            "cant",
            "block",
            "with",
            "creatures",
            "with",
        ],
    ) && suffix(words, &["mana", "values"])
    {
        let parity = match words.get(7).copied()? {
            "odd" => ParityRequirement::Odd,
            "even" => ParityRequirement::Even,
            _ => return None,
        };
        return Some(GlobalCantRestrictionFact::OpponentsBlockManaValueParity(
            parity,
        ));
    }

    let fact = if exact(words, &["players", "cant", "gain", "life"]) {
        GlobalCantRestrictionFact::GainLife(PlayerFilter::Any)
    } else if exact(words, &["players", "cant", "search", "libraries"]) {
        GlobalCantRestrictionFact::SearchLibraries(PlayerFilter::Any)
    } else if exact(words, &["players", "cant", "draw", "cards"]) {
        GlobalCantRestrictionFact::DrawCards(PlayerFilter::Any)
    } else if exact(
        words,
        &[
            "players", "cant", "draw", "more", "than", "one", "card", "each", "turn",
        ],
    ) {
        GlobalCantRestrictionFact::DrawExtraCards(PlayerFilter::Any)
    } else if exact(words, &["damage", "cant", "be", "prevented"]) {
        GlobalCantRestrictionFact::PreventDamage
    } else if exact(words, &["you", "cant", "lose", "the", "game"]) {
        GlobalCantRestrictionFact::LoseGame(PlayerFilter::You)
    } else if exact(words, &["your", "opponents", "cant", "win", "the", "game"]) {
        GlobalCantRestrictionFact::WinGame(PlayerFilter::Opponent)
    } else if exact(words, &["your", "life", "total", "cant", "change"]) {
        GlobalCantRestrictionFact::ChangeLifeTotal(PlayerFilter::You)
    } else if exact_any(
        words,
        &[
            &[
                "your",
                "opponents",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ],
            &[
                "each", "opponent", "cant", "draw", "more", "than", "one", "card", "each", "turn",
            ],
        ],
    ) {
        GlobalCantRestrictionFact::DrawExtraCards(PlayerFilter::Opponent)
    } else if exact(words, &["you", "cant", "gain", "life"]) {
        GlobalCantRestrictionFact::GainLife(PlayerFilter::You)
    } else if exact(words, &["you", "cant", "search", "libraries"]) {
        GlobalCantRestrictionFact::SearchLibraries(PlayerFilter::You)
    } else if exact(words, &["you", "cant", "draw", "cards"]) {
        GlobalCantRestrictionFact::DrawCards(PlayerFilter::You)
    } else if exact_any(
        words,
        &[
            &["you", "cant", "become", "the", "monarch"],
            &["you", "cant", "become", "monarch"],
            &["you", "cant", "become", "the", "monarch", "this", "turn"],
            &["you", "cant", "become", "monarch", "this", "turn"],
        ],
    ) {
        GlobalCantRestrictionFact::BecomeMonarch(PlayerFilter::You)
    } else if exact_any(
        words,
        &[
            &["they", "cant", "gain", "life"],
            &["that", "player", "cant", "gain", "life"],
        ],
    ) {
        GlobalCantRestrictionFact::GainLife(PlayerFilter::IteratedPlayer)
    } else if exact(words, &["opponents", "cant", "gain", "life"]) {
        GlobalCantRestrictionFact::GainLife(PlayerFilter::Opponent)
    } else {
        return None;
    };
    Some(fact)
}

pub fn parse_or_win_game_tail_words(words: &[&str]) -> Option<OrWinGameTail> {
    (contains(words, &["or", "win", "the", "game"])
        || contains(words, &["or", "win", "the", "game", "this"]))
    .then_some(OrWinGameTail)
}

pub fn parse_player_negated_subject_words(words: &[&str]) -> Option<PlayerFilter> {
    if exact(words, &["you"]) {
        Some(PlayerFilter::You)
    } else if exact_any(words, &[&["your", "opponents"], &["opponents"]]) {
        Some(PlayerFilter::Opponent)
    } else if exact_any(words, &[&["players"], &["each", "player"]]) {
        Some(PlayerFilter::Any)
    } else if exact(words, &["enchanted", "player"]) {
        Some(PlayerFilter::TaggedPlayer(
            (crate::tag::CompilerReferenceTag::Enchanted.bind()).into(),
        ))
    } else {
        None
    }
}

pub fn parse_player_restriction_subject_words(words: &[&str]) -> Option<PlayerFilter> {
    if exact(words, &["you"]) {
        Some(PlayerFilter::You)
    } else if exact_any(words, &[&["that", "player"], &["they"]]) {
        Some(PlayerFilter::IteratedPlayer)
    } else if exact(words, &["players", "dealt", "damage", "this", "way"]) {
        Some(PlayerFilter::TaggedPlayer(
            (crate::tag::CompilerReferenceTag::Damaged0.bind()).into(),
        ))
    } else if exact_any(
        words,
        &[
            &["your", "opponents"],
            &["each", "opponent"],
            &["opponents"],
        ],
    ) {
        Some(PlayerFilter::Opponent)
    } else if exact_any(words, &[&["players"], &["each", "player"]]) {
        Some(PlayerFilter::Any)
    } else if exact(words, &["defending", "player"]) {
        Some(PlayerFilter::Defending)
    } else if exact(words, &["attacking", "player"]) {
        Some(PlayerFilter::Attacking)
    } else if exact_any(words, &[&["its", "controller"], &["their", "controller"]]) {
        Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged(crate::tag::CompilerReferenceTag::It.bind()),
        ))
    } else if exact_any(words, &[&["its", "owner"], &["their", "owner"]]) {
        Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
        )))
    } else {
        None
    }
}

pub fn parse_cant_cast_subject_words(words: &[&str]) -> Option<CantCastSubject> {
    let (player, consumed) = if prefix(words, &["players", "dealt", "damage", "this", "way"]) {
        (
            PlayerFilter::TaggedPlayer((crate::tag::CompilerReferenceTag::Damaged0.bind()).into()),
            5,
        )
    } else if prefix(words, &["that", "player"]) {
        (PlayerFilter::IteratedPlayer, 2)
    } else if prefix(words, &["your", "opponents", "who", "have"]) {
        (PlayerFilter::Opponent, 4)
    } else if prefix(words, &["each", "player", "who", "has"]) {
        (PlayerFilter::Any, 4)
    } else if prefix(words, &["each", "opponent", "who", "has"]) {
        (PlayerFilter::Opponent, 4)
    } else if prefix(words, &["your", "opponents"]) {
        (PlayerFilter::Opponent, 2)
    } else if prefix(words, &["each", "player"]) {
        (PlayerFilter::Any, 2)
    } else if prefix(words, &["each", "opponent"]) {
        (PlayerFilter::Opponent, 2)
    } else {
        match words.first().copied()? {
            "players" => (PlayerFilter::Any, 1),
            "opponents" => (PlayerFilter::Opponent, 1),
            "they" => (PlayerFilter::IteratedPlayer, 1),
            "you" => (PlayerFilter::You, 1),
            _ => return None,
        }
    };
    Some(CantCastSubject { player, consumed })
}

pub fn parse_player_restriction_tail_words(words: &[&str]) -> Option<PlayerRestrictionTailKind> {
    let kind = if prefix(words, &["gain", "life"]) {
        PlayerRestrictionTailKind::GainLife
    } else if prefix(words, &["search", "libraries"]) {
        PlayerRestrictionTailKind::SearchLibraries
    } else if prefix(words, &["lose", "the", "game"]) {
        PlayerRestrictionTailKind::LoseGame
    } else if prefix(words, &["lose", "life"]) {
        PlayerRestrictionTailKind::LoseLife
    } else if prefix(words, &["win", "the", "game"]) {
        PlayerRestrictionTailKind::WinGame
    } else if prefix(words, &["draw", "cards"]) {
        PlayerRestrictionTailKind::DrawCards
    } else if prefix(words, &["draw", "more", "than", "one", "card"]) {
        PlayerRestrictionTailKind::DrawExtraCards
    } else if prefix_any(
        words,
        &[
            &["get", "additional", "poison", "counters"],
            &["get", "poison", "counters"],
        ],
    ) {
        PlayerRestrictionTailKind::PoisonCounters
    } else if prefix(
        words,
        &["cast", "more", "than", "one", "spell", "each", "turn"],
    ) {
        PlayerRestrictionTailKind::CastMoreThanOneSpellEachTurn
    } else if exact_any(
        words,
        &[&["cast", "spells"], &["cast", "spells", "this", "turn"]],
    ) {
        PlayerRestrictionTailKind::CastSpells
    } else {
        return None;
    };
    Some(kind)
}

pub fn parse_damage_life_loss_tail_words(words: &[&str]) -> Option<DamageLifeLossSubject> {
    if prefix(words, &["cause", "you", "to", "lose", "life"]) {
        Some(DamageLifeLossSubject::You)
    } else if prefix_any(
        words,
        &[
            &["cause", "players", "to", "lose", "life"],
            &["cause", "each", "player", "to", "lose", "life"],
        ],
    ) {
        Some(DamageLifeLossSubject::AnyPlayer)
    } else if prefix(words, &["cause", "that", "player", "to", "lose", "life"]) {
        Some(DamageLifeLossSubject::IteratedPlayer)
    } else {
        None
    }
}

pub fn parse_simple_object_restriction_words(
    words: &[&str],
) -> Option<SimpleObjectRestrictionKind> {
    let kind = if exact_any(words, &[&["attack"], &["attack", "this", "turn"]]) {
        SimpleObjectRestrictionKind::Attack
    } else if exact_any(
        words,
        &[&["attack", "alone"], &["attack", "alone", "this", "turn"]],
    ) {
        SimpleObjectRestrictionKind::AttackAlone
    } else if exact_any(
        words,
        &[
            &["attack", "or", "block"],
            &["attack", "or", "block", "this", "turn"],
        ],
    ) {
        SimpleObjectRestrictionKind::AttackOrBlock
    } else if exact_any(
        words,
        &[
            &["attack", "or", "block", "alone"],
            &["attack", "or", "block", "alone", "this", "turn"],
        ],
    ) {
        SimpleObjectRestrictionKind::AttackOrBlockAlone
    } else if exact_any(words, &[&["block"], &["block", "this", "turn"]]) {
        SimpleObjectRestrictionKind::Block
    } else if exact_any(
        words,
        &[&["block", "alone"], &["block", "alone", "this", "turn"]],
    ) {
        SimpleObjectRestrictionKind::BlockAlone
    } else if exact_any(
        words,
        &[&["be", "blocked"], &["be", "blocked", "this", "turn"]],
    ) {
        SimpleObjectRestrictionKind::BeBlocked
    } else if exact(words, &["be", "destroyed"]) {
        SimpleObjectRestrictionKind::BeDestroyed
    } else if exact_any(
        words,
        &[
            &["be", "regenerated"],
            &["be", "regenerated", "this", "turn"],
        ],
    ) {
        SimpleObjectRestrictionKind::BeRegenerated
    } else if exact(words, &["be", "sacrificed"]) {
        SimpleObjectRestrictionKind::BeSacrificed
    } else if exact(words, &["be", "countered"]) {
        SimpleObjectRestrictionKind::BeCountered
    } else if exact(words, &["transform"]) {
        SimpleObjectRestrictionKind::Transform
    } else if exact_any(
        words,
        &[
            &["phase", "out"],
            &["phase", "out", "this", "turn"],
            &["phases", "out"],
        ],
    ) {
        SimpleObjectRestrictionKind::PhaseOut
    } else if exact_any(
        words,
        &[
            &["phase", "in"],
            &["phase", "in", "this", "turn"],
            &["phases", "in"],
        ],
    ) {
        SimpleObjectRestrictionKind::PhaseIn
    } else if exact(words, &["be", "targeted"]) {
        SimpleObjectRestrictionKind::BeTargeted
    } else {
        return None;
    };
    Some(kind)
}

pub fn parse_restriction_subject_surface_words(
    words: &[&str],
) -> Option<RestrictionSubjectSurface> {
    if exact_any(
        words,
        &[&["damage"], &["the", "damage"], &["that", "damage"]],
    ) {
        Some(RestrictionSubjectSurface::Damage)
    } else if exact(words, &["this"]) {
        Some(RestrictionSubjectSurface::Source)
    } else if exact_any(
        words,
        &[&["it"], &["they"], &["them"], &["itself"], &["themselves"]],
    ) {
        Some(RestrictionSubjectSurface::TaggedObjectPronoun)
    } else if exact_any(
        words,
        &[
            &["you"],
            &["your", "opponents"],
            &["opponents"],
            &["players"],
            &["each", "player"],
            &["enchanted", "player"],
        ],
    ) {
        Some(RestrictionSubjectSurface::Player)
    } else {
        None
    }
}

pub fn parse_power_or_toughness_subject_words(words: &[&str]) -> Option<PowerOrToughnessSubject> {
    (contains(words, &["power", "or", "toughness"])
        || contains(words, &["toughness", "or", "power"]))
    .then_some(PowerOrToughnessSubject)
}

pub fn parse_dealt_damage_this_way_words(words: &[&str]) -> Option<DealtDamageThisWay> {
    contains(words, &["dealt", "damage", "this", "way"]).then_some(DealtDamageThisWay)
}

#[path = "clause_facts/core.rs"]
mod core_programs;
pub(super) use core_programs::{
    contains, exact, exact_any, prefix, prefix_any, prefix_remainder, suffix, suffix_remainder,
};
#[path = "clause_facts/reference.rs"]
mod reference_programs;
pub use reference_programs::parse_leading_if_restriction_subject_words;
#[path = "clause_facts/condition.rs"]
mod condition_programs;
pub use condition_programs::parse_effect_action_restriction_tail_words;
#[path = "clause_facts/resource.rs"]
mod resource_programs;
pub use resource_programs::{
    parse_mana_retention_negated_clause_words, parse_mana_retention_tail_words,
};
#[path = "clause_facts/combat.rs"]
mod combat_programs;
pub use combat_programs::parse_dealt_damage_by_source_subject_words;
