//! Miscellaneous static abilities.
//!
//! This module contains static abilities that don't fit neatly into other categories.

use super::{StaticAbilityId, StaticAbilityKind};
use crate::ability::LevelAbility;
use crate::effect::Value;
use crate::events::cards::matchers::WouldDiscardMatcher;
use crate::events::zones::matchers::{
    ThisWouldEnterBattlefieldMatcher, ThisWouldGoToGraveyardMatcher, WouldEnterBattlefieldMatcher,
};
use crate::game_state::GameState;
use crate::grant::GrantSpec;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::ObjectFilter;
use crate::zone::Zone;

/// Doesn't untap during your untap step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DoesntUntap;

impl StaticAbilityKind for DoesntUntap {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DoesntUntap
    }

    fn display(&self) -> String {
        "Doesn't untap during your untap step".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn affects_untap(&self) -> bool {
        true
    }
}

/// Enters the battlefield tapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntersTapped;

impl StaticAbilityKind for EntersTapped {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EntersTapped
    }

    fn display(&self) -> String {
        "Enters the battlefield tapped".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn enters_tapped(&self) -> bool {
        true
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::EnterTapped,
            )
            .self_replacing(),
        )
    }
}

/// If this would be put into a graveyard from anywhere, shuffle into library instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShuffleIntoLibraryFromGraveyard;

impl StaticAbilityKind for ShuffleIntoLibraryFromGraveyard {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ShuffleIntoLibraryFromGraveyard
    }

    fn display(&self) -> String {
        "If this would be put into a graveyard from anywhere, shuffle it into its owner's library instead".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldGoToGraveyardMatcher,
                ReplacementAction::ChangeDestination(crate::zone::Zone::Library),
            )
            .self_replacing(),
        )
    }
}

/// All permanents enter the battlefield tapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllPermanentsEnterTapped;

impl StaticAbilityKind for AllPermanentsEnterTapped {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AllPermanentsEnterTapped
    }

    fn display(&self) -> String {
        "Permanents enter the battlefield tapped".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldEnterBattlefieldMatcher::any(),
            ReplacementAction::EnterTapped,
        ))
    }
}

/// Players may spend mana as though it were mana of any color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpendManaAsAnyColor;

impl StaticAbilityKind for SpendManaAsAnyColor {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SpendManaAsAnyColor
    }

    fn display(&self) -> String {
        "Players may spend mana as though it were mana of any color".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, _controller: PlayerId) {
        for player in &game.players {
            if player.is_in_game() {
                game.mana_spend_effects.any_color_players.insert(player.id);
            }
        }
    }
}

/// You may spend mana as though it were mana of any color to pay activation costs of this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpendManaAsAnyColorForSourceActivation;

impl StaticAbilityKind for SpendManaAsAnyColorForSourceActivation {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SpendManaAsAnyColorActivationCosts
    }

    fn display(&self) -> String {
        "You may spend mana as though it were mana of any color to pay activation costs of this"
            .to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        game.mana_spend_effects
            .any_color_activation_sources
            .insert(source);
    }
}

/// Permanents matching a filter enter the battlefield tapped.
#[derive(Debug, Clone, PartialEq)]
pub struct EnterTappedForFilter {
    pub filter: ObjectFilter,
}

impl EnterTappedForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for EnterTappedForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterTappedForFilter
    }

    fn display(&self) -> String {
        "Permanents enter the battlefield tapped".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldEnterBattlefieldMatcher::new(self.filter.clone()),
            ReplacementAction::EnterTapped,
        ))
    }
}

/// Permanents matching a filter enter the battlefield with counters.
#[derive(Debug, Clone, PartialEq)]
pub struct EnterWithCountersForFilter {
    pub filter: ObjectFilter,
    pub counter_type: CounterType,
    pub count: u32,
}

impl EnterWithCountersForFilter {
    pub fn new(filter: ObjectFilter, counter_type: CounterType, count: u32) -> Self {
        Self {
            filter,
            counter_type,
            count,
        }
    }
}

impl StaticAbilityKind for EnterWithCountersForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterWithCountersForFilter
    }

    fn display(&self) -> String {
        "Permanents enter the battlefield with counters".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldEnterBattlefieldMatcher::new(self.filter.clone()),
            ReplacementAction::EnterWithCounters {
                counter_type: self.counter_type,
                count: Value::Fixed(self.count as i32),
            },
        ))
    }
}

/// Players can't cycle cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayersCantCycle;

impl StaticAbilityKind for PlayersCantCycle {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PlayersCantCycle
    }

    fn display(&self) -> String {
        "Players can't cycle cards".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

/// Start the game with an additional amount of life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartingLifeBonus {
    pub amount: i32,
}

impl StartingLifeBonus {
    pub fn new(amount: i32) -> Self {
        Self { amount }
    }
}

impl StaticAbilityKind for StartingLifeBonus {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::StartingLifeBonus
    }

    fn display(&self) -> String {
        format!("You start the game with an additional {} life", self.amount)
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

/// Buyback costs cost less (placeholder ability).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuybackCostReduction {
    pub amount: u32,
}

impl BuybackCostReduction {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl StaticAbilityKind for BuybackCostReduction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::BuybackCostReduction
    }

    fn display(&self) -> String {
        format!("Buyback costs cost {{{}}} less", self.amount)
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

/// Players skip their upkeep steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayersSkipUpkeep;

impl StaticAbilityKind for PlayersSkipUpkeep {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PlayersSkipUpkeep
    }

    fn display(&self) -> String {
        "Players skip their upkeep steps".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

/// The legend rule doesn't apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LegendRuleDoesntApply;

impl StaticAbilityKind for LegendRuleDoesntApply {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LegendRuleDoesntApply
    }

    fn display(&self) -> String {
        "The legend rule doesn't apply".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

/// You may play an additional land on each of your turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdditionalLandPlay;

impl StaticAbilityKind for AdditionalLandPlay {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AdditionalLandPlay
    }

    fn display(&self) -> String {
        "You may play an additional land on each of your turns".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

/// Can be your commander.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CanBeCommander;

impl StaticAbilityKind for CanBeCommander {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CanBeCommander
    }

    fn display(&self) -> String {
        "Can be your commander".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

// =============================================================================
// Unified Grant System
// =============================================================================

/// Unified grant ability that grants abilities or alternative casting methods
/// to cards matching a filter in a specific zone.
///
/// This is the generic version that replaces bespoke types like `GrantEscape`
/// and `GrantFlashToNoncreatureSpells`. It provides a uniform way to express
/// "cards matching X in zone Y have Z".
///
/// # Examples
///
/// ```ignore
/// // Valley Floodcaller: "You may cast noncreature spells as though they had flash."
/// StaticAbility::grants(GrantSpec::flash_to_noncreature_spells())
///
/// // Underworld Breach: "Each nonland card in your graveyard has escape."
/// StaticAbility::grants(GrantSpec::escape_to_nonland(3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Grants {
    pub spec: GrantSpec,
}

impl Grants {
    /// Create a new Grants ability from a grant specification.
    pub fn new(spec: GrantSpec) -> Self {
        Self { spec }
    }
}

impl StaticAbilityKind for Grants {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Grants
    }

    fn display(&self) -> String {
        self.spec.display()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn grant_spec(&self) -> Option<GrantSpec> {
        Some(self.spec.clone())
    }
}

/// Level abilities for level-up creatures.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelAbilities {
    pub levels: Vec<LevelAbility>,
}

impl LevelAbilities {
    pub fn new(levels: Vec<LevelAbility>) -> Self {
        Self { levels }
    }
}

impl StaticAbilityKind for LevelAbilities {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LevelAbilities
    }

    fn display(&self) -> String {
        "Level up abilities".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn level_abilities(&self) -> Option<&[LevelAbility]> {
        Some(&self.levels)
    }
}

/// "You have no maximum hand size"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoMaximumHandSize;

impl StaticAbilityKind for NoMaximumHandSize {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::NoMaximumHandSize
    }

    fn display(&self) -> String {
        "You have no maximum hand size".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }
}

/// Library of Leng's discard replacement effect.
///
/// "If an effect causes you to discard a card, you may put it on top of
/// your library instead of into your graveyard."
///
/// Key rules:
/// - Only applies to discards from effects (not costs)
/// - Uses the composable EventCause system to filter on cause type
/// - Offers an interactive choice between graveyard and library
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LibraryOfLengDiscardReplacement;

impl StaticAbilityKind for LibraryOfLengDiscardReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LibraryOfLengDiscardReplacement
    }

    fn display(&self) -> String {
        "If an effect causes you to discard a card, you may put it on top of your library instead"
            .to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            // Use the composable matcher that filters on cause type
            WouldDiscardMatcher::you_from_effect(),
            ReplacementAction::InteractiveChooseDestination {
                destinations: vec![Zone::Graveyard, Zone::Library],
                description:
                    "Library of Leng: Put discarded card on top of library instead of graveyard?"
                        .to_string(),
            },
        ))
    }
}

// =============================================================================
// Interactive ETB Replacement Abilities (Unified System)
// =============================================================================

/// "You may discard a card matching [filter]. If you don't, put this into [zone]."
///
/// Used by: Mox Diamond (discard land or goes to graveyard)
///
/// This is an interactive replacement effect that uses the unified replacement
/// system rather than the deprecated EtbReplacementHandler trait.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardOrRedirectReplacement {
    /// Filter for cards that can be discarded to satisfy the replacement.
    pub filter: ObjectFilter,
    /// Where the permanent goes if no card is discarded.
    pub redirect_zone: Zone,
}

impl DiscardOrRedirectReplacement {
    /// Create a new discard-or-redirect replacement ability.
    pub fn new(filter: ObjectFilter, redirect_zone: Zone) -> Self {
        Self {
            filter,
            redirect_zone,
        }
    }
}

impl StaticAbilityKind for DiscardOrRedirectReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DiscardOrRedirectReplacement
    }

    fn display(&self) -> String {
        format!(
            "If this would enter the battlefield, you may discard a card. If you don't, put it into {:?}",
            self.redirect_zone
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::InteractiveDiscardOrRedirect {
                    filter: self.filter.clone(),
                    redirect_zone: self.redirect_zone,
                },
            )
            .self_replacing(),
        )
    }
}

/// "As this enters the battlefield, you may pay N life. If you don't, it enters tapped."
///
/// Used by: Shock lands (Godless Shrine, etc.), slow fetches (Vault of Champions, etc.)
///
/// This is an interactive replacement effect that uses the unified replacement
/// system rather than the deprecated EtbReplacementHandler trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayLifeOrEnterTappedReplacement {
    /// The amount of life to pay to enter untapped.
    pub life_cost: u32,
}

impl PayLifeOrEnterTappedReplacement {
    /// Create a new pay-life-or-enter-tapped replacement ability.
    pub fn new(life_cost: u32) -> Self {
        Self { life_cost }
    }
}

impl StaticAbilityKind for PayLifeOrEnterTappedReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PayLifeOrEnterTappedReplacement
    }

    fn display(&self) -> String {
        format!(
            "As this enters the battlefield, you may pay {} life. If you don't, it enters tapped.",
            self.life_cost
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::InteractivePayLifeOrEnterTapped {
                    life_cost: self.life_cost,
                },
            )
            .self_replacing(),
        )
    }

    fn enters_tapped(&self) -> bool {
        // This is conditionally enters tapped, so we return false here
        // The actual tapped state is determined by the replacement effect
        false
    }
}

// =============================================================================
// Custom Abilities
// =============================================================================

/// Custom static ability with a unique ID.
#[derive(Debug, Clone, PartialEq)]
pub struct Custom {
    pub custom_id: &'static str,
    pub description: String,
}

impl Custom {
    pub fn new(id: &'static str, description: String) -> Self {
        Self {
            custom_id: id,
            description,
        }
    }
}

impl StaticAbilityKind for Custom {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Custom
    }

    fn display(&self) -> String {
        self.description.clone()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doesnt_untap() {
        let ability = DoesntUntap;
        assert_eq!(ability.id(), StaticAbilityId::DoesntUntap);
        assert!(ability.affects_untap());
    }

    #[test]
    fn test_enters_tapped() {
        let ability = EntersTapped;
        assert_eq!(ability.id(), StaticAbilityId::EntersTapped);
        assert!(ability.enters_tapped());
    }

    #[test]
    fn test_no_maximum_hand_size() {
        let ability = NoMaximumHandSize;
        assert_eq!(ability.id(), StaticAbilityId::NoMaximumHandSize);
    }

    #[test]
    fn test_custom() {
        let ability = Custom::new("test_ability", "Test description".to_string());
        assert_eq!(ability.id(), StaticAbilityId::Custom);
        assert_eq!(ability.display(), "Test description");
    }
}
