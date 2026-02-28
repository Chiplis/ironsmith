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

/// Can't be blocked except by creatures with flying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlyingOnlyRestriction;

impl StaticAbilityKind for FlyingOnlyRestriction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::FlyingOnlyRestriction
    }

    fn display(&self) -> String {
        "Can't be blocked except by creatures with flying".to_string()
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

/// Can block only creatures with flying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CanBlockOnlyFlying;

impl StaticAbilityKind for CanBlockOnlyFlying {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CanBlockOnlyFlying
    }

    fn display(&self) -> String {
        "Can block only creatures with flying".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

/// "This creature can block an additional creature each combat."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanBlockAdditionalCreatureEachCombat {
    pub additional: usize,
}

impl CanBlockAdditionalCreatureEachCombat {
    pub const fn new(additional: usize) -> Self {
        Self { additional }
    }
}

impl StaticAbilityKind for CanBlockAdditionalCreatureEachCombat {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CanBlockAdditionalCreatureEachCombat
    }

    fn display(&self) -> String {
        if self.additional == 1 {
            "Can block an additional creature each combat".to_string()
        } else {
            format!(
                "Can block {} additional creatures each combat",
                self.additional
            )
        }
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn additional_blockable_attackers(&self) -> Option<usize> {
        Some(self.additional)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxCreaturesCanAttackEachCombat {
    pub maximum: usize,
}

impl MaxCreaturesCanAttackEachCombat {
    pub const fn new(maximum: usize) -> Self {
        Self { maximum }
    }
}

impl StaticAbilityKind for MaxCreaturesCanAttackEachCombat {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MaxCreaturesCanAttackEachCombat
    }

    fn display(&self) -> String {
        let noun = if self.maximum == 1 {
            "creature"
        } else {
            "creatures"
        };
        format!(
            "No more than {} {} can attack each combat",
            self.maximum, noun
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn max_creatures_can_attack_each_combat(&self) -> Option<usize> {
        Some(self.maximum)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxCreaturesCanBlockEachCombat {
    pub maximum: usize,
}

impl MaxCreaturesCanBlockEachCombat {
    pub const fn new(maximum: usize) -> Self {
        Self { maximum }
    }
}

impl StaticAbilityKind for MaxCreaturesCanBlockEachCombat {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MaxCreaturesCanBlockEachCombat
    }

    fn display(&self) -> String {
        let noun = if self.maximum == 1 {
            "creature"
        } else {
            "creatures"
        };
        format!(
            "No more than {} {} can block each combat",
            self.maximum, noun
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn max_creatures_can_block_each_combat(&self) -> Option<usize> {
        Some(self.maximum)
    }
}

/// Landwalk: can't be blocked as long as defending player controls a land of the given subtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Landwalk {
    pub land_subtype: crate::types::Subtype,
}

impl Landwalk {
    pub const fn new(land_subtype: crate::types::Subtype) -> Self {
        Self { land_subtype }
    }
}

impl StaticAbilityKind for Landwalk {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Landwalk
    }

    fn display(&self) -> String {
        format!("{:?}walk", self.land_subtype)
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn grants_evasion(&self) -> bool {
        true
    }

    fn required_defending_player_land_subtype_for_unblockable(
        &self,
    ) -> Option<crate::types::Subtype> {
        Some(self.land_subtype)
    }
}

/// "Can't be blocked as long as defending player controls an object of the given card type."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeBlockedAsLongAsDefendingPlayerControlsCardType {
    pub card_type: crate::types::CardType,
}

fn unblockable_card_type_word(card_type: crate::types::CardType) -> &'static str {
    match card_type {
        crate::types::CardType::Artifact => "artifact",
        crate::types::CardType::Battle => "battle",
        crate::types::CardType::Creature => "creature",
        crate::types::CardType::Enchantment => "enchantment",
        crate::types::CardType::Land => "land",
        crate::types::CardType::Planeswalker => "planeswalker",
        crate::types::CardType::Instant => "instant",
        crate::types::CardType::Sorcery => "sorcery",
        crate::types::CardType::Kindred => "kindred",
    }
}

impl CantBeBlockedAsLongAsDefendingPlayerControlsCardType {
    pub const fn new(card_type: crate::types::CardType) -> Self {
        Self { card_type }
    }
}

impl StaticAbilityKind for CantBeBlockedAsLongAsDefendingPlayerControlsCardType {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantBeBlockedAsLongAsDefendingPlayerControlsCardType
    }

    fn display(&self) -> String {
        let type_word = unblockable_card_type_word(self.card_type);
        let article = if matches!(type_word.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u')) {
            "an"
        } else {
            "a"
        };
        format!("Can't be blocked as long as defending player controls {article} {type_word}")
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn grants_evasion(&self) -> bool {
        true
    }

    fn required_defending_player_card_type_for_unblockable(
        &self,
    ) -> Option<crate::types::CardType> {
        Some(self.card_type)
    }
}

/// "Can't be blocked as long as defending player controls a permanent with all listed card types."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes {
    pub card_types: Vec<crate::types::CardType>,
}

impl CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes {
    pub fn new(card_types: Vec<crate::types::CardType>) -> Self {
        assert!(
            !card_types.is_empty(),
            "multi-card-type unblockable condition requires at least one card type"
        );
        Self { card_types }
    }
}

impl StaticAbilityKind for CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
    }

    fn display(&self) -> String {
        let type_words = self
            .card_types
            .iter()
            .map(|card_type| unblockable_card_type_word(*card_type))
            .collect::<Vec<_>>()
            .join(" ");
        let article = if matches!(type_words.chars().next(), Some('a' | 'e' | 'i' | 'o' | 'u')) {
            "an"
        } else {
            "a"
        };
        format!("Can't be blocked as long as defending player controls {article} {type_words}")
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn grants_evasion(&self) -> bool {
        true
    }

    fn required_defending_player_card_types_for_unblockable(
        &self,
    ) -> Option<Vec<crate::types::CardType>> {
        Some(self.card_types.clone())
    }
}

/// Can't be blocked by creatures with power N or less.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeBlockedByPowerOrLess {
    pub threshold: i32,
}

impl CantBeBlockedByPowerOrLess {
    pub const fn new(threshold: i32) -> Self {
        Self { threshold }
    }
}

impl StaticAbilityKind for CantBeBlockedByPowerOrLess {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantBeBlockedByPowerOrLess
    }

    fn display(&self) -> String {
        format!(
            "Can't be blocked by creatures with power {} or less",
            self.threshold
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn grants_evasion(&self) -> bool {
        true
    }

    fn cant_be_blocked_by_power_or_less(&self) -> Option<i32> {
        Some(self.threshold)
    }
}

/// Can't be blocked by creatures with power N or greater.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeBlockedByPowerOrGreater {
    pub threshold: i32,
}

impl CantBeBlockedByPowerOrGreater {
    pub const fn new(threshold: i32) -> Self {
        Self { threshold }
    }
}

impl StaticAbilityKind for CantBeBlockedByPowerOrGreater {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantBeBlockedByPowerOrGreater
    }

    fn display(&self) -> String {
        format!(
            "Can't be blocked by creatures with power {} or greater",
            self.threshold
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn grants_evasion(&self) -> bool {
        true
    }

    fn cant_be_blocked_by_power_or_greater(&self) -> Option<i32> {
        Some(self.threshold)
    }
}

/// Can't be blocked by more than N creatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeBlockedByMoreThan {
    pub max_blockers: usize,
}

impl CantBeBlockedByMoreThan {
    pub const fn new(max_blockers: usize) -> Self {
        Self { max_blockers }
    }
}

impl StaticAbilityKind for CantBeBlockedByMoreThan {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantBeBlockedByMoreThan
    }

    fn display(&self) -> String {
        let noun = if self.max_blockers == 1 {
            "creature"
        } else {
            "creatures"
        };
        format!(
            "Can't be blocked by more than {} {}",
            self.max_blockers, noun
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn grants_evasion(&self) -> bool {
        true
    }

    fn maximum_blockers(&self) -> Option<usize> {
        Some(self.max_blockers)
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

/// Can't attack unless defending player controls a land with the specified subtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantAttackUnlessDefendingPlayerControlsLandSubtype {
    pub land_subtype: crate::types::Subtype,
}

impl CantAttackUnlessDefendingPlayerControlsLandSubtype {
    pub const fn new(land_subtype: crate::types::Subtype) -> Self {
        Self { land_subtype }
    }
}

impl StaticAbilityKind for CantAttackUnlessDefendingPlayerControlsLandSubtype {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantAttackUnlessDefendingPlayerControlsLandSubtype
    }

    fn display(&self) -> String {
        format!(
            "Can't attack unless defending player controls {}",
            format!("{:?}", self.land_subtype).to_ascii_lowercase()
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn required_defending_player_land_subtype_for_attack(&self) -> Option<crate::types::Subtype> {
        Some(self.land_subtype)
    }
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

/// Can't attack unless you've cast a creature spell this turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CantAttackUnlessControllerCastCreatureSpellThisTurn;

impl StaticAbilityKind for CantAttackUnlessControllerCastCreatureSpellThisTurn {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantAttackUnlessControllerCastCreatureSpellThisTurn
    }

    fn display(&self) -> String {
        "Can't attack unless you've cast a creature spell this turn".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        let cast_creature_spell_this_turn =
            game.spells_cast_this_turn_snapshots.iter().any(|snapshot| {
                snapshot.controller == controller
                    && snapshot
                        .card_types
                        .contains(&crate::types::CardType::Creature)
            });
        if cast_creature_spell_this_turn {
            return;
        }

        let mut tracker = CantEffectTracker::default();
        Restriction::attack(ObjectFilter::specific(source)).apply(
            game,
            &mut tracker,
            controller,
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
    "You may have this creature assign its combat damage as though it weren't blocked"
);

define_combat_ability!(
    CreaturesAssignCombatDamageUsingToughness,
    CreaturesAssignCombatDamageUsingToughness,
    "Each creature assigns combat damage equal to its toughness rather than its power"
);

define_combat_ability!(
    CreaturesYouControlAssignCombatDamageUsingToughness,
    CreaturesYouControlAssignCombatDamageUsingToughness,
    "Each creature you control assigns combat damage equal to its toughness rather than its power"
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

    #[test]
    fn test_max_creatures_can_attack_each_combat() {
        let cap = MaxCreaturesCanAttackEachCombat::new(2);
        assert_eq!(cap.id(), StaticAbilityId::MaxCreaturesCanAttackEachCombat);
        assert_eq!(cap.max_creatures_can_attack_each_combat(), Some(2));
    }

    #[test]
    fn test_max_creatures_can_block_each_combat() {
        let cap = MaxCreaturesCanBlockEachCombat::new(1);
        assert_eq!(cap.id(), StaticAbilityId::MaxCreaturesCanBlockEachCombat);
        assert_eq!(cap.max_creatures_can_block_each_combat(), Some(1));
    }
}
