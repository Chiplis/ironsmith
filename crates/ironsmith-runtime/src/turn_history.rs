use std::collections::{HashMap, HashSet};

use crate::color::ColorSet;
use crate::events::EnterBattlefieldEvent;
use crate::events::combat::{CreatureBecameBlockedEvent, CreatureBlockedEvent};
use crate::events::other::{
    CardDiscardedEvent, CardsDrawnEvent, ControlChangedEvent, KeywordActionEvent,
    KeywordActionKind, SearchLibraryEvent,
};
use crate::events::permanents::SacrificeEvent;
use crate::events::spells::SpellCastEvent;
use crate::events::zones::ZoneChangeEvent;
use crate::events::{DamageEvent, EventKind, LifeGainEvent, LifeLossEvent};
use crate::game_state::TurnCounterTracker;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::provenance::{ProvNodeId, ProvenanceGraph};
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::triggers::TriggerIdentity;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// One ingested trigger/event observation for the current turn.
#[derive(Debug, Clone)]
pub struct TurnEventRecord {
    pub event: TriggerEvent,
    pub object_snapshot: Option<ObjectSnapshot>,
    pub source_snapshot: Option<ObjectSnapshot>,
}

/// Unified owner for turn-scoped bookkeeping and history.
#[derive(Debug, Clone, Default)]
pub struct TurnHistory {
    pub activated_abilities_this_turn: HashSet<(ObjectId, usize)>,
    pub activated_abilities_resolved_this_turn: HashMap<(ObjectId, usize), u32>,
    pub chosen_modes_by_ability_this_turn: HashMap<(ObjectId, usize), HashSet<usize>>,
    pub triggers_fired_this_turn: HashMap<(ObjectId, TriggerIdentity), u32>,
    pub triggered_abilities_resolved_this_turn: HashMap<(ObjectId, TriggerIdentity), u32>,
    pub turn_counters: TurnCounterTracker,
    pub foretell_actions_this_turn: HashSet<PlayerId>,
    pub mana_spent_to_cast_spells_this_turn: HashMap<PlayerId, u32>,
    pub players_attacked_this_turn: HashSet<PlayerId>,
    pub players_tapped_land_for_mana_this_turn: HashSet<PlayerId>,
    pub creatures_attacked_this_turn: HashSet<ObjectId>,
    pub creature_attack_counts_this_turn: HashMap<ObjectId, u32>,
    pub crewed_this_turn: HashMap<ObjectId, Vec<ObjectId>>,
    pub saddled_this_turn: HashMap<ObjectId, Vec<ObjectId>>,
    pub event_records: Vec<TurnEventRecord>,
    pub staged_event_records: Vec<TurnEventRecord>,
}

impl TurnHistory {
    pub fn clear_for_new_turn(&mut self) -> u32 {
        let spells_cast_last_turn_total = self.total_spells_cast_this_turn();

        self.activated_abilities_this_turn.clear();
        self.activated_abilities_resolved_this_turn.clear();
        self.chosen_modes_by_ability_this_turn.clear();
        self.triggers_fired_this_turn.clear();
        self.triggered_abilities_resolved_this_turn.clear();
        self.turn_counters.clear();
        self.foretell_actions_this_turn.clear();
        self.mana_spent_to_cast_spells_this_turn.clear();
        self.players_attacked_this_turn.clear();
        self.players_tapped_land_for_mana_this_turn.clear();
        self.creatures_attacked_this_turn.clear();
        self.creature_attack_counts_this_turn.clear();
        self.crewed_this_turn.clear();
        self.saddled_this_turn.clear();
        self.event_records.clear();
        self.staged_event_records.clear();

        spells_cast_last_turn_total
    }

    fn projected_records(&self) -> impl DoubleEndedIterator<Item = &TurnEventRecord> {
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

    pub fn object_was_put_into_graveyard_from_battlefield_this_turn(
        &self,
        stable_id: StableId,
    ) -> bool {
        self.projected_records().any(|record| {
            record
                .event
                .downcast::<ZoneChangeEvent>()
                .is_some_and(|event| {
                    event.from == Zone::Battlefield
                        && event.to == Zone::Graveyard
                        && record
                            .object_snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.stable_id == stable_id)
                })
        })
    }

    pub fn player_was_dealt_damage_by_creature_this_turn(&self, player: PlayerId) -> bool {
        self.total_creature_damage_to_player(player) > 0
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
                || record
                    .event
                    .downcast::<CreatureBecameBlockedEvent>()
                    .is_some_and(|event| event.attacker == creature)
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
