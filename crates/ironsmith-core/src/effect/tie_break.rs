//! Repeated greatest-mana-value tie breaks among players (Timesifter: "The
//! player who exiled the card with the greatest mana value takes an extra
//! turn after this one. If two or more players' cards are tied for greatest,
//! the tied players repeat this process until the tie is broken.").
//!
//! The process is an ordinary `RepeatProcessEffect`: the contenders (players
//! tagged `players_tag`) each exile the top card of their library, then
//! `KeepGreatestManaValuePlayersEffect` keeps only the contenders whose card
//! has the greatest mana value; the loop continues while two or more remain.
use crate::PlayerFilter;
use crate::tag::{TagKey, TagKeyWalk};

/// Tag every in-game player matching `filter` under `tag`, replacing any
/// earlier players in that tag.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct TagPlayersEffect {
    pub filter: PlayerFilter,
    pub tag: TagKey,
}

impl TagPlayersEffect {
    pub fn new(filter: PlayerFilter, tag: impl Into<TagKey>) -> Self {
        Self {
            filter,
            tag: tag.into(),
        }
    }
}

/// Keep, among the players tagged `players_tag`, those whose card tagged
/// `objects_tag` has the greatest mana value. The outcome count is the number
/// kept (zero when none of them has such a card); the card tag is cleared so
/// the next round compares only that round's cards.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct KeepGreatestManaValuePlayersEffect {
    pub players_tag: TagKey,
    pub objects_tag: TagKey,
}

impl KeepGreatestManaValuePlayersEffect {
    pub fn new(players_tag: impl Into<TagKey>, objects_tag: impl Into<TagKey>) -> Self {
        Self {
            players_tag: players_tag.into(),
            objects_tag: objects_tag.into(),
        }
    }
}

/// "Each player rolls a d20. If one or more opponents had the highest
/// result, ..." (Chaos Dragon): every contender tagged `players_tag` rolls a
/// `sides`-sided die in turn order; the tag then keeps only the contenders
/// who rolled the highest result and also match `keep`. The outcome count
/// is the number kept.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct RollDieKeepHighestPlayersEffect {
    pub players_tag: TagKey,
    pub sides: u32,
    pub keep: PlayerFilter,
}

impl RollDieKeepHighestPlayersEffect {
    pub fn new(players_tag: impl Into<TagKey>, sides: u32, keep: PlayerFilter) -> Self {
        Self {
            players_tag: players_tag.into(),
            sides,
            keep,
        }
    }
}
