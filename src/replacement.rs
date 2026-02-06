//! Replacement effect system.
//!
//! Replacement effects modify or replace events as they happen.
//! Per MTG rule 614, they use "instead" or "as [event]" or "skip".

use crate::effect::{Effect, Value};
use crate::events::ReplacementMatcher;
use crate::events::cards::matchers::{WouldDiscardMatcher, WouldDrawCardMatcher};
use crate::events::damage::matchers::{DamageFromSourceMatcher, DamageToPlayerMatcher};
use crate::events::life::matchers::WouldGainLifeMatcher;
use crate::events::permanents::matchers::ThisWouldBeDestroyedMatcher;
use crate::events::zones::matchers::{
    ThisWouldDieMatcher, ThisWouldEnterBattlefieldMatcher, WouldEnterBattlefieldMatcher,
};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

/// A replacement effect that modifies events.
#[derive(Debug, Clone)]
pub struct ReplacementEffect {
    /// Unique identifier for this effect
    pub id: ReplacementEffectId,

    /// The source that created this effect
    pub source: ObjectId,

    /// The controller of this effect
    pub controller: PlayerId,

    /// What happens instead
    pub replacement: ReplacementAction,

    /// Whether this is a self-replacement effect (affects only its source)
    pub self_replacement: bool,

    /// Trait-based matcher for checking if this effect applies.
    pub matcher: Option<Box<dyn ReplacementMatcher>>,
}

/// Unique identifier for a replacement effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReplacementEffectId(pub u64);

impl ReplacementEffectId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// What happens instead when a replacement triggers.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplacementAction {
    /// Prevent the event entirely
    Prevent,

    /// Apply the event but modified
    Modify(EventModification),

    /// Do something different instead
    Instead(Vec<Effect>),

    /// Redirect to a different target.
    /// Use `which` to specify which target to redirect for multi-target events.
    Redirect {
        target: RedirectTarget,
        /// Which target to redirect (default: First).
        which: RedirectWhich,
    },

    /// Change the zone an object would go to
    ChangeDestination(Zone),

    /// Enter with additional counters
    EnterWithCounters {
        counter_type: CounterType,
        count: Value,
    },

    /// Enter tapped
    EnterTapped,

    /// Enter as a copy of something
    EnterAsCopy(ObjectId),

    /// Double the effect (e.g., double damage, double counters)
    Double,

    /// Add an additional effect
    Additionally(Vec<Effect>),

    /// Skip (for "skip your draw step" etc.)
    Skip,

    /// Interactive: Discard a card matching filter, or redirect to a different zone.
    ///
    /// Used by Mox Diamond: "If Mox Diamond would enter the battlefield, you may discard
    /// a land card instead. If you do, put Mox Diamond onto the battlefield. If you don't,
    /// put it into its owner's graveyard."
    ///
    /// When this applies:
    /// 1. Check if controller has any cards in hand matching the filter
    /// 2. If no matching cards, automatically redirect to redirect_zone
    /// 3. If matching cards exist, prompt the player to choose one or decline
    /// 4. If player discards a matching card, the permanent enters the battlefield
    /// 5. If player declines, the permanent goes to redirect_zone instead
    InteractiveDiscardOrRedirect {
        /// Filter for cards that can be discarded to satisfy the replacement.
        filter: ObjectFilter,
        /// Where the permanent goes if no card is discarded.
        redirect_zone: Zone,
    },

    /// Interactive: Pay life or enter tapped.
    ///
    /// Used by shock lands (Godless Shrine, etc.): "As ~ enters the battlefield,
    /// you may pay 2 life. If you don't, it enters the battlefield tapped."
    ///
    /// When this applies:
    /// 1. Prompt the player if they want to pay life_cost life
    /// 2. If player pays, the permanent enters untapped
    /// 3. If player declines (or can't pay), the permanent enters tapped
    InteractivePayLifeOrEnterTapped {
        /// The amount of life to pay.
        life_cost: u32,
    },

    /// Interactive: Choose alternate destination for a zone-changing event.
    ///
    /// Used by Library of Leng: "If an effect causes you to discard a card,
    /// you may put it on top of your library instead of into your graveyard."
    ///
    /// When this applies:
    /// 1. Prompt the player with the choice of destinations
    /// 2. If player chooses the alternate destination, modify the event
    /// 3. If player declines, the event proceeds with its original destination
    ///
    /// This is a generic version that could work for various "instead of X, you may Y"
    /// effects involving zone changes.
    InteractiveChooseDestination {
        /// The destinations the player can choose from.
        /// The first destination is typically the default (original destination).
        destinations: Vec<Zone>,
        /// Description for the choice prompt.
        description: String,
    },
}

/// How to modify an event.
#[derive(Debug, Clone, PartialEq)]
pub enum EventModification {
    /// Multiply by a factor (e.g., double strike)
    Multiply(u32),

    /// Add to the value
    Add(i32),

    /// Subtract from the value (minimum 0)
    Subtract(u32),

    /// Set to a specific value
    SetTo(u32),

    /// Reduce to zero (prevent)
    ReduceToZero,
}

/// Where to redirect an effect.
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectTarget {
    /// Redirect to this permanent's controller
    ToController,

    /// Redirect to a specific player
    ToPlayer(PlayerId),

    /// Redirect to a specific object
    ToObject(ObjectId),

    /// Redirect to the source of the effect
    ToSource,
}

/// Which target to redirect in a multi-target event.
///
/// For events like `MoveCounters` that have multiple redirectable targets
/// (e.g., "source" and "destination"), this specifies which one to redirect.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RedirectWhich {
    /// Redirect the first (or only) redirectable target.
    /// This is the default behavior.
    #[default]
    First,

    /// Redirect the Nth target (0-indexed).
    Index(usize),

    /// Redirect targets matching this description.
    /// The description is matched against `RedirectableTarget::description`.
    /// E.g., "counter source" or "counter destination" for MoveCounters.
    ByDescription(&'static str),
}

/// Source type for replacement effects - distinguishes between static abilities
/// and resolution-based effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementEffectSource {
    /// Effect from a static ability (regenerated each state refresh)
    StaticAbility,
    /// Effect from a resolved spell/ability (persists until removed)
    Resolution,
}

/// Manages all replacement effects in the game.
#[derive(Debug, Clone, Default)]
pub struct ReplacementEffectManager {
    /// All active replacement effects
    effects: Vec<ReplacementEffect>,

    /// Source type for each effect (by ID)
    effect_sources: std::collections::HashMap<u64, ReplacementEffectSource>,

    /// One-shot effects that are consumed after a single use (e.g., regeneration shields).
    /// These are removed after being applied once.
    one_shot_effects: std::collections::HashSet<ReplacementEffectId>,

    /// Next effect ID to assign
    next_id: u64,
}

impl ReplacementEffectManager {
    /// Create a new empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot active replacement effects.
    pub fn effects(&self) -> &[ReplacementEffect] {
        &self.effects
    }

    /// Snapshot effect sources in deterministic order.
    pub fn effect_sources_snapshot(&self) -> Vec<(u64, ReplacementEffectSource)> {
        let mut entries: Vec<(u64, ReplacementEffectSource)> = self
            .effect_sources
            .iter()
            .map(|(id, source)| (*id, *source))
            .collect();
        entries.sort_by_key(|(id, _)| *id);
        entries
    }

    /// Snapshot one-shot effect ids in deterministic order.
    pub fn one_shot_effects_snapshot(&self) -> Vec<u64> {
        let mut entries: Vec<u64> = self.one_shot_effects.iter().map(|id| id.0).collect();
        entries.sort();
        entries
    }

    /// Get the next effect id (for deterministic state hashing).
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Add a new replacement effect.
    pub fn add_effect(&mut self, mut effect: ReplacementEffect) -> ReplacementEffectId {
        let id = ReplacementEffectId::new(self.next_id);
        self.next_id += 1;
        effect.id = id;
        self.effects.push(effect);
        id
    }

    /// Remove an effect by ID.
    pub fn remove_effect(&mut self, id: ReplacementEffectId) {
        self.effects.retain(|e| e.id != id);
        self.effect_sources.remove(&id.0);
        self.one_shot_effects.remove(&id);
    }

    /// Remove all effects from a specific source.
    pub fn remove_effects_from_source(&mut self, source: ObjectId) {
        self.effects.retain(|e| e.source != source);
    }

    /// Get all effects that might apply to a damage event.
    /// All effects are returned and filtered at runtime via matches_event().
    pub fn get_damage_replacements(&self) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.matcher.is_some())
            .collect()
    }

    /// Get all effects that might apply to a zone change.
    /// All effects are returned and filtered at runtime via matches_event().
    pub fn get_zone_change_replacements(&self) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.matcher.is_some())
            .collect()
    }

    /// Get all effects that might apply to drawing cards.
    /// All effects are returned and filtered at runtime via matches_event().
    pub fn get_draw_replacements(&self) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.matcher.is_some())
            .collect()
    }

    /// Get all self-replacement effects for a specific source.
    pub fn get_self_replacements(&self, source: ObjectId) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.self_replacement && e.source == source)
            .collect()
    }

    /// Get all non-self replacement effects.
    pub fn get_other_replacements(&self) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| !e.self_replacement)
            .collect()
    }

    /// Get a replacement effect by its ID.
    pub fn get_effect(&self, id: ReplacementEffectId) -> Option<&ReplacementEffect> {
        self.effects.iter().find(|e| e.id == id)
    }

    /// Add a replacement effect from a static ability.
    ///
    /// These effects are regenerated each state refresh, so they are tracked
    /// separately from resolution-based effects.
    pub fn add_static_ability_effect(&mut self, effect: ReplacementEffect) -> ReplacementEffectId {
        let id = self.add_effect(effect);
        self.effect_sources
            .insert(id.0, ReplacementEffectSource::StaticAbility);
        id
    }

    /// Add a replacement effect from a resolved spell/ability.
    pub fn add_resolution_effect(&mut self, effect: ReplacementEffect) -> ReplacementEffectId {
        let id = self.add_effect(effect);
        self.effect_sources
            .insert(id.0, ReplacementEffectSource::Resolution);
        id
    }

    /// Clear all effects from static abilities.
    ///
    /// Called before regenerating static ability effects during state refresh.
    pub fn clear_static_ability_effects(&mut self) {
        let static_ids: Vec<ReplacementEffectId> = self
            .effects
            .iter()
            .filter(|e| {
                self.effect_sources
                    .get(&e.id.0)
                    .map(|s| *s == ReplacementEffectSource::StaticAbility)
                    .unwrap_or(false)
            })
            .map(|e| e.id)
            .collect();

        for id in static_ids {
            self.remove_effect(id);
        }
    }

    /// Get ETB replacement effects that apply to a specific object entering.
    ///
    /// Returns self-replacement effects for the object plus any other effects
    /// Get ETB replacement effects that apply to objects entering the battlefield.
    /// All effects are returned and filtered at runtime via matches_event().
    pub fn get_etb_replacements(&self, entering_object: ObjectId) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.matcher.is_some() && (e.source == entering_object || !e.self_replacement))
            .collect()
    }

    /// Get discard replacement effects that might apply.
    /// All effects are returned and filtered at runtime via matches_event().
    pub fn get_discard_replacements(&self) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.matcher.is_some())
            .collect()
    }

    // =========================================================================
    // One-Shot Effect Management
    // =========================================================================

    /// Add a one-shot replacement effect that will be consumed after use.
    ///
    /// One-shot effects are used for things like regeneration shields, which
    /// replace destruction once and then are removed. The effect is registered
    /// and tracked separately from static ability effects.
    ///
    /// Returns the ID of the added effect.
    pub fn add_one_shot_effect(&mut self, effect: ReplacementEffect) -> ReplacementEffectId {
        let id = self.add_effect(effect);
        self.one_shot_effects.insert(id);
        id
    }

    /// Mark a one-shot effect as used and remove it.
    ///
    /// Returns true if the effect was found and removed, false if it wasn't
    /// a one-shot effect or didn't exist.
    pub fn mark_effect_used(&mut self, id: ReplacementEffectId) -> bool {
        if self.one_shot_effects.remove(&id) {
            self.remove_effect(id);
            true
        } else {
            false
        }
    }

    /// Check if an effect is a one-shot effect.
    pub fn is_one_shot(&self, id: ReplacementEffectId) -> bool {
        self.one_shot_effects.contains(&id)
    }

    /// Clear all one-shot effects (e.g., at end of turn).
    pub fn clear_one_shot_effects(&mut self) {
        let one_shot_ids: Vec<_> = self.one_shot_effects.iter().copied().collect();
        for id in one_shot_ids {
            self.remove_effect(id);
        }
        self.one_shot_effects.clear();
    }

    /// Get the count of one-shot effects from a specific source.
    ///
    /// This is useful for checking how many regeneration shields a creature has.
    pub fn count_one_shot_effects_from_source(&self, source: ObjectId) -> u32 {
        self.effects
            .iter()
            .filter(|e| e.source == source && self.one_shot_effects.contains(&e.id))
            .count() as u32
    }
}

impl ReplacementEffect {
    /// Create a new replacement effect using a trait-based matcher.
    pub fn with_matcher<M: ReplacementMatcher + 'static>(
        source: ObjectId,
        controller: PlayerId,
        matcher: M,
        replacement: ReplacementAction,
    ) -> Self {
        Self {
            id: ReplacementEffectId(0),
            source,
            controller,
            replacement,
            self_replacement: false,
            matcher: Some(Box::new(matcher)),
        }
    }

    /// Set a trait-based matcher on this effect.
    pub fn with_trait_matcher<M: ReplacementMatcher + 'static>(mut self, matcher: M) -> Self {
        self.matcher = Some(Box::new(matcher));
        self
    }

    /// Mark this as a self-replacement effect.
    pub fn self_replacing(mut self) -> Self {
        self.self_replacement = true;
        self
    }

    /// Create a damage prevention effect.
    pub fn prevent_damage(source: ObjectId, controller: PlayerId, amount: u32) -> Self {
        Self::with_matcher(
            source,
            controller,
            DamageToPlayerMatcher::to_you(),
            ReplacementAction::Modify(EventModification::Subtract(amount)),
        )
    }

    /// Create a "can't gain life" effect.
    pub fn cant_gain_life(source: ObjectId, controller: PlayerId) -> Self {
        Self::with_matcher(
            source,
            controller,
            WouldGainLifeMatcher::any_player(),
            ReplacementAction::Prevent,
        )
    }

    /// Create an "enters tapped" effect.
    pub fn enters_tapped(source: ObjectId, controller: PlayerId, filter: ObjectFilter) -> Self {
        Self::with_matcher(
            source,
            controller,
            WouldEnterBattlefieldMatcher::new(filter),
            ReplacementAction::EnterTapped,
        )
    }

    /// Create a "this enters with N counters" effect.
    pub fn enters_with_counters(
        source: ObjectId,
        controller: PlayerId,
        counter_type: CounterType,
        count: Value,
    ) -> Self {
        Self::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::EnterWithCounters {
                counter_type,
                count,
            },
        )
        .self_replacing()
    }

    /// Create a "if this would die, exile it instead" effect.
    pub fn exile_instead_of_dying(source: ObjectId, controller: PlayerId) -> Self {
        Self::with_matcher(
            source,
            controller,
            ThisWouldDieMatcher,
            ReplacementAction::ChangeDestination(Zone::Exile),
        )
        .self_replacing()
    }

    /// Create a "double damage" effect.
    pub fn double_damage(
        source: ObjectId,
        controller: PlayerId,
        from_filter: ObjectFilter,
    ) -> Self {
        Self::with_matcher(
            source,
            controller,
            DamageFromSourceMatcher::new(from_filter),
            ReplacementAction::Double,
        )
    }

    /// Create a "skip draw step" effect.
    pub fn skip_draw(source: ObjectId, controller: PlayerId, player: PlayerFilter) -> Self {
        Self::with_matcher(
            source,
            controller,
            WouldDrawCardMatcher::new(player),
            ReplacementAction::Skip,
        )
    }

    // Note: "Can't lose the game" (Platinum Angel) is implemented via CantEffects,
    // not as a replacement effect. See game_state.rs CantEffects::can_lose_game().

    /// Create an indestructible effect.
    pub fn indestructible(source: ObjectId, controller: PlayerId) -> Self {
        Self::with_matcher(
            source,
            controller,
            ThisWouldBeDestroyedMatcher,
            ReplacementAction::Prevent,
        )
        .self_replacing()
    }

    /// Create a Library of Leng style discard replacement effect.
    ///
    /// "If an effect causes you to discard a card, you may put it on top of
    /// your library instead of into your graveyard."
    pub fn library_of_leng_discard(source: ObjectId, controller: PlayerId) -> Self {
        Self::with_matcher(
            source,
            controller,
            WouldDiscardMatcher::you_from_effect(),
            ReplacementAction::ChangeDestination(Zone::Library),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_prevention() {
        let effect =
            ReplacementEffect::prevent_damage(ObjectId::from_raw(1), PlayerId::from_index(0), 3);

        assert!(
            effect.matcher.is_some(),
            "prevent_damage should use trait-based matcher"
        );
        assert!(matches!(
            effect.replacement,
            ReplacementAction::Modify(EventModification::Subtract(3))
        ));
    }

    #[test]
    fn test_enters_with_counters() {
        let effect = ReplacementEffect::enters_with_counters(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            CounterType::PlusOnePlusOne,
            Value::Fixed(3),
        );

        assert!(effect.self_replacement);
        assert!(
            effect.matcher.is_some(),
            "enters_with_counters should use trait-based matcher"
        );
        assert!(matches!(
            effect.replacement,
            ReplacementAction::EnterWithCounters { .. }
        ));
    }

    #[test]
    fn test_exile_instead_of_dying() {
        let effect = ReplacementEffect::exile_instead_of_dying(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
        );

        assert!(effect.self_replacement);
        assert!(
            effect.matcher.is_some(),
            "exile_instead_of_dying should use trait-based matcher"
        );
        assert!(matches!(
            effect.replacement,
            ReplacementAction::ChangeDestination(Zone::Exile)
        ));
    }

    #[test]
    fn test_replacement_manager() {
        let mut manager = ReplacementEffectManager::new();

        let effect1 =
            ReplacementEffect::prevent_damage(ObjectId::from_raw(1), PlayerId::from_index(0), 3);
        let effect2 = ReplacementEffect::enters_with_counters(
            ObjectId::from_raw(2),
            PlayerId::from_index(0),
            CounterType::PlusOnePlusOne,
            Value::Fixed(2),
        );

        let id1 = manager.add_effect(effect1);
        let id2 = manager.add_effect(effect2);

        // Effects are tracked
        assert_eq!(manager.effects().len(), 2);

        // Both effects have matchers (used by the new trait-based system)
        assert!(manager.effects().iter().all(|e| e.matcher.is_some()));

        // Remove one effect
        manager.remove_effect(id1);
        assert_eq!(manager.effects().len(), 1);
        assert_eq!(manager.effects()[0].id, id2);
    }

    #[test]
    fn test_self_vs_other_replacements() {
        let mut manager = ReplacementEffectManager::new();

        let self_effect = ReplacementEffect::enters_with_counters(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            CounterType::PlusOnePlusOne,
            Value::Fixed(2),
        );

        let other_effect = ReplacementEffect::enters_tapped(
            ObjectId::from_raw(2),
            PlayerId::from_index(0),
            ObjectFilter::creature(),
        );

        manager.add_effect(self_effect);
        manager.add_effect(other_effect);

        assert_eq!(
            manager.get_self_replacements(ObjectId::from_raw(1)).len(),
            1
        );
        assert_eq!(
            manager.get_self_replacements(ObjectId::from_raw(2)).len(),
            0
        );
        assert_eq!(manager.get_other_replacements().len(), 1);
    }
}
