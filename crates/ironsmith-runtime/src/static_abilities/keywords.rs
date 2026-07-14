//! Simple keyword abilities.
//!
//! These are keyword abilities that don't have parameters and don't generate
//! continuous effects. They're just flags that are checked when relevant.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::continuous::{ContinuousEffect, EffectSourceType, EffectTarget, Modification};
use crate::effect::Restriction;
use crate::effect::RestrictionExt as _;
use crate::game_state::{CantEffectTracker, GameState};
use crate::ids::{ObjectId, PlayerId};
use crate::target::ObjectFilter;
use crate::types::{CardType, SubtypeFamily};

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

define_keyword!(Skulk, Skulk, "Skulk",
    grants_evasion => true
);

define_keyword!(Prowess, Prowess, "Prowess");

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

define_keyword!(Banding, Banding, "Banding");

define_keyword!(Reach, Reach, "Reach",
    has_reach => true
);

define_keyword!(Flanking, Flanking, "Flanking");
define_keyword!(Partner, Partner, "Partner");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartnerVariant {
    display: String,
}

impl PartnerVariant {
    pub fn new(display: impl AsRef<str>) -> Self {
        Self {
            display: display.as_ref().trim().to_string(),
        }
    }
}

impl StaticAbilityKind for PartnerVariant {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Partner
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn is_keyword(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartnerWith {
    display: String,
}

impl PartnerWith {
    pub fn new(partner_name: impl AsRef<str>) -> Self {
        let partner_name = partner_name.as_ref().trim();
        let lower = partner_name.to_ascii_lowercase();
        let partner_name = if lower.starts_with("partner with ") {
            partner_name["partner with ".len()..].trim()
        } else {
            partner_name
        };
        Self {
            display: format!("Partner with {partner_name}"),
        }
    }
}

impl StaticAbilityKind for PartnerWith {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PartnerWith
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn is_keyword(&self) -> bool {
        true
    }
}

define_keyword!(StartYourEngines, StartYourEngines, "Start your engines!");
define_keyword!(DoctorsCompanion, DoctorsCompanion, "Doctor's companion");
define_keyword!(Assist, Assist, "Assist");
define_keyword!(ReadAhead, ReadAhead, "Read ahead");

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
            None,
        );
        game.effect_store.cant_effects.merge(tracker);
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
            None,
        );
        game.effect_store.cant_effects.merge(tracker);
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
            None,
        );
        game.effect_store.cant_effects.merge(tracker);
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
            None,
        );
        game.effect_store.cant_effects.merge(tracker);
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

    fn is_keyword(&self) -> bool {
        true
    }

    fn is_changeling(&self) -> bool {
        true
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::AddAllSubtypesOfFamily(SubtypeFamily::Creature),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Living metal (CR 702.161a) makes its source an artifact creature during its
/// controller's turn, in addition to its other types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LivingMetal;

impl StaticAbilityKind for LivingMetal {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LivingMetal
    }

    fn display(&self) -> String {
        "Living metal".to_string()
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        if game.turn.active_player != controller {
            return vec![];
        }

        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::AddCardTypes(vec![CardType::Artifact, CardType::Creature]),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn living_metal_adds_artifact_creature_types_only_during_controllers_turn() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        game.turn.active_player = alice;
        let own_turn = LivingMetal.generate_effects(source, alice, &game);
        assert!(matches!(
            own_turn.as_slice(),
            [effect]
                if matches!(
                    &effect.modification,
                    Modification::AddCardTypes(types)
                        if types == &[CardType::Artifact, CardType::Creature]
                )
        ));

        game.turn.active_player = bob;
        assert!(
            LivingMetal
                .generate_effects(source, alice, &game)
                .is_empty()
        );
        assert_eq!(LivingMetal.generate_effects(source, bob, &game).len(), 1);
    }
}
