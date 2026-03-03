use std::collections::{HashMap, HashSet};

use crate::alternative_cast::CastingMethod;
use crate::card::Card;
use crate::continuous::{ContinuousEffect, ContinuousEffectManager};
use crate::cost::OptionalCostsPaid;
use crate::decision::KeywordPaymentContribution;
use crate::events::{Event, EventKind};
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::object::Object;
use crate::player::Player;
use crate::prevention::PreventionEffectManager;
use crate::replacement::{ReplacementEffectId, ReplacementEffectManager};
use crate::static_abilities::StaticAbility;
use crate::triggers::TriggerIdentity;
use crate::types::Subtype;
use crate::zone::Zone;

/// Pending replacement effect choice when multiple effects apply to the same event.
///
/// Per Rule 616.1e, when multiple replacement effects at the same priority level
/// could apply to an event, the affected player (or controller of the affected
/// object) must choose which one to apply first.
#[derive(Debug, Clone)]
pub struct PendingReplacementChoice {
    /// The event that replacement effects are trying to modify (new trait-based Event)
    pub event: Event,
    /// IDs of the applicable replacement effects
    pub applicable_effects: Vec<ReplacementEffectId>,
    /// The player who must choose which effect to apply
    pub player: PlayerId,
}

/// Result of moving an object to the battlefield with ETB replacement processing.
///
/// This captures all the modifications that were applied by replacement effects.
#[derive(Debug, Clone)]
pub struct EntersResult {
    /// The new object ID (zone changes create new IDs per rule 400.7)
    pub new_id: ObjectId,
    /// Whether the permanent entered tapped
    pub enters_tapped: bool,
}

/// Linked exile group metadata for "exile ... until ..." effects.
#[derive(Debug, Clone)]
pub struct LinkedExileGroup {
    /// Stable identities of objects exiled as part of this linked group.
    pub stable_ids: Vec<StableId>,
    /// Zone to return objects to when the delayed condition is met.
    pub return_zone: Zone,
    /// If returning to the battlefield, reset controller to owner.
    pub return_under_owner_control: bool,
}

/// Key type for extensible per-turn counters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TurnCounterKey {
    /// Count by trigger event kind.
    EventKind(EventKind),
    /// Count by structural trigger identity.
    TriggerIdentity(TriggerIdentity),
    /// Arbitrary named counters (cards drawn, ETBs, etc.).
    Named(String),
}

/// Generic per-turn counter tracker.
#[derive(Debug, Clone, Default)]
pub struct TurnCounterTracker {
    counters: HashMap<TurnCounterKey, u32>,
}

fn activated_ability_turn_counter_name(source: ObjectId, ability_index: usize) -> String {
    format!("activated_ability:{}:{}", source.0, ability_index)
}

impl TurnCounterTracker {
    pub fn increment(&mut self, key: TurnCounterKey) {
        *self.counters.entry(key).or_insert(0) += 1;
    }

    pub fn increment_event_kind(&mut self, event_kind: EventKind) {
        self.increment(TurnCounterKey::EventKind(event_kind));
    }

    pub fn increment_trigger_identity(&mut self, trigger_id: TriggerIdentity) {
        self.increment(TurnCounterKey::TriggerIdentity(trigger_id));
    }

    pub fn increment_named(&mut self, name: impl Into<String>) {
        self.increment(TurnCounterKey::Named(name.into()));
    }

    pub fn get(&self, key: &TurnCounterKey) -> u32 {
        self.counters.get(key).copied().unwrap_or(0)
    }

    pub fn clear(&mut self) {
        self.counters.clear();
    }

    pub fn snapshot(&self) -> Vec<(TurnCounterKey, u32)> {
        self.counters
            .iter()
            .map(|(key, count)| (key.clone(), *count))
            .collect()
    }
}

// =============================================================================
// "Can't" Effect Tracking (Rule 614.17)
// =============================================================================
//
// "Can't" effects are NOT replacement effects. They are prohibitions that must
// be checked BEFORE attempting an action or event. Per Rule 614.17a, events
// that "can't" happen simply don't happen.
//
// Examples:
// - "You can't gain life" (Sulfuric Vortex)
// - "Players can't search libraries" (Stranglehold)
// - "This creature can't attack" (Pacifism)
// - "That creature can't block" (Goblin War Drums)
// - "Damage can't be prevented" (Leyline of Punishment)
// - "This permanent can't be destroyed" (Indestructible)

/// Tracks active "can't" effects in the game.
///
/// Per Rule 614.17, "can't" effects are not replacement effects - they are
/// prohibitions that prevent events from happening at all. They must be
/// checked BEFORE attempting an action or event.
#[derive(Debug, Clone, Default)]
pub struct CantEffectTracker {
    /// Players who can't gain life.
    /// Example: Sulfuric Vortex, Erebos, God of the Dead
    pub cant_gain_life: HashSet<PlayerId>,

    /// Players who can't search libraries.
    /// Example: Stranglehold, Aven Mindcensor (partial)
    pub cant_search: HashSet<PlayerId>,

    /// Creatures that can't attack.
    /// Example: Pacifism, Propaganda (if unpaid), Maze of Ith
    pub cant_attack: HashSet<ObjectId>,

    /// Creatures that can't attack alone.
    /// Example: "This creature can't attack alone."
    pub cant_attack_alone: HashSet<ObjectId>,

    /// Creatures that can't block.
    /// Example: Goblin War Drums, Madcap Skills
    pub cant_block: HashSet<ObjectId>,

    /// Blocker -> attackers this blocker can't block this turn.
    /// Example: "Target creature can't block this creature this turn."
    pub cant_block_specific_attackers: HashMap<ObjectId, HashSet<ObjectId>>,

    /// Blocker -> attackers this blocker must block this turn if able.
    /// Example: "Target creature blocks this creature this turn if able."
    pub must_block_specific_attackers: HashMap<ObjectId, HashSet<ObjectId>>,

    /// Creatures that can't block alone.
    /// Example: "This creature can't block alone."
    pub cant_block_alone: HashSet<ObjectId>,

    /// Permanents that can't untap during their controller's untap step.
    /// Example: "It doesn't untap during its controller's untap step"
    pub cant_untap: HashSet<ObjectId>,

    /// Permanents that can't be destroyed (indestructible via effect, not ability).
    /// Note: Intrinsic indestructible keyword is checked separately on the object.
    pub cant_be_destroyed: HashSet<ObjectId>,

    /// Permanents that can't be regenerated.
    /// Example: "Target creature can't be regenerated this turn."
    pub cant_be_regenerated: HashSet<ObjectId>,

    /// Permanents that can't be sacrificed.
    /// Example: Sigarda, Host of Herons (for creatures you control)
    pub cant_be_sacrificed: HashSet<ObjectId>,

    /// Per-player spell filters that cannot be cast.
    ///
    /// Examples:
    /// - default filter => "can't cast spells"
    /// - creature filter => "can't cast creature spells"
    pub cant_cast_filters: HashMap<PlayerId, Vec<crate::target::ObjectFilter>>,

    /// Players who can't activate non-mana abilities.
    /// Example: Split second while a split-second spell is on the stack.
    pub cant_activate_non_mana_abilities: HashSet<PlayerId>,

    /// Permanents whose activated abilities can't be activated (including mana abilities).
    /// Example: Collector Ouphe ("Activated abilities of artifacts can't be activated.")
    pub cant_activate_abilities_of: HashSet<ObjectId>,

    /// Permanents whose activated abilities with {T} in their costs can't be activated.
    pub cant_activate_tap_abilities_of: HashSet<ObjectId>,

    /// Permanents whose non-mana activated abilities can't be activated.
    /// Example: Damping Matrix ("... can't be activated unless they're mana abilities.")
    pub cant_activate_non_mana_abilities_of: HashSet<ObjectId>,

    /// Per-player "can't cast more than one matching spell each turn" restrictions.
    ///
    /// Each filter applies to both:
    /// - the spell being cast now, and
    /// - spells this player has already cast this turn.
    ///
    /// This keeps cast-limit restrictions generic (nonartifact, non-Phyrexian, etc.)
    /// without hard-coding one tracker set per variant.
    pub cant_cast_limit_filters: HashMap<PlayerId, Vec<crate::target::ObjectFilter>>,

    /// Players who can't draw cards.
    /// Example: Notion Thief redirecting draws
    pub cant_draw: HashSet<PlayerId>,

    /// Players who can't draw extra cards (more than one per turn).
    /// Maps: restricted player -> restricting player (e.g., opponent of Narset controller)
    /// Example: Narset, Parter of Veils ("Your opponents can't draw more than one card each turn")
    pub cant_draw_extra_cards: HashSet<PlayerId>,

    /// Creatures that can't be blocked.
    /// Example: Whispersilk Cloak, Invisible Stalker
    pub cant_be_blocked: HashSet<ObjectId>,

    /// Permanents that can't have counters placed on them.
    /// Example: Melira, Sylvok Outcast (for -1/-1 counters on creatures you control)
    /// Note: This is actually a replacement effect in Melira's case, but some
    /// effects truly prevent counters.
    pub cant_have_counters_placed: HashSet<ObjectId>,

    /// Whether damage prevention is globally disabled.
    /// Example: Leyline of Punishment, Everlasting Torment
    pub damage_cant_be_prevented: bool,

    /// Players whose life total can't change.
    /// Example: Platinum Emperion
    pub life_total_cant_change: HashSet<PlayerId>,

    /// Players who can't lose the game.
    /// Example: Platinum Angel
    pub cant_lose_game: HashSet<PlayerId>,

    /// Players who can't win the game.
    /// Example: Angel's Grace preventing opponent's win
    pub cant_win_game: HashSet<PlayerId>,

    /// Permanents that can't be targeted.
    /// Example: Hexproof/Shroud (tracked separately), but also effects like
    /// "can't be the target of spells or abilities"
    pub cant_be_targeted: HashSet<ObjectId>,

    /// Players that can't be targeted.
    pub cant_target_players: HashSet<PlayerId>,

    /// Permanents that can't be countered while on the stack.
    /// Example: Vexing Shusher, Prowling Serpopard
    pub cant_be_countered: HashSet<ObjectId>,

    /// Permanents that can't transform.
    /// Example: "Non-Human Werewolves you control can't transform."
    pub cant_transform: HashSet<ObjectId>,
}

#[derive(Debug, Clone)]
pub struct RestrictionEffectInstance {
    pub restriction: crate::effect::Restriction,
    pub controller: PlayerId,
    pub source: ObjectId,
    pub duration: crate::effect::Until,
    pub expires_end_of_turn: u32,
}

impl RestrictionEffectInstance {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        matches!(self.duration, crate::effect::Until::EndOfTurn)
            && current_turn > self.expires_end_of_turn
    }

    pub fn is_active(&self, game: &GameState, current_turn: u32) -> bool {
        if self.is_expired(current_turn) {
            return false;
        }

        match self.duration {
            crate::effect::Until::YourNextTurn => {
                !(current_turn > self.expires_end_of_turn
                    && game.turn.active_player == self.controller)
            }
            crate::effect::Until::ControllersNextUntapStep => {
                if current_turn <= self.expires_end_of_turn
                    || game.turn.active_player != self.controller
                {
                    true
                } else {
                    matches!(game.turn.phase, Phase::Beginning)
                        && matches!(game.turn.step, Some(Step::Untap))
                }
            }
            crate::effect::Until::ThisLeavesTheBattlefield => game
                .object(self.source)
                .is_some_and(|obj| obj.zone == Zone::Battlefield),
            crate::effect::Until::YouStopControllingThis => {
                game.object(self.source).is_some_and(|obj| {
                    obj.zone == Zone::Battlefield && obj.controller == self.controller
                })
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoadEffectInstance {
    pub creature: ObjectId,
    pub goaded_by: PlayerId,
    pub source: ObjectId,
    pub duration: crate::effect::Until,
    pub expires_end_of_turn: u32,
}

impl GoadEffectInstance {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        matches!(self.duration, crate::effect::Until::EndOfTurn)
            && current_turn > self.expires_end_of_turn
    }

    pub fn is_active(&self, game: &GameState, current_turn: u32) -> bool {
        if self.is_expired(current_turn) {
            return false;
        }

        match self.duration {
            crate::effect::Until::YourNextTurn => {
                !(current_turn > self.expires_end_of_turn
                    && game.turn.active_player == self.goaded_by)
            }
            crate::effect::Until::ThisLeavesTheBattlefield => game
                .object(self.source)
                .is_some_and(|obj| obj.zone == Zone::Battlefield),
            crate::effect::Until::YouStopControllingThis => {
                game.object(self.source).is_some_and(|obj| {
                    obj.zone == Zone::Battlefield && obj.controller == self.goaded_by
                })
            }
            _ => true,
        }
    }
}

impl CantEffectTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: CantEffectTracker) {
        self.cant_gain_life.extend(other.cant_gain_life);
        self.cant_search.extend(other.cant_search);
        self.cant_attack.extend(other.cant_attack);
        self.cant_attack_alone.extend(other.cant_attack_alone);
        self.cant_block.extend(other.cant_block);
        for (blocker, attackers) in other.cant_block_specific_attackers {
            self.cant_block_specific_attackers
                .entry(blocker)
                .or_default()
                .extend(attackers);
        }
        for (blocker, attackers) in other.must_block_specific_attackers {
            self.must_block_specific_attackers
                .entry(blocker)
                .or_default()
                .extend(attackers);
        }
        self.cant_block_alone.extend(other.cant_block_alone);
        self.cant_untap.extend(other.cant_untap);
        self.cant_be_destroyed.extend(other.cant_be_destroyed);
        self.cant_be_regenerated.extend(other.cant_be_regenerated);
        self.cant_be_sacrificed.extend(other.cant_be_sacrificed);
        for (player, filters) in other.cant_cast_filters {
            for filter in filters {
                self.add_cant_cast_filter(player, filter);
            }
        }
        self.cant_activate_non_mana_abilities
            .extend(other.cant_activate_non_mana_abilities);
        self.cant_activate_abilities_of
            .extend(other.cant_activate_abilities_of);
        self.cant_activate_tap_abilities_of
            .extend(other.cant_activate_tap_abilities_of);
        self.cant_activate_non_mana_abilities_of
            .extend(other.cant_activate_non_mana_abilities_of);
        for (player, filters) in other.cant_cast_limit_filters {
            for filter in filters {
                self.add_cast_limit_filter(player, filter);
            }
        }
        self.cant_draw.extend(other.cant_draw);
        self.cant_draw_extra_cards
            .extend(other.cant_draw_extra_cards);
        self.cant_be_blocked.extend(other.cant_be_blocked);
        self.cant_have_counters_placed
            .extend(other.cant_have_counters_placed);
        self.damage_cant_be_prevented |= other.damage_cant_be_prevented;
        self.life_total_cant_change
            .extend(other.life_total_cant_change);
        self.cant_lose_game.extend(other.cant_lose_game);
        self.cant_win_game.extend(other.cant_win_game);
        self.cant_be_targeted.extend(other.cant_be_targeted);
        self.cant_target_players.extend(other.cant_target_players);
        self.cant_be_countered.extend(other.cant_be_countered);
        self.cant_transform.extend(other.cant_transform);
    }

    /// Clear all tracked "can't" effects.
    /// Called when rebuilding the tracker from current game state.
    pub fn clear(&mut self) {
        self.cant_gain_life.clear();
        self.cant_search.clear();
        self.cant_attack.clear();
        self.cant_attack_alone.clear();
        self.cant_block.clear();
        self.cant_block_specific_attackers.clear();
        self.must_block_specific_attackers.clear();
        self.cant_block_alone.clear();
        self.cant_untap.clear();
        self.cant_be_destroyed.clear();
        self.cant_be_regenerated.clear();
        self.cant_be_sacrificed.clear();
        self.cant_cast_filters.clear();
        self.cant_activate_non_mana_abilities.clear();
        self.cant_activate_abilities_of.clear();
        self.cant_activate_tap_abilities_of.clear();
        self.cant_activate_non_mana_abilities_of.clear();
        self.cant_cast_limit_filters.clear();
        self.cant_draw.clear();
        self.cant_draw_extra_cards.clear();
        self.cant_be_blocked.clear();
        self.cant_have_counters_placed.clear();
        self.damage_cant_be_prevented = false;
        self.life_total_cant_change.clear();
        self.cant_lose_game.clear();
        self.cant_win_game.clear();
        self.cant_be_targeted.clear();
        self.cant_target_players.clear();
        self.cant_be_countered.clear();
        self.cant_transform.clear();
    }

    /// Check if a player can gain life.
    pub fn can_gain_life(&self, player: PlayerId) -> bool {
        !self.cant_gain_life.contains(&player) && !self.life_total_cant_change.contains(&player)
    }

    /// Check if a player can lose life (not from damage).
    pub fn can_lose_life(&self, player: PlayerId) -> bool {
        !self.life_total_cant_change.contains(&player)
    }

    /// Check if a player's life total can change (Platinum Emperion, etc.).
    pub fn can_change_life_total(&self, player: PlayerId) -> bool {
        !self.life_total_cant_change.contains(&player)
    }

    /// Check if a player can search their library.
    pub fn can_search_library(&self, player: PlayerId) -> bool {
        !self.cant_search.contains(&player)
    }

    /// Check if a creature can attack.
    pub fn can_attack(&self, creature: ObjectId) -> bool {
        !self.cant_attack.contains(&creature)
    }

    /// Check if a creature can attack alone (as the only attacker).
    pub fn can_attack_alone(&self, creature: ObjectId) -> bool {
        !self.cant_attack_alone.contains(&creature)
    }

    /// Check if a creature can block.
    pub fn can_block(&self, creature: ObjectId) -> bool {
        !self.cant_block.contains(&creature)
    }

    /// Check if a creature can block a specific attacker.
    pub fn can_block_attacker(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        self.can_block(blocker)
            && self
                .cant_block_specific_attackers
                .get(&blocker)
                .is_none_or(|attackers| !attackers.contains(&attacker))
    }

    /// Check if a creature can block alone (as the only blocker).
    pub fn can_block_alone(&self, creature: ObjectId) -> bool {
        !self.cant_block_alone.contains(&creature)
    }

    /// Check if a creature must block a specific attacker this turn if able.
    pub fn must_block_attacker(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        self.must_block_specific_attackers
            .get(&blocker)
            .is_some_and(|attackers| attackers.contains(&attacker))
    }

    /// Get required attackers for a blocker, if any.
    pub fn required_attackers_for_blocker(&self, blocker: ObjectId) -> Option<&HashSet<ObjectId>> {
        self.must_block_specific_attackers.get(&blocker)
    }

    /// Check if a permanent can untap during untap step.
    pub fn can_untap(&self, permanent: ObjectId) -> bool {
        !self.cant_untap.contains(&permanent)
    }

    /// Check if damage can be prevented.
    pub fn can_prevent_damage(&self) -> bool {
        !self.damage_cant_be_prevented
    }

    /// Check if a permanent can be destroyed.
    pub fn can_be_destroyed(&self, permanent: ObjectId) -> bool {
        !self.cant_be_destroyed.contains(&permanent)
    }

    /// Check if a permanent can be regenerated.
    pub fn can_be_regenerated(&self, permanent: ObjectId) -> bool {
        !self.cant_be_regenerated.contains(&permanent)
    }

    /// Check if a permanent can be sacrificed.
    pub fn can_be_sacrificed(&self, permanent: ObjectId) -> bool {
        !self.cant_be_sacrificed.contains(&permanent)
    }

    /// Check if a creature can be blocked.
    pub fn can_be_blocked(&self, creature: ObjectId) -> bool {
        !self.cant_be_blocked.contains(&creature)
    }

    /// Check if a player can lose the game.
    pub fn can_lose_game(&self, player: PlayerId) -> bool {
        !self.cant_lose_game.contains(&player)
    }

    /// Check if a player can win the game.
    pub fn can_win_game(&self, player: PlayerId) -> bool {
        !self.cant_win_game.contains(&player)
    }

    /// Check if a player can draw cards at all.
    pub fn can_draw(&self, player: PlayerId) -> bool {
        !self.cant_draw.contains(&player)
    }

    /// Check if a player can draw extra cards this turn.
    pub fn can_draw_extra_cards(&self, player: PlayerId) -> bool {
        !self.cant_draw_extra_cards.contains(&player)
    }

    /// Check if a player can cast spells.
    pub fn can_cast_spells(&self, player: PlayerId) -> bool {
        self.cast_filters_for_player(player).is_none_or(|filters| {
            !filters
                .iter()
                .any(|filter| filter == &crate::target::ObjectFilter::default())
        })
    }

    /// Check if a player can activate non-mana abilities.
    pub fn can_activate_non_mana_abilities(&self, player: PlayerId) -> bool {
        !self.cant_activate_non_mana_abilities.contains(&player)
    }

    /// Check if activated abilities of a permanent can be activated (including mana abilities).
    pub fn can_activate_abilities_of(&self, source: ObjectId) -> bool {
        !self.cant_activate_abilities_of.contains(&source)
    }

    /// Check if activated abilities with {T} in their costs of a permanent can be activated.
    pub fn can_activate_tap_abilities_of(&self, source: ObjectId) -> bool {
        !self.cant_activate_tap_abilities_of.contains(&source)
    }

    /// Check if non-mana activated abilities of a permanent can be activated.
    pub fn can_activate_non_mana_abilities_of(&self, source: ObjectId) -> bool {
        !self.cant_activate_non_mana_abilities_of.contains(&source)
    }

    /// Check if a player can cast creature spells.
    pub fn can_cast_creature_spells(&self, player: PlayerId) -> bool {
        self.cast_filters_for_player(player).is_none_or(|filters| {
            !filters.iter().any(|filter| {
                filter
                    == &crate::target::ObjectFilter::default()
                        .with_type(crate::types::CardType::Creature)
            })
        })
    }

    /// Add a cast-prohibition filter for a player ("can't cast [matching] spells").
    pub fn add_cant_cast_filter(
        &mut self,
        player: PlayerId,
        spell_filter: crate::target::ObjectFilter,
    ) {
        let filters = self.cant_cast_filters.entry(player).or_default();
        if !filters.iter().any(|existing| existing == &spell_filter) {
            filters.push(spell_filter);
        }
    }

    /// Get active cast-prohibition filters for a player, if any.
    pub fn cast_filters_for_player(
        &self,
        player: PlayerId,
    ) -> Option<&[crate::target::ObjectFilter]> {
        self.cant_cast_filters.get(&player).map(Vec::as_slice)
    }

    /// Add a cast-limit filter for a player ("can't cast more than one matching spell each turn").
    pub fn add_cast_limit_filter(
        &mut self,
        player: PlayerId,
        spell_filter: crate::target::ObjectFilter,
    ) {
        let filters = self.cant_cast_limit_filters.entry(player).or_default();
        if !filters.iter().any(|existing| existing == &spell_filter) {
            filters.push(spell_filter);
        }
    }

    /// Get active cast-limit filters for a player, if any.
    pub fn cast_limit_filters_for_player(
        &self,
        player: PlayerId,
    ) -> Option<&[crate::target::ObjectFilter]> {
        self.cant_cast_limit_filters.get(&player).map(Vec::as_slice)
    }

    /// Check if a player can cast an additional spell matching a specific filter this turn.
    pub fn can_cast_additional_spell_matching_this_turn(
        &self,
        player: PlayerId,
        spell_filter: &crate::target::ObjectFilter,
    ) -> bool {
        !self
            .cast_limit_filters_for_player(player)
            .is_some_and(|filters| filters.iter().any(|filter| filter == spell_filter))
    }

    /// Check if a player can cast an additional spell this turn.
    pub fn can_cast_additional_spell_this_turn(&self, player: PlayerId) -> bool {
        self.can_cast_additional_spell_matching_this_turn(
            player,
            &crate::target::ObjectFilter::default(),
        )
    }

    /// Check if a player can cast an additional noncreature spell this turn.
    pub fn can_cast_additional_noncreature_spell_this_turn(&self, player: PlayerId) -> bool {
        self.can_cast_additional_spell_matching_this_turn(
            player,
            &crate::target::ObjectFilter::default().without_type(crate::types::CardType::Creature),
        )
    }

    /// Check if a player can cast an additional nonartifact spell this turn.
    pub fn can_cast_additional_nonartifact_spell_this_turn(&self, player: PlayerId) -> bool {
        self.can_cast_additional_spell_matching_this_turn(
            player,
            &crate::target::ObjectFilter::default().without_type(crate::types::CardType::Artifact),
        )
    }

    /// Check if a player can cast an additional non-Phyrexian spell this turn.
    pub fn can_cast_additional_nonphyrexian_spell_this_turn(&self, player: PlayerId) -> bool {
        self.can_cast_additional_spell_matching_this_turn(
            player,
            &crate::target::ObjectFilter::default()
                .without_subtype(crate::types::Subtype::Phyrexian),
        )
    }

    /// Check if a permanent can have counters placed on it.
    pub fn can_have_counters_placed(&self, permanent: ObjectId) -> bool {
        !self.cant_have_counters_placed.contains(&permanent)
    }

    /// Check if a permanent is untargetable by the rules tracker.
    pub fn is_untargetable(&self, permanent: ObjectId) -> bool {
        self.cant_be_targeted.contains(&permanent)
    }

    /// Check if a player can be targeted.
    pub fn can_target_player(&self, player: PlayerId) -> bool {
        !self.cant_target_players.contains(&player)
    }

    /// Check if a spell on the stack can be countered by effects.
    pub fn can_be_countered(&self, spell: ObjectId) -> bool {
        !self.cant_be_countered.contains(&spell)
    }

    /// Check if a permanent can transform.
    pub fn can_transform(&self, permanent: ObjectId) -> bool {
        !self.cant_transform.contains(&permanent)
    }

    /// Add a player to the "can't gain life" set.
    pub fn add_cant_gain_life(&mut self, player: PlayerId) {
        self.cant_gain_life.insert(player);
    }

    /// Add a creature to the "can't attack" set.
    pub fn add_cant_attack(&mut self, creature: ObjectId) {
        self.cant_attack.insert(creature);
    }

    /// Add a creature to the "can't attack alone" set.
    pub fn add_cant_attack_alone(&mut self, creature: ObjectId) {
        self.cant_attack_alone.insert(creature);
    }

    /// Add a creature to the "can't block" set.
    pub fn add_cant_block(&mut self, creature: ObjectId) {
        self.cant_block.insert(creature);
    }

    /// Add a creature to the "can't block alone" set.
    pub fn add_cant_block_alone(&mut self, creature: ObjectId) {
        self.cant_block_alone.insert(creature);
    }

    /// Add a permanent to the "can't untap" set.
    pub fn add_cant_untap(&mut self, permanent: ObjectId) {
        self.cant_untap.insert(permanent);
    }

    /// Add a creature to the "can't be blocked" set.
    pub fn add_cant_be_blocked(&mut self, creature: ObjectId) {
        self.cant_be_blocked.insert(creature);
    }

    /// Set that damage can't be prevented.
    pub fn set_damage_cant_be_prevented(&mut self, value: bool) {
        self.damage_cant_be_prevented = value;
    }

    /// Add a player to the "can't lose game" set.
    pub fn add_cant_lose_game(&mut self, player: PlayerId) {
        self.cant_lose_game.insert(player);
    }

    /// Add a player to the "life total can't change" set.
    pub fn add_life_total_cant_change(&mut self, player: PlayerId) {
        self.life_total_cant_change.insert(player);
    }
}

// =============================================================================
// "Spend Mana As Though Any Color" Tracking
// =============================================================================
//
// These effects allow mana to be spent as though it were any color.
// They are not replacement effects and must be consulted during mana payment.
//
// Examples:
// - "Players may spend mana as though it were mana of any color." (Mycosynth Lattice)
// - "You may spend mana as though it were mana of any color to pay activation costs
//    of ~'s abilities." (Manascape Refractor)

#[derive(Debug, Clone, Default)]
pub struct ManaSpendEffectTracker {
    /// Players who may spend mana as though it were any color for all costs.
    pub any_color_players: HashSet<PlayerId>,
    /// Sources whose activation costs may be paid as though mana were any color.
    pub any_color_activation_sources: HashSet<ObjectId>,
}

impl ManaSpendEffectTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.any_color_players.clear();
        self.any_color_activation_sources.clear();
    }
}

/// Game phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Beginning,
    FirstMain,
    Combat,
    NextMain,
    Ending,
}

/// Steps within phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    // Beginning phase
    Untap,
    Upkeep,
    Draw,
    // Combat phase
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndCombat,
    // Ending phase
    End,
    Cleanup,
}

/// Turn state tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnState {
    pub active_player: PlayerId,
    pub priority_player: Option<PlayerId>,
    pub turn_number: u32,
    pub phase: Phase,
    pub step: Option<Step>,
}

impl TurnState {
    pub fn new(active_player: PlayerId) -> Self {
        Self {
            active_player,
            priority_player: Some(active_player),
            turn_number: 1,
            phase: Phase::Beginning,
            step: Some(Step::Untap),
        }
    }
}

/// When a player-control effect starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerControlStart {
    /// Starts immediately when the effect resolves.
    Immediate,
    /// Starts at the beginning of the target player's next turn.
    NextTurn,
}

/// How long a player-control effect lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerControlDuration {
    /// Until end of the current turn.
    UntilEndOfTurn,
    /// Until the source leaves the battlefield.
    UntilSourceLeaves,
    /// No duration limit.
    Forever,
}

/// An effect that causes one player to control another player's decisions.
#[derive(Debug, Clone)]
pub struct PlayerControlEffect {
    pub controller: PlayerId,
    pub target: PlayerId,
    pub start: PlayerControlStart,
    pub duration: PlayerControlDuration,
    pub source: Option<StableId>,
    pub timestamp: u64,
    pub active: bool,
    pub expires_on_turn: Option<u32>,
}

/// A target for spells or abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Object(ObjectId),
    Player(PlayerId),
}

/// An entry on the stack.
#[derive(Debug, Clone)]
pub struct StackEntry {
    pub object_id: ObjectId,
    pub controller: PlayerId,
    pub targets: Vec<Target>,
    pub x_value: Option<u32>,
    /// For triggered/activated abilities, the effects to execute.
    /// For spells, this is None and effects come from the spell itself.
    pub ability_effects: Option<Vec<crate::effect::Effect>>,
    /// Whether this is an ability (triggered or activated) vs a spell.
    pub is_ability: bool,
    /// The casting method used (normal or alternative like flashback).
    pub casting_method: CastingMethod,
    /// Which optional costs were paid (kicker, buyback, etc.).
    pub optional_costs_paid: OptionalCostsPaid,
    /// The defending player for combat-related triggers.
    pub defending_player: Option<PlayerId>,
    /// If this is a saga's final chapter ability, the saga's object ID.
    /// When this ability resolves, the saga should be marked for sacrifice.
    pub saga_final_chapter_source: Option<ObjectId>,
    /// The stable instance ID of the source (persists across zone changes).
    /// Used to track the source even after it leaves the battlefield.
    pub source_stable_id: Option<StableId>,
    /// Last known snapshot of the source at the time this stack entry was created.
    /// Used for source-dependent checks when the source object no longer exists.
    pub source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    /// The name of the source card/permanent for display purposes.
    /// Captured at the time the ability is put on the stack.
    pub source_name: Option<String>,
    /// The event that triggered this ability (for triggered abilities).
    /// Contains information about what caused the trigger (e.g., which object entered the battlefield).
    pub triggering_event: Option<crate::triggers::TriggerEvent>,
    /// Intervening-if condition that must be true at resolution time (for triggered abilities).
    /// If this condition is false when the ability would resolve, the ability does nothing.
    pub intervening_if: Option<crate::ConditionExpr>,
    /// Pre-chosen modes for modal spells (chosen during casting per rule 601.2b).
    /// If Some, resolution should use these instead of prompting.
    pub chosen_modes: Option<Vec<usize>>,
    /// Permanents that contributed keyword-ability alternative payments to this spell cast.
    pub keyword_payment_contributions: Vec<KeywordPaymentContribution>,
    /// Creatures that crewed this object this turn, captured when the entry was created.
    ///
    /// Used to populate runtime tags for filters like "each creature that crewed it this turn".
    pub crew_contributors: Vec<ObjectId>,

    /// Creatures that saddled this object this turn, captured when the entry was created.
    ///
    /// Used to populate runtime tags for filters like "each creature that saddled it this turn".
    pub saddle_contributors: Vec<ObjectId>,
    /// Tagged object snapshots preserved from cost payment and targeting flows.
    ///
    /// This supports resolution-time references like `sacrifice_cost_0`.
    pub tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
}

/// A mana ability granted to a player until end of turn.
///
/// This models effects like Channel that temporarily give a player a mana ability
/// not tied to any permanent.
#[derive(Debug, Clone)]
pub struct GrantedManaAbility {
    pub controller: PlayerId,
    pub ability: crate::ability::ActivatedAbility,
    pub expires_end_of_turn: u32,
}

impl StackEntry {
    pub fn new(object_id: ObjectId, controller: PlayerId) -> Self {
        Self {
            object_id,
            controller,
            targets: Vec::new(),
            x_value: None,
            ability_effects: None,
            is_ability: false,
            casting_method: CastingMethod::Normal,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            saga_final_chapter_source: None,
            source_stable_id: None,
            source_snapshot: None,
            source_name: None,
            triggering_event: None,
            intervening_if: None,
            chosen_modes: None,
            keyword_payment_contributions: Vec::new(),
            crew_contributors: Vec::new(),
            saddle_contributors: Vec::new(),
            tagged_objects: std::collections::HashMap::new(),
        }
    }

    /// Create a stack entry for a triggered or activated ability.
    pub fn ability(
        source_id: ObjectId,
        controller: PlayerId,
        effects: Vec<crate::effect::Effect>,
    ) -> Self {
        Self {
            object_id: source_id,
            controller,
            targets: Vec::new(),
            x_value: None,
            ability_effects: Some(effects),
            is_ability: true,
            casting_method: CastingMethod::Normal,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            saga_final_chapter_source: None,
            source_stable_id: None,
            source_snapshot: None,
            source_name: None,
            triggering_event: None,
            intervening_if: None,
            chosen_modes: None,
            keyword_payment_contributions: Vec::new(),
            crew_contributors: Vec::new(),
            saddle_contributors: Vec::new(),
            tagged_objects: std::collections::HashMap::new(),
        }
    }

    /// Mark this as a saga's final chapter ability.
    pub fn with_saga_final_chapter(mut self, saga_id: ObjectId) -> Self {
        self.saga_final_chapter_source = Some(saga_id);
        self
    }

    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }

    pub fn with_x(mut self, x: u32) -> Self {
        self.x_value = Some(x);
        self
    }

    pub fn with_casting_method(mut self, method: CastingMethod) -> Self {
        self.casting_method = method;
        self
    }

    pub fn with_optional_costs_paid(mut self, paid: OptionalCostsPaid) -> Self {
        self.optional_costs_paid = paid;
        self
    }

    pub fn with_defending_player(mut self, player: PlayerId) -> Self {
        self.defending_player = Some(player);
        self
    }

    /// Set the source instance ID (stable identifier across zone changes).
    pub fn with_source_stable_id(mut self, stable_id: StableId) -> Self {
        self.source_stable_id = Some(stable_id);
        self
    }

    /// Set the source snapshot for source-LKI lookups during resolution.
    pub fn with_source_snapshot(mut self, snapshot: crate::snapshot::ObjectSnapshot) -> Self {
        self.source_snapshot = Some(snapshot);
        self
    }

    /// Set the source name for display purposes.
    pub fn with_source_name(mut self, name: String) -> Self {
        self.source_name = Some(name);
        self
    }

    /// Set both source instance ID and name from a source object.
    pub fn with_source_info(mut self, stable_id: StableId, name: String) -> Self {
        self.source_stable_id = Some(stable_id);
        self.source_name = Some(name);
        self
    }

    /// Set the triggering event for this triggered ability.
    pub fn with_triggering_event(mut self, event: crate::triggers::TriggerEvent) -> Self {
        self.triggering_event = Some(event);
        self
    }

    /// Set the intervening-if condition that must be true at resolution time.
    pub fn with_intervening_if(mut self, condition: crate::ConditionExpr) -> Self {
        self.intervening_if = Some(condition);
        self
    }

    /// Set pre-chosen modes for modal spells (per MTG rule 601.2b).
    pub fn with_chosen_modes(mut self, modes: Option<Vec<usize>>) -> Self {
        self.chosen_modes = modes;
        self
    }

    /// Set keyword-ability payment contributors for this stack entry.
    pub fn with_keyword_payment_contributions(
        mut self,
        contributions: Vec<KeywordPaymentContribution>,
    ) -> Self {
        self.keyword_payment_contributions = contributions;
        self
    }

    /// Carry tagged object snapshots into stack resolution context.
    pub fn with_tagged_objects(
        mut self,
        tagged: std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    ) -> Self {
        self.tagged_objects = tagged;
        self
    }
}

/// Complete game state.
#[derive(Debug, Clone)]
pub struct GameState {
    // Players
    pub players: Vec<Player>,
    pub turn_order: Vec<PlayerId>,

    // Objects
    objects: HashMap<ObjectId, Object>,

    // The stack
    pub stack: Vec<StackEntry>,

    // Zone indexes (denormalized for efficiency)
    pub battlefield: Vec<ObjectId>,
    pub command_zone: Vec<ObjectId>,
    pub exile: Vec<ObjectId>,

    // Turn tracking
    pub turn: TurnState,

    // Effect managers
    pub continuous_effects: ContinuousEffectManager,
    pub replacement_effects: ReplacementEffectManager,
    pub prevention_effects: PreventionEffectManager,

    /// Tracker for "can't" effects (Rule 614.17).
    /// These are checked BEFORE events happen, not as replacements.
    pub cant_effects: CantEffectTracker,
    /// Tracker for "spend mana as though it were mana of any color" effects.
    pub mana_spend_effects: ManaSpendEffectTracker,

    // Delayed triggers waiting to fire
    pub delayed_triggers: Vec<crate::triggers::DelayedTrigger>,

    /// Pending trigger events generated by effects.
    /// Effects (like VoteEffect) can push events here, and the game loop
    /// processes them after effect resolution.
    pub pending_trigger_events: Vec<crate::triggers::TriggerEvent>,

    /// Current combat state (Some during combat phase, None otherwise).
    /// Effects can directly add creatures to combat when this is set.
    pub combat: Option<crate::combat_state::CombatState>,
    /// Whether the game is currently in night mode (day/night designation).
    pub is_night: bool,
    /// Current monarch designation holder, if any.
    pub monarch: Option<PlayerId>,

    /// Tracks activated abilities that have been used this turn.
    /// Used for OncePerTurn timing restrictions.
    /// Key is (source ObjectId, ability index within that object).
    pub activated_abilities_this_turn: HashSet<(ObjectId, usize)>,

    /// Tracks modal choices that were already selected for an activated ability.
    /// Key is (source ObjectId, ability index), value is the set of chosen mode indices.
    pub chosen_modes_by_ability: HashMap<(ObjectId, usize), HashSet<usize>>,

    /// Tracks per-turn modal choices for abilities with "that hasn't been chosen this turn".
    pub chosen_modes_by_ability_this_turn: HashMap<(ObjectId, usize), HashSet<usize>>,

    /// Tracks how many cards each player has drawn this turn.
    /// Used for "first card drawn this turn" triggers and replacement effects.
    /// Reset at the start of each turn.
    pub cards_drawn_this_turn: HashMap<PlayerId, u32>,

    /// Pending replacement effect choice when multiple effects could apply.
    /// When set, advance_priority returns a ChooseReplacementEffect decision
    /// before continuing with normal game flow.
    pub pending_replacement_choice: Option<PendingReplacementChoice>,

    /// Registry for tracking granted alternative casts and abilities.
    pub grant_registry: crate::grant_registry::GrantRegistry,

    /// Extra turns queued up (Time Walk, etc.).
    /// Players take these turns in order after the current turn ends.
    pub extra_turns: Vec<PlayerId>,

    /// Players who will skip their next turn.
    /// Checked and cleared when a player would start their turn.
    pub skip_next_turn: HashSet<PlayerId>,
    /// Players who will skip their next draw step.
    /// Checked and cleared when a player would draw in draw step.
    pub skip_next_draw_step: HashSet<PlayerId>,
    /// Players who will skip all combat phases on their next turn.
    /// Checked and cleared when entering combat phase.
    pub skip_next_combat_phases: HashSet<PlayerId>,

    /// Active and pending player-control effects.
    pub player_control_effects: Vec<PlayerControlEffect>,

    /// Timestamp counter for player-control effects.
    pub player_control_timestamp: u64,

    /// Number of creatures that have died this turn.
    /// Reset at the start of each turn.
    pub creatures_died_this_turn: u32,

    /// Number of creatures that died this turn per controller (at the time they died).
    /// Reset at the start of each turn.
    pub creatures_died_under_controller_this_turn: HashMap<PlayerId, u32>,
    /// Number of creatures that left the battlefield this turn per controller.
    /// Reset at the start of each turn.
    pub creatures_left_battlefield_under_controller_this_turn: HashMap<PlayerId, u32>,
    /// Number of permanents that left the battlefield this turn per controller.
    /// Reset at the start of each turn.
    pub permanents_left_battlefield_under_controller_this_turn: HashMap<PlayerId, u32>,

    /// Cards/tokens that were put into a graveyard from anywhere this turn.
    ///
    /// Tracked by stable ID so zone changes (Rule 400.7) don't lose identity.
    /// Reset at the start of each turn.
    pub objects_put_into_graveyard_this_turn: HashSet<StableId>,

    /// Cards/tokens that were put into a graveyard from the battlefield this turn.
    ///
    /// Tracked by stable ID so zone changes (Rule 400.7) don't lose identity.
    /// Reset at the start of each turn.
    pub objects_put_into_graveyard_from_battlefield_this_turn: HashSet<StableId>,

    /// Number of times each (source stable id, trigger identity) has fired this turn.
    pub triggers_fired_this_turn: HashMap<(ObjectId, TriggerIdentity), u32>,

    /// Extensible per-turn counters for event kinds and custom metrics.
    pub turn_counters: TurnCounterTracker,

    /// Number of spells cast this turn per player.
    /// Reset at the start of each turn.
    pub spells_cast_this_turn: HashMap<PlayerId, u32>,
    /// Number of crimes committed this turn per player.
    /// Reset at the start of each turn.
    pub crimes_committed_this_turn: HashMap<PlayerId, u32>,
    /// Number of artifacts sacrificed this turn per player.
    /// Reset at the start of each turn.
    pub artifacts_sacrificed_this_turn: HashMap<PlayerId, u32>,

    /// Total mana spent to cast spells this turn per player.
    ///
    /// Used by the Expend mechanic: "You expend N as you spend your Nth total mana to cast spells during a turn."
    ///
    /// Reset at the start of each turn.
    pub mana_spent_to_cast_spells_this_turn: HashMap<PlayerId, u32>,

    /// Total number of spells cast this turn.
    /// Reset at the start of each turn.
    pub spells_cast_this_turn_total: u32,

    /// Snapshots of spells cast this turn, captured at cast time.
    ///
    /// This supports "spells you've cast this turn" counts that need to persist
    /// even after spells resolve or change zones.
    ///
    /// Cleared at the start of each turn.
    pub spells_cast_this_turn_snapshots: Vec<crate::snapshot::ObjectSnapshot>,

    /// Cast order for spells cast this turn (object id -> 1-based cast index).
    /// Cleared at the start of each turn.
    pub spell_cast_order_this_turn: HashMap<ObjectId, u32>,

    /// Players who attacked with at least one creature this turn.
    /// Reset at the start of each turn.
    pub players_attacked_this_turn: HashSet<PlayerId>,

    /// Players who tapped a land for mana this turn.
    /// Reset at the start of each turn.
    pub players_tapped_land_for_mana_this_turn: HashSet<PlayerId>,

    /// Creatures that attacked this turn.
    /// Used for activated ability restrictions like "Activate only if this creature attacked this turn".
    pub creatures_attacked_this_turn: HashSet<ObjectId>,

    /// Total number of spells cast during the immediately previous turn.
    /// Updated when turn advances.
    pub spells_cast_last_turn_total: u32,

    /// Players who searched their library this turn.
    /// Used for trap conditions like Archive Trap.
    pub library_searches_this_turn: HashSet<PlayerId>,

    /// Number of creatures that entered the battlefield this turn, per controller.
    /// Used for trap conditions like Balustrade Spy.
    pub creatures_entered_this_turn: HashMap<PlayerId, u32>,

    /// Objects that entered the battlefield this turn, keyed by stable ID with entry controller.
    /// Reset at the start of each turn.
    pub objects_entered_battlefield_this_turn: HashMap<StableId, PlayerId>,

    /// Creatures that crewed a Vehicle this turn, keyed by Vehicle object id.
    ///
    /// Used for references like "each creature that crewed it this turn".
    /// Cleared at the start of each turn.
    pub crewed_this_turn: HashMap<ObjectId, Vec<ObjectId>>,

    /// Mounts that are saddled until end of turn.
    ///
    /// Cleared at the start of each turn.
    pub saddled_until_end_of_turn: HashSet<ObjectId>,

    /// Creatures that saddled a Mount this turn, keyed by Mount object id.
    ///
    /// Used for references like "each creature that saddled it this turn".
    /// Cleared at the start of each turn.
    pub saddled_this_turn: HashMap<ObjectId, Vec<ObjectId>>,

    /// Soulbond pairings (stored bidirectionally: A -> B and B -> A).
    pub soulbond_pairs: HashMap<ObjectId, ObjectId>,

    /// Attack targets captured while paying Ninjutsu costs, keyed by the
    /// source card object ID in hand.
    ///
    /// Multiple entries per source are stored in activation order so nested
    /// activations can resolve LIFO.
    pub ninjutsu_attack_targets: HashMap<ObjectId, Vec<crate::combat_state::AttackTarget>>,

    /// Damage dealt to each player by creatures this turn.
    /// Used for trap conditions like Summoning Trap.
    pub creature_damage_to_players_this_turn: HashMap<PlayerId, u32>,

    /// Damage dealt to each player this turn (from any source).
    /// Used for mechanics like Bloodthirst.
    pub damage_to_players_this_turn: HashMap<PlayerId, u32>,

    /// Noncombat damage dealt to each player this turn.
    pub noncombat_damage_to_players_this_turn: HashMap<PlayerId, u32>,

    /// Life lost by each player this turn.
    pub life_lost_this_turn: HashMap<PlayerId, u32>,

    /// Life gained by each player this turn.
    pub life_gained_this_turn: HashMap<PlayerId, u32>,

    /// Creatures that have been dealt damage this turn, keyed by the damaged creature.
    ///
    /// Value is the set of object IDs that dealt damage to that creature this turn.
    /// Cleared at the start of each turn.
    pub creatures_damaged_by_this_turn: HashMap<ObjectId, HashSet<ObjectId>>,

    /// Combat-damage-to-player hits already processed in the current trigger batch.
    /// Used for "one or more ... deal combat damage to a player" trigger matching.
    pub combat_damage_player_batch_hits: Vec<(ObjectId, PlayerId)>,

    /// Temporary mana abilities granted to players (e.g., Channel), expiring at end of turn.
    pub granted_mana_abilities: Vec<GrantedManaAbility>,

    /// Active restriction effects (spell/ability-based "can't" effects).
    pub restriction_effects: Vec<RestrictionEffectInstance>,

    /// Active goad effects (a creature attacks each combat and attacks a player
    /// other than the goader if able).
    pub goad_effects: Vec<GoadEffectInstance>,

    // =========================================================================
    // Battlefield State Extension Maps
    // =========================================================================
    // These track state that was previously on Object but is only relevant
    // for permanents on the battlefield. Cleared when objects leave battlefield.
    /// Tapped permanents on the battlefield.
    pub tapped_permanents: HashSet<ObjectId>,

    /// Creatures that have summoning sickness.
    pub summoning_sick: HashSet<ObjectId>,

    /// Damage marked on creatures (cleared at cleanup step).
    pub damage_marked: HashMap<ObjectId, u32>,

    /// Permanents whose damage is not removed during cleanup.
    pub damage_persists: HashSet<ObjectId>,

    /// Chosen colors for permanents ("as this enters, choose a color").
    pub chosen_colors: HashMap<ObjectId, crate::color::Color>,

    /// Chosen basic land types for permanents ("as this Aura enters, choose a basic land type").
    pub chosen_basic_land_types: HashMap<ObjectId, crate::types::Subtype>,

    /// Regeneration shields on permanents (expires at end of turn).
    pub regeneration_shields: HashMap<ObjectId, u32>,

    /// Creatures that are monstrous (from monstrosity ability).
    pub monstrous: HashSet<ObjectId>,

    /// Creatures that are renowned.
    pub renowned: HashSet<ObjectId>,

    /// Flipped permanents (for flip cards like Budoka Gardener).
    pub flipped: HashSet<ObjectId>,

    /// Face-down permanents (for morph, manifest, etc.).
    pub face_down: HashSet<ObjectId>,

    /// Phased-out permanents.
    pub phased_out: HashSet<ObjectId>,

    /// Cards exiled via Madness (can be cast from exile for madness cost).
    pub madness_exiled: HashSet<ObjectId>,

    /// Sagas whose final chapter ability has resolved (ready to be sacrificed).
    pub saga_final_chapter_resolved: HashSet<ObjectId>,

    /// Objects designated as commanders.
    pub commanders: HashSet<ObjectId>,

    /// Imprinted cards - maps a permanent to the card(s) exiled with it via imprint.
    /// Used by Chrome Mox, Isochron Scepter, etc.
    pub imprinted_cards: HashMap<ObjectId, Vec<ObjectId>>,

    /// Cards exiled by a specific source object ID.
    ///
    /// This powers "cards exiled with <this object>" style references.
    pub exiled_with_source: HashMap<ObjectId, Vec<ObjectId>>,

    /// Linked exile groups keyed by generated runtime ID.
    pub linked_exile_groups: HashMap<u64, LinkedExileGroup>,

    /// Monotonic ID generator for linked exile groups.
    pub next_linked_exile_group_id: u64,
}

impl GameState {
    /// Creates a new game state with the given players.
    pub fn new(player_names: Vec<String>, starting_life: i32) -> Self {
        let players: Vec<Player> = player_names
            .into_iter()
            .enumerate()
            .map(|(i, name)| Player::new(PlayerId::from_index(i as u8), name, starting_life))
            .collect();

        let turn_order: Vec<PlayerId> = players.iter().map(|p| p.id).collect();
        let active_player = turn_order
            .first()
            .copied()
            .unwrap_or(PlayerId::from_index(0));

        Self {
            players,
            turn_order,
            objects: HashMap::new(),
            stack: Vec::new(),
            battlefield: Vec::new(),
            command_zone: Vec::new(),
            exile: Vec::new(),
            turn: TurnState::new(active_player),
            continuous_effects: ContinuousEffectManager::new(),
            replacement_effects: ReplacementEffectManager::new(),
            prevention_effects: PreventionEffectManager::new(),
            cant_effects: CantEffectTracker::new(),
            mana_spend_effects: ManaSpendEffectTracker::new(),
            delayed_triggers: Vec::new(),
            pending_trigger_events: Vec::new(),
            combat: None,
            is_night: false,
            monarch: None,
            activated_abilities_this_turn: HashSet::new(),
            chosen_modes_by_ability: HashMap::new(),
            chosen_modes_by_ability_this_turn: HashMap::new(),
            cards_drawn_this_turn: HashMap::new(),
            pending_replacement_choice: None,
            grant_registry: crate::grant_registry::GrantRegistry::new(),
            extra_turns: Vec::new(),
            skip_next_turn: HashSet::new(),
            skip_next_draw_step: HashSet::new(),
            skip_next_combat_phases: HashSet::new(),
            player_control_effects: Vec::new(),
            player_control_timestamp: 0,
            creatures_died_this_turn: 0,
            creatures_died_under_controller_this_turn: HashMap::new(),
            creatures_left_battlefield_under_controller_this_turn: HashMap::new(),
            permanents_left_battlefield_under_controller_this_turn: HashMap::new(),
            objects_put_into_graveyard_this_turn: HashSet::new(),
            objects_put_into_graveyard_from_battlefield_this_turn: HashSet::new(),
            triggers_fired_this_turn: HashMap::new(),
            turn_counters: TurnCounterTracker::default(),
            spells_cast_this_turn: HashMap::new(),
            crimes_committed_this_turn: HashMap::new(),
            artifacts_sacrificed_this_turn: HashMap::new(),
            mana_spent_to_cast_spells_this_turn: HashMap::new(),
            spells_cast_this_turn_total: 0,
            spells_cast_this_turn_snapshots: Vec::new(),
            spell_cast_order_this_turn: HashMap::new(),
            players_attacked_this_turn: HashSet::new(),
            players_tapped_land_for_mana_this_turn: HashSet::new(),
            creatures_attacked_this_turn: HashSet::new(),
            spells_cast_last_turn_total: 0,
            library_searches_this_turn: HashSet::new(),
            creatures_entered_this_turn: HashMap::new(),
            objects_entered_battlefield_this_turn: HashMap::new(),
            crewed_this_turn: HashMap::new(),
            saddled_until_end_of_turn: HashSet::new(),
            saddled_this_turn: HashMap::new(),
            soulbond_pairs: HashMap::new(),
            ninjutsu_attack_targets: HashMap::new(),
            creature_damage_to_players_this_turn: HashMap::new(),
            damage_to_players_this_turn: HashMap::new(),
            noncombat_damage_to_players_this_turn: HashMap::new(),
            life_lost_this_turn: HashMap::new(),
            life_gained_this_turn: HashMap::new(),
            creatures_damaged_by_this_turn: HashMap::new(),
            combat_damage_player_batch_hits: Vec::new(),
            granted_mana_abilities: Vec::new(),
            restriction_effects: Vec::new(),
            goad_effects: Vec::new(),
            // Battlefield state extension maps
            tapped_permanents: HashSet::new(),
            summoning_sick: HashSet::new(),
            damage_marked: HashMap::new(),
            damage_persists: HashSet::new(),
            chosen_colors: HashMap::new(),
            chosen_basic_land_types: HashMap::new(),
            regeneration_shields: HashMap::new(),
            monstrous: HashSet::new(),
            renowned: HashSet::new(),
            flipped: HashSet::new(),
            face_down: HashSet::new(),
            phased_out: HashSet::new(),
            madness_exiled: HashSet::new(),
            saga_final_chapter_resolved: HashSet::new(),
            commanders: HashSet::new(),
            imprinted_cards: HashMap::new(),
            exiled_with_source: HashMap::new(),
            linked_exile_groups: HashMap::new(),
            next_linked_exile_group_id: 0,
        }
    }

    /// Generates a new unique object ID.
    pub fn new_object_id(&mut self) -> ObjectId {
        // Use global atomic counter for ID generation
        ObjectId::new()
    }

    pub fn add_restriction_effect(
        &mut self,
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        source: ObjectId,
        controller: PlayerId,
    ) {
        let expires_end_of_turn = match duration {
            crate::effect::Until::EndOfTurn => self.turn.turn_number,
            crate::effect::Until::Forever => u32::MAX,
            _ => self.turn.turn_number,
        };

        self.restriction_effects.push(RestrictionEffectInstance {
            restriction,
            controller,
            source,
            duration,
            expires_end_of_turn,
        });
    }

    pub fn add_goad_effect(
        &mut self,
        creature: ObjectId,
        goaded_by: PlayerId,
        duration: crate::effect::Until,
        source: ObjectId,
    ) {
        let expires_end_of_turn = match duration {
            crate::effect::Until::EndOfTurn => self.turn.turn_number,
            crate::effect::Until::Forever => u32::MAX,
            _ => self.turn.turn_number,
        };

        self.goad_effects.push(GoadEffectInstance {
            creature,
            goaded_by,
            source,
            duration,
            expires_end_of_turn,
        });
    }

    pub fn active_goaders_for(&self, creature: ObjectId) -> HashSet<PlayerId> {
        let current_turn = self.turn.turn_number;
        self.goad_effects
            .iter()
            .filter(|effect| effect.creature == creature && effect.is_active(self, current_turn))
            .map(|effect| effect.goaded_by)
            .collect()
    }

    pub fn is_goaded(&self, creature: ObjectId) -> bool {
        !self.active_goaders_for(creature).is_empty()
    }

    pub fn cleanup_restrictions_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        self.restriction_effects.retain(|effect| {
            !matches!(effect.duration, crate::effect::Until::EndOfTurn)
                || effect.expires_end_of_turn > current_turn
        });
    }

    pub fn cleanup_granted_mana_abilities_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        self.granted_mana_abilities
            .retain(|grant| grant.expires_end_of_turn > current_turn);
    }

    /// Can the player draw any cards?
    pub fn can_draw(&self, player: PlayerId) -> bool {
        self.cant_effects.can_draw(player)
    }

    /// Can the player gain life?
    pub fn can_gain_life(&self, player: PlayerId) -> bool {
        self.cant_effects.can_gain_life(player)
    }

    /// Can the player lose life (not from damage)?
    pub fn can_lose_life(&self, player: PlayerId) -> bool {
        self.cant_effects.can_lose_life(player)
    }

    /// Can the player's life total change?
    pub fn can_change_life_total(&self, player: PlayerId) -> bool {
        self.cant_effects.can_change_life_total(player)
    }

    /// Returns true if a player can currently pay the given amount of life.
    pub fn can_pay_life(&self, player: PlayerId, amount: u32) -> bool {
        if amount == 0 {
            return self.player(player).is_some();
        }
        self.can_change_life_total(player)
            && self.player(player).is_some_and(|p| p.life >= amount as i32)
    }

    /// Makes a player lose life if their life total can change.
    ///
    /// Returns the amount of life actually lost.
    pub fn lose_life(&mut self, player: PlayerId, amount: u32) -> u32 {
        if amount == 0 || !self.can_change_life_total(player) {
            return 0;
        }
        if let Some(p) = self.player_mut(player) {
            p.lose_life(amount);
            return amount;
        }
        0
    }

    /// Pays life as a cost.
    ///
    /// Returns true if the player could pay and life was deducted.
    pub fn pay_life(&mut self, player: PlayerId, amount: u32) -> bool {
        if amount == 0 {
            return self.player(player).is_some();
        }
        if !self.can_pay_life(player, amount) {
            return false;
        }
        self.lose_life(player, amount) == amount
    }

    /// Can the player search their library?
    pub fn can_search_library(&self, player: PlayerId) -> bool {
        self.cant_effects.can_search_library(player)
    }

    /// Can the player draw extra cards this turn?
    pub fn can_draw_extra_cards(&self, player: PlayerId) -> bool {
        self.cant_effects.can_draw_extra_cards(player)
    }

    /// Can the creature attack?
    pub fn can_attack(&self, creature: ObjectId) -> bool {
        self.cant_effects.can_attack(creature)
    }

    /// Can the creature attack as the only attacker?
    pub fn can_attack_alone(&self, creature: ObjectId) -> bool {
        self.cant_effects.can_attack_alone(creature)
    }

    /// Can the creature block?
    pub fn can_block(&self, creature: ObjectId) -> bool {
        self.cant_effects.can_block(creature)
    }

    /// Can the creature block a specific attacker?
    pub fn can_block_attacker(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        self.cant_effects.can_block_attacker(blocker, attacker)
    }

    /// Must the creature block a specific attacker this turn if able?
    pub fn must_block_attacker(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        self.cant_effects.must_block_attacker(blocker, attacker)
    }

    /// Get required attackers for a blocker, if any.
    pub fn required_attackers_for_blocker(&self, blocker: ObjectId) -> Option<&HashSet<ObjectId>> {
        self.cant_effects.required_attackers_for_blocker(blocker)
    }

    /// Can the creature block as the only blocker?
    pub fn can_block_alone(&self, creature: ObjectId) -> bool {
        self.cant_effects.can_block_alone(creature)
    }

    /// Can the permanent untap during untap step?
    pub fn can_untap(&self, permanent: ObjectId) -> bool {
        self.cant_effects.can_untap(permanent)
    }

    /// Can damage be prevented?
    pub fn can_prevent_damage(&self) -> bool {
        self.cant_effects.can_prevent_damage()
    }

    /// Can the permanent be destroyed?
    pub fn can_be_destroyed(&self, permanent: ObjectId) -> bool {
        self.cant_effects.can_be_destroyed(permanent)
    }

    /// Can the permanent be regenerated?
    pub fn can_be_regenerated(&self, permanent: ObjectId) -> bool {
        self.cant_effects.can_be_regenerated(permanent)
    }

    /// Can the permanent be sacrificed?
    pub fn can_be_sacrificed(&self, permanent: ObjectId) -> bool {
        self.cant_effects.can_be_sacrificed(permanent)
    }

    /// Can the creature be blocked?
    pub fn can_be_blocked(&self, creature: ObjectId) -> bool {
        self.cant_effects.can_be_blocked(creature)
    }

    /// Can the player lose the game?
    pub fn can_lose_game(&self, player: PlayerId) -> bool {
        self.cant_effects.can_lose_game(player)
    }

    /// Can the player win the game?
    pub fn can_win_game(&self, player: PlayerId) -> bool {
        self.cant_effects.can_win_game(player)
    }

    /// Can the player cast spells?
    pub fn can_cast_spells(&self, player: PlayerId) -> bool {
        self.cant_effects.can_cast_spells(player)
    }

    /// Can the player activate non-mana abilities?
    pub fn can_activate_non_mana_abilities(&self, player: PlayerId) -> bool {
        self.cant_effects.can_activate_non_mana_abilities(player)
    }

    /// Can activated abilities of this permanent be activated (including mana abilities)?
    pub fn can_activate_abilities_of(&self, source: ObjectId) -> bool {
        self.cant_effects.can_activate_abilities_of(source)
    }

    /// Can activated abilities with {T} in their costs of this permanent be activated?
    pub fn can_activate_tap_abilities_of(&self, source: ObjectId) -> bool {
        self.cant_effects.can_activate_tap_abilities_of(source)
    }

    /// Can non-mana activated abilities of this permanent be activated?
    pub fn can_activate_non_mana_abilities_of(&self, source: ObjectId) -> bool {
        self.cant_effects.can_activate_non_mana_abilities_of(source)
    }

    /// Can the player cast creature spells?
    pub fn can_cast_creature_spells(&self, player: PlayerId) -> bool {
        self.cant_effects.can_cast_creature_spells(player)
    }

    /// Can the player cast another spell this turn?
    pub fn can_cast_additional_spell_this_turn(&self, player: PlayerId) -> bool {
        self.cant_effects
            .can_cast_additional_spell_this_turn(player)
    }

    /// Can the player cast another noncreature spell this turn?
    pub fn can_cast_additional_noncreature_spell_this_turn(&self, player: PlayerId) -> bool {
        self.cant_effects
            .can_cast_additional_noncreature_spell_this_turn(player)
    }

    /// Can the player cast another nonartifact spell this turn?
    pub fn can_cast_additional_nonartifact_spell_this_turn(&self, player: PlayerId) -> bool {
        self.cant_effects
            .can_cast_additional_nonartifact_spell_this_turn(player)
    }

    /// Can the player cast another non-Phyrexian spell this turn?
    pub fn can_cast_additional_nonphyrexian_spell_this_turn(&self, player: PlayerId) -> bool {
        self.cant_effects
            .can_cast_additional_nonphyrexian_spell_this_turn(player)
    }

    /// Can counters be placed on this permanent?
    pub fn can_have_counters_placed(&self, permanent: ObjectId) -> bool {
        self.cant_effects.can_have_counters_placed(permanent)
    }

    /// Is this permanent untargetable (by shroud/hexproof-style effects)?
    pub fn is_untargetable(&self, permanent: ObjectId) -> bool {
        self.cant_effects.is_untargetable(permanent)
    }

    /// Can this player be targeted?
    pub fn can_target_player(&self, player: PlayerId) -> bool {
        self.cant_effects.can_target_player(player)
    }

    /// Can this spell on the stack be countered?
    pub fn can_be_countered(&self, spell: ObjectId) -> bool {
        self.cant_effects.can_be_countered(spell)
    }

    /// Can this permanent transform?
    pub fn can_transform(&self, permanent: ObjectId) -> bool {
        self.cant_effects.can_transform(permanent)
    }

    /// Adds an object to the game.
    pub fn add_object(&mut self, object: Object) {
        let zone = object.zone;
        let id = object.id;
        let owner = object.owner;

        self.objects.insert(id, object);

        // Update zone indexes
        match zone {
            Zone::Battlefield => self.battlefield.push(id),
            Zone::Command => self.command_zone.push(id),
            Zone::Exile => self.exile.push(id),
            Zone::Library => {
                if let Some(player) = self.player_mut(owner) {
                    player.library.push(id);
                }
            }
            Zone::Hand => {
                if let Some(player) = self.player_mut(owner) {
                    player.hand.push(id);
                }
            }
            Zone::Graveyard => {
                if let Some(player) = self.player_mut(owner) {
                    player.graveyard.push(id);
                }
            }
            Zone::Stack => {
                // Stack entries are managed separately via StackEntry
            }
        }

        // Validate zone consistency in debug builds
        #[cfg(debug_assertions)]
        self.debug_assert_zone_consistency();
    }

    /// Creates an object from a card and adds it to the specified zone.
    pub fn create_object_from_card(
        &mut self,
        card: &Card,
        owner: PlayerId,
        zone: Zone,
    ) -> ObjectId {
        let id = self.new_object_id();
        let object = Object::from_card(id, card, owner, zone);
        self.add_object(object);
        if zone == Zone::Battlefield {
            // Seed battlefield objects with an entry timestamp so layer timestamp
            // ordering is deterministic (replay setup, fixtures, etc.).
            self.continuous_effects.record_entry(id);
        }
        id
    }

    /// Creates an object from a CardDefinition (includes abilities and spell effects).
    pub fn create_object_from_definition(
        &mut self,
        def: &crate::cards::CardDefinition,
        owner: PlayerId,
        zone: Zone,
    ) -> ObjectId {
        let id = self.new_object_id();
        let object = Object::from_card_definition(id, def, owner, zone);
        self.add_object(object);
        if zone == Zone::Battlefield {
            // Seed battlefield objects with an entry timestamp so static ability
            // effects use proper timestamp order in layers.
            self.continuous_effects.record_entry(id);
        }
        id
    }

    /// Draws cards for a player, moving them from library to hand.
    /// Uses move_object to properly update the object's zone.
    /// Returns the new ObjectIds of the drawn cards.
    pub fn draw_cards(&mut self, player: PlayerId, count: usize) -> Vec<ObjectId> {
        let mut drawn = Vec::new();
        for _ in 0..count {
            // Get the top card of the library (last element)
            let card_id = if let Some(player_obj) = self.player(player) {
                player_obj.library.last().copied()
            } else {
                None
            };

            if let Some(id) = card_id {
                // Move from library to hand
                if let Some(new_id) = self.move_object(id, Zone::Hand) {
                    drawn.push(new_id);
                }
            } else {
                // Can't draw from empty library
                break;
            }
        }
        drawn
    }

    /// Moves an object to a new zone.
    /// Per MTG rule 400.7, this creates a new object (new ID).
    /// Returns the new ObjectId.
    pub fn move_object(&mut self, old_id: ObjectId, new_zone: Zone) -> Option<ObjectId> {
        // Capture a full pre-move snapshot for LKI-based trigger matching.
        let pre_move_snapshot = self
            .objects
            .get(&old_id)
            .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, self));

        let old_object = self.objects.remove(&old_id)?;
        let old_zone = old_object.zone;
        let owner = old_object.owner;
        let controller = old_object.controller;

        // Track creature deaths (battlefield to graveyard) and record pending death event
        let is_creature_dying = old_zone == Zone::Battlefield
            && new_zone == Zone::Graveyard
            && old_object.is_creature();
        if is_creature_dying {
            self.creatures_died_this_turn += 1;
            *self
                .creatures_died_under_controller_this_turn
                .entry(controller)
                .or_insert(0) += 1;
        }
        if old_zone == Zone::Battlefield
            && new_zone != Zone::Battlefield
            && old_object.is_creature()
        {
            *self
                .creatures_left_battlefield_under_controller_this_turn
                .entry(controller)
                .or_insert(0) += 1;
        }
        if old_zone == Zone::Battlefield && new_zone != Zone::Battlefield {
            *self
                .permanents_left_battlefield_under_controller_this_turn
                .entry(controller)
                .or_insert(0) += 1;
        }
        if new_zone == Zone::Graveyard {
            self.objects_put_into_graveyard_this_turn
                .insert(old_object.stable_id);
        }
        if old_zone == Zone::Battlefield && new_zone == Zone::Graveyard {
            self.objects_put_into_graveyard_from_battlefield_this_turn
                .insert(old_object.stable_id);
        }
        if new_zone == Zone::Battlefield {
            self.objects_entered_battlefield_this_turn
                .insert(old_object.stable_id, controller);
        }

        // Remove from old zone index
        self.remove_from_zone_index(old_id, old_zone, owner);

        // Clear state from old zone's extension maps
        if old_zone == Zone::Battlefield {
            self.clear_battlefield_state(old_id);
            self.clear_player_control_from_source(old_object.stable_id);
        }
        if old_zone == Zone::Exile {
            self.clear_exile_state(old_id);
        }

        // Create new object with new ID (zone change = new object per rule 400.7)
        let new_id = self.new_object_id();
        let mut new_object = old_object;
        new_object.id = new_id;
        new_object.zone = new_zone;

        // Reset zone-specific state on the object
        new_object.attached_to = None;
        new_object.attachments.clear();
        // Casting-contribution state should not persist across arbitrary zone changes.
        // Preserve it only for Stack -> Battlefield (a spell resolving into a permanent).
        if !(old_zone == Zone::Stack && new_zone == Zone::Battlefield) {
            new_object.keyword_payment_contributions_to_cast.clear();
            new_object.x_value = None;
            new_object.bestow_cast_state = None;
        }

        // Set battlefield state for new permanents
        if new_zone == Zone::Battlefield {
            self.set_summoning_sick(new_id);
        }

        self.add_object(new_object);

        // Record entry timestamp per Rule 613.7d when entering the battlefield
        if new_zone == Zone::Battlefield {
            self.continuous_effects.record_entry(new_id);
        }

        // Queue zone change event for triggers.
        if old_zone != new_zone {
            use crate::events::zones::ZoneChangeEvent;
            use crate::triggers::TriggerEvent;

            // For LTB-style moves we keep the pre-move object ID; for all others use
            // the destination object ID so ETB/"this enters" matching remains stable.
            let event_object_id = if old_zone == Zone::Battlefield {
                old_id
            } else {
                new_id
            };
            let event =
                ZoneChangeEvent::new(event_object_id, old_zone, new_zone, pre_move_snapshot);
            self.queue_trigger_event(TriggerEvent::new(event));
        }

        // Validate zone consistency in debug builds
        #[cfg(debug_assertions)]
        self.debug_assert_zone_consistency();

        Some(new_id)
    }

    /// Move an object to the battlefield with ETB replacement effect processing.
    ///
    /// This processes replacement effects that modify how a permanent enters the battlefield:
    /// - "Enters tapped" effects (from the permanent itself or other sources)
    /// - "Enters with N counters" effects
    /// - "If this would enter the battlefield, exile it instead"
    ///
    /// For moves TO the battlefield, this should be used instead of `move_object`
    /// to ensure replacement effects are properly applied.
    pub fn move_object_with_etb_processing(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
    ) -> Option<EntersResult> {
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        self.move_object_with_etb_processing_with_dm(old_id, new_zone, &mut dm)
    }

    /// Move an object to the battlefield with ETB replacement processing and decisions.
    pub fn move_object_with_etb_processing_with_dm(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        decision_maker: &mut impl crate::decision::DecisionMaker,
    ) -> Option<EntersResult> {
        let old_zone = self.object(old_id)?.zone;

        // Only process ETB replacement for moves TO the battlefield
        if new_zone != Zone::Battlefield {
            let new_id = self.move_object(old_id, new_zone)?;
            return Some(EntersResult {
                new_id,
                enters_tapped: false,
            });
        }

        // Process through ETB replacement effects
        let result = crate::event_processor::process_etb_with_event_and_dm(
            self,
            old_id,
            old_zone,
            decision_maker,
        );

        // If ETB was prevented or redirected to a different zone
        if result.prevented {
            if let Some(dest) = result.new_destination {
                // Move to the alternate destination
                let new_id = self.move_object(old_id, dest)?;
                return Some(EntersResult {
                    new_id,
                    enters_tapped: false,
                });
            }
            return None;
        }

        // Proceed with normal battlefield entry
        let new_id = self.move_object(old_id, Zone::Battlefield)?;

        // Apply "enters as copy" before tapped/counter modifications.
        if let Some(copy_source_id) = result.enters_as_copy_of {
            let copy_source = self.object(copy_source_id).cloned();
            if let (Some(source_obj), Some(new_obj)) = (copy_source, self.object_mut(new_id)) {
                new_obj.copy_copiable_values_from(&source_obj);
            }
        }

        // Apply enters tapped
        if result.enters_tapped {
            self.tap(new_id);
        }

        // Apply enters with counters
        for (counter_type, count) in &result.enters_with_counters {
            if let Some(obj) = self.object_mut(new_id) {
                *obj.counters.entry(*counter_type).or_insert(0) += count;
            }
        }

        // Apply "as this enters, choose a color" selections.
        let choose_color_abilities = self
            .object(new_id)
            .map(|obj| (obj.controller, obj.abilities.clone()));
        if let Some((controller, abilities)) = choose_color_abilities {
            for ability in abilities {
                if let crate::ability::AbilityKind::Static(static_ability) = &ability.kind {
                    if let Some(spec) = static_ability.color_choice_as_enters() {
                        let mut options = vec![
                            crate::color::Color::White,
                            crate::color::Color::Blue,
                            crate::color::Color::Black,
                            crate::color::Color::Red,
                            crate::color::Color::Green,
                        ];
                        if let Some(excluded) = spec.excluded {
                            options.retain(|color| *color != excluded);
                        }
                        if options.is_empty() {
                            continue;
                        }
                        let choice_spec = crate::decisions::specs::ManaColorsSpec::restricted(
                            new_id,
                            1,
                            true,
                            options.clone(),
                        );
                        let mut chosen = crate::decisions::make_decision(
                            self,
                            decision_maker,
                            controller,
                            Some(new_id),
                            choice_spec,
                        );
                        let chosen_color = chosen.pop().unwrap_or(options[0]);
                        self.set_chosen_color(new_id, chosen_color);
                    }
                    if static_ability.basic_land_type_choice_as_enters().is_some() {
                        let options = [
                            crate::types::Subtype::Plains,
                            crate::types::Subtype::Island,
                            crate::types::Subtype::Swamp,
                            crate::types::Subtype::Mountain,
                            crate::types::Subtype::Forest,
                        ];
                        let display_options = options
                            .iter()
                            .enumerate()
                            .map(|(idx, subtype)| {
                                crate::decisions::spec::DisplayOption::new(
                                    idx,
                                    format!("{subtype:?}"),
                                )
                            })
                            .collect::<Vec<_>>();
                        let choice_spec =
                            crate::decisions::specs::ChoiceSpec::single(new_id, display_options);
                        let mut chosen = crate::decisions::make_decision(
                            self,
                            decision_maker,
                            controller,
                            Some(new_id),
                            choice_spec,
                        );
                        let chosen_idx =
                            chosen.pop().filter(|idx| *idx < options.len()).unwrap_or(0);
                        self.set_chosen_basic_land_type(new_id, options[chosen_idx]);
                    }
                }
            }
        }

        // If this is an Aura entering from a non-stack zone, choose what to attach to
        if old_zone != Zone::Stack
            && let Some(obj) = self.object(new_id)
            && obj.subtypes.contains(&Subtype::Aura)
            && obj.attached_to.is_none()
            && let Some(filter) = obj.aura_attach_filter.clone()
        {
            let chooser = obj.owner;
            let filter_ctx = self.filter_context_for(chooser, Some(new_id));
            let mut candidates = Vec::new();
            for (id, candidate) in &self.objects {
                if *id == new_id || candidate.zone != Zone::Battlefield {
                    continue;
                }
                if filter.matches(candidate, &filter_ctx, self) {
                    candidates.push(crate::decisions::context::SelectableObject::new(
                        *id,
                        candidate.name.clone(),
                    ));
                }
            }

            if candidates.is_empty() {
                // No legal attachment target - put the Aura into the graveyard
                self.move_object(new_id, Zone::Graveyard);
            } else {
                let ctx = crate::decisions::context::SelectObjectsContext::new(
                    chooser,
                    Some(new_id),
                    "Attach Aura to",
                    candidates,
                    1,
                    Some(1),
                );
                let chosen = decision_maker.decide_objects(self, &ctx);
                if let Some(target_id) = chosen.first().copied() {
                    if let Some(aura) = self.object_mut(new_id) {
                        aura.attached_to = Some(target_id);
                    }
                    if let Some(target) = self.object_mut(target_id)
                        && !target.attachments.contains(&new_id)
                    {
                        target.attachments.push(new_id);
                    }
                    self.continuous_effects.record_attachment(new_id);
                }
            }
        }

        Some(EntersResult {
            new_id,
            enters_tapped: result.enters_tapped,
        })
    }

    /// Removes an object from the game completely (e.g., tokens ceasing to exist).
    /// This does NOT create a new object - the object is simply gone.
    pub fn remove_object(&mut self, id: ObjectId) {
        if let Some(obj) = self.objects.remove(&id) {
            self.remove_from_zone_index(id, obj.zone, obj.owner);
        }
    }

    /// Removes an object ID from its zone index.
    fn remove_from_zone_index(&mut self, id: ObjectId, zone: Zone, owner: PlayerId) {
        match zone {
            Zone::Battlefield => self.battlefield.retain(|&x| x != id),
            Zone::Command => self.command_zone.retain(|&x| x != id),
            Zone::Exile => self.exile.retain(|&x| x != id),
            Zone::Library => {
                if let Some(player) = self.player_mut(owner) {
                    player.library.retain(|&x| x != id);
                }
            }
            Zone::Hand => {
                if let Some(player) = self.player_mut(owner) {
                    player.hand.retain(|&x| x != id);
                }
            }
            Zone::Graveyard => {
                if let Some(player) = self.player_mut(owner) {
                    player.graveyard.retain(|&x| x != id);
                }
            }
            Zone::Stack => {}
        }
    }

    // =========================================================================
    // Zone Consistency Validation (Debug Only)
    // =========================================================================

    /// Validate that zone indexes are consistent with the canonical objects HashMap.
    ///
    /// This checks that:
    /// - Every ID in denormalized zone indexes (battlefield, exile, etc.) exists in objects
    /// - Every object's zone field matches exactly one denormalized index
    /// - No ID appears in multiple zone indexes
    ///
    /// Only runs in debug builds to avoid release performance impact.
    #[cfg(debug_assertions)]
    pub fn validate_zone_consistency(&self) -> Result<(), String> {
        use std::collections::HashSet;

        let mut seen_ids: HashSet<ObjectId> = HashSet::new();

        // Check battlefield
        for &id in &self.battlefield {
            if seen_ids.contains(&id) {
                return Err(format!("Object {:?} appears in multiple zone indexes", id));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Battlefield => {}
                Some(obj) => {
                    return Err(format!(
                        "Object {:?} in battlefield index has zone {:?}",
                        id, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object {:?} in battlefield index doesn't exist in objects",
                        id
                    ));
                }
            }
        }

        // Check exile
        for &id in &self.exile {
            if seen_ids.contains(&id) {
                return Err(format!("Object {:?} appears in multiple zone indexes", id));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Exile => {}
                Some(obj) => {
                    return Err(format!(
                        "Object {:?} in exile index has zone {:?}",
                        id, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object {:?} in exile index doesn't exist in objects",
                        id
                    ));
                }
            }
        }

        // Check command zone
        for &id in &self.command_zone {
            if seen_ids.contains(&id) {
                return Err(format!("Object {:?} appears in multiple zone indexes", id));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Command => {}
                Some(obj) => {
                    return Err(format!(
                        "Object {:?} in command zone index has zone {:?}",
                        id, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object {:?} in command zone index doesn't exist in objects",
                        id
                    ));
                }
            }
        }

        // Check player zones
        for player in &self.players {
            // Library
            for &id in &player.library {
                if seen_ids.contains(&id) {
                    return Err(format!("Object {:?} appears in multiple zone indexes", id));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Library => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object {:?} in {}'s library has zone {:?}",
                            id, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object {:?} in {}'s library doesn't exist in objects",
                            id, player.name
                        ));
                    }
                }
            }

            // Hand
            for &id in &player.hand {
                if seen_ids.contains(&id) {
                    return Err(format!("Object {:?} appears in multiple zone indexes", id));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Hand => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object {:?} in {}'s hand has zone {:?}",
                            id, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object {:?} in {}'s hand doesn't exist in objects",
                            id, player.name
                        ));
                    }
                }
            }

            // Graveyard
            for &id in &player.graveyard {
                if seen_ids.contains(&id) {
                    return Err(format!("Object {:?} appears in multiple zone indexes", id));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Graveyard => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object {:?} in {}'s graveyard has zone {:?}",
                            id, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object {:?} in {}'s graveyard doesn't exist in objects",
                            id, player.name
                        ));
                    }
                }
            }
        }

        // Check that all objects with non-Stack zones are in exactly one index
        for (&id, obj) in &self.objects {
            if obj.zone == Zone::Stack {
                // Stack objects are managed via StackEntry, not indexed
                continue;
            }
            if !seen_ids.contains(&id) {
                return Err(format!(
                    "Object {:?} with zone {:?} is not in any zone index",
                    id, obj.zone
                ));
            }
        }

        Ok(())
    }

    /// Debug assertion for zone consistency. Panics if zones are inconsistent.
    #[cfg(debug_assertions)]
    pub fn debug_assert_zone_consistency(&self) {
        if let Err(e) = self.validate_zone_consistency() {
            panic!("Zone consistency violation: {}", e);
        }
    }

    /// Gets a reference to an object by ID.
    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    /// Gets a mutable reference to an object by ID.
    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects.get_mut(&id)
    }

    // =========================================================================
    // Counter Management
    // =========================================================================

    /// Add counters to an object and return a CounterPlaced event for trigger checking.
    ///
    /// This method adds the counters and returns the event that should be used
    /// to check for triggers (like saga chapter abilities).
    ///
    /// Returns None if the object doesn't exist.
    pub fn add_counters(
        &mut self,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
    ) -> Option<crate::triggers::TriggerEvent> {
        let obj = self.object_mut(id)?;
        obj.add_counters(counter_type, amount);

        Some(crate::triggers::TriggerEvent::new(
            crate::events::other::CounterPlacedEvent::new(id, counter_type, amount),
        ))
    }

    /// Remove counters from an object.
    ///
    /// Returns the actual number of counters removed and a trigger event.
    /// The actual removed amount may be less than requested if there weren't enough.
    pub fn remove_counters(
        &mut self,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<(u32, crate::triggers::TriggerEvent)> {
        let obj = self.object_mut(id)?;
        let removed = obj.remove_counters(counter_type, amount);

        if removed == 0 {
            return None;
        }

        let event =
            crate::triggers::TriggerEvent::new(crate::events::MarkersChangedEvent::removed(
                counter_type,
                id,
                removed,
                source,
                source_controller,
            ));

        Some((removed, event))
    }

    /// Add counters with full tracking (source, controller) for the unified marker system.
    ///
    /// Returns a MarkersChangedEvent for trigger checking.
    pub fn add_counters_with_source(
        &mut self,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<crate::triggers::TriggerEvent> {
        if amount == 0 {
            return None;
        }

        let obj = self.object_mut(id)?;
        obj.add_counters(counter_type, amount);

        Some(crate::triggers::TriggerEvent::new(
            crate::events::MarkersChangedEvent::added(
                counter_type,
                id,
                amount,
                source,
                source_controller,
            ),
        ))
    }

    /// Get the number of counters of a specific type on an object.
    pub fn counter_count(&self, id: ObjectId, counter_type: crate::object::CounterType) -> u32 {
        self.object(id)
            .and_then(|obj| obj.counters.get(&counter_type).copied())
            .unwrap_or(0)
    }

    /// Add counters to a player and emit a unified marker event when applicable.
    ///
    /// Returns `None` for unsupported player counter types.
    pub fn add_player_counters_with_source(
        &mut self,
        player_id: PlayerId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<crate::triggers::TriggerEvent> {
        if amount == 0 {
            return None;
        }

        let player = self.player_mut(player_id)?;
        match counter_type {
            crate::object::CounterType::Poison => {
                player.poison_counters = player.poison_counters.saturating_add(amount);
            }
            crate::object::CounterType::Energy => {
                player.energy_counters = player.energy_counters.saturating_add(amount);
            }
            crate::object::CounterType::Experience => {
                player.experience_counters = player.experience_counters.saturating_add(amount);
            }
            _ => return None,
        }

        Some(crate::triggers::TriggerEvent::new(
            crate::events::MarkersChangedEvent::added(
                counter_type,
                player_id,
                amount,
                source,
                source_controller,
            ),
        ))
    }

    /// Check if an object has any counters of a specific type.
    pub fn has_counters(&self, id: ObjectId, counter_type: crate::object::CounterType) -> bool {
        self.counter_count(id, counter_type) > 0
    }

    // =========================================================================
    // Calculated Characteristics (with continuous effects applied)
    // =========================================================================

    /// Calculate all characteristics for an object, applying continuous effects.
    ///
    /// This includes effects from:
    /// - Registered continuous effects (from resolved spells/abilities)
    /// - Static abilities on permanents (generated dynamically)
    pub fn all_continuous_effects(&self) -> Vec<ContinuousEffect> {
        crate::static_ability_processor::get_all_continuous_effects(self)
    }

    /// Calculate all characteristics for an object using precomputed continuous effects.
    ///
    /// This avoids rebuilding/allocating the full effect list when multiple
    /// characteristic lookups happen in the same operation.
    pub fn calculated_characteristics_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Option<crate::continuous::CalculatedCharacteristics> {
        crate::continuous::calculate_characteristics_with_effects(
            id,
            &self.objects,
            effects,
            &self.battlefield,
            &self.commanders,
            self,
        )
    }

    pub fn calculated_characteristics(
        &self,
        id: ObjectId,
    ) -> Option<crate::continuous::CalculatedCharacteristics> {
        let all_effects = self.all_continuous_effects();
        self.calculated_characteristics_with_effects(id, &all_effects)
    }

    /// Check if an object has a specific static ability using precomputed effects.
    pub fn object_has_ability_with_effects(
        &self,
        id: ObjectId,
        ability: &StaticAbility,
        effects: &[ContinuousEffect],
    ) -> bool {
        self.calculated_characteristics_with_effects(id, effects)
            .map(|c| c.static_abilities.contains(ability))
            .unwrap_or(false)
    }

    /// Check if an object has a specific card type using precomputed effects.
    pub fn object_has_card_type_with_effects(
        &self,
        id: ObjectId,
        card_type: crate::types::CardType,
        effects: &[ContinuousEffect],
    ) -> bool {
        self.calculated_characteristics_with_effects(id, effects)
            .map(|c| c.card_types.contains(&card_type))
            .unwrap_or(false)
    }

    /// Get calculated subtypes using precomputed effects.
    pub fn calculated_subtypes_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Vec<crate::types::Subtype> {
        self.calculated_characteristics_with_effects(id, effects)
            .map(|c| c.subtypes)
            .unwrap_or_default()
    }

    /// Get calculated toughness using precomputed effects.
    pub fn calculated_toughness_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Option<i32> {
        self.calculated_characteristics_with_effects(id, effects)
            .and_then(|c| c.toughness)
    }

    /// Get the calculated power of a creature (with continuous effects applied).
    pub fn calculated_power(&self, id: ObjectId) -> Option<i32> {
        self.calculated_characteristics(id).and_then(|c| c.power)
    }

    /// Get the calculated toughness of a creature (with continuous effects applied).
    pub fn calculated_toughness(&self, id: ObjectId) -> Option<i32> {
        self.calculated_characteristics(id)
            .and_then(|c| c.toughness)
    }

    /// Check if an object has a specific static ability (with continuous effects applied).
    pub fn object_has_ability(&self, id: ObjectId, ability: &StaticAbility) -> bool {
        self.calculated_characteristics(id)
            .map(|c| c.static_abilities.contains(ability))
            .unwrap_or(false)
    }

    /// Check if an object has a static ability with the given ID.
    pub fn object_has_static_ability_id(
        &self,
        id: ObjectId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        if let Some(chars) = self.calculated_characteristics(id) {
            return chars
                .static_abilities
                .iter()
                .any(|ability| ability.id() == ability_id);
        }

        self.object(id)
            .is_some_and(|object| object.has_static_ability_id(ability_id))
    }

    /// Get the calculated subtypes of an object (with continuous effects applied).
    pub fn calculated_subtypes(&self, id: ObjectId) -> Vec<crate::types::Subtype> {
        self.calculated_characteristics(id)
            .map(|c| c.subtypes)
            .unwrap_or_default()
    }

    /// Get the calculated card types of an object (with continuous effects applied).
    pub fn calculated_card_types(&self, id: ObjectId) -> Vec<crate::types::CardType> {
        self.calculated_characteristics(id)
            .map(|c| c.card_types)
            .unwrap_or_default()
    }

    /// Check if an object has a specific card type (with continuous effects applied).
    pub fn object_has_card_type(&self, id: ObjectId, card_type: crate::types::CardType) -> bool {
        self.calculated_card_types(id).contains(&card_type)
    }

    // =========================================================================
    // "Can't" Effect Tracking (Rule 614.17)
    // =========================================================================

    /// Update the CantEffectTracker by scanning static abilities on the battlefield.
    ///
    /// Per Rule 614.17, "can't" effects are not replacement effects - they must
    /// be checked BEFORE attempting an action or event. This function scans all
    /// permanents on the battlefield and populates the tracker based on their
    /// static abilities.
    ///
    /// Call this after:
    /// - State-based actions are checked
    /// - Before processing any event that might be affected by "can't" effects
    /// - After any permanent enters or leaves the battlefield
    pub fn update_cant_effects(&mut self) {
        use crate::ability::AbilityKind;
        use crate::static_abilities::StaticAbility;

        // Clear existing tracker
        self.cant_effects.clear();
        self.mana_spend_effects.clear();
        self.damage_persists.clear();
        for player in &mut self.players {
            player.max_hand_size = 7;
        }

        // First, collect static abilities from objects in zones where they function
        // (currently battlefield and stack).
        // We collect first to avoid borrow conflicts while applying restrictions.
        let abilities_to_apply: Vec<(StaticAbility, ObjectId, PlayerId)> = self
            .objects
            .iter()
            .filter_map(|(&object_id, object)| {
                let zone = object.zone;
                if zone != Zone::Battlefield && zone != Zone::Stack {
                    return None;
                }

                let controller = object.controller;
                Some(
                    object
                        .abilities
                        .iter()
                        .filter_map(|ability| {
                            if let AbilityKind::Static(static_ability) = &ability.kind {
                                if ability.functions_in(&zone) {
                                    Some((static_ability.clone(), object_id, controller))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect();

        // Now apply each ability's restrictions using the trait method
        for (static_ability, permanent_id, controller) in abilities_to_apply {
            static_ability.apply_restrictions(self, permanent_id, controller);
        }

        // Apply active restriction effects from spells/abilities.
        let current_turn = self.turn.turn_number;
        let mut active_restrictions = Vec::new();
        for effect in &self.restriction_effects {
            if effect.is_active(self, current_turn) {
                active_restrictions.push(effect.clone());
            }
        }
        self.restriction_effects = active_restrictions.clone();

        let mut active_goad = Vec::new();
        for effect in &self.goad_effects {
            if effect.is_active(self, current_turn) {
                active_goad.push(effect.clone());
            }
        }
        self.goad_effects = active_goad;

        let mut restriction_tracker = CantEffectTracker::default();
        for effect in active_restrictions {
            effect.restriction.apply(
                self,
                &mut restriction_tracker,
                effect.controller,
                Some(effect.source),
            );
        }
        self.cant_effects.merge(restriction_tracker);

        // "Can't be regenerated" restrictions disable both new and existing shields.
        let cant_be_regenerated: Vec<_> = self
            .cant_effects
            .cant_be_regenerated
            .iter()
            .copied()
            .collect();
        for object_id in cant_be_regenerated {
            self.replacement_effects
                .remove_one_shot_effects_from_source(object_id);
            self.clear_regeneration_shields(object_id);
        }
    }

    pub fn keep_damage_marked(&mut self, object: ObjectId) {
        self.damage_persists.insert(object);
    }

    /// Update continuous effects from static abilities on the battlefield.
    ///
    /// This scans all permanents with static abilities that generate continuous
    /// effects (anthems, abilities that grant abilities, etc.) and updates the
    /// ContinuousEffectManager with these effects.
    ///
    /// Per Rule 611.3a, static ability effects apply dynamically.
    pub fn update_static_ability_effects(&mut self) {
        use crate::static_ability_processor::generate_continuous_effects_from_static_abilities;

        let effects = generate_continuous_effects_from_static_abilities(self);
        self.continuous_effects.set_static_ability_effects(effects);
    }

    /// Update replacement effects from static abilities on the battlefield.
    ///
    /// This scans all permanents with static abilities that generate replacement
    /// effects (enters tapped, enters with counters, etc.) and updates the
    /// ReplacementEffectManager with these effects.
    pub fn update_replacement_effects(&mut self) {
        use crate::replacement_ability_processor::generate_replacement_effects_from_abilities;

        // Clear existing static ability replacement effects
        self.replacement_effects.clear_static_ability_effects();

        // Generate and register new ones from current battlefield state
        let effects = generate_replacement_effects_from_abilities(self);
        for effect in effects {
            self.replacement_effects.add_static_ability_effect(effect);
        }
    }

    /// Perform a full refresh of all dynamic game state that depends on continuous effects.
    ///
    /// This should be called:
    /// - After state-based actions are checked
    /// - Before processing priority or combat decisions
    /// - After permanents enter or leave the battlefield
    ///
    /// It updates:
    /// - Static ability continuous effects (anthems, etc.)
    /// - Replacement effects from static abilities
    /// - "Can't" effect tracking
    pub fn refresh_continuous_state(&mut self) {
        // Update continuous effects from static abilities
        self.update_static_ability_effects();

        // Update replacement effects from static abilities
        self.update_replacement_effects();

        // Update "can't" effect tracking
        self.update_cant_effects();
    }

    /// Check if a player may spend mana as though it were mana of any color.
    ///
    /// If `source` is provided, this also checks for source-specific activation permissions.
    pub fn can_spend_mana_as_any_color(&self, payer: PlayerId, source: Option<ObjectId>) -> bool {
        if self.mana_spend_effects.any_color_players.contains(&payer) {
            return true;
        }

        let Some(source_id) = source else {
            return false;
        };

        if !self
            .mana_spend_effects
            .any_color_activation_sources
            .contains(&source_id)
        {
            return false;
        }

        self.object(source_id)
            .is_some_and(|obj| obj.controller == payer)
    }

    /// Check if a player can pay a mana cost, accounting for "spend as though any color".
    pub fn can_pay_mana_cost(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
    ) -> bool {
        let Some(player) = self.player(payer) else {
            return false;
        };

        let allow_any_color = self.can_spend_mana_as_any_color(payer, source);
        player
            .mana_pool
            .can_pay_with_any_color(cost, x_value, allow_any_color)
    }

    /// Attempt to pay a mana cost, accounting for "spend as though any color".
    pub fn try_pay_mana_cost(
        &mut self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
    ) -> bool {
        let allow_any_color = self.can_spend_mana_as_any_color(payer, source);
        let Some(player) = self.player_mut(payer) else {
            return false;
        };

        player
            .mana_pool
            .try_pay_with_any_color(cost, x_value, allow_any_color)
    }

    /// Gets a reference to a player by ID.
    pub fn player(&self, id: PlayerId) -> Option<&Player> {
        self.players.get(id.index())
    }

    /// Gets a mutable reference to a player by ID.
    pub fn player_mut(&mut self, id: PlayerId) -> Option<&mut Player> {
        self.players.get_mut(id.index())
    }

    /// Designate an object as a commander for a player.
    ///
    /// This sets the commander status on the game state and adds it to the player's commander list.
    pub fn set_as_commander(&mut self, object_id: ObjectId, owner: PlayerId) {
        // Set the commander flag in the extension map
        self.set_commander(object_id);
        // Add to the player's commander list
        if let Some(player) = self.player_mut(owner) {
            player.add_commander(object_id);
        }
    }

    /// Set the current monarch designation holder.
    ///
    /// Use `None` to clear the designation.
    pub fn set_monarch(&mut self, monarch: Option<PlayerId>) {
        self.monarch = monarch;
    }

    /// Returns true if the given player is currently the monarch.
    pub fn is_monarch(&self, player: PlayerId) -> bool {
        self.monarch == Some(player)
    }

    /// Returns true if the given player has the city's blessing designation.
    pub fn has_citys_blessing(&self, player: PlayerId) -> bool {
        self.command_zone.iter().any(|&obj_id| {
            self.object(obj_id).is_some_and(|obj| {
                obj.controller == player && obj.name.eq_ignore_ascii_case("City's Blessing")
            })
        })
    }

    /// Returns all object IDs in a given zone.
    pub fn objects_in_zone(&self, zone: Zone) -> Vec<ObjectId> {
        self.objects
            .values()
            .filter(|o| o.zone == zone)
            .map(|o| o.id)
            .collect()
    }

    /// Returns an iterator over all objects in the game.
    pub fn objects_iter(&self) -> impl Iterator<Item = &Object> {
        self.objects.values()
    }

    /// Returns all permanents controlled by a player.
    pub fn permanents_controlled_by(&self, controller: PlayerId) -> Vec<ObjectId> {
        self.battlefield
            .iter()
            .filter(|&&id| {
                self.objects
                    .get(&id)
                    .is_some_and(|o| o.controller == controller)
            })
            .copied()
            .collect()
    }

    /// Returns all creatures controlled by a player.
    pub fn creatures_controlled_by(&self, controller: PlayerId) -> Vec<ObjectId> {
        self.battlefield
            .iter()
            .filter(|&&id| {
                self.objects
                    .get(&id)
                    .is_some_and(|o| o.controller == controller && o.is_creature())
            })
            .copied()
            .collect()
    }

    /// Returns devotion to a color for permanents controlled by `controller`.
    ///
    /// Devotion counts colored mana symbols in mana costs. Hybrid symbols count
    /// if they include the queried color.
    pub fn devotion_to_color(&self, controller: PlayerId, color: crate::color::Color) -> usize {
        self.permanents_controlled_by(controller)
            .into_iter()
            .filter_map(|id| self.object(id))
            .filter_map(|obj| obj.mana_cost.as_ref())
            .map(|mana_cost| {
                mana_cost
                    .pips()
                    .iter()
                    .map(|pip| {
                        usize::from(pip.iter().copied().any(|symbol| {
                            matches!(
                                (symbol, color),
                                (crate::mana::ManaSymbol::White, crate::color::Color::White)
                                    | (crate::mana::ManaSymbol::Blue, crate::color::Color::Blue)
                                    | (crate::mana::ManaSymbol::Black, crate::color::Color::Black)
                                    | (crate::mana::ManaSymbol::Red, crate::color::Color::Red)
                                    | (crate::mana::ManaSymbol::Green, crate::color::Color::Green)
                            )
                        }))
                    })
                    .sum::<usize>()
            })
            .sum()
    }

    /// Advances to the next turn.
    ///
    /// Turn order rules:
    /// 1. If there are extra turns queued, the first one is taken instead of normal turn order
    /// 2. If the next player should skip their turn, they are skipped (and removed from skip list)
    /// 3. Otherwise, proceed to the next player in turn order
    pub fn next_turn(&mut self) {
        // Check for extra turns first (Time Walk, etc.)
        let next_player = if !self.extra_turns.is_empty() {
            // Take the first extra turn from the queue
            self.extra_turns.remove(0)
        } else {
            // Find next player in turn order
            let current_index = self
                .turn_order
                .iter()
                .position(|&p| p == self.turn.active_player)
                .unwrap_or(0);

            let mut next_index = (current_index + 1) % self.turn_order.len();
            let start_index = next_index;

            // Find next valid player (skip players who left or should skip their turn)
            loop {
                let candidate = self.turn_order[next_index];

                // Check if player is still in game
                let is_in_game = self.player(candidate).is_some_and(|p| p.is_in_game());

                if is_in_game {
                    // Check if this player should skip their turn
                    if self.skip_next_turn.remove(&candidate) {
                        // Player skips this turn, continue to next player
                        next_index = (next_index + 1) % self.turn_order.len();
                        if next_index == start_index {
                            // Wrapped around - all players are skipping (shouldn't happen)
                            break;
                        }
                        continue;
                    }
                    // Found a valid player
                    break;
                }

                // Player has left, skip to next
                next_index = (next_index + 1) % self.turn_order.len();
                if next_index == start_index {
                    // All other players have left
                    break;
                }
            }

            self.turn_order[next_index]
        };

        // Reset turn state
        self.turn.active_player = next_player;
        self.turn.priority_player = Some(next_player);
        self.turn.turn_number += 1;
        self.turn.phase = Phase::Beginning;
        self.turn.step = Some(Step::Untap);

        // Clear turn-based tracking
        self.clear_activated_abilities_tracking();
        self.chosen_modes_by_ability_this_turn.clear();
        self.cards_drawn_this_turn.clear();
        self.creatures_died_this_turn = 0;
        self.creatures_died_under_controller_this_turn.clear();
        self.creatures_left_battlefield_under_controller_this_turn
            .clear();
        self.permanents_left_battlefield_under_controller_this_turn
            .clear();
        self.objects_put_into_graveyard_this_turn.clear();
        self.objects_put_into_graveyard_from_battlefield_this_turn
            .clear();
        self.objects_entered_battlefield_this_turn.clear();
        self.triggers_fired_this_turn.clear();
        self.turn_counters.clear();
        self.spells_cast_last_turn_total = self.spells_cast_this_turn_total;
        self.spells_cast_this_turn.clear();
        self.crimes_committed_this_turn.clear();
        self.artifacts_sacrificed_this_turn.clear();
        self.mana_spent_to_cast_spells_this_turn.clear();
        self.spells_cast_this_turn_total = 0;
        self.spells_cast_this_turn_snapshots.clear();
        self.spell_cast_order_this_turn.clear();
        self.players_attacked_this_turn.clear();
        self.players_tapped_land_for_mana_this_turn.clear();
        self.creatures_attacked_this_turn.clear();
        self.library_searches_this_turn.clear();
        self.creatures_entered_this_turn.clear();
        self.crewed_this_turn.clear();
        self.saddled_until_end_of_turn.clear();
        self.saddled_this_turn.clear();
        self.ninjutsu_attack_targets.clear();
        self.creature_damage_to_players_this_turn.clear();
        self.damage_to_players_this_turn.clear();
        self.noncombat_damage_to_players_this_turn.clear();
        self.life_lost_this_turn.clear();
        self.life_gained_this_turn.clear();
        self.creatures_damaged_by_this_turn.clear();
        self.combat_damage_player_batch_hits.clear();

        // Activate any pending player-control effects for the new active player.
        self.activate_pending_player_control(next_player);

        // Begin turn for the player
        if let Some(player) = self.player_mut(next_player) {
            player.begin_turn();
        }
    }

    /// Add a player-control effect.
    pub fn add_player_control(
        &mut self,
        controller: PlayerId,
        target: PlayerId,
        start: PlayerControlStart,
        duration: PlayerControlDuration,
        source: Option<StableId>,
    ) {
        if matches!(duration, PlayerControlDuration::UntilSourceLeaves)
            && source.is_some_and(|stable| !self.is_source_on_battlefield(stable))
        {
            return;
        }

        self.player_control_timestamp = self.player_control_timestamp.saturating_add(1);
        let mut effect = PlayerControlEffect {
            controller,
            target,
            start,
            duration,
            source,
            timestamp: self.player_control_timestamp,
            active: matches!(start, PlayerControlStart::Immediate),
            expires_on_turn: None,
        };

        if effect.active && matches!(duration, PlayerControlDuration::UntilEndOfTurn) {
            effect.expires_on_turn = Some(self.turn.turn_number);
        }

        self.player_control_effects.push(effect);
    }

    /// Return the controlling player for the given player, if any effect applies.
    pub fn controlling_player_for(&self, player: PlayerId) -> PlayerId {
        let mut best: Option<&PlayerControlEffect> = None;
        for effect in &self.player_control_effects {
            if !effect.active || effect.target != player {
                continue;
            }
            if matches!(effect.duration, PlayerControlDuration::UntilSourceLeaves)
                && effect
                    .source
                    .is_some_and(|stable| !self.is_source_on_battlefield(stable))
            {
                continue;
            }
            if best.is_none_or(|current| effect.timestamp > current.timestamp) {
                best = Some(effect);
            }
        }

        best.map(|effect| effect.controller).unwrap_or(player)
    }

    /// Activate pending player-control effects for the current active player.
    pub fn activate_pending_player_control(&mut self, active_player: PlayerId) {
        let current_turn = self.turn.turn_number;
        for effect in &mut self.player_control_effects {
            if effect.active {
                continue;
            }
            if !matches!(effect.start, PlayerControlStart::NextTurn) {
                continue;
            }
            if effect.target != active_player {
                continue;
            }

            effect.active = true;
            if matches!(effect.duration, PlayerControlDuration::UntilEndOfTurn) {
                effect.expires_on_turn = Some(current_turn);
            }
        }
    }

    /// Cleanup player-control effects that expire at end of turn.
    pub fn cleanup_player_control_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        let battlefield_sources: HashSet<StableId> = self
            .battlefield
            .iter()
            .filter_map(|&id| self.object(id).map(|obj| obj.stable_id))
            .collect();
        self.player_control_effects.retain(|effect| {
            if matches!(effect.duration, PlayerControlDuration::UntilEndOfTurn)
                && effect.expires_on_turn == Some(current_turn)
            {
                return false;
            }
            if matches!(effect.duration, PlayerControlDuration::UntilSourceLeaves)
                && effect
                    .source
                    .is_some_and(|stable| !battlefield_sources.contains(&stable))
            {
                return false;
            }
            true
        });
    }

    fn clear_player_control_from_source(&mut self, stable_id: StableId) {
        self.player_control_effects.retain(|effect| {
            !(matches!(effect.duration, PlayerControlDuration::UntilSourceLeaves)
                && effect.source == Some(stable_id))
        });
    }

    fn is_source_on_battlefield(&self, stable_id: StableId) -> bool {
        self.find_object_by_stable_id(stable_id)
            .and_then(|id| self.object(id))
            .is_some_and(|obj| obj.zone == Zone::Battlefield)
    }

    /// Empties all players' mana pools.
    /// Called at the end of each step and phase per MTG rules.
    pub fn empty_mana_pools(&mut self) {
        for player in &mut self.players {
            player.mana_pool.empty();
        }
    }

    /// Clears the tracking for OncePerTurn activated abilities.
    /// Called at the beginning of each turn.
    pub fn clear_activated_abilities_tracking(&mut self) {
        self.activated_abilities_this_turn.clear();
    }

    /// Record that a creature has attacked this turn.
    pub fn mark_creature_attacked_this_turn(&mut self, creature: ObjectId) {
        self.creatures_attacked_this_turn.insert(creature);
    }

    /// Check whether a creature has attacked this turn.
    pub fn creature_attacked_this_turn(&self, creature: ObjectId) -> bool {
        self.creatures_attacked_this_turn.contains(&creature)
    }

    /// Record that a specific trigger fired this turn.
    pub fn record_trigger_fired(
        &mut self,
        source_object_id: ObjectId,
        trigger_id: TriggerIdentity,
    ) {
        *self
            .triggers_fired_this_turn
            .entry((source_object_id, trigger_id))
            .or_insert(0) += 1;
        self.turn_counters.increment_trigger_identity(trigger_id);
    }

    /// Get how many times this trigger fired this turn.
    pub fn trigger_fire_count_this_turn(
        &self,
        source_object_id: ObjectId,
        trigger_id: TriggerIdentity,
    ) -> u32 {
        self.triggers_fired_this_turn
            .get(&(source_object_id, trigger_id))
            .copied()
            .unwrap_or(0)
    }

    /// Record an event kind occurrence this turn.
    pub fn record_trigger_event_kind(&mut self, event_kind: EventKind) {
        self.turn_counters.increment_event_kind(event_kind);
    }

    /// Get event kind occurrence count this turn.
    pub fn trigger_event_kind_count_this_turn(&self, event_kind: EventKind) -> u32 {
        self.turn_counters
            .get(&TurnCounterKey::EventKind(event_kind))
    }

    /// Clear combat-damage player hits tracked for the current trigger batch.
    pub fn clear_combat_damage_player_batch_hits(&mut self) {
        self.combat_damage_player_batch_hits.clear();
    }

    /// Record a combat-damage player hit for the current trigger batch.
    pub fn record_combat_damage_player_batch_hit(&mut self, source: ObjectId, player: PlayerId) {
        self.combat_damage_player_batch_hits.push((source, player));
    }

    /// Return combat-damage player hits already seen in the current trigger batch.
    pub fn combat_damage_player_batch_hits(&self) -> &[(ObjectId, PlayerId)] {
        &self.combat_damage_player_batch_hits
    }

    /// Increment an arbitrary named turn counter.
    pub fn increment_named_turn_counter(&mut self, name: impl Into<String>) {
        self.turn_counters.increment_named(name);
    }

    /// Get an arbitrary named turn counter value.
    pub fn named_turn_counter(&self, name: &str) -> u32 {
        self.turn_counters
            .get(&TurnCounterKey::Named(name.to_string()))
    }

    /// Records that an activated ability was used.
    /// Used for OncePerTurn timing restrictions.
    pub fn record_ability_activation(&mut self, source: ObjectId, ability_index: usize) {
        self.activated_abilities_this_turn
            .insert((source, ability_index));
        self.turn_counters
            .increment_named(activated_ability_turn_counter_name(source, ability_index));
    }

    /// Check if an activated ability has been used this turn.
    pub fn ability_activated_this_turn(&self, source: ObjectId, ability_index: usize) -> bool {
        self.activated_abilities_this_turn
            .contains(&(source, ability_index))
    }

    /// Get how many times an activated ability has been used this turn.
    pub fn ability_activation_count_this_turn(
        &self,
        source: ObjectId,
        ability_index: usize,
    ) -> u32 {
        self.named_turn_counter(&activated_ability_turn_counter_name(source, ability_index))
    }

    /// Record that a mode index was chosen for an activated modal ability.
    pub fn record_ability_mode_choice(
        &mut self,
        source: ObjectId,
        ability_index: usize,
        mode_index: usize,
        this_turn: bool,
    ) {
        let target_map = if this_turn {
            &mut self.chosen_modes_by_ability_this_turn
        } else {
            &mut self.chosen_modes_by_ability
        };
        target_map
            .entry((source, ability_index))
            .or_default()
            .insert(mode_index);
    }

    /// Check whether a given mode index has already been chosen for an activated ability.
    pub fn ability_mode_was_chosen(
        &self,
        source: ObjectId,
        ability_index: usize,
        mode_index: usize,
        this_turn: bool,
    ) -> bool {
        let target_map = if this_turn {
            &self.chosen_modes_by_ability_this_turn
        } else {
            &self.chosen_modes_by_ability
        };
        target_map
            .get(&(source, ability_index))
            .is_some_and(|modes| modes.contains(&mode_index))
    }

    /// Check whether an activated modal ability still has an unchosen mode available.
    pub fn ability_has_unchosen_mode(
        &self,
        source: ObjectId,
        ability_index: usize,
        total_mode_count: usize,
        this_turn: bool,
    ) -> bool {
        if total_mode_count == 0 {
            return false;
        }
        let target_map = if this_turn {
            &self.chosen_modes_by_ability_this_turn
        } else {
            &self.chosen_modes_by_ability
        };
        let chosen_count = target_map
            .get(&(source, ability_index))
            .map_or(0, HashSet::len);
        chosen_count < total_mode_count
    }

    /// Returns the active player.
    pub fn active_player(&self) -> Option<&Player> {
        self.player(self.turn.active_player)
    }

    /// Returns a mutable reference to the active player.
    pub fn active_player_mut(&mut self) -> Option<&mut Player> {
        self.player_mut(self.turn.active_player)
    }

    /// Pushes a spell or ability onto the stack.
    pub fn push_to_stack(&mut self, entry: StackEntry) {
        self.stack.push(entry);
    }

    /// Pops and returns the top item from the stack.
    pub fn pop_from_stack(&mut self) -> Option<StackEntry> {
        self.stack.pop()
    }

    /// Returns true if the stack is empty.
    pub fn stack_is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Returns the number of players still in the game.
    pub fn players_in_game(&self) -> usize {
        self.players.iter().filter(|p| p.is_in_game()).count()
    }

    // =========================================================================
    // Object Dual-Identity Helpers (id vs stable_id)
    // =========================================================================
    //
    // Objects have two identifiers:
    // - `id`: Changes on each zone change (per MTG rule 400.7)
    // - `stable_id`: Stable identifier that persists across zone changes
    //
    // Commander tracking uses the original ObjectId, which becomes the stable_id
    // after zone changes. These helpers abstract over this complexity.

    /// Check if an object is a commander (by current ID or stable_id).
    ///
    /// This handles the dual-identity nature of objects where zone changes
    /// create new IDs but stable_id persists.
    pub fn is_commander(&self, obj_id: ObjectId) -> bool {
        // Check by current ID in all player commander lists
        for player in &self.players {
            if player.commanders.contains(&obj_id) {
                return true;
            }
        }

        // Check by stable_id in case the commander moved zones
        if let Some(obj) = self.object(obj_id) {
            for player in &self.players {
                if player.commanders.contains(&obj.stable_id.object_id()) {
                    return true;
                }
            }
        }

        false
    }

    /// Find an object by its stable_id (stable identifier).
    ///
    /// Returns the current ObjectId of the object with the given stable_id,
    /// or None if no such object exists.
    pub fn find_object_by_stable_id(&self, stable_id: StableId) -> Option<ObjectId> {
        self.objects
            .values()
            .find(|o| o.stable_id == stable_id)
            .map(|o| o.id)
    }

    /// Check if a player controls any of their own commanders on the battlefield.
    ///
    /// This checks if the player controls a permanent that is designated as
    /// one of their own commanders.
    pub fn player_controls_own_commander(&self, player_id: PlayerId) -> bool {
        let commanders = if let Some(player) = self.player(player_id) {
            player.get_commanders().to_vec()
        } else {
            return false;
        };

        // Check if any of the player's commanders are on the battlefield
        // under their control
        for &commander_id in &commanders {
            // A commander might have a different ObjectId now due to zone changes.
            // We check both the current ID and the stable_id (which persists across zone changes).
            for &bf_id in &self.battlefield {
                if let Some(obj) = self.object(bf_id)
                    && obj.controller == player_id
                {
                    // Check if this is the commander by current ID
                    if bf_id == commander_id {
                        return true;
                    }
                    // Also check stable_id in case the commander moved zones
                    if obj.stable_id == StableId::from(commander_id) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if a player controls ANY commander on the battlefield.
    ///
    /// This checks if the player controls a permanent that is designated as
    /// a commander by ANY player (including opponents' commanders that were stolen).
    /// Used for cards like Akroma's Will which say "if you control a commander".
    pub fn player_controls_a_commander(&self, player_id: PlayerId) -> bool {
        // Collect all commander IDs from all players
        let all_commanders: Vec<ObjectId> = self
            .players
            .iter()
            .flat_map(|p| p.get_commanders().iter().copied())
            .collect();

        // Check if any commander is on the battlefield under this player's control
        for &commander_id in &all_commanders {
            for &bf_id in &self.battlefield {
                if let Some(obj) = self.object(bf_id)
                    && obj.controller == player_id
                {
                    // Check if this is a commander by current ID or stable_id
                    if bf_id == commander_id || obj.stable_id == StableId::from(commander_id) {
                        return true;
                    }
                }
            }
        }

        false
    }

    // =========================================================================
    // FilterContext Factory Methods
    // =========================================================================

    /// Create a FilterContext for a controller and optional source.
    ///
    /// This factory method ensures consistent FilterContext construction across
    /// the codebase. It properly populates:
    /// - `you` - the controller
    /// - `source` - the source object (if any)
    /// - `active_player` - the current active player
    /// - `opponents` - all opponents of the controller
    /// - `your_commanders` - the controller's commander IDs
    ///
    /// Use `filter_context_for_combat()` if you also need combat context.
    pub fn filter_context_for(
        &self,
        controller: PlayerId,
        source: Option<ObjectId>,
    ) -> crate::target::FilterContext {
        let opponents = self
            .players
            .iter()
            .filter(|p| p.id != controller && p.is_in_game())
            .map(|p| p.id)
            .collect();

        let your_commanders = self
            .player(controller)
            .map(|p| p.commanders.clone())
            .unwrap_or_default();

        let mut tagged_objects = std::collections::HashMap::new();
        if let Some(source_id) = source
            && let Some(source_obj) = self.object(source_id)
            && let Some(attached_id) = source_obj.attached_to
            && let Some(attached_obj) = self.object(attached_id)
        {
            let attached_snapshot =
                crate::snapshot::ObjectSnapshot::from_object(attached_obj, self);
            if source_obj.subtypes.contains(&crate::types::Subtype::Aura) {
                tagged_objects.insert(
                    crate::tag::TagKey::from("enchanted"),
                    vec![attached_snapshot.clone()],
                );
            }
            if source_obj
                .subtypes
                .contains(&crate::types::Subtype::Equipment)
            {
                tagged_objects.insert(
                    crate::tag::TagKey::from("equipped"),
                    vec![attached_snapshot],
                );
            }
        }

        crate::target::FilterContext {
            you: Some(controller),
            source,
            caster: None,
            active_player: Some(self.turn.active_player),
            opponents,
            teammates: Vec::new(), // Team formats are not modeled yet.
            defending_player: None,
            attacking_player: None,
            your_commanders,
            iterated_player: None,
            target_players: Vec::new(),
            tagged_objects,
        }
    }

    /// Create a FilterContext with combat context.
    ///
    /// This extends `filter_context_for()` with combat-specific fields:
    /// - `defending_player` - the player being attacked
    /// - `attacking_player` - the player who declared attackers
    pub fn filter_context_for_combat(
        &self,
        controller: PlayerId,
        source: Option<ObjectId>,
        defending_player: Option<PlayerId>,
        attacking_player: Option<PlayerId>,
    ) -> crate::target::FilterContext {
        let mut ctx = self.filter_context_for(controller, source);
        ctx.defending_player = defending_player;
        ctx.attacking_player = attacking_player;
        ctx
    }

    /// Get the combined color identity of a player's commanders.
    ///
    /// This returns the union of color identities of all the player's commanders.
    /// Used for cards like Arcane Signet and Command Tower.
    /// If the player has no commanders, returns COLORLESS (producing colorless mana).
    pub fn get_commander_color_identity(&self, player_id: PlayerId) -> crate::color::ColorSet {
        let commanders = if let Some(player) = self.player(player_id) {
            player.get_commanders().to_vec()
        } else {
            return crate::color::ColorSet::COLORLESS;
        };

        let mut identity = crate::color::ColorSet::COLORLESS;

        for &commander_id in &commanders {
            // Try to find the commander object - it might be on battlefield,
            // in command zone, or elsewhere
            if let Some(obj) = self.object(commander_id) {
                identity = identity.union(obj.color_identity());
            } else {
                // Commander might have moved zones and have a different ID.
                // Search through all objects for one with matching stable_id
                for obj in self.objects.values() {
                    if obj.stable_id == StableId::from(commander_id) {
                        identity = identity.union(obj.color_identity());
                        break;
                    }
                }
            }
        }

        identity
    }

    // =========================================================================
    // Battlefield State Extension Map Helpers
    // =========================================================================

    /// Check if a permanent is tapped.
    pub fn is_tapped(&self, id: ObjectId) -> bool {
        self.tapped_permanents.contains(&id)
    }

    /// Tap a permanent.
    pub fn tap(&mut self, id: ObjectId) {
        self.tapped_permanents.insert(id);
    }

    /// Untap a permanent.
    pub fn untap(&mut self, id: ObjectId) {
        self.tapped_permanents.remove(&id);
    }

    /// Check if a creature has summoning sickness.
    pub fn is_summoning_sick(&self, id: ObjectId) -> bool {
        self.summoning_sick.contains(&id)
    }

    /// Set summoning sickness on a creature.
    pub fn set_summoning_sick(&mut self, id: ObjectId) {
        self.summoning_sick.insert(id);
    }

    /// Remove summoning sickness from a creature (e.g., haste).
    pub fn remove_summoning_sickness(&mut self, id: ObjectId) {
        self.summoning_sick.remove(&id);
    }

    /// Get the damage marked on an object.
    pub fn damage_on(&self, id: ObjectId) -> u32 {
        self.damage_marked.get(&id).copied().unwrap_or(0)
    }

    /// Mark damage on an object.
    pub fn mark_damage(&mut self, id: ObjectId, amount: u32) {
        if amount > 0 {
            *self.damage_marked.entry(id).or_insert(0) += amount;
        }
    }

    /// Record that a creature was dealt damage by a given source this turn.
    pub fn record_creature_damaged_by_this_turn(&mut self, creature: ObjectId, source: ObjectId) {
        self.creatures_damaged_by_this_turn
            .entry(creature)
            .or_default()
            .insert(source);
    }

    /// Returns true if `creature` was dealt damage by `source` this turn.
    pub fn creature_was_damaged_by_source_this_turn(
        &self,
        creature: ObjectId,
        source: ObjectId,
    ) -> bool {
        self.creatures_damaged_by_this_turn
            .get(&creature)
            .is_some_and(|sources| sources.contains(&source))
    }

    /// Returns true if `creature` was dealt damage by any source this turn.
    pub fn creature_was_damaged_this_turn(&self, creature: ObjectId) -> bool {
        self.creatures_damaged_by_this_turn
            .get(&creature)
            .is_some_and(|sources| !sources.is_empty())
    }

    /// Clear damage from an object.
    pub fn clear_damage(&mut self, id: ObjectId) {
        self.damage_marked.remove(&id);
    }

    /// Get the number of regeneration shields on an object.
    pub fn regeneration_shield_count(&self, id: ObjectId) -> u32 {
        self.regeneration_shields.get(&id).copied().unwrap_or(0)
    }

    /// Add regeneration shields to an object.
    pub fn add_regeneration_shield(&mut self, id: ObjectId, count: u32) {
        if count > 0 {
            *self.regeneration_shields.entry(id).or_insert(0) += count;
        }
    }

    /// Use one regeneration shield. Returns true if a shield was used.
    pub fn use_regeneration_shield(&mut self, id: ObjectId) -> bool {
        if let Some(shields) = self.regeneration_shields.get_mut(&id)
            && *shields > 0
        {
            *shields -= 1;
            if *shields == 0 {
                self.regeneration_shields.remove(&id);
            }
            return true;
        }
        false
    }

    /// Clear all regeneration shields from an object.
    pub fn clear_regeneration_shields(&mut self, id: ObjectId) {
        self.regeneration_shields.remove(&id);
    }

    /// Check if a creature is monstrous.
    pub fn is_monstrous(&self, id: ObjectId) -> bool {
        self.monstrous.contains(&id)
    }

    /// Mark a creature as monstrous.
    pub fn set_monstrous(&mut self, id: ObjectId) {
        self.monstrous.insert(id);
    }

    /// Check if a creature is renowned.
    pub fn is_renowned(&self, id: ObjectId) -> bool {
        self.renowned.contains(&id)
    }

    /// Mark a creature as renowned.
    pub fn set_renowned(&mut self, id: ObjectId) {
        self.renowned.insert(id);
    }

    /// Check if a permanent is saddled (until end of turn).
    pub fn is_saddled(&self, id: ObjectId) -> bool {
        self.saddled_until_end_of_turn.contains(&id)
    }

    /// Mark a permanent as saddled until end of turn.
    pub fn set_saddled_until_end_of_turn(&mut self, id: ObjectId) {
        self.saddled_until_end_of_turn.insert(id);
    }

    /// Check if a permanent is flipped.
    pub fn is_flipped(&self, id: ObjectId) -> bool {
        self.flipped.contains(&id)
    }

    /// Flip a permanent.
    pub fn flip(&mut self, id: ObjectId) {
        self.flipped.insert(id);
    }

    /// Check if a permanent is face-down.
    pub fn is_face_down(&self, id: ObjectId) -> bool {
        self.face_down.contains(&id)
    }

    /// Set a permanent as face-down.
    pub fn set_face_down(&mut self, id: ObjectId) {
        self.face_down.insert(id);
    }

    /// Turn a permanent face-up.
    pub fn set_face_up(&mut self, id: ObjectId) {
        self.face_down.remove(&id);
    }

    /// Check if a permanent is phased out.
    pub fn is_phased_out(&self, id: ObjectId) -> bool {
        self.phased_out.contains(&id)
    }

    /// Phase out a permanent.
    pub fn phase_out(&mut self, id: ObjectId) {
        self.phased_out.insert(id);
    }

    /// Phase in a permanent.
    pub fn phase_in(&mut self, id: ObjectId) {
        self.phased_out.remove(&id);
    }

    /// Check if a card is exiled via madness.
    pub fn is_madness_exiled(&self, id: ObjectId) -> bool {
        self.madness_exiled.contains(&id)
    }

    /// Mark a card as exiled via madness.
    pub fn set_madness_exiled(&mut self, id: ObjectId) {
        self.madness_exiled.insert(id);
    }

    /// Clear madness exiled status.
    pub fn clear_madness_exiled(&mut self, id: ObjectId) {
        self.madness_exiled.remove(&id);
    }

    /// Check if a saga's final chapter has resolved.
    pub fn is_saga_final_chapter_resolved(&self, id: ObjectId) -> bool {
        self.saga_final_chapter_resolved.contains(&id)
    }

    /// Mark a saga's final chapter as resolved.
    pub fn set_saga_final_chapter_resolved(&mut self, id: ObjectId) {
        self.saga_final_chapter_resolved.insert(id);
    }

    /// Alias for set_saga_final_chapter_resolved.
    pub fn mark_saga_final_chapter_resolved(&mut self, id: ObjectId) {
        self.saga_final_chapter_resolved.insert(id);
    }

    /// Clear a saga's final chapter resolved flag.
    pub fn clear_saga_final_chapter_resolved(&mut self, id: ObjectId) {
        self.saga_final_chapter_resolved.remove(&id);
    }

    /// Check if an object is designated as a commander.
    pub fn is_commander_object(&self, id: ObjectId) -> bool {
        self.commanders.contains(&id)
    }

    /// Designate an object as a commander.
    pub fn set_commander(&mut self, id: ObjectId) {
        self.commanders.insert(id);
    }

    /// Clear battlefield state for an object (when leaving battlefield).
    pub fn clear_battlefield_state(&mut self, id: ObjectId) {
        self.clear_soulbond_pair(id);
        self.tapped_permanents.remove(&id);
        self.summoning_sick.remove(&id);
        self.damage_marked.remove(&id);
        self.regeneration_shields.remove(&id);
        self.monstrous.remove(&id);
        self.renowned.remove(&id);
        self.flipped.remove(&id);
        self.face_down.remove(&id);
        self.phased_out.remove(&id);
        self.imprinted_cards.remove(&id);
        self.chosen_colors.remove(&id);
        self.chosen_basic_land_types.remove(&id);
        self.chosen_modes_by_ability
            .retain(|(source, _), _| *source != id);
        self.chosen_modes_by_ability_this_turn
            .retain(|(source, _), _| *source != id);
        // Note: saga_final_chapter_resolved and commanders persist across zone changes
    }

    fn soulbond_pair_is_valid(&self, left: ObjectId, right: ObjectId) -> bool {
        if left == right {
            return false;
        }
        let Some(left_obj) = self.object(left) else {
            return false;
        };
        let Some(right_obj) = self.object(right) else {
            return false;
        };
        if left_obj.zone != Zone::Battlefield || right_obj.zone != Zone::Battlefield {
            return false;
        }
        if !left_obj.is_creature() || !right_obj.is_creature() {
            return false;
        }
        left_obj.controller == right_obj.controller
    }

    pub fn clear_soulbond_pair(&mut self, object_id: ObjectId) {
        let partner = self.soulbond_pairs.remove(&object_id);
        if let Some(partner_id) = partner {
            self.soulbond_pairs.remove(&partner_id);
        }
    }

    pub fn set_soulbond_pair(&mut self, left: ObjectId, right: ObjectId) {
        if !self.soulbond_pair_is_valid(left, right) {
            return;
        }
        self.clear_soulbond_pair(left);
        self.clear_soulbond_pair(right);
        self.soulbond_pairs.insert(left, right);
        self.soulbond_pairs.insert(right, left);
    }

    pub fn soulbond_partner(&self, object_id: ObjectId) -> Option<ObjectId> {
        let partner = self.soulbond_pairs.get(&object_id).copied()?;
        if self
            .soulbond_pairs
            .get(&partner)
            .is_none_or(|paired_back| *paired_back != object_id)
        {
            return None;
        }
        self.soulbond_pair_is_valid(object_id, partner)
            .then_some(partner)
    }

    pub fn is_soulbond_paired(&self, object_id: ObjectId) -> bool {
        self.soulbond_partner(object_id).is_some()
    }

    /// Clear exile state for an object (when leaving exile).
    pub fn clear_exile_state(&mut self, id: ObjectId) {
        self.madness_exiled.remove(&id);
        self.remove_exiled_with_source_link(id);
    }

    // === Chosen color helpers ===

    /// Record a chosen color for a permanent.
    pub fn set_chosen_color(&mut self, permanent_id: ObjectId, color: crate::color::Color) {
        self.chosen_colors.insert(permanent_id, color);
    }

    /// Get a chosen color for a permanent, if any.
    pub fn chosen_color(&self, permanent_id: ObjectId) -> Option<crate::color::Color> {
        self.chosen_colors.get(&permanent_id).copied()
    }

    // === Chosen basic land type helpers ===

    /// Record a chosen basic land type for a permanent.
    pub fn set_chosen_basic_land_type(
        &mut self,
        permanent_id: ObjectId,
        subtype: crate::types::Subtype,
    ) {
        self.chosen_basic_land_types.insert(permanent_id, subtype);
    }

    /// Get a chosen basic land type for a permanent, if any.
    pub fn chosen_basic_land_type(&self, permanent_id: ObjectId) -> Option<crate::types::Subtype> {
        self.chosen_basic_land_types.get(&permanent_id).copied()
    }

    // === Imprint helpers ===

    /// Imprint a card onto a permanent (used by Chrome Mox, Isochron Scepter, etc.).
    pub fn imprint_card(&mut self, permanent_id: ObjectId, exiled_card_id: ObjectId) {
        self.imprinted_cards
            .entry(permanent_id)
            .or_default()
            .push(exiled_card_id);
    }

    /// Get the cards imprinted on a permanent.
    pub fn get_imprinted_cards(&self, permanent_id: ObjectId) -> &[ObjectId] {
        self.imprinted_cards
            .get(&permanent_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a permanent has any imprinted cards.
    pub fn has_imprinted_cards(&self, permanent_id: ObjectId) -> bool {
        self.imprinted_cards
            .get(&permanent_id)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Clear imprinted cards when a permanent leaves the battlefield.
    pub fn clear_imprinted_cards(&mut self, permanent_id: ObjectId) {
        self.imprinted_cards.remove(&permanent_id);
    }

    /// Record that `exiled_card_id` was exiled by `source_id`.
    pub fn add_exiled_with_source_link(&mut self, source_id: ObjectId, exiled_card_id: ObjectId) {
        let entry = self.exiled_with_source.entry(source_id).or_default();
        if !entry.contains(&exiled_card_id) {
            entry.push(exiled_card_id);
        }
    }

    /// Get cards exiled by a specific source object ID.
    pub fn get_exiled_with_source_links(&self, source_id: ObjectId) -> &[ObjectId] {
        self.exiled_with_source
            .get(&source_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Remove an exiled card from all source-link lists.
    pub fn remove_exiled_with_source_link(&mut self, exiled_card_id: ObjectId) {
        self.exiled_with_source.retain(|_, linked| {
            linked.retain(|id| *id != exiled_card_id);
            !linked.is_empty()
        });
    }

    /// Create a linked exile group and return its generated group ID.
    pub fn create_linked_exile_group(
        &mut self,
        mut stable_ids: Vec<StableId>,
        return_zone: Zone,
        return_under_owner_control: bool,
    ) -> u64 {
        // Keep stable order while de-duplicating.
        stable_ids.dedup();

        self.next_linked_exile_group_id = self.next_linked_exile_group_id.saturating_add(1);
        let group_id = self.next_linked_exile_group_id;
        self.linked_exile_groups.insert(
            group_id,
            LinkedExileGroup {
                stable_ids,
                return_zone,
                return_under_owner_control,
            },
        );
        group_id
    }

    /// Take (and clear) a linked exile group.
    pub fn take_linked_exile_group(&mut self, group_id: u64) -> Option<LinkedExileGroup> {
        self.linked_exile_groups.remove(&group_id)
    }

    /// Queue a trigger event to be processed by the game loop.
    /// Use this when effects need to emit events that should generate triggers.
    pub fn queue_trigger_event(&mut self, event: crate::triggers::TriggerEvent) {
        self.pending_trigger_events.push(event);
    }

    /// Take all pending trigger events (empties the queue).
    pub fn take_pending_trigger_events(&mut self) -> Vec<crate::triggers::TriggerEvent> {
        std::mem::take(&mut self.pending_trigger_events)
    }
}
