//! Game rule restriction abilities.
//!
//! These abilities modify game rules like preventing life gain,
//! preventing searching, etc.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::effect::Restriction;
use crate::game_state::{CantEffectTracker, GameState};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::{ObjectFilter, PlayerFilter};

/// "Players can't gain life"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayersCantGainLife;

impl StaticAbilityKind for PlayersCantGainLife {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PlayersCantGainLife
    }

    fn display(&self) -> String {
        "Players can't gain life".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::gain_life(PlayerFilter::Any).apply(game, &mut tracker, _controller, None);
        game.cant_effects.merge(tracker);
    }
}

/// "Players can't search libraries"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayersCantSearch;

impl StaticAbilityKind for PlayersCantSearch {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PlayersCantSearch
    }

    fn display(&self) -> String {
        "Players can't search libraries".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::search_libraries(PlayerFilter::Any).apply(
            game,
            &mut tracker,
            _controller,
            None,
        );
        game.cant_effects.merge(tracker);
    }
}

/// "Damage can't be prevented"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DamageCantBePrevented;

impl StaticAbilityKind for DamageCantBePrevented {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DamageCantBePrevented
    }

    fn display(&self) -> String {
        "Damage can't be prevented".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::prevent_damage().apply(game, &mut tracker, _controller, None);
        game.cant_effects.merge(tracker);
    }
}

/// "You can't lose the game" (Platinum Angel)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct YouCantLoseGame;

impl StaticAbilityKind for YouCantLoseGame {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::YouCantLoseGame
    }

    fn display(&self) -> String {
        "You can't lose the game".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::lose_game(PlayerFilter::You).apply(game, &mut tracker, controller, None);
        game.cant_effects.merge(tracker);
    }
}

/// "Your opponents can't win the game" (Platinum Angel)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpponentsCantWinGame;

impl StaticAbilityKind for OpponentsCantWinGame {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::OpponentsCantWinGame
    }

    fn display(&self) -> String {
        "Your opponents can't win the game".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::win_game(PlayerFilter::Opponent).apply(game, &mut tracker, controller, None);
        game.cant_effects.merge(tracker);
    }
}

/// "Your life total can't change" (Platinum Emperion)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct YourLifeTotalCantChange;

impl StaticAbilityKind for YourLifeTotalCantChange {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::YourLifeTotalCantChange
    }

    fn display(&self) -> String {
        "Your life total can't change".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::change_life_total(PlayerFilter::You).apply(
            game,
            &mut tracker,
            controller,
            None,
        );
        game.cant_effects.merge(tracker);
    }
}

/// "Permanents you control can't be sacrificed"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PermanentsCantBeSacrificed;

impl StaticAbilityKind for PermanentsCantBeSacrificed {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PermanentsCantBeSacrificed
    }

    fn display(&self) -> String {
        "Permanents you control can't be sacrificed".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::be_sacrificed(ObjectFilter::permanent().you_control()).apply(
            game,
            &mut tracker,
            controller,
            None,
        );
        game.cant_effects.merge(tracker);
    }
}

/// "Your opponents can't cast spells"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpponentsCantCastSpells;

impl StaticAbilityKind for OpponentsCantCastSpells {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::OpponentsCantCastSpells
    }

    fn display(&self) -> String {
        "Your opponents can't cast spells".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::cast_spells(PlayerFilter::Opponent).apply(
            game,
            &mut tracker,
            controller,
            None,
        );
        game.cant_effects.merge(tracker);
    }
}

/// "Split second" (while this spell is on the stack).
///
/// As long as this spell is on the stack, players can't cast spells or activate abilities
/// that aren't mana abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SplitSecond;

impl StaticAbilityKind for SplitSecond {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SplitSecond
    }

    fn display(&self) -> String {
        "Split second".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::cast_spells(PlayerFilter::Any).apply(game, &mut tracker, controller, None);
        Restriction::activate_non_mana_abilities(PlayerFilter::Any).apply(
            game,
            &mut tracker,
            controller,
            None,
        );
        game.cant_effects.merge(tracker);
    }
}

/// "Rebound" spell keyword.
///
/// Runtime handling is performed during spell resolution in `game_loop.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rebound;

impl StaticAbilityKind for Rebound {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Rebound
    }

    fn display(&self) -> String {
        "Rebound".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }
}

/// "Cascade" spell keyword.
///
/// Runtime handling is performed as a synthetic cast trigger in `triggers/check.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cascade;

impl StaticAbilityKind for Cascade {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Cascade
    }

    fn display(&self) -> String {
        "Cascade".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }
}

/// "Unleash" static restriction.
///
/// A creature with unleash can't block as long as it has a +1/+1 counter on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Unleash;

impl StaticAbilityKind for Unleash {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Unleash
    }

    fn display(&self) -> String {
        "This creature can't block as long as it has a +1/+1 counter on it".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::block(
            ObjectFilter::specific(source).with_counter_type(CounterType::PlusOnePlusOne),
        )
        .apply(game, &mut tracker, controller, Some(source));
        game.cant_effects.merge(tracker);
    }
}

/// "Your opponents can't draw more than one card each turn"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpponentsCantDrawExtraCards;

impl StaticAbilityKind for OpponentsCantDrawExtraCards {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::OpponentsCantDrawExtraCards
    }

    fn display(&self) -> String {
        "Your opponents can't draw more than one card each turn".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::draw_extra_cards(PlayerFilter::Opponent).apply(
            game,
            &mut tracker,
            controller,
            None,
        );
        game.cant_effects.merge(tracker);
    }
}

/// "Counters can't be put on this permanent"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CantHaveCountersPlaced;

impl StaticAbilityKind for CantHaveCountersPlaced {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantHaveCountersPlaced
    }

    fn display(&self) -> String {
        "Counters can't be put on this permanent".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::have_counters_placed(ObjectFilter::specific(source)).apply(
            game,
            &mut tracker,
            _controller,
            Some(source),
        );
        game.cant_effects.merge(tracker);
    }
}

/// "This spell can't be countered"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CantBeCountered;

impl StaticAbilityKind for CantBeCountered {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantBeCountered
    }

    fn display(&self) -> String {
        "This spell can't be countered".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn cant_be_countered(&self) -> bool {
        true
    }
}

/// Generic static restriction ability with custom display text.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleRestriction {
    pub restriction: Restriction,
    pub display: String,
}

impl RuleRestriction {
    pub fn new(restriction: Restriction, display: String) -> Self {
        Self {
            restriction,
            display,
        }
    }
}

impl StaticAbilityKind for RuleRestriction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RuleRestriction
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        self.restriction
            .apply(game, &mut tracker, controller, Some(source));
        game.cant_effects.merge(tracker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_players_cant_gain_life() {
        let ability = PlayersCantGainLife;
        assert_eq!(ability.id(), StaticAbilityId::PlayersCantGainLife);
        assert_eq!(ability.display(), "Players can't gain life");
    }

    #[test]
    fn test_you_cant_lose_game() {
        let ability = YouCantLoseGame;
        assert_eq!(ability.id(), StaticAbilityId::YouCantLoseGame);
    }

    #[test]
    fn test_cant_be_countered() {
        let ability = CantBeCountered;
        assert!(ability.cant_be_countered());
    }
}
