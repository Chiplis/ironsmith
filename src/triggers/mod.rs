//! Modular trigger system for MTG.
//!
//! This module provides a trait-based architecture for trigger matching.
//! Each trigger type implements the `TriggerMatcher` trait, allowing for:
//! - Co-located tests with each trigger implementation
//! - Self-contained matching logic
//! - Easy addition of new triggers without modifying central dispatcher
//!
//! # Module Structure
//!
//! ```text
//! triggers/
//!   mod.rs              - This file, module organization and Trigger wrapper
//!   matcher_trait.rs    - TriggerMatcher trait and TriggerContext
//!   event.rs            - GameEvent enum and related types
//!   zone_changes/       - ETB, dies, LTB triggers
//!   phase_step/         - Upkeep, draw step, end step triggers
//!   combat/             - Attack, block, damage triggers
//!   life_damage/        - Life gain/loss, damage triggers
//!   spell_ability/      - Spell cast, ability activation triggers
//!   cards/              - Draw, discard triggers
//!   counters/           - Counter placement triggers
//!   other/              - Tap, untap, sacrifice triggers
//!   special/            - Undying, persist, custom triggers
//! ```
//!
//! # Usage
//!
//! Triggers can be created using factory methods on the `Trigger` struct:
//!
//! ```ignore
//! use ironsmith::triggers::Trigger;
//!
//! // Create a "dies" trigger for any creature
//! let trigger = Trigger::dies(ObjectFilter::creature());
//!
//! // Create a "this enters the battlefield" trigger
//! let trigger = Trigger::this_enters_battlefield();
//!
//! // Check if a trigger matches an event
//! let matches = trigger.matches(&event, &ctx);
//! ```

pub mod check;
pub mod event;
pub mod matcher_trait;
pub mod trigger_event;

// Trigger category submodules
pub mod cards;
pub mod combat;
pub mod counters;
pub mod life_damage;
pub mod other;
pub mod phase_step;
pub mod special;
pub mod spell_ability;
pub mod zone_changes;

// Re-export core types
pub use check::{
    DelayedTrigger, TriggerIdentity, TriggerQueue, TriggeredAbilityEntry, check_delayed_triggers,
    check_triggers, compute_delayed_trigger_identity, compute_trigger_identity,
    generate_step_trigger_events, player_filter_matches_with_context, verify_intervening_if,
};
pub use event::{AttackEventTarget, DamageEventTarget};
pub use matcher_trait::{TriggerContext, TriggerMatcher};
pub use trigger_event::TriggerEvent;

// Re-export trigger implementations from submodules
pub use cards::*;
pub use combat::*;
pub use counters::*;
pub use life_damage::*;
pub use other::*;
pub use phase_step::*;
pub use special::*;
pub use spell_ability::*;
pub use zone_changes::*;

use crate::events::EventKind;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::zone::Zone;

/// Wrapper around a boxed TriggerMatcher for ergonomic usage.
///
/// This struct provides factory methods for creating common trigger types
/// and implements the TriggerMatcher trait by delegating to the inner matcher.
#[derive(Debug)]
pub struct Trigger(pub Box<dyn TriggerMatcher>);

impl Clone for Trigger {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

impl PartialEq for Trigger {
    fn eq(&self, other: &Self) -> bool {
        self.display() == other.display()
    }
}

impl Trigger {
    /// Create a new Trigger wrapping a TriggerMatcher implementation.
    pub fn new<T: TriggerMatcher + 'static>(matcher: T) -> Self {
        Self(Box::new(matcher))
    }

    /// Check if this trigger matches a game event.
    pub fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.0.matches(event, ctx)
    }

    /// Get the display text for this trigger.
    pub fn display(&self) -> String {
        self.0.display()
    }

    /// Whether this trigger uses snapshot-based matching.
    pub fn uses_snapshot(&self) -> bool {
        self.0.uses_snapshot()
    }

    /// Saga chapter numbers for saga chapter triggers.
    pub fn saga_chapters(&self) -> Option<&[u32]> {
        self.0.saga_chapters()
    }

    // === Zone Change Triggers ===

    /// Create a "when this permanent enters the battlefield" trigger.
    pub fn this_enters_battlefield() -> Self {
        Self::new(ZoneChangeTrigger::this_enters_battlefield())
    }

    /// Create a "when [filter] enters the battlefield" trigger.
    pub fn enters_battlefield(filter: ObjectFilter) -> Self {
        Self::new(ZoneChangeTrigger::enters_battlefield(filter))
    }

    /// Create a "when [filter] enters the battlefield tapped" trigger.
    pub fn enters_battlefield_tapped(filter: ObjectFilter) -> Self {
        Self::new(EntersBattlefieldTappedTrigger::new(filter))
    }

    /// Create a "when [filter] enters the battlefield untapped" trigger.
    pub fn enters_battlefield_untapped(filter: ObjectFilter) -> Self {
        Self::new(EntersBattlefieldUntappedTrigger::new(filter))
    }

    /// Create a "when this creature dies" trigger.
    pub fn this_dies() -> Self {
        Self::new(ZoneChangeTrigger::this_dies())
    }

    /// Create a "when this permanent dies or is exiled" trigger.
    pub fn this_dies_or_is_exiled() -> Self {
        let dies = Trigger::new(ZoneChangeTrigger::this_dies());
        let exiled = Trigger::new(
            ZoneChangeTrigger::new()
                .from(Zone::Battlefield)
                .to(Zone::Exile)
                .this(),
        );
        Self::new(OrTrigger::new(vec![dies, exiled]))
    }

    /// Create a "when [filter] dies" trigger.
    pub fn dies(filter: ObjectFilter) -> Self {
        Self::new(ZoneChangeTrigger::dies(filter))
    }

    /// Create a "when this permanent leaves the battlefield" trigger.
    pub fn this_leaves_battlefield() -> Self {
        Self::new(ZoneChangeTrigger::this_leaves_battlefield())
    }

    /// Create a "when [filter] leaves the battlefield" trigger.
    pub fn leaves_battlefield(filter: ObjectFilter) -> Self {
        Self::new(ZoneChangeTrigger::leaves_battlefield(filter))
    }

    /// Create a "when [filter] is put into a graveyard from anywhere" trigger.
    pub fn put_into_graveyard(filter: ObjectFilter) -> Self {
        Self::new(ZoneChangeTrigger::new().to(Zone::Graveyard).filter(filter))
    }

    /// Create a "when [filter] is exiled" trigger.
    pub fn exiled(filter: ObjectFilter) -> Self {
        Self::new(ZoneChangeTrigger::new().to(Zone::Exile).filter(filter))
    }

    /// Create a "when a card is put into your hand" trigger.
    pub fn card_put_into_hand() -> Self {
        Self::new(
            ZoneChangeTrigger::new()
                .to(Zone::Hand)
                .player(PlayerRelation::You),
        )
    }

    // === Phase/Step Triggers ===

    /// Create a "at the beginning of [player]'s upkeep" trigger.
    pub fn beginning_of_upkeep(player: PlayerFilter) -> Self {
        Self::new(BeginningOfUpkeepTrigger::new(player))
    }

    /// Create a "at the beginning of [player]'s draw step" trigger.
    pub fn beginning_of_draw_step(player: PlayerFilter) -> Self {
        Self::new(BeginningOfDrawStepTrigger::new(player))
    }

    /// Create a "at the beginning of [player]'s end step" trigger.
    pub fn beginning_of_end_step(player: PlayerFilter) -> Self {
        Self::new(BeginningOfEndStepTrigger::new(player))
    }

    /// Create a "at the beginning of combat on [player]'s turn" trigger.
    pub fn beginning_of_combat(player: PlayerFilter) -> Self {
        Self::new(BeginningOfCombatTrigger::new(player))
    }

    /// Create a "at end of combat" trigger.
    pub fn end_of_combat() -> Self {
        Self::new(EndOfCombatTrigger)
    }

    /// Create a "at the beginning of [player]'s precombat main phase" trigger.
    pub fn beginning_of_precombat_main_phase(player: PlayerFilter) -> Self {
        Self::new(BeginningOfMainPhaseTrigger::new(
            player,
            MainPhaseType::Precombat,
        ))
    }

    /// Create a "at the beginning of [player]'s postcombat main phase" trigger.
    pub fn beginning_of_postcombat_main_phase(player: PlayerFilter) -> Self {
        Self::new(BeginningOfMainPhaseTrigger::new(
            player,
            MainPhaseType::Postcombat,
        ))
    }

    /// Create a "at the beginning of [player]'s main phase" trigger (either).
    pub fn beginning_of_main_phase(player: PlayerFilter) -> Self {
        Self::new(BeginningOfMainPhaseTrigger::new(
            player,
            MainPhaseType::Precombat,
        ))
    }

    // === Combat Triggers ===

    /// Create a "when this creature attacks" trigger.
    pub fn this_attacks() -> Self {
        Self::new(ThisAttacksTrigger)
    }

    /// Create a "when [filter] attacks" trigger.
    pub fn attacks(filter: ObjectFilter) -> Self {
        Self::new(AttacksTrigger::new(filter))
    }

    /// Create a "when [filter] attacks alone" trigger.
    pub fn attacks_alone(filter: ObjectFilter) -> Self {
        Self::new(AttacksAloneTrigger::new(filter))
    }

    /// Create a "when [filter] attacks you or a planeswalker you control" trigger.
    pub fn attacks_you(filter: ObjectFilter) -> Self {
        Self::new(AttacksYouTrigger::new(filter))
    }

    /// Create a "when this creature blocks" trigger.
    pub fn this_blocks() -> Self {
        Self::new(ThisBlocksTrigger)
    }

    /// Create a "when this creature blocks [filter]" trigger.
    pub fn this_blocks_object(filter: ObjectFilter) -> Self {
        Self::new(ThisBlocksObjectTrigger::new(filter))
    }

    /// Create a "when [filter] blocks" trigger.
    pub fn blocks(filter: ObjectFilter) -> Self {
        Self::new(BlocksTrigger::new(filter))
    }

    /// Create a "when this creature becomes blocked" trigger.
    pub fn this_becomes_blocked() -> Self {
        Self::new(ThisBecomesBlockedTrigger)
    }

    /// Create a "when [filter] becomes blocked" trigger.
    pub fn becomes_blocked(filter: ObjectFilter) -> Self {
        Self::new(BecomesBlockedTrigger::new(filter))
    }

    /// Create a "when [filter] blocks or becomes blocked" trigger.
    pub fn blocks_or_becomes_blocked(filter: ObjectFilter) -> Self {
        Self::new(BlocksOrBecomesBlockedTrigger::new(filter))
    }

    /// Create a "when this creature blocks or becomes blocked" trigger.
    pub fn this_blocks_or_becomes_blocked() -> Self {
        Self::either(Self::this_blocks(), Self::this_becomes_blocked())
    }

    /// Create a "when this creature deals combat damage to a player" trigger.
    pub fn this_deals_combat_damage_to_player() -> Self {
        Self::new(ThisDealsCombatDamageToPlayerTrigger)
    }

    /// Create a "when [filter] deals combat damage to a player" trigger.
    pub fn deals_combat_damage_to_player(filter: ObjectFilter) -> Self {
        Self::new(DealsCombatDamageToPlayerTrigger::new(filter))
    }

    /// Create a "when this permanent deals damage" trigger.
    pub fn this_deals_damage() -> Self {
        Self::new(ThisDealsDamageTrigger)
    }

    /// Create a "when this permanent deals damage to [filter]" trigger.
    pub fn this_deals_damage_to(filter: ObjectFilter) -> Self {
        Self::new(ThisDealsDamageToTrigger::new(filter))
    }

    /// Create a "when [filter] deals damage" trigger.
    pub fn deals_damage(filter: ObjectFilter) -> Self {
        Self::new(DealsDamageTrigger::new(filter))
    }

    // === Life/Damage Triggers ===

    /// Create a "whenever you gain life" trigger.
    pub fn you_gain_life() -> Self {
        Self::new(YouGainLifeTrigger)
    }

    /// Create a "whenever you lose life" trigger.
    pub fn you_lose_life() -> Self {
        Self::new(YouLoseLifeTrigger)
    }

    /// Create a "whenever [player] loses life" trigger.
    pub fn player_loses_life(player: PlayerFilter) -> Self {
        Self::new(PlayerLosesLifeTrigger::new(player))
    }

    /// Create a "when [target] is dealt damage" trigger.
    pub fn is_dealt_damage(target: ChooseSpec) -> Self {
        Self::new(IsDealtDamageTrigger::new(target))
    }

    // === Spell/Ability Triggers ===

    /// Create a "when [player] casts a spell" trigger.
    pub fn spell_cast(filter: Option<ObjectFilter>, caster: PlayerFilter) -> Self {
        Self::new(SpellCastTrigger::new(filter, caster))
    }

    /// Create a "when [player] copies a spell" trigger.
    pub fn spell_copied(filter: Option<ObjectFilter>, copier: PlayerFilter) -> Self {
        Self::new(SpellCopiedTrigger::new(filter, copier))
    }

    /// Create a "when you cast this spell" trigger.
    pub fn you_cast_this_spell() -> Self {
        Self::new(YouCastThisSpellTrigger)
    }

    /// Create a "when [filter] ability is activated" trigger.
    pub fn ability_activated(filter: ObjectFilter) -> Self {
        Self::new(AbilityActivatedTrigger::new(filter))
    }

    /// Create a "when this permanent becomes the target of a spell or ability" trigger.
    pub fn becomes_targeted() -> Self {
        Self::new(BecomesTargetedTrigger)
    }

    // === Card Triggers ===

    /// Create a "whenever you draw a card" trigger (fires once per card drawn).
    pub fn you_draw_card() -> Self {
        Self::new(PlayerDrawsCardTrigger::per_card(PlayerFilter::You))
    }

    /// Create a "whenever you draw one or more cards" trigger (fires once per draw action).
    pub fn you_draw_cards() -> Self {
        Self::new(PlayerDrawsCardTrigger::new(PlayerFilter::You))
    }

    /// Create a "whenever a player draws a card" trigger.
    pub fn player_draws_card(player: PlayerFilter) -> Self {
        Self::new(PlayerDrawsCardTrigger::per_card(player))
    }

    /// Create a "whenever a player draws one or more cards" trigger.
    pub fn player_draws_cards(player: PlayerFilter) -> Self {
        Self::new(PlayerDrawsCardTrigger::new(player))
    }

    /// Create a "whenever you discard a card" trigger.
    pub fn you_discard_card() -> Self {
        Self::new(YouDiscardCardTrigger)
    }

    /// Create a "whenever a card is put into your graveyard" trigger.
    pub fn card_put_into_your_graveyard() -> Self {
        Self::new(CardPutIntoYourGraveyardTrigger)
    }

    // === Counter Triggers ===

    /// Create a "when a counter is put on [filter]" trigger.
    pub fn counter_put_on(filter: ObjectFilter) -> Self {
        Self::new(CounterPutOnTrigger::new(filter))
    }

    /// Create a "when a counter is removed from [filter]" trigger.
    pub fn counter_removed_from(filter: ObjectFilter) -> Self {
        Self::new(CounterRemovedFromTrigger::new(filter))
    }

    /// Create a saga chapter trigger for specific chapters.
    pub fn saga_chapter(chapters: Vec<u32>) -> Self {
        Self::new(SagaChapterTrigger::new(chapters))
    }

    // === Other Triggers ===

    /// Create a "when this permanent becomes tapped" trigger.
    pub fn becomes_tapped() -> Self {
        Self::new(BecomesTappedTrigger)
    }

    /// Create a "when this permanent becomes untapped" trigger.
    pub fn becomes_untapped() -> Self {
        Self::new(BecomesUntappedTrigger)
    }

    /// Create a "when [filter] becomes tapped" trigger.
    pub fn permanent_becomes_tapped(filter: ObjectFilter) -> Self {
        Self::new(PermanentBecomesTappedTrigger::new(filter))
    }

    /// Create a "when a player sacrifices [filter]" trigger.
    pub fn player_sacrifices(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self::new(PlayerSacrificesTrigger::new(player, filter))
    }

    /// Create a "at the beginning of each player's turn" trigger.
    pub fn each_players_turn() -> Self {
        Self::new(EachPlayersTurnTrigger)
    }

    /// Create a "when this permanent transforms" trigger.
    pub fn transforms() -> Self {
        Self::new(TransformsTrigger)
    }

    /// Create a "when this creature becomes monstrous" trigger.
    pub fn this_becomes_monstrous() -> Self {
        Self::new(ThisEventObjectTrigger::new(
            EventKind::BecameMonstrous,
            "When this creature becomes monstrous",
        ))
    }

    /// Create a "when this permanent is turned face up" trigger.
    pub fn this_is_turned_face_up() -> Self {
        Self::new(ThisEventObjectTrigger::new(
            EventKind::TurnedFaceUp,
            "When this permanent is turned face up",
        ))
    }

    /// Create a "whenever players finish voting" trigger.
    ///
    /// This is represented as a keyword-action trigger on "vote".
    pub fn players_finish_voting() -> Self {
        Self::keyword_action(crate::events::KeywordActionKind::Vote, PlayerFilter::Any)
    }

    /// Create a "whenever [player] [keyword action]" trigger.
    pub fn keyword_action(action: crate::events::KeywordActionKind, player: PlayerFilter) -> Self {
        Self::new(KeywordActionTrigger::new(action, player))
    }

    // === Special Triggers ===

    /// Create an undying trigger.
    pub fn undying() -> Self {
        Self::new(KeywordAbilityTrigger::undying())
    }

    /// Create a persist trigger.
    pub fn persist() -> Self {
        Self::new(KeywordAbilityTrigger::persist())
    }

    /// Create a miracle trigger.
    ///
    /// Miracle triggers when this card is drawn as the first card of the turn.
    pub fn miracle() -> Self {
        Self::new(KeywordAbilityTrigger::miracle())
    }

    /// Create a custom trigger with a unique ID and description.
    pub fn custom(id: &'static str, description: String) -> Self {
        Self::new(CustomTrigger::new(id, description))
    }

    // === Trigger Combinators ===

    /// Create an "or" trigger that matches if any of the inner triggers match.
    ///
    /// This is useful for cards like Tivit which trigger on multiple conditions:
    /// "Whenever Tivit enters the battlefield or deals combat damage to a player"
    ///
    /// # Example
    ///
    /// ```ignore
    /// let trigger = Trigger::or(vec![
    ///     Trigger::this_enters_battlefield(),
    ///     Trigger::this_deals_combat_damage_to_player(),
    /// ]);
    /// ```
    pub fn or(triggers: Vec<Self>) -> Self {
        Self::new(OrTrigger::new(triggers))
    }

    /// Create an "or" trigger from exactly two triggers.
    ///
    /// Convenience method for the common case of combining two triggers.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let trigger = Trigger::either(
    ///     Trigger::this_enters_battlefield(),
    ///     Trigger::this_deals_combat_damage_to_player(),
    /// );
    /// ```
    pub fn either(a: Self, b: Self) -> Self {
        Self::new(OrTrigger::two(a, b))
    }
}

impl TriggerMatcher for Trigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.0.matches(event, ctx)
    }

    fn display(&self) -> String {
        self.0.display()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }

    fn uses_snapshot(&self) -> bool {
        self.0.uses_snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_wrapper_this_enters_battlefield() {
        let trigger = Trigger::this_enters_battlefield();
        assert!(trigger.display().contains("enters"));
    }

    #[test]
    fn test_trigger_wrapper_dies() {
        let trigger = Trigger::dies(ObjectFilter::creature());
        assert!(trigger.display().contains("dies"));
        assert!(trigger.uses_snapshot());
    }

    #[test]
    fn test_trigger_wrapper_this_blocks_or_becomes_blocked() {
        let trigger = Trigger::this_blocks_or_becomes_blocked();
        assert!(trigger.display().contains("blocks"));
    }

    #[test]
    fn test_trigger_wrapper_clone() {
        let trigger = Trigger::this_enters_battlefield();
        let cloned = trigger.clone();
        assert_eq!(trigger.display(), cloned.display());
    }

    #[test]
    fn test_trigger_as_trait_object() {
        let trigger: Box<dyn TriggerMatcher> = Box::new(Trigger::this_enters_battlefield());
        assert!(trigger.display().contains("enters"));
    }
}
