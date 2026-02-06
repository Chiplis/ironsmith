//! Simple keyword abilities.
//!
//! These are keyword abilities that don't have parameters and don't generate
//! continuous effects. They're just flags that are checked when relevant.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::effect::Restriction;
use crate::game_state::{CantEffectTracker, GameState};
use crate::ids::{ObjectId, PlayerId};
use crate::target::ObjectFilter;

/// Macro to define simple keyword abilities.
///
/// Creates a unit struct that implements StaticAbilityKind with the given
/// ID, display name, and optional query method overrides.
macro_rules! define_keyword {
    ($name:ident, $id:ident, $display:expr $(, $method:ident => $value:expr)*) => {
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

            fn is_keyword(&self) -> bool {
                true
            }

            $(
                fn $method(&self) -> bool {
                    $value
                }
            )*
        }
    };
}

// === Evasion keywords ===

define_keyword!(Flying, Flying, "Flying",
    has_flying => true,
    grants_evasion => true
);

define_keyword!(Shadow, Shadow, "Shadow",
    grants_evasion => true
);

define_keyword!(Horsemanship, Horsemanship, "Horsemanship",
    grants_evasion => true
);

define_keyword!(Fear, Fear, "Fear",
    grants_evasion => true
);

define_keyword!(Intimidate, Intimidate, "Intimidate",
    grants_evasion => true
);

// === Combat keywords ===

define_keyword!(FirstStrike, FirstStrike, "First strike",
    has_first_strike => true
);

define_keyword!(DoubleStrike, DoubleStrike, "Double strike",
    has_first_strike => true,
    has_double_strike => true
);

define_keyword!(Deathtouch, Deathtouch, "Deathtouch",
    has_deathtouch => true
);

define_keyword!(Lifelink, Lifelink, "Lifelink",
    has_lifelink => true
);

define_keyword!(Trample, Trample, "Trample",
    has_trample => true
);

define_keyword!(Vigilance, Vigilance, "Vigilance",
    has_vigilance => true
);

define_keyword!(Menace, Menace, "Menace",
    has_menace => true
);

define_keyword!(Reach, Reach, "Reach",
    has_reach => true
);

define_keyword!(Flanking, Flanking, "Flanking");

// === Defensive keywords ===

/// Defender - This creature can't attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Defender;

impl StaticAbilityKind for Defender {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Defender
    }

    fn display(&self) -> String {
        "Defender".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn has_defender(&self) -> bool {
        true
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

/// Indestructible - This permanent can't be destroyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Indestructible;

impl StaticAbilityKind for Indestructible {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Indestructible
    }

    fn display(&self) -> String {
        "Indestructible".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn has_indestructible(&self) -> bool {
        true
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::be_destroyed(ObjectFilter::specific(source)).apply(
            game,
            &mut tracker,
            _controller,
            Some(source),
        );
        game.cant_effects.merge(tracker);
    }
}

/// Hexproof - Can't be the target of spells or abilities opponents control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hexproof;

impl StaticAbilityKind for Hexproof {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Hexproof
    }

    fn display(&self) -> String {
        "Hexproof".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn has_hexproof(&self) -> bool {
        true
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::be_targeted(ObjectFilter::specific(source)).apply(
            game,
            &mut tracker,
            _controller,
            Some(source),
        );
        game.cant_effects.merge(tracker);
    }
}

/// Shroud - Can't be the target of spells or abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Shroud;

impl StaticAbilityKind for Shroud {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Shroud
    }

    fn display(&self) -> String {
        "Shroud".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn has_shroud(&self) -> bool {
        true
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        let mut tracker = CantEffectTracker::default();
        Restriction::be_targeted(ObjectFilter::specific(source)).apply(
            game,
            &mut tracker,
            _controller,
            Some(source),
        );
        game.cant_effects.merge(tracker);
    }
}

// === Timing keywords ===

define_keyword!(Flash, Flash, "Flash",
    has_flash => true
);

define_keyword!(Haste, Haste, "Haste",
    has_haste => true
);

define_keyword!(Phasing, Phasing, "Phasing");

// === Damage modification keywords ===

define_keyword!(Wither, Wither, "Wither");

define_keyword!(Infect, Infect, "Infect");

// === Type-granting keywords ===

/// Changeling - This creature is every creature type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Changeling;

impl StaticAbilityKind for Changeling {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Changeling
    }

    fn display(&self) -> String {
        "Changeling".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn is_changeling(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flying() {
        let flying = Flying;
        assert_eq!(flying.id(), StaticAbilityId::Flying);
        assert_eq!(flying.display(), "Flying");
        assert!(flying.is_keyword());
        assert!(flying.has_flying());
        assert!(flying.grants_evasion());
        assert!(!flying.has_trample());
    }

    #[test]
    fn test_first_strike() {
        let first_strike = FirstStrike;
        assert_eq!(first_strike.id(), StaticAbilityId::FirstStrike);
        assert!(first_strike.has_first_strike());
        assert!(!first_strike.has_double_strike());
    }

    #[test]
    fn test_double_strike() {
        let double_strike = DoubleStrike;
        assert_eq!(double_strike.id(), StaticAbilityId::DoubleStrike);
        assert!(double_strike.has_first_strike());
        assert!(double_strike.has_double_strike());
    }

    #[test]
    fn test_changeling() {
        let changeling = Changeling;
        assert_eq!(changeling.id(), StaticAbilityId::Changeling);
        assert!(changeling.is_changeling());
        assert!(changeling.is_keyword());
    }

    #[test]
    fn test_clone_box() {
        let flying = Flying;
        let cloned = flying.clone_box();
        assert_eq!(cloned.id(), StaticAbilityId::Flying);
    }
}
