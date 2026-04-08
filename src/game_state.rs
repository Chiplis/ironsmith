use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use rand::seq::SliceRandom;
use rand::{SeedableRng, rngs::StdRng};

use crate::ability::{Ability, AbilityKind, ActivatedAbility};
use crate::alternative_cast::CastingMethod;
use crate::card::Card;
use crate::continuous::{CalculatedCharacteristics, ContinuousEffect, ContinuousEffectManager};
use crate::cost::OptionalCostsPaid;
use crate::decision::KeywordPaymentContribution;
use crate::dungeon::ActiveDungeonProgress;
use crate::events::{Event, EventKind, KeywordActionKind};
use crate::ids::{ObjectId, PlayerId, StableId, reset_runtime_id_counters};
use crate::object::{AttachmentTarget, AuraAttachmentFilter, Object};
use crate::player::Player;
use crate::prevention::PreventionEffectManager;
use crate::provenance::{ProvNodeId, ProvenanceGraph, ProvenanceNodeKind};
use crate::replacement::{ReplacementEffectId, ReplacementEffectManager};
use crate::static_abilities::StaticAbility;
use crate::target::ChooseSpec;
use crate::triggers::TriggerIdentity;
use crate::turn_history::TurnHistory;
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

/// Stored front-face identity for a melded permanent's component card.
#[derive(Debug, Clone)]
pub struct MeldComponentState {
    pub stable_id: StableId,
    pub owner: PlayerId,
    pub name: String,
}

/// Battlefield metadata for a melded permanent.
#[derive(Debug, Clone)]
pub struct MeldedPermanentState {
    pub components: Vec<MeldComponentState>,
}

/// One-shot battlefield transition hints for the UI animation layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiBattlefieldTransitionKind {
    Damaged,
    Destroyed,
    Sacrificed,
    Exiled,
}

/// A UI-only battlefield transition record keyed by stable object identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiBattlefieldTransition {
    pub stable_id: StableId,
    pub kind: UiBattlefieldTransitionKind,
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

    /// Players who can't become the monarch.
    pub cant_become_monarch: HashSet<PlayerId>,

    /// Permanents that can't be targeted.
    /// Example: Hexproof/Shroud (tracked separately), but also effects like
    /// "can't be the target of spells or abilities"
    pub cant_be_targeted: HashSet<ObjectId>,

    /// Players that can't be targeted.
    pub cant_target_players: HashSet<PlayerId>,

    /// Players that can't be targeted by sources matching a filter.
    pub cant_target_players_from: Vec<PlayerCantBeTargetedFrom>,

    /// Permanents that can't be countered while on the stack.
    /// Example: Vexing Shusher, Prowling Serpopard
    pub cant_be_countered: HashSet<ObjectId>,

    /// Permanents that can't transform.
    /// Example: "Non-Human Werewolves you control can't transform."
    pub cant_transform: HashSet<ObjectId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCantBeTargetedFrom {
    pub player: PlayerId,
    pub source_filter: crate::target::ObjectFilter,
    pub controller: PlayerId,
}

#[derive(Debug, Clone)]
pub struct RestrictionEffectInstance {
    pub restriction: crate::effect::Restriction,
    pub controller: PlayerId,
    pub source: ObjectId,
    pub duration: crate::effect::Until,
    pub expires_end_of_turn: u32,
    pub consumed_next_untap: bool,
}

impl RestrictionEffectInstance {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        if matches!(
            self.duration,
            crate::effect::Until::ControllersNextUntapStep
        ) && self.consumed_next_untap
        {
            return true;
        }
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

#[derive(Debug, Clone)]
pub struct TemporarySpellCostReductionEffectInstance {
    pub player: PlayerId,
    pub source: ObjectId,
    pub filter: crate::target::ObjectFilter,
    pub reduction: crate::mana::ManaCost,
    pub remaining_uses: u32,
    pub expires_end_of_turn: u32,
}

impl TemporarySpellCostReductionEffectInstance {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        self.remaining_uses == 0 || current_turn > self.expires_end_of_turn
    }
}

#[derive(Debug, Clone)]
pub struct TemporarySpellAbilityGrantEffectInstance {
    pub player: PlayerId,
    pub source: ObjectId,
    pub filter: crate::target::ObjectFilter,
    pub ability: crate::static_abilities::StaticAbility,
    pub remaining_uses: u32,
    pub expires_end_of_turn: u32,
}

impl TemporarySpellAbilityGrantEffectInstance {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        self.remaining_uses == 0 || current_turn > self.expires_end_of_turn
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
        self.cant_become_monarch.extend(other.cant_become_monarch);
        self.cant_be_targeted.extend(other.cant_be_targeted);
        self.cant_target_players.extend(other.cant_target_players);
        self.cant_target_players_from
            .extend(other.cant_target_players_from.clone());
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
        self.cant_become_monarch.clear();
        self.cant_be_targeted.clear();
        self.cant_target_players.clear();
        self.cant_target_players_from.clear();
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

    /// Check if a permanent can untap during the specified player's untap step.
    pub fn can_untap_during_step(
        &self,
        permanent: ObjectId,
        permanent_controller: PlayerId,
        untap_player: PlayerId,
    ) -> bool {
        permanent_controller != untap_player || !self.cant_untap.contains(&permanent)
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

    /// Check if a player can become the monarch.
    pub fn can_become_monarch(&self, player: PlayerId) -> bool {
        !self.cant_become_monarch.contains(&player)
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

    /// Check if a player can be targeted by a specific source.
    pub fn can_target_player_from_source(
        &self,
        game: &GameState,
        player: PlayerId,
        source_id: ObjectId,
    ) -> bool {
        if !self.can_target_player(player) {
            return false;
        }

        let Some(source) = game.object(source_id) else {
            return true;
        };

        !self.cant_target_players_from.iter().any(|restriction| {
            if restriction.player != player {
                return false;
            }
            let filter_ctx = game.filter_context_for(restriction.controller, Some(source_id));
            restriction.source_filter.matches(source, &filter_ctx, game)
        })
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
    /// Active permissions that let a player spend mana as though it were mana
    /// of any color.
    pub permissions: Vec<ActiveManaSpendPermission>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveManaSpendPermission {
    pub permission: crate::effect::ManaSpendPermission,
    pub controller: PlayerId,
    pub source: ManaSpendPermissionSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ManaSpendPermissionSource {
    StaticAbility,
    Effect {
        source_id: ObjectId,
        expires_end_of_turn: u32,
    },
}

impl ManaSpendEffectTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.permissions.clear();
    }

    pub fn retain_effect_permissions(&mut self, current_turn: u32) {
        self.permissions.retain(|permission| {
            matches!(
                permission.source,
                ManaSpendPermissionSource::Effect {
                    expires_end_of_turn,
                    ..
                } if current_turn <= expires_end_of_turn
            )
        });
    }

    pub fn cleanup_expired(&mut self, current_turn: u32) {
        self.permissions
            .retain(|permission| match permission.source {
                ManaSpendPermissionSource::StaticAbility => true,
                ManaSpendPermissionSource::Effect {
                    expires_end_of_turn,
                    ..
                } => current_turn <= expires_end_of_turn,
            });
    }
}

impl ActiveManaSpendPermission {
    pub fn allows(&self, game: &GameState, payer: PlayerId, source: Option<ObjectId>) -> bool {
        if matches!(
            self.source,
            ManaSpendPermissionSource::Effect {
                expires_end_of_turn,
                ..
            } if game.turn.turn_number > expires_end_of_turn
        ) {
            return false;
        }

        let combat = game.combat.as_ref();
        if !crate::game_loop::player_matches_filter_with_combat(
            payer,
            &self.permission.player,
            game,
            self.controller,
            combat,
        ) {
            return false;
        }

        match &self.permission.scope {
            crate::effect::ManaSpendScope::AllCosts => true,
            crate::effect::ManaSpendScope::ActivationCostsOf(filter) => {
                let Some(source_id) = source else {
                    return false;
                };
                let Some(source_obj) = game.object(source_id) else {
                    return false;
                };
                let filter_ctx = game.filter_context_for(self.controller, Some(source_id));
                filter.matches(source_obj, &filter_ctx, game)
            }
            crate::effect::ManaSpendScope::CastingSpellsWithStableIds(stable_ids) => {
                let Some(source_id) = source else {
                    return false;
                };
                let Some(source_obj) = game.object(source_id) else {
                    return false;
                };
                stable_ids.contains(&source_obj.stable_id)
            }
        }
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

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::Beginning => "beginning phase",
            Phase::FirstMain => "first main phase",
            Phase::Combat => "combat phase",
            Phase::NextMain => "second main phase",
            Phase::Ending => "ending phase",
        }
    }
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
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

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Untap => "untap step",
            Step::Upkeep => "upkeep step",
            Step::Draw => "draw step",
            Step::BeginCombat => "begin combat step",
            Step::DeclareAttackers => "declare attackers step",
            Step::DeclareBlockers => "declare blockers step",
            Step::CombatDamage => "combat damage step",
            Step::EndCombat => "end combat step",
            Step::End => "end step",
            Step::Cleanup => "cleanup step",
        }
    }
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
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

/// A chosen target requirement bound to a range within the flattened target list.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetAssignment {
    pub spec: ChooseSpec,
    pub range: Range<usize>,
}

/// An entry on the stack.
#[derive(Debug, Clone)]
pub struct StackEntry {
    pub object_id: ObjectId,
    pub controller: PlayerId,
    pub provenance: ProvNodeId,
    pub targets: Vec<Target>,
    pub target_assignments: Vec<TargetAssignment>,
    pub x_value: Option<u32>,
    /// For triggered/activated abilities, the effects to execute.
    /// For spells, this is None and effects come from the spell itself.
    pub ability_effects: Option<crate::resolution::ResolutionProgram>,
    /// Whether this is an ability (triggered or activated) vs a spell.
    pub is_ability: bool,
    /// The casting method used (normal or alternative like flashback).
    pub casting_method: CastingMethod,
    /// Which optional costs were paid (kicker, buyback, etc.).
    pub optional_costs_paid: OptionalCostsPaid,
    /// The defending player for combat-related triggers.
    pub defending_player: Option<PlayerId>,
    /// The chosen player linked to this source at the time the stack entry was created.
    pub chosen_player: Option<PlayerId>,
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
            provenance: ProvNodeId::default(),
            targets: Vec::new(),
            target_assignments: Vec::new(),
            x_value: None,
            ability_effects: None,
            is_ability: false,
            casting_method: CastingMethod::Normal,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            chosen_player: None,
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
        effects: impl Into<crate::resolution::ResolutionProgram>,
    ) -> Self {
        Self {
            object_id: source_id,
            controller,
            provenance: ProvNodeId::default(),
            targets: Vec::new(),
            target_assignments: Vec::new(),
            x_value: None,
            ability_effects: Some(effects.into()),
            is_ability: true,
            casting_method: CastingMethod::Normal,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            chosen_player: None,
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

    pub fn with_target_assignments(mut self, target_assignments: Vec<TargetAssignment>) -> Self {
        self.target_assignments = target_assignments;
        self
    }

    pub fn with_provenance(mut self, provenance: ProvNodeId) -> Self {
        self.provenance = provenance;
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

    pub fn with_chosen_player(mut self, player: Option<PlayerId>) -> Self {
        self.chosen_player = player;
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
    // Fast index: stable id -> current object id.
    stable_id_index: HashMap<StableId, ObjectId>,
    /// Game-local cache for linked-face definitions so transform/split/disturb
    /// resolution doesn't depend on the shared runtime custom-card registry.
    linked_face_definitions_by_id: HashMap<crate::ids::CardId, crate::cards::CardDefinition>,
    linked_face_definitions_by_name: HashMap<String, crate::cards::CardDefinition>,

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
    /// State-triggered abilities whose conditions are currently true.
    pub active_state_trigger_conditions: HashSet<crate::triggers::ActiveStateTriggerKey>,
    /// One-shot battlefield transition hints consumed by the UI snapshot layer.
    pub ui_battlefield_transitions: Vec<UiBattlefieldTransition>,
    /// Event provenance graph for this game.
    pub provenance_graph: ProvenanceGraph,

    /// Current combat state (Some during combat phase, None otherwise).
    /// Effects can directly add creatures to combat when this is set.
    pub combat: Option<crate::combat_state::CombatState>,
    /// Whether the game is currently in night mode (day/night designation).
    pub is_night: bool,
    /// Current monarch designation holder, if any.
    pub monarch: Option<PlayerId>,
    /// Current initiative designation holder, if any.
    pub initiative: Option<PlayerId>,
    /// Current dungeon progress for each player, if any.
    pub active_dungeons: HashMap<PlayerId, ActiveDungeonProgress>,
    /// Named dungeons each player has completed this game.
    pub completed_dungeons: HashMap<PlayerId, Vec<String>>,

    /// Tracks modal choices that were already selected for an activated ability.
    /// Key is (source ObjectId, ability index), value is the set of chosen mode indices.
    pub chosen_modes_by_ability: HashMap<(ObjectId, usize), HashSet<usize>>,

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

    /// Unified owner for per-turn event and action history.
    pub turn_history: TurnHistory,

    /// Total number of spells cast during the immediately previous turn.
    /// Updated when turn advances.
    pub spells_cast_last_turn_total: u32,

    /// Mounts that are saddled until end of turn.
    ///
    /// Cleared at the start of each turn.
    pub saddled_until_end_of_turn: HashSet<ObjectId>,

    /// Soulbond pairings (stored bidirectionally: A -> B and B -> A).
    pub soulbond_pairs: HashMap<ObjectId, ObjectId>,

    /// Attack targets captured while paying Ninjutsu costs, keyed by the
    /// source card object ID in hand.
    ///
    /// Multiple entries per source are stored in activation order so nested
    /// activations can resolve LIFO.
    pub ninjutsu_attack_targets: HashMap<ObjectId, Vec<crate::combat_state::AttackTarget>>,

    /// Combat-damage-to-player hits already processed in the current trigger batch.
    /// Used for "one or more ... deal combat damage to a player" trigger matching.
    pub combat_damage_player_batch_hits: Vec<(ObjectId, PlayerId)>,

    /// Temporary mana abilities granted to players (e.g., Channel), expiring at end of turn.
    pub granted_mana_abilities: Vec<GrantedManaAbility>,

    /// Temporary spell-cost reductions waiting for the next matching spell this turn.
    pub temporary_spell_cost_reductions: Vec<TemporarySpellCostReductionEffectInstance>,

    /// Temporary spell-ability grants waiting for the next matching spell this turn.
    pub temporary_spell_ability_grants: Vec<TemporarySpellAbilityGrantEffectInstance>,

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

    /// Creatures that have been dealt nonzero damage by a source with deathtouch
    /// since the last time state-based actions were checked.
    pub dealt_deathtouch_damage_since_sba: HashSet<ObjectId>,

    /// Permanents whose damage is not removed during cleanup.
    pub damage_persists: HashSet<ObjectId>,

    /// Chosen colors for permanents ("as this enters, choose a color").
    pub chosen_colors: HashMap<ObjectId, crate::color::Color>,

    /// Chosen basic land types for permanents ("as this Aura enters, choose a basic land type").
    pub chosen_basic_land_types: HashMap<ObjectId, crate::types::Subtype>,

    /// Chosen land types for permanents ("as this enters, choose a land type").
    pub chosen_land_types: HashMap<ObjectId, crate::types::Subtype>,

    /// Chosen creature types for permanents ("as this enters, choose a creature type").
    pub chosen_creature_types: HashMap<ObjectId, crate::types::Subtype>,

    /// Chosen players for permanents ("as this enters, choose a player").
    pub chosen_players: HashMap<ObjectId, PlayerId>,

    /// Chosen named options for permanents ("as this enters, choose A or B").
    pub chosen_named_options: HashMap<ObjectId, String>,

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

    /// Face-down permanents created via manifest.
    pub manifested: HashSet<ObjectId>,

    /// Number of times each battlefield permanent has transformed.
    ///
    /// Used to enforce CR 701.27f for abilities that try to transform their source.
    pub transform_count: HashMap<ObjectId, u64>,

    /// Which players may inspect a face-down card in exile.
    pub face_down_exile_viewers: HashMap<ObjectId, HashSet<PlayerId>>,

    /// Phased-out permanents.
    pub phased_out: HashSet<ObjectId>,

    /// Cards exiled via Madness (can be cast from exile for madness cost).
    pub madness_exiled: HashSet<ObjectId>,

    /// Cards exiled via Foretell (can be cast from exile for their foretell cost).
    pub foretold_cards: HashSet<ObjectId>,

    /// Cards exiled via Plot, keyed by object id -> (player who plotted it, turn plotted).
    pub plotted_cards: HashMap<ObjectId, (PlayerId, u32)>,

    /// Sagas whose final chapter ability has resolved (ready to be sacrificed).
    pub saga_final_chapter_resolved: HashSet<ObjectId>,

    /// Objects designated as commanders.
    pub commanders: HashSet<ObjectId>,

    /// Number of times each commander has been cast from the command zone.
    pub commander_casts_from_command_zone: HashMap<ObjectId, u32>,

    /// Commanders whose owner declined the current graveyard/exile -> command
    /// zone choice for this specific object instance.
    pub declined_commander_command_zone_moves: HashSet<ObjectId>,

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

    /// Component-card identity for battlefield melded permanents, keyed by the
    /// melded permanent's stable ID.
    pub melded_permanents: HashMap<StableId, MeldedPermanentState>,

    /// The full set of destination object IDs created by the most recent move
    /// of a given source object.
    pub zone_change_result_objects: HashMap<ObjectId, Vec<ObjectId>>,

    /// Deterministic match RNG state used for shuffles and other random gameplay effects.
    random_state: Cell<u64>,

    /// Monotonic counter incremented whenever gameplay consumes irreversible randomness.
    irreversible_random_count: Cell<u64>,

    /// Whether the cached static-ability continuous effects snapshot may be stale.
    continuous_state_dirty: Cell<bool>,

    /// Final calculated characteristics for the current clean continuous-effects state.
    calculated_characteristics_cache: RefCell<HashMap<ObjectId, Option<CalculatedCharacteristics>>>,

    /// Continuous-effect revision the current characteristic cache was built against.
    calculated_characteristics_cache_revision: Cell<u64>,
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
            stable_id_index: HashMap::new(),
            linked_face_definitions_by_id: HashMap::new(),
            linked_face_definitions_by_name: HashMap::new(),
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
            active_state_trigger_conditions: HashSet::new(),
            ui_battlefield_transitions: Vec::new(),
            provenance_graph: ProvenanceGraph::new(),
            combat: None,
            is_night: false,
            monarch: None,
            initiative: None,
            active_dungeons: HashMap::new(),
            completed_dungeons: HashMap::new(),
            chosen_modes_by_ability: HashMap::new(),
            turn_history: TurnHistory::default(),
            pending_replacement_choice: None,
            grant_registry: crate::grant_registry::GrantRegistry::new(),
            extra_turns: Vec::new(),
            skip_next_turn: HashSet::new(),
            skip_next_draw_step: HashSet::new(),
            skip_next_combat_phases: HashSet::new(),
            player_control_effects: Vec::new(),
            player_control_timestamp: 0,
            spells_cast_last_turn_total: 0,
            saddled_until_end_of_turn: HashSet::new(),
            soulbond_pairs: HashMap::new(),
            ninjutsu_attack_targets: HashMap::new(),
            combat_damage_player_batch_hits: Vec::new(),
            granted_mana_abilities: Vec::new(),
            temporary_spell_cost_reductions: Vec::new(),
            temporary_spell_ability_grants: Vec::new(),
            restriction_effects: Vec::new(),
            goad_effects: Vec::new(),
            // Battlefield state extension maps
            tapped_permanents: HashSet::new(),
            summoning_sick: HashSet::new(),
            damage_marked: HashMap::new(),
            dealt_deathtouch_damage_since_sba: HashSet::new(),
            damage_persists: HashSet::new(),
            chosen_colors: HashMap::new(),
            chosen_basic_land_types: HashMap::new(),
            chosen_land_types: HashMap::new(),
            chosen_creature_types: HashMap::new(),
            chosen_players: HashMap::new(),
            chosen_named_options: HashMap::new(),
            regeneration_shields: HashMap::new(),
            monstrous: HashSet::new(),
            renowned: HashSet::new(),
            flipped: HashSet::new(),
            face_down: HashSet::new(),
            manifested: HashSet::new(),
            transform_count: HashMap::new(),
            face_down_exile_viewers: HashMap::new(),
            phased_out: HashSet::new(),
            madness_exiled: HashSet::new(),
            foretold_cards: HashSet::new(),
            plotted_cards: HashMap::new(),
            saga_final_chapter_resolved: HashSet::new(),
            commanders: HashSet::new(),
            commander_casts_from_command_zone: HashMap::new(),
            declined_commander_command_zone_moves: HashSet::new(),
            imprinted_cards: HashMap::new(),
            exiled_with_source: HashMap::new(),
            linked_exile_groups: HashMap::new(),
            next_linked_exile_group_id: 0,
            melded_permanents: HashMap::new(),
            zone_change_result_objects: HashMap::new(),
            random_state: Cell::new(Self::normalize_random_seed(0)),
            irreversible_random_count: Cell::new(0),
            continuous_state_dirty: Cell::new(true),
            calculated_characteristics_cache: RefCell::new(HashMap::new()),
            calculated_characteristics_cache_revision: Cell::new(0),
        }
    }

    /// Creates a new game state after explicitly resetting runtime player/object IDs.
    ///
    /// Frontend/bootstrap paths should prefer this constructor when they need a
    /// fresh match identity space. Plain `new()` intentionally does not reset
    /// global counters so existing engine tests and embedded callers keep their
    /// current behavior.
    pub fn new_with_runtime_id_reset(player_names: Vec<String>, starting_life: i32) -> Self {
        reset_runtime_id_counters();
        Self::new(player_names, starting_life)
    }

    fn normalize_random_seed(seed: u64) -> u64 {
        if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        }
    }

    fn mark_continuous_state_dirty(&self) {
        self.continuous_state_dirty.set(true);
        self.calculated_characteristics_cache.borrow_mut().clear();
    }

    fn mark_continuous_state_clean(&self) {
        self.continuous_state_dirty.set(false);
    }

    pub(crate) fn continuous_state_is_clean(&self) -> bool {
        !self.continuous_state_dirty.get()
    }

    /// Set the deterministic RNG seed for this match.
    pub fn set_random_seed(&mut self, seed: u64) {
        self.random_state.set(Self::normalize_random_seed(seed));
    }

    /// Return the current deterministic RNG state.
    pub fn random_seed(&self) -> u64 {
        self.random_state.get()
    }

    /// Return the count of irreversible random gameplay operations that have occurred.
    pub fn irreversible_random_count(&self) -> u64 {
        self.irreversible_random_count.get()
    }

    fn record_irreversible_random(&self) {
        self.irreversible_random_count
            .set(self.irreversible_random_count.get().wrapping_add(1));
    }

    /// Advance the deterministic RNG and return the next 64 random bits.
    pub fn next_random_u64(&self) -> u64 {
        let mut z = self.random_state.get().wrapping_add(0x9e37_79b9_7f4a_7c15);
        self.random_state.set(z);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Shuffle a slice using the deterministic match RNG.
    pub fn shuffle_slice<T>(&self, values: &mut [T]) {
        self.record_irreversible_random();
        let mut rng = StdRng::seed_from_u64(self.next_random_u64());
        values.shuffle(&mut rng);
    }

    /// Shuffle a player's library using the deterministic match RNG.
    pub fn shuffle_player_library(&mut self, player_id: PlayerId) {
        self.record_irreversible_random();
        let seed = self.next_random_u64();
        let Some(index) = self
            .players
            .iter()
            .position(|player| player.id == player_id)
        else {
            return;
        };
        let mut rng = StdRng::seed_from_u64(seed);
        self.players[index].library.shuffle(&mut rng);
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
            consumed_next_untap: false,
        });
    }

    pub fn add_goad_effect(
        &mut self,
        creature: ObjectId,
        goaded_by: PlayerId,
        duration: crate::effect::Until,
        source: ObjectId,
    ) {
        let current_turn = self.turn.turn_number;
        if self.goad_effects.iter().any(|effect| {
            effect.creature == creature
                && effect.goaded_by == goaded_by
                && effect.is_active(self, current_turn)
        }) {
            return;
        }

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

    pub fn add_temporary_spell_cost_reduction(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        filter: crate::target::ObjectFilter,
        reduction: crate::mana::ManaCost,
        remaining_uses: u32,
    ) {
        self.temporary_spell_cost_reductions
            .push(TemporarySpellCostReductionEffectInstance {
                player,
                source,
                filter,
                reduction,
                remaining_uses,
                expires_end_of_turn: self.turn.turn_number,
            });
    }

    pub fn add_temporary_spell_ability_grant(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        filter: crate::target::ObjectFilter,
        ability: crate::static_abilities::StaticAbility,
        remaining_uses: u32,
    ) {
        self.temporary_spell_ability_grants
            .push(TemporarySpellAbilityGrantEffectInstance {
                player,
                source,
                filter,
                ability,
                remaining_uses,
                expires_end_of_turn: self.turn.turn_number,
            });
    }

    pub fn temporary_granted_spell_abilities(
        &self,
        spell_id: ObjectId,
        player: PlayerId,
    ) -> Vec<crate::static_abilities::StaticAbility> {
        let Some(spell_obj) = self.object(spell_id).cloned() else {
            return Vec::new();
        };
        let current_turn = self.turn.turn_number;
        let ctx = crate::filter::FilterContext::new(player)
            .with_source(spell_id)
            .with_active_player(self.turn.active_player)
            .with_opponents(
                self.turn_order
                    .iter()
                    .copied()
                    .filter(|player_id| *player_id != player)
                    .collect(),
            )
            .with_caster(Some(player));
        self.temporary_spell_ability_grants
            .iter()
            .filter(|effect| {
                effect.player == player
                    && !effect.is_expired(current_turn)
                    && effect.filter.matches(&spell_obj, &ctx, self)
            })
            .map(|effect| effect.ability.clone())
            .collect()
    }

    pub fn consume_temporary_spell_ability_grants_for_spell(
        &mut self,
        spell_id: ObjectId,
        player: PlayerId,
    ) {
        let Some(spell_obj) = self.object(spell_id).cloned() else {
            return;
        };
        let current_turn = self.turn.turn_number;
        let ctx = crate::filter::FilterContext::new(player)
            .with_source(spell_id)
            .with_active_player(self.turn.active_player)
            .with_opponents(
                self.turn_order
                    .iter()
                    .copied()
                    .filter(|player_id| *player_id != player)
                    .collect(),
            )
            .with_caster(Some(player));
        let matching = self
            .temporary_spell_ability_grants
            .iter()
            .enumerate()
            .filter_map(|(idx, effect)| {
                (effect.player == player
                    && !effect.is_expired(current_turn)
                    && effect.filter.matches(&spell_obj, &ctx, self))
                .then_some(idx)
            })
            .collect::<Vec<_>>();
        for idx in matching {
            if let Some(effect) = self.temporary_spell_ability_grants.get_mut(idx)
                && effect.remaining_uses > 0
            {
                effect.remaining_uses -= 1;
            }
        }
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

    pub fn cleanup_mana_spend_permissions_end_of_turn(&mut self) {
        self.mana_spend_effects
            .cleanup_expired(self.turn.turn_number);
    }

    pub fn cleanup_temporary_spell_cost_reductions_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        self.temporary_spell_cost_reductions
            .retain(|effect| !effect.is_expired(current_turn));
    }

    pub fn cleanup_temporary_spell_ability_grants_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        self.temporary_spell_ability_grants
            .retain(|effect| !effect.is_expired(current_turn));
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

    /// Returns true if a player can currently pay life for the given reason.
    pub fn can_pay_life_with_reason(
        &self,
        player: PlayerId,
        amount: u32,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        if reason.is_cast_or_ability_payment()
            && self.player_cant_pay_life_to_cast_or_activate(player)
            && amount > 0
        {
            return false;
        }
        self.can_pay_life(player, amount)
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

    /// Can the permanent untap during the specified player's untap step?
    pub fn can_untap_during_step(&self, permanent: ObjectId, untap_player: PlayerId) -> bool {
        self.object(permanent).is_some_and(|object| {
            self.cant_effects
                .can_untap_during_step(permanent, object.controller, untap_player)
        })
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

    /// Can the player become the monarch?
    pub fn can_become_monarch(&self, player: PlayerId) -> bool {
        self.cant_effects.can_become_monarch(player)
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

    /// Can this player be targeted by the specified source object?
    pub fn can_target_player_from_source(&self, player: PlayerId, source_id: ObjectId) -> bool {
        self.cant_effects
            .can_target_player_from_source(self, player, source_id)
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
        self.mark_continuous_state_dirty();
        let zone = object.zone;
        let id = object.id;
        let owner = object.owner;
        let stable_id = object.stable_id;

        self.objects.insert(id, object);
        self.stable_id_index.insert(stable_id, id);

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
        self.prime_linked_face_lookup(card.other_face_name.as_deref(), card.other_face);
        let id = self.new_object_id();
        let mut object = Object::from_card(id, card, owner, zone);
        if zone == Zone::Battlefield
            && let Some(loyalty) = object.base_loyalty
            && loyalty > 0
        {
            object.add_counters(crate::object::CounterType::Loyalty, loyalty);
        }
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
        self.prime_linked_face_definitions(def);
        let id = self.new_object_id();
        let mut object = Object::from_card_definition(id, def, owner, zone);
        if zone == Zone::Battlefield
            && let Some(loyalty) = object.base_loyalty
            && loyalty > 0
        {
            object.add_counters(crate::object::CounterType::Loyalty, loyalty);
        }
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
                if let Some(new_id) = self.move_object_by_game_rule(id, Zone::Hand) {
                    drawn.push(new_id);
                }
            } else {
                // Can't draw from empty library
                break;
            }
        }
        drawn
    }

    /// Draws cards for a player, allowing commander draw replacements to be chosen.
    ///
    /// Only cards that actually move to hand are returned.
    pub fn draw_cards_with_dm(
        &mut self,
        player: PlayerId,
        count: usize,
        decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
    ) -> Vec<ObjectId> {
        let mut drawn = Vec::new();
        for _ in 0..count {
            let card_id = if let Some(player_obj) = self.player(player) {
                player_obj.library.last().copied()
            } else {
                None
            };

            let Some(id) = card_id else {
                break;
            };

            let final_zone =
                self.resolve_commander_move_destination(id, Zone::Hand, decision_maker);
            if let Some(new_id) = self.move_object_by_game_rule(id, final_zone)
                && final_zone == Zone::Hand
            {
                drawn.push(new_id);
            }
        }
        drawn
    }

    /// Moves an object to a new zone.
    /// Per MTG rule 400.7, this creates a new object (new ID).
    /// Returns the new ObjectId.
    fn create_meld_component_object(
        &mut self,
        component: &MeldComponentState,
        zone: Zone,
    ) -> Option<ObjectId> {
        let def = self.linked_face_definition_by_name_or_id(Some(&component.name), None)?;
        let new_id = self.new_object_id();
        let mut object =
            crate::object::Object::from_card_definition(new_id, &def, component.owner, zone);
        object.stable_id = component.stable_id;
        self.add_object(object);
        Some(new_id)
    }

    fn cache_linked_face_definition(&mut self, def: &crate::cards::CardDefinition) {
        self.linked_face_definitions_by_id
            .insert(def.card.id, def.clone());
        self.linked_face_definitions_by_name
            .insert(def.card.name.clone(), def.clone());
    }

    fn prime_linked_face_lookup(&mut self, name: Option<&str>, id: Option<crate::ids::CardId>) {
        if let Some(other_def) = crate::cards::linked_face_definition_by_name_or_id(name, id) {
            self.cache_linked_face_definition(&other_def);
        }
    }

    fn prime_linked_face_definitions(&mut self, def: &crate::cards::CardDefinition) {
        if def.card.other_face.is_none() && def.card.other_face_name.is_none() {
            return;
        }

        self.cache_linked_face_definition(def);
        self.prime_linked_face_lookup(def.card.other_face_name.as_deref(), def.card.other_face);
    }

    pub fn linked_face_definition_by_name_or_id(
        &self,
        name: Option<&str>,
        id: Option<crate::ids::CardId>,
    ) -> Option<crate::cards::CardDefinition> {
        if let Some(card_id) = id
            && let Some(definition) = self.linked_face_definitions_by_id.get(&card_id)
        {
            return Some(definition.clone());
        }

        if let Some(face_name) = name
            && let Some(definition) = self.linked_face_definitions_by_name.get(face_name)
        {
            return Some(definition.clone());
        }

        crate::cards::linked_face_definition_by_name_or_id(name, id)
    }

    pub fn move_object(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
    ) -> Option<ObjectId> {
        let was_face_down = self.is_face_down(old_id);
        let preserved_exile_viewers = if self
            .objects
            .get(&old_id)
            .is_some_and(|obj| obj.zone == Zone::Exile)
        {
            self.face_down_exile_viewers.remove(&old_id)
        } else {
            None
        };
        // Capture a full pre-move snapshot for LKI-based trigger matching.
        let pre_move_snapshot = self.objects.get(&old_id).map(|obj| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, self)
        });

        let old_object = self.objects.remove(&old_id)?;
        self.stable_id_index.remove(&old_object.stable_id);
        self.declined_commander_command_zone_moves.remove(&old_id);
        let old_zone = old_object.zone;
        let owner = old_object.owner;

        if let Some(target) = old_object.attached_to {
            match target {
                AttachmentTarget::Object(id) => {
                    if let Some(parent) = self.object_mut(id) {
                        parent.attachments.retain(|existing| *existing != old_id);
                    }
                }
                AttachmentTarget::Player(id) => {
                    if let Some(player) = self.player_mut(id) {
                        player.attachments.retain(|existing| *existing != old_id);
                    }
                }
            }
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

        if old_zone == Zone::Battlefield
            && new_zone != Zone::Battlefield
            && let Some(melded) = self.melded_permanent(old_object.stable_id).cloned()
        {
            let mut result_object_ids = Vec::with_capacity(melded.components.len());
            for component in &melded.components {
                let new_component_id = self.create_meld_component_object(component, new_zone)?;
                result_object_ids.push(new_component_id);
            }
            self.melded_permanents.remove(&old_object.stable_id);

            use crate::events::zones::ZoneChangeEvent;
            use crate::triggers::TriggerEvent;

            let event = ZoneChangeEvent::with_results(
                old_id,
                result_object_ids.clone(),
                old_zone,
                new_zone,
                cause,
                pre_move_snapshot,
            );
            let event_provenance = self
                .provenance_graph
                .alloc_root_event(crate::events::EventKind::ZoneChange);
            self.queue_trigger_event(
                event_provenance,
                TriggerEvent::new_with_provenance(event, event_provenance),
            );
            self.record_zone_change_results(old_id, result_object_ids.clone());

            #[cfg(debug_assertions)]
            self.debug_assert_zone_consistency();

            self.reconcile_ring_bearers();

            return result_object_ids.first().copied();
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
        let preserve_face_down_overlay =
            new_zone == Zone::Battlefield && new_object.face_down_cast_state.is_some();
        let preserve_bestow_overlay =
            new_zone == Zone::Battlefield && new_object.bestow_cast_state.is_some();
        if !preserve_face_down_overlay && !preserve_bestow_overlay {
            new_object.keyword_payment_contributions_to_cast.clear();
            new_object.x_value = None;
            new_object.bestow_cast_state = None;
            new_object.face_down_cast_state = None;
        }

        // Set battlefield state for new permanents
        if new_zone == Zone::Battlefield {
            self.set_summoning_sick(new_id);
        }

        self.add_object(new_object);

        if new_zone == Zone::Battlefield
            && (was_face_down
                || self
                    .object(new_id)
                    .is_some_and(|obj| obj.face_down_cast_state.is_some()))
        {
            self.set_face_down(new_id);
        }
        if old_zone == Zone::Exile && new_zone == Zone::Exile && was_face_down {
            self.set_face_down(new_id);
            if let Some(viewers) = preserved_exile_viewers {
                for viewer in viewers {
                    self.grant_face_down_exile_view(new_id, viewer);
                }
            }
        }

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
            let event = ZoneChangeEvent::with_cause(
                event_object_id,
                old_zone,
                new_zone,
                cause,
                pre_move_snapshot,
            );
            let mut event = event;
            if old_zone == Zone::Battlefield {
                event.result_objects = vec![new_id];
            }
            let event_provenance = self
                .provenance_graph
                .alloc_root_event(crate::events::EventKind::ZoneChange);
            self.queue_trigger_event(
                event_provenance,
                TriggerEvent::new_with_provenance(event, event_provenance),
            );
        }
        self.record_zone_change_results(old_id, vec![new_id]);

        // Validate zone consistency in debug builds
        #[cfg(debug_assertions)]
        self.debug_assert_zone_consistency();

        self.reconcile_ring_bearers();

        Some(new_id)
    }

    pub fn move_object_by_effect(&mut self, old_id: ObjectId, new_zone: Zone) -> Option<ObjectId> {
        self.move_object(old_id, new_zone, crate::events::cause::EventCause::effect())
    }

    pub fn move_object_by_game_rule(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
    ) -> Option<ObjectId> {
        self.move_object(
            old_id,
            new_zone,
            crate::events::cause::EventCause::from_game_rule(),
        )
    }

    pub fn move_object_by_sba(&mut self, old_id: ObjectId, new_zone: Zone) -> Option<ObjectId> {
        self.move_object(
            old_id,
            new_zone,
            crate::events::cause::EventCause::from_sba(),
        )
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
        self.move_object_with_etb_processing_with_dm_and_cause(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
        )
    }

    /// Move an object to the battlefield with ETB replacement processing and an explicit cause.
    pub fn move_object_with_etb_processing_with_dm_and_cause(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut impl crate::decision::DecisionMaker,
    ) -> Option<EntersResult> {
        let old_zone = self.object(old_id)?.zone;

        // Only process ETB replacement for moves TO the battlefield
        if new_zone != Zone::Battlefield {
            let new_id = self.move_object(old_id, new_zone, cause.clone())?;
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
                let new_id = self.move_object(old_id, dest, cause.clone())?;
                return Some(EntersResult {
                    new_id,
                    enters_tapped: false,
                });
            }
            return None;
        }

        // Proceed with normal battlefield entry
        let new_id = self.move_object(old_id, Zone::Battlefield, cause)?;

        // Apply "enters as copy" before tapped/counter modifications.
        if let Some(copy_source_id) = result.enters_as_copy_of {
            let copy_source = self.object(copy_source_id).cloned();
            if let (Some(source_obj), Some(new_obj)) = (copy_source, self.object_mut(new_id)) {
                new_obj.copy_copiable_values_from(&source_obj);
            }
        }
        if !result.added_card_types.is_empty()
            && let Some(new_obj) = self.object_mut(new_id)
        {
            for card_type in &result.added_card_types {
                if !new_obj.card_types.contains(card_type) {
                    new_obj.card_types.push(*card_type);
                }
            }
        }
        if !result.added_subtypes.is_empty()
            && let Some(new_obj) = self.object_mut(new_id)
        {
            for subtype in &result.added_subtypes {
                if !new_obj.subtypes.contains(subtype) {
                    new_obj.subtypes.push(*subtype);
                }
            }
        }
        if !result.added_abilities.is_empty()
            && let Some(new_obj) = self.object_mut(new_id)
        {
            for ability in &result.added_abilities {
                if !new_obj.abilities.contains(ability) {
                    new_obj.abilities.push(ability.clone());
                }
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
                        if let Some(chosen_color) =
                            chosen.pop().filter(|color| options.contains(color))
                        {
                            self.set_chosen_color(new_id, chosen_color);
                        }
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
                                crate::decisions::spec::DisplayOption::new(idx, subtype.to_string())
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
                        if let Some(chosen_idx) = chosen.pop().filter(|idx| *idx < options.len()) {
                            self.set_chosen_basic_land_type(new_id, options[chosen_idx]);
                        }
                    }
                    if static_ability.land_type_choice_as_enters().is_some() {
                        let options = crate::types::Subtype::all_land_types();
                        let display_options = options
                            .iter()
                            .enumerate()
                            .map(|(idx, subtype)| {
                                crate::decisions::spec::DisplayOption::new(idx, subtype.to_string())
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
                        if let Some(chosen_idx) = chosen.pop().filter(|idx| *idx < options.len()) {
                            self.set_chosen_land_type(new_id, options[chosen_idx]);
                        }
                    }
                    if static_ability.creature_type_choice_as_enters().is_some() {
                        let options =
                            crate::effects::BecomeCreatureTypeChoiceEffect::all_creature_types();
                        let display_options = options
                            .iter()
                            .enumerate()
                            .map(|(idx, subtype)| {
                                crate::decisions::spec::DisplayOption::new(idx, subtype.to_string())
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
                        if let Some(chosen_idx) = chosen.pop().filter(|idx| *idx < options.len()) {
                            self.set_chosen_creature_type(new_id, options[chosen_idx]);
                        }
                    }
                    if static_ability.player_choice_as_enters().is_some() {
                        let options = self
                            .players
                            .iter()
                            .filter(|player| player.is_in_game())
                            .map(|player| player.id)
                            .collect::<Vec<_>>();
                        if options.is_empty() {
                            continue;
                        }
                        let display_options = options
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, player_id)| {
                                self.player(*player_id).map(|player| {
                                    crate::decisions::spec::DisplayOption::new(
                                        idx,
                                        player.name.clone(),
                                    )
                                })
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
                        if let Some(chosen_idx) = chosen.pop().filter(|idx| *idx < options.len()) {
                            self.set_chosen_player(new_id, options[chosen_idx]);
                        }
                    }
                    if let Some(spec) = static_ability.named_option_choice_as_enters() {
                        if spec.options.is_empty() {
                            continue;
                        }
                        let display_options = spec
                            .options
                            .iter()
                            .enumerate()
                            .map(|(idx, option)| {
                                crate::decisions::spec::DisplayOption::new(idx, option.clone())
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
                        if let Some(chosen_idx) =
                            chosen.pop().filter(|idx| *idx < spec.options.len())
                        {
                            self.set_chosen_named_option(new_id, spec.options[chosen_idx].clone());
                        }
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
            let chosen_target = match filter {
                AuraAttachmentFilter::Object(filter) => {
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
                        None
                    } else {
                        let ctx = crate::decisions::context::SelectObjectsContext::new(
                            chooser,
                            Some(new_id),
                            "Attach Aura to",
                            candidates,
                            1,
                            Some(1),
                        );
                        decision_maker
                            .decide_objects(self, &ctx)
                            .first()
                            .copied()
                            .map(AttachmentTarget::Object)
                    }
                }
                AuraAttachmentFilter::Player(filter) => {
                    let candidates = self
                        .players
                        .iter()
                        .filter(|player| {
                            player.is_in_game() && filter.matches_player(player.id, &filter_ctx)
                        })
                        .map(|player| (player.id, player.name.clone()))
                        .collect::<Vec<_>>();
                    if candidates.is_empty() {
                        None
                    } else if candidates.len() == 1 {
                        Some(AttachmentTarget::Player(candidates[0].0))
                    } else {
                        let choice_spec = crate::decisions::specs::ChoiceSpec::single(
                            new_id,
                            candidates
                                .iter()
                                .enumerate()
                                .map(|(idx, (_, name))| {
                                    crate::decisions::spec::DisplayOption::new(idx, name.clone())
                                })
                                .collect(),
                        );
                        let mut chosen = crate::decisions::make_decision(
                            self,
                            decision_maker,
                            chooser,
                            Some(new_id),
                            choice_spec,
                        );
                        chosen
                            .pop()
                            .and_then(|idx| candidates.get(idx).map(|(player_id, _)| *player_id))
                            .map(AttachmentTarget::Player)
                            .or_else(|| Some(AttachmentTarget::Player(candidates[0].0)))
                    }
                }
            };

            if let Some(target) = chosen_target {
                if self.attach_object_to_target(new_id, target) {
                    self.continuous_effects.record_attachment(new_id);
                }
            } else {
                // No legal attachment target - put the Aura into the graveyard
                self.move_object_by_effect(new_id, Zone::Graveyard);
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
            if let Some(target) = obj.attached_to {
                match target {
                    AttachmentTarget::Object(parent_id) => {
                        if let Some(parent) = self.object_mut(parent_id) {
                            parent.attachments.retain(|existing| *existing != id);
                        }
                    }
                    AttachmentTarget::Player(player_id) => {
                        if let Some(player) = self.player_mut(player_id) {
                            player.attachments.retain(|existing| *existing != id);
                        }
                    }
                }
            }
            self.stable_id_index.remove(&obj.stable_id);
            self.melded_permanents.remove(&obj.stable_id);
            self.declined_commander_command_zone_moves.remove(&id);
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
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Battlefield => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in battlefield index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in battlefield index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check exile
        for &id in &self.exile {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Exile => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in exile index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in exile index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check command zone
        for &id in &self.command_zone {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Command => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in command zone index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in command zone index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check player zones
        for player in &self.players {
            // Library
            for &id in &player.library {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Library => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s library has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s library doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }

            // Hand
            for &id in &player.hand {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Hand => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s hand has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s hand doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }

            // Graveyard
            for &id in &player.graveyard {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Graveyard => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s graveyard has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s graveyard doesn't exist in objects",
                            id.0, player.name
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
                    "Object #{} with zone {} is not in any zone index",
                    id.0, obj.zone
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
        self.mark_continuous_state_dirty();
        self.objects.get_mut(&id)
    }

    pub(crate) fn objects_map(&self) -> &HashMap<ObjectId, Object> {
        &self.objects
    }

    pub fn attachment_target_exists_on_battlefield(&self, target: AttachmentTarget) -> bool {
        match target {
            AttachmentTarget::Object(id) => self
                .object(id)
                .is_some_and(|object| object.zone == Zone::Battlefield),
            AttachmentTarget::Player(id) => {
                self.player(id).is_some_and(|player| player.is_in_game())
            }
        }
    }

    pub fn detach_object_from_current_target(&mut self, attachment_id: ObjectId) -> bool {
        self.mark_continuous_state_dirty();
        let Some(current_target) = self
            .object(attachment_id)
            .and_then(|object| object.attached_to)
        else {
            return false;
        };

        match current_target {
            AttachmentTarget::Object(id) => {
                if let Some(parent) = self.object_mut(id) {
                    parent
                        .attachments
                        .retain(|existing| *existing != attachment_id);
                }
            }
            AttachmentTarget::Player(id) => {
                if let Some(player) = self.player_mut(id) {
                    player
                        .attachments
                        .retain(|existing| *existing != attachment_id);
                }
            }
        }

        if let Some(object) = self.object_mut(attachment_id) {
            object.attached_to = None;
        }

        true
    }

    pub fn attach_object_to_target(
        &mut self,
        attachment_id: ObjectId,
        target: AttachmentTarget,
    ) -> bool {
        self.mark_continuous_state_dirty();
        if !self
            .object(attachment_id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
            || !self.attachment_target_exists_on_battlefield(target)
        {
            return false;
        }

        self.detach_object_from_current_target(attachment_id);

        if let Some(object) = self.object_mut(attachment_id) {
            object.attached_to = Some(target);
        } else {
            return false;
        }

        match target {
            AttachmentTarget::Object(id) => {
                if let Some(parent) = self.object_mut(id)
                    && !parent.attachments.contains(&attachment_id)
                {
                    parent.attachments.push(attachment_id);
                }
            }
            AttachmentTarget::Player(id) => {
                if let Some(player) = self.player_mut(id)
                    && !player.attachments.contains(&attachment_id)
                {
                    player.attachments.push(attachment_id);
                }
            }
        }

        true
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
        self.mark_continuous_state_dirty();
        let obj = self.object_mut(id)?;
        obj.add_counters(counter_type, amount);

        let event_provenance = self
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::CounterPlaced);
        Some(crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::CounterPlacedEvent::new(id, counter_type, amount),
            event_provenance,
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
        self.mark_continuous_state_dirty();
        let obj = self.object_mut(id)?;
        let removed = obj.remove_counters(counter_type, amount);

        if removed == 0 {
            return None;
        }

        let event_provenance = self
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::MarkersChangedEvent::removed(
                counter_type,
                id,
                removed,
                source,
                source_controller,
            ),
            event_provenance,
        );

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
        self.mark_continuous_state_dirty();
        if amount == 0 {
            return None;
        }

        let obj = self.object_mut(id)?;
        obj.add_counters(counter_type, amount);

        let event_provenance = self
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        Some(crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::MarkersChangedEvent::added(
                counter_type,
                id,
                amount,
                source,
                source_controller,
            ),
            event_provenance,
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

        let event_provenance = self
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        Some(crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::MarkersChangedEvent::added(
                counter_type,
                player_id,
                amount,
                source,
                source_controller,
            ),
            event_provenance,
        ))
    }

    /// Remove counters from a player and emit a unified marker event when applicable.
    ///
    /// Returns the actual number removed and the corresponding event.
    pub fn remove_player_counters_with_source(
        &mut self,
        player_id: PlayerId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<(u32, crate::triggers::TriggerEvent)> {
        if amount == 0 {
            return None;
        }

        let player = self.player_mut(player_id)?;
        let removed = match counter_type {
            crate::object::CounterType::Poison => {
                let removed = player.poison_counters.min(amount);
                player.poison_counters = player.poison_counters.saturating_sub(removed);
                removed
            }
            crate::object::CounterType::Energy => {
                let removed = player.energy_counters.min(amount);
                player.energy_counters = player.energy_counters.saturating_sub(removed);
                removed
            }
            crate::object::CounterType::Experience => {
                let removed = player.experience_counters.min(amount);
                player.experience_counters = player.experience_counters.saturating_sub(removed);
                removed
            }
            _ => return None,
        };

        if removed == 0 {
            return None;
        }

        let event_provenance = self
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        Some((
            removed,
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::MarkersChangedEvent::removed(
                    counter_type,
                    player_id,
                    removed,
                    source,
                    source_controller,
                ),
                event_provenance,
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
        if self.continuous_state_is_clean() {
            return self.cached_continuous_effects_snapshot();
        }
        crate::static_ability_processor::get_all_continuous_effects(self)
    }

    /// Combine registered and cached static-ability continuous effects.
    ///
    /// Unlike `all_continuous_effects`, this does not regenerate static-ability
    /// effects dynamically. Callers must only use this after
    /// `refresh_continuous_state` (or `update_static_ability_effects`) for the
    /// current state.
    pub(crate) fn cached_continuous_effects_snapshot(&self) -> Vec<ContinuousEffect> {
        let mut effects: Vec<ContinuousEffect> = self
            .continuous_effects
            .effects_sorted()
            .into_iter()
            .cloned()
            .collect();
        effects.reserve(self.continuous_effects.static_ability_effects().len());
        effects.extend(
            self.continuous_effects
                .static_ability_effects()
                .iter()
                .cloned(),
        );
        effects
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
        if let Some(chars) = crate::continuous::in_progress_characteristics(id) {
            return Some(chars);
        }
        crate::continuous::calculate_characteristics_with_effects(
            id,
            &self.objects,
            effects,
            &self.battlefield,
            &self.commanders,
            self,
        )
    }

    pub(crate) fn calculated_characteristics_batch_with_effects(
        &self,
        ids: &[ObjectId],
        effects: &[ContinuousEffect],
    ) -> HashMap<ObjectId, crate::continuous::CalculatedCharacteristics> {
        crate::continuous::calculate_characteristics_batch_with_effects(
            ids,
            &self.objects,
            effects,
            &self.battlefield,
            &self.commanders,
            self,
        )
    }

    pub(crate) fn prewarm_calculated_characteristics(&self, ids: &[ObjectId]) {
        if !self.continuous_state_is_clean() {
            return;
        }

        let effects_revision = self.continuous_effects.revision();
        if self.calculated_characteristics_cache_revision.get() != effects_revision {
            self.calculated_characteristics_cache.borrow_mut().clear();
            self.calculated_characteristics_cache_revision
                .set(effects_revision);
        }

        let missing: Vec<_> = {
            let cache = self.calculated_characteristics_cache.borrow();
            ids.iter()
                .copied()
                .filter(|id| !cache.contains_key(id))
                .collect()
        };
        if missing.is_empty() {
            return;
        }

        let effects = self.cached_continuous_effects_snapshot();
        let calculated = self.calculated_characteristics_batch_with_effects(&missing, &effects);
        let mut cache = self.calculated_characteristics_cache.borrow_mut();
        for id in missing {
            cache.insert(id, calculated.get(&id).cloned());
        }
    }

    pub fn calculated_characteristics(
        &self,
        id: ObjectId,
    ) -> Option<crate::continuous::CalculatedCharacteristics> {
        if let Some(chars) = crate::continuous::in_progress_characteristics(id) {
            return Some(chars);
        }
        let effects_revision = self.continuous_effects.revision();
        if self.continuous_state_is_clean() {
            if self.calculated_characteristics_cache_revision.get() != effects_revision {
                self.calculated_characteristics_cache.borrow_mut().clear();
                self.calculated_characteristics_cache_revision
                    .set(effects_revision);
            }

            if let Some(cached) = self.calculated_characteristics_cache.borrow().get(&id) {
                return cached.clone();
            }
        }

        let all_effects = self.all_continuous_effects();
        let calculated = self.calculated_characteristics_with_effects(id, &all_effects);
        if self.continuous_state_is_clean() {
            self.calculated_characteristics_cache_revision
                .set(effects_revision);
            self.calculated_characteristics_cache
                .borrow_mut()
                .insert(id, calculated.clone());
        }
        calculated
    }

    /// Return the object's current characteristics in its zone.
    ///
    /// This view reflects continuous effects across all zones and expands
    /// semantic subtype implications like changeling.
    pub fn current_characteristics(&self, id: ObjectId) -> Option<CalculatedCharacteristics> {
        let object = self.object(id)?;
        let mut chars =
            self.calculated_characteristics(id)
                .unwrap_or_else(|| CalculatedCharacteristics {
                    name: object.name.clone(),
                    oracle_text: object.oracle_text.clone(),
                    power: object.power(),
                    toughness: object.toughness(),
                    card_types: object.card_types.clone(),
                    subtypes: object.subtypes.clone(),
                    supertypes: object.supertypes.clone(),
                    colors: object.colors(),
                    abilities: object.abilities.clone(),
                    static_abilities: object
                        .abilities
                        .iter()
                        .filter_map(|ability| match &ability.kind {
                            AbilityKind::Static(static_ability) => Some(static_ability.clone()),
                            _ => None,
                        })
                        .chain(object.level_granted_abilities().iter().cloned())
                        .collect(),
                    controller: object.controller,
                });

        let has_changeling = chars
            .static_abilities
            .iter()
            .any(|ability| ability.id() == crate::static_abilities::StaticAbilityId::Changeling);
        let can_have_creature_subtypes = chars.card_types.iter().any(|card_type| {
            matches!(
                card_type,
                crate::types::CardType::Creature | crate::types::CardType::Kindred
            )
        });
        if has_changeling && can_have_creature_subtypes {
            for subtype in crate::types::Subtype::all_creature_types() {
                if !chars.subtypes.contains(subtype) {
                    chars.subtypes.push(*subtype);
                }
            }
        }

        Some(chars)
    }

    /// Return the object's current name in its zone.
    pub fn current_name(&self, id: ObjectId) -> Option<String> {
        Some(self.current_characteristics(id)?.name)
    }

    /// Return the object's current controller in its zone.
    pub fn current_controller(&self, id: ObjectId) -> Option<PlayerId> {
        Some(self.current_characteristics(id)?.controller)
    }

    /// Return the object's current card types in its zone.
    pub fn current_card_types(&self, id: ObjectId) -> Option<Vec<crate::types::CardType>> {
        Some(self.current_characteristics(id)?.card_types)
    }

    /// Return the object's current subtypes in its zone.
    pub fn current_subtypes(&self, id: ObjectId) -> Option<Vec<crate::types::Subtype>> {
        Some(self.current_characteristics(id)?.subtypes)
    }

    /// Return the object's current supertypes in its zone.
    pub fn current_supertypes(&self, id: ObjectId) -> Option<Vec<crate::types::Supertype>> {
        Some(self.current_characteristics(id)?.supertypes)
    }

    /// Return the object's current colors in its zone.
    pub fn current_colors(&self, id: ObjectId) -> Option<crate::color::ColorSet> {
        Some(self.current_characteristics(id)?.colors)
    }

    /// Return the object's current power in its zone, if any.
    pub fn current_power(&self, id: ObjectId) -> Option<i32> {
        self.current_characteristics(id)?.power
    }

    /// Return the object's current toughness in its zone, if any.
    pub fn current_toughness(&self, id: ObjectId) -> Option<i32> {
        self.current_characteristics(id)?.toughness
    }

    /// Return the abilities an object currently has in its zone.
    pub fn current_abilities(&self, id: ObjectId) -> Option<Vec<Ability>> {
        Some(self.current_characteristics(id)?.abilities)
    }

    /// Return a specific current ability by index.
    pub fn current_ability(&self, id: ObjectId, ability_index: usize) -> Option<Ability> {
        self.current_abilities(id)?.get(ability_index).cloned()
    }

    /// Return a specific current activated ability by index.
    pub fn current_activated_ability(
        &self,
        id: ObjectId,
        ability_index: usize,
    ) -> Option<ActivatedAbility> {
        let ability = self.current_ability(id, ability_index)?;
        match ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        }
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
        self.current_has_static_ability_id(id, ability_id)
    }

    /// Check if an object currently has a static ability with the given ID.
    pub fn current_has_static_ability_id(
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
        self.current_card_types(id)
            .is_some_and(|card_types| card_types.contains(&card_type))
    }

    /// Check if an object currently has a specific card type.
    pub fn current_has_card_type(&self, id: ObjectId, card_type: crate::types::CardType) -> bool {
        self.object_has_card_type(id, card_type)
    }

    /// Check if an object currently has a specific subtype.
    pub fn current_has_subtype(&self, id: ObjectId, subtype: crate::types::Subtype) -> bool {
        self.current_subtypes(id)
            .is_some_and(|subtypes| subtypes.contains(&subtype))
    }

    /// Check if an object currently has a specific supertype.
    pub fn current_has_supertype(&self, id: ObjectId, supertype: crate::types::Supertype) -> bool {
        self.current_supertypes(id)
            .is_some_and(|supertypes| supertypes.contains(&supertype))
    }

    /// Check if an object is currently a creature.
    pub fn current_is_creature(&self, id: ObjectId) -> bool {
        self.current_has_card_type(id, crate::types::CardType::Creature)
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
        self.mana_spend_effects
            .retain_effect_permissions(self.turn.turn_number);
        self.damage_persists.clear();
        for player in &mut self.players {
            player.max_hand_size = 7;
            player.land_plays_per_turn = 1;
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
        self.mark_continuous_state_clean();
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
        if self.continuous_state_is_clean() {
            return;
        }

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
        self.mana_spend_effects
            .permissions
            .iter()
            .any(|permission| permission.allows(self, payer, source))
    }

    fn with_active_battlefield_static_abilities<T>(
        &self,
        mut f: impl FnMut(ObjectId, PlayerId, &crate::static_abilities::StaticAbility) -> Option<T>,
    ) -> Option<T> {
        let all_effects = self.all_continuous_effects();
        for &perm_id in &self.battlefield {
            let Some(object) = self.object(perm_id) else {
                continue;
            };
            let static_abilities = self
                .calculated_characteristics_with_effects(perm_id, &all_effects)
                .map(|chars| chars.static_abilities)
                .unwrap_or_default();
            for static_ability in static_abilities {
                if !static_ability.is_active(self, perm_id) {
                    continue;
                }
                if let Some(result) = f(perm_id, object.controller, &static_ability) {
                    return Some(result);
                }
            }
        }
        None
    }

    pub fn player_can_pay_black_with_life(
        &self,
        payer: PlayerId,
        _source: Option<ObjectId>,
    ) -> bool {
        self.with_active_battlefield_static_abilities(|_, controller, ability| {
            (controller == payer && ability.black_mana_may_be_paid_with_life()).then_some(true)
        })
        .unwrap_or(false)
    }

    pub fn player_can_pay_black_with_life_for_reason(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        self.player_can_pay_black_with_life(payer, source)
            && (!reason.is_cast_or_ability_payment()
                || !self.player_cant_pay_life_to_cast_or_activate(payer))
    }

    pub fn minimum_total_spell_mana_payment(&self) -> Option<u32> {
        let all_effects = self.all_continuous_effects();
        let mut minimum = None;
        for &perm_id in &self.battlefield {
            let Some(_object) = self.object(perm_id) else {
                continue;
            };
            let static_abilities = self
                .calculated_characteristics_with_effects(perm_id, &all_effects)
                .map(|chars| chars.static_abilities)
                .unwrap_or_default();
            for static_ability in static_abilities {
                if !static_ability.is_active(self, perm_id) {
                    continue;
                }
                if let Some(candidate) = static_ability.minimum_total_spell_mana() {
                    minimum =
                        Some(minimum.map_or(candidate, |current: u32| current.max(candidate)));
                }
            }
        }
        minimum
    }

    pub fn player_cant_pay_life_to_cast_or_activate(&self, player: PlayerId) -> bool {
        self.with_active_battlefield_static_abilities(|_, _, ability| {
            ability
                .forbids_paying_life_for_cast_or_activate()
                .then_some(true)
        })
        .unwrap_or(false)
            && self.player(player).is_some()
    }

    pub fn player_cant_sacrifice_nonland_to_cast_or_activate(&self, player: PlayerId) -> bool {
        self.with_active_battlefield_static_abilities(|_, _, ability| {
            ability
                .forbids_sacrificing_nonland_for_cast_or_activate()
                .then_some(true)
        })
        .unwrap_or(false)
            && self.player(player).is_some()
    }

    fn object_is_land_for_cost_restrictions(&self, object_id: ObjectId) -> bool {
        let Some(object) = self.object(object_id) else {
            return false;
        };
        if object.zone == Zone::Battlefield {
            return self
                .calculated_characteristics(object_id)
                .is_some_and(|chars| chars.card_types.contains(&crate::types::CardType::Land));
        }
        object.card_types.contains(&crate::types::CardType::Land)
    }

    fn required_sacrifice_count_for_cost(&self, cost: &crate::costs::Cost) -> usize {
        if cost.is_sacrifice_self() {
            return 1;
        }
        cost.effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::SacrificeEffect>())
            .and_then(|effect| match effect.count {
                crate::effect::Value::Fixed(count) => Some(count.max(0) as usize),
                _ => None,
            })
            .unwrap_or(1)
    }

    fn legal_land_sacrifice_targets_for_cost(
        &self,
        payer: PlayerId,
        source: ObjectId,
        filter: &crate::filter::ObjectFilter,
    ) -> usize {
        let filter_ctx = crate::filter::FilterContext::new(payer).with_source(source);
        self.battlefield
            .iter()
            .filter_map(|&id| self.object(id).map(|obj| (id, obj)))
            .filter(|(id, obj)| {
                obj.controller == payer
                    && self.object_is_land_for_cost_restrictions(*id)
                    && filter.matches(obj, &filter_ctx, self)
                    && self.can_be_sacrificed(*id)
            })
            .count()
    }

    pub fn validate_cost_for_payment_reason(
        &self,
        payer: PlayerId,
        source: ObjectId,
        cost: &crate::costs::Cost,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), crate::cost::CostPaymentError> {
        if !reason.is_cast_or_ability_payment() {
            return Ok(());
        }

        if self.player_cant_pay_life_to_cast_or_activate(payer) && cost.is_life_cost() {
            return Err(crate::cost::CostPaymentError::InsufficientLife);
        }

        if !self.player_cant_sacrifice_nonland_to_cast_or_activate(payer) {
            return Ok(());
        }

        if cost.is_sacrifice_self() && !self.object_is_land_for_cost_restrictions(source) {
            return Err(crate::cost::CostPaymentError::NoValidSacrificeTarget);
        }

        if let Some(filter) = cost.sacrifice_filter() {
            let required = self.required_sacrifice_count_for_cost(cost);
            if self.legal_land_sacrifice_targets_for_cost(payer, source, filter) < required {
                return Err(crate::cost::CostPaymentError::NoValidSacrificeTarget);
            }
        }

        Ok(())
    }

    pub fn adjust_mana_cost_for_payment_reason(
        &self,
        payer: PlayerId,
        _source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        reason: crate::costs::PaymentReason,
    ) -> crate::mana::ManaCost {
        use crate::mana::ManaSymbol;

        let mut pips = cost.pips().to_vec();

        if reason.is_cast_or_ability_payment()
            && self.player_cant_pay_life_to_cast_or_activate(payer)
        {
            for pip in &mut pips {
                pip.retain(|symbol| !matches!(symbol, ManaSymbol::Life(_)));
            }
        }

        crate::mana::ManaCost::from_pips(pips)
    }

    /// Check if a player can pay a mana cost, accounting for "spend as though any color".
    pub fn can_pay_mana_cost(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
    ) -> bool {
        self.can_pay_mana_cost_with_reason(
            payer,
            source,
            cost,
            x_value,
            crate::costs::PaymentReason::Other,
        )
    }

    /// Check if a player can pay a mana cost for a specific reason.
    pub fn can_pay_mana_cost_with_reason(
        &self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        let Some(player) = self.player(payer) else {
            return false;
        };

        let allow_any_color = self.can_spend_mana_as_any_color(payer, source);
        let allow_black_life =
            self.player_can_pay_black_with_life_for_reason(payer, source, reason);
        let mut preview_pool = player.mana_pool.clone();
        let (can_pay, life_to_pay) = preview_pool
            .try_pay_tracking_life_with_any_color_and_black_life(
                cost,
                x_value,
                allow_any_color,
                allow_black_life,
            );
        can_pay && self.can_pay_life_with_reason(payer, life_to_pay, reason)
    }

    /// Attempt to pay a mana cost, accounting for "spend as though any color".
    pub fn try_pay_mana_cost(
        &mut self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
    ) -> bool {
        self.try_pay_mana_cost_with_reason(
            payer,
            source,
            cost,
            x_value,
            crate::costs::PaymentReason::Other,
        )
    }

    /// Attempt to pay a mana cost for a specific reason.
    pub fn try_pay_mana_cost_with_reason(
        &mut self,
        payer: PlayerId,
        source: Option<ObjectId>,
        cost: &crate::mana::ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        let allow_any_color = self.can_spend_mana_as_any_color(payer, source);
        let allow_black_life =
            self.player_can_pay_black_with_life_for_reason(payer, source, reason);
        let original_pool = self.player(payer).map(|player| player.mana_pool.clone());
        let (paid, life_to_pay) = {
            let Some(player) = self.player_mut(payer) else {
                return false;
            };
            player
                .mana_pool
                .try_pay_tracking_life_with_any_color_and_black_life(
                    cost,
                    x_value,
                    allow_any_color,
                    allow_black_life,
                )
        };
        if !paid {
            return false;
        }
        if !self.can_pay_life_with_reason(payer, life_to_pay, reason) {
            if let (Some(original_pool), Some(player)) = (original_pool, self.player_mut(payer)) {
                player.mana_pool = original_pool;
            }
            return false;
        }
        if life_to_pay > 0 && !self.pay_life(payer, life_to_pay) {
            if let (Some(original_pool), Some(player)) = (original_pool, self.player_mut(payer)) {
                player.mana_pool = original_pool;
            }
            return false;
        }
        true
    }

    /// Gets a reference to a player by ID.
    pub fn player(&self, id: PlayerId) -> Option<&Player> {
        self.players.get(id.index())
    }

    /// Gets a mutable reference to a player by ID.
    pub fn player_mut(&mut self, id: PlayerId) -> Option<&mut Player> {
        self.mark_continuous_state_dirty();
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

    /// Resolve a commander's stable identity from either its original or current object ID.
    pub fn commander_identity(&self, obj_id: ObjectId) -> Option<ObjectId> {
        if self
            .players
            .iter()
            .any(|player| player.commanders.contains(&obj_id))
        {
            return Some(obj_id);
        }

        let obj = self.object(obj_id)?;
        let stable_identity = obj.stable_id.object_id();
        self.players
            .iter()
            .any(|player| player.commanders.contains(&stable_identity))
            .then_some(stable_identity)
    }

    /// Resolve the current object ID for a stored commander identity.
    pub fn current_commander_object(&self, commander_id: ObjectId) -> Option<ObjectId> {
        if self.object(commander_id).is_some() {
            return Some(commander_id);
        }

        self.find_object_by_stable_id(StableId::from(commander_id))
    }

    /// Resolve the destination for a commander moving to hand or library.
    ///
    /// For all other zone changes, this returns `requested_zone` unchanged.
    pub fn resolve_commander_move_destination(
        &self,
        object_id: ObjectId,
        requested_zone: Zone,
        decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
    ) -> Zone {
        let destination_text = match requested_zone {
            Zone::Hand => "putting it into its owner's hand",
            Zone::Library => "putting it into its owner's library",
            _ => return requested_zone,
        };

        if !self.is_commander(object_id) {
            return requested_zone;
        }

        let Some(obj) = self.object(object_id) else {
            return requested_zone;
        };
        let owner = obj.owner;
        let name = obj.name.clone();
        let choice_ctx = crate::decisions::context::BooleanContext::new(
            owner,
            Some(object_id),
            format!("move it to the command zone instead of {destination_text}"),
        )
        .with_source_name(name);

        if decision_maker.decide_boolean(self, &choice_ctx) {
            Zone::Command
        } else {
            requested_zone
        }
    }

    /// Move an object while applying commander hand/library replacement choices.
    pub fn move_object_with_commander_options(
        &mut self,
        object_id: ObjectId,
        requested_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
    ) -> Option<(ObjectId, Zone)> {
        let final_zone =
            self.resolve_commander_move_destination(object_id, requested_zone, decision_maker);
        self.move_object(object_id, final_zone, cause)
            .map(|new_id| (new_id, final_zone))
    }

    /// Returns how many times a commander has been cast from the command zone.
    pub fn commander_cast_count(&self, commander_id: ObjectId) -> u32 {
        let identity = self
            .commander_identity(commander_id)
            .unwrap_or(commander_id);
        self.commander_casts_from_command_zone
            .get(&identity)
            .copied()
            .unwrap_or(0)
    }

    /// Records that a commander was cast from the command zone.
    pub fn record_commander_cast_from_command_zone(&mut self, commander_id: ObjectId) {
        if let Some(identity) = self.commander_identity(commander_id) {
            *self
                .commander_casts_from_command_zone
                .entry(identity)
                .or_insert(0) += 1;
        }
    }

    /// Records combat damage dealt to a player by a commander.
    pub fn record_commander_damage(
        &mut self,
        player_id: PlayerId,
        commander_id: ObjectId,
        amount: u32,
    ) {
        if amount == 0 {
            return;
        }
        let Some(identity) = self.commander_identity(commander_id) else {
            return;
        };
        if let Some(player) = self.player_mut(player_id) {
            player.record_commander_damage(identity, amount);
        }
    }

    /// Returns true if this exact commander object already declined moving to command zone.
    pub fn commander_command_zone_move_declined(&self, object_id: ObjectId) -> bool {
        self.declined_commander_command_zone_moves
            .contains(&object_id)
    }

    /// Mark this commander object as having declined the current command-zone move.
    pub fn decline_commander_command_zone_move(&mut self, object_id: ObjectId) {
        self.declined_commander_command_zone_moves.insert(object_id);
    }

    /// Set the current monarch designation holder.
    ///
    /// Use `None` to clear the designation.
    pub fn set_monarch(&mut self, monarch: Option<PlayerId>) {
        self.monarch = monarch;
    }

    /// Set the current initiative designation holder.
    ///
    /// Use `None` to clear the designation.
    pub fn set_initiative(&mut self, initiative: Option<PlayerId>) {
        self.initiative = initiative;
    }

    /// Reconcile any Ring-bearers that are no longer valid.
    pub fn reconcile_ring_bearers(&mut self) {
        let player_ids = self
            .players
            .iter()
            .map(|player| player.id)
            .collect::<Vec<_>>();
        for player in player_ids {
            self.reconcile_ring_bearer(player);
        }
    }

    /// Reconcile one player's Ring-bearer state against the live battlefield.
    pub fn reconcile_ring_bearer(&mut self, player: PlayerId) {
        if self.current_ring_bearer(player).is_some() {
            return;
        }
        self.clear_ring_bearer(player);
    }

    /// Returns how many times the Ring has tempted this player this game.
    pub fn ring_temptations(&self, player: PlayerId) -> u32 {
        self.player(player)
            .map(|player| player.ring_temptations)
            .unwrap_or(0)
    }

    /// Returns the unlocked Ring tier for this player, capped at four.
    pub fn ring_level(&self, player: PlayerId) -> u32 {
        self.ring_temptations(player).min(4)
    }

    /// Returns the player's current Ring-bearer if it is still valid.
    pub fn current_ring_bearer(&self, player: PlayerId) -> Option<ObjectId> {
        let bearer = self.player(player)?.ring_bearer?;
        if !self.battlefield.contains(&bearer) {
            return None;
        }
        if self.current_controller(bearer) != Some(player) {
            return None;
        }
        if !self.current_is_creature(bearer) {
            return None;
        }
        Some(bearer)
    }

    /// Increments the number of times the Ring has tempted the player.
    pub fn increment_ring_temptations(&mut self, player: PlayerId) {
        if let Some(player_state) = self.player_mut(player) {
            player_state.ring_temptations = player_state.ring_temptations.saturating_add(1);
        }
    }

    /// Clear the player's current Ring-bearer designation.
    pub fn clear_ring_bearer(&mut self, player: PlayerId) {
        let previous_legendary_added = self
            .player(player)
            .and_then(|player_state| player_state.ring_legendary_added);
        if let Some(object_id) = previous_legendary_added
            && let Some(object) = self.object_mut(object_id)
        {
            object
                .supertypes
                .retain(|supertype| *supertype != crate::types::Supertype::Legendary);
        }

        if let Some(player_state) = self.player_mut(player) {
            player_state.ring_bearer = None;
            player_state.ring_legendary_added = None;
        }
    }

    /// Set the player's Ring-bearer designation to the given creature.
    pub fn set_ring_bearer(&mut self, player: PlayerId, bearer: ObjectId) {
        self.clear_ring_bearer(player);

        let mut legendary_added = None;
        if let Some(object) = self.object_mut(bearer)
            && !object.has_supertype(crate::types::Supertype::Legendary)
        {
            object.supertypes.push(crate::types::Supertype::Legendary);
            legendary_added = Some(bearer);
        }

        if let Some(player_state) = self.player_mut(player) {
            player_state.ring_bearer = Some(bearer);
            player_state.ring_legendary_added = legendary_added;
        }
    }

    /// Returns true if the given player is currently the monarch.
    pub fn is_monarch(&self, player: PlayerId) -> bool {
        self.monarch == Some(player)
    }

    /// Returns true if the given player currently has the initiative.
    pub fn has_initiative(&self, player: PlayerId) -> bool {
        self.initiative == Some(player)
    }

    /// Returns the player's active dungeon progress, if any.
    pub fn active_dungeon(&self, player: PlayerId) -> Option<&ActiveDungeonProgress> {
        self.active_dungeons.get(&player)
    }

    /// Set the player's active dungeon progress.
    pub fn set_active_dungeon(&mut self, player: PlayerId, progress: ActiveDungeonProgress) {
        self.active_dungeons.insert(player, progress);
    }

    /// Clear the player's active dungeon progress.
    pub fn clear_active_dungeon(&mut self, player: PlayerId) {
        self.active_dungeons.remove(&player);
    }

    /// Record that the player completed the named dungeon.
    pub fn record_completed_dungeon(&mut self, player: PlayerId, dungeon_name: impl Into<String>) {
        self.completed_dungeons
            .entry(player)
            .or_default()
            .push(dungeon_name.into());
    }

    /// Returns the names of dungeons the player has completed this game.
    pub fn completed_dungeons(&self, player: PlayerId) -> &[String] {
        self.completed_dungeons
            .get(&player)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Returns true if the player has completed one or more dungeons this game.
    pub fn has_completed_dungeon(&self, player: PlayerId) -> bool {
        !self.completed_dungeons(player).is_empty()
    }

    /// Returns true if the player has completed the named dungeon this game.
    pub fn has_completed_named_dungeon(&self, player: PlayerId, dungeon_name: &str) -> bool {
        self.completed_dungeons(player)
            .iter()
            .any(|completed| completed.eq_ignore_ascii_case(dungeon_name))
    }

    /// Returns the count of differently named dungeons the player has completed this game.
    pub fn completed_different_dungeon_names_count(&self, player: PlayerId) -> usize {
        let mut seen = HashSet::new();
        for completed in self.completed_dungeons(player) {
            seen.insert(completed.to_ascii_lowercase());
        }
        seen.len()
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
        match zone {
            Zone::Battlefield => self.battlefield.clone(),
            Zone::Graveyard => self
                .players
                .iter()
                .flat_map(|player| player.graveyard.iter().copied())
                .collect(),
            Zone::Hand => self
                .players
                .iter()
                .flat_map(|player| player.hand.iter().copied())
                .collect(),
            Zone::Library => self
                .players
                .iter()
                .flat_map(|player| player.library.iter().copied())
                .collect(),
            Zone::Stack => self.stack.iter().map(|entry| entry.object_id).collect(),
            Zone::Exile => self.exile.clone(),
            Zone::Command => self.command_zone.clone(),
        }
    }

    /// Returns all object IDs in deterministic order.
    pub fn object_ids_in_deterministic_order(&self) -> Vec<ObjectId> {
        let mut ids: Vec<_> = self.objects.keys().copied().collect();
        ids.sort();
        ids
    }

    /// Returns all objects in deterministic order by object ID.
    pub fn objects_in_deterministic_order(&self) -> Vec<&Object> {
        self.object_ids_in_deterministic_order()
            .into_iter()
            .filter_map(|id| self.objects.get(&id))
            .collect()
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
                    .is_some_and(|o| o.controller == controller && self.current_is_creature(id))
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
        self.spells_cast_last_turn_total = self.turn_history.clear_for_new_turn();
        self.saddled_until_end_of_turn.clear();
        self.ninjutsu_attack_targets.clear();
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
            player.restricted_mana.clear();
        }
    }

    /// Clears the tracking for OncePerTurn activated abilities.
    /// Called at the beginning of each turn.
    pub fn clear_activated_abilities_tracking(&mut self) {
        self.turn_history.activated_abilities_this_turn.clear();
    }

    /// Record that a creature has attacked this turn.
    pub fn mark_creature_attacked_this_turn(&mut self, creature: ObjectId) {
        self.turn_history
            .creatures_attacked_this_turn
            .insert(creature);
    }

    /// Check whether a creature has attacked this turn.
    pub fn creature_attacked_this_turn(&self, creature: ObjectId) -> bool {
        self.turn_history
            .creatures_attacked_this_turn
            .contains(&creature)
    }

    /// Check whether an object performed a specific keyword action this turn.
    pub fn object_performed_keyword_action_this_turn(
        &self,
        object_id: ObjectId,
        action: KeywordActionKind,
    ) -> bool {
        let stable_id = self
            .object(object_id)
            .map(|object| object.stable_id.object_id())
            .unwrap_or(object_id);

        self.turn_history
            .event_records
            .iter()
            .chain(self.turn_history.staged_event_records.iter())
            .filter_map(|record| record.event.downcast::<crate::events::KeywordActionEvent>())
            .any(|event| {
                event.action == action && (event.source == object_id || event.source == stable_id)
            })
    }

    /// Check whether an object was exerted this turn.
    pub fn object_exerted_this_turn(&self, object_id: ObjectId) -> bool {
        self.object_performed_keyword_action_this_turn(object_id, KeywordActionKind::Exert)
    }

    pub fn creature_blocked_this_turn(&self, creature: ObjectId) -> bool {
        self.turn_history.creature_blocked_this_turn(creature)
    }

    /// Record that a specific trigger fired this turn.
    pub fn record_trigger_fired(
        &mut self,
        source_object_id: ObjectId,
        trigger_id: TriggerIdentity,
    ) {
        *self
            .turn_history
            .triggers_fired_this_turn
            .entry((source_object_id, trigger_id))
            .or_insert(0) += 1;
        self.turn_history
            .turn_counters
            .increment_trigger_identity(trigger_id);
    }

    /// Get how many times this trigger fired this turn.
    pub fn trigger_fire_count_this_turn(
        &self,
        source_object_id: ObjectId,
        trigger_id: TriggerIdentity,
    ) -> u32 {
        self.turn_history
            .triggers_fired_this_turn
            .get(&(source_object_id, trigger_id))
            .copied()
            .unwrap_or(0)
    }

    /// Record an event kind occurrence this turn.
    pub fn record_trigger_event_kind(&mut self, event_kind: EventKind) {
        self.turn_history
            .turn_counters
            .increment_event_kind(event_kind);
    }

    /// Get event kind occurrence count this turn.
    pub fn trigger_event_kind_count_this_turn(&self, event_kind: EventKind) -> u32 {
        self.turn_history
            .turn_counters
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
        self.turn_history.turn_counters.increment_named(name);
    }

    /// Get an arbitrary named turn counter value.
    pub fn named_turn_counter(&self, name: &str) -> u32 {
        self.turn_history
            .turn_counters
            .get(&TurnCounterKey::Named(name.to_string()))
    }

    /// Records that an activated ability was used.
    /// Used for OncePerTurn timing restrictions.
    pub fn record_ability_activation(&mut self, source: ObjectId, ability_index: usize) {
        self.turn_history
            .activated_abilities_this_turn
            .insert((source, ability_index));
        self.turn_history
            .turn_counters
            .increment_named(activated_ability_turn_counter_name(source, ability_index));
    }

    /// Check if an activated ability has been used this turn.
    pub fn ability_activated_this_turn(&self, source: ObjectId, ability_index: usize) -> bool {
        self.turn_history
            .activated_abilities_this_turn
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
            &mut self.turn_history.chosen_modes_by_ability_this_turn
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
            &self.turn_history.chosen_modes_by_ability_this_turn
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
            &self.turn_history.chosen_modes_by_ability_this_turn
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

    /// Returns true when the match is using commander designations.
    pub fn is_commander_game(&self) -> bool {
        self.players
            .iter()
            .any(|player| !player.commanders.is_empty())
    }

    /// Returns true if this player's turn-one draw-step draw should be skipped.
    pub fn should_skip_first_turn_draw(&self, player_id: PlayerId) -> bool {
        self.turn.turn_number == 1
            && self.turn.active_player == player_id
            && self.turn_order.first().copied() == Some(player_id)
            && !self.is_commander_game()
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
        self.commander_identity(obj_id).is_some()
    }

    /// Find an object by its stable_id (stable identifier).
    ///
    /// Returns the current ObjectId of the object with the given stable_id,
    /// or None if no such object exists.
    pub fn find_object_by_stable_id(&self, stable_id: StableId) -> Option<ObjectId> {
        let id = *self.stable_id_index.get(&stable_id)?;
        self.objects
            .get(&id)
            .filter(|o| o.stable_id == stable_id)
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
        let mut tagged_players = std::collections::HashMap::new();
        if let Some(source_id) = source
            && let Some(source_obj) = self.object(source_id)
        {
            if let Some(attached_target) = source_obj.attached_to {
                match attached_target {
                    AttachmentTarget::Object(attached_id) => {
                        if let Some(attached_obj) = self.object(attached_id) {
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
                    }
                    AttachmentTarget::Player(attached_player) => {
                        if source_obj.subtypes.contains(&crate::types::Subtype::Aura) {
                            tagged_players.insert(
                                crate::tag::TagKey::from("enchanted"),
                                vec![attached_player],
                            );
                        }
                    }
                }
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
            chosen_player: source.and_then(|source_id| self.chosen_player(source_id)),
            target_players: Vec::new(),
            target_objects: Vec::new(),
            tagged_objects,
            tagged_players,
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
        self.mark_continuous_state_dirty();
        self.tapped_permanents.insert(id);
    }

    /// Untap a permanent.
    pub fn untap(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.tapped_permanents.remove(&id);
    }

    /// Check if a creature has summoning sickness.
    pub fn is_summoning_sick(&self, id: ObjectId) -> bool {
        self.summoning_sick.contains(&id)
    }

    /// Set summoning sickness on a creature.
    pub fn set_summoning_sick(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.summoning_sick.insert(id);
    }

    /// Remove summoning sickness from a creature (e.g., haste).
    pub fn remove_summoning_sickness(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
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

    /// Record that a creature was dealt nonzero damage by a source with deathtouch.
    pub fn mark_deathtouch_damage_since_sba(&mut self, id: ObjectId) {
        self.dealt_deathtouch_damage_since_sba.insert(id);
    }

    /// Returns true if the creature was dealt nonzero damage by a source with
    /// deathtouch since the last time state-based actions were checked.
    pub fn has_deathtouch_damage_since_sba(&self, id: ObjectId) -> bool {
        self.dealt_deathtouch_damage_since_sba.contains(&id)
    }

    /// Clears the transient deathtouch-damage tracker used by SBA evaluation.
    pub fn clear_deathtouch_damage_since_sba(&mut self) {
        self.dealt_deathtouch_damage_since_sba.clear();
    }

    /// Returns true if `creature` was dealt damage by `source` this turn.
    pub fn creature_was_damaged_by_source_this_turn(
        &self,
        creature: ObjectId,
        source: ObjectId,
    ) -> bool {
        self.turn_history
            .creature_was_damaged_by_source_this_turn(creature, source)
    }

    /// Returns true if `creature` was dealt damage by any source this turn.
    pub fn creature_was_damaged_this_turn(&self, creature: ObjectId) -> bool {
        self.turn_history.creature_was_damaged_this_turn(creature)
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
        self.mark_continuous_state_dirty();
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
        self.mark_continuous_state_dirty();
        self.saddled_until_end_of_turn.insert(id);
    }

    /// Check if a permanent is flipped.
    pub fn is_flipped(&self, id: ObjectId) -> bool {
        self.flipped.contains(&id)
    }

    /// Flip a permanent.
    pub fn flip(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.flipped.insert(id);
    }

    /// Check if a permanent is face-down.
    pub fn is_face_down(&self, id: ObjectId) -> bool {
        self.face_down.contains(&id)
    }

    /// Set a permanent as face-down.
    pub fn set_face_down(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.face_down.insert(id);
    }

    /// Mark a face-down permanent as manifested.
    pub fn set_manifested(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.manifested.insert(id);
    }

    /// Check if a permanent is manifested.
    pub fn is_manifested(&self, id: ObjectId) -> bool {
        self.manifested.contains(&id)
    }

    /// Turn a permanent face-up.
    pub fn set_face_up(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.face_down.remove(&id);
        self.manifested.remove(&id);
    }

    /// Return how many times a permanent has transformed since it entered the battlefield.
    pub fn transform_count(&self, id: ObjectId) -> u64 {
        self.transform_count.get(&id).copied().unwrap_or(0)
    }

    /// Record that a permanent transformed and refresh its timestamp per CR 613.7g.
    pub fn mark_transformed(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        let next = self
            .transform_count
            .get(&id)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        self.transform_count.insert(id, next);
        self.continuous_effects.record_entry(id);
    }

    /// Check if a permanent is phased out.
    pub fn is_phased_out(&self, id: ObjectId) -> bool {
        self.phased_out.contains(&id)
    }

    /// Phase out a permanent.
    pub fn phase_out(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.phased_out.insert(id);
    }

    /// Phase in a permanent.
    pub fn phase_in(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
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

    /// Check if a card is exiled via foretell.
    pub fn is_foretold(&self, id: ObjectId) -> bool {
        self.foretold_cards.contains(&id)
    }

    /// Mark a card as exiled via foretell.
    pub fn set_foretold(&mut self, id: ObjectId) {
        self.foretold_cards.insert(id);
    }

    /// Clear foretell exiled status.
    pub fn clear_foretold(&mut self, id: ObjectId) {
        self.foretold_cards.remove(&id);
    }

    /// Check if a card is exiled via plot by the given player.
    pub fn is_plotted_by(&self, id: ObjectId, player: PlayerId) -> bool {
        self.plotted_cards
            .get(&id)
            .is_some_and(|(plotter, _)| *plotter == player)
    }

    /// Return the turn number on which a card was plotted.
    pub fn plotted_turn(&self, id: ObjectId) -> Option<u32> {
        self.plotted_cards.get(&id).map(|(_, turn)| *turn)
    }

    /// Mark a card as plotted by a player on the current turn.
    pub fn set_plotted(&mut self, id: ObjectId, player: PlayerId) {
        self.plotted_cards
            .insert(id, (player, self.turn.turn_number));
    }

    /// Clear plot state for a card.
    pub fn clear_plotted(&mut self, id: ObjectId) {
        self.plotted_cards.remove(&id);
    }

    /// Track that a player has taken the foretell special action this turn.
    pub fn record_foretell_action(&mut self, player: PlayerId) {
        self.turn_history.foretell_actions_this_turn.insert(player);
    }

    /// Check whether the player has already taken the foretell special action this turn.
    pub fn has_foretold_this_turn(&self, player: PlayerId) -> bool {
        self.turn_history
            .foretell_actions_this_turn
            .contains(&player)
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
        self.is_commander(id)
    }

    /// Designate an object as a commander.
    pub fn set_commander(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.commanders.insert(id);
    }

    /// Clear battlefield state for an object (when leaving battlefield).
    pub fn clear_battlefield_state(&mut self, id: ObjectId) {
        self.clear_soulbond_pair(id);
        self.tapped_permanents.remove(&id);
        self.summoning_sick.remove(&id);
        self.damage_marked.remove(&id);
        self.dealt_deathtouch_damage_since_sba.remove(&id);
        self.regeneration_shields.remove(&id);
        self.monstrous.remove(&id);
        self.renowned.remove(&id);
        self.flipped.remove(&id);
        self.face_down.remove(&id);
        self.manifested.remove(&id);
        self.transform_count.remove(&id);
        self.phased_out.remove(&id);
        self.imprinted_cards.remove(&id);
        self.chosen_colors.remove(&id);
        self.chosen_basic_land_types.remove(&id);
        self.chosen_land_types.remove(&id);
        self.chosen_creature_types.remove(&id);
        self.chosen_players.remove(&id);
        self.chosen_named_options.remove(&id);
        self.chosen_modes_by_ability
            .retain(|(source, _), _| *source != id);
        self.turn_history
            .chosen_modes_by_ability_this_turn
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
        if !self.current_is_creature(left) || !self.current_is_creature(right) {
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
        self.foretold_cards.remove(&id);
        self.plotted_cards.remove(&id);
        self.face_down_exile_viewers.remove(&id);
        self.remove_exiled_with_source_link(id);
    }

    /// Allow a player to keep looking at a face-down exiled card.
    pub fn grant_face_down_exile_view(&mut self, id: ObjectId, viewer: PlayerId) {
        self.face_down_exile_viewers
            .entry(id)
            .or_default()
            .insert(viewer);
    }

    /// Check whether a player may inspect a face-down exiled card.
    pub fn can_player_look_at_face_down_exiled_card(&self, id: ObjectId, viewer: PlayerId) -> bool {
        self.face_down_exile_viewers
            .get(&id)
            .is_some_and(|viewers| viewers.contains(&viewer))
    }

    // === Chosen color helpers ===

    /// Record a chosen color for a permanent.
    pub fn set_chosen_color(&mut self, permanent_id: ObjectId, color: crate::color::Color) {
        self.mark_continuous_state_dirty();
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
        self.mark_continuous_state_dirty();
        self.chosen_basic_land_types.insert(permanent_id, subtype);
    }

    /// Get a chosen basic land type for a permanent, if any.
    pub fn chosen_basic_land_type(&self, permanent_id: ObjectId) -> Option<crate::types::Subtype> {
        self.chosen_basic_land_types.get(&permanent_id).copied()
    }

    // === Chosen land type helpers ===

    /// Record a chosen land type for a permanent.
    pub fn set_chosen_land_type(&mut self, permanent_id: ObjectId, subtype: crate::types::Subtype) {
        self.mark_continuous_state_dirty();
        self.chosen_land_types.insert(permanent_id, subtype);
    }

    /// Get a chosen land type for a permanent, if any.
    pub fn chosen_land_type(&self, permanent_id: ObjectId) -> Option<crate::types::Subtype> {
        self.chosen_land_types.get(&permanent_id).copied()
    }

    // === Chosen creature type helpers ===

    /// Record a chosen creature type for a permanent.
    pub fn set_chosen_creature_type(
        &mut self,
        permanent_id: ObjectId,
        subtype: crate::types::Subtype,
    ) {
        self.mark_continuous_state_dirty();
        self.chosen_creature_types.insert(permanent_id, subtype);
    }

    /// Get a chosen creature type for a permanent, if any.
    pub fn chosen_creature_type(&self, permanent_id: ObjectId) -> Option<crate::types::Subtype> {
        self.chosen_creature_types.get(&permanent_id).copied()
    }

    // === Chosen player helpers ===

    /// Record a chosen player for a permanent.
    pub fn set_chosen_player(&mut self, permanent_id: ObjectId, player: PlayerId) {
        self.mark_continuous_state_dirty();
        self.chosen_players.insert(permanent_id, player);
    }

    /// Get a chosen player for a permanent, if any.
    pub fn chosen_player(&self, permanent_id: ObjectId) -> Option<PlayerId> {
        self.chosen_players.get(&permanent_id).copied()
    }

    // === Chosen named option helpers ===

    /// Record a chosen named option for a permanent.
    pub fn set_chosen_named_option(&mut self, permanent_id: ObjectId, option: String) {
        self.mark_continuous_state_dirty();
        self.chosen_named_options.insert(permanent_id, option);
    }

    /// Get a chosen named option for a permanent, if any.
    pub fn chosen_named_option(&self, permanent_id: ObjectId) -> Option<&str> {
        self.chosen_named_options
            .get(&permanent_id)
            .map(String::as_str)
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

    /// Record the component-card identity for a melded permanent.
    pub fn set_melded_permanent(
        &mut self,
        permanent_id: ObjectId,
        components: Vec<MeldComponentState>,
    ) {
        let Some(permanent) = self.object(permanent_id) else {
            return;
        };
        self.melded_permanents
            .insert(permanent.stable_id, MeldedPermanentState { components });
    }

    /// Get meld metadata for a permanent by its stable ID.
    pub fn melded_permanent(&self, stable_id: StableId) -> Option<&MeldedPermanentState> {
        self.melded_permanents.get(&stable_id)
    }

    /// Remove and return meld metadata for a permanent by stable ID.
    pub fn take_melded_permanent(&mut self, stable_id: StableId) -> Option<MeldedPermanentState> {
        self.melded_permanents.remove(&stable_id)
    }

    /// Record the destination objects created by a zone change.
    pub fn record_zone_change_results(&mut self, source_id: ObjectId, result_ids: Vec<ObjectId>) {
        self.zone_change_result_objects
            .insert(source_id, result_ids);
    }

    /// Take the destination objects created by a zone change.
    pub fn take_zone_change_results(&mut self, source_id: ObjectId) -> Vec<ObjectId> {
        self.zone_change_result_objects
            .remove(&source_id)
            .unwrap_or_default()
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
    ///
    /// `parent` is the causal provenance node for this emitted event. If the
    /// event already has a valid provenance, it is preserved.
    fn projected_turn_event_snapshots(
        &self,
        event: &crate::triggers::TriggerEvent,
    ) -> (
        Option<crate::snapshot::ObjectSnapshot>,
        Option<crate::snapshot::ObjectSnapshot>,
    ) {
        let object_snapshot = event
            .downcast::<crate::events::zones::ZoneChangeEvent>()
            .filter(|zone_change| zone_change.to == Zone::Battlefield)
            .and_then(|zone_change| {
                zone_change.objects.first().copied().and_then(|id| {
                    self.object(id)
                        .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, self))
                })
            })
            .or_else(|| event.snapshot().cloned())
            .or_else(|| {
                event.object_id().and_then(|id| {
                    self.object(id)
                        .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, self))
                })
            });
        let source_snapshot = event.inner().source_object().and_then(|id| {
            self.object(id)
                .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, self))
        });
        (object_snapshot, source_snapshot)
    }

    pub(crate) fn stage_turn_history_event(&mut self, event: &crate::triggers::TriggerEvent) {
        let (object_snapshot, source_snapshot) = self.projected_turn_event_snapshots(event);
        self.turn_history
            .stage_event(event, object_snapshot, source_snapshot);
    }

    pub(crate) fn record_turn_history_event(&mut self, event: &crate::triggers::TriggerEvent) {
        let (object_snapshot, source_snapshot) = self.projected_turn_event_snapshots(event);
        self.turn_history
            .record_event(event, object_snapshot, source_snapshot);
    }

    pub fn queue_trigger_event(
        &mut self,
        parent: ProvNodeId,
        mut event: crate::triggers::TriggerEvent,
    ) {
        use crate::events::DamageEvent;
        use crate::events::permanents::SacrificeEvent;
        use crate::events::zones::ZoneChangeEvent;
        use crate::game_event::DamageTarget;

        if let Some(damage) = event.downcast::<DamageEvent>()
            && let DamageTarget::Object(object_id) = damage.target
            && let Some(obj) = self.object(object_id)
            && obj.zone == Zone::Battlefield
        {
            self.record_ui_battlefield_transition(
                UiBattlefieldTransitionKind::Damaged,
                obj.stable_id,
            );
        }

        if let Some(sacrifice) = event.downcast::<SacrificeEvent>() {
            let stable_id = sacrifice
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.stable_id)
                .or_else(|| self.object(sacrifice.permanent).map(|obj| obj.stable_id));
            if let Some(stable_id) = stable_id {
                self.record_ui_battlefield_transition(
                    UiBattlefieldTransitionKind::Sacrificed,
                    stable_id,
                );
            }
        }

        if let Some(zone_change) = event.downcast::<ZoneChangeEvent>()
            && zone_change.from == Zone::Battlefield
            && zone_change.to == Zone::Exile
        {
            let stable_id = zone_change
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.stable_id)
                .or_else(|| {
                    zone_change
                        .objects
                        .first()
                        .and_then(|object_id| self.object(*object_id))
                        .map(|obj| obj.stable_id)
                });
            if let Some(stable_id) = stable_id {
                self.record_ui_battlefield_transition(
                    UiBattlefieldTransitionKind::Exiled,
                    stable_id,
                );
            }
        }

        let initial_provenance = event.provenance();
        if initial_provenance == ProvNodeId::default()
            || self.provenance_graph.node(initial_provenance).is_none()
        {
            let event_provenance = if parent == ProvNodeId::default()
                || self.provenance_graph.node(parent).is_none()
            {
                self.provenance_graph.alloc_root_event(event.kind())
            } else {
                self.alloc_child_event_provenance(parent, event.kind())
            };
            event.set_provenance(event_provenance);
        }

        let queued = self
            .provenance_graph
            .alloc_child(event.provenance(), ProvenanceNodeKind::TriggerQueued);
        event.set_provenance(queued);
        self.turn_history.remove_staged_event(initial_provenance);
        self.stage_turn_history_event(&event);
        self.pending_trigger_events.push(event);
    }

    /// Take all pending trigger events (empties the queue).
    pub fn take_pending_trigger_events(&mut self) -> Vec<crate::triggers::TriggerEvent> {
        std::mem::take(&mut self.pending_trigger_events)
    }

    pub fn record_ui_battlefield_transition(
        &mut self,
        kind: UiBattlefieldTransitionKind,
        stable_id: StableId,
    ) {
        if self
            .ui_battlefield_transitions
            .iter()
            .any(|entry| entry.kind == kind && entry.stable_id == stable_id)
        {
            return;
        }
        self.ui_battlefield_transitions
            .push(UiBattlefieldTransition { stable_id, kind });
    }

    pub fn take_ui_battlefield_transitions(&mut self) -> Vec<UiBattlefieldTransition> {
        std::mem::take(&mut self.ui_battlefield_transitions)
    }

    /// Ensure a replacement-event envelope has provenance.
    pub fn ensure_event_provenance(&mut self, mut event: Event) -> Event {
        let provenance = event.provenance();
        if provenance == ProvNodeId::default() || self.provenance_graph.node(provenance).is_none() {
            let provenance = self.provenance_graph.alloc_root_event(event.kind());
            event.set_provenance(provenance);
        }
        event
    }

    /// Ensure a trigger-event envelope has provenance.
    pub fn ensure_trigger_event_provenance(
        &mut self,
        mut event: crate::triggers::TriggerEvent,
    ) -> crate::triggers::TriggerEvent {
        let provenance = event.provenance();
        if provenance == ProvNodeId::default() || self.provenance_graph.node(provenance).is_none() {
            let provenance = self.provenance_graph.alloc_root_event(event.kind());
            event.set_provenance(provenance);
        }
        event
    }

    /// Allocate a provenance child event under `parent` (or a root when parent is unset/invalid).
    pub fn alloc_child_event_provenance(
        &mut self,
        parent: ProvNodeId,
        kind: EventKind,
    ) -> ProvNodeId {
        self.provenance_graph.alloc_child_event(parent, kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::types::CardType;

    #[test]
    fn shuffle_slice_marks_irreversible_random_usage() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let before = game.irreversible_random_count();
        let mut values = vec![1, 2, 3, 4];

        game.shuffle_slice(&mut values);

        assert_eq!(
            game.irreversible_random_count(),
            before + 1,
            "gameplay shuffles should mark the action chain as irreversible"
        );
    }

    #[test]
    fn creatures_controlled_by_includes_animated_land() {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::cards::definitions::basic_mountain;
        use crate::effect::Effect;
        use crate::effects::EarthbendEffect;
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::ids::CardId;
        use crate::target::ChooseSpec;
        use crate::types::CardType;
        use crate::zone::Zone;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(200), "Kyoshi")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let land_id =
            game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);

        let effect = Effect::new(EarthbendEffect::new(ChooseSpec::SpecificObject(land_id), 8));
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        execute_effect(&mut game, &effect, &mut ctx).expect("earthbend should resolve");

        let creatures = game.creatures_controlled_by(alice);
        assert!(
            creatures.contains(&land_id),
            "animated lands should be counted by creature-control helpers"
        );
    }

    #[test]
    fn current_characteristic_helpers_reflect_animation() {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::cards::definitions::basic_mountain;
        use crate::effect::Effect;
        use crate::effects::EarthbendEffect;
        use crate::executor::{ExecutionContext, execute_effect};
        use crate::ids::CardId;
        use crate::static_abilities::StaticAbilityId;
        use crate::target::ChooseSpec;
        use crate::types::CardType;
        use crate::zone::Zone;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(201), "Kyoshi")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let land_id =
            game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);

        let effect = Effect::new(EarthbendEffect::new(ChooseSpec::SpecificObject(land_id), 8));
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        execute_effect(&mut game, &effect, &mut ctx).expect("earthbend should resolve");

        assert!(game.current_is_creature(land_id));
        assert!(
            game.current_card_types(land_id)
                .is_some_and(|types| types.contains(&CardType::Creature))
        );
        assert_eq!(game.current_power(land_id), Some(8));
        assert_eq!(game.current_toughness(land_id), Some(8));
        assert!(game.current_has_static_ability_id(land_id, StaticAbilityId::Haste));
    }

    #[test]
    fn current_subtypes_reflect_graveyard_effects_and_changeling() {
        use crate::ability::Ability;
        use crate::card::{CardBuilder, PowerToughness};
        use crate::static_abilities::StaticAbility;
        use crate::target::{ObjectFilter, PlayerFilter};
        use crate::types::Subtype;
        use crate::zone::Zone;

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let _beacon_id = game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::from_raw(202), "Graveyard Beacon")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::static_ability(StaticAbility::add_subtypes(
                    ObjectFilter::default()
                        .in_zone(Zone::Graveyard)
                        .owned_by(PlayerFilter::You)
                        .with_type(CardType::Creature),
                    vec![Subtype::Wizard],
                )))
                .build(),
            alice,
            Zone::Battlefield,
        );

        let graveyard_creature_id = game.create_object_from_card(
            &CardBuilder::new(CardId::from_raw(203), "Vanilla Bear")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build(),
            alice,
            Zone::Graveyard,
        );

        assert!(game.current_has_subtype(graveyard_creature_id, Subtype::Wizard));
        assert!(
            game.current_subtypes(graveyard_creature_id)
                .is_some_and(|subtypes| subtypes.contains(&Subtype::Wizard))
        );

        let changeling_spell_id = game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::from_raw(204), "Velis Probe")
                .card_types(vec![CardType::Kindred, CardType::Instant])
                .with_ability(Ability::static_ability(StaticAbility::changeling()))
                .build(),
            alice,
            Zone::Graveyard,
        );

        assert!(game.current_has_subtype(changeling_spell_id, Subtype::Wizard));
        assert!(game.current_has_subtype(changeling_spell_id, Subtype::Elf));
        assert!(
            game.current_subtypes(changeling_spell_id)
                .is_some_and(|subtypes| subtypes.contains(&Subtype::Wizard))
        );
    }

    #[test]
    fn azusa_after_first_land_grants_two_remaining_land_plays() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let azusa = CardDefinitionBuilder::new(CardId::new(), "Azusa, Lost but Seeking")
            .card_types(vec![CardType::Creature])
            .parse_text("You may play two additional lands on each of your turns.")
            .expect("Azusa text should parse");

        game.player_mut(alice)
            .expect("alice should exist")
            .record_land_play();
        assert!(
            !game
                .player(alice)
                .expect("alice should exist")
                .can_play_land(),
            "a player who already played a land should be out of normal land plays"
        );

        game.create_object_from_definition(&azusa, alice, Zone::Battlefield);
        game.refresh_continuous_state();

        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .land_plays_per_turn,
            3,
            "Azusa should raise the land-play limit to three total for the turn"
        );
        assert!(
            game.player(alice)
                .expect("alice should exist")
                .can_play_land(),
            "after Azusa enters, the player should still have two land plays remaining"
        );

        game.player_mut(alice)
            .expect("alice should exist")
            .record_land_play();
        assert!(
            game.player(alice)
                .expect("alice should exist")
                .can_play_land(),
            "the second land play after Azusa should still leave one more available"
        );

        game.player_mut(alice)
            .expect("alice should exist")
            .record_land_play();
        assert!(
            !game
                .player(alice)
                .expect("alice should exist")
                .can_play_land(),
            "the third total land play should exhaust Azusa's extra allowance"
        );
    }

    #[test]
    fn filtered_activation_mana_spend_permissions_match_allowed_sources() {
        use crate::card::CardBuilder;
        use crate::effect::ManaSpendPermission;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature_card = CardBuilder::new(CardId::from_raw(300), "Test Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let artifact_card = CardBuilder::new(CardId::from_raw(301), "Test Artifact")
            .card_types(vec![CardType::Artifact])
            .build();

        let alice_creature = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
        let bob_creature = game.create_object_from_card(&creature_card, bob, Zone::Battlefield);
        let alice_artifact = game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);

        game.mana_spend_effects
            .permissions
            .push(ActiveManaSpendPermission {
                permission: ManaSpendPermission::any_color_for_activation(
                    crate::target::PlayerFilter::You,
                    crate::target::ObjectFilter::creature().you_control(),
                ),
                controller: alice,
                source: ManaSpendPermissionSource::StaticAbility,
            });

        assert!(game.can_spend_mana_as_any_color(alice, Some(alice_creature)));
        assert!(!game.can_spend_mana_as_any_color(alice, Some(alice_artifact)));
        assert!(!game.can_spend_mana_as_any_color(alice, Some(bob_creature)));
        assert!(!game.can_spend_mana_as_any_color(bob, Some(bob_creature)));
    }
}
