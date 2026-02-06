//! Combat-related static abilities.
//!
//! These abilities modify combat rules like blocking restrictions,
//! attack requirements, etc.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::effect::Restriction;
use crate::game_state::{CantEffectTracker, GameState};
use crate::ids::{ObjectId, PlayerId};
use crate::target::ObjectFilter;

/// Macro to define simple combat abilities.
macro_rules! define_combat_ability {
    ($name:ident, $id:ident, $display:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $name;

        impl StaticAbilityKind for $name {
            fn id(&self) -> StaticAbilityId {
                StaticAbilityId::$id
            }

            fn display(&self) -> String {
                $display.to_string()
            }

            fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
                Box::new(*self)
            }
        }
    };
}

/// Can't be blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Unblockable;

impl StaticAbilityKind for Unblockable {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Unblockable
    }

    fn display(&self) -> String {
        "Can't be blocked".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_unblockable(&self) -> bool {
        true
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::be_blocked(ObjectFilter::specific(source)).apply(
            game,
            &mut tracker,
            _controller,
            Some(source),
        );
        game.cant_effects.merge(tracker);
    }
}

/// Can't be blocked except by creatures with flying or reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlyingRestriction;

impl StaticAbilityKind for FlyingRestriction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::FlyingRestriction
    }

    fn display(&self) -> String {
        "Can't be blocked except by creatures with flying or reach".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn grants_evasion(&self) -> bool {
        true
    }
}

/// Can block creatures with flying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CanBlockFlying;

impl StaticAbilityKind for CanBlockFlying {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CanBlockFlying
    }

    fn display(&self) -> String {
        "Can block creatures with flying".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

// Can attack as though it didn't have defender.
define_combat_ability!(
    CanAttackAsThoughNoDefender,
    CanAttackAsThoughNoDefender,
    "Can attack as though it didn't have defender"
);

/// Must attack each combat if able.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MustAttack;

impl StaticAbilityKind for MustAttack {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MustAttack
    }

    fn display(&self) -> String {
        "Attacks each combat if able".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    // Note: Must attack checking is done in the combat rules engine
    // by checking if creatures have this ability, rather than using a tracker.
}

/// Must block if able.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MustBlock;

impl StaticAbilityKind for MustBlock {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MustBlock
    }

    fn display(&self) -> String {
        "Blocks each combat if able".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    // Note: Must block checking is done in the combat rules engine
    // by checking if creatures have this ability, rather than using a tracker.
}

/// Can't attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CantAttack;

impl StaticAbilityKind for CantAttack {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantAttack
    }

    fn display(&self) -> String {
        "Can't attack".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::attack(ObjectFilter::specific(source)).apply(
            game,
            &mut tracker,
            _controller,
            Some(source),
        );
        game.cant_effects.merge(tracker);
    }
}

/// Can't block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CantBlock;

impl StaticAbilityKind for CantBlock {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantBlock
    }

    fn display(&self) -> String {
        "Can't block".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::block(ObjectFilter::specific(source)).apply(
            game,
            &mut tracker,
            _controller,
            Some(source),
        );
        game.cant_effects.merge(tracker);
    }
}

// May assign combat damage as though it weren't blocked (Thorn Elemental).
define_combat_ability!(
    MayAssignDamageAsUnblocked,
    MayAssignDamageAsUnblocked,
    "May assign combat damage as though it weren't blocked"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unblockable() {
        let unblockable = Unblockable;
        assert_eq!(unblockable.id(), StaticAbilityId::Unblockable);
        assert!(unblockable.is_unblockable());
    }

    #[test]
    fn test_cant_attack() {
        let cant_attack = CantAttack;
        assert_eq!(cant_attack.id(), StaticAbilityId::CantAttack);
        assert_eq!(cant_attack.display(), "Can't attack");
    }

    #[test]
    fn test_must_attack() {
        let must_attack = MustAttack;
        assert_eq!(must_attack.id(), StaticAbilityId::MustAttack);
        assert_eq!(must_attack.display(), "Attacks each combat if able");
    }
}
