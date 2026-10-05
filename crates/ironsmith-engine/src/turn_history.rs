use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::color::ColorSet;
use crate::events::EnterBattlefieldEvent;
use crate::events::combat::{CreatureAttackedEvent, CreatureBlockedEvent};
use crate::events::other::CounterPlacedEvent;
use crate::events::other::{
    CardDiscardedEvent, CardsDrawnEvent, ControlChangedEvent, KeywordActionEvent,
    KeywordActionKind, SearchLibraryEvent,
};
use crate::events::permanents::SacrificeEvent;
use crate::events::spells::SpellCastEvent;
use crate::events::tokens::CreateTokensEvent;
use crate::events::zones::ZoneChangeEvent;
use crate::events::{DamageEvent, DamageTarget, EventKind, LifeGainEvent, LifeLossEvent};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::game_state::GameState;
use crate::game_state::TurnCounterTracker;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::provenance::{ProvNodeId, ProvenanceGraph};
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::StaticAbilityInstanceId;
use crate::triggers::TriggerEvent;
use crate::triggers::TriggerIdentity;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use ironsmith_core::TurnHistoryCount;

/// One ingested trigger/event observation for the current turn.
#[derive(Debug, Clone)]
pub struct TurnEventRecord {
    pub event: TriggerEvent,
    pub object_snapshot: Option<ObjectSnapshot>,
    pub source_snapshot: Option<ObjectSnapshot>,
}

impl TurnEventRecord {
    /// Whether this event record contains rules-relevant information about a
    /// player's action or its result.
    ///
    /// The event envelope identifies direct actors/affected players, while the
    /// captured object/source snapshots retain controller and owner identity
    /// after the object itself changes zones or leaves the game.
    pub fn involves_player(&self, player: PlayerId) -> bool {
        self.event.player() == Some(player)
            || self.event.controller() == Some(player)
            || self
                .object_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.controller == player || snapshot.owner == player)
            || self
                .source_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.controller == player || snapshot.owner == player)
    }
}

/// Ordered immutable observations shared by speculative game branches. An
/// append/retain/truncate changes only this collection; recorded snapshots are
/// never edited through the collection's API.
#[derive(Clone, Default)]
pub struct TurnEventRecords(im::Vector<Arc<TurnEventRecord>>);

impl std::fmt::Debug for TurnEventRecords {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

impl TurnEventRecords {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &TurnEventRecord> + ExactSizeIterator {
        self.0.iter().map(Arc::as_ref)
    }
    pub fn last(&self) -> Option<&TurnEventRecord> {
        self.0.back().map(Arc::as_ref)
    }
    pub(crate) fn last_shared(&self) -> Option<Arc<TurnEventRecord>> {
        self.0.back().cloned()
    }
    pub fn push(&mut self, record: TurnEventRecord) {
        self.0.push_back(Arc::new(record));
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&TurnEventRecord) -> bool) {
        self.0.retain(|record| keep(record));
    }
}

impl std::ops::Index<usize> for TurnEventRecords {
    type Output = TurnEventRecord;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

/// Unified owner for turn-scoped bookkeeping and history.
#[derive(Debug, Clone, Default)]
pub struct TurnHistory {
    /// Exact public holder when this turn began. Fresh games have no monarch;
    /// authoritative restore requires a presence-bearing carrier for this fact.
    pub monarch_at_turn_start: Option<PlayerId>,
    /// Per-player snapshot taken before the untap step begins.
    pub untapped_lands_at_turn_start: HashMap<PlayerId, u32>,
    pub activated_abilities_this_turn: HashSet<(ObjectId, usize)>,
    pub loyalty_abilities_activated_this_turn: HashSet<ObjectId>,
    pub activated_abilities_resolved_this_turn: HashMap<(ObjectId, usize), u32>,
    pub chosen_modes_by_ability_this_turn: HashMap<(ObjectId, usize), HashSet<usize>>,
    pub triggers_fired_this_turn: HashMap<(ObjectId, TriggerIdentity), u32>,
    pub triggered_abilities_resolved_this_turn: HashMap<(ObjectId, TriggerIdentity), u32>,
    /// Times each triggered ability's "Do this only once each turn" optional
    /// instruction was actually performed this turn.
    pub do_this_actions_this_turn: HashMap<(ObjectId, TriggerIdentity), u32>,
    pub turn_counters: TurnCounterTracker,
    pub foretell_actions_this_turn: HashSet<PlayerId>,
    pub mana_spent_to_cast_spells_this_turn: HashMap<PlayerId, u32>,
    pub players_attacked_this_turn: HashSet<PlayerId>,
    pub players_tapped_land_for_mana_this_turn: HashSet<PlayerId>,
    /// Player/counter pairs locked by a replacement effect for the rest of
    /// this turn (for example, "you can't get additional poison counters this
    /// turn"). The lock is established when that replacement applies, rather
    /// than retroactively counting counters received earlier in the turn.
    pub player_counter_locks_this_turn: HashSet<(PlayerId, crate::object::CounterType)>,
    pub die_rolls_this_turn: HashMap<PlayerId, Vec<u32>>,
    /// Completed physical rolls, including nonnumeric planar rolls. Ignored
    /// and superseded rerolls never enter this ordinal history.
    pub completed_die_rolls_this_turn: HashMap<PlayerId, u32>,
    pub die_roll_result_adjustments_this_turn: HashSet<(ObjectId, StaticAbilityInstanceId)>,
    /// Source/player pairs for attached-object rule restrictions that player
    /// has paid to ignore until the turn ends.
    pub players_ignoring_attached_static_restrictions_this_turn: HashSet<(ObjectId, PlayerId)>,
    /// Source/player pairs for source-wide rule restrictions that the player
    /// has paid to ignore until the turn ends.
    pub players_ignoring_source_static_effects_this_turn: HashSet<(ObjectId, PlayerId)>,
    pub creatures_attacked_this_turn: HashSet<ObjectId>,
    /// Creatures each player declared as attackers this turn, keyed by the
    /// attacking player at declaration. "Attacked with" is a turn-history fact
    /// that survives the creature leaving the battlefield or changing control.
    pub creatures_attacked_by_player_this_turn: HashMap<PlayerId, HashSet<ObjectId>>,
    /// Players each player attacked with a declared attacker, keyed by
    /// (combat phase number this turn, attacking player). Creatures put onto
    /// the battlefield attacking never "attacked" (CR 508.4), and removal from
    /// combat doesn't undo the attack (CR 702.121a melee counts these).
    pub players_attacked_in_combat: HashMap<(u32, PlayerId), HashSet<PlayerId>>,
    pub creatures_attacked_battles_this_turn: HashSet<ObjectId>,
    pub creature_attack_counts_this_turn: HashMap<ObjectId, u32>,
    pub crewed_this_turn: HashMap<ObjectId, Vec<ObjectId>>,
    /// Vehicles a crew ability of which resolved this turn (CR 702.122d:
    /// that is what "becomes crewed" means, so "for the first time each
    /// turn" is decided here, not when a crew cost is paid).
    pub crew_abilities_resolved_this_turn: HashSet<ObjectId>,
    pub saddled_this_turn: HashMap<ObjectId, Vec<ObjectId>>,
    pub spell_warped_this_turn: bool,
    /// Spells each player has cast this game (never cleared between turns).
    pub spells_cast_this_game: HashMap<PlayerId, u32>,
    /// Completed unqualified target history restored without fabricating old
    /// event records. Exact incarnations remain distinct after a zone change.
    checkpoint_targeted_objects: HashSet<ObjectId>,
    pub event_records: TurnEventRecords,
    pub staged_event_records: TurnEventRecords,
    /// Index into `event_records` where the simultaneous action whose events
    /// are being matched began (CR 603.2c: e.g. all combat damage of one
    /// step, CR 510.2). Records from that index on are the same event, not
    /// earlier ones, for "for the first time each turn". Transient: set and
    /// restored around one batch's trigger matching.
    pub simultaneous_batch_start: Option<usize>,
}

impl TurnHistory {
    pub fn clear_for_new_turn(&mut self) -> u32 {
        let spells_cast_last_turn_total = self.total_spells_cast_this_turn();

        self.monarch_at_turn_start = None;
        self.activated_abilities_this_turn.clear();
        self.loyalty_abilities_activated_this_turn.clear();
        self.activated_abilities_resolved_this_turn.clear();
        self.chosen_modes_by_ability_this_turn.clear();
        self.triggers_fired_this_turn.clear();
        self.triggered_abilities_resolved_this_turn.clear();
        self.do_this_actions_this_turn.clear();
        self.turn_counters.clear();
        self.foretell_actions_this_turn.clear();
        self.mana_spent_to_cast_spells_this_turn.clear();
        self.players_attacked_this_turn.clear();
        self.players_tapped_land_for_mana_this_turn.clear();
        self.untapped_lands_at_turn_start.clear();
        self.player_counter_locks_this_turn.clear();
        self.die_rolls_this_turn.clear();
        self.completed_die_rolls_this_turn.clear();
        self.die_roll_result_adjustments_this_turn.clear();
        self.players_ignoring_attached_static_restrictions_this_turn
            .clear();
        self.players_ignoring_source_static_effects_this_turn
            .clear();
        self.creatures_attacked_this_turn.clear();
        self.creatures_attacked_by_player_this_turn.clear();
        self.players_attacked_in_combat.clear();
        self.creatures_attacked_battles_this_turn.clear();
        self.creature_attack_counts_this_turn.clear();
        self.crewed_this_turn.clear();
        self.crew_abilities_resolved_this_turn.clear();
        self.saddled_this_turn.clear();
        self.spell_warped_this_turn = false;
        self.checkpoint_targeted_objects.clear();
        self.event_records.clear();
        self.staged_event_records.clear();
        self.simultaneous_batch_start = None;

        spells_cast_last_turn_total
    }

    /// Mark the start of one simultaneous action's events; returns the
    /// previous mark for [`Self::end_simultaneous_batch`].
    pub(crate) fn begin_simultaneous_batch(&mut self) -> Option<usize> {
        self.simultaneous_batch_start
            .replace(self.event_records.len())
    }

    pub(crate) fn end_simultaneous_batch(&mut self, previous: Option<usize>) {
        self.simultaneous_batch_start = previous;
    }

    /// Plain completed first-target facts. Provisional copy targets are not
    /// included: publication must finish before a continuation-free checkpoint.
    pub fn targeted_object_history_for_checkpoint(&self) -> Vec<ObjectId> {
        let mut ids = self.checkpoint_targeted_objects.clone();
        ids.extend(self.event_records.iter().filter_map(|record| {
            record
                .event
                .downcast::<crate::events::BecomesTargetedEvent>()
                .and_then(|targeted| targeted.target_object())
        }));
        let mut ids: Vec<_> = ids.into_iter().collect();
        ids.sort();
        ids
    }

    pub fn restore_targeted_object_history(&mut self, ids: Vec<ObjectId>) -> Result<(), String> {
        let unique: HashSet<_> = ids.iter().copied().collect();
        if unique.len() != ids.len() {
            return Err("duplicate exact object in completed target history".into());
        }
        self.checkpoint_targeted_objects = unique;
        Ok(())
    }

    pub(crate) fn object_was_targeted_before_checkpoint(&self, id: ObjectId) -> bool {
        self.checkpoint_targeted_objects.contains(&id)
    }

    pub(crate) fn projected_records(&self) -> impl DoubleEndedIterator<Item = &TurnEventRecord> {
        self.event_records
            .iter()
            .chain(self.staged_event_records.iter())
    }

    /// The power of creatures this player actually declared as attackers in
    /// one combat, using the immutable event snapshots. Later power changes,
    /// control changes, leaving the battlefield, or token disappearance do
    /// not change that fact. Put-onto-the-battlefield-attacking creatures
    /// produce no declaration events and therefore do not contribute.
    pub fn declared_attack_power_in_combat(&self, combat_phase: u32, player: PlayerId) -> i64 {
        let mut seen = HashSet::new();
        self.projected_records()
            .filter_map(|record| {
                let attack = record.event.downcast::<CreatureAttackedEvent>()?;
                let snapshot = record.object_snapshot.as_ref()?;
                (attack.combat_phase == Some(combat_phase)
                    && snapshot.controller == player
                    && seen.insert(attack.attacker))
                .then_some(i64::from(snapshot.power.unwrap_or(0)))
            })
            .sum()
    }

    pub fn remove_staged_event(&mut self, provenance: ProvNodeId) {
        if provenance == ProvNodeId::default() {
            return;
        }
        self.staged_event_records
            .retain(|record| record.event.provenance() != provenance);
    }

    pub fn stage_event(
        &mut self,
        event: &TriggerEvent,
        object_snapshot: Option<ObjectSnapshot>,
        source_snapshot: Option<ObjectSnapshot>,
    ) {
        // Republishing an observation already committed to history must not
        // create a second staged copy of the same physical event.
        if self
            .event_records
            .iter()
            .any(|record| record.event.occurrence_key() == event.occurrence_key())
        {
            return;
        }
        self.staged_event_records
            .retain(|record| record.event.occurrence_key() != event.occurrence_key());
        self.remove_staged_event(event.provenance());
        self.staged_event_records.push(TurnEventRecord {
            event: event.clone(),
            object_snapshot,
            source_snapshot,
        });
    }

    pub fn record_event(
        &mut self,
        event: &TriggerEvent,
        object_snapshot: Option<ObjectSnapshot>,
        source_snapshot: Option<ObjectSnapshot>,
    ) {
        self.staged_event_records
            .retain(|record| record.event.occurrence_key() != event.occurrence_key());
        self.remove_staged_event(event.provenance());
        if self
            .event_records
            .iter()
            .any(|record| record.event.occurrence_key() == event.occurrence_key())
        {
            return;
        }
        self.turn_counters.increment_event_kind(event.kind());
        if let Some(cast) = event.downcast::<SpellCastEvent>() {
            *self.spells_cast_this_game.entry(cast.caster).or_insert(0) += 1;
        }
        self.event_records.push(TurnEventRecord {
            event: event.clone(),
            object_snapshot,
            source_snapshot,
        });
    }

    pub fn event_kind_count(&self, kind: EventKind) -> u32 {
        self.turn_counters
            .get(&crate::game_state::TurnCounterKey::EventKind(kind))
            .saturating_add(
                self.staged_event_records
                    .iter()
                    .filter(|record| record.event.kind() == kind)
                    .count() as u32,
            )
    }

    pub fn player_counter_is_locked_this_turn(
        &self,
        player: PlayerId,
        counter_type: crate::object::CounterType,
    ) -> bool {
        self.player_counter_locks_this_turn
            .contains(&(player, counter_type))
    }

    pub fn lock_player_counter_for_turn(
        &mut self,
        player: PlayerId,
        counter_type: crate::object::CounterType,
    ) {
        self.player_counter_locks_this_turn
            .insert((player, counter_type));
    }

    pub fn total_spells_cast_this_turn(&self) -> u32 {
        self.projected_records()
            .filter(|record| record.event.downcast::<SpellCastEvent>().is_some())
            .count() as u32
    }

    /// Pre-change snapshots of every object that left the battlefield this
    /// turn and went to `to` (any zone when `None`). A simultaneous batch
    /// record ("destroy all", devour) contributes each of its objects
    /// (CR 700.4, 603.6c), not only its first one.
    fn battlefield_departure_snapshots(
        &self,
        to: Option<Zone>,
    ) -> impl Iterator<Item = &crate::snapshot::ObjectSnapshot> + '_ {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(move |event| {
                event.from == Zone::Battlefield && to.is_none_or(|zone| event.to == zone)
            })
            .flat_map(|event| event.snapshots().iter())
    }

    pub fn total_creatures_died_this_turn(&self) -> u32 {
        self.battlefield_departure_snapshots(Some(Zone::Graveyard))
            .filter(|snapshot| snapshot.card_types.contains(&CardType::Creature))
            .count() as u32
    }

    pub fn creatures_died_under_controller(&self, player: PlayerId) -> u32 {
        self.battlefield_departure_snapshots(Some(Zone::Graveyard))
            .filter(|snapshot| {
                snapshot.controller == player && snapshot.card_types.contains(&CardType::Creature)
            })
            .count() as u32
    }

    pub fn cards_drawn_by_player(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<CardsDrawnEvent>())
            .filter(|event| event.player == player)
            .map(CardsDrawnEvent::amount)
            .sum()
    }

    pub fn cards_discarded_by_player(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<CardDiscardedEvent>())
            .filter(|event| event.player == player)
            .count() as u32
    }

    pub fn total_cards_discarded_for_players(&self, players: &[PlayerId]) -> u32 {
        players
            .iter()
            .map(|player| self.cards_discarded_by_player(*player))
            .sum()
    }

    pub fn total_attractions_visited_for_players(&self, players: &[PlayerId]) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<KeywordActionEvent>())
            .filter(|event| {
                event.action == KeywordActionKind::VisitAttraction
                    && players.contains(&event.player)
            })
            .map(|event| event.amount)
            .sum()
    }

    /// The latest actual draw, not the latest attempted draw. Empty/replaced
    /// draws contribute no identity. Preserve this historical identity even if
    /// its current object later leaves Hand; consumers must not choose an older
    /// draw as a substitute.
    pub fn last_card_drawn_by_player(&self, player: PlayerId) -> Option<ObjectId> {
        self.projected_records()
            .rev()
            .filter_map(|record| record.event.downcast::<CardsDrawnEvent>())
            .filter(|event| event.player == player)
            .find_map(|event| event.cards.last().copied())
    }

    pub fn object_was_drawn_this_turn(&self, object: ObjectId) -> bool {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<CardsDrawnEvent>())
            .any(|event| event.cards.contains(&object))
    }

    pub fn max_cards_drawn_for_players(&self, players: &[PlayerId]) -> u32 {
        players
            .iter()
            .map(|player| self.cards_drawn_by_player(*player))
            .max()
            .unwrap_or(0)
    }

    pub fn max_die_rolls_for_players(&self, players: &[PlayerId]) -> u32 {
        players
            .iter()
            .map(|player| self.completed_die_roll_count(*player))
            .max()
            .unwrap_or(0)
    }

    /// Spells `player` has cast this game, counting only completed casts.
    pub fn spells_cast_by_player_this_game(&self, player: PlayerId) -> u32 {
        self.spells_cast_this_game
            .get(&player)
            .copied()
            .unwrap_or(0)
    }

    pub fn spells_cast_by_player(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<SpellCastEvent>())
            .filter(|event| event.caster == player)
            .count() as u32
    }

    pub fn total_spells_cast_for_players(&self, players: &[PlayerId]) -> u32 {
        players
            .iter()
            .map(|player| self.spells_cast_by_player(*player))
            .sum()
    }

    pub fn any_spell_was_cast_this_turn(&self) -> bool {
        self.projected_records()
            .any(|record| record.event.downcast::<SpellCastEvent>().is_some())
    }

    /// The zone the object with this stable identity was most recently cast
    /// from this turn, if it was cast this turn.
    pub fn latest_cast_zone(&self, stable_id: StableId) -> Option<Zone> {
        self.projected_records()
            .filter_map(|record| {
                let cast = record.event.downcast::<SpellCastEvent>()?;
                cast.snapshot
                    .as_ref()
                    .or(record.object_snapshot.as_ref())
                    .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                    .then_some(cast.from_zone)
            })
            .last()
    }

    /// Whether the object with this stable identity was cast from `zone` this turn.
    pub fn object_was_cast_from_zone(&self, stable_id: StableId, zone: Zone) -> bool {
        self.object_was_cast_from_zone_by(stable_id, zone, None)
    }

    /// Whether the object with this stable identity was cast from `zone` this
    /// turn by `caster` (any caster when `None`).
    pub fn object_was_cast_from_zone_by(
        &self,
        stable_id: StableId,
        zone: Zone,
        caster: Option<PlayerId>,
    ) -> bool {
        self.projected_records().any(|record| {
            let Some(cast) = record.event.downcast::<SpellCastEvent>() else {
                return false;
            };
            cast.from_zone == zone
                && caster.is_none_or(|caster| cast.caster == caster)
                && cast
                    .snapshot
                    .as_ref()
                    .or(record.object_snapshot.as_ref())
                    .is_some_and(|snapshot| snapshot.stable_id == stable_id)
        })
    }

    pub fn total_life_gained_for_players(&self, players: &[PlayerId]) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<LifeGainEvent>())
            .filter(|event| players.contains(&event.player))
            .map(|event| event.amount)
            .sum()
    }

    pub fn total_life_lost_for_players(&self, players: &[PlayerId]) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<LifeLossEvent>())
            .filter(|event| players.contains(&event.player))
            .map(|event| event.amount)
            .sum()
    }

    pub fn total_noncombat_damage_to_players(&self, players: &[PlayerId]) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<DamageEvent>())
            .filter(|event| !event.is_combat)
            .filter_map(|event| match event.target {
                crate::events::DamageTarget::Player(player) if players.contains(&player) => {
                    Some(event.amount)
                }
                _ => None,
            })
            .sum()
    }

    pub fn total_noncombat_damage_dealt_by_sources_controlled_by(
        &self,
        players: &[PlayerId],
        colors: Option<ColorSet>,
    ) -> u32 {
        self.projected_records()
            .filter_map(|record| {
                let damage = record.event.downcast::<DamageEvent>()?;
                if damage.is_combat {
                    return None;
                }
                let source = record.source_snapshot.as_ref()?;
                if !players.contains(&source.controller) {
                    return None;
                }
                if let Some(colors) = colors
                    && source.colors.intersection(colors).is_empty()
                {
                    return None;
                }
                Some(damage.amount)
            })
            .sum()
    }

    pub fn total_damage_to_player(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<DamageEvent>())
            .filter_map(|event| match event.target {
                crate::events::DamageTarget::Player(pid) if pid == player => Some(event.amount),
                _ => None,
            })
            .sum()
    }

    pub fn total_damage_to_players(&self, players: &[PlayerId]) -> u32 {
        players
            .iter()
            .map(|player| self.total_damage_to_player(*player))
            .sum()
    }

    pub fn total_creature_damage_to_player(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter_map(|record| {
                let damage = record.event.downcast::<DamageEvent>()?;
                match damage.target {
                    crate::events::DamageTarget::Player(pid) if pid == player => {
                        let source_is_creature =
                            record.source_snapshot.as_ref().is_some_and(|snapshot| {
                                snapshot.card_types.contains(&CardType::Creature)
                            });
                        source_is_creature.then_some(damage.amount)
                    }
                    _ => None,
                }
            })
            .sum()
    }

    pub fn player_was_dealt_damage_this_turn(&self, player: PlayerId) -> bool {
        self.total_damage_to_player(player) > 0
    }

    pub fn player_was_dealt_combat_damage_by_creature_subtype_this_turn(
        &self,
        players: &[PlayerId],
        subtype: Subtype,
    ) -> bool {
        self.projected_records().any(|record| {
            let Some(event) = record.event.downcast::<DamageEvent>() else {
                return false;
            };
            if !event.is_combat || event.amount == 0 {
                return false;
            }
            match event.target {
                crate::events::DamageTarget::Player(player) if players.contains(&player) => {}
                _ => return false,
            }

            record
                .source_snapshot
                .as_ref()
                .or(record.object_snapshot.as_ref())
                .is_some_and(|snapshot| {
                    snapshot.card_types.contains(&CardType::Creature)
                        && snapshot.subtypes.contains(&subtype)
                })
        })
    }

    pub fn player_lost_life_this_turn(&self, player: PlayerId) -> bool {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<LifeLossEvent>())
            .any(|event| event.player == player && event.amount > 0)
    }

    pub fn creatures_entered_under_controller(&self, player: PlayerId) -> u32 {
        let mut entered = std::collections::HashSet::new();
        for record in self.projected_records() {
            if let Some(event) = record.event.downcast::<EnterBattlefieldEvent>() {
                if record.object_snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.controller == player
                        && snapshot.card_types.contains(&CardType::Creature)
                }) {
                    entered.insert(event.object);
                }
            } else if let Some(event) = record.event.downcast::<ZoneChangeEvent>()
                && event.is_etb()
            {
                let snapshots = if event.destination_snapshots.is_empty() {
                    event.snapshots()
                } else {
                    &event.destination_snapshots
                };
                for (index, snapshot) in snapshots.iter().enumerate() {
                    if snapshot.controller == player
                        && snapshot.card_types.contains(&CardType::Creature)
                    {
                        entered.insert(
                            event
                                .result_objects
                                .get(index)
                                .copied()
                                .unwrap_or(snapshot.object_id),
                        );
                    }
                }
                if snapshots.is_empty()
                    && let Some(snapshot) = record.object_snapshot.as_ref()
                    && snapshot.controller == player
                    && snapshot.card_types.contains(&CardType::Creature)
                {
                    entered.insert(
                        event
                            .result_objects
                            .first()
                            .copied()
                            .unwrap_or(snapshot.object_id),
                    );
                }
            }
        }
        entered
            .len()
            .try_into()
            .expect("creature entry count fits its public carrier")
    }

    pub fn player_had_creature_enter_battlefield_this_turn(&self, player: PlayerId) -> bool {
        self.creatures_entered_under_controller(player) > 0
    }

    pub fn player_had_land_enter_battlefield_this_turn(&self, player: PlayerId) -> bool {
        self.lands_entered_under_controller(player) > 0
    }

    pub fn lands_entered_under_controller(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter(|record| {
                (record.event.downcast::<EnterBattlefieldEvent>().is_some()
                    || record
                        .event
                        .downcast::<ZoneChangeEvent>()
                        .is_some_and(|event| event.is_etb()))
                    && record.object_snapshot.as_ref().is_some_and(|snapshot| {
                        snapshot.controller == player
                            && snapshot.card_types.contains(&CardType::Land)
                    })
            })
            .count() as u32
    }

    pub fn total_lands_entered_for_players(&self, players: &[PlayerId]) -> u32 {
        players
            .iter()
            .map(|player| self.lands_entered_under_controller(*player))
            .sum()
    }

    pub fn object_entered_battlefield_controller_this_turn(
        &self,
        stable_id: StableId,
    ) -> Option<PlayerId> {
        self.projected_records().rev().find_map(|record| {
            let is_entry = record.event.downcast::<EnterBattlefieldEvent>().is_some()
                || record
                    .event
                    .downcast::<ZoneChangeEvent>()
                    .is_some_and(|event| event.is_etb());
            is_entry.then_some(())?;
            record
                .object_snapshot
                .as_ref()
                .filter(|snapshot| snapshot.stable_id == stable_id)
                .map(|snapshot| snapshot.controller)
        })
    }

    pub fn entered_battlefield_snapshots_this_turn(&self) -> Vec<ObjectSnapshot> {
        self.projected_records()
            .filter_map(|record| {
                let is_entry = record.event.downcast::<EnterBattlefieldEvent>().is_some()
                    || record
                        .event
                        .downcast::<ZoneChangeEvent>()
                        .is_some_and(|event| event.is_etb());
                is_entry.then(|| record.object_snapshot.clone()).flatten()
            })
            .collect()
    }

    pub fn object_came_under_controller_this_turn(
        &self,
        stable_id: StableId,
        player: PlayerId,
    ) -> bool {
        if self
            .object_entered_battlefield_controller_this_turn(stable_id)
            .is_some_and(|controller| controller == player)
        {
            return true;
        }

        self.projected_records().rev().any(|record| {
            record
                .event
                .downcast::<ControlChangedEvent>()
                .is_some_and(|event| event.new_controller == player)
                && record
                    .object_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.stable_id == stable_id)
        })
    }

    pub fn object_was_put_into_graveyard_this_turn(&self, stable_id: StableId) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<ZoneChangeEvent>()
                .is_some_and(|event| {
                    event.to == Zone::Graveyard
                        && record
                            .object_snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                })
        })
    }

    /// Whether this exact graveyard incarnation arrived this turn, optionally
    /// from a specific zone. Live-card filters must use destination object IDs:
    /// under CR 400.7, leaving and reentering a graveyard creates a new object
    /// that cannot inherit an earlier incarnation's battlefield/library origin.
    /// Stable-ID history queries above intentionally retain historical facts.
    pub fn graveyard_incarnation_entered_this_turn(
        &self,
        object_id: ObjectId,
        from: Option<Zone>,
    ) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<ZoneChangeEvent>()
                .is_some_and(|event| {
                    event.to == Zone::Graveyard
                        && from.is_none_or(|from| event.from == from)
                        && event.destination_objects().contains(&object_id)
                })
        })
    }

    /// Mill is a keyword action, not every library-to-graveyard movement.
    /// Destination IDs reject a later incarnation that returned to the graveyard.
    pub fn graveyard_incarnation_was_milled_this_turn(&self, object_id: ObjectId) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<crate::events::other::CardMilledEvent>()
                .is_some_and(|event| {
                    event.card == object_id
                        && event
                            .snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.zone == Zone::Graveyard)
                })
        })
    }

    /// Counts the number of times a player descended this turn.
    ///
    /// Descend looks at the card's last known characteristics and owner when it
    /// moved, so the result remains true even if the card later leaves the
    /// graveyard. Tokens do not count because they are permanents, not
    /// permanent cards.
    pub fn player_descended_count_this_turn(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.to == Zone::Graveyard)
            .flat_map(|event| event.snapshots.iter())
            .filter(|snapshot| snapshot.owner == player && snapshot_is_permanent_card(snapshot))
            .count() as u32
    }

    /// Cards (not tokens) put into `player`'s graveyard from anywhere this
    /// turn, counted when they moved even if they have since left.
    pub fn cards_put_into_graveyard_count_this_turn(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.to == Zone::Graveyard)
            .flat_map(|event| event.snapshots.iter())
            .filter(|snapshot| snapshot.owner == player && !snapshot.is_token)
            .count() as u32
    }

    pub fn object_was_put_into_graveyard_from_battlefield_this_turn(
        &self,
        stable_id: StableId,
    ) -> bool {
        self.object_was_put_into_graveyard_from_zone_this_turn(stable_id, Zone::Battlefield)
    }

    pub fn object_was_put_into_graveyard_from_zone_this_turn(
        &self,
        stable_id: StableId,
        from: Zone,
    ) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<ZoneChangeEvent>()
                .is_some_and(|event| {
                    event.from == from
                        && event.to == Zone::Graveyard
                        && record
                            .object_snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                })
        })
    }

    /// Last known information of `object` as it left its zone this turn.
    ///
    /// Object ids are not reused across zone changes, so the latest zone
    /// change that moved `object` holds the snapshot taken right before it
    /// moved (CR 608.2h). Used for an ability that triggered while its source
    /// was still there and is put on the stack after the source left.
    pub fn departed_object_snapshot(&self, object: ObjectId) -> Option<&ObjectSnapshot> {
        self.projected_records().rev().find_map(|record| {
            let event = record.event.downcast::<ZoneChangeEvent>()?;
            if !event.objects.contains(&object) {
                return None;
            }
            event
                .snapshots()
                .iter()
                .find(|snapshot| snapshot.object_id == object)
        })
    }

    /// A true departure receipt for an exact object incarnation. Phasing does
    /// not make a new object or freeze later noncopiable choices on that object.
    pub fn source_departure_snapshot(&self, object: ObjectId) -> Option<&ObjectSnapshot> {
        self.projected_records().rev().find_map(|record| {
            if let Some(event) = record
                .event
                .downcast::<crate::events::zones::ObjectLeavesGameEvent>()
            {
                return (event.object == object).then_some(&event.snapshot);
            }
            let event = record.event.downcast::<ZoneChangeEvent>()?;
            event
                .snapshots()
                .iter()
                .find(|snapshot| snapshot.object_id == object)
        })
    }

    /// Characteristics immediately before this source left its zone, left the
    /// game, or phased out this turn. Timed programs can outlive these transitions.
    pub fn source_last_known_snapshot(&self, object: ObjectId) -> Option<&ObjectSnapshot> {
        self.projected_records().rev().find_map(|record| {
            if let Some(event) = record
                .event
                .downcast::<crate::events::PermanentPhasedOutEvent>()
            {
                return (event.permanent == object)
                    .then_some(event.snapshot.as_ref())
                    .flatten();
            }
            if let Some(event) = record
                .event
                .downcast::<crate::events::zones::ObjectLeavesGameEvent>()
            {
                return (event.object == object).then_some(&event.snapshot);
            }
            let event = record.event.downcast::<ZoneChangeEvent>()?;
            event
                .snapshots()
                .iter()
                .find(|snapshot| snapshot.object_id == object)
        })
    }

    /// Whether this object fought this turn (CR 701.14): a fight keyword
    /// action names each fighter as its source.
    pub fn object_fought_this_turn(&self, object_id: ObjectId, stable_id: StableId) -> bool {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<KeywordActionEvent>())
            .filter(|event| event.action == KeywordActionKind::Fight)
            .any(|event| match event.snapshot.as_ref() {
                Some(snapshot) => snapshot.stable_id == stable_id,
                None => event.source == object_id,
            })
    }

    pub fn object_was_surveilled_this_turn(&self, stable_id: StableId) -> bool {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<KeywordActionEvent>())
            .filter(|event| event.action == KeywordActionKind::Surveil)
            .any(|event| {
                event
                    .object_tags
                    .get(crate::tag::SURVEILLED_THIS_TURN_TAG)
                    .is_some_and(|snapshots| {
                        snapshots
                            .iter()
                            .any(|snapshot| snapshot.stable_id == stable_id)
                    })
            })
    }

    pub fn object_was_discarded_or_cycled_by_this_turn(
        &self,
        object_id: ObjectId,
        _stable_id: StableId,
        player: PlayerId,
    ) -> bool {
        // Discard history may find the object created by that hand departure,
        // but must not follow the physical card through later zone changes.
        let is_discard_result = |hand_id: ObjectId| {
            object_id == hand_id
                || self.projected_records().any(|record| {
                    record
                        .event
                        .downcast::<ZoneChangeEvent>()
                        .is_some_and(|event| {
                            event.from == Zone::Hand
                                && event
                                    .snapshots()
                                    .iter()
                                    .any(|snapshot| snapshot.object_id == hand_id)
                                && if event.result_objects.is_empty() {
                                    event.objects.contains(&object_id)
                                } else {
                                    event.result_objects.contains(&object_id)
                                }
                        })
                })
        };
        self.projected_records().any(|record| {
            if let Some(event) = record.event.downcast::<CardDiscardedEvent>() {
                return event.player == player && is_discard_result(event.card);
            }
            record
                .event
                .downcast::<KeywordActionEvent>()
                .is_some_and(|event| {
                    event.action == KeywordActionKind::Cycle
                        && event.player == player
                        && is_discard_result(event.source)
                })
        })
    }

    pub fn player_was_dealt_damage_by_creature_this_turn(&self, player: PlayerId) -> bool {
        self.total_creature_damage_to_player(player) > 0
    }

    pub fn source_dealt_combat_damage_to_player_this_turn(
        &self,
        source: ObjectId,
        source_stable_id: Option<StableId>,
    ) -> bool {
        self.projected_records().any(|record| {
            record.event.downcast::<DamageEvent>().is_some_and(|event| {
                event.is_combat
                    && event.amount > 0
                    && matches!(event.target, crate::events::DamageTarget::Player(_))
                    && (event.source == source
                        || source_stable_id.is_some_and(|stable_id| {
                            record
                                .source_snapshot
                                .as_ref()
                                .or(record.object_snapshot.as_ref())
                                .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                        }))
            })
        })
    }

    /// The object dealt damage to anything this turn (active voice:
    /// "target creature that dealt damage this turn").
    pub fn source_dealt_damage_this_turn(
        &self,
        source: ObjectId,
        source_stable_id: Option<StableId>,
    ) -> bool {
        self.projected_records().any(|record| {
            record.event.downcast::<DamageEvent>().is_some_and(|event| {
                event.amount > 0
                    && (event.source == source
                        || source_stable_id.is_some_and(|stable_id| {
                            record
                                .source_snapshot
                                .as_ref()
                                .or(record.object_snapshot.as_ref())
                                .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                        }))
            })
        })
    }

    pub fn source_dealt_damage_to_player_this_turn(
        &self,
        source: ObjectId,
        source_stable_id: Option<StableId>,
        player: PlayerId,
    ) -> bool {
        self.source_dealt_damage_to_player_this_turn_matching(
            source,
            source_stable_id,
            player,
            false,
        )
    }

    pub fn source_dealt_damage_to_player_this_turn_matching(
        &self,
        source: ObjectId,
        source_stable_id: Option<StableId>,
        player: PlayerId,
        combat_only: bool,
    ) -> bool {
        self.projected_records().any(|record| {
            record.event.downcast::<DamageEvent>().is_some_and(|event| {
                event.amount > 0
                    && (!combat_only || event.is_combat)
                    && matches!(
                        event.target,
                        crate::events::DamageTarget::Player(pid) if pid == player
                    )
                    && (event.source == source
                        || source_stable_id.is_some_and(|stable_id| {
                            record
                                .source_snapshot
                                .as_ref()
                                .or(record.object_snapshot.as_ref())
                                .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                        }))
            })
        })
    }

    pub fn player_dealt_combat_damage_to_player_with_subtype_this_turn(
        &self,
        dealer: PlayerId,
        subtype: Subtype,
    ) -> bool {
        self.projected_records().any(|record| {
            let Some(event) = record.event.downcast::<DamageEvent>() else {
                return false;
            };
            if !event.is_combat || event.amount == 0 {
                return false;
            }
            if !matches!(event.target, crate::events::DamageTarget::Player(_)) {
                return false;
            }

            record
                .source_snapshot
                .as_ref()
                .or(record.object_snapshot.as_ref())
                .is_some_and(|snapshot| {
                    snapshot.controller == dealer
                        && snapshot.card_types.contains(&CardType::Creature)
                        && snapshot_had_subtype(snapshot, subtype)
                })
        })
    }

    /// Prowl's check (CR 702.76a): combat damage to a player this turn from a
    /// source `dealer` controlled that, at the time, had any of `subtypes`.
    pub fn player_dealt_combat_damage_to_player_with_any_subtype_this_turn(
        &self,
        dealer: PlayerId,
        subtypes: &[Subtype],
    ) -> bool {
        self.projected_records().any(|record| {
            let Some(event) = record.event.downcast::<DamageEvent>() else {
                return false;
            };
            if !event.is_combat || event.amount == 0 {
                return false;
            }
            if !matches!(event.target, crate::events::DamageTarget::Player(_)) {
                return false;
            }

            record
                .source_snapshot
                .as_ref()
                .or(record.object_snapshot.as_ref())
                .is_some_and(|snapshot| {
                    snapshot.controller == dealer
                        && subtypes
                            .iter()
                            .any(|subtype| snapshot_had_subtype(snapshot, *subtype))
                })
        })
    }

    pub fn player_dealt_combat_damage_to_player_with_subtype_or_commander_this_turn(
        &self,
        dealer: PlayerId,
        subtype: Subtype,
    ) -> bool {
        self.projected_records().any(|record| {
            let Some(event) = record.event.downcast::<DamageEvent>() else {
                return false;
            };
            if !event.is_combat || event.amount == 0 {
                return false;
            }
            if !matches!(event.target, crate::events::DamageTarget::Player(_)) {
                return false;
            }

            record
                .source_snapshot
                .as_ref()
                .or(record.object_snapshot.as_ref())
                .is_some_and(|snapshot| {
                    snapshot.controller == dealer
                        && snapshot.card_types.contains(&CardType::Creature)
                        && (snapshot_had_subtype(snapshot, subtype) || snapshot.is_commander)
                })
        })
    }

    pub fn creature_was_damaged_by_source_this_turn(
        &self,
        creature: ObjectId,
        source: ObjectId,
    ) -> bool {
        self.creature_was_damaged_by_source_identity_this_turn(creature, None, source, None)
    }

    pub fn creature_was_damaged_by_source_identity_this_turn(
        &self,
        creature: ObjectId,
        creature_stable_id: Option<StableId>,
        source: ObjectId,
        source_stable_id: Option<StableId>,
    ) -> bool {
        self.projected_records().any(|record| {
            record.event.downcast::<DamageEvent>().is_some_and(|event| {
                let target_matches = match event.target {
                    crate::events::DamageTarget::Object(target) if target == creature => true,
                    crate::events::DamageTarget::Object(_) => {
                        creature_stable_id.is_some_and(|stable_id| {
                            event
                                .target_snapshot
                                .as_ref()
                                .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                        })
                    }
                    crate::events::DamageTarget::Player(_) => false,
                };
                let source_matches = event.source == source
                    || source_stable_id.is_some_and(|stable_id| {
                        record
                            .source_snapshot
                            .as_ref()
                            .or(record.object_snapshot.as_ref())
                            .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                    });
                target_matches && source_matches && event.amount > 0
            })
        })
    }

    pub fn creature_was_damaged_by_source_attached_to_this_turn(
        &self,
        creature: ObjectId,
        creature_stable_id: Option<StableId>,
        attachment_source: ObjectId,
    ) -> bool {
        self.projected_records().any(|record| {
            record.event.downcast::<DamageEvent>().is_some_and(|event| {
                let target_matches = match event.target {
                    crate::events::DamageTarget::Object(target) if target == creature => true,
                    crate::events::DamageTarget::Object(_) => {
                        creature_stable_id.is_some_and(|stable_id| {
                            event
                                .target_snapshot
                                .as_ref()
                                .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                        })
                    }
                    crate::events::DamageTarget::Player(_) => false,
                };
                let source_was_attached = record
                    .object_snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.attachments.contains(&attachment_source));
                target_matches && source_was_attached && event.amount > 0
            })
        })
    }

    pub fn creature_was_damaged_this_turn(&self, creature: ObjectId) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<DamageEvent>()
                .is_some_and(|event| {
                    matches!(event.target, crate::events::DamageTarget::Object(target) if target == creature)
                        && event.amount > 0
                })
        })
    }

    pub fn creature_was_blocked_this_turn(&self, creature: ObjectId) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<crate::events::CreatureBecameBlockedEvent>()
                .is_some_and(|event| event.attacker == creature)
                || record
                    .event
                    .downcast::<CreatureBlockedEvent>()
                    .is_some_and(|event| event.attacker == creature)
        })
    }

    pub fn creature_blocked_this_turn(&self, creature: ObjectId) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<CreatureBlockedEvent>()
                .is_some_and(|event| event.blocker == creature)
        })
    }

    pub fn creature_was_blocked_by_this_turn(&self, attacker: ObjectId, blocker: ObjectId) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<CreatureBlockedEvent>()
                .is_some_and(|event| event.attacker == attacker && event.blocker == blocker)
        })
    }

    /// Current-combat LKI for a source that left after an ability triggered.
    /// The object IDs deliberately remain the declaration identities, so a
    /// leave-and-return is not mistaken for the creature from that combat.
    pub fn creature_was_blocked_by_in_combat(
        &self,
        attacker: ObjectId,
        blocker: ObjectId,
        combat_phase: u32,
    ) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<CreatureBlockedEvent>()
                .is_some_and(|event| {
                    event.attacker == attacker
                        && event.blocker == blocker
                        && event.combat_phase.is_none_or(|phase| phase == combat_phase)
                })
        })
    }

    pub fn player_searched_library_this_turn(&self, player: PlayerId) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<SearchLibraryEvent>()
                .is_some_and(|event| event.player == player)
        })
    }

    pub fn player_committed_crime_this_turn(&self, player: PlayerId) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<KeywordActionEvent>()
                .is_some_and(|event| {
                    event.player == player && event.action == KeywordActionKind::CommitCrime
                })
        })
    }

    pub fn completed_die_roll_count(&self, player: PlayerId) -> u32 {
        self.completed_die_rolls_this_turn
            .get(&player)
            .copied()
            .unwrap_or(0)
            .max(self.die_rolls_this_turn.get(&player).map_or(0, Vec::len) as u32)
    }

    /// Commit the whole completed roll batch together, after all reroll/result
    /// choices. Returns its first ordinal; callers preserve each exact result.
    pub(crate) fn check_completed_die_roll_capacity(
        &self,
        player: PlayerId,
        count: usize,
    ) -> Result<(u32, u32), crate::effects::ExecutionError> {
        let before = self.completed_die_roll_count(player);
        let total = u128::from(before) + count as u128;
        let after = i32::try_from(total).map_err(|_| {
            crate::effects::ExecutionError::ResourceLimitExceeded {
                resource: "completed die-roll ordinal",
                requested: total,
                maximum: i32::MAX as u128,
            }
        })? as u32;
        Ok((before, after))
    }

    pub(crate) fn record_completed_die_rolls(
        &mut self,
        player: PlayerId,
        results: &[u32],
        planar: bool,
    ) -> Result<u32, crate::effects::ExecutionError> {
        let (before, after) = self.check_completed_die_roll_capacity(player, results.len())?;
        if !planar {
            let history = self.die_rolls_this_turn.entry(player).or_default();
            history.try_reserve(results.len()).map_err(|_| {
                crate::effects::ExecutionError::ResourceAllocationFailed {
                    resource: "completed die-roll history",
                    requested: results.len(),
                }
            })?;
            history.extend_from_slice(results);
        }
        self.completed_die_rolls_this_turn.insert(player, after);
        Ok(before + 1)
    }

    pub fn record_die_roll(&mut self, player: PlayerId, result: u32) {
        self.die_rolls_this_turn
            .entry(player)
            .or_default()
            .push(result);
    }

    pub fn record_die_roll_result_adjustment(
        &mut self,
        source: ObjectId,
        ability: StaticAbilityInstanceId,
    ) {
        self.die_roll_result_adjustments_this_turn
            .insert((source, ability));
    }

    pub fn die_roll_modifier_used_this_turn(
        &self,
        source: ObjectId,
        ability: StaticAbilityInstanceId,
    ) -> bool {
        self.die_roll_result_adjustments_this_turn
            .contains(&(source, ability))
    }

    pub fn die_roll_result_adjusted_this_turn(&self, source: ObjectId) -> bool {
        self.die_roll_result_adjustments_this_turn
            .iter()
            .any(|(used_source, _)| *used_source == source)
    }

    pub fn player_rolled_result_this_turn(&self, player: PlayerId, result: u32) -> bool {
        self.die_rolls_this_turn
            .get(&player)
            .is_some_and(|rolls| rolls.contains(&result))
    }

    pub fn player_sacrificed_artifact_this_turn(&self, player: PlayerId) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<SacrificeEvent>()
                .is_some_and(|event| {
                    let sacrificing_player = event
                        .sacrificing_player
                        .or_else(|| event.snapshot.as_ref().map(|snapshot| snapshot.controller));
                    let sacrificed_artifact = event
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.card_types.contains(&CardType::Artifact));
                    sacrificing_player == Some(player) && sacrificed_artifact
                })
        })
    }

    pub fn permanents_left_battlefield_under_controller(&self, player: PlayerId) -> u32 {
        self.battlefield_departure_snapshots(None)
            .filter(|snapshot| snapshot.controller == player)
            .count() as u32
    }

    pub fn permanents_left_battlefield_this_turn(&self) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.from == Zone::Battlefield)
            .map(|event| event.objects.len().max(event.snapshots().len()).max(1))
            .sum::<usize>() as u32
    }

    pub fn nonland_permanents_left_battlefield_this_turn(&self) -> u32 {
        self.battlefield_departure_snapshots(None)
            .filter(|snapshot| !snapshot.card_types.contains(&CardType::Land))
            .count() as u32
    }

    pub fn spell_was_warped_this_turn(&self) -> bool {
        self.spell_warped_this_turn
    }

    pub fn creatures_left_battlefield_under_controller(&self, player: PlayerId) -> u32 {
        self.battlefield_departure_snapshots(None)
            .filter(|snapshot| {
                snapshot.controller == player && snapshot.card_types.contains(&CardType::Creature)
            })
            .count() as u32
    }

    pub fn spell_cast_event_provenance(&self, spell: ObjectId) -> Option<ProvNodeId> {
        self.projected_records().find_map(|record| {
            record
                .event
                .downcast::<SpellCastEvent>()
                .filter(|event| event.spell == spell)
                .map(|_| record.event.provenance())
        })
    }

    pub fn spell_cast_order(&self, spell: ObjectId) -> Option<u32> {
        let mut order = 0u32;
        for record in self.projected_records() {
            let Some(event) = record.event.downcast::<SpellCastEvent>() else {
                continue;
            };
            order = order.saturating_add(1);
            if event.spell == spell {
                return Some(order);
            }
        }
        None
    }

    /// Spells cast this turn by any of `players` strictly before `spell` was
    /// cast (CR 702.40a storm count). `None` when `spell` wasn't cast this turn.
    pub fn spells_cast_before_spell_for_players(
        &self,
        spell: ObjectId,
        players: &[PlayerId],
    ) -> Option<u32> {
        let mut count = 0u32;
        for record in self.projected_records() {
            let Some(event) = record.event.downcast::<SpellCastEvent>() else {
                continue;
            };
            if event.spell == spell {
                return Some(count);
            }
            if players.contains(&event.caster) {
                count = count.saturating_add(1);
            }
        }
        None
    }

    pub fn spell_cast_order_for_player(&self, spell: ObjectId, player: PlayerId) -> Option<u32> {
        let mut order = 0u32;
        for record in self.projected_records() {
            let Some(event) = record.event.downcast::<SpellCastEvent>() else {
                continue;
            };
            if event.caster != player {
                continue;
            }
            order = order.saturating_add(1);
            if event.spell == spell {
                return Some(order);
            }
        }
        None
    }

    pub fn spell_cast_snapshot_history(&self) -> Vec<ObjectSnapshot> {
        let mut order = 0u32;
        let mut snapshots = Vec::new();
        for record in self.projected_records() {
            if record.event.downcast::<SpellCastEvent>().is_none() {
                continue;
            }
            order = order.saturating_add(1);
            if let Some(snapshot) = record.object_snapshot.as_ref() {
                let mut snapshot = snapshot.clone();
                snapshot.cast_order_this_turn = Some(order);
                snapshots.push(snapshot);
            }
        }
        snapshots
    }

    pub fn damage_dealt_by_spell_this_turn(
        &self,
        provenance_graph: &ProvenanceGraph,
        spell: ObjectId,
    ) -> u32 {
        let cast_event_provenance = self.spell_cast_event_provenance(spell).filter(|prov| {
            *prov != ProvNodeId::default() && provenance_graph.node(*prov).is_some()
        });

        self.projected_records()
            .filter_map(|record| {
                let damage = record.event.downcast::<DamageEvent>()?;
                if damage.source != spell {
                    return None;
                }

                if let Some(cast_provenance) = cast_event_provenance
                    && !provenance_graph
                        .is_descendant_of(record.event.provenance(), cast_provenance)
                {
                    return None;
                }

                Some(damage.amount)
            })
            .sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HistoricalObjectIdentity {
    Stable(StableId),
    Object(ObjectId),
}

fn historical_identity(
    object: ObjectId,
    snapshot: Option<&ObjectSnapshot>,
) -> HistoricalObjectIdentity {
    snapshot
        .map(|snapshot| HistoricalObjectIdentity::Stable(snapshot.stable_id))
        .unwrap_or(HistoricalObjectIdentity::Object(object))
}

fn snapshot_is_permanent_card(snapshot: &ObjectSnapshot) -> bool {
    const PERMANENT_CARD_TYPES: [CardType; 6] = [
        CardType::Artifact,
        CardType::Battle,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
    ];

    !snapshot.is_token
        && snapshot
            .card_types
            .iter()
            .any(|card_type| PERMANENT_CARD_TYPES.contains(card_type))
}

/// One entry can publish a zone notification and an ETB notification. Prefer
/// the latter's completed characteristics and count its exact destination
/// incarnation once. A later leave/reenter has a new ID and counts again.
fn completed_entry_snapshots(history: &TurnHistory) -> Vec<&ObjectSnapshot> {
    let mut entries: HashMap<ObjectId, (bool, &ObjectSnapshot)> = HashMap::new();
    for record in history.projected_records() {
        if let Some(event) = record.event.downcast::<EnterBattlefieldEvent>() {
            if let Some(snapshot) = event
                .completed_snapshot
                .as_ref()
                .or(record.object_snapshot.as_ref())
                .filter(|snapshot| {
                    snapshot.zone == Zone::Battlefield && snapshot.object_id == event.object
                })
            {
                match entries.entry(event.object) {
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert((true, snapshot));
                    }
                    std::collections::hash_map::Entry::Occupied(mut entry) if !entry.get().0 => {
                        entry.insert((true, snapshot));
                    }
                    _ => {}
                }
            }
        } else if let Some(event) = record
            .event
            .downcast::<ZoneChangeEvent>()
            .filter(|event| event.is_etb())
        {
            let destinations = if event.result_objects.is_empty() {
                &event.objects
            } else {
                &event.result_objects
            };
            if let Some(snapshot) = record.object_snapshot.as_ref().filter(|snapshot| {
                snapshot.zone == Zone::Battlefield && destinations.contains(&snapshot.object_id)
            }) {
                entries
                    .entry(snapshot.object_id)
                    .or_insert((false, snapshot));
            }
        }
    }
    entries
        .into_values()
        .map(|(_, snapshot)| snapshot)
        .collect()
}

fn historical_cause_matches(
    requested: &crate::events::cause::CauseFilter,
    actual: &crate::events::cause::EventCause,
    source_snapshot: Option<&ObjectSnapshot>,
    game: &GameState,
    ctx: &crate::target::FilterContext,
) -> bool {
    use crate::events::cause::CauseFilterRuntimeExt;
    let Some(you) = ctx.you else {
        return false;
    };
    // Never reopen a later live incarnation to satisfy a historical cause.
    if let Some(filter) = &requested.source_filter
        && !source_snapshot.is_some_and(|snapshot| filter.matches_snapshot(snapshot, ctx, game))
    {
        return false;
    }
    let mut without_source = requested.clone();
    without_source.source_filter = None;
    without_source.matches_with_context_controller(actual, game, you, you)
}

/// Resolve a typed turn-history count against event snapshots.  This is shared
/// by ordinary effect values and continuous/static values so both paths use the
/// same retained-event semantics.
pub(crate) fn resolve_turn_history_count(
    game: &GameState,
    query: &TurnHistoryCount,
    filter_ctx: &crate::target::FilterContext,
    triggering_event: Option<&TriggerEvent>,
) -> i32 {
    let history = &game.turn_store.turn_history;

    match query {
        TurnHistoryCount::LibrarySearches {
            player,
            own_library_only,
        } => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<SearchLibraryEvent>())
            .filter(|event| {
                player.matches_player(event.player, filter_ctx)
                    && (!*own_library_only || event.library_owner == Some(event.player))
            })
            .count() as i32,
        TurnHistoryCount::MaxEnteredBattlefieldByController { player, filter } => {
            let mut historical_filter = filter.clone();
            historical_filter.zone = None;
            let mut counts: HashMap<PlayerId, i32> = HashMap::new();
            for snapshot in completed_entry_snapshots(history) {
                if player.matches_player(snapshot.controller, filter_ctx)
                    && historical_filter.matches_snapshot(snapshot, filter_ctx, game)
                {
                    let count = counts.entry(snapshot.controller).or_default();
                    *count = count.saturating_add(1);
                }
            }
            counts.into_values().max().unwrap_or(0)
        }
        TurnHistoryCount::DestroyedBy { filter, cause } => history
            .projected_records()
            .filter(|record| {
                let Some(event) = record.event.downcast::<crate::events::DestroyEvent>() else {
                    return false;
                };
                event.final_zone.is_some()
                    && event
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| filter.matches_snapshot(snapshot, filter_ctx, game))
                    && event.cause.as_ref().is_some_and(|actual| {
                        historical_cause_matches(
                            cause,
                            actual,
                            record.source_snapshot.as_ref(),
                            game,
                            filter_ctx,
                        )
                    })
            })
            .count() as i32,
        TurnHistoryCount::CastSpellsCounteredBy {
            caster,
            filter,
            cause,
        } => {
            let mut casts = HashSet::new();
            let mut countered = HashSet::new();
            for record in history.projected_records() {
                if let Some(event) = record.event.downcast::<SpellCastEvent>() {
                    if caster.matches_player(event.caster, filter_ctx) {
                        casts.insert(event.spell);
                    }
                    continue;
                }
                let Some(event) = record
                    .event
                    .downcast::<crate::events::SpellCounteredEvent>()
                else {
                    continue;
                };
                if casts.contains(&event.spell)
                    && event
                        .snapshot
                        .as_ref()
                        .or(record.object_snapshot.as_ref())
                        .is_some_and(|snapshot| filter.matches_snapshot(snapshot, filter_ctx, game))
                    && event.cause.as_ref().is_some_and(|actual| {
                        historical_cause_matches(
                            cause,
                            actual,
                            record.source_snapshot.as_ref(),
                            game,
                            filter_ctx,
                        )
                    })
                {
                    countered.insert(event.spell);
                }
            }
            countered.len() as i32
        }

        TurnHistoryCount::Died { filter, .. } => {
            let mut historical_filter = filter.clone();
            historical_filter.zone = None;
            history
                .projected_records()
                .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
                .filter(|event| event.is_dies())
                .flat_map(ZoneChangeEvent::snapshots)
                .filter(|snapshot| historical_filter.matches_snapshot(snapshot, filter_ctx, game))
                .count() as i32
        }
        TurnHistoryCount::EnteredBattlefield(filter) => {
            let mut historical_filter = filter.clone();
            historical_filter.zone = None;
            completed_entry_snapshots(history)
                .into_iter()
                .filter(|snapshot| historical_filter.matches_snapshot(snapshot, filter_ctx, game))
                .count() as i32
        }
        TurnHistoryCount::TurnedFaceUp(player_filter) => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<crate::events::TurnedFaceUpEvent>())
            .filter(|event| player_filter.matches_player(event.player, filter_ctx))
            .count() as i32,
        TurnHistoryCount::TokensCreated(player_filter) => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<CreateTokensEvent>())
            .filter(|event| player_filter.matches_player(event.controller, filter_ctx))
            .map(|event| {
                u32::try_from(event.total_count())
                    .expect("published token groups passed checked creation preflight")
            })
            .sum::<u32>() as i32,
        TurnHistoryCount::PutIntoGraveyard { owner, from } => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.to == Zone::Graveyard)
            .filter(|event| from.is_empty() || from.contains(&event.from))
            .flat_map(ZoneChangeEvent::snapshots)
            .filter(|snapshot| owner.matches_player(snapshot.owner, filter_ctx))
            .count() as i32,
        TurnHistoryCount::MovedZones { filter, from, to } => {
            let mut historical_filter = filter.clone();
            historical_filter.zone = None;
            history
                .projected_records()
                .filter_map(|record| {
                    let event = record.event.downcast::<ZoneChangeEvent>()?;
                    if from.is_some_and(|zone| zone != event.from)
                        || to.is_some_and(|zone| zone != event.to)
                    {
                        return None;
                    }
                    if !event.snapshots().is_empty() {
                        Some(event.snapshots().to_vec())
                    } else {
                        record
                            .object_snapshot
                            .clone()
                            .map(|snapshot| vec![snapshot])
                    }
                })
                .flatten()
                .filter(|snapshot| historical_filter.matches_snapshot(snapshot, filter_ctx, game))
                .count() as i32
        }
        TurnHistoryCount::Sacrificed { player, filter } => history
            .projected_records()
            .filter_map(|record| {
                let event = record.event.downcast::<SacrificeEvent>()?;
                let snapshot = event
                    .snapshot
                    .as_ref()
                    .or(record.object_snapshot.as_ref())?;
                let sacrificing_player = event.sacrificing_player.unwrap_or(snapshot.controller);
                Some((sacrificing_player, snapshot))
            })
            .filter(|(sacrificing_player, snapshot)| {
                player.matches_player(*sacrificing_player, filter_ctx)
                    && filter.matches_snapshot(snapshot, filter_ctx, game)
            })
            .count() as i32,
        TurnHistoryCount::SacrificedCardTypes { player, filter } => {
            // A card type counts once however many sacrificed permanents had
            // it; each permanent is read as it last existed on the battlefield.
            let mut card_types = Vec::<CardType>::new();
            for record in history.projected_records() {
                let Some(event) = record.event.downcast::<SacrificeEvent>() else {
                    continue;
                };
                let Some(snapshot) = event.snapshot.as_ref().or(record.object_snapshot.as_ref())
                else {
                    continue;
                };
                let sacrificing_player = event.sacrificing_player.unwrap_or(snapshot.controller);
                if !player.matches_player(sacrificing_player, filter_ctx)
                    || !filter.matches_snapshot(snapshot, filter_ctx, game)
                {
                    continue;
                }
                for card_type in &snapshot.card_types {
                    if !card_types.contains(card_type) {
                        card_types.push(*card_type);
                    }
                }
            }
            card_types.len() as i32
        }
        TurnHistoryCount::CountersPutOn {
            source_controller,
            counter_type,
            filter,
        } => history
            .projected_records()
            .filter_map(|record| {
                let snapshot = record.object_snapshot.as_ref()?;
                if let Some(player) = source_controller {
                    let event = record
                        .event
                        .downcast::<crate::events::MarkersChangedEvent>()?;
                    return (event.is_added()
                        && event.object().is_some()
                        && event.marker.as_counter().is_some()
                        && counter_type
                            .is_none_or(|kind| event.marker.as_counter() == Some(kind))
                        && event
                            .source_controller
                            .is_some_and(|actor| player.matches_player(actor, filter_ctx))
                        && filter.matches_snapshot(snapshot, filter_ctx, game))
                    .then_some(event.amount);
                }
                let event = record.event.downcast::<CounterPlacedEvent>()?;
                (counter_type.is_none_or(|counter_type| event.counter_type == counter_type)
                    && filter.matches_snapshot(snapshot, filter_ctx, game))
                .then_some(event.amount)
            })
            .sum::<u32>() as i32,
        TurnHistoryCount::CreaturesAttackedWith { player, filter } => {
            let mut seen = HashSet::new();
            for record in history.projected_records() {
                let Some(event) = record.event.downcast::<CreatureAttackedEvent>() else {
                    continue;
                };
                let Some(snapshot) = record.object_snapshot.as_ref() else {
                    continue;
                };
                if !player.matches_player(snapshot.controller, filter_ctx)
                    || !filter.matches_snapshot(snapshot, filter_ctx, game)
                {
                    continue;
                }
                seen.insert(historical_identity(event.attacker, Some(snapshot)));
            }
            seen.len() as i32
        }
        TurnHistoryCount::PlayersAttackedThisCombat(player) => {
            let mut seen = HashSet::new();
            for record in history.projected_records().rev() {
                if record
                    .event
                    .downcast::<crate::events::BeginningOfCombatEvent>()
                    .is_some()
                {
                    break;
                }
                let Some(event) = record.event.downcast::<CreatureAttackedEvent>() else {
                    continue;
                };
                let Some(snapshot) = record.object_snapshot.as_ref() else {
                    continue;
                };
                if player.matches_player(snapshot.controller, filter_ctx)
                    && let crate::triggers::event::AttackEventTarget::Player(defender) =
                        event.target
                {
                    seen.insert(defender);
                }
            }
            seen.len() as i32
        }
        TurnHistoryCount::OpponentsAttacked(player) => {
            let mut seen = HashSet::new();
            for record in history.projected_records() {
                let Some(event) = record.event.downcast::<CreatureAttackedEvent>() else {
                    continue;
                };
                let Some(snapshot) = record.object_snapshot.as_ref() else {
                    continue;
                };
                if !player.matches_player(snapshot.controller, filter_ctx) {
                    continue;
                }
                if let crate::triggers::event::AttackEventTarget::Player(defender) = event.target
                    && filter_ctx.opponents.contains(&defender)
                {
                    seen.insert(defender);
                }
            }
            seen.len() as i32
        }
        TurnHistoryCount::PlayersDiscarded(player) => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<CardDiscardedEvent>())
            .map(|event| event.player)
            .filter(|discarding_player| player.matches_player(*discarding_player, filter_ctx))
            .collect::<HashSet<_>>()
            .len() as i32,
        TurnHistoryCount::PlayersDealtDamage(player) => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<DamageEvent>())
            .filter(|event| event.amount > 0)
            .filter_map(|event| match event.target {
                DamageTarget::Player(target) if player.matches_player(target, filter_ctx) => {
                    Some(target)
                }
                _ => None,
            })
            .collect::<HashSet<_>>()
            .len() as i32,
        TurnHistoryCount::PlayersDealtCombatDamageBy { players, sources } => history
            .projected_records()
            .filter_map(|record| {
                let event = record.event.downcast::<DamageEvent>()?;
                if !event.is_combat || event.amount == 0 {
                    return None;
                }
                let DamageTarget::Player(target) = event.target else {
                    return None;
                };
                let source = record
                    .source_snapshot
                    .as_ref()
                    .or(record.object_snapshot.as_ref())?;
                (players.matches_player(target, filter_ctx)
                    && sources.matches_snapshot(source, filter_ctx, game))
                .then_some(target)
            })
            .collect::<HashSet<_>>()
            .len()
            as i32,
        TurnHistoryCount::DiscardedOrCycled(player) => {
            let mut seen = HashSet::new();
            for record in history.projected_records() {
                if let Some(event) = record.event.downcast::<CardDiscardedEvent>()
                    && player.matches_player(event.player, filter_ctx)
                {
                    seen.insert(historical_identity(event.card, event.snapshot.as_ref()));
                }
                if let Some(event) = record.event.downcast::<KeywordActionEvent>()
                    && event.action == KeywordActionKind::Cycle
                    && player.matches_player(event.player, filter_ctx)
                {
                    seen.insert(historical_identity(event.source, event.snapshot.as_ref()));
                }
            }
            seen.len() as i32
        }
        TurnHistoryCount::CardsDrawn(player) => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<CardsDrawnEvent>())
            .filter(|event| player.matches_player(event.player, filter_ctx))
            .map(CardsDrawnEvent::amount)
            .sum::<u32>() as i32,
        TurnHistoryCount::KeywordActionsPerformed { player, actions } => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<KeywordActionEvent>())
            .filter(|event| {
                actions.contains(&event.action) && player.matches_player(event.player, filter_ctx)
            })
            .count()
            as i32,
        TurnHistoryCount::CountersRemovedFrom {
            counter_type,
            filter,
        } => {
            let mut historical_filter = filter.clone();
            historical_filter.zone = None;
            history
                .projected_records()
                .filter_map(|record| {
                    let event = record
                        .event
                        .downcast::<crate::events::MarkersChangedEvent>()?;
                    let counter = event.marker.as_counter()?;
                    if !event.is_removed()
                        || event.object().is_none()
                        || counter_type.is_some_and(|kind| kind != counter)
                    {
                        return None;
                    }
                    let snapshot = record.object_snapshot.as_ref()?;
                    historical_filter
                        .matches_snapshot(snapshot, filter_ctx, game)
                        .then_some(event.amount)
                })
                .sum::<u32>() as i32
        }
        TurnHistoryCount::Cycled(player) => {
            let mut seen = HashSet::new();
            for record in history.projected_records() {
                let Some(event) = record.event.downcast::<KeywordActionEvent>() else {
                    continue;
                };
                if event.action != KeywordActionKind::Cycle
                    || !player.matches_player(event.player, filter_ctx)
                {
                    continue;
                }
                seen.insert(historical_identity(event.source, event.snapshot.as_ref()));
            }
            seen.len() as i32
        }
        TurnHistoryCount::PlayersLostLife(player) => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<LifeLossEvent>())
            .filter(|event| event.amount > 0 && player.matches_player(event.player, filter_ctx))
            .map(|event| event.player)
            .collect::<HashSet<_>>()
            .len() as i32,
        TurnHistoryCount::UntappedLandsAtTurnStart(player) => history
            .untapped_lands_at_turn_start
            .iter()
            .filter(|(player_id, _)| player.matches_player(**player_id, filter_ctx))
            .map(|(_, count)| *count as i32)
            .sum(),
        TurnHistoryCount::Descended(player) => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.to == Zone::Graveyard)
            .flat_map(ZoneChangeEvent::snapshots)
            .filter(|snapshot| {
                snapshot_is_permanent_card(snapshot)
                    && player.matches_player(snapshot.owner, filter_ctx)
            })
            .count() as i32,
        TurnHistoryCount::DamageDealtBySource => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<DamageEvent>())
            .filter(|event| Some(event.source) == filter_ctx.source)
            .map(|event| event.amount)
            .sum::<u32>() as i32,
        TurnHistoryCount::DamageDealtToSource => {
            let source_object = filter_ctx.source.or_else(|| {
                filter_ctx
                    .source_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.object_id)
            });
            history.projected_records()
                .filter_map(|record| record.event.downcast::<DamageEvent>())
                .filter(|event| matches!(event.target,DamageTarget::Object(target) if source_object == Some(target)))
                .map(|event| event.amount)
                .sum::<u32>() as i32
        }
        TurnHistoryCount::SpellsCast {
            player,
            filter,
            from_zone,
            from_outside_hand,
            exclude_source,
            before_triggering_spell,
        } => {
            let triggering_cast = if *before_triggering_spell {
                triggering_event.and_then(|event| {
                    event
                        .downcast::<SpellCastEvent>()
                        .map(|cast| (event.provenance(), cast.spell))
                })
            } else {
                None
            };
            if *before_triggering_spell && triggering_cast.is_none() {
                return 0;
            }

            let mut count = 0i32;
            let mut found_boundary = !*before_triggering_spell;
            for record in history.projected_records() {
                let Some(event) = record.event.downcast::<SpellCastEvent>() else {
                    continue;
                };
                if let Some((trigger_provenance, trigger_spell)) = triggering_cast {
                    let same_provenance = trigger_provenance != ProvNodeId::default()
                        && record.event.provenance() == trigger_provenance;
                    if same_provenance || event.spell == trigger_spell {
                        found_boundary = true;
                        break;
                    }
                }
                let Some(snapshot) = event.snapshot.as_ref().or(record.object_snapshot.as_ref())
                else {
                    continue;
                };
                if player.matches_player(event.caster, filter_ctx)
                    && from_zone.is_none_or(|zone| event.from_zone == zone)
                    && (!*from_outside_hand || event.from_zone != Zone::Hand)
                    && (!*exclude_source || Some(snapshot.object_id) != filter_ctx.source)
                    && filter.matches_snapshot(snapshot, filter_ctx, game)
                {
                    count = count.saturating_add(1);
                }
            }
            if found_boundary { count } else { 0 }
        }
        TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(player) => {
            let mut colors = ColorSet::new();
            for &object_id in &game.battlefield {
                let Some(object) = game.object(object_id) else {
                    continue;
                };
                if player.matches_player(game.controller_of(object), filter_ctx) {
                    colors = colors.union(object.colors());
                }
            }
            for record in history.projected_records() {
                let Some(event) = record.event.downcast::<SpellCastEvent>() else {
                    continue;
                };
                if !player.matches_player(event.caster, filter_ctx) {
                    continue;
                }
                if let Some(snapshot) = event.snapshot.as_ref().or(record.object_snapshot.as_ref())
                {
                    colors = colors.union(snapshot.colors);
                }
            }
            colors.count() as i32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::events::EventCause;
    use crate::filter::{ObjectFilter, PlayerFilter, StackObjectKind};
    use crate::ids::CardId;

    fn cast_event(game: &GameState, spell: ObjectId, caster: PlayerId) -> TriggerEvent {
        let snapshot = ObjectSnapshot::from_object(
            game.object(spell).expect("spell object should exist"),
            game,
        );
        TriggerEvent::new_with_provenance(
            SpellCastEvent::new_with_snapshot(spell, caster, Zone::Hand, snapshot),
            ProvNodeId::default(),
        )
    }

    #[test]
    fn persistent_turn_records_preserve_staging_batches_and_branch_isolation() {
        let mut game = GameState::new(vec!["Alice".into()], 20);
        let alice = PlayerId::from_index(0);
        let card = CardDefinitionBuilder::new(CardId::new(), "Recorded object")
            .card_types(vec![CardType::Creature])
            .build();
        let object = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let snapshot = ObjectSnapshot::from_object(game.object(object).unwrap(), &game);
        let mut provenance = ProvenanceGraph::default();
        let mut history = TurnHistory::default();
        for amount in 1..=130 {
            let event = TriggerEvent::new_with_provenance(
                LifeLossEvent::from_effect(alice, amount),
                provenance.alloc_root_event(EventKind::LifeLoss),
            );
            history.record_event(&event, Some(snapshot.clone()), Some(snapshot.clone()));
        }
        let previous = history.clone();
        let mut branch = history.clone();
        assert!(std::ptr::eq(
            &history.event_records[64],
            &branch.event_records[64]
        ));
        assert!(std::ptr::eq(
            history.event_records[64].object_snapshot.as_ref().unwrap(),
            branch.event_records[64].object_snapshot.as_ref().unwrap()
        ));
        let mark = history.begin_simultaneous_batch();
        assert_eq!(mark, None);
        assert_eq!(history.simultaneous_batch_start, Some(130));
        assert_eq!(branch.simultaneous_batch_start, None);
        let staged_id = provenance.alloc_root_event(EventKind::LifeLoss);
        let staged =
            TriggerEvent::new_with_provenance(LifeLossEvent::from_effect(alice, 2), staged_id);
        history.stage_event(&staged, Some(snapshot.clone()), None);
        let replacement =
            TriggerEvent::new_with_provenance(LifeLossEvent::from_effect(alice, 3), staged_id);
        history.stage_event(&replacement, Some(snapshot.clone()), None);
        assert_eq!(history.staged_event_records.len(), 1);
        assert_eq!(history.projected_records().count(), 131);
        let staged_branch = history.clone();
        history.record_event(&replacement, Some(snapshot.clone()), None);
        assert!(history.staged_event_records.is_empty());
        assert_eq!(staged_branch.staged_event_records.len(), 1);
        assert_eq!(history.event_kind_count(EventKind::LifeLoss), 131);
        assert_eq!(history.total_life_lost_for_players(&[alice]), 8518);
        history.end_simultaneous_batch(mark);
        assert_eq!(history.simultaneous_batch_start, None);
        branch.event_records.truncate(65);
        assert_eq!(branch.event_records.len(), 65);
        assert_eq!(previous.event_records.len(), 130);
        assert_eq!(
            previous
                .event_records
                .iter()
                .rev()
                .next()
                .unwrap()
                .event
                .downcast::<LifeLossEvent>()
                .unwrap()
                .amount,
            130
        );
        history.clear_for_new_turn();
        assert!(history.event_records.is_empty());
        assert_eq!(previous.total_life_lost_for_players(&[alice]), 8515);
        assert_eq!(staged_branch.projected_records().count(), 131);
    }

    #[test]
    fn triggering_cast_boundary_excludes_the_trigger_and_later_responses() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let instant = CardDefinitionBuilder::new(CardId::new(), "History Instant")
            .card_types(vec![CardType::Instant])
            .build();

        let alice_first = game.create_object_from_definition(&instant, alice, Zone::Stack);
        let bob_first = game.create_object_from_definition(&instant, bob, Zone::Stack);
        let triggering_spell = game.create_object_from_definition(&instant, alice, Zone::Stack);
        let response_spell = game.create_object_from_definition(&instant, alice, Zone::Stack);
        for (spell, caster) in [(alice_first, alice), (bob_first, bob)] {
            game.record_turn_history_event(&cast_event(&game, spell, caster));
        }
        let triggering_event = cast_event(&game, triggering_spell, alice);
        game.record_turn_history_event(&triggering_event);
        game.record_turn_history_event(&cast_event(&game, response_spell, alice));

        let mut spell_filter = ObjectFilter::default();
        spell_filter.card_types = vec![CardType::Instant, CardType::Sorcery];
        spell_filter.stack_kind = Some(StackObjectKind::Spell);
        let before_trigger = TurnHistoryCount::SpellsCast {
            player: PlayerFilter::You,
            filter: spell_filter.clone(),
            from_zone: None,
            from_outside_hand: false,
            exclude_source: false,
            before_triggering_spell: true,
        };
        let alice_ctx = crate::filter::FilterContext::new(alice);
        assert_eq!(
            resolve_turn_history_count(&game, &before_trigger, &alice_ctx, Some(&triggering_event),),
            1,
            "Alice's response after the triggering cast is outside the boundary"
        );

        let before_trigger_any = TurnHistoryCount::SpellsCast {
            player: PlayerFilter::Any,
            filter: spell_filter.clone(),
            from_zone: None,
            from_outside_hand: false,
            exclude_source: false,
            before_triggering_spell: true,
        };
        assert_eq!(
            resolve_turn_history_count(
                &game,
                &before_trigger_any,
                &alice_ctx,
                Some(&triggering_event),
            ),
            2,
            "Sentinel-style counts include all players but stop at the triggering cast"
        );

        let ordinary_other = TurnHistoryCount::SpellsCast {
            player: PlayerFilter::You,
            filter: spell_filter,
            from_zone: None,
            from_outside_hand: false,
            exclude_source: true,
            before_triggering_spell: false,
        };
        let source_ctx = crate::filter::FilterContext::new(alice).with_source(triggering_spell);
        assert_eq!(
            resolve_turn_history_count(&game, &ordinary_other, &source_ctx, None),
            2,
            "ordinary other-spell counts include later casts and exclude only the source spell"
        );
    }

    #[test]
    fn descended_history_count_uses_owner_and_permanent_card_lki() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = CardDefinitionBuilder::new(CardId::new(), "History Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let instant = CardDefinitionBuilder::new(CardId::new(), "History Instant")
            .card_types(vec![CardType::Instant])
            .build();

        let alice_first = game.create_object_from_definition(&creature, alice, Zone::Library);
        let alice_second = game.create_object_from_definition(&creature, alice, Zone::Hand);
        let bob_permanent = game.create_object_from_definition(&creature, bob, Zone::Library);
        let alice_instant = game.create_object_from_definition(&instant, alice, Zone::Library);

        for (object, from) in [
            (alice_first, Zone::Library),
            (alice_second, Zone::Hand),
            (bob_permanent, Zone::Library),
            (alice_instant, Zone::Library),
        ] {
            let snapshot = ObjectSnapshot::from_object(
                game.object(object).expect("history object should exist"),
                &game,
            );
            let event = TriggerEvent::new_with_provenance(
                ZoneChangeEvent::with_cause(
                    object,
                    from,
                    Zone::Graveyard,
                    EventCause::effect(),
                    Some(snapshot),
                ),
                ProvNodeId::default(),
            );
            game.record_turn_history_event(&event);
        }

        let query = TurnHistoryCount::Descended(PlayerFilter::You);
        let alice_ctx = crate::filter::FilterContext::new(alice);
        let bob_ctx = crate::filter::FilterContext::new(bob);
        assert_eq!(
            resolve_turn_history_count(&game, &query, &alice_ctx, None),
            2
        );
        assert_eq!(resolve_turn_history_count(&game, &query, &bob_ctx, None), 1);
    }

    #[test]
    fn graveyard_entry_from_library_history_tracks_stable_identity_and_origin() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let creature = CardDefinitionBuilder::new(CardId::new(), "History Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let from_library = game.create_object_from_definition(&creature, alice, Zone::Graveyard);
        let from_hand = game.create_object_from_definition(&creature, alice, Zone::Graveyard);

        for (object, from) in [(from_library, Zone::Library), (from_hand, Zone::Hand)] {
            let snapshot = ObjectSnapshot::from_object(
                game.object(object).expect("history object should exist"),
                &game,
            );
            let event = TriggerEvent::new_with_provenance(
                ZoneChangeEvent::with_cause(
                    object,
                    from,
                    Zone::Graveyard,
                    EventCause::effect(),
                    Some(snapshot),
                ),
                ProvNodeId::default(),
            );
            game.record_turn_history_event(&event);
        }

        let library_stable = game
            .object(from_library)
            .expect("library-origin object should exist")
            .stable_id;
        let hand_stable = game
            .object(from_hand)
            .expect("hand-origin object should exist")
            .stable_id;
        assert!(
            game.turn_store
                .turn_history
                .object_was_put_into_graveyard_from_zone_this_turn(library_stable, Zone::Library)
        );
        assert!(
            !game
                .turn_store
                .turn_history
                .object_was_put_into_graveyard_from_zone_this_turn(hand_stable, Zone::Library)
        );
    }

    #[test]
    fn graveyard_incarnation_history_does_not_reuse_an_earlier_origin() {
        for origin in [Zone::Battlefield, Zone::Library] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let card = CardDefinitionBuilder::new(CardId::new(), "History Creature")
                .card_types(vec![CardType::Creature])
                .build();
            let original = game.create_object_from_definition(&card, alice, origin);
            let stable = game.object(original).unwrap().stable_id;
            let first = game
                .move_object_by_effect(original, Zone::Graveyard)
                .unwrap();
            assert!(
                game.turn_store
                    .turn_history
                    .graveyard_incarnation_entered_this_turn(first, Some(origin))
            );
            let hand = game.move_object_by_effect(first, Zone::Hand).unwrap();
            let second = game.move_object_by_effect(hand, Zone::Graveyard).unwrap();
            assert_ne!(first, second);
            assert!(
                !game
                    .turn_store
                    .turn_history
                    .graveyard_incarnation_entered_this_turn(second, Some(origin))
            );
            assert!(
                game.turn_store
                    .turn_history
                    .graveyard_incarnation_entered_this_turn(second, Some(Zone::Hand))
            );
            assert!(
                game.turn_store
                    .turn_history
                    .graveyard_incarnation_entered_this_turn(second, None)
            );
            assert!(
                game.turn_store
                    .turn_history
                    .object_was_put_into_graveyard_from_zone_this_turn(stable, origin),
                "historical stable-ID facts remain true after the card leaves"
            );
            game.take_pending_trigger_events();
            game.turn_store.turn_history.clear_for_new_turn();
            assert!(
                !game
                    .turn_store
                    .turn_history
                    .graveyard_incarnation_entered_this_turn(second, None)
            );
        }
    }

    #[test]
    fn source_damage_history_count_uses_exact_incarnation_even_when_snapshots_share_card_identity()
    {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let creature = CardDefinitionBuilder::new(CardId::new(), "History Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let dealer = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let source_snapshot =
            ObjectSnapshot::from_object(game.object(source).expect("source should exist"), &game);
        let other_snapshot =
            ObjectSnapshot::from_object(game.object(other).expect("other should exist"), &game);

        for event in [
            DamageEvent::with_cause(
                dealer,
                DamageTarget::Object(source),
                2,
                false,
                EventCause::effect(),
            )
            .with_target_snapshot(source_snapshot.clone()),
            DamageEvent::with_cause(
                dealer,
                DamageTarget::Object(ObjectId::from_raw(u64::MAX - 1)),
                3,
                false,
                EventCause::effect(),
            )
            .with_target_snapshot(source_snapshot.clone()),
            DamageEvent::with_cause(
                dealer,
                DamageTarget::Object(other),
                7,
                false,
                EventCause::effect(),
            )
            .with_target_snapshot(other_snapshot),
        ] {
            let event = TriggerEvent::new_with_provenance(event, ProvNodeId::default());
            game.record_turn_history_event(&event);
        }

        let mut ctx = crate::filter::FilterContext::new(alice);
        ctx.source_snapshot = Some(source_snapshot);
        assert_eq!(
            resolve_turn_history_count(&game, &TurnHistoryCount::DamageDealtToSource, &ctx, None,),
            2
        );
        let graveyard = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let returned = game
            .move_object_by_effect(graveyard, Zone::Battlefield)
            .unwrap();
        assert_eq!(
            resolve_turn_history_count(&game, &TurnHistoryCount::DamageDealtToSource, &ctx, None),
            2,
            "the old source snapshot still names the departed incarnation"
        );
        let returned_ctx = game.filter_context_for(alice, Some(returned));
        assert_eq!(
            resolve_turn_history_count(
                &game,
                &TurnHistoryCount::DamageDealtToSource,
                &returned_ctx,
                None
            ),
            0,
            "the returned card is a new source with no damage history"
        );
    }
}

/// Whether a last-known snapshot had `subtype`, counting changeling: a
/// creature with changeling is every creature type (CR 702.73a).
fn snapshot_had_subtype(snapshot: &ObjectSnapshot, subtype: Subtype) -> bool {
    snapshot.subtypes.contains(&subtype)
        || (subtype.is_creature_type()
            && snapshot.card_types.contains(&CardType::Creature)
            && snapshot.has_static_ability_id(crate::static_abilities::StaticAbilityId::Changeling))
}

#[cfg(test)]
mod passive_blocked_history_tests {
    use super::*;
    #[test]
    fn passive_history_uses_attacker_identity_and_expires_at_turn_boundary() {
        let mut history = TurnHistory::default();
        let attacker = ObjectId::from_raw(101);
        let blocker = ObjectId::from_raw(102);
        let event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(blocker, attacker),
            ProvNodeId::default(),
        );
        history.record_event(&event, None, None);
        assert!(history.creature_was_blocked_this_turn(attacker));
        assert!(!history.creature_was_blocked_this_turn(blocker));
        assert!(history.creature_blocked_this_turn(blocker));
        assert!(
            !history.creature_was_blocked_this_turn(ObjectId::from_raw(103)),
            "new incarnation has no earlier block"
        );
        history.clear_for_new_turn();
        assert!(!history.creature_was_blocked_this_turn(attacker));
        let event = TriggerEvent::new_with_provenance(
            crate::events::CreatureBecameBlockedEvent::new(attacker, 0),
            ProvNodeId::default(),
        );
        history.record_event(&event, None, None);
        assert!(
            history.creature_was_blocked_this_turn(attacker),
            "an effect can make an attacker blocked without a blocker"
        );
    }
}
