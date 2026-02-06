//! Modular static ability system for MTG.
//!
//! This module provides a trait-based architecture for static abilities.
//! Each ability type implements the `StaticAbilityKind` trait, allowing for:
//! - Co-located tests with each ability implementation
//! - Self-contained ability logic
//! - Easy addition of new abilities without modifying central code
//! - Scalable to thousands of unique card abilities
//!
//! # Module Structure
//!
//! ```text
//! static_abilities/
//!   mod.rs              - This file, trait definition and StaticAbility wrapper
//!   id.rs               - StaticAbilityId enum for identity checking
//!   keywords.rs         - Simple keyword abilities (Flying, Trample, etc.)
//!   combat.rs           - Combat modifiers (MustAttack, CantBlock, etc.)
//!   protection.rs       - Protection, Hexproof, Ward, Shroud
//!   continuous.rs       - Effect-generating abilities (Anthem, GrantAbility, etc.)
//!   cost_modifiers.rs   - Cost modification (Affinity, Delve, Convoke, etc.)
//!   restrictions.rs     - Game rule restrictions (PlayersCantGainLife, etc.)
//!   characteristics.rs  - Characteristic-defining abilities
//!   misc.rs             - Other abilities
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
//!
//! // Create abilities using convenience constructors
//! let flying = StaticAbility::flying();
//! let anthem = StaticAbility::anthem(filter, 1, 1);
//!
//! // Check ability identity
//! if ability.id() == StaticAbilityId::Flying {
//!     // Handle flying
//! }
//!
//! // Generate continuous effects
//! let effects = ability.generate_effects(source, controller);
//! ```

mod characteristics;
mod combat;
mod continuous;
mod cost_modifiers;
mod id;
mod keywords;
mod misc;
mod protection;
mod restrictions;

// Re-export the ID enum
pub use id::StaticAbilityId;

// Re-export ability structs for direct construction
pub use characteristics::*;
pub use combat::*;
pub use continuous::*;
pub use cost_modifiers::*;
pub use keywords::*;
pub use misc::*;
pub use protection::*;
pub use restrictions::*;

use crate::continuous::ContinuousEffect;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Trait for static ability behavior.
///
/// All static abilities implement this trait. Each ability is responsible for:
/// - Providing its identity (for equality/matching checks)
/// - Generating continuous effects (if applicable)
/// - Applying game restrictions (if applicable)
/// - Providing display text
///
/// Most abilities only override a few methods - the defaults handle the common case
/// of simple keyword abilities that don't generate effects.
pub trait StaticAbilityKind: std::fmt::Debug + Send + Sync {
    /// Get the unique identifier for this ability type.
    ///
    /// Used for identity checks like `ability.id() == StaticAbilityId::Flying`.
    fn id(&self) -> StaticAbilityId;

    /// Human-readable display name for this ability.
    ///
    /// Examples: "Flying", "Protection from red", "Creatures you control get +1/+1"
    fn display(&self) -> String;

    /// Clone this ability into a boxed trait object.
    ///
    /// Required because `Clone` is not object-safe.
    fn clone_box(&self) -> Box<dyn StaticAbilityKind>;

    /// Generate continuous effects for this ability.
    ///
    /// Called by the static ability processor to create effects that go through
    /// the layer system. Most abilities return empty (the default).
    ///
    /// Override for: Anthem, GrantAbility, BloodMoon, Humility, etc.
    fn generate_effects(
        &self,
        _source: ObjectId,
        _controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![]
    }

    /// Apply game restrictions for this ability.
    ///
    /// Called when a permanent with this ability is on the battlefield.
    /// Modifies the game's restriction trackers.
    ///
    /// Override for: PlayersCantGainLife, CantAttack, Hexproof, etc.
    fn apply_restrictions(&self, _game: &mut GameState, _source: ObjectId, _controller: PlayerId) {
        // Default: no restrictions
    }

    /// Generate a replacement effect for this ability.
    ///
    /// Returns None if this ability doesn't create a replacement effect.
    /// Override for: EntersTapped, ShuffleIntoLibraryFromGraveyard, etc.
    fn generate_replacement_effect(
        &self,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Option<crate::replacement::ReplacementEffect> {
        None
    }

    /// Check if this ability is currently active.
    ///
    /// Most abilities are always active. Override for conditional abilities
    /// like Metalcraft or Devotion-based effects.
    fn is_active(&self, _game: &GameState, _source: ObjectId) -> bool {
        true
    }

    // ========================================================================
    // Query methods for specific ability checks
    // These allow checking ability properties without pattern matching.
    // ========================================================================

    /// Returns true if this is a keyword ability (Flying, Trample, etc.)
    fn is_keyword(&self) -> bool {
        false
    }

    /// Returns true if this ability grants evasion (Flying, Shadow, etc.)
    fn grants_evasion(&self) -> bool {
        false
    }

    /// Returns true if this ability prevents blocking (Unblockable, etc.)
    fn is_unblockable(&self) -> bool {
        false
    }

    /// Returns true if this is a first/double strike ability.
    fn has_first_strike(&self) -> bool {
        false
    }

    /// Returns true if this is a double strike ability.
    fn has_double_strike(&self) -> bool {
        false
    }

    /// Returns true if this grants deathtouch.
    fn has_deathtouch(&self) -> bool {
        false
    }

    /// Returns true if this grants lifelink.
    fn has_lifelink(&self) -> bool {
        false
    }

    /// Returns true if this grants trample.
    fn has_trample(&self) -> bool {
        false
    }

    /// Returns true if this grants vigilance.
    fn has_vigilance(&self) -> bool {
        false
    }

    /// Returns true if this grants haste.
    fn has_haste(&self) -> bool {
        false
    }

    /// Returns true if this grants flash.
    fn has_flash(&self) -> bool {
        false
    }

    /// Returns true if this grants reach.
    fn has_reach(&self) -> bool {
        false
    }

    /// Returns true if this grants defender.
    fn has_defender(&self) -> bool {
        false
    }

    /// Returns true if this grants indestructible.
    fn has_indestructible(&self) -> bool {
        false
    }

    /// Returns true if this grants hexproof.
    fn has_hexproof(&self) -> bool {
        false
    }

    /// Get hexproof-from filter if this is a hexproof-from ability.
    fn hexproof_from_filter(&self) -> Option<&crate::target::ObjectFilter> {
        None
    }

    /// Returns true if this grants shroud.
    fn has_shroud(&self) -> bool {
        false
    }

    /// Returns true if this is menace.
    fn has_menace(&self) -> bool {
        false
    }

    /// Returns true if this is flying.
    fn has_flying(&self) -> bool {
        false
    }

    /// Returns true if this grants protection from something.
    fn has_protection(&self) -> bool {
        false
    }

    /// Get protection details if this is a protection ability.
    fn protection_from(&self) -> Option<&crate::ability::ProtectionFrom> {
        None
    }

    /// Get ward cost if this is a ward ability.
    fn ward_cost(&self) -> Option<&crate::cost::TotalCost> {
        None
    }

    /// Returns true if this is an anthem effect.
    fn is_anthem(&self) -> bool {
        false
    }

    /// Returns true if this grants abilities to other permanents.
    fn grants_abilities(&self) -> bool {
        false
    }

    /// Returns true if this modifies casting costs.
    fn modifies_costs(&self) -> bool {
        false
    }

    /// Returns true if this is affinity for artifacts.
    fn has_affinity(&self) -> bool {
        false
    }

    /// Returns true if this is delve.
    fn has_delve(&self) -> bool {
        false
    }

    /// Returns true if this is convoke.
    fn has_convoke(&self) -> bool {
        false
    }

    /// Returns true if this is improvise.
    fn has_improvise(&self) -> bool {
        false
    }

    /// Get cost reduction details if this is a cost reduction ability.
    fn cost_reduction(&self) -> Option<&CostReduction> {
        None
    }

    /// Get cost increase details if this is a cost increase ability.
    fn cost_increase(&self) -> Option<&CostIncrease> {
        None
    }

    /// Returns true if this affects the untap step.
    fn affects_untap(&self) -> bool {
        false
    }

    /// Returns true if this causes entering tapped.
    fn enters_tapped(&self) -> bool {
        false
    }

    /// Returns true if this is changeling (all creature types).
    fn is_changeling(&self) -> bool {
        false
    }

    /// Returns true if this ability can't be countered.
    fn cant_be_countered(&self) -> bool {
        false
    }

    /// Get level abilities if this is a level-up ability.
    fn level_abilities(&self) -> Option<&[crate::ability::LevelAbility]> {
        None
    }

    /// Get equipment grant abilities if this is an equipment grant.
    fn equipment_grant(&self) -> Option<&[Box<dyn StaticAbilityKind>]> {
        None
    }

    /// Get equipment grant abilities as StaticAbility slice (for convenience).
    fn equipment_grant_abilities(&self) -> Option<&[StaticAbility]> {
        None
    }

    /// Get the grant specification if this ability grants something to cards.
    ///
    /// This is the unified way to check if a static ability grants abilities
    /// or alternative casting methods to cards in non-battlefield zones.
    fn grant_spec(&self) -> Option<crate::grant::GrantSpec> {
        None
    }
}

// Implement Clone for Box<dyn StaticAbilityKind>
impl Clone for Box<dyn StaticAbilityKind> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A wrapper around a boxed StaticAbilityKind trait object.
///
/// This provides a convenient way to work with static abilities as values
/// while maintaining the flexibility of trait objects.
#[derive(Debug, Clone)]
pub struct StaticAbility(pub Box<dyn StaticAbilityKind>);

impl PartialEq for StaticAbility {
    fn eq(&self, other: &Self) -> bool {
        // Compare by ID and display (for abilities with parameters)
        self.0.id() == other.0.id() && self.0.display() == other.0.display()
    }
}

impl StaticAbility {
    /// Create a new StaticAbility from any StaticAbilityKind implementation.
    pub fn new<K: StaticAbilityKind + 'static>(kind: K) -> Self {
        StaticAbility(Box::new(kind))
    }

    /// Get the ability's unique identifier.
    pub fn id(&self) -> StaticAbilityId {
        self.0.id()
    }

    /// Get the display text for this ability.
    pub fn display(&self) -> String {
        self.0.display()
    }

    /// Generate continuous effects for this ability.
    pub fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        self.0.generate_effects(source, controller, game)
    }

    /// Apply game restrictions for this ability.
    pub fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        self.0.apply_restrictions(game, source, controller)
    }

    /// Check if this ability is currently active.
    pub fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        self.0.is_active(game, source)
    }

    /// Generate a replacement effect for this ability.
    pub fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<crate::replacement::ReplacementEffect> {
        self.0.generate_replacement_effect(source, controller)
    }

    // ========================================================================
    // Delegate query methods
    // ========================================================================

    pub fn is_keyword(&self) -> bool {
        self.0.is_keyword()
    }

    pub fn grants_evasion(&self) -> bool {
        self.0.grants_evasion()
    }

    pub fn is_unblockable(&self) -> bool {
        self.0.is_unblockable()
    }

    pub fn has_first_strike(&self) -> bool {
        self.0.has_first_strike()
    }

    pub fn has_double_strike(&self) -> bool {
        self.0.has_double_strike()
    }

    pub fn has_deathtouch(&self) -> bool {
        self.0.has_deathtouch()
    }

    pub fn has_lifelink(&self) -> bool {
        self.0.has_lifelink()
    }

    pub fn has_trample(&self) -> bool {
        self.0.has_trample()
    }

    pub fn has_vigilance(&self) -> bool {
        self.0.has_vigilance()
    }

    pub fn has_haste(&self) -> bool {
        self.0.has_haste()
    }

    pub fn has_flash(&self) -> bool {
        self.0.has_flash()
    }

    pub fn has_reach(&self) -> bool {
        self.0.has_reach()
    }

    pub fn has_defender(&self) -> bool {
        self.0.has_defender()
    }

    pub fn has_indestructible(&self) -> bool {
        self.0.has_indestructible()
    }

    pub fn has_hexproof(&self) -> bool {
        self.0.has_hexproof()
    }

    pub fn hexproof_from_filter(&self) -> Option<&crate::target::ObjectFilter> {
        self.0.hexproof_from_filter()
    }

    pub fn has_shroud(&self) -> bool {
        self.0.has_shroud()
    }

    pub fn has_menace(&self) -> bool {
        self.0.has_menace()
    }

    pub fn has_flying(&self) -> bool {
        self.0.has_flying()
    }

    pub fn has_protection(&self) -> bool {
        self.0.has_protection()
    }

    pub fn protection_from(&self) -> Option<&crate::ability::ProtectionFrom> {
        self.0.protection_from()
    }

    pub fn ward_cost(&self) -> Option<&crate::cost::TotalCost> {
        self.0.ward_cost()
    }

    pub fn cost_reduction(&self) -> Option<&CostReduction> {
        self.0.cost_reduction()
    }

    pub fn cost_increase(&self) -> Option<&CostIncrease> {
        self.0.cost_increase()
    }

    pub fn is_anthem(&self) -> bool {
        self.0.is_anthem()
    }

    pub fn grants_abilities(&self) -> bool {
        self.0.grants_abilities()
    }

    pub fn modifies_costs(&self) -> bool {
        self.0.modifies_costs()
    }

    pub fn has_affinity(&self) -> bool {
        self.0.has_affinity()
    }

    pub fn has_delve(&self) -> bool {
        self.0.has_delve()
    }

    pub fn has_convoke(&self) -> bool {
        self.0.has_convoke()
    }

    pub fn has_improvise(&self) -> bool {
        self.0.has_improvise()
    }

    pub fn affects_untap(&self) -> bool {
        self.0.affects_untap()
    }

    pub fn enters_tapped(&self) -> bool {
        self.0.enters_tapped()
    }

    pub fn is_changeling(&self) -> bool {
        self.0.is_changeling()
    }

    pub fn cant_be_countered(&self) -> bool {
        self.0.cant_be_countered()
    }

    pub fn level_abilities(&self) -> Option<&[crate::ability::LevelAbility]> {
        self.0.level_abilities()
    }

    pub fn equipment_grant_abilities(&self) -> Option<&[StaticAbility]> {
        self.0.equipment_grant_abilities()
    }

    /// Get the grant specification if this ability grants something to cards.
    pub fn grant_spec(&self) -> Option<crate::grant::GrantSpec> {
        self.0.grant_spec()
    }

    // ========================================================================
    // Convenience constructors for common abilities
    // ========================================================================

    pub fn flying() -> Self {
        Self::new(Flying)
    }

    pub fn first_strike() -> Self {
        Self::new(FirstStrike)
    }

    pub fn double_strike() -> Self {
        Self::new(DoubleStrike)
    }

    pub fn deathtouch() -> Self {
        Self::new(Deathtouch)
    }

    pub fn defender() -> Self {
        Self::new(Defender)
    }

    pub fn flash() -> Self {
        Self::new(Flash)
    }

    pub fn haste() -> Self {
        Self::new(Haste)
    }

    pub fn hexproof() -> Self {
        Self::new(Hexproof)
    }

    pub fn indestructible() -> Self {
        Self::new(Indestructible)
    }

    pub fn lifelink() -> Self {
        Self::new(Lifelink)
    }

    pub fn menace() -> Self {
        Self::new(Menace)
    }

    pub fn reach() -> Self {
        Self::new(Reach)
    }

    pub fn shroud() -> Self {
        Self::new(Shroud)
    }

    pub fn trample() -> Self {
        Self::new(Trample)
    }

    pub fn vigilance() -> Self {
        Self::new(Vigilance)
    }

    pub fn fear() -> Self {
        Self::new(Fear)
    }

    pub fn intimidate() -> Self {
        Self::new(Intimidate)
    }

    pub fn shadow() -> Self {
        Self::new(Shadow)
    }

    pub fn horsemanship() -> Self {
        Self::new(Horsemanship)
    }

    pub fn flanking() -> Self {
        Self::new(Flanking)
    }

    pub fn phasing() -> Self {
        Self::new(Phasing)
    }

    pub fn wither() -> Self {
        Self::new(Wither)
    }

    pub fn infect() -> Self {
        Self::new(Infect)
    }

    pub fn changeling() -> Self {
        Self::new(Changeling)
    }

    pub fn protection(from: crate::ability::ProtectionFrom) -> Self {
        Self::new(Protection::new(from))
    }

    pub fn ward(cost: crate::cost::TotalCost) -> Self {
        Self::new(Ward::new(cost))
    }

    pub fn hexproof_from(filter: crate::target::ObjectFilter) -> Self {
        Self::new(HexproofFrom::new(filter))
    }

    pub fn unblockable() -> Self {
        Self::new(Unblockable)
    }

    pub fn cant_attack() -> Self {
        Self::new(CantAttack)
    }

    pub fn cant_block() -> Self {
        Self::new(CantBlock)
    }

    pub fn must_attack() -> Self {
        Self::new(MustAttack)
    }

    pub fn must_block() -> Self {
        Self::new(MustBlock)
    }

    pub fn flying_restriction() -> Self {
        Self::new(FlyingRestriction)
    }

    pub fn can_block_flying() -> Self {
        Self::new(CanBlockFlying)
    }

    pub fn can_attack_as_though_no_defender() -> Self {
        Self::new(CanAttackAsThoughNoDefender)
    }

    pub fn doesnt_untap() -> Self {
        Self::new(DoesntUntap)
    }

    pub fn enters_tapped_ability() -> Self {
        Self::new(EntersTapped)
    }

    pub fn permanents_enter_tapped() -> Self {
        Self::new(AllPermanentsEnterTapped)
    }

    pub fn enters_tapped_for_filter(filter: crate::target::ObjectFilter) -> Self {
        Self::new(EnterTappedForFilter::new(filter))
    }

    pub fn enters_with_counters_for_filter(
        filter: crate::target::ObjectFilter,
        counter_type: crate::object::CounterType,
        count: u32,
    ) -> Self {
        Self::new(EnterWithCountersForFilter::new(filter, counter_type, count))
    }

    pub fn anthem(filter: crate::target::ObjectFilter, power: i32, toughness: i32) -> Self {
        Self::new(Anthem::new(filter, power, toughness))
    }

    pub fn grant_ability(filter: crate::target::ObjectFilter, ability: StaticAbility) -> Self {
        Self::new(GrantAbility::new(filter, ability))
    }

    pub fn remove_ability(filter: crate::target::ObjectFilter, ability: StaticAbility) -> Self {
        Self::new(RemoveAbilityForFilter::new(filter, ability))
    }

    pub fn remove_all_abilities(filter: crate::target::ObjectFilter) -> Self {
        Self::new(RemoveAllAbilitiesForFilter::new(filter))
    }

    pub fn set_base_power_toughness(
        filter: crate::target::ObjectFilter,
        power: i32,
        toughness: i32,
    ) -> Self {
        Self::new(SetBasePowerToughnessForFilter::new(
            filter, power, toughness,
        ))
    }

    pub fn set_colors(filter: crate::target::ObjectFilter, colors: crate::color::ColorSet) -> Self {
        Self::new(SetColorsForFilter::new(filter, colors))
    }

    pub fn add_card_types(
        filter: crate::target::ObjectFilter,
        card_types: Vec<crate::types::CardType>,
    ) -> Self {
        Self::new(AddCardTypesForFilter::new(filter, card_types))
    }

    pub fn make_colorless(filter: crate::target::ObjectFilter) -> Self {
        Self::new(MakeColorlessForFilter::new(filter))
    }

    pub fn remove_supertypes(
        filter: crate::target::ObjectFilter,
        supertypes: Vec<crate::types::Supertype>,
    ) -> Self {
        Self::new(RemoveSupertypesForFilter::new(filter, supertypes))
    }

    pub fn equipment_grant(abilities: Vec<StaticAbility>) -> Self {
        Self::new(EquipmentGrant::new(abilities))
    }

    pub fn copy_activated_abilities(ability: CopyActivatedAbilities) -> Self {
        Self::new(ability)
    }

    pub fn attached_ability_grant(ability: crate::ability::Ability, display: String) -> Self {
        Self::new(AttachedAbilityGrant::new(ability, display))
    }

    pub fn spend_mana_as_any_color_players() -> Self {
        Self::new(SpendManaAsAnyColor)
    }

    pub fn spend_mana_as_any_color_activation_costs() -> Self {
        Self::new(SpendManaAsAnyColorForSourceActivation)
    }

    pub fn with_level_abilities(levels: Vec<crate::ability::LevelAbility>) -> Self {
        Self::new(LevelAbilities::new(levels))
    }

    pub fn may_assign_damage_as_unblocked() -> Self {
        Self::new(MayAssignDamageAsUnblocked)
    }

    pub fn shuffle_into_library_from_graveyard() -> Self {
        Self::new(ShuffleIntoLibraryFromGraveyard)
    }

    pub fn affinity_for_artifacts() -> Self {
        Self::new(AffinityForArtifacts)
    }

    pub fn delve() -> Self {
        Self::new(Delve)
    }

    pub fn convoke() -> Self {
        Self::new(Convoke)
    }

    pub fn improvise() -> Self {
        Self::new(Improvise)
    }

    pub fn blood_moon() -> Self {
        Self::new(BloodMoon)
    }

    pub fn no_maximum_hand_size() -> Self {
        Self::new(NoMaximumHandSize)
    }

    pub fn players_cant_cycle() -> Self {
        Self::new(PlayersCantCycle)
    }

    pub fn players_skip_upkeep() -> Self {
        Self::new(PlayersSkipUpkeep)
    }

    pub fn starting_life_bonus(amount: i32) -> Self {
        Self::new(StartingLifeBonus::new(amount))
    }

    pub fn buyback_cost_reduction(amount: u32) -> Self {
        Self::new(BuybackCostReduction::new(amount))
    }

    pub fn legend_rule_doesnt_apply() -> Self {
        Self::new(LegendRuleDoesntApply)
    }

    pub fn additional_land_play() -> Self {
        Self::new(AdditionalLandPlay)
    }

    pub fn library_of_leng_discard_replacement() -> Self {
        Self::new(LibraryOfLengDiscardReplacement)
    }

    pub fn players_cant_gain_life() -> Self {
        Self::new(PlayersCantGainLife)
    }

    pub fn players_cant_search() -> Self {
        Self::new(PlayersCantSearch)
    }

    pub fn damage_cant_be_prevented() -> Self {
        Self::new(DamageCantBePrevented)
    }

    pub fn you_cant_lose_game() -> Self {
        Self::new(YouCantLoseGame)
    }

    pub fn opponents_cant_win_game() -> Self {
        Self::new(OpponentsCantWinGame)
    }

    pub fn your_life_total_cant_change() -> Self {
        Self::new(YourLifeTotalCantChange)
    }

    pub fn opponents_cant_cast_spells() -> Self {
        Self::new(OpponentsCantCastSpells)
    }

    pub fn opponents_cant_draw_extra_cards() -> Self {
        Self::new(OpponentsCantDrawExtraCards)
    }

    pub fn cant_have_counters_placed() -> Self {
        Self::new(CantHaveCountersPlaced)
    }

    pub fn permanents_you_control_cant_be_sacrificed() -> Self {
        Self::new(PermanentsCantBeSacrificed)
    }

    pub fn can_be_commander() -> Self {
        Self::new(CanBeCommander)
    }

    pub fn uncounterable() -> Self {
        Self::new(CantBeCountered)
    }

    pub fn characteristic_defining_pt(
        power: crate::effect::Value,
        toughness: crate::effect::Value,
    ) -> Self {
        Self::new(CharacteristicDefiningPT::new(power, toughness))
    }

    /// Create a discard-or-redirect ETB replacement ability.
    ///
    /// Used by Mox Diamond: "If Mox Diamond would enter the battlefield, you may discard
    /// a land card instead. If you do, put Mox Diamond onto the battlefield. If you don't,
    /// put it into its owner's graveyard."
    pub fn discard_or_redirect_replacement(
        filter: crate::target::ObjectFilter,
        redirect_zone: crate::zone::Zone,
    ) -> Self {
        Self::new(DiscardOrRedirectReplacement::new(filter, redirect_zone))
    }

    /// Create a pay-life-or-enter-tapped ETB replacement ability.
    ///
    /// Used by shock lands (Godless Shrine, etc.): "As ~ enters the battlefield,
    /// you may pay 2 life. If you don't, it enters the battlefield tapped."
    pub fn pay_life_or_enter_tapped(life_cost: u32) -> Self {
        Self::new(PayLifeOrEnterTappedReplacement::new(life_cost))
    }

    pub fn custom(id: &'static str, description: String) -> Self {
        Self::new(Custom::new(id, description))
    }

    pub fn cant_be_countered_ability() -> Self {
        Self::new(CantBeCountered)
    }

    /// Create a unified grant ability from a grant specification.
    ///
    /// This is the preferred way to create abilities that grant things to cards
    /// in non-battlefield zones (like granting flash to cards in hand, or
    /// granting escape to cards in graveyard).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Grant flash to noncreature spells in hand
    /// StaticAbility::grants(GrantSpec::flash_to_noncreature_spells())
    ///
    /// // Grant escape to nonland cards in graveyard
    /// StaticAbility::grants(GrantSpec::escape_to_nonland(3))
    /// ```
    pub fn grants(spec: crate::grant::GrantSpec) -> Self {
        Self::new(Grants::new(spec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_ability_equality() {
        let flying1 = StaticAbility::flying();
        let flying2 = StaticAbility::flying();
        let trample = StaticAbility::trample();

        assert_eq!(flying1, flying2);
        assert_ne!(flying1, trample);
    }

    #[test]
    fn test_static_ability_id() {
        let flying = StaticAbility::flying();
        assert_eq!(flying.id(), StaticAbilityId::Flying);

        let trample = StaticAbility::trample();
        assert_eq!(trample.id(), StaticAbilityId::Trample);
    }

    #[test]
    fn test_keyword_query_methods() {
        let flying = StaticAbility::flying();
        assert!(flying.is_keyword());
        assert!(flying.has_flying());
        assert!(flying.grants_evasion());
        assert!(!flying.has_trample());

        let trample = StaticAbility::trample();
        assert!(trample.is_keyword());
        assert!(trample.has_trample());
        assert!(!trample.has_flying());
    }

    #[test]
    fn test_static_ability_clone() {
        let flying = StaticAbility::flying();
        let cloned = flying.clone();
        assert_eq!(flying, cloned);
    }

    #[test]
    fn test_display() {
        assert_eq!(StaticAbility::flying().display(), "Flying");
        assert_eq!(StaticAbility::trample().display(), "Trample");
        assert_eq!(StaticAbility::deathtouch().display(), "Deathtouch");
    }
}
