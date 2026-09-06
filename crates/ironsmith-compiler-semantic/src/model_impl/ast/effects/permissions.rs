//! The permissions actions of `EffectAst`.

use super::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum PermissionEffectAst {
    MayCastMatchingSpellWithoutPayingManaCost {
        player: PlayerAst,
        zone_owner: PlayerAst,
        filter: ObjectFilter,
        zone: Zone,
        payment: ironsmith_core::MayCastMatchingSpellPayment,
    },
    May {
        effects: Vec<EffectAst>,
    },
    MayByPlayer {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    /// Offer each matching player, beginning with the effect controller and
    /// proceeding in turn order, the option to perform `effects`. Stop after
    /// one accepts.
    AnyPlayerMay {
        players: PlayerFilter,
        effects: Vec<EffectAst>,
    },
}
