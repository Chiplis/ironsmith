//! Combat-related static abilities.
//!
//! These abilities modify combat rules like blocking restrictions,
//! attack requirements, etc.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::effect::Restriction;
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::events::permanents::SacrificeEvent;
use crate::game_state::{CantEffectTracker, GameState};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::snapshot::ObjectSnapshot;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

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
    card_type.name()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CantAttackUnlessConditionSpec {
    YouControlMoreCreaturesThanDefendingPlayer,
    YouControlMoreLandsThanDefendingPlayer,
    YouControlAnotherCreatureWithPowerAtLeast(u32),
    YouControlAnotherArtifact,
    YouControlArtifact,
    YouControlKnightOrSoldier,
    YouControlCreatureWithPowerAtLeast(u32),
    YouControlCreatureWithPowerAndToughness(u32, u32),
    ThereIsALandWithSubtypeOnBattlefield(crate::types::Subtype),
    ThereAreLandsWithSubtypeOnBattlefieldOrMore {
        subtype: crate::types::Subtype,
        count: u32,
    },
    ThereAreCardsInYourGraveyardOrMore(u32),
    DefendingPlayerIsPoisoned,
    DefendingPlayerHasCardsInGraveyardOrMore(u32),
    DefendingPlayerControlsEnchantmentOrEnchantedPermanent,
    DefendingPlayerControlsSnowLand,
    DefendingPlayerControlsCreatureWithFlying,
    DefendingPlayerControlsBluePermanent,
    OpponentWasDealtDamageThisTurn,
    YouControlArtifactsOrMore(u32),
    AtLeastNOtherCreaturesAttack(u32),
    CreatureWithGreaterPowerAlsoAttacks,
    BlackOrGreenCreatureAlsoAttacks,
    SacrificeLands {
        count: u32,
        subtype: Option<crate::types::Subtype>,
    },
    ReturnEnchantmentYouControlToOwnersHand,
    PayOneForEachPlusOnePlusOneCounterOnIt,
    DefendingPlayerIsMonarch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CantAttackUnlessCondition {
    pub condition: CantAttackUnlessConditionSpec,
    pub display_text: String,
}

impl CantAttackUnlessCondition {
    pub fn new(condition: CantAttackUnlessConditionSpec, display_text: impl Into<String>) -> Self {
        Self {
            condition,
            display_text: display_text.into(),
        }
    }

    fn source_can_attack_without_defender(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<bool> {
        use crate::types::{CardType, Subtype};

        let count_controlled = |card_type: CardType| -> u32 {
            game.battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    obj.controller == controller && game.object_has_card_type(obj.id, card_type)
                })
                .count() as u32
        };

        let controller_has_other_creature_with_power_at_least = |threshold: u32| -> bool {
            game.battlefield.iter().any(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.id != source
                        && obj.controller == controller
                        && game.object_has_card_type(obj.id, CardType::Creature)
                        && game
                            .calculated_power(obj.id)
                            .or_else(|| obj.power())
                            .is_some_and(|power| power >= threshold as i32)
                })
            })
        };

        let controller_has_creature_with_power_at_least = |threshold: u32| -> bool {
            game.battlefield.iter().any(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == controller
                        && game.object_has_card_type(obj.id, CardType::Creature)
                        && game
                            .calculated_power(obj.id)
                            .or_else(|| obj.power())
                            .is_some_and(|power| power >= threshold as i32)
                })
            })
        };

        let controller_has_creature_with_exact_power_toughness =
            |required_power: u32, required_toughness: u32| -> bool {
                game.battlefield.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == controller
                            && game.object_has_card_type(obj.id, CardType::Creature)
                            && game
                                .calculated_power(obj.id)
                                .or_else(|| obj.power())
                                .is_some_and(|power| power == required_power as i32)
                            && game
                                .calculated_toughness(obj.id)
                                .or_else(|| obj.toughness())
                                .is_some_and(|toughness| toughness == required_toughness as i32)
                    })
                })
            };

        let count_lands_with_subtype = |subtype: Subtype| -> u32 {
            game.battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    game.object_has_card_type(obj.id, CardType::Land)
                        && game.calculated_subtypes(obj.id).contains(&subtype)
                })
                .count() as u32
        };

        match self.condition {
            CantAttackUnlessConditionSpec::YouControlAnotherCreatureWithPowerAtLeast(threshold) => {
                Some(controller_has_other_creature_with_power_at_least(threshold))
            }
            CantAttackUnlessConditionSpec::YouControlAnotherArtifact => Some(
                game.battlefield.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.id != source
                            && obj.controller == controller
                            && game.object_has_card_type(obj.id, CardType::Artifact)
                    })
                }),
            ),
            CantAttackUnlessConditionSpec::YouControlArtifact => Some(
                game.battlefield.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == controller
                            && game.object_has_card_type(obj.id, CardType::Artifact)
                    })
                }),
            ),
            CantAttackUnlessConditionSpec::YouControlKnightOrSoldier => Some(
                game.battlefield.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == controller
                            && game.object_has_card_type(obj.id, CardType::Creature)
                            && {
                                let subtypes = game.calculated_subtypes(obj.id);
                                subtypes.contains(&Subtype::Knight)
                                    || subtypes.contains(&Subtype::Soldier)
                            }
                    })
                }),
            ),
            CantAttackUnlessConditionSpec::YouControlCreatureWithPowerAtLeast(threshold) => {
                Some(controller_has_creature_with_power_at_least(threshold))
            }
            CantAttackUnlessConditionSpec::YouControlCreatureWithPowerAndToughness(
                required_power,
                required_toughness,
            ) => Some(controller_has_creature_with_exact_power_toughness(
                required_power,
                required_toughness,
            )),
            CantAttackUnlessConditionSpec::ThereIsALandWithSubtypeOnBattlefield(subtype) => {
                Some(count_lands_with_subtype(subtype) >= 1)
            }
            CantAttackUnlessConditionSpec::ThereAreLandsWithSubtypeOnBattlefieldOrMore {
                subtype,
                count,
            } => Some(count_lands_with_subtype(subtype) >= count),
            CantAttackUnlessConditionSpec::ThereAreCardsInYourGraveyardOrMore(count) => Some(
                game.player(controller)
                    .is_some_and(|player| player.graveyard.len() >= count as usize),
            ),
            CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn => Some(
                game.players.iter().any(|player| {
                    player.is_in_game()
                        && player.id != controller
                        && game
                            .damage_to_players_this_turn
                            .get(&player.id)
                            .copied()
                            .unwrap_or(0)
                            > 0
                }),
            ),
            CantAttackUnlessConditionSpec::YouControlArtifactsOrMore(required_count) => {
                Some(count_controlled(CardType::Artifact) >= required_count)
            }
            CantAttackUnlessConditionSpec::YouControlMoreCreaturesThanDefendingPlayer
            | CantAttackUnlessConditionSpec::YouControlMoreLandsThanDefendingPlayer
            | CantAttackUnlessConditionSpec::DefendingPlayerIsPoisoned
            | CantAttackUnlessConditionSpec::DefendingPlayerHasCardsInGraveyardOrMore(_)
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsEnchantmentOrEnchantedPermanent
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsSnowLand
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsCreatureWithFlying
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsBluePermanent
            | CantAttackUnlessConditionSpec::AtLeastNOtherCreaturesAttack(_)
            | CantAttackUnlessConditionSpec::CreatureWithGreaterPowerAlsoAttacks
            | CantAttackUnlessConditionSpec::BlackOrGreenCreatureAlsoAttacks
            | CantAttackUnlessConditionSpec::SacrificeLands { .. }
            | CantAttackUnlessConditionSpec::ReturnEnchantmentYouControlToOwnersHand
            | CantAttackUnlessConditionSpec::PayOneForEachPlusOnePlusOneCounterOnIt
            | CantAttackUnlessConditionSpec::DefendingPlayerIsMonarch => None,
        }
    }

    fn source_can_attack_defender(
        &self,
        game: &GameState,
        controller: PlayerId,
        defending_player: PlayerId,
    ) -> Option<bool> {
        use crate::types::{CardType, Subtype, Supertype};

        match self.condition {
            CantAttackUnlessConditionSpec::YouControlMoreCreaturesThanDefendingPlayer => {
                let controller_creatures = game
                    .battlefield
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        obj.controller == controller
                            && game.object_has_card_type(obj.id, CardType::Creature)
                    })
                    .count();
                let defending_creatures = game
                    .battlefield
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        obj.controller == defending_player
                            && game.object_has_card_type(obj.id, CardType::Creature)
                    })
                    .count();
                Some(controller_creatures > defending_creatures)
            }
            CantAttackUnlessConditionSpec::YouControlMoreLandsThanDefendingPlayer => {
                let controller_lands = game
                    .battlefield
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        obj.controller == controller
                            && game.object_has_card_type(obj.id, CardType::Land)
                    })
                    .count();
                let defending_lands = game
                    .battlefield
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        obj.controller == defending_player
                            && game.object_has_card_type(obj.id, CardType::Land)
                    })
                    .count();
                Some(controller_lands > defending_lands)
            }
            CantAttackUnlessConditionSpec::DefendingPlayerIsPoisoned => Some(
                game.player(defending_player)
                    .is_some_and(|player| player.poison_counters > 0),
            ),
            CantAttackUnlessConditionSpec::DefendingPlayerHasCardsInGraveyardOrMore(count) => Some(
                game.player(defending_player)
                    .is_some_and(|player| player.graveyard.len() >= count as usize),
            ),
            CantAttackUnlessConditionSpec::DefendingPlayerControlsEnchantmentOrEnchantedPermanent => {
                Some(game.battlefield.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == defending_player
                            && (game.object_has_card_type(obj.id, CardType::Enchantment)
                                || obj.attachments.iter().any(|attachment_id| {
                                    game.object(*attachment_id).is_some_and(|attachment| {
                                        game.calculated_subtypes(attachment.id).contains(&Subtype::Aura)
                                    })
                                }))
                    })
                }))
            }
            CantAttackUnlessConditionSpec::DefendingPlayerControlsSnowLand => {
                Some(game.battlefield.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == defending_player
                            && game.object_has_card_type(obj.id, CardType::Land)
                            && game
                                .calculated_characteristics(obj.id)
                                .map(|chars| chars.supertypes.contains(&Supertype::Snow))
                                .unwrap_or_else(|| obj.supertypes.contains(&Supertype::Snow))
                    })
                }))
            }
            CantAttackUnlessConditionSpec::DefendingPlayerControlsCreatureWithFlying => {
                Some(game.battlefield.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == defending_player
                            && game.object_has_card_type(obj.id, CardType::Creature)
                            && game.object_has_ability(obj.id, &crate::static_abilities::StaticAbility::flying())
                    })
                }))
            }
            CantAttackUnlessConditionSpec::DefendingPlayerControlsBluePermanent => {
                Some(game.battlefield.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == defending_player
                            && game
                                .calculated_characteristics(obj.id)
                                .map(|chars| chars.colors.contains(crate::color::Color::Blue))
                                .unwrap_or_else(|| {
                                    obj.colors().contains(crate::color::Color::Blue)
                                })
                    })
                }))
            }
            CantAttackUnlessConditionSpec::DefendingPlayerIsMonarch => {
                Some(game.monarch == Some(defending_player))
            }
            CantAttackUnlessConditionSpec::YouControlAnotherCreatureWithPowerAtLeast(_)
            | CantAttackUnlessConditionSpec::YouControlAnotherArtifact
            | CantAttackUnlessConditionSpec::YouControlArtifact
            | CantAttackUnlessConditionSpec::YouControlKnightOrSoldier
            | CantAttackUnlessConditionSpec::YouControlCreatureWithPowerAtLeast(_)
            | CantAttackUnlessConditionSpec::YouControlCreatureWithPowerAndToughness(_, _)
            | CantAttackUnlessConditionSpec::ThereIsALandWithSubtypeOnBattlefield(_)
            | CantAttackUnlessConditionSpec::ThereAreLandsWithSubtypeOnBattlefieldOrMore { .. }
            | CantAttackUnlessConditionSpec::ThereAreCardsInYourGraveyardOrMore(_)
            | CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn
            | CantAttackUnlessConditionSpec::YouControlArtifactsOrMore(_)
            | CantAttackUnlessConditionSpec::AtLeastNOtherCreaturesAttack(_)
            | CantAttackUnlessConditionSpec::CreatureWithGreaterPowerAlsoAttacks
            | CantAttackUnlessConditionSpec::BlackOrGreenCreatureAlsoAttacks
            | CantAttackUnlessConditionSpec::SacrificeLands { .. }
            | CantAttackUnlessConditionSpec::ReturnEnchantmentYouControlToOwnersHand
            | CantAttackUnlessConditionSpec::PayOneForEachPlusOnePlusOneCounterOnIt => None,
        }
    }

    fn source_can_attack_with_group(
        &self,
        game: &GameState,
        source: ObjectId,
        attacking_creatures: &[ObjectId],
    ) -> Option<bool> {
        use crate::types::CardType;

        match self.condition {
            CantAttackUnlessConditionSpec::AtLeastNOtherCreaturesAttack(required) => {
                let other_attacking_creatures = attacking_creatures
                    .iter()
                    .copied()
                    .filter(|id| *id != source)
                    .filter(|id| game.object_has_card_type(*id, CardType::Creature))
                    .count() as u32;
                Some(other_attacking_creatures >= required)
            }
            CantAttackUnlessConditionSpec::CreatureWithGreaterPowerAlsoAttacks => {
                let source_power = game
                    .calculated_power(source)
                    .or_else(|| game.object(source).and_then(|obj| obj.power()))
                    .unwrap_or(0);
                Some(attacking_creatures.iter().any(|id| {
                    if *id == source {
                        return false;
                    }
                    game.object(*id).is_some_and(|obj| {
                        game.object_has_card_type(obj.id, CardType::Creature)
                            && game
                                .calculated_power(obj.id)
                                .or_else(|| obj.power())
                                .is_some_and(|power| power > source_power)
                    })
                }))
            }
            CantAttackUnlessConditionSpec::BlackOrGreenCreatureAlsoAttacks => Some(
                attacking_creatures.iter().any(|id| {
                    if *id == source {
                        return false;
                    }
                    game.object(*id).is_some_and(|obj| {
                        game.object_has_card_type(obj.id, CardType::Creature)
                            && {
                                let colors = game
                                    .calculated_characteristics(obj.id)
                                    .map(|chars| chars.colors)
                                    .unwrap_or(obj.colors());
                                colors.contains(crate::color::Color::Black)
                                    || colors.contains(crate::color::Color::Green)
                            }
                    })
                }),
            ),
            CantAttackUnlessConditionSpec::YouControlMoreCreaturesThanDefendingPlayer
            | CantAttackUnlessConditionSpec::YouControlMoreLandsThanDefendingPlayer
            | CantAttackUnlessConditionSpec::YouControlAnotherCreatureWithPowerAtLeast(_)
            | CantAttackUnlessConditionSpec::YouControlAnotherArtifact
            | CantAttackUnlessConditionSpec::YouControlArtifact
            | CantAttackUnlessConditionSpec::YouControlKnightOrSoldier
            | CantAttackUnlessConditionSpec::YouControlCreatureWithPowerAtLeast(_)
            | CantAttackUnlessConditionSpec::YouControlCreatureWithPowerAndToughness(_, _)
            | CantAttackUnlessConditionSpec::ThereIsALandWithSubtypeOnBattlefield(_)
            | CantAttackUnlessConditionSpec::ThereAreLandsWithSubtypeOnBattlefieldOrMore { .. }
            | CantAttackUnlessConditionSpec::ThereAreCardsInYourGraveyardOrMore(_)
            | CantAttackUnlessConditionSpec::DefendingPlayerIsPoisoned
            | CantAttackUnlessConditionSpec::DefendingPlayerHasCardsInGraveyardOrMore(_)
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsEnchantmentOrEnchantedPermanent
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsSnowLand
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsCreatureWithFlying
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsBluePermanent
            | CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn
            | CantAttackUnlessConditionSpec::YouControlArtifactsOrMore(_)
            | CantAttackUnlessConditionSpec::SacrificeLands { .. }
            | CantAttackUnlessConditionSpec::ReturnEnchantmentYouControlToOwnersHand
            | CantAttackUnlessConditionSpec::PayOneForEachPlusOnePlusOneCounterOnIt
            | CantAttackUnlessConditionSpec::DefendingPlayerIsMonarch => None,
        }
    }

    fn eligible_sacrificial_lands_for_controller(
        game: &GameState,
        controller: PlayerId,
        subtype: Option<crate::types::Subtype>,
    ) -> Vec<ObjectId> {
        game.battlefield
            .iter()
            .copied()
            .filter(|&id| {
                game.object(id).is_some_and(|obj| {
                    obj.controller == controller
                        && game.object_has_card_type(id, crate::types::CardType::Land)
                        && subtype
                            .is_none_or(|required| game.calculated_subtypes(id).contains(&required))
                        && game.can_be_sacrificed(id)
                })
            })
            .collect()
    }

    fn can_pay_attack_cost_now(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<bool> {
        match self.condition {
            CantAttackUnlessConditionSpec::SacrificeLands { count, subtype } => Some(
                Self::eligible_sacrificial_lands_for_controller(game, controller, subtype).len()
                    >= count as usize,
            ),
            CantAttackUnlessConditionSpec::ReturnEnchantmentYouControlToOwnersHand => {
                Some(game.battlefield.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.controller == controller
                            && game.object_has_card_type(id, crate::types::CardType::Enchantment)
                    })
                }))
            }
            CantAttackUnlessConditionSpec::PayOneForEachPlusOnePlusOneCounterOnIt => {
                Some(game.object(source).is_some())
            }
            CantAttackUnlessConditionSpec::YouControlMoreCreaturesThanDefendingPlayer
            | CantAttackUnlessConditionSpec::YouControlMoreLandsThanDefendingPlayer
            | CantAttackUnlessConditionSpec::YouControlAnotherCreatureWithPowerAtLeast(_)
            | CantAttackUnlessConditionSpec::YouControlAnotherArtifact
            | CantAttackUnlessConditionSpec::YouControlArtifact
            | CantAttackUnlessConditionSpec::YouControlKnightOrSoldier
            | CantAttackUnlessConditionSpec::YouControlCreatureWithPowerAtLeast(_)
            | CantAttackUnlessConditionSpec::YouControlCreatureWithPowerAndToughness(_, _)
            | CantAttackUnlessConditionSpec::ThereIsALandWithSubtypeOnBattlefield(_)
            | CantAttackUnlessConditionSpec::ThereAreLandsWithSubtypeOnBattlefieldOrMore { .. }
            | CantAttackUnlessConditionSpec::ThereAreCardsInYourGraveyardOrMore(_)
            | CantAttackUnlessConditionSpec::DefendingPlayerIsPoisoned
            | CantAttackUnlessConditionSpec::DefendingPlayerHasCardsInGraveyardOrMore(_)
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsEnchantmentOrEnchantedPermanent
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsSnowLand
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsCreatureWithFlying
            | CantAttackUnlessConditionSpec::DefendingPlayerControlsBluePermanent
            | CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn
            | CantAttackUnlessConditionSpec::YouControlArtifactsOrMore(_)
            | CantAttackUnlessConditionSpec::AtLeastNOtherCreaturesAttack(_)
            | CantAttackUnlessConditionSpec::CreatureWithGreaterPowerAlsoAttacks
            | CantAttackUnlessConditionSpec::BlackOrGreenCreatureAlsoAttacks
            | CantAttackUnlessConditionSpec::DefendingPlayerIsMonarch => None,
        }
    }

    fn attack_generic_mana_requirement(
        &self,
        game: &GameState,
        source: ObjectId,
        _controller: PlayerId,
    ) -> Option<u32> {
        match self.condition {
            CantAttackUnlessConditionSpec::PayOneForEachPlusOnePlusOneCounterOnIt => {
                let amount = game
                    .object(source)
                    .and_then(|obj| obj.counters.get(&CounterType::PlusOnePlusOne).copied())
                    .unwrap_or(0);
                Some(amount)
            }
            _ => None,
        }
    }

    fn pay_sacrifice_attack_cost(
        game: &mut GameState,
        source: ObjectId,
        controller: PlayerId,
        count: u32,
        subtype: Option<crate::types::Subtype>,
    ) -> Result<(), String> {
        let candidates = Self::eligible_sacrificial_lands_for_controller(game, controller, subtype);
        if candidates.len() < count as usize {
            return Err("Cannot pay required attack cost".to_string());
        }

        let chosen: Vec<ObjectId> = candidates.into_iter().take(count as usize).collect();
        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;

        for target_id in chosen {
            let snapshot = game
                .object(target_id)
                .map(|obj| ObjectSnapshot::from_object(obj, game));
            let sacrificing_player = snapshot
                .as_ref()
                .map(|snap| snap.controller)
                .or(Some(controller));

            match process_zone_change(
                game,
                target_id,
                Zone::Battlefield,
                Zone::Graveyard,
                &mut decision_maker,
            ) {
                EventOutcome::Prevented | EventOutcome::NotApplicable => {
                    return Err("Cannot pay required attack cost".to_string());
                }
                EventOutcome::Proceed(final_zone) => {
                    game.move_object(target_id, final_zone);
                    if final_zone == Zone::Graveyard {
                        game.queue_trigger_event(TriggerEvent::new(
                            SacrificeEvent::new(target_id, Some(source))
                                .with_snapshot(snapshot, sacrificing_player),
                        ));
                    }
                }
                EventOutcome::Replaced => {}
            }
        }

        Ok(())
    }

    fn pay_return_enchantment_attack_cost(
        game: &mut GameState,
        controller: PlayerId,
    ) -> Result<(), String> {
        let Some(target_id) = game.battlefield.iter().copied().find(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.controller == controller
                    && game.object_has_card_type(id, crate::types::CardType::Enchantment)
            })
        }) else {
            return Err("Cannot pay required attack cost".to_string());
        };

        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
        match process_zone_change(
            game,
            target_id,
            Zone::Battlefield,
            Zone::Hand,
            &mut decision_maker,
        ) {
            EventOutcome::Prevented | EventOutcome::NotApplicable | EventOutcome::Replaced => {
                Err("Cannot pay required attack cost".to_string())
            }
            EventOutcome::Proceed(final_zone) => {
                if final_zone != Zone::Hand {
                    return Err("Cannot pay required attack cost".to_string());
                }
                game.move_object(target_id, final_zone);
                Ok(())
            }
        }
    }

    fn pay_non_mana_attack_cost_now(
        &self,
        game: &mut GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<Result<(), String>> {
        match self.condition {
            CantAttackUnlessConditionSpec::SacrificeLands { count, subtype } => Some(
                Self::pay_sacrifice_attack_cost(game, source, controller, count, subtype),
            ),
            CantAttackUnlessConditionSpec::ReturnEnchantmentYouControlToOwnersHand => {
                Some(Self::pay_return_enchantment_attack_cost(game, controller))
            }
            CantAttackUnlessConditionSpec::PayOneForEachPlusOnePlusOneCounterOnIt => Some(Ok(())),
            _ => None,
        }
    }
}

impl StaticAbilityKind for CantAttackUnlessCondition {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantAttackUnlessCondition
    }

    fn display(&self) -> String {
        self.display_text.clone()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        let Some(can_attack) = self.source_can_attack_without_defender(game, source, controller)
        else {
            return;
        };
        if can_attack {
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

    fn can_attack_specific_defender(
        &self,
        game: &GameState,
        _source: ObjectId,
        controller: PlayerId,
        defending_player: PlayerId,
    ) -> Option<bool> {
        self.source_can_attack_defender(game, controller, defending_player)
    }

    fn can_attack_with_attacking_group(
        &self,
        game: &GameState,
        source: ObjectId,
        _controller: PlayerId,
        attacking_creatures: &[ObjectId],
    ) -> Option<bool> {
        self.source_can_attack_with_group(game, source, attacking_creatures)
    }

    fn can_pay_attack_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<bool> {
        self.can_pay_attack_cost_now(game, source, controller)
    }

    fn generic_attack_mana_cost_for_source(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<u32> {
        self.attack_generic_mana_requirement(game, source, controller)
    }

    fn pay_non_mana_attack_cost(
        &self,
        game: &mut GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<Result<(), String>> {
        self.pay_non_mana_attack_cost_now(game, source, controller)
    }
}

/// "Creatures can't attack you unless their controller pays {N} for each creature they control
/// that's attacking you."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantAttackYouUnlessControllerPaysPerAttacker {
    amount: u32,
}

impl CantAttackYouUnlessControllerPaysPerAttacker {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl StaticAbilityKind for CantAttackYouUnlessControllerPaysPerAttacker {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttacker
    }

    fn display(&self) -> String {
        format!(
            "Creatures can't attack you unless their controller pays {{{}}} for each creature they control that's attacking you",
            self.amount
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generic_attack_tax_per_attacker_against_you(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Option<u32> {
        Some(self.amount)
    }
}

fn count_basic_land_types_among_lands_you_control(game: &GameState, controller: PlayerId) -> u32 {
    use crate::types::{CardType, Subtype};
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    for &object_id in &game.battlefield {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if object.controller != controller || !game.object_has_card_type(object_id, CardType::Land)
        {
            continue;
        }

        for subtype in game.calculated_subtypes(object_id) {
            if matches!(
                subtype,
                Subtype::Plains
                    | Subtype::Island
                    | Subtype::Swamp
                    | Subtype::Mountain
                    | Subtype::Forest
            ) {
                seen.insert(subtype);
            }
        }
    }

    seen.len() as u32
}

/// "Creatures can't attack you unless their controller pays {X} for each creature they control
/// that's attacking you, where X is the number of basic land types among lands you control."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl;

impl StaticAbilityKind
    for CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
{
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
    }

    fn display(&self) -> String {
        "Creatures can't attack you unless their controller pays {X} for each creature they control that's attacking you, where X is the number of basic land types among lands you control".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generic_attack_tax_per_attacker_against_you(
        &self,
        game: &GameState,
        _source: ObjectId,
        controller: PlayerId,
    ) -> Option<u32> {
        Some(count_basic_land_types_among_lands_you_control(
            game, controller,
        ))
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

/// Can't attack unless you've cast a noncreature spell this turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CantAttackUnlessControllerCastNonCreatureSpellThisTurn;

impl StaticAbilityKind for CantAttackUnlessControllerCastNonCreatureSpellThisTurn {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CantAttackUnlessControllerCastNonCreatureSpellThisTurn
    }

    fn display(&self) -> String {
        "Can't attack unless you've cast a noncreature spell this turn".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        let cast_noncreature_spell_this_turn =
            game.spells_cast_this_turn_snapshots.iter().any(|snapshot| {
                snapshot.controller == controller
                    && !snapshot
                        .card_types
                        .contains(&crate::types::CardType::Creature)
            });
        if cast_noncreature_spell_this_turn {
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
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::object::CounterType;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

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
    fn test_cant_attack_unless_cast_creature_spell_this_turn() {
        let ability = CantAttackUnlessControllerCastCreatureSpellThisTurn;
        assert_eq!(
            ability.id(),
            StaticAbilityId::CantAttackUnlessControllerCastCreatureSpellThisTurn
        );
        assert_eq!(
            ability.display(),
            "Can't attack unless you've cast a creature spell this turn"
        );
    }

    #[test]
    fn test_cant_attack_unless_cast_noncreature_spell_this_turn() {
        let ability = CantAttackUnlessControllerCastNonCreatureSpellThisTurn;
        assert_eq!(
            ability.id(),
            StaticAbilityId::CantAttackUnlessControllerCastNonCreatureSpellThisTurn
        );
        assert_eq!(
            ability.display(),
            "Can't attack unless you've cast a noncreature spell this turn"
        );
    }

    #[test]
    fn test_cant_attack_unless_condition_id_and_display() {
        let ability = CantAttackUnlessCondition::new(
            CantAttackUnlessConditionSpec::DefendingPlayerIsPoisoned,
            "Can't attack unless defending player is poisoned",
        );
        assert_eq!(ability.id(), StaticAbilityId::CantAttackUnlessCondition);
        assert_eq!(
            ability.display(),
            "Can't attack unless defending player is poisoned"
        );
    }

    #[test]
    fn test_cant_attack_unless_condition_monarch_defender_check() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let ability = CantAttackUnlessCondition::new(
            CantAttackUnlessConditionSpec::DefendingPlayerIsMonarch,
            "Can't attack unless defending player is the monarch",
        );
        assert_eq!(
            ability.can_attack_specific_defender(&game, ObjectId::new(), alice, bob),
            Some(false)
        );
        game.set_monarch(Some(bob));
        assert_eq!(
            ability.can_attack_specific_defender(&game, ObjectId::new(), alice, bob),
            Some(true)
        );
    }

    #[test]
    fn test_cant_attack_unless_condition_counter_attack_mana_requirement() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::new(), "Counter Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("source should exist")
            .add_counters(CounterType::PlusOnePlusOne, 3);

        let ability = CantAttackUnlessCondition::new(
            CantAttackUnlessConditionSpec::PayOneForEachPlusOnePlusOneCounterOnIt,
            "Can't attack unless you pay {1} for each +1/+1 counter on it",
        );
        assert_eq!(
            ability.generic_attack_mana_cost_for_source(&game, source_id, alice),
            Some(3)
        );
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

    #[test]
    fn test_collective_restraint_attack_tax_id_and_display() {
        let ability =
            CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl;
        assert_eq!(
            ability.id(),
            StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
        );
        assert!(
            ability
                .display()
                .to_ascii_lowercase()
                .contains("basic land types among lands you control")
        );
    }

    #[test]
    fn test_collective_restraint_attack_tax_domain_value() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let bob = PlayerId::from_index(1);

        let plains = CardBuilder::new(CardId::new(), "Plains")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Plains])
            .build();
        let island = CardBuilder::new(CardId::new(), "Island")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Island])
            .build();

        let plains_id = game.create_object_from_card(&plains, bob, Zone::Battlefield);
        let _island_id = game.create_object_from_card(&island, bob, Zone::Battlefield);

        let ability =
            CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl;
        let tax = ability
            .generic_attack_tax_per_attacker_against_you(&game, plains_id, bob)
            .expect("collective restraint tax should resolve");
        assert_eq!(tax, 2);
    }

    #[test]
    fn test_fixed_attack_tax_id_display_and_value() {
        let ability = CantAttackYouUnlessControllerPaysPerAttacker::new(2);
        assert_eq!(
            ability.id(),
            StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttacker
        );
        assert!(
            ability
                .display()
                .contains("controller pays {2} for each creature they control")
        );

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let bob = PlayerId::from_index(1);
        let tax = ability
            .generic_attack_tax_per_attacker_against_you(&game, ObjectId::new(), bob)
            .expect("fixed attack tax should resolve");
        assert_eq!(tax, 2);
    }
}
