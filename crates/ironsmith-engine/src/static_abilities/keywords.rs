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

            fn compiled_model(&self) -> Option<&super::CompiledStaticAbility> {
                static MODEL: std::sync::LazyLock<super::CompiledStaticAbility> =
                    std::sync::LazyLock::new(|| super::CompiledStaticAbility {
                        id: Some(StaticAbilityId::$id),
                        label: $display.to_owned(),
                        payload: ironsmith_core::StaticAbilityPayload::None,
                    });
                Some(&MODEL)
            }

            fn may_generate_continuous_effects(&self) -> bool {
                false
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

// CR 702.19c: a variant of trample, not trample itself (`has_trample` stays
// false, so "creatures with trample" checks don't match it).
define_keyword!(
    TrampleOverPlaneswalkers,
    TrampleOverPlaneswalkers,
    "Trample over planeswalkers"
);

define_keyword!(Vigilance, Vigilance, "Vigilance",
    has_vigilance => true
);

define_keyword!(Menace, Menace, "Menace",
    has_menace => true
);

define_keyword!(Banding, Banding, "Banding");

#[derive(Debug, Clone, PartialEq)]
pub struct BandsWithOther {
    filter: ObjectFilter,
    display: String,
}

impl BandsWithOther {
    pub fn new(filter: ObjectFilter, display: impl Into<String>) -> Self {
        Self {
            filter,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for BandsWithOther {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::BandsWithOther
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn bands_with_other_filter(&self) -> Option<&ObjectFilter> {
        Some(&self.filter)
    }
}

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

/// Toxic N (CR 702.164a): a static ability. Combat damage dealt to a player
/// by a creature with toxic also gives that player poison counters equal to
/// the creature's total toxic value (CR 702.164c).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toxic {
    amount: u32,
}

impl Toxic {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }

    pub fn amount(&self) -> u32 {
        self.amount
    }
}

impl StaticAbilityKind for Toxic {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Toxic
    }

    fn display(&self) -> String {
        format!("Toxic {}", self.amount)
    }

    fn is_keyword(&self) -> bool {
        true
    }
}

define_keyword!(StartYourEngines, StartYourEngines, "Start your engines!");
define_keyword!(SpaceSculptor, SpaceSculptor, "Space sculptor");
define_keyword!(DoctorsCompanion, DoctorsCompanion, "Doctor's companion");
define_keyword!(Assist, Assist, "Assist");
define_keyword!(ReadAhead, ReadAhead, "Read ahead");

// === Defensive keywords ===

/// Defender - This creature can't attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Defender;

impl StaticAbilityKind for Defender {
    fn compiled_model(&self) -> Option<&super::CompiledStaticAbility> {
        static MODEL: std::sync::LazyLock<super::CompiledStaticAbility> =
            std::sync::LazyLock::new(|| super::CompiledStaticAbility {
                id: Some(StaticAbilityId::Defender),
                label: "Defender".to_owned(),
                payload: ironsmith_core::StaticAbilityPayload::None,
            });
        Some(&MODEL)
    }

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
        if game.current_has_static_ability_id(source, StaticAbilityId::CanAttackAsThoughNoDefender)
        {
            return;
        }
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
    fn compiled_model(&self) -> Option<&super::CompiledStaticAbility> {
        static MODEL: std::sync::LazyLock<super::CompiledStaticAbility> =
            std::sync::LazyLock::new(|| super::CompiledStaticAbility {
                id: Some(StaticAbilityId::Indestructible),
                label: "Indestructible".to_owned(),
                payload: ironsmith_core::StaticAbilityPayload::None,
            });
        Some(&MODEL)
    }

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
    fn compiled_model(&self) -> Option<&super::CompiledStaticAbility> {
        static MODEL: std::sync::LazyLock<super::CompiledStaticAbility> =
            std::sync::LazyLock::new(|| super::CompiledStaticAbility {
                id: Some(StaticAbilityId::Hexproof),
                label: "Hexproof".to_owned(),
                payload: ironsmith_core::StaticAbilityPayload::None,
            });
        Some(&MODEL)
    }

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
        Restriction::be_targeted_from(
            ObjectFilter::specific(source),
            ObjectFilter::default().controlled_by(crate::target::PlayerFilter::Opponent),
        )
        .apply(game, &mut tracker, _controller, Some(source), None);
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
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::AddCardTypes(vec![CardType::Artifact, CardType::Creature]),
            )
            .with_source_type(EffectSourceType::StaticAbility)
            .with_condition(crate::ConditionExpr::YourTurn),
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

        assert_eq!(own_turn[0].condition, Some(crate::ConditionExpr::YourTurn));
        assert!(
            crate::continuous::continuous_effect_duration_and_condition_are_active(
                &own_turn[0],
                &game
            )
        );
        game.turn.active_player = bob;
        let other_turn = LivingMetal.generate_effects(source, alice, &game);
        assert_eq!(
            other_turn.len(),
            1,
            "retain the conditional descriptor until layer application"
        );
        assert_eq!(
            other_turn[0].condition,
            Some(crate::ConditionExpr::YourTurn)
        );
        assert!(
            !crate::continuous::continuous_effect_duration_and_condition_are_active(
                &other_turn[0],
                &game
            )
        );
        let bob_turn = LivingMetal.generate_effects(source, bob, &game);
        assert_eq!(bob_turn.len(), 1);
        assert!(
            crate::continuous::continuous_effect_duration_and_condition_are_active(
                &bob_turn[0],
                &game
            )
        );
    }

    #[test]
    fn living_metal_rechecks_turn_condition_after_static_control() {
        use crate::ability::Ability;
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::static_abilities::StaticAbility;
        use crate::target::ObjectFilter;
        use crate::zone::Zone;
        for active in [PlayerId::from_index(0), PlayerId::from_index(1)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            game.turn.active_player = active;
            let vehicle = CardBuilder::new(CardId::new(), "Living metal control recipient")
                .card_types(vec![CardType::Artifact])
                .subtypes(vec![crate::types::Subtype::Vehicle])
                .build();
            let source = game.create_object_from_card(&vehicle, bob, Zone::Battlefield);
            game.object_mut(source)
                .unwrap()
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::living_metal()));
            let original = game
                .continuous_query_snapshot()
                .expect("original living metal query is finite");
            assert_eq!(
                original
                    .current_characteristics(source)
                    .unwrap()
                    .card_types
                    .contains(&CardType::Creature),
                active == bob,
                "positive control uses the original controller's turn"
            );
            let aura = CardBuilder::new(CardId::new(), "Living metal control source")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![crate::types::Subtype::Aura])
                .build();
            let control = game.create_object_from_card(&aura, alice, Zone::Battlefield);
            game.object_mut(control).unwrap().attached_to =
                Some(crate::object::AttachmentTarget::Object(source));
            game.object_mut(source).unwrap().attachments.push(control);
            game.object_mut(control).unwrap().abilities_mut().extend([
                Ability::static_ability(StaticAbility::enchant(
                    crate::object::AuraAttachmentFilter::Object(ObjectFilter::permanent()),
                )),
                Ability::static_ability(StaticAbility::control_attached_permanent(
                    "You control the enchanted permanent".into(),
                )),
            ]);
            let revision = game.effect_store.continuous_effects.revision();
            let query = game
                .continuous_query_snapshot()
                .expect("controlled living metal query is finite");
            assert_eq!(query.current_controller(source), Some(alice));
            assert_eq!(
                query
                    .current_characteristics(source)
                    .unwrap()
                    .card_types
                    .contains(&CardType::Creature),
                active == alice,
                "living metal must evaluate its condition after static source control"
            );
            assert!(
                query
                    .current_characteristics(source)
                    .unwrap()
                    .card_types
                    .contains(&CardType::Artifact)
            );
            assert_eq!(game.object(source).unwrap().owner, bob);
            assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        }
    }
}

#[cfg(test)]
mod retained_keyword_model_tests {
    use super::*;
    use crate::static_abilities::StaticAbility;
    use ironsmith_core::functional_zones::StaticAbilityFunctionalZones;

    #[test]
    fn retained_keyword_models_restore_every_declared_unit_keyword() {
        let abilities = vec![
            StaticAbility::new(Flying),
            StaticAbility::new(Shadow),
            StaticAbility::new(Horsemanship),
            StaticAbility::new(Fear),
            StaticAbility::new(Intimidate),
            StaticAbility::new(Skulk),
            StaticAbility::new(Prowess),
            StaticAbility::new(FirstStrike),
            StaticAbility::new(DoubleStrike),
            StaticAbility::new(Deathtouch),
            StaticAbility::new(Lifelink),
            StaticAbility::new(Trample),
            StaticAbility::new(TrampleOverPlaneswalkers),
            StaticAbility::new(Vigilance),
            StaticAbility::new(Menace),
            StaticAbility::new(Banding),
            StaticAbility::new(Reach),
            StaticAbility::new(Flanking),
            StaticAbility::new(Partner),
            StaticAbility::new(StartYourEngines),
            StaticAbility::new(SpaceSculptor),
            StaticAbility::new(DoctorsCompanion),
            StaticAbility::new(Assist),
            StaticAbility::new(ReadAhead),
            StaticAbility::new(Flash),
            StaticAbility::new(Haste),
            StaticAbility::new(Phasing),
            StaticAbility::new(Wither),
            StaticAbility::new(Infect),
        ];
        for original in abilities {
            let model = original
                .compiled_model()
                .expect("unit keyword has a canonical model");
            let restored = StaticAbility::from_model(model.clone());
            assert_eq!(restored.id(), original.id());
            assert_eq!(restored.display(), original.display());
            assert_eq!(
                restored.default_functional_zones(),
                original.default_functional_zones()
            );
            assert_eq!(
                restored.is_keyword(),
                original.is_keyword(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.may_generate_continuous_effects(),
                original.may_generate_continuous_effects()
            );
            assert_eq!(
                restored.grants_evasion(),
                original.grants_evasion(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_deathtouch(),
                original.has_deathtouch(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_double_strike(),
                original.has_double_strike(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_first_strike(),
                original.has_first_strike(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_flash(),
                original.has_flash(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_flying(),
                original.has_flying(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_haste(),
                original.has_haste(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_lifelink(),
                original.has_lifelink(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_menace(),
                original.has_menace(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_reach(),
                original.has_reach(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_trample(),
                original.has_trample(),
                "{:?}",
                original.id()
            );
            assert_eq!(
                restored.has_vigilance(),
                original.has_vigilance(),
                "{:?}",
                original.id()
            );
        }
    }
}
