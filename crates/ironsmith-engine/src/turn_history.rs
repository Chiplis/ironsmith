use std::collections::{HashMap, HashSet};

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

/// Unified owner for turn-scoped bookkeeping and history.
#[derive(Debug, Clone, Default)]
pub struct TurnHistory {
    /// Per-player snapshot taken before the untap step begins.
    pub untapped_lands_at_turn_start: HashMap<PlayerId, u32>,
    pub activated_abilities_this_turn: HashSet<(ObjectId, usize)>,
    pub loyalty_abilities_activated_this_turn: HashSet<ObjectId>,
    pub activated_abilities_resolved_this_turn: HashMap<(ObjectId, usize), u32>,
    pub chosen_modes_by_ability_this_turn: HashMap<(ObjectId, usize), HashSet<usize>>,
    pub triggers_fired_this_turn: HashMap<(ObjectId, TriggerIdentity), u32>,
    pub triggered_abilities_resolved_this_turn: HashMap<(ObjectId, TriggerIdentity), u32>,
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
    pub die_roll_result_adjustments_this_turn: HashSet<(ObjectId, StaticAbilityInstanceId)>,
    /// Source/player pairs for attached-object rule restrictions that player
    /// has paid to ignore until the turn ends.
    pub players_ignoring_attached_static_restrictions_this_turn: HashSet<(ObjectId, PlayerId)>,
    /// Source/player pairs for source-wide rule restrictions that the player
    /// has paid to ignore until the turn ends.
    pub players_ignoring_source_static_effects_this_turn: HashSet<(ObjectId, PlayerId)>,
    pub creatures_attacked_this_turn: HashSet<ObjectId>,
    pub creatures_attacked_battles_this_turn: HashSet<ObjectId>,
    pub creature_attack_counts_this_turn: HashMap<ObjectId, u32>,
    pub crewed_this_turn: HashMap<ObjectId, Vec<ObjectId>>,
    pub saddled_this_turn: HashMap<ObjectId, Vec<ObjectId>>,
    pub spell_warped_this_turn: bool,
    pub event_records: Vec<TurnEventRecord>,
    pub staged_event_records: Vec<TurnEventRecord>,
}

impl TurnHistory {
    pub fn clear_for_new_turn(&mut self) -> u32 {
        let spells_cast_last_turn_total = self.total_spells_cast_this_turn();

        self.activated_abilities_this_turn.clear();
        self.loyalty_abilities_activated_this_turn.clear();
        self.activated_abilities_resolved_this_turn.clear();
        self.chosen_modes_by_ability_this_turn.clear();
        self.triggers_fired_this_turn.clear();
        self.triggered_abilities_resolved_this_turn.clear();
        self.turn_counters.clear();
        self.foretell_actions_this_turn.clear();
        self.mana_spent_to_cast_spells_this_turn.clear();
        self.players_attacked_this_turn.clear();
        self.players_tapped_land_for_mana_this_turn.clear();
        self.untapped_lands_at_turn_start.clear();
        self.player_counter_locks_this_turn.clear();
        self.die_rolls_this_turn.clear();
        self.die_roll_result_adjustments_this_turn.clear();
        self.players_ignoring_attached_static_restrictions_this_turn
            .clear();
        self.players_ignoring_source_static_effects_this_turn
            .clear();
        self.creatures_attacked_this_turn.clear();
        self.creatures_attacked_battles_this_turn.clear();
        self.creature_attack_counts_this_turn.clear();
        self.crewed_this_turn.clear();
        self.saddled_this_turn.clear();
        self.spell_warped_this_turn = false;
        self.event_records.clear();
        self.staged_event_records.clear();

        spells_cast_last_turn_total
    }

    pub(crate) fn projected_records(&self) -> impl DoubleEndedIterator<Item = &TurnEventRecord> {
        self.event_records
            .iter()
            .chain(self.staged_event_records.iter())
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
        self.remove_staged_event(event.provenance());
        self.turn_counters.increment_event_kind(event.kind());
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

    pub fn total_creatures_died_this_turn(&self) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.is_dies())
            .filter(|event| {
                event
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.card_types.contains(&CardType::Creature))
            })
            .count() as u32
    }

    pub fn creatures_died_under_controller(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.is_dies())
            .filter(|event| {
                event.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.controller == player
                        && snapshot.card_types.contains(&CardType::Creature)
                })
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
            .map(|player| self.die_rolls_this_turn.get(player).map_or(0, Vec::len) as u32)
            .max()
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
        self.projected_records()
            .map(|record| {
                if let Some(zone_change) = record.event.downcast::<ZoneChangeEvent>()
                    && zone_change.is_etb()
                    && zone_change.objects.len() > 1
                    && !zone_change.snapshots().is_empty()
                {
                    return zone_change
                        .snapshots()
                        .iter()
                        .filter(|snapshot| {
                            snapshot.controller == player
                                && snapshot.card_types.contains(&CardType::Creature)
                        })
                        .count() as u32;
                }

                let is_entry = record.event.downcast::<EnterBattlefieldEvent>().is_some()
                    || record
                        .event
                        .downcast::<ZoneChangeEvent>()
                        .is_some_and(|event| event.is_etb());
                (is_entry
                    && record.object_snapshot.as_ref().is_some_and(|snapshot| {
                        snapshot.controller == player
                            && snapshot.card_types.contains(&CardType::Creature)
                    })) as u32
            })
            .sum()
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
        stable_id: StableId,
        player: PlayerId,
    ) -> bool {
        let discarded = self
            .projected_records()
            .filter_map(|record| record.event.downcast::<CardDiscardedEvent>())
            .filter(|event| event.player == player)
            .any(|event| {
                event.card == object_id
                    || event
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                    || event
                        .batch_snapshots
                        .iter()
                        .any(|snapshot| snapshot.stable_id == stable_id)
                    || event.batch_cards.contains(&object_id)
            });
        if discarded {
            return true;
        }

        self.projected_records()
            .filter_map(|record| {
                record
                    .event
                    .downcast::<KeywordActionEvent>()
                    .map(|event| (record, event))
            })
            .filter(|(_, event)| event.action == KeywordActionKind::Cycle && event.player == player)
            .any(|(record, event)| {
                event.source == object_id
                    || event
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                    || record
                        .source_snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                    || event.object_tags.values().any(|snapshots| {
                        snapshots
                            .iter()
                            .any(|snapshot| snapshot.stable_id == stable_id)
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
        self.projected_records().any(|record| {
            record.event.downcast::<DamageEvent>().is_some_and(|event| {
                event.amount > 0
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
                        && snapshot.subtypes.contains(&subtype)
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
                        && (snapshot.subtypes.contains(&subtype) || snapshot.is_commander)
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
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.from == Zone::Battlefield)
            .filter(|event| {
                event
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.controller == player)
            })
            .count() as u32
    }

    pub fn permanents_left_battlefield_this_turn(&self) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.from == Zone::Battlefield)
            .count() as u32
    }

    pub fn nonland_permanents_left_battlefield_this_turn(&self) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.from == Zone::Battlefield)
            .filter(|event| {
                event
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| !snapshot.card_types.contains(&CardType::Land))
            })
            .count() as u32
    }

    pub fn spell_was_warped_this_turn(&self) -> bool {
        self.spell_warped_this_turn
    }

    pub fn creatures_left_battlefield_under_controller(&self, player: PlayerId) -> u32 {
        self.projected_records()
            .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
            .filter(|event| event.from == Zone::Battlefield)
            .filter(|event| {
                event.snapshot.as_ref().is_some_and(|snapshot| {
                    snapshot.controller == player
                        && snapshot.card_types.contains(&CardType::Creature)
                })
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
            history
                .projected_records()
                .filter_map(|record| {
                    if let Some(event) = record.event.downcast::<ZoneChangeEvent>() {
                        if !event.is_etb() {
                            return None;
                        }
                        if !event.snapshots().is_empty() {
                            return Some(event.snapshots().to_vec());
                        }
                        return record
                            .object_snapshot
                            .clone()
                            .map(|snapshot| vec![snapshot]);
                    }

                    if record.event.downcast::<EnterBattlefieldEvent>().is_some() {
                        return record
                            .object_snapshot
                            .clone()
                            .map(|snapshot| vec![snapshot]);
                    }

                    None
                })
                .flatten()
                .filter(|snapshot| historical_filter.matches_snapshot(snapshot, filter_ctx, game))
                .count() as i32
        }
        TurnHistoryCount::TokensCreated(player_filter) => history
            .projected_records()
            .filter_map(|record| record.event.downcast::<CreateTokensEvent>())
            .filter(|event| player_filter.matches_player(event.controller, filter_ctx))
            .map(|event| {
                event.count.saturating_add(
                    event
                        .additional_tokens
                        .iter()
                        .map(|(_, count)| *count)
                        .sum::<u32>(),
                )
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
        TurnHistoryCount::CountersPutOn {
            counter_type,
            filter,
        } => history
            .projected_records()
            .filter_map(|record| {
                let event = record.event.downcast::<CounterPlacedEvent>()?;
                let snapshot = record.object_snapshot.as_ref()?;
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
        TurnHistoryCount::DamageDealtToSource => {
            let source_object = filter_ctx.source;
            let source_stable_id = source_object
                .and_then(|source| game.object(source).map(|object| object.stable_id))
                .or_else(|| {
                    filter_ctx
                        .source_snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.stable_id)
                });

            history
                .projected_records()
                .filter_map(|record| record.event.downcast::<DamageEvent>())
                .filter(|event| event.amount > 0)
                .filter(|event| match event.target {
                    DamageTarget::Object(target) => {
                        source_object == Some(target)
                            || source_stable_id.is_some_and(|stable_id| {
                                event
                                    .target_snapshot
                                    .as_ref()
                                    .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                            })
                    }
                    DamageTarget::Player(_) => false,
                })
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
    fn source_damage_history_count_follows_stable_identity_after_zone_change() {
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
            5
        );
    }
}
