use super::bounded_cache::BoundedCache;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::Serialize;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

use ironsmith::cards::{CardDefinition, CardRegistry};
use ironsmith::combat_state::AttackTarget;
use ironsmith::decision::GameResult;
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_state::{
    GameState, Target, UiBattlefieldTransition, UiBattlefieldTransitionKind,
};
use ironsmith::ids::{ObjectId, PlayerId, StableId};
use ironsmith::object::AttachmentTarget;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::types::{CardType, Subtype};
use ironsmith::zone::Zone;

use super::{
    ActiveViewedCards, CryptoRequirementView, DecisionView, GameOverView, ManaPaymentView,
    hidden_object_label, object_visible_to_perspective,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum BattlefieldLane {
    Artifacts,
    Lands,
    Creatures,
    Enchantments,
    Planeswalkers,
    Battles,
    Other,
}

impl BattlefieldLane {
    fn as_str(self) -> &'static str {
        match self {
            BattlefieldLane::Artifacts => "artifacts",
            BattlefieldLane::Lands => "lands",
            BattlefieldLane::Creatures => "creatures",
            BattlefieldLane::Enchantments => "enchantments",
            BattlefieldLane::Planeswalkers => "planeswalkers",
            BattlefieldLane::Battles => "battles",
            BattlefieldLane::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct BattlefieldGroupKey {
    lane: BattlefieldLane,
    name: String,
    tapped: bool,
    characteristic_signature: String,
    counter_signature: String,
    token: bool,
    force_single_object: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PermanentObjectViewCacheKey {
    dependency_revision: u64,
    object_id: ObjectId,
    object_revision: u64,
    continuous_revision: u64,
    turn_number: u32,
    active_player: PlayerId,
    priority_player: Option<PlayerId>,
    phase: u8,
    step: Option<u8>,
    tapped: bool,
    flipped: bool,
    face_down: bool,
    manifested: bool,
    phased_out: bool,
    counter_signature: String,
}

#[derive(Debug, Clone)]
struct PermanentObjectView {
    characteristics: Option<Arc<ironsmith::continuous::CalculatedCharacteristics>>,
    id: u64,
    stable_id: u64,
    name: String,
    token: bool,
    tapped: bool,
    lane: BattlefieldLane,
    characteristic_signature: String,
    counter_signature: String,
    mana_cost: Option<String>,
    oracle_text: String,
    power_toughness: Option<String>,
    counters: Vec<CounterSnapshot>,
}

const SNAPSHOT_OBJECT_VIEW_CACHE_LIMIT: usize = 8_192;

#[derive(Debug, Default)]
pub(super) struct SnapshotObjectViewCache {
    battlefield: RefCell<
        BoundedCache<
            PermanentObjectViewCacheKey,
            Arc<PermanentObjectView>,
            SNAPSHOT_OBJECT_VIEW_CACHE_LIMIT,
        >,
    >,
    dependency_clock: std::cell::Cell<u64>,
    dependency_revisions: RefCell<HashMap<ObjectId, u64>>,
    groups: RefCell<IncrementalBattlefieldGroups>,
    hands: RefCell<HashMap<PlayerId, IncrementalZoneCards<HandCardSnapshot>>>,
    zones: RefCell<HashMap<(PlayerId, Zone), IncrementalZoneCards<ZoneCardSnapshot>>>,
    looks: RefCell<HashMap<(PlayerId, Zone), IncrementalZoneCards<ViewedCardSnapshot>>>,
    combined_looks: RefCell<HashMap<PlayerId, CombinedLookCards>>,
    grant_sources: RefCell<IncrementalGrantSources>,
}

#[derive(Debug, Default)]
struct IncrementalGrantSources {
    cursor: Option<ironsmith::incremental::ChangeCursor>,
    sources: HashSet<ObjectId>,
}
impl IncrementalGrantSources {
    fn update(&mut self, game: &GameState) -> bool {
        let dirty = if let Some(changed) = self
            .cursor
            .as_ref()
            .and_then(|cursor| game.render_changes_since(cursor))
        {
            changed
        } else {
            self.sources.clear();
            game.objects_in_deterministic_order()
                .iter()
                .map(|object| object.id)
                .collect()
        };
        for id in dirty {
            let potential = game.object(id).is_some_and(|object|
                matches!(object.zone, Zone::Battlefield | Zone::Graveyard | Zone::Exile | Zone::Command)
                && object.abilities.iter().any(|ability| matches!(&ability.kind,
                    ironsmith::ability::AbilityKind::Static(ability) if ability.grant_spec().is_some())));
            if potential {
                self.sources.insert(id);
            } else {
                self.sources.remove(&id);
            }
        }
        self.cursor = Some(game.render_change_cursor());
        !self.sources.is_empty() || !game.effect_store.grant_registry.grants.is_empty()
    }
}

#[derive(Debug, Default)]
struct CombinedLookCards {
    top: Option<ViewedCardSnapshot>,
    hand: Arc<Vec<Arc<ViewedCardSnapshot>>>,
    exile: Arc<Vec<Arc<ViewedCardSnapshot>>>,
    output: Arc<Vec<Arc<ViewedCardSnapshot>>>,
}
fn viewed_card_snapshot(object: &ironsmith::object::Object) -> ViewedCardSnapshot {
    ViewedCardSnapshot {
        id: object.id.0,
        stable_id: object.stable_id.0.0,
        name: object.name.to_string(),
        oracle_text: object.compiled_card_text.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZoneRenderKey {
    perspective: PlayerId,
    revision: Option<(u64, u64)>,
    grant_inputs: Option<ironsmith::incremental::ChangeCursor>,
    auxiliary: Option<(
        ironsmith::incremental::ChangeCursor,
        (
            ironsmith::incremental::ChangeCursor,
            ironsmith::incremental::ChangeCursor,
        ),
    )>,
    turn: Option<(u32, PlayerId, Option<PlayerId>, u8, Option<u8>)>,
    viewed: Option<ActiveViewedCards>,
    visibility: u8,
}
impl ZoneRenderKey {
    fn new(
        game: &GameState,
        perspective: PlayerId,
        viewed: Option<&ActiveViewedCards>,
        visibility: u8,
    ) -> Self {
        Self {
            perspective,
            revision: Some(game.derived_view_revision()),
            grant_inputs: None,
            auxiliary: Some(game.zone_view_identity()),
            turn: Some((
                game.turn.turn_number,
                game.turn.active_player,
                game.turn.priority_player,
                game.turn.phase as u8,
                game.turn.step.map(|step| step as u8),
            )),
            viewed: viewed.cloned(),
            visibility,
        }
    }
    fn raw_card_fields(mut self) -> Self {
        self.revision = None;
        self.auxiliary = None;
        self.turn = None;
        self
    }
}

#[derive(Debug)]
struct IncrementalZoneCards<T> {
    order: ironsmith::zone_sequence::ZoneOrder,
    cursor: Option<ironsmith::incremental::ChangeCursor>,
    key: Option<ZoneRenderKey>,
    by_id: HashMap<ObjectId, Arc<T>>,
    labels: HashMap<ObjectId, u128>,
    ordered: std::collections::BTreeMap<u128, Arc<T>>,
    output: Arc<Vec<Arc<T>>>,
    rendered: usize,
}
impl<T> Default for IncrementalZoneCards<T> {
    fn default() -> Self {
        Self {
            order: Default::default(),
            cursor: None,
            key: None,
            by_id: HashMap::new(),
            labels: HashMap::new(),
            ordered: Default::default(),
            output: Arc::new(Vec::new()),
            rendered: 0,
        }
    }
}
impl<T: PartialEq> IncrementalZoneCards<T> {
    fn update(
        &mut self,
        game: &GameState,
        zone: &ironsmith::zone_sequence::ZoneSequence,
        key: ZoneRenderKey,
        reverse: bool,
        mut render: impl FnMut(ObjectId) -> Option<T>,
    ) -> Arc<Vec<Arc<T>>> {
        let membership = self.order.synchronize(zone);
        let changes = self
            .cursor
            .as_ref()
            .and_then(|cursor| game.render_changes_since(cursor));
        let rebuild = membership.is_none() || changes.is_none() || self.key.as_ref() != Some(&key);
        let mut output_changed = rebuild;
        let mut dirty = if rebuild {
            self.ordered.clear();
            self.labels.clear();
            self.by_id.retain(|id, _| self.order.label(*id).is_some());
            zone.iter().copied().collect::<Vec<_>>()
        } else {
            let mut dirty = changes.unwrap_or_default();
            dirty.extend(membership.unwrap_or_default());
            dirty
        };
        dirty.sort_unstable();
        dirty.dedup();
        for id in dirty {
            let old_label = self.labels.get(&id).copied();
            let new_label = self.order.label(id);
            let new = if new_label.is_some() {
                self.rendered += 1;
                render(id)
            } else {
                None
            };
            match new {
                Some(new) => {
                    let value = match self.by_id.get(&id) {
                        Some(old) if old.as_ref() == &new => old.clone(),
                        _ => Arc::new(new),
                    };
                    let label = new_label.expect("rendered zone member has an order label");
                    let unchanged = old_label == Some(label)
                        && self
                            .by_id
                            .get(&id)
                            .is_some_and(|old| Arc::ptr_eq(old, &value));
                    if !unchanged {
                        if let Some(old) = old_label {
                            self.ordered.remove(&old);
                        }
                        self.ordered.insert(label, value.clone());
                        self.labels.insert(id, label);
                        self.by_id.insert(id, value);
                        output_changed = true;
                    }
                }
                None => {
                    if let Some(label) = self.labels.remove(&id) {
                        self.ordered.remove(&label);
                        output_changed = true;
                    }
                    self.by_id.remove(&id);
                }
            }
        }
        if output_changed {
            let mut output: Vec<_> = self.ordered.values().cloned().collect();
            if reverse {
                output.reverse();
            }
            if self.output.len() != output.len()
                || !self
                    .output
                    .iter()
                    .zip(&output)
                    .all(|(a, b)| Arc::ptr_eq(a, b))
            {
                self.output = Arc::new(output);
            }
        }
        self.cursor = Some(game.render_change_cursor());
        self.key = Some(key);
        self.output.clone()
    }
}

#[derive(Debug, Default)]
struct IncrementalBattlefieldGroups {
    objects_updated: usize,
    groups_rebuilt: usize,
    order: ironsmith::zone_sequence::ZoneOrder,
    cursor: Option<ironsmith::incremental::ChangeCursor>,
    revision: Option<((u64, u64), u32, PlayerId, Option<PlayerId>, u8, Option<u8>)>,
    protected: HashSet<ObjectId>,
    dependencies: HashMap<ObjectId, HashSet<ObjectId>>,
    dependents: HashMap<ObjectId, HashSet<ObjectId>>,
    visibility_cursor: Option<ironsmith::incremental::ChangeCursor>,
    visibility_candidates: HashSet<ObjectId>,
    visibility: HashMap<ObjectId, (PlayerId, u8)>,
    membership: HashMap<ObjectId, (PlayerId, BattlefieldGroupKey, u128)>,
    members: HashMap<(PlayerId, BattlefieldGroupKey), std::collections::BTreeMap<u128, ObjectId>>,
    snapshots: HashMap<(PlayerId, BattlefieldGroupKey), Arc<PermanentSnapshot>>,
    player_outputs: HashMap<PlayerId, (Arc<Vec<Arc<PermanentSnapshot>>>, usize)>,
    empty: Arc<Vec<Arc<PermanentSnapshot>>>,
}

impl IncrementalBattlefieldGroups {
    fn update(
        &mut self,
        game: &GameState,
        protected: &HashSet<ObjectId>,
        views: &SnapshotObjectViewCache,
    ) {
        let membership = self.order.synchronize(&game.battlefield);
        let changed = self
            .cursor
            .as_ref()
            .and_then(|cursor| game.render_changes_since(cursor));
        let revision = (
            game.derived_view_revision(),
            game.turn.turn_number,
            game.turn.active_player,
            game.turn.priority_player,
            game.turn.phase as u8,
            game.turn.step.map(|step| step as u8),
        );
        let rebuild = membership.is_none() || changed.is_none() || self.revision != Some(revision);
        let mut dirty = if rebuild {
            self.membership.clear();
            self.members.clear();
            self.snapshots.clear();
            self.visibility_candidates.clear();
            self.visibility.clear();
            self.player_outputs.clear();
            self.dependencies.clear();
            self.dependents.clear();
            game.battlefield.iter().copied().collect::<Vec<_>>()
        } else {
            let mut dirty = changed.unwrap_or_default();
            dirty.extend(membership.unwrap_or_default());
            dirty.extend(self.protected.symmetric_difference(protected).copied());
            dirty
        };
        if !rebuild {
            let mut pending = dirty.clone();
            let mut seen: HashSet<_> = dirty.iter().copied().collect();
            while let Some(id) = pending.pop() {
                if let Some(dependents) = self.dependents.get(&id) {
                    for dependent in dependents.iter().copied() {
                        if seen.insert(dependent) {
                            dirty.push(dependent);
                            pending.push(dependent);
                        }
                    }
                }
            }
        } else {
            views
                .dependency_revisions
                .borrow_mut()
                .retain(|id, _| self.order.label(*id).is_some());
        }
        dirty.sort_unstable();
        dirty.dedup();
        let refresh_visibility = !dirty.is_empty()
            || self
                .visibility_cursor
                .as_ref()
                .and_then(|cursor| game.object_changes_since(cursor))
                .is_none_or(|changes| !changes.is_empty());
        self.visibility_cursor = Some(game.object_change_cursor());
        self.objects_updated += dirty.len();
        game.prewarm_calculated_characteristics(&dirty);
        let mut affected = HashSet::new();
        for id in dirty {
            views.invalidate_dependency(id);
            if let Some(previous) = self.dependencies.remove(&id) {
                for dependency in previous {
                    if let Some(dependents) = self.dependents.get_mut(&dependency) {
                        dependents.remove(&id);
                        if dependents.is_empty() {
                            self.dependents.remove(&dependency);
                        }
                    }
                }
            }
            self.visibility_candidates.remove(&id);
            self.visibility.remove(&id);
            if let Some((player, key, label)) = self.membership.remove(&id) {
                let group = (player, key);
                if let Some(members) = self.members.get_mut(&group) {
                    members.remove(&label);
                }
                affected.insert(group);
            }
            let Some(label) = self.order.label(id) else {
                continue;
            };
            let Some(object) = game.object(id) else {
                continue;
            };
            let player = game.current_controller(id).unwrap_or(object.owner);
            let dependencies: HashSet<_> = object
                .attachments
                .iter()
                .copied()
                .chain(object.attached_to.and_then(|target| target.object_id()))
                .collect();
            for dependency in dependencies.iter().copied() {
                self.dependents.entry(dependency).or_default().insert(id);
            }
            if !dependencies.is_empty() {
                self.dependencies.insert(id, dependencies);
            }

            let view = views.battlefield_view(game, object);
            if view.characteristics.as_ref().is_some_and(|chars| {
                chars.static_abilities.iter().any(|ability| {
                    matches!(
                        ability.id(),
                        StaticAbilityId::LookAtTopCardOfLibrary
                            | StaticAbilityId::AllPlayersLookAtYourTopLibraryCard
                            | StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries
                            | StaticAbilityId::OpponentsPlayWithHandsRevealed
                    )
                })
            }) {
                self.visibility_candidates.insert(id);
            }

            let key = BattlefieldGroupKey {
                lane: view.lane,
                name: view.name.clone(),
                tapped: view.tapped,
                characteristic_signature: view.characteristic_signature.clone(),
                counter_signature: view.counter_signature.clone(),
                token: view.token,
                force_single_object: protected.contains(&id).then_some(id.0),
            };
            self.membership.insert(id, (player, key.clone(), label));
            let group = (player, key);
            self.members
                .entry(group.clone())
                .or_default()
                .insert(label, id);
            affected.insert(group);
        }
        if refresh_visibility {
            for id in self.visibility_candidates.iter().copied() {
                let flags = [
                    StaticAbilityId::LookAtTopCardOfLibrary,
                    StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
                    StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries,
                    StaticAbilityId::OpponentsPlayWithHandsRevealed,
                ]
                .into_iter()
                .enumerate()
                .fold(0u8, |flags, (bit, ability)| {
                    flags | (u8::from(game.object_has_static_ability_id(id, ability)) << bit)
                });
                if let Some(object) = game.object(id) {
                    self.visibility.insert(
                        id,
                        (game.current_controller(id).unwrap_or(object.owner), flags),
                    );
                }
            }
        }
        let dirty_players: HashSet<_> = affected.iter().map(|(player, _)| *player).collect();
        for group in affected {
            if self
                .members
                .get(&group)
                .is_none_or(|members| members.is_empty())
            {
                self.members.remove(&group);
                self.snapshots.remove(&group);
                continue;
            }
            let members = &self.members[&group];
            let (snapshots, _) =
                grouped_battlefield_for_ids(game, members.values().copied(), protected, views);
            self.groups_rebuilt += 1;
            debug_assert_eq!(snapshots.len(), 1);
            self.snapshots.insert(
                group,
                Arc::new(snapshots.into_iter().next().expect("nonempty group")),
            );
        }
        for player in dirty_players {
            let output = self.collect_for_player(player);
            self.player_outputs.insert(player, output);
        }
        self.player_outputs
            .retain(|player, _| game.players.iter().any(|current| current.id == *player));
        self.cursor = Some(game.render_change_cursor());
        self.revision = Some(revision);
        self.protected.clone_from(protected);
    }

    fn visibility_for_player(
        &self,
        game: &GameState,
        perspective: PlayerId,
        player: PlayerId,
    ) -> (bool, bool) {
        let own = perspective == player || game.controlling_player_for(player) == perspective;
        let top = self.visibility.values().any(|(controller, flags)| {
            flags & 4 != 0 || (*controller == player && (flags & 2 != 0 || (own && flags & 1 != 0)))
        });
        let hand = self
            .visibility
            .values()
            .any(|(controller, flags)| *controller != player && flags & 8 != 0);
        (top, hand)
    }

    fn for_player(&self, player: PlayerId) -> (Arc<Vec<Arc<PermanentSnapshot>>>, usize) {
        self.player_outputs
            .get(&player)
            .cloned()
            .unwrap_or_else(|| (self.empty.clone(), 0))
    }

    fn collect_for_player(&self, player: PlayerId) -> (Arc<Vec<Arc<PermanentSnapshot>>>, usize) {
        let mut groups: Vec<_> = self
            .snapshots
            .iter()
            .filter(|((owner, _), _)| *owner == player)
            .collect();
        groups.sort_unstable_by(|(left, _), (right, _)| {
            let a = &left.1;
            let b = &right.1;
            a.lane
                .cmp(&b.lane)
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.tapped.cmp(&b.tapped))
                .then_with(|| a.token.cmp(&b.token))
                .then_with(|| {
                    self.members[*left]
                        .first_key_value()
                        .map(|(_, id)| *id)
                        .cmp(&self.members[*right].first_key_value().map(|(_, id)| *id))
                })
        });
        let total = groups.iter().map(|(_, snapshot)| snapshot.count).sum();
        (
            Arc::new(
                groups
                    .into_iter()
                    .map(|(_, snapshot)| snapshot.clone())
                    .collect(),
            ),
            total,
        )
    }
}

impl SnapshotObjectViewCache {
    fn invalidate_dependency(&self, id: ObjectId) {
        let next = self
            .dependency_clock
            .get()
            .checked_add(1)
            .expect("snapshot dependency generation exhausted");
        self.dependency_clock.set(next);
        self.dependency_revisions.borrow_mut().insert(id, next);
    }

    fn persistent_look_cards(
        &self,
        game: &GameState,
        owner: PlayerId,
        perspective: PlayerId,
        library_top: bool,
        hand_revealed: bool,
    ) -> Arc<Vec<Arc<ViewedCardSnapshot>>> {
        let Some(player) = game.players.iter().find(|player| player.id == owner) else {
            return Arc::new(Vec::new());
        };
        let top = library_top
            .then(|| {
                player
                    .library
                    .last()
                    .and_then(|id| game.object(*id))
                    .map(viewed_card_snapshot)
            })
            .flatten();
        let mut looks = self.looks.borrow_mut();
        let exile = looks.entry((owner, Zone::Exile)).or_default().update(
            game,
            &game.exile,
            ZoneRenderKey::new(game, perspective, None, 0),
            false,
            |id| {
                let object = game.object(id)?;
                (object.owner == owner
                    && game.is_face_down(id)
                    && game.can_player_look_at_face_down_exiled_card(id, perspective))
                .then(|| viewed_card_snapshot(object))
            },
        );
        let hand = looks.entry((owner, Zone::Hand)).or_default().update(
            game,
            &player.hand,
            ZoneRenderKey::new(game, perspective, None, u8::from(hand_revealed)).raw_card_fields(),
            false,
            |id| {
                if !hand_revealed {
                    return None;
                }
                game.object(id).map(viewed_card_snapshot)
            },
        );
        let mut combined = self.combined_looks.borrow_mut();
        let cached = combined.entry(owner).or_default();
        if cached.top != top
            || !Arc::ptr_eq(&cached.hand, &hand)
            || !Arc::ptr_eq(&cached.exile, &exile)
        {
            cached.output = Arc::new(
                top.iter()
                    .cloned()
                    .map(Arc::new)
                    .chain(exile.iter().cloned())
                    .chain(hand.iter().cloned())
                    .collect(),
            );
            cached.top = top;
            cached.hand = hand;
            cached.exile = exile;
        }
        cached.output.clone()
    }

    fn hand_cards(
        &self,
        game: &GameState,
        owner: PlayerId,
        perspective: PlayerId,
        viewed: Option<&ActiveViewedCards>,
        visibility: u8,
    ) -> Arc<Vec<Arc<HandCardSnapshot>>> {
        let Some(player) = game.players.iter().find(|player| player.id == owner) else {
            return Arc::new(Vec::new());
        };
        let key = ZoneRenderKey::new(game, perspective, viewed, visibility).raw_card_fields();
        self.hands.borrow_mut().entry(owner).or_default().update(
            game,
            &player.hand,
            key,
            true,
            |id| {
                if visibility != 2
                    && !(visibility == 1
                        && viewed.is_some_and(|view| view.contains_object(game, id)))
                {
                    return None;
                }
                let object = game.object(id)?;
                Some(HandCardSnapshot {
                    id: object.id.0,
                    stable_id: object.stable_id.0.0,
                    name: object.name.to_string(),
                    mana_cost: object.mana_cost.as_ref().map(|cost| cost.to_oracle()),
                    oracle_text: object.compiled_card_text.to_string(),
                    power_toughness: match (object.power(), object.toughness()) {
                        (Some(power), Some(toughness)) => Some(format!("{power}/{toughness}")),
                        _ => None,
                    },
                    loyalty: object.loyalty(),
                    defense: object.defense(),
                    card_types: object
                        .card_types
                        .iter()
                        .map(|kind| kind.name().to_string())
                        .collect(),
                })
            },
        )
    }

    fn zone_cards(
        &self,
        game: &GameState,
        owner: PlayerId,
        perspective: PlayerId,
        zone: Zone,
        viewed: Option<&ActiveViewedCards>,
        grants: &std::cell::OnceCell<Vec<ironsmith::grant_registry::Grant>>,
        potential_grants: bool,
    ) -> Arc<Vec<Arc<ZoneCardSnapshot>>> {
        let Some(player) = game.players.iter().find(|player| player.id == owner) else {
            return Arc::new(Vec::new());
        };
        let (ids, filter_owner) = match zone {
            Zone::Graveyard => (&player.graveyard, false),
            Zone::Exile => (&game.exile, true),
            Zone::Command => (&game.command_zone, true),
            Zone::Ante => (&game.ante, true),
            Zone::OutsideGame => (&player.sideboard, false),
            _ => unreachable!("zone card projection has a public-zone or sideboard input"),
        };
        let visible = zone != Zone::OutsideGame || owner == perspective;
        let mut key = ZoneRenderKey::new(game, perspective, viewed, u8::from(visible));
        if potential_grants {
            key.grant_inputs = Some(game.object_change_cursor());
        } else {
            key.revision = None;
            if !matches!(zone, Zone::Exile | Zone::Command) {
                key.auxiliary = None;
                key.turn = None;
            }
        }
        self.zones
            .borrow_mut()
            .entry((owner, zone))
            .or_default()
            .update(game, ids, key, zone != Zone::Ante, |id| {
                if !visible {
                    return None;
                }
                let object = game.object(id)?;
                if filter_owner && object.owner != owner {
                    return None;
                }
                Some(build_zone_card_snapshot_with_grants(
                    game,
                    perspective,
                    viewed,
                    object,
                    zone,
                    Some(grants),
                ))
            })
    }

    fn battlefield_view(
        &self,
        game: &GameState,
        obj: &ironsmith::object::Object,
    ) -> Arc<PermanentObjectView> {
        let tapped = game.is_tapped(obj.id);
        let counter_signature = counter_signature_for_group(obj);
        let key = PermanentObjectViewCacheKey {
            dependency_revision: self
                .dependency_revisions
                .borrow()
                .get(&obj.id)
                .copied()
                .unwrap_or(0),
            object_id: obj.id,
            object_revision: obj.last_modified,
            continuous_revision: game.effect_store.continuous_effects.revision(),
            turn_number: game.turn.turn_number,
            active_player: game.turn.active_player,
            priority_player: game.turn.priority_player,
            phase: game.turn.phase as u8,
            step: game.turn.step.map(|step| step as u8),
            tapped,
            flipped: game.is_flipped(obj.id),
            face_down: game.is_face_down(obj.id),
            manifested: game.is_manifested(obj.id),
            phased_out: game.is_phased_out(obj.id),
            counter_signature,
        };

        let current = game.calculated_characteristics_arc(obj.id);
        if let Some(view) = self.battlefield.borrow_mut().get(&key).cloned()
            && match (&view.characteristics, &current) {
                (Some(before), Some(after)) => Arc::ptr_eq(before, after),
                (None, None) => true,
                _ => false,
            }
        {
            return view;
        }
        let current_card_types = current
            .as_ref()
            .map(|chars| chars.card_types.as_slice())
            .unwrap_or(&obj.card_types);
        let name = current
            .as_ref()
            .map(|chars| chars.name.to_owned_string())
            .unwrap_or_else(|| obj.name.to_string());
        let power_toughness = {
            let p = current
                .as_ref()
                .and_then(|chars| chars.power)
                .or_else(|| obj.power());
            let t = current
                .as_ref()
                .and_then(|chars| chars.toughness)
                .or_else(|| obj.toughness());
            match (p, t) {
                (Some(power), Some(toughness)) => Some(format!("{power}/{toughness}")),
                _ => None,
            }
        };
        let oracle_text = current
            .as_ref()
            .map(|chars| chars.compiled_card_text.to_string())
            .unwrap_or_else(|| obj.compiled_card_text.to_string());
        let counter_signature = key.counter_signature.clone();
        let view = Arc::new(PermanentObjectView {
            characteristics: current.clone(),
            id: obj.id.0,
            stable_id: obj.stable_id.0.0,
            name,
            token: matches!(obj.kind, ironsmith::object::ObjectKind::Token),
            tapped,
            lane: battlefield_lane_for_card_types(current_card_types),
            characteristic_signature: object_characteristic_signature(game, obj, true),
            counter_signature,
            mana_cost: obj.mana_cost.as_ref().map(|mc| mc.to_oracle()),
            oracle_text,
            power_toughness,
            counters: counter_snapshots_for_object(obj),
        });

        let mut cache = self.battlefield.borrow_mut();
        cache.insert(key, view.clone());
        view
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SnapshotEncodedSubtreeKind {
    Permanent,
    HandCard,
    ZoneCard,
    ViewedCard,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SnapshotEncodedSubtreeKey {
    kind: SnapshotEncodedSubtreeKind,
    id: u64,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
struct SnapshotEncodedSubtreeValue {
    content: EncodedSubtreeContent,
    value: JsValue,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
enum EncodedSubtreeContent {
    Permanent(Arc<PermanentSnapshot>),
    Hand(Arc<HandCardSnapshot>),
    Zone(Arc<ZoneCardSnapshot>),
    Viewed(Arc<ViewedCardSnapshot>),
}
#[cfg(target_arch = "wasm32")]
trait CachedSubtree {
    fn object_id(&self) -> u64;
    fn matches(&self, previous: &EncodedSubtreeContent) -> bool;
    fn to_cached(&self) -> EncodedSubtreeContent;
}
#[cfg(target_arch = "wasm32")]
macro_rules! cached_subtree {
    ($kind:ty, $variant:ident) => {
        impl CachedSubtree for Arc<$kind> {
            fn object_id(&self) -> u64 { self.id }
            fn matches(&self, previous: &EncodedSubtreeContent) -> bool {
                matches!(previous, EncodedSubtreeContent::$variant(value) if Arc::ptr_eq(value, self))
            }
            fn to_cached(&self) -> EncodedSubtreeContent { EncodedSubtreeContent::$variant(self.clone()) }
        }
    };
}
#[cfg(target_arch = "wasm32")]
cached_subtree!(PermanentSnapshot, Permanent);
#[cfg(target_arch = "wasm32")]
cached_subtree!(ViewedCardSnapshot, Viewed);
#[cfg(target_arch = "wasm32")]
cached_subtree!(HandCardSnapshot, Hand);
#[cfg(target_arch = "wasm32")]
cached_subtree!(ZoneCardSnapshot, Zone);

#[cfg(target_arch = "wasm32")]
const SNAPSHOT_JS_ENCODING_CACHE_LIMIT: usize = 16_384;

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
// Owning references prevent pointer-address reuse while array keys are cached.
#[allow(dead_code)]
enum EncodedArrayIdentity {
    Permanents(Arc<Vec<Arc<PermanentSnapshot>>>),
    Hand(Arc<Vec<Arc<HandCardSnapshot>>>),
    Zone(Arc<Vec<Arc<ZoneCardSnapshot>>>),
    Viewed(Arc<Vec<Arc<ViewedCardSnapshot>>>),
}

#[derive(Debug, Default)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(super) struct SnapshotJsEncodingCache {
    #[cfg(target_arch = "wasm32")]
    subtrees: RefCell<
        BoundedCache<
            SnapshotEncodedSubtreeKey,
            SnapshotEncodedSubtreeValue,
            SNAPSHOT_JS_ENCODING_CACHE_LIMIT,
        >,
    >,
    #[cfg(target_arch = "wasm32")]
    arrays: RefCell<BoundedCache<(u8, usize), (EncodedArrayIdentity, JsValue), 512>>,
}

#[cfg(target_arch = "wasm32")]
impl SnapshotJsEncodingCache {
    pub(super) fn encode_snapshot(&self, snapshot: &GameSnapshot) -> Result<JsValue, JsValue> {
        let object = js_sys::Object::new();
        self.set_serde(&object, "snapshot_id", &snapshot.snapshot_id)?;
        self.set_serde(&object, "perspective", &snapshot.perspective)?;
        self.set_serde(&object, "turn_number", &snapshot.turn_number)?;
        self.set_serde(&object, "active_player", &snapshot.active_player)?;
        self.set_serde(&object, "active_players", &snapshot.active_players)?;
        self.set_serde(&object, "priority_player", &snapshot.priority_player)?;
        self.set_serde(
            &object,
            "priority_team_players",
            &snapshot.priority_team_players,
        )?;
        self.set_serde(&object, "phase", &snapshot.phase)?;
        self.set_serde(&object, "step", &snapshot.step)?;
        self.set_serde(&object, "stack_size", &snapshot.stack_size)?;
        self.set_serde(&object, "stack_preview", &snapshot.stack_preview)?;
        self.set_serde(&object, "stack_objects", &snapshot.stack_objects)?;
        self.set_serde(
            &object,
            "resolving_stack_object",
            &snapshot.resolving_stack_object,
        )?;
        self.set_serde(&object, "battlefield_size", &snapshot.battlefield_size)?;
        self.set_serde(&object, "exile_size", &snapshot.exile_size)?;
        self.set_value(
            &object,
            "players",
            self.encode_players(&snapshot.players)?.as_ref(),
        )?;
        self.set_serde(
            &object,
            "battlefield_transitions",
            &snapshot.battlefield_transitions,
        )?;
        self.set_serde(&object, "zone_transitions", &snapshot.zone_transitions)?;
        self.set_serde(&object, "effect_events", &snapshot.effect_events)?;
        self.set_serde(
            &object,
            "crypto_requirements",
            &snapshot.crypto_requirements,
        )?;
        self.set_serde(&object, "viewed_cards", &snapshot.viewed_cards)?;
        self.set_serde(&object, "decision", &snapshot.decision)?;
        self.set_serde(&object, "mana_payment", &snapshot.mana_payment)?;
        self.set_serde(&object, "game_over", &snapshot.game_over)?;
        self.set_serde(&object, "cancelable", &snapshot.cancelable)?;
        self.set_serde(
            &object,
            "undo_land_stable_id",
            &snapshot.undo_land_stable_id,
        )?;
        Ok(object.into())
    }

    fn encode_players(&self, players: &[PlayerSnapshot]) -> Result<JsValue, JsValue> {
        let array = js_sys::Array::new();
        for player in players {
            array.push(&self.encode_player(player)?);
        }
        Ok(array.into())
    }

    fn encode_player(&self, player: &PlayerSnapshot) -> Result<JsValue, JsValue> {
        let object = js_sys::Object::new();
        self.set_serde(&object, "id", &player.id)?;
        self.set_serde(&object, "name", &player.name)?;
        self.set_serde(&object, "life", &player.life)?;
        self.set_serde(&object, "mana_pool", &player.mana_pool)?;
        self.set_serde(&object, "can_view_hand", &player.can_view_hand)?;
        self.set_serde(
            &object,
            "can_view_library_top",
            &player.can_view_library_top,
        )?;
        self.set_serde(&object, "hand_size", &player.hand_size)?;
        self.set_serde(&object, "library_size", &player.library_size)?;
        self.set_serde(&object, "graveyard_size", &player.graveyard_size)?;
        self.set_serde(&object, "command_size", &player.command_size)?;
        self.set_serde(&object, "ante_size", &player.ante_size)?;
        self.set_value(
            &object,
            "hand_cards",
            self.encode_hand_cards(&player.hand_cards)?.as_ref(),
        )?;
        self.set_value(
            &object,
            "graveyard_cards",
            self.encode_zone_cards(&player.graveyard_cards)?.as_ref(),
        )?;
        self.set_value(
            &object,
            "exile_cards",
            self.encode_zone_cards(&player.exile_cards)?.as_ref(),
        )?;
        self.set_value(
            &object,
            "command_cards",
            self.encode_zone_cards(&player.command_cards)?.as_ref(),
        )?;
        self.set_value(
            &object,
            "ante_cards",
            self.encode_zone_cards(&player.ante_cards)?.as_ref(),
        )?;
        self.set_value(
            &object,
            "sideboard_cards",
            self.encode_zone_cards(&player.sideboard_cards)?.as_ref(),
        )?;
        self.set_serde(&object, "library_top", &player.library_top)?;
        self.set_value(
            &object,
            "persistent_look_cards",
            self.encode_viewed_cards(&player.persistent_look_cards)?
                .as_ref(),
        )?;
        self.set_serde(&object, "graveyard_top", &player.graveyard_top)?;
        self.set_value(
            &object,
            "battlefield",
            self.encode_permanents(&player.battlefield)?.as_ref(),
        )?;
        self.set_serde(&object, "battlefield_total", &player.battlefield_total)?;
        Ok(object.into())
    }

    fn encode_array(
        &self,
        key: (u8, usize),
        identity: EncodedArrayIdentity,
        build: impl FnOnce() -> Result<JsValue, JsValue>,
    ) -> Result<JsValue, JsValue> {
        if let Some((_, value)) = self.arrays.borrow_mut().get(&key) {
            return Ok(value.clone());
        }
        let value = build()?;
        freeze_snapshot_subtree(&value);
        self.arrays
            .borrow_mut()
            .insert(key, (identity, value.clone()));
        Ok(value)
    }

    fn encode_permanents(
        &self,
        cards: &Arc<Vec<Arc<PermanentSnapshot>>>,
    ) -> Result<JsValue, JsValue> {
        self.encode_array(
            (0, Arc::as_ptr(cards) as usize),
            EncodedArrayIdentity::Permanents(cards.clone()),
            || {
                let array = js_sys::Array::new();
                for card in cards.iter() {
                    array.push(
                        &self.encode_cached_subtree(SnapshotEncodedSubtreeKind::Permanent, card)?,
                    );
                }
                Ok(array.into())
            },
        )
    }
    fn encode_hand_cards(
        &self,
        cards: &Arc<Vec<Arc<HandCardSnapshot>>>,
    ) -> Result<JsValue, JsValue> {
        self.encode_array(
            (1, Arc::as_ptr(cards) as usize),
            EncodedArrayIdentity::Hand(cards.clone()),
            || {
                let array = js_sys::Array::new();
                for card in cards.iter() {
                    array.push(
                        &self.encode_cached_subtree(SnapshotEncodedSubtreeKind::HandCard, card)?,
                    );
                }
                Ok(array.into())
            },
        )
    }
    fn encode_zone_cards(
        &self,
        cards: &Arc<Vec<Arc<ZoneCardSnapshot>>>,
    ) -> Result<JsValue, JsValue> {
        self.encode_array(
            (2, Arc::as_ptr(cards) as usize),
            EncodedArrayIdentity::Zone(cards.clone()),
            || {
                let array = js_sys::Array::new();
                for card in cards.iter() {
                    array.push(
                        &self.encode_cached_subtree(SnapshotEncodedSubtreeKind::ZoneCard, card)?,
                    );
                }
                Ok(array.into())
            },
        )
    }
    fn encode_viewed_cards(
        &self,
        cards: &Arc<Vec<Arc<ViewedCardSnapshot>>>,
    ) -> Result<JsValue, JsValue> {
        self.encode_array(
            (3, Arc::as_ptr(cards) as usize),
            EncodedArrayIdentity::Viewed(cards.clone()),
            || {
                let array = js_sys::Array::new();
                for card in cards.iter() {
                    array.push(
                        &self
                            .encode_cached_subtree(SnapshotEncodedSubtreeKind::ViewedCard, card)?,
                    );
                }
                Ok(array.into())
            },
        )
    }

    fn encode_cached_subtree<T>(
        &self,
        kind: SnapshotEncodedSubtreeKind,
        value: &T,
    ) -> Result<JsValue, JsValue>
    where
        T: Serialize + CachedSubtree,
    {
        let key = SnapshotEncodedSubtreeKey {
            kind,
            id: value.object_id(),
        };
        if let Some(cached) = self.subtrees.borrow_mut().get(&key)
            && value.matches(&cached.content)
        {
            return Ok(cached.value.clone());
        }

        let encoded = serde_wasm_bindgen::to_value(value).map_err(|error| {
            JsValue::from_str(&format!("snapshot subtree encode failed: {error}"))
        })?;
        freeze_snapshot_subtree(&encoded);
        let mut subtrees = self.subtrees.borrow_mut();
        subtrees.insert(
            key,
            SnapshotEncodedSubtreeValue {
                content: value.to_cached(),
                value: encoded.clone(),
            },
        );
        Ok(encoded)
    }

    fn set_serde<T>(&self, object: &js_sys::Object, key: &str, value: &T) -> Result<(), JsValue>
    where
        T: Serialize,
    {
        let value = serde_wasm_bindgen::to_value(value).map_err(|error| {
            JsValue::from_str(&format!("snapshot field {key} encode failed: {error}"))
        })?;
        self.set_value(object, key, &value)
    }

    fn set_value(
        &self,
        object: &js_sys::Object,
        key: &str,
        value: &JsValue,
    ) -> Result<(), JsValue> {
        js_sys::Reflect::set(object, &JsValue::from_str(key), value).map(|_| ())
    }
}

#[cfg(target_arch = "wasm32")]
fn freeze_snapshot_subtree(value: &JsValue) {
    #[cfg(debug_assertions)]
    if value.is_object() {
        let object = js_sys::Object::from(value.clone());
        js_sys::Object::freeze(&object);
    }

    #[cfg(not(debug_assertions))]
    let _ = value;
}

fn battlefield_lane_for_card_types(card_types: &[CardType]) -> BattlefieldLane {
    if card_types.contains(&CardType::Enchantment) {
        return BattlefieldLane::Enchantments;
    }
    if card_types.contains(&CardType::Creature) {
        return BattlefieldLane::Creatures;
    }
    if card_types.contains(&CardType::Artifact) {
        return BattlefieldLane::Artifacts;
    }
    if card_types.contains(&CardType::Land) {
        return BattlefieldLane::Lands;
    }
    if card_types.contains(&CardType::Planeswalker) {
        return BattlefieldLane::Planeswalkers;
    }
    if card_types.contains(&CardType::Battle) {
        return BattlefieldLane::Battles;
    }
    BattlefieldLane::Other
}

fn counter_signature_for_group(obj: &ironsmith::object::Object) -> String {
    let mut parts: Vec<(String, u32)> = obj
        .counters
        .iter()
        .map(|(counter_type, amount)| (counter_type.description().into_owned(), *amount))
        .collect();
    parts.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    if parts.is_empty() {
        return "-".to_string();
    }
    parts
        .into_iter()
        .map(|(kind, amount)| format!("{kind}:{amount}"))
        .collect::<Vec<_>>()
        .join("|")
}

fn sorted_name_signature<T, F>(items: &[T], mut name: F) -> String
where
    F: FnMut(&T) -> String,
{
    let mut parts = items.iter().map(&mut name).collect::<Vec<_>>();
    parts.sort_unstable();
    parts.join(",")
}

fn color_signature(colors: ironsmith::color::ColorSet) -> String {
    let mut parts = Vec::new();
    for color in ironsmith::color::Color::ALL {
        if colors.contains(color) {
            parts.push(color.name());
        }
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(",")
    }
}

fn ability_signature(abilities: &[ironsmith::ability::Ability]) -> String {
    let mut parts = abilities
        .iter()
        .map(|ability| format!("{:?}", ability.kind))
        .collect::<Vec<_>>();
    parts.sort_unstable();
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("|")
    }
}

fn static_ability_signature(
    static_abilities: &[ironsmith::static_abilities::StaticAbility],
) -> String {
    let mut parts = static_abilities
        .iter()
        .map(|ability| format!("{ability:?}"))
        .collect::<Vec<_>>();
    parts.sort_unstable();
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("|")
    }
}

fn attached_to_signature(
    game: &GameState,
    obj: &ironsmith::object::Object,
    visiting: &mut HashSet<ObjectId>,
) -> String {
    match obj.attached_to {
        Some(AttachmentTarget::Object(id)) => game
            .object(id)
            .map(|target| object_characteristic_signature_inner(game, target, false, visiting))
            .unwrap_or_else(|| "object:missing".to_string()),
        Some(AttachmentTarget::Player(id)) => format!("player:{}", id.0),
        None => "-".to_string(),
    }
}

fn attachment_signature(
    game: &GameState,
    obj: &ironsmith::object::Object,
    visiting: &mut HashSet<ObjectId>,
) -> String {
    let mut parts = obj
        .attachments
        .iter()
        .filter_map(|attachment_id| game.object(*attachment_id))
        .map(|attachment| object_characteristic_signature_inner(game, attachment, false, visiting))
        .collect::<Vec<_>>();
    parts.sort_unstable();
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("||")
    }
}

fn object_characteristic_signature(
    game: &GameState,
    obj: &ironsmith::object::Object,
    include_attachments: bool,
) -> String {
    object_characteristic_signature_inner(game, obj, include_attachments, &mut HashSet::new())
}

fn object_characteristic_signature_inner(
    game: &GameState,
    obj: &ironsmith::object::Object,
    include_attachments: bool,
    visiting: &mut HashSet<ObjectId>,
) -> String {
    if !visiting.insert(obj.id) {
        return "attachment_cycle:".to_owned();
    }
    let current = game.current_characteristics(obj.id);
    let name = current
        .as_ref()
        .map(|chars| chars.name.to_owned_string())
        .unwrap_or_else(|| obj.name.to_string());
    let compiled_card_text = current
        .as_ref()
        .map(|chars| chars.compiled_card_text.to_string())
        .unwrap_or_else(|| obj.compiled_card_text.to_string());
    let power = current
        .as_ref()
        .and_then(|chars| chars.power)
        .or_else(|| obj.power());
    let toughness = current
        .as_ref()
        .and_then(|chars| chars.toughness)
        .or_else(|| obj.toughness());
    let card_types: &[CardType] = current
        .as_ref()
        .map(|chars| chars.card_types.as_slice())
        .unwrap_or(&obj.card_types);
    let subtypes: &[Subtype] = current
        .as_ref()
        .map(|chars| chars.subtypes.as_slice())
        .unwrap_or(&obj.subtypes);
    let supertypes: &[ironsmith::types::Supertype] = current
        .as_ref()
        .map(|chars| chars.supertypes.as_slice())
        .unwrap_or(&obj.supertypes);
    let colors = current
        .as_ref()
        .map(|chars| chars.colors)
        .unwrap_or_else(|| obj.colors());
    let owned_abilities: Vec<ironsmith::ability::Ability>;
    let abilities = if let Some(chars) = current.as_ref() {
        chars.abilities.as_slice()
    } else {
        owned_abilities = obj.abilities_vec();
        owned_abilities.as_slice()
    };
    let owned_static_abilities: Vec<ironsmith::static_abilities::StaticAbility>;
    let static_abilities = if let Some(chars) = current.as_ref() {
        chars.static_abilities.as_slice()
    } else {
        owned_static_abilities = obj
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Static(static_ability) => {
                    Some(static_ability.clone())
                }
                _ => None,
            })
            .collect();
        owned_static_abilities.as_slice()
    };
    let controller = current
        .as_ref()
        .map(|chars| chars.controller)
        .unwrap_or(obj.owner);

    let card_type_signature =
        sorted_name_signature(card_types, |card_type| card_type.name().to_string());
    let subtype_signature = sorted_name_signature(subtypes, |subtype| subtype.display_name());
    let supertype_signature =
        sorted_name_signature(supertypes, |supertype| supertype.name().to_string());
    let attachment_part = if include_attachments {
        attachment_signature(game, obj, visiting)
    } else {
        "-".to_string()
    };

    let signature = [
        format!("owner:{}", obj.owner.0),
        format!("controller:{}", controller.0),
        format!("kind:{}", obj.kind.name()),
        format!("name:{name}"),
        format!(
            "mana:{}",
            obj.mana_cost
                .as_ref()
                .map(|mana_cost| mana_cost.to_oracle())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!("colors:{}", color_signature(colors)),
        format!("supertypes:{supertype_signature}"),
        format!("types:{card_type_signature}"),
        format!("subtypes:{subtype_signature}"),
        format!("oracle:{compiled_card_text}"),
        format!(
            "pt:{}/{}",
            power.map_or("-".to_string(), |p| p.to_string()),
            toughness.map_or("-".to_string(), |t| t.to_string())
        ),
        format!(
            "loyalty:{}",
            obj.loyalty()
                .map_or_else(|| "-".to_string(), |loyalty| loyalty.to_string())
        ),
        format!(
            "defense:{}",
            obj.defense()
                .map_or_else(|| "-".to_string(), |defense| defense.to_string())
        ),
        format!("counters:{}", counter_signature_for_group(obj)),
        format!("abilities:{}", ability_signature(abilities)),
        format!("static:{}", static_ability_signature(static_abilities)),
        format!("attached_to:{}", attached_to_signature(game, obj, visiting)),
        format!("attachments:{attachment_part}"),
    ]
    .join("\n");
    visiting.remove(&obj.id);
    signature
}

pub(super) fn counter_snapshots_for_object(
    obj: &ironsmith::object::Object,
) -> Vec<CounterSnapshot> {
    let mut counters: Vec<CounterSnapshot> = obj
        .counters
        .iter()
        .map(|(kind, amount)| CounterSnapshot {
            kind: kind.description().into_owned(),
            amount: *amount,
        })
        .collect();
    counters.sort_unstable_by(|left, right| left.kind.cmp(&right.kind));
    counters
}

pub(super) fn protected_object_ids_for_decision(
    decision: Option<&DecisionContext>,
) -> HashSet<ObjectId> {
    let mut ids = HashSet::new();
    let Some(decision) = decision else {
        return ids;
    };

    match decision {
        DecisionContext::ManaPayment(payment) => {
            ids.insert(payment.source);
            ids.extend(
                payment
                    .plan
                    .mana_ability_steps
                    .iter()
                    .map(|step| step.source),
            );
            ids.extend(payment.plan.allocations.iter().filter_map(|allocation| {
                match allocation.payment {
                    ironsmith::mana_payment::PlannedPipPayment::Convoke(source)
                    | ironsmith::mana_payment::PlannedPipPayment::Improvise(source) => Some(source),
                    _ => None,
                }
            }));
        }
        DecisionContext::Priority(_) => {}
        DecisionContext::Targets(targets) => {
            for requirement in &targets.requirements {
                for target in &requirement.legal_targets {
                    if let Target::Object(object_id) = target {
                        ids.insert(*object_id);
                    }
                }
            }
        }
        DecisionContext::SelectObjects(objects) => {
            for candidate in &objects.candidates {
                if candidate.legal {
                    ids.insert(candidate.id);
                }
            }
        }
        DecisionContext::Attackers(attackers) => {
            for option in &attackers.attacker_options {
                ids.insert(option.creature);
                for target in &option.valid_targets {
                    if let AttackTarget::Planeswalker(object_id) | AttackTarget::Battle(object_id) =
                        target
                    {
                        ids.insert(*object_id);
                    }
                }
            }
        }
        DecisionContext::Blockers(blockers) => {
            for option in &blockers.blocker_options {
                ids.insert(option.attacker);
                for (blocker, _) in &option.valid_blockers {
                    ids.insert(*blocker);
                }
            }
        }
        DecisionContext::SelectOptions(options)
            if options
                .description
                .starts_with("Activate mana abilities before paying costs")
                && options.options.iter().any(|option| {
                    option
                        .description
                        .eq_ignore_ascii_case("Finish activating mana abilities")
                }) =>
        {
            // Mana-window actions use the battlefield groups to collapse
            // equivalent sources into one UI option. Keep those source IDs
            // groupable; the selected member is carried by the option itself.
        }
        DecisionContext::SelectOptions(options) => {
            for option in &options.options {
                if let Some(object_id) = option.object_id {
                    ids.insert(object_id);
                }
                if let Some(related_object_ids) = &option.related_object_ids {
                    ids.extend(related_object_ids.iter().copied());
                }
            }
        }
        DecisionContext::Modes(_)
        | DecisionContext::HybridChoice(_)
        | DecisionContext::TextInput(_)
        | DecisionContext::Boolean(_)
        | DecisionContext::Number(_)
        | DecisionContext::Order(_)
        | DecisionContext::Distribute(_)
        | DecisionContext::Colors(_)
        | DecisionContext::Counters(_)
        | DecisionContext::Partition(_)
        | DecisionContext::Proliferate(_) => {}
    }

    ids
}

#[cfg(test)]
pub(super) fn grouped_battlefield_for_player(
    game: &GameState,
    player: PlayerId,
    protected_ids: &HashSet<ObjectId>,
) -> (Vec<PermanentSnapshot>, usize) {
    let object_view_cache = SnapshotObjectViewCache::default();
    grouped_battlefield_for_player_with_cache(game, player, protected_ids, &object_view_cache)
}

#[cfg(test)]
fn grouped_battlefield_for_player_with_cache(
    game: &GameState,
    player: PlayerId,
    protected_ids: &HashSet<ObjectId>,
    object_view_cache: &SnapshotObjectViewCache,
) -> (Vec<PermanentSnapshot>, usize) {
    let ids = game.battlefield.iter().copied().filter(|id| {
        game.object(*id)
            .is_some_and(|object| game.current_controller(*id).unwrap_or(object.owner) == player)
    });
    grouped_battlefield_for_ids(game, ids, protected_ids, object_view_cache)
}

fn grouped_battlefield_for_ids(
    game: &GameState,
    ids: impl Iterator<Item = ObjectId>,
    protected_ids: &HashSet<ObjectId>,
    object_view_cache: &SnapshotObjectViewCache,
) -> (Vec<PermanentSnapshot>, usize) {
    let mut grouped: HashMap<BattlefieldGroupKey, Vec<Arc<PermanentObjectView>>> = HashMap::new();
    let mut total = 0usize;

    for object_id in ids {
        let Some(obj) = game.object(object_id) else {
            continue;
        };
        total += 1;

        let force_single = protected_ids.contains(&obj.id).then_some(obj.id.0);
        let view = object_view_cache.battlefield_view(game, obj);
        let key = BattlefieldGroupKey {
            lane: view.lane,
            name: view.name.clone(),
            tapped: view.tapped,
            characteristic_signature: view.characteristic_signature.clone(),
            counter_signature: view.counter_signature.clone(),
            token: view.token,
            force_single_object: force_single,
        };
        grouped.entry(key).or_default().push(view);
    }

    let mut groups: Vec<(BattlefieldGroupKey, Vec<Arc<PermanentObjectView>>)> =
        grouped.into_iter().collect();
    groups.sort_unstable_by(|(left_key, left_members), (right_key, right_members)| {
        left_key
            .lane
            .cmp(&right_key.lane)
            .then_with(|| left_key.name.cmp(&right_key.name))
            .then_with(|| left_key.tapped.cmp(&right_key.tapped))
            .then_with(|| left_key.token.cmp(&right_key.token))
            .then_with(|| {
                left_members
                    .first()
                    .map(|view| view.id)
                    .cmp(&right_members.first().map(|view| view.id))
            })
    });

    let snapshots = groups
        .into_iter()
        .map(|(key, mut members)| {
            members.sort_unstable_by_key(|view| view.id);
            let representative = members.first();
            let member_ids: Vec<u64> = members.iter().map(|view| view.id).collect();
            let member_stable_ids: Vec<u64> = members.iter().map(|view| view.stable_id).collect();
            let id = representative.map(|view| view.id).unwrap_or_default();
            let stable_id = representative
                .map(|view| view.stable_id)
                .unwrap_or_default();
            let name = representative
                .map(|view| view.name.clone())
                .unwrap_or_else(|| key.name.clone());
            let power_toughness = representative.and_then(|view| view.power_toughness.clone());
            let mana_cost = representative.and_then(|view| view.mana_cost.clone());
            let compiled_card_text = representative
                .map(|view| view.oracle_text.clone())
                .unwrap_or_default();
            let counters = representative
                .map(|view| view.counters.clone())
                .unwrap_or_default();
            PermanentSnapshot {
                view_identity: PermanentViewIdentity(members.clone()),
                id,
                stable_id,
                name,
                token: key.token,
                tapped: key.tapped,
                count: member_ids.len().max(1),
                member_ids,
                member_stable_ids,
                lane: key.lane.as_str().to_string(),
                mana_cost,
                oracle_text: compiled_card_text,
                power_toughness,
                counter_signature: key.counter_signature.clone(),
                counters,
            }
        })
        .collect();

    (snapshots, total)
}

fn pseudo_hand_glow_kind_with_grants(
    game: &GameState,
    perspective: PlayerId,
    object: &ironsmith::object::Object,
    zone: Zone,
    grants: Option<&std::cell::OnceCell<Vec<ironsmith::grant_registry::Grant>>>,
) -> Option<&'static str> {
    if object.zone != zone
        || matches!(
            zone,
            Zone::Hand | Zone::Library | Zone::Battlefield | Zone::Stack
        )
    {
        return None;
    }

    if zone == Zone::Command && object.owner == perspective && game.is_commander(object.id) {
        return Some("extra");
    }

    let kinds = if let Some(grants) = grants {
        game.effect_store
            .grant_registry
            .zone_card_grant_kinds_from_snapshot(
                game,
                object.id,
                zone,
                perspective,
                grants.get_or_init(|| game.effect_store.grant_registry.active_grants(game)),
            )
    } else {
        (
            !game
                .effect_store
                .grant_registry
                .granted_play_from_for_card(game, object.id, zone, perspective)
                .is_empty(),
            !game
                .effect_store
                .grant_registry
                .granted_alternative_casts_for_card(game, object.id, zone, perspective)
                .is_empty(),
        )
    };
    if kinds.0 {
        return Some("play-from");
    }
    if kinds.1 {
        return Some("extra");
    }

    if object.owner != perspective {
        return None;
    }

    object
        .alternative_casts
        .iter()
        .any(|method| method.cast_from_zone() == zone)
        .then_some("extra")
}

#[cfg(test)]
fn battlefield_has_static_ability(game: &GameState, ability_id: StaticAbilityId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id)
            .is_some_and(|_| game.object_has_static_ability_id(*id, ability_id))
    })
}

#[cfg(test)]
fn can_view_own_library_top(game: &GameState, player: PlayerId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id).is_some_and(|object| {
            game.current_controller(*id).unwrap_or(object.owner) == player
                && game.object_has_static_ability_id(*id, StaticAbilityId::LookAtTopCardOfLibrary)
        })
    })
}

#[cfg(test)]
fn library_top_revealed_by_static_ability(game: &GameState, player: PlayerId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id).is_some_and(|object| {
            game.current_controller(*id).unwrap_or(object.owner) == player
                && game.object_has_static_ability_id(
                    *id,
                    StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
                )
        })
    })
}

#[cfg(test)]
fn can_view_library_top(game: &GameState, perspective: PlayerId, player: PlayerId) -> bool {
    if (perspective == player || game.controlling_player_for(player) == perspective)
        && can_view_own_library_top(game, player)
    {
        return true;
    }

    if library_top_revealed_by_static_ability(game, player) {
        return true;
    }

    battlefield_has_static_ability(game, StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries)
}

#[cfg(test)]
fn hand_revealed_by_static_ability(game: &GameState, player: PlayerId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id).is_some_and(|object| {
            game.current_controller(*id).unwrap_or(object.owner) != player
                && game.object_has_static_ability_id(
                    *id,
                    StaticAbilityId::OpponentsPlayWithHandsRevealed,
                )
        })
    })
}

fn build_zone_card_snapshot(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    object: &ironsmith::object::Object,
    zone: Zone,
) -> ZoneCardSnapshot {
    build_zone_card_snapshot_with_grants(game, perspective, viewed_cards, object, zone, None)
}

fn build_zone_card_snapshot_with_grants(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    object: &ironsmith::object::Object,
    zone: Zone,
    grants: Option<&std::cell::OnceCell<Vec<ironsmith::grant_registry::Grant>>>,
) -> ZoneCardSnapshot {
    let visible = object_visible_to_perspective(game, perspective, viewed_cards, object.id);
    let pseudo_hand_glow_kind = visible
        .then(|| pseudo_hand_glow_kind_with_grants(game, perspective, object, zone, grants))
        .flatten()
        .map(str::to_string);
    let power_toughness = visible
        .then(|| match (object.power(), object.toughness()) {
            (Some(power), Some(toughness)) => Some(format!("{power}/{toughness}")),
            _ => None,
        })
        .flatten();

    ZoneCardSnapshot {
        id: object.id.0,
        stable_id: object.stable_id.0.0,
        face_down: game.is_face_down(object.id),
        name: if visible {
            object.name.to_string()
        } else {
            hidden_object_label()
        },
        mana_cost: visible
            .then(|| object.mana_cost.as_ref().map(|mc| mc.to_oracle()))
            .flatten(),
        oracle_text: if visible {
            object.compiled_card_text.to_string()
        } else {
            String::new()
        },
        power_toughness,
        loyalty: visible.then(|| object.loyalty()).flatten(),
        defense: visible.then(|| object.defense()).flatten(),
        card_types: if visible {
            object
                .card_types
                .iter()
                .map(|ct| ct.name().to_string())
                .collect()
        } else {
            Vec::new()
        },
        show_in_pseudo_hand: visible && pseudo_hand_glow_kind.is_some(),
        pseudo_hand_glow_kind,
    }
}

// Retaining the Arc identities makes cache lookup collision-free and avoids
// reformatting or comparing oracle text on unchanged battlefield groups.
#[derive(Debug, Clone)]
struct PermanentViewIdentity(Vec<Arc<PermanentObjectView>>);
impl PartialEq for PermanentViewIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.0.iter().zip(&other.0).all(|(a, b)| Arc::ptr_eq(a, b))
    }
}
impl Eq for PermanentViewIdentity {}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct PermanentSnapshot {
    #[serde(skip)]
    view_identity: PermanentViewIdentity,
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) token: bool,
    pub(super) tapped: bool,
    pub(super) count: usize,
    pub(super) member_ids: Vec<u64>,
    pub(super) member_stable_ids: Vec<u64>,
    pub(super) lane: String,
    pub(super) mana_cost: Option<String>,
    pub(super) oracle_text: String,
    pub(super) power_toughness: Option<String>,
    pub(super) counter_signature: String,
    pub(super) counters: Vec<CounterSnapshot>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct CounterSnapshot {
    pub(super) kind: String,
    pub(super) amount: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BattlefieldTransitionKindSnapshot {
    Damaged,
    Destroyed,
    Sacrificed,
    Exiled,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct BattlefieldTransitionSnapshot {
    pub(super) stable_id: u64,
    pub(super) kind: BattlefieldTransitionKindSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ZoneTransitionSnapshot {
    pub(super) id: u64,
    pub(super) old_object_id: u64,
    pub(super) new_object_id: u64,
    pub(super) stable_id: u64,
    pub(super) owner: u8,
    pub(super) controller: u8,
    pub(super) from_zone: String,
    pub(super) to_zone: String,
    pub(super) card: ZoneCardSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct UiEffectEventSnapshot {
    pub(super) id: u64,
    pub(super) kind: String,
    pub(super) player: Option<u8>,
    pub(super) other_player: Option<u8>,
    pub(super) stable_ids: Vec<u64>,
    pub(super) value: Option<i64>,
    pub(super) text: Option<String>,
}

fn effect_event_snapshots(game: &GameState) -> Vec<UiEffectEventSnapshot> {
    game.ui_effect_events()
        .map(|event| UiEffectEventSnapshot {
            id: event.id,
            kind: event.kind.clone(),
            player: event.player.map(|p| p.0),
            other_player: event.other_player.map(|p| p.0),
            stable_ids: event.stable_ids.iter().map(|sid| sid.0.0).collect(),
            value: event.value,
            text: event.text.clone(),
        })
        .collect()
}

pub(super) fn battlefield_transition_snapshots(
    transitions: impl IntoIterator<Item = UiBattlefieldTransition>,
) -> Vec<BattlefieldTransitionSnapshot> {
    transitions
        .into_iter()
        .map(|transition| BattlefieldTransitionSnapshot {
            stable_id: transition.stable_id.0.0,
            kind: match transition.kind {
                UiBattlefieldTransitionKind::Damaged => BattlefieldTransitionKindSnapshot::Damaged,
                UiBattlefieldTransitionKind::Destroyed => {
                    BattlefieldTransitionKindSnapshot::Destroyed
                }
                UiBattlefieldTransitionKind::Sacrificed => {
                    BattlefieldTransitionKindSnapshot::Sacrificed
                }
                UiBattlefieldTransitionKind::Exiled => BattlefieldTransitionKindSnapshot::Exiled,
            },
        })
        .collect()
}

fn zone_transition_snapshots(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
) -> Vec<ZoneTransitionSnapshot> {
    let hidden_label = hidden_object_label().to_ascii_lowercase();
    game.ui_zone_transitions()
        .filter_map(|transition| {
            let object = game.object(transition.new_object_id)?;
            if !object_visible_to_perspective(game, perspective, viewed_cards, object.id) {
                return None;
            }
            let card =
                build_zone_card_snapshot(game, perspective, viewed_cards, object, transition.to);
            if card.name.trim().to_ascii_lowercase() == hidden_label {
                return None;
            }
            Some(ZoneTransitionSnapshot {
                id: transition.id,
                old_object_id: transition.old_object_id.0,
                new_object_id: transition.new_object_id.0,
                stable_id: transition.stable_id.0.0,
                owner: transition.owner.0,
                controller: transition.controller.0,
                from_zone: transition.from.to_string(),
                to_zone: transition.to.to_string(),
                card,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ObjectDetailsSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) zone: String,
    pub(super) owner: u8,
    pub(super) controller: u8,
    pub(super) type_line: String,
    pub(super) type_line_display: String,
    pub(super) type_line_badges: Vec<String>,
    pub(super) mana_cost: Option<String>,
    pub(super) oracle_text: String,
    pub(super) power: Option<i32>,
    pub(super) toughness: Option<i32>,
    pub(super) loyalty: Option<u32>,
    pub(super) tapped: bool,
    pub(super) counters: Vec<CounterSnapshot>,
    pub(super) compiled_text: Vec<String>,
    pub(super) abilities: Vec<String>,
    pub(super) chosen_color: Option<String>,
    pub(super) raw_compilation: String,
    pub(super) semantic_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct GameSnapshot {
    pub(super) snapshot_id: u64,
    pub(super) perspective: u8,
    pub(super) turn_number: u32,
    pub(super) active_player: u8,
    pub(super) active_players: Vec<u8>,
    pub(super) priority_player: Option<u8>,
    pub(super) priority_team_players: Vec<u8>,
    pub(super) phase: String,
    pub(super) step: Option<String>,
    pub(super) stack_size: usize,
    pub(super) subgame_depth: usize,
    pub(super) subgame_starting_procedure_pending: bool,
    pub(super) stack_preview: Vec<String>,
    pub(super) stack_objects: Vec<super::StackObjectSnapshot>,
    pub(super) resolving_stack_object: Option<super::StackObjectSnapshot>,
    pub(super) battlefield_size: usize,
    pub(super) exile_size: usize,
    pub(super) players: Vec<PlayerSnapshot>,
    pub(super) planechase: Option<PlanechaseSnapshot>,
    pub(super) vanguard: Option<VanguardSnapshot>,
    pub(super) archenemy: Option<ArchenemySnapshot>,
    pub(super) conspiracy: Option<ConspiracySnapshot>,
    pub(super) grand_melee: Option<GrandMeleeSnapshot>,
    pub(super) battlefield_transitions: Vec<BattlefieldTransitionSnapshot>,
    pub(super) zone_transitions: Vec<ZoneTransitionSnapshot>,
    pub(super) effect_events: Vec<UiEffectEventSnapshot>,
    pub(super) crypto_requirements: Vec<CryptoRequirementView>,
    pub(super) viewed_cards: Option<ViewedCardsSnapshot>,
    pub(super) decision: Option<DecisionView>,
    pub(super) mana_payment: Option<ManaPaymentView>,
    pub(super) game_over: Option<GameOverView>,
    pub(super) cancelable: bool,
    pub(super) undo_land_stable_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PlanechaseSnapshot {
    pub(super) planar_controller: u8,
    pub(super) planar_controllers: Vec<u8>,
    pub(super) die_roll_cost: u32,
    pub(super) planeswalk_count: u64,
    pub(super) communal_deck: bool,
    pub(super) deck_sizes: Vec<PlanarDeckSizeSnapshot>,
    pub(super) face_up: Vec<PlanarCardSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct GrandMeleeSnapshot {
    pub(super) seats: Vec<u8>,
    pub(super) starting_player_count: usize,
    pub(super) focused_marker: u32,
    pub(super) markers: Vec<GrandMeleeMarkerSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct GrandMeleeMarkerSnapshot {
    pub(super) number: u32,
    pub(super) holder: u8,
    pub(super) status: String,
    pub(super) stack_size: usize,
    pub(super) removal_designations: usize,
    pub(super) normal_turn_pending: bool,
    pub(super) retained_extra_turn_waiting: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct VanguardSnapshot {
    pub(super) cards: Vec<VanguardCardSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ArchenemySnapshot {
    pub(super) variant: String,
    pub(super) archenemies: Vec<u8>,
    pub(super) deck_sizes: Vec<ArchenemyDeckSizeSnapshot>,
    pub(super) face_up: Vec<SchemeCardSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ArchenemyDeckSizeSnapshot {
    pub(super) owner: u8,
    pub(super) size: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SchemeCardSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) owner: u8,
    pub(super) ongoing: bool,
    pub(super) oracle_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ConspiracySnapshot {
    pub(super) cards: Vec<ConspiracyCardSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ConspiracyCardSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) owner: u8,
    pub(super) face_down: bool,
    pub(super) name: Option<String>,
    pub(super) oracle_text: Option<String>,
    pub(super) agenda_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct VanguardCardSnapshot {
    pub(super) owner: u8,
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) hand_modifier: i32,
    pub(super) life_modifier: i32,
    pub(super) oracle_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PlanarDeckSizeSnapshot {
    pub(super) owner: Option<u8>,
    pub(super) size: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PlanarCardSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) controller: u8,
    pub(super) oracle_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ManaPoolSnapshot {
    pub(super) white: u32,
    pub(super) blue: u32,
    pub(super) black: u32,
    pub(super) red: u32,
    pub(super) green: u32,
    pub(super) colorless: u32,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PlayerSnapshot {
    pub(super) id: u8,
    pub(super) name: String,
    pub(super) life: i32,
    pub(super) poison_counters: u32,
    pub(super) mana_pool: ManaPoolSnapshot,
    pub(super) can_view_hand: bool,
    pub(super) can_view_library_top: bool,
    pub(super) hand_size: usize,
    pub(super) library_size: usize,
    pub(super) graveyard_size: usize,
    pub(super) command_size: usize,
    pub(super) ante_size: usize,
    pub(super) hand_cards: Arc<Vec<Arc<HandCardSnapshot>>>,
    pub(super) graveyard_cards: Arc<Vec<Arc<ZoneCardSnapshot>>>,
    pub(super) exile_cards: Arc<Vec<Arc<ZoneCardSnapshot>>>,
    pub(super) command_cards: Arc<Vec<Arc<ZoneCardSnapshot>>>,
    pub(super) ante_cards: Arc<Vec<Arc<ZoneCardSnapshot>>>,
    pub(super) sideboard_cards: Arc<Vec<Arc<ZoneCardSnapshot>>>,
    pub(super) library_top: Option<String>,
    pub(super) persistent_look_cards: Arc<Vec<Arc<ViewedCardSnapshot>>>,
    pub(super) graveyard_top: Option<String>,
    pub(super) battlefield: Arc<Vec<Arc<PermanentSnapshot>>>,
    pub(super) battlefield_total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ViewedCardsSnapshot {
    pub(super) viewer: u8,
    pub(super) subject: u8,
    pub(super) zone: String,
    pub(super) visibility: String,
    pub(super) cards: Vec<ViewedCardSnapshot>,
    pub(super) card_ids: Vec<u64>,
    pub(super) source: Option<u64>,
    pub(super) description: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct ViewedCardSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) oracle_text: String,
}

fn resolve_viewed_card(
    game: &GameState,
    id: ObjectId,
    stable_id: StableId,
) -> (ObjectId, u64, String, String) {
    if let Some(current_id) = game.find_object_by_stable_id(stable_id)
        && let Some(obj) = game.object(current_id)
    {
        return (
            current_id,
            obj.stable_id.0.0,
            obj.name.to_string(),
            obj.compiled_card_text.to_string(),
        );
    }

    if let Some(obj) = game.object(id) {
        return (
            id,
            obj.stable_id.0.0,
            obj.name.to_string(),
            obj.compiled_card_text.to_string(),
        );
    }

    let id_as_stable_id = StableId::from_raw(id.0);
    if id_as_stable_id != stable_id
        && let Some(current_id) = game.find_object_by_stable_id(id_as_stable_id)
        && let Some(obj) = game.object(current_id)
    {
        return (
            current_id,
            obj.stable_id.0.0,
            obj.name.to_string(),
            obj.compiled_card_text.to_string(),
        );
    }

    (id, stable_id.0.0, format!("Card #{}", id.0), String::new())
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct HandCardSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) mana_cost: Option<String>,
    pub(super) oracle_text: String,
    pub(super) power_toughness: Option<String>,
    pub(super) loyalty: Option<u32>,
    pub(super) defense: Option<u32>,
    pub(super) card_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct ZoneCardSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) face_down: bool,
    pub(super) name: String,
    pub(super) mana_cost: Option<String>,
    pub(super) oracle_text: String,
    pub(super) power_toughness: Option<String>,
    pub(super) loyalty: Option<u32>,
    pub(super) defense: Option<u32>,
    pub(super) card_types: Vec<String>,
    pub(super) show_in_pseudo_hand: bool,
    pub(super) pseudo_hand_glow_kind: Option<String>,
}

impl GameSnapshot {
    #[cfg(test)]
    pub(super) fn from_game(
        game: &GameState,
        perspective: PlayerId,
        decision: Option<&DecisionContext>,
        mana_payment: Option<ManaPaymentView>,
        game_over: Option<&GameResult>,
        pending_cast_stack_id: Option<ObjectId>,
        resolving_stack_object: Option<super::StackObjectSnapshot>,
        battlefield_transitions: Vec<BattlefieldTransitionSnapshot>,
        viewed_cards: Option<&ActiveViewedCards>,
        cancelable: bool,
        undo_land_stable_id: Option<u64>,
        snapshot_id: u64,
    ) -> Self {
        let object_view_cache = SnapshotObjectViewCache::default();
        Self::from_game_with_object_view_cache(
            game,
            perspective,
            decision,
            mana_payment,
            game_over,
            pending_cast_stack_id,
            resolving_stack_object,
            battlefield_transitions,
            viewed_cards,
            cancelable,
            undo_land_stable_id,
            snapshot_id,
            &object_view_cache,
        )
    }

    pub(super) fn from_game_with_object_view_cache(
        game: &GameState,
        perspective: PlayerId,
        decision: Option<&DecisionContext>,
        mana_payment: Option<ManaPaymentView>,
        game_over: Option<&GameResult>,
        pending_cast_stack_id: Option<ObjectId>,
        resolving_stack_object: Option<super::StackObjectSnapshot>,
        battlefield_transitions: Vec<BattlefieldTransitionSnapshot>,
        viewed_cards: Option<&ActiveViewedCards>,
        cancelable: bool,
        undo_land_stable_id: Option<u64>,
        snapshot_id: u64,
        object_view_cache: &SnapshotObjectViewCache,
    ) -> Self {
        let stack_viewed_cards = super::stack_revealed_view(game);
        let viewed_cards = viewed_cards.or(stack_viewed_cards.as_ref());
        let mut protected_ids = protected_object_ids_for_decision(decision);
        if let Some(payment) = mana_payment.as_ref() {
            protected_ids.extend(
                payment
                    .mana_abilities
                    .iter()
                    .filter_map(|ability| ability.source_id.parse::<u64>().ok().map(ObjectId)),
            );
        }
        let mut characteristic_ids = Vec::new();
        characteristic_ids.extend(game.stack.iter().map(|entry| entry.object_id));
        if let Some(stack_id) = pending_cast_stack_id {
            characteristic_ids.push(stack_id);
        }
        characteristic_ids.sort_unstable();
        characteristic_ids.dedup();
        game.prewarm_calculated_characteristics(&characteristic_ids);
        object_view_cache
            .groups
            .borrow_mut()
            .update(game, &protected_ids, object_view_cache);
        object_view_cache
            .hands
            .borrow_mut()
            .retain(|owner, _| game.players.iter().any(|player| player.id == *owner));
        object_view_cache
            .zones
            .borrow_mut()
            .retain(|(owner, _), _| game.players.iter().any(|player| player.id == *owner));
        object_view_cache
            .looks
            .borrow_mut()
            .retain(|(owner, _), _| game.players.iter().any(|player| player.id == *owner));
        object_view_cache
            .combined_looks
            .borrow_mut()
            .retain(|owner, _| game.players.iter().any(|player| player.id == *owner));
        let potential_grants = object_view_cache.grant_sources.borrow_mut().update(game);
        let active_grants = std::cell::OnceCell::new();
        if !potential_grants {
            let _ = active_grants.set(Vec::new());
        }
        let players = game
            .players
            .iter()
            .map(|p| {
                let (battlefield, battlefield_total) =
                    object_view_cache.groups.borrow().for_player(p.id);
                let is_perspective_player = p.id == perspective;
                let controls_player = game.controlling_player_for(p.id) == perspective;
                let visible_hand_view = viewed_cards.filter(|view| {
                    view.zone == Zone::Hand
                        && view.subject == p.id
                        && (view.public
                            || view.viewer == perspective
                            || game.controlling_player_for(view.viewer) == perspective)
                });
                let (can_view_library_top, hand_revealed_by_static) = object_view_cache
                    .groups
                    .borrow()
                    .visibility_for_player(game, perspective, p.id);
                let can_view_hand = is_perspective_player
                    || controls_player
                    || game.can_review_teammate_hand(perspective, p.id)
                    || visible_hand_view.is_some()
                    || hand_revealed_by_static;

                // Only ongoing permissions belong here; resolution views have their own lifetime.
                let persistent_look_cards = object_view_cache.persistent_look_cards(
                    game,
                    p.id,
                    perspective,
                    can_view_library_top,
                    hand_revealed_by_static,
                );
                PlayerSnapshot {
                    persistent_look_cards,
                    can_view_hand,
                    can_view_library_top,
                    hand_cards: object_view_cache.hand_cards(
                        game,
                        p.id,
                        perspective,
                        visible_hand_view,
                        if is_perspective_player
                            || controls_player
                            || game.can_review_teammate_hand(perspective, p.id)
                            || hand_revealed_by_static
                        {
                            2
                        } else if can_view_hand {
                            1
                        } else {
                            0
                        },
                    ),
                    graveyard_cards: object_view_cache.zone_cards(
                        game,
                        p.id,
                        perspective,
                        Zone::Graveyard,
                        viewed_cards,
                        &active_grants,
                        potential_grants,
                    ),
                    exile_cards: object_view_cache.zone_cards(
                        game,
                        p.id,
                        perspective,
                        Zone::Exile,
                        viewed_cards,
                        &active_grants,
                        potential_grants,
                    ),
                    command_cards: object_view_cache.zone_cards(
                        game,
                        p.id,
                        perspective,
                        Zone::Command,
                        viewed_cards,
                        &active_grants,
                        potential_grants,
                    ),
                    ante_cards: object_view_cache.zone_cards(
                        game,
                        p.id,
                        perspective,
                        Zone::Ante,
                        viewed_cards,
                        &active_grants,
                        potential_grants,
                    ),
                    sideboard_cards: object_view_cache.zone_cards(
                        game,
                        p.id,
                        perspective,
                        Zone::OutsideGame,
                        viewed_cards,
                        &active_grants,
                        potential_grants,
                    ),
                    library_top: can_view_library_top
                        .then(|| {
                            p.library
                                .last()
                                .and_then(|id| game.object(*id))
                                .map(|o| o.name.to_string())
                        })
                        .flatten(),
                    graveyard_top: p
                        .graveyard
                        .last()
                        .and_then(|id| game.object(*id))
                        .map(|o| o.name.to_string()),
                    battlefield,
                    battlefield_total,
                    id: p.id.0,
                    name: p.name.clone(),
                    life: p.life,
                    poison_counters: p.poison_counters,
                    mana_pool: ManaPoolSnapshot {
                        white: p.mana_pool.white,
                        blue: p.mana_pool.blue,
                        black: p.mana_pool.black,
                        red: p.mana_pool.red,
                        green: p.mana_pool.green,
                        colorless: p.mana_pool.colorless,
                    },
                    hand_size: p.hand.len(),
                    library_size: p.library.len(),
                    graveyard_size: p.graveyard.len(),
                    command_size: object_view_cache
                        .zone_cards(
                            game,
                            p.id,
                            perspective,
                            Zone::Command,
                            viewed_cards,
                            &active_grants,
                            potential_grants,
                        )
                        .len(),
                    ante_size: object_view_cache
                        .zone_cards(
                            game,
                            p.id,
                            perspective,
                            Zone::Ante,
                            viewed_cards,
                            &active_grants,
                            potential_grants,
                        )
                        .len(),
                }
            })
            .collect();
        let zone_transitions = zone_transition_snapshots(game, perspective, viewed_cards);
        let effect_events = effect_event_snapshots(game);
        let planechase = game.planechase.as_ref().map(|state| {
            let communal_deck = state.communal_deck.is_some();
            let deck_sizes = if let Some(deck) = state.communal_deck.as_ref() {
                vec![PlanarDeckSizeSnapshot {
                    owner: None,
                    size: deck.len(),
                }]
            } else {
                game.turn_store
                    .turn_order
                    .iter()
                    .map(|player| PlanarDeckSizeSnapshot {
                        owner: Some(player.0),
                        size: state.decks.get(player).map_or(0, Vec::len),
                    })
                    .collect()
            };
            let face_up = state
                .face_up
                .iter()
                .filter_map(|id| {
                    let object = game.object(*id)?;
                    let kind = match state.card_kinds.get(id)? {
                        ironsmith::game_state::PlanarCardKind::Plane => "plane",
                        ironsmith::game_state::PlanarCardKind::Phenomenon => "phenomenon",
                    };
                    Some(PlanarCardSnapshot {
                        id: id.0,
                        stable_id: object.stable_id.0.0,
                        name: object.name.to_string(),
                        kind: kind.to_string(),
                        controller: state
                            .face_up_controllers
                            .get(id)
                            .copied()
                            .unwrap_or(state.planar_controller)
                            .0,
                        oracle_text: object.compiled_card_text.to_string(),
                    })
                })
                .collect();
            PlanechaseSnapshot {
                planar_controller: state.planar_controller.0,
                planar_controllers: game
                    .planar_controllers()
                    .into_iter()
                    .map(|player| player.0)
                    .collect(),
                die_roll_cost: state
                    .voluntary_rolls_this_turn
                    .get(&state.planar_controller)
                    .copied()
                    .unwrap_or(0),
                planeswalk_count: state.planeswalk_count,
                communal_deck,
                deck_sizes,
                face_up,
            }
        });
        let vanguard = game.vanguard.as_ref().map(|state| {
            let mut cards = state
                .cards
                .iter()
                .filter_map(|(owner, id)| {
                    let object = game.object(*id)?;
                    Some(VanguardCardSnapshot {
                        owner: owner.0,
                        id: id.0,
                        stable_id: object.stable_id.0.0,
                        name: object.name.to_string(),
                        hand_modifier: object.hand_modifier,
                        life_modifier: object.life_modifier,
                        oracle_text: object.compiled_card_text.to_string(),
                    })
                })
                .collect::<Vec<_>>();
            cards.sort_by_key(|card| card.owner);
            VanguardSnapshot { cards }
        });
        let archenemy = game.archenemy.as_ref().map(|state| {
            let variant = match state.variant {
                ironsmith::game_state::ArchenemyVariant::Default => "default",
                ironsmith::game_state::ArchenemyVariant::SupervillainRumble => {
                    "supervillain_rumble"
                }
                ironsmith::game_state::ArchenemyVariant::Commander => "commander",
            }
            .to_string();
            let mut archenemies = state
                .archenemies
                .iter()
                .map(|player| player.0)
                .collect::<Vec<_>>();
            archenemies.sort_unstable();
            let mut deck_sizes = state
                .scheme_decks
                .iter()
                .map(|(owner, deck)| ArchenemyDeckSizeSnapshot {
                    owner: owner.0,
                    size: deck.len(),
                })
                .collect::<Vec<_>>();
            deck_sizes.sort_by_key(|deck| deck.owner);
            let face_up = state
                .face_up
                .iter()
                .filter_map(|id| {
                    let object = game.object(*id)?;
                    Some(SchemeCardSnapshot {
                        id: id.0,
                        stable_id: object.stable_id.0.0,
                        name: object.name.to_string(),
                        owner: object.owner.0,
                        ongoing: object
                            .supertypes
                            .contains(&ironsmith::types::Supertype::Ongoing),
                        oracle_text: object.compiled_card_text.to_string(),
                    })
                })
                .collect();
            ArchenemySnapshot {
                variant,
                archenemies,
                deck_sizes,
                face_up,
            }
        });
        let conspiracy = game.conspiracy.as_ref().map(|state| {
            let mut cards = state
                .cards
                .iter()
                .flat_map(|(owner, objects)| {
                    objects.iter().filter_map(|object_id| {
                        let object = game.object(*object_id)?;
                        let face_down = state.face_down.contains(object_id);
                        let visible = !face_down || *owner == perspective;
                        Some(ConspiracyCardSnapshot {
                            id: object_id.0,
                            stable_id: object.stable_id.0.0,
                            owner: owner.0,
                            face_down,
                            name: visible.then(|| object.name.to_string()),
                            oracle_text: visible.then(|| object.compiled_card_text.to_string()),
                            agenda_names: visible
                                .then(|| state.agenda_names.get(object_id).cloned())
                                .flatten(),
                        })
                    })
                })
                .collect::<Vec<_>>();
            cards.sort_by_key(|card| (card.owner, card.id));
            ConspiracySnapshot { cards }
        });
        let grand_melee = game.grand_melee().map(|state| GrandMeleeSnapshot {
            seats: state.seats().iter().map(|player| player.0).collect(),
            starting_player_count: state.starting_player_count(),
            focused_marker: state.focused_marker(),
            markers: game
                .grand_melee_marker_views()
                .into_iter()
                .map(|marker| GrandMeleeMarkerSnapshot {
                    number: marker.number,
                    holder: marker.holder.0,
                    status: match marker.status {
                        ironsmith::GrandMeleeMarkerStatus::Active => "active",
                        ironsmith::GrandMeleeMarkerStatus::Waiting => "waiting",
                    }
                    .to_string(),
                    stack_size: marker.stack_size,
                    removal_designations: marker.removal_designations,
                    normal_turn_pending: marker.normal_turn_pending,
                    retained_extra_turn_waiting: marker.retained_extra_turn_waiting,
                })
                .collect(),
        });

        let mut stack_preview: Vec<String> = game
            .stack
            .iter()
            .rev()
            .map(|entry| {
                game.object(entry.object_id)
                    .map(|obj| obj.name.to_string())
                    .unwrap_or_else(|| format!("Object#{}", entry.object_id.0))
            })
            .collect();
        let mut stack_objects: Vec<super::StackObjectSnapshot> = game
            .stack
            .iter()
            .rev()
            .map(|entry| super::build_stack_object_snapshot(game, perspective, viewed_cards, entry))
            .collect();
        let mut stack_size = game.stack.len();

        if let Some(stack_id) = pending_cast_stack_id
            && !game.stack.iter().any(|entry| entry.object_id == stack_id)
            && let Some(obj) = game.object(stack_id)
        {
            stack_preview.insert(0, obj.name.to_string());
            let pending_effect_text = {
                let lines: Vec<_> = obj
                    .compiled_card_text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .collect();
                if lines.is_empty() {
                    None
                } else {
                    Some(lines.join("; "))
                }
            };
            stack_objects.insert(
                0,
                super::StackObjectSnapshot {
                    id: stack_id.0,
                    inspect_object_id: Some(stack_id.0),
                    stable_id: Some(obj.stable_id.0.0),
                    source_stable_id: None,
                    controller: game.current_controller(stack_id).unwrap_or(obj.owner).0,
                    name: obj.name.to_string(),
                    mana_cost: obj.mana_cost.as_ref().map(|mc| mc.to_oracle()),
                    effect_text: pending_effect_text,
                    ability_kind: None,
                    ability_text: None,
                    source_ability_text: None,
                    targets: Vec::new(),
                },
            );
            stack_size += 1;
        }
        Self {
            snapshot_id,
            perspective: perspective.0,
            turn_number: game.turn.turn_number,
            active_player: game.turn.active_player.0,
            active_players: game
                .active_players()
                .into_iter()
                .map(|player| player.0)
                .collect(),
            priority_player: game.turn.priority_player.map(|p| p.0),
            priority_team_players: game
                .priority_team_players()
                .into_iter()
                .map(|player| player.0)
                .collect(),
            phase: game.turn.phase.to_string(),
            step: game.turn.step.map(|step| step.to_string()),
            stack_size,
            subgame_depth: game.subgame_depth(),
            subgame_starting_procedure_pending: game.subgame_starting_procedure_pending(),
            stack_preview,
            stack_objects,
            resolving_stack_object,
            battlefield_size: game.battlefield.len(),
            exile_size: game.exile.len(),
            players,
            planechase,
            vanguard,
            archenemy,
            conspiracy,
            grand_melee,
            battlefield_transitions,
            zone_transitions,
            effect_events,
            crypto_requirements: Vec::new(),
            viewed_cards: viewed_cards
                .filter(|view| {
                    view.public
                        || view.viewer == perspective
                        || (view.zone != Zone::OutsideGame
                            && game.controlling_player_for(view.viewer) == perspective)
                })
                .map(|view| ViewedCardsSnapshot {
                    viewer: view.viewer.0,
                    subject: view.subject.0,
                    zone: view.zone.to_string(),
                    visibility: if view.public {
                        "public".to_string()
                    } else {
                        "private".to_string()
                    },
                    cards: view
                        .cards
                        .iter()
                        .enumerate()
                        .map(|(index, id)| {
                            let (current_id, stable_id, name, oracle_text) =
                                resolve_viewed_card(game, *id, view.stable_id_at(index, *id));
                            ViewedCardSnapshot {
                                id: current_id.0,
                                stable_id,
                                name,
                                oracle_text,
                            }
                        })
                        .collect(),
                    card_ids: view
                        .cards
                        .iter()
                        .enumerate()
                        .map(|(index, id)| {
                            resolve_viewed_card(game, *id, view.stable_id_at(index, *id))
                                .0
                                .0
                        })
                        .collect(),
                    source: view.source.map(|id| id.0),
                    description: view.description.clone(),
                }),
            decision: decision.map(|ctx| {
                DecisionView::from_context(
                    game,
                    ctx,
                    perspective,
                    viewed_cards,
                    undo_land_stable_id,
                )
            }),
            mana_payment,
            game_over: game_over.map(|r| GameOverView::from_result(game, r)),
            cancelable,
            undo_land_stable_id,
        }
    }
}

pub(super) fn build_object_details_snapshot(
    game: &GameState,
    id: ObjectId,
    definition: Option<&CardDefinition>,
) -> Option<ObjectDetailsSnapshot> {
    let obj = game.object(id)?;
    let current_name = game
        .current_name(id)
        .unwrap_or_else(|| obj.name.to_string());
    let current_controller = game.current_controller(id).unwrap_or(obj.owner);
    let current_supertypes = game
        .current_supertypes(id)
        .unwrap_or_else(|| obj.supertypes.to_vec());
    let current_card_types = game
        .current_card_types(id)
        .unwrap_or_else(|| obj.card_types.to_vec());
    let current_subtypes = game
        .current_subtypes(id)
        .unwrap_or_else(|| obj.subtypes.to_vec());
    let (power, toughness) = if obj.zone == Zone::Battlefield {
        (
            game.calculated_power(id).or_else(|| obj.power()),
            game.calculated_toughness(id).or_else(|| obj.toughness()),
        )
    } else {
        (obj.power(), obj.toughness())
    };
    let counters = counter_snapshots_for_object(obj);
    let compiled_text = ironsmith::runtime_display::compiled_text_lines(&obj.to_card_definition());

    let type_line =
        format_type_line_parts(&current_supertypes, &current_card_types, &current_subtypes);
    let (type_line_display, type_line_badges) = format_type_line_display_parts(
        &current_supertypes,
        &current_card_types,
        &current_subtypes,
        &obj.subtypes,
    );

    let current_abilities = game
        .current_abilities(id)
        .unwrap_or_else(|| obj.abilities_vec());
    let abilities =
        ironsmith::runtime_display::current_ability_surface_texts(&current_abilities, definition);

    Some(ObjectDetailsSnapshot {
        id: obj.id.0,
        stable_id: obj.stable_id.0.0,
        name: current_name,
        kind: obj.kind.to_string(),
        zone: zone_label(obj.zone),
        owner: obj.owner.0,
        controller: current_controller.0,
        type_line,
        type_line_display,
        type_line_badges,
        mana_cost: obj.mana_cost.as_ref().map(|cost| cost.to_oracle()),
        oracle_text: obj.compiled_card_text.to_string(),
        power,
        toughness,
        loyalty: obj.loyalty(),
        tapped: game.is_tapped(obj.id),
        counters,
        compiled_text,
        abilities,
        chosen_color: game
            .chosen_color(obj.id)
            .map(|color| color.name().to_string()),
        raw_compilation: format!("{:#?}", obj.to_card_definition()),
        semantic_score: CardRegistry::generated_parser_semantic_score(obj.name.as_str()),
    })
}

fn format_type_line_parts(
    supertypes: &[ironsmith::types::Supertype],
    card_types: &[ironsmith::types::CardType],
    subtypes: &[ironsmith::types::Subtype],
) -> String {
    let mut left = Vec::new();
    left.extend(supertypes.iter().map(|value| format!("{value:?}")));
    left.extend(card_types.iter().map(|value| format!("{value:?}")));

    let mut type_line = left.join(" ");
    if !subtypes.is_empty() {
        let subtypes = subtypes
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(" ");
        if type_line.is_empty() {
            type_line = subtypes;
        } else {
            type_line.push_str(" - ");
            type_line.push_str(&subtypes);
        }
    }

    if type_line.is_empty() {
        "Object".to_string()
    } else {
        type_line
    }
}

fn format_type_line_display_parts(
    supertypes: &[ironsmith::types::Supertype],
    card_types: &[ironsmith::types::CardType],
    subtypes: &[ironsmith::types::Subtype],
    printed_subtypes: &[ironsmith::types::Subtype],
) -> (String, Vec<String>) {
    if object_has_all_creature_types(card_types, subtypes) {
        let display_subtypes =
            compact_all_creature_type_display_subtypes(printed_subtypes, subtypes);
        return (
            format_type_line_parts(supertypes, card_types, &display_subtypes),
            vec!["All creature types".to_string()],
        );
    }

    (
        format_type_line_parts(supertypes, card_types, subtypes),
        Vec::new(),
    )
}

fn object_has_all_creature_types(
    card_types: &[ironsmith::types::CardType],
    subtypes: &[ironsmith::types::Subtype],
) -> bool {
    let can_have_creature_types = card_types
        .iter()
        .any(|card_type| matches!(card_type, CardType::Creature | CardType::Kindred));
    can_have_creature_types
        && Subtype::all_creature_types()
            .iter()
            .all(|subtype| subtypes.contains(subtype))
}

fn compact_all_creature_type_display_subtypes(
    printed_subtypes: &[ironsmith::types::Subtype],
    current_subtypes: &[ironsmith::types::Subtype],
) -> Vec<ironsmith::types::Subtype> {
    let mut display_subtypes = Vec::new();
    for subtype in printed_subtypes.iter().chain(
        current_subtypes
            .iter()
            .filter(|subtype| !subtype.is_creature_type()),
    ) {
        if current_subtypes.contains(subtype) && !display_subtypes.contains(subtype) {
            display_subtypes.push(*subtype);
        }
    }
    display_subtypes
}

fn zone_label(zone: Zone) -> String {
    match zone {
        Zone::Library => "library",
        Zone::Hand => "hand",
        Zone::Battlefield => "battlefield",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Stack => "stack",
        Zone::Command => "command",
        Zone::Ante => "ante",
        Zone::OutsideGame => "outside_game",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith::alternative_cast::AlternativeCastingMethod;
    use ironsmith::card::{Card, CardBuilder, PowerToughness};
    use ironsmith::cards::tokens::cursed_role_token_definition;
    use ironsmith::decisions::context::{DecisionContext, SelectObjectsContext, SelectableObject};
    use ironsmith::game_state::{GameState, PlayerControlDuration, PlayerControlStart};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::object::AttachmentTarget;
    use ironsmith::types::Subtype;
    use ironsmith_registry_test::cards::builders::CardDefinitionBuilder;

    fn test_bears_card() -> Card {
        CardBuilder::new(CardId::from_raw(90_001), "Grizzly Bears")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn add_native_escape_to_object(game: &mut GameState, id: ironsmith::ids::ObjectId) {
        game.object_mut(id)
            .expect("escape object should exist")
            .alternative_casts
            .push(AlternativeCastingMethod::Escape {
                cost: Some(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Red],
                    vec![ManaSymbol::Red],
                ])),
                exile_count: 2,
                additional_cost: ironsmith::TotalCost::from_cost(
                    ironsmith::costs::Cost::exile_from_graveyard(2, None),
                ),
            });
    }

    #[test]
    fn snapshot_with_two_conditionally_animated_artifacts_terminates() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let animate_artifact =
            CardDefinitionBuilder::new(CardId::from_raw(90_030), "Animate Artifact")
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(1)],
                    vec![ManaSymbol::Blue],
                ]))
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Aura])
                .parse_text(
                    "Enchant artifact\nAs long as enchanted artifact isn't a creature, it's an artifact creature with power and toughness each equal to its mana value.",
                )
                .expect("Animate Artifact should parse");
        let mine = CardBuilder::new(CardId::from_raw(90_031), "Howling Mine")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Artifact])
            .build();

        let mine_one = game.create_object_from_card(&mine, alice, Zone::Battlefield);
        let mine_two = game.create_object_from_card(&mine, alice, Zone::Battlefield);
        let aura_one =
            game.create_object_from_definition(&animate_artifact, alice, Zone::Battlefield);
        let aura_two =
            game.create_object_from_definition(&animate_artifact, alice, Zone::Battlefield);
        for (aura, host) in [(aura_one, mine_one), (aura_two, mine_two)] {
            game.object_mut(aura).unwrap().attached_to = Some(AttachmentTarget::Object(host));
            game.object_mut(host).unwrap().attachments.push(aura);
        }

        let snapshot = GameSnapshot::from_game(
            &game,
            alice,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let alice_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("Alice snapshot should exist");
        assert!(
            alice_snapshot.battlefield_total >= 2,
            "animated mines should appear on the battlefield snapshot"
        );
    }

    #[test]
    fn visible_hand_snapshots_include_oracle_text() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let definition = CardDefinitionBuilder::new(CardId::from_raw(90_012), "Hand Flying Probe")
            .card_types(vec![CardType::Creature])
            .flying()
            .build();
        let object_id = game.create_object_from_definition(&definition, alice, Zone::Hand);

        let snapshot = GameSnapshot::from_game(
            &game,
            alice,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let alice_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("Alice snapshot should exist");
        let card = alice_snapshot
            .hand_cards
            .iter()
            .find(|card| card.id == object_id.0)
            .expect("visible hand card should be in snapshot");

        assert!(
            card.oracle_text.contains("Flying"),
            "hand snapshots should carry oracle text for inspector fallback"
        );
    }

    #[test]
    fn team_vs_team_snapshots_reveal_hands_to_teammates_only() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Diana".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        game.restore_team_vs_team(
            vec![vec![alice, bob], vec![charlie, PlayerId::from_index(3)]],
            vec![alice, bob, charlie, PlayerId::from_index(3)],
            0,
            alice,
        )
        .expect("Team vs. Team profile");
        let card = CardBuilder::new(CardId::from_raw(90_808), "Teammate Secret")
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, bob, Zone::Hand);

        let alice_snapshot = GameSnapshot::from_game(
            &game,
            alice,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let bob_for_alice = alice_snapshot
            .players
            .iter()
            .find(|player| player.id == bob.0)
            .expect("Bob in teammate snapshot");
        assert!(bob_for_alice.can_view_hand);
        assert_eq!(bob_for_alice.hand_cards[0].name, "Teammate Secret");

        let charlie_snapshot = GameSnapshot::from_game(
            &game,
            charlie,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let bob_for_charlie = charlie_snapshot
            .players
            .iter()
            .find(|player| player.id == bob.0)
            .expect("Bob in opponent snapshot");
        assert!(!bob_for_charlie.can_view_hand);
        assert!(bob_for_charlie.hand_cards.is_empty());
    }

    #[test]
    fn emperor_snapshots_reveal_hands_to_teammates_only() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new((0..6).map(|index| format!("Player {index}")).collect(), 20);
        let seats = (0..6)
            .map(|index| PlayerId::from_index(index as u8))
            .collect::<Vec<_>>();
        game.restore_emperor(
            vec![seats[0..3].to_vec(), seats[3..6].to_vec()],
            seats.clone(),
            0,
            seats[1],
            vec![1, 2, 1, 1, 2, 1],
        )
        .expect("Emperor profile");
        let card = CardBuilder::new(CardId::from_raw(90_809), "General Secret")
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, seats[2], Zone::Hand);

        let snapshot_for = |perspective| {
            GameSnapshot::from_game(
                &game,
                perspective,
                None,
                None,
                None,
                None,
                None,
                Vec::new(),
                None,
                false,
                None,
                0,
            )
        };
        let teammate = snapshot_for(seats[0]);
        let hand = teammate
            .players
            .iter()
            .find(|player| player.id == seats[2].0)
            .expect("teammate hand");
        assert!(hand.can_view_hand);
        assert_eq!(hand.hand_cards[0].name, "General Secret");

        let opponent = snapshot_for(seats[3]);
        let hand = opponent
            .players
            .iter()
            .find(|player| player.id == seats[2].0)
            .expect("opponent hand");
        assert!(!hand.can_view_hand);
        assert!(hand.hand_cards.is_empty());
    }

    #[test]
    fn two_headed_giant_snapshots_reveal_teammate_hands_and_shared_pools() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new((0..4).map(|index| format!("Player {index}")).collect(), 20);
        let seats = (0..4)
            .map(|index| PlayerId::from_index(index as u8))
            .collect::<Vec<_>>();
        game.set_random_seed(810);
        game.enable_two_headed_giant(vec![seats[0..2].to_vec(), seats[2..4].to_vec()])
            .expect("Two-Headed Giant profile");
        game.lose_life(seats[0], 6);
        game.add_player_counters_with_source(
            seats[1],
            ironsmith::CounterType::Poison,
            3,
            None,
            None,
        );
        let card = CardBuilder::new(CardId::from_raw(90_810), "Other Head Secret")
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, seats[1], Zone::Hand);

        let snapshot_for = |perspective| {
            GameSnapshot::from_game(
                &game,
                perspective,
                None,
                None,
                None,
                None,
                None,
                Vec::new(),
                None,
                false,
                None,
                0,
            )
        };
        let teammate = snapshot_for(seats[0]);
        let head = teammate
            .players
            .iter()
            .find(|player| player.id == seats[1].0)
            .expect("teammate snapshot");
        assert!(head.can_view_hand);
        assert_eq!(head.hand_cards[0].name, "Other Head Secret");
        assert_eq!(head.life, 24);
        assert_eq!(head.poison_counters, 3);

        let opponent = snapshot_for(seats[2]);
        let head = opponent
            .players
            .iter()
            .find(|player| player.id == seats[1].0)
            .expect("opponent snapshot");
        assert!(!head.can_view_hand);
        assert!(head.hand_cards.is_empty());
        assert_eq!(head.life, 24);
        assert_eq!(head.poison_counters, 3);
    }

    #[test]
    fn alternating_teams_snapshots_keep_nonadjacent_teammate_hands_hidden() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new((0..4).map(|index| format!("Player {index}")).collect(), 20);
        let seats = (0..4)
            .map(|index| PlayerId::from_index(index as u8))
            .collect::<Vec<_>>();
        game.restore_alternating_teams(
            vec![vec![seats[0], seats[1]], vec![seats[2], seats[3]]],
            vec![seats[0], seats[2], seats[1], seats[3]],
            seats[0],
            ironsmith::FreeForAllAttackOption::MultiplePlayers,
            Some(2),
            false,
        )
        .expect("Alternating Teams profile");
        let card = CardBuilder::new(CardId::from_raw(90_811), "Distant Teammate Secret")
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, seats[1], Zone::Hand);

        let snapshot = GameSnapshot::from_game(
            &game,
            seats[0],
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let teammate = snapshot
            .players
            .iter()
            .find(|player| player.id == seats[1].0)
            .expect("teammate snapshot");
        assert!(!teammate.can_view_hand);
        assert!(teammate.hand_cards.is_empty());
    }

    #[test]
    fn visible_zone_snapshots_include_oracle_text() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = CardDefinitionBuilder::new(CardId::from_raw(90_013), "Zone Flying Probe")
            .card_types(vec![CardType::Creature])
            .flying()
            .build();
        let object_id = game.create_object_from_definition(&definition, bob, Zone::Graveyard);

        let snapshot = GameSnapshot::from_game(
            &game,
            alice,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let bob_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == bob.0)
            .expect("Bob snapshot should exist");
        let card = bob_snapshot
            .graveyard_cards
            .iter()
            .find(|card| card.id == object_id.0)
            .expect("visible graveyard card should be in snapshot");

        assert!(
            card.oracle_text.contains("Flying"),
            "zone snapshots should carry oracle text for inspector fallback"
        );
    }

    #[test]
    fn ante_cards_are_in_every_perspectives_public_zone_snapshot() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = CardDefinitionBuilder::new(CardId::from_raw(407_002), "Ante Snapshot")
            .card_types(vec![CardType::Artifact])
            .build();
        let object_id = game.create_object_from_definition(&definition, alice, Zone::Ante);

        for perspective in [alice, bob] {
            let snapshot = GameSnapshot::from_game(
                &game,
                perspective,
                None,
                None,
                None,
                None,
                None,
                Vec::new(),
                None,
                false,
                None,
                0,
            );
            let alice_snapshot = snapshot
                .players
                .iter()
                .find(|player| player.id == alice.0)
                .expect("Alice snapshot should exist");
            assert_eq!(alice_snapshot.ante_size, 1);
            assert_eq!(alice_snapshot.ante_cards.len(), 1);
            assert_eq!(alice_snapshot.ante_cards[0].id, object_id.0);
        }
    }

    #[test]
    fn pseudo_hand_hides_opponent_native_escape_card() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let escape_card = CardBuilder::new(CardId::from_raw(90_010), "Escape Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        let escape_id = game.create_object_from_card(&escape_card, bob, Zone::Graveyard);
        add_native_escape_to_object(&mut game, escape_id);

        let snapshot = GameSnapshot::from_game(
            &game,
            alice,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let bob_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == bob.0)
            .expect("opponent snapshot should exist");
        let graveyard_card = bob_snapshot
            .graveyard_cards
            .iter()
            .find(|card| card.id == escape_id.0)
            .expect("opponent graveyard card should be visible");

        assert!(
            !graveyard_card.show_in_pseudo_hand,
            "an opponent-owned native escape card should not be surfaced in Alice's pseudo-hand"
        );
        assert_eq!(graveyard_card.pseudo_hand_glow_kind, None);
    }

    #[test]
    fn pseudo_hand_keeps_own_native_escape_card() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let escape_card = CardBuilder::new(CardId::from_raw(90_011), "Own Escape Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        let escape_id = game.create_object_from_card(&escape_card, alice, Zone::Graveyard);
        add_native_escape_to_object(&mut game, escape_id);

        let snapshot = GameSnapshot::from_game(
            &game,
            alice,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let alice_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("Alice snapshot should exist");
        let graveyard_card = alice_snapshot
            .graveyard_cards
            .iter()
            .find(|card| card.id == escape_id.0)
            .expect("own graveyard card should be visible");

        assert!(
            graveyard_card.show_in_pseudo_hand,
            "an owned native escape card should still be surfaced in Alice's pseudo-hand"
        );
        assert_eq!(
            graveyard_card.pseudo_hand_glow_kind.as_deref(),
            Some("extra")
        );
    }

    #[test]
    fn decision_snapshot_routes_controlled_player_prompt_to_controller() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::from_raw(90_012), "Primeval Titan")
            .card_types(vec![CardType::Creature])
            .build();
        let card_id = game.create_object_from_card(&card, alice, Zone::Hand);
        let future_sight = CardDefinitionBuilder::new(CardId::from_raw(90_014), "Future Sight")
            .card_types(vec![CardType::Enchantment])
            .with_ability(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::look_at_top_card_of_library(),
            ))
            .build();
        game.create_object_from_definition(&future_sight, alice, Zone::Battlefield);
        let top_card = CardBuilder::new(CardId::from_raw(90_015), "Alice's Future")
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&top_card, alice, Zone::Library);

        game.add_player_control(
            bob,
            alice,
            PlayerControlStart::Immediate,
            PlayerControlDuration::UntilEndOfTurn,
            None,
        );

        let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
            alice,
            None,
            "Choose a card",
            vec![SelectableObject::new(card_id, "Primeval Titan")],
            1,
            Some(1),
        ));
        let snapshot = GameSnapshot::from_game(
            &game,
            bob,
            Some(&decision),
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );

        match snapshot
            .decision
            .as_ref()
            .expect("decision should be present")
        {
            DecisionView::SelectObjects {
                player, candidates, ..
            } => {
                assert_eq!(
                    *player, bob.0,
                    "the browser prompt should belong to the controlling player"
                );
                assert_eq!(candidates[0].id, card_id.0);
                assert_eq!(candidates[0].name, "Primeval Titan");
            }
            other => panic!("expected select-object decision, got {other:?}"),
        }

        let alice_in_bob_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("Alice snapshot should exist");
        assert!(alice_in_bob_snapshot.can_view_hand);
        assert_eq!(alice_in_bob_snapshot.hand_cards[0].name, "Primeval Titan");
        assert!(alice_in_bob_snapshot.can_view_library_top);
        assert_eq!(
            alice_in_bob_snapshot.library_top.as_deref(),
            Some("Alice's Future")
        );

        let alice_snapshot = GameSnapshot::from_game(
            &game,
            alice,
            Some(&decision),
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let alice_player = alice_snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("Alice snapshot should exist");
        assert!(alice_player.can_view_hand);
        assert_eq!(alice_player.hand_cards[0].name, "Primeval Titan");
        assert!(alice_player.can_view_library_top);
        assert_eq!(alice_player.library_top.as_deref(), Some("Alice's Future"));
        assert!(
            alice_player
                .persistent_look_cards
                .iter()
                .any(|card| card.name == "Alice's Future")
        );
    }

    #[test]
    fn controlled_player_keeps_outside_game_identity_private_from_controller() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let lesson = CardBuilder::new(CardId::from_raw(90_013), "Environmental Sciences")
            .card_types(vec![CardType::Sorcery])
            .build();
        let lesson_id = game.create_object_from_card(&lesson, alice, Zone::OutsideGame);

        game.add_player_control(
            bob,
            alice,
            PlayerControlStart::Immediate,
            PlayerControlDuration::UntilEndOfTurn,
            None,
        );

        let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
            alice,
            None,
            "Choose a Lesson you own from outside the game",
            vec![SelectableObject::new(lesson_id, "Environmental Sciences")],
            1,
            Some(1),
        ));
        let bob_snapshot = GameSnapshot::from_game(
            &game,
            bob,
            Some(&decision),
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );

        let bob_candidate = match bob_snapshot.decision.as_ref().expect("decision") {
            DecisionView::SelectObjects {
                player, candidates, ..
            } => {
                assert_eq!(*player, bob.0);
                &candidates[0]
            }
            other => panic!("expected select-object decision, got {other:?}"),
        };
        assert_eq!(bob_candidate.name, "Hidden card");
        assert_ne!(bob_candidate.id, lesson_id.0);
        assert_eq!(bob_candidate.selection_identity, "hidden_reference");
        assert_eq!(
            bob_candidate
                .hidden_ref
                .as_ref()
                .and_then(|hidden_ref| hidden_ref.zone.as_deref()),
            Some("outside_game")
        );
        assert!(
            bob_snapshot
                .players
                .iter()
                .find(|player| player.id == alice.0)
                .expect("Alice snapshot should exist")
                .sideboard_cards
                .is_empty()
        );

        let alice_snapshot = GameSnapshot::from_game(
            &game,
            alice,
            Some(&decision),
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let alice_sideboard = &alice_snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("Alice snapshot should exist")
            .sideboard_cards;
        assert_eq!(alice_sideboard.len(), 1);
        assert_eq!(alice_sideboard[0].name, "Environmental Sciences");
        let alice_candidate = match alice_snapshot.decision.as_ref().expect("decision") {
            DecisionView::SelectObjects { candidates, .. } => &candidates[0],
            other => panic!("expected select-object decision, got {other:?}"),
        };
        assert_eq!(alice_candidate.name, "Environmental Sciences");
        assert_eq!(alice_candidate.id, lesson_id.0);
    }

    #[test]
    fn battlefield_grouping_uses_calculated_characteristics_and_matching_attachments() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let protected_ids = std::collections::HashSet::new();
        let bears_card = test_bears_card();
        let role_def = cursed_role_token_definition();

        let unenchanted_bear = game.create_object_from_card(&bears_card, alice, Zone::Battlefield);
        let enchanted_bears: Vec<_> = (0..3)
            .map(|_| {
                let bear = game.create_object_from_card(&bears_card, alice, Zone::Battlefield);
                let role = game.create_object_from_definition(&role_def, alice, Zone::Battlefield);
                assert!(
                    game.attach_object_to_target(role, AttachmentTarget::Object(bear)),
                    "role should attach to bear"
                );
                bear
            })
            .collect();

        assert_eq!(
            game.current_power(unenchanted_bear),
            Some(2),
            "control bear should keep printed power"
        );
        assert_eq!(
            battlefield_lane_for_card_types(
                &game
                    .object(unenchanted_bear)
                    .expect("bear should exist")
                    .card_types
            ),
            BattlefieldLane::Creatures
        );
        assert!(
            enchanted_bears
                .iter()
                .all(|bear| game.current_power(*bear) == Some(1)),
            "cursed roles should set each attached bear to 1/1"
        );

        let (battlefield, _) = grouped_battlefield_for_player(&game, alice, &protected_ids);
        let bear_groups: Vec<_> = battlefield
            .iter()
            .filter(|permanent| permanent.name == "Grizzly Bears")
            .collect();

        assert_eq!(
            bear_groups.len(),
            2,
            "unenchanted and cursed bears should not collapse together"
        );
        assert!(
            bear_groups
                .iter()
                .any(|group| group.count == 1 && group.power_toughness.as_deref() == Some("2/2")),
            "single unenchanted bear should stay in its own group: {bear_groups:?}"
        );
        assert!(
            bear_groups
                .iter()
                .any(|group| group.count == 3 && group.power_toughness.as_deref() == Some("1/1")),
            "matching cursed bears should group together: {bear_groups:?}"
        );
    }

    #[test]
    fn battlefield_grouping_splits_each_protected_legal_target() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bears_card = test_bears_card();
        let role_def = cursed_role_token_definition();
        let mut protected_ids = std::collections::HashSet::new();

        for _ in 0..3 {
            let bear = game.create_object_from_card(&bears_card, alice, Zone::Battlefield);
            let role = game.create_object_from_definition(&role_def, alice, Zone::Battlefield);
            assert!(
                game.attach_object_to_target(role, AttachmentTarget::Object(bear)),
                "role should attach to bear"
            );
            protected_ids.insert(bear);
        }

        let (battlefield, _) = grouped_battlefield_for_player(&game, alice, &protected_ids);
        let bear_groups: Vec<_> = battlefield
            .iter()
            .filter(|permanent| permanent.name == "Grizzly Bears")
            .collect();

        assert_eq!(
            bear_groups.len(),
            3,
            "every legal target should stay individually clickable"
        );
        assert!(
            bear_groups.iter().all(|group| group.count == 1),
            "protected legal targets should not be grouped: {bear_groups:?}"
        );
    }

    #[test]
    fn snapshot_object_view_cache_refreshes_tapped_state() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let protected_ids = std::collections::HashSet::new();
        let bears_card = test_bears_card();
        let bear = game.create_object_from_card(&bears_card, alice, Zone::Battlefield);
        let object_view_cache = SnapshotObjectViewCache::default();

        let (untapped_battlefield, _) = grouped_battlefield_for_player_with_cache(
            &game,
            alice,
            &protected_ids,
            &object_view_cache,
        );
        assert_eq!(
            object_view_cache.battlefield.borrow().len(),
            1,
            "first snapshot should populate one cached permanent view"
        );
        assert!(
            untapped_battlefield
                .iter()
                .any(|permanent| permanent.member_ids.contains(&bear.0) && !permanent.tapped),
            "initial cached view should show the bear as untapped: {untapped_battlefield:?}"
        );

        game.tap(bear);
        let (tapped_battlefield, _) = grouped_battlefield_for_player_with_cache(
            &game,
            alice,
            &protected_ids,
            &object_view_cache,
        );

        assert_eq!(
            object_view_cache.battlefield.borrow().len(),
            2,
            "a changed per-object key should create a fresh cached view"
        );
        assert!(
            tapped_battlefield
                .iter()
                .any(|permanent| permanent.member_ids.contains(&bear.0) && permanent.tapped),
            "second snapshot should not reuse the stale untapped view: {tapped_battlefield:?}"
        );
    }

    #[test]
    fn incremental_zone_views_retain_arrays_and_render_only_changed_raw_cards() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let card = test_bears_card();
        let hand: Vec<_> = (0..128)
            .map(|_| game.create_object_from_card(&card, alice, Zone::Hand))
            .collect();
        let grave: Vec<_> = (0..128)
            .map(|_| game.create_object_from_card(&card, alice, Zone::Graveyard))
            .collect();
        let cache = SnapshotObjectViewCache::default();
        let grants = std::cell::OnceCell::new();
        let _ = grants.set(Vec::new());
        let first_hand = cache.hand_cards(&game, alice, alice, None, 2);
        let first_grave =
            cache.zone_cards(&game, alice, alice, Zone::Graveyard, None, &grants, false);
        assert!(Arc::ptr_eq(
            &first_hand,
            &cache.hand_cards(&game, alice, alice, None, 2)
        ));
        assert!(Arc::ptr_eq(
            &first_grave,
            &cache.zone_cards(&game, alice, alice, Zone::Graveyard, None, &grants, false)
        ));
        assert_eq!(cache.hands.borrow()[&alice].rendered, 128);
        game.object_mut(hand[19]).unwrap().name = "Renamed hand card".into();
        let second_hand = cache.hand_cards(&game, alice, alice, None, 2);
        assert_eq!(cache.hands.borrow()[&alice].rendered, 129);
        assert!(Arc::ptr_eq(&first_hand[0], &second_hand[0]));
        assert_eq!(
            second_hand
                .iter()
                .find(|card| card.id == hand[19].0)
                .unwrap()
                .name,
            "Renamed hand card"
        );
        assert!(Arc::ptr_eq(
            &first_grave,
            &cache.zone_cards(&game, alice, alice, Zone::Graveyard, None, &grants, false)
        ));
        game.object_mut(grave[7]).unwrap().name = "Renamed graveyard card".into();
        let second_grave =
            cache.zone_cards(&game, alice, alice, Zone::Graveyard, None, &grants, false);
        assert_eq!(
            cache.zones.borrow()[&(alice, Zone::Graveyard)].rendered,
            129
        );
        assert!(Arc::ptr_eq(&first_grave[0], &second_grave[0]));
        let expected: Vec<_> = game.players[0]
            .graveyard
            .iter()
            .rev()
            .map(|id| {
                build_zone_card_snapshot(
                    &game,
                    alice,
                    None,
                    game.object(*id).unwrap(),
                    Zone::Graveyard,
                )
            })
            .collect();
        assert_eq!(
            serde_json::to_value(second_grave).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }

    #[test]
    fn incremental_exile_visibility_handles_grants_expiry_perspective_and_rollback() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = test_bears_card();
        let first = game.create_object_from_card(&card, alice, Zone::Exile);
        let second = game.create_object_from_card(&card, alice, Zone::Exile);
        game.set_face_down(first);
        game.set_face_down(second);
        let cache = SnapshotObjectViewCache::default();
        let grants = std::cell::OnceCell::new();
        let _ = grants.set(Vec::new());
        let hidden = cache.zone_cards(&game, alice, bob, Zone::Exile, None, &grants, false);
        assert!(
            hidden
                .iter()
                .all(|card| card.oracle_text.is_empty() && card.name == hidden_object_label())
        );
        let checkpoint = game.clone();
        game.grant_face_down_exile_view(first, bob);
        let visible = cache.zone_cards(&game, alice, bob, Zone::Exile, None, &grants, false);
        assert_eq!(
            cache.zones.borrow()[&(alice, Zone::Exile)].rendered,
            3,
            "one permission grant should re-render one card"
        );
        assert_eq!(
            visible.iter().find(|card| card.id == first.0).unwrap().name,
            "Grizzly Bears"
        );
        assert_eq!(
            visible
                .iter()
                .find(|card| card.id == second.0)
                .unwrap()
                .name,
            hidden_object_label()
        );
        let alice_view = cache.zone_cards(&game, alice, alice, Zone::Exile, None, &grants, false);
        assert!(
            alice_view
                .iter()
                .all(|card| card.name == hidden_object_label())
        );
        let view = ActiveViewedCards {
            viewer: bob,
            subject: alice,
            zone: Zone::Exile,
            cards: vec![second],
            card_stable_ids: vec![game.object(second).unwrap().stable_id],
            public: false,
            source: None,
            description: "Temporary reveal".into(),
        };
        let both = cache.zone_cards(&game, alice, bob, Zone::Exile, Some(&view), &grants, false);
        assert!(both.iter().all(|card| card.name == "Grizzly Bears"));
        let expired = cache.zone_cards(&game, alice, bob, Zone::Exile, None, &grants, false);
        assert_eq!(
            expired
                .iter()
                .find(|card| card.id == second.0)
                .unwrap()
                .name,
            hidden_object_label()
        );
        game = checkpoint;
        let restored = cache.zone_cards(&game, alice, bob, Zone::Exile, None, &grants, false);
        assert!(
            restored
                .iter()
                .all(|card| card.name == hidden_object_label())
        );
    }

    #[test]
    fn attachment_cycles_have_finite_group_signatures() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let card = test_bears_card();
        let first = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let second = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(first).unwrap().attached_to = Some(AttachmentTarget::Object(second));
        game.object_mut(second).unwrap().attached_to = Some(AttachmentTarget::Object(first));
        let signature = object_characteristic_signature(&game, game.object(first).unwrap(), true);
        assert!(signature.contains("attachment_cycle:"));
        assert!(signature.len() < 10000);
    }

    #[test]
    fn incremental_visibility_matches_full_rules_through_control_and_removal() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardDefinitionBuilder::new(CardId::from_raw(9986), "Visibility source")
            .card_types(vec![CardType::Enchantment])
            .with_ability(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::look_at_top_card_of_library(),
            ))
            .with_ability(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::opponents_play_with_hands_revealed(),
            ))
            .build();
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let views = SnapshotObjectViewCache::default();
        let mut groups = IncrementalBattlefieldGroups::default();
        let compare = |game: &GameState, groups: &mut IncrementalBattlefieldGroups| {
            groups.update(game, &HashSet::new(), &views);
            for perspective in [alice, bob] {
                for player in [alice, bob] {
                    assert_eq!(
                        groups.visibility_for_player(game, perspective, player),
                        (
                            can_view_library_top(game, perspective, player),
                            hand_revealed_by_static_ability(game, player)
                        )
                    );
                }
            }
        };
        compare(&game, &mut groups);
        game.set_current_controller(source, bob);
        compare(&game, &mut groups);
        let checkpoint = game.clone();
        game.remove_object(source);
        compare(&game, &mut groups);
        game = checkpoint;
        compare(&game, &mut groups);
    }

    #[test]
    fn incremental_attachment_groups_match_full_rebuild_after_local_and_global_edits() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = test_bears_card();
        let bear = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let other = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let role = game.create_object_from_definition(
            &cursed_role_token_definition(),
            alice,
            Zone::Battlefield,
        );
        assert!(game.attach_object_to_target(role, AttachmentTarget::Object(bear)));
        let views = SnapshotObjectViewCache::default();
        let mut groups = IncrementalBattlefieldGroups::default();
        let protected = HashSet::new();
        let compare = |game: &GameState, groups: &mut IncrementalBattlefieldGroups| {
            groups.update(game, &protected, &views);
            for player in [alice, bob] {
                let (full, _) = grouped_battlefield_for_player(game, player, &protected);
                assert_eq!(
                    serde_json::to_value(groups.for_player(player).0).unwrap(),
                    serde_json::to_value(full).unwrap()
                );
            }
        };
        compare(&game, &mut groups);
        game.tap(role);
        compare(&game, &mut groups);
        assert!(game.attach_object_to_target(role, AttachmentTarget::Object(other)));
        compare(&game, &mut groups);
        game.set_current_controller(role, bob);
        compare(&game, &mut groups);
        game.remove_object(role);
        compare(&game, &mut groups);
    }

    #[test]
    fn incremental_groups_reuse_unchanged_members_and_match_full_rebuild_after_rollback() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let card = test_bears_card();
        let first = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let second = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.refresh_continuous_state();
        let protected = HashSet::from([first, second]);
        let views = SnapshotObjectViewCache::default();
        let mut groups = IncrementalBattlefieldGroups::default();
        groups.update(&game, &protected, &views);
        let original = groups.for_player(alice).0;
        let checkpoint = game.clone();
        game.mark_damage(first, 1);
        groups.update(&game, &protected, &views);
        assert!(Arc::ptr_eq(&original, &groups.for_player(alice).0));
        groups.update(&game, &protected, &views);
        assert_eq!(groups.objects_updated, 2);
        assert_eq!(groups.groups_rebuilt, 2);
        game.tap(first);
        groups.update(&game, &protected, &views);
        assert_eq!(
            groups.objects_updated, 3,
            "one local mutation must visit one object"
        );
        assert_eq!(groups.groups_rebuilt, 3);
        let after = groups.for_player(alice).0;
        let unchanged = after.iter().find(|group| group.id == second.0).unwrap();
        assert!(Arc::ptr_eq(
            unchanged,
            original.iter().find(|group| group.id == second.0).unwrap()
        ));
        let (full, _) = grouped_battlefield_for_player(&game, alice, &protected);
        assert_eq!(
            serde_json::to_value(&after).unwrap(),
            serde_json::to_value(full).unwrap()
        );
        game = checkpoint;
        game.tap(second);
        groups.update(&game, &protected, &views);
        let (full, _) = grouped_battlefield_for_player(&game, alice, &protected);
        assert_eq!(
            serde_json::to_value(groups.for_player(alice).0).unwrap(),
            serde_json::to_value(full).unwrap()
        );
        game.battlefield.reverse();
        groups.update(&game, &HashSet::new(), &views);
        let (full, _) = grouped_battlefield_for_player(&game, alice, &HashSet::new());
        assert_eq!(
            serde_json::to_value(groups.for_player(alice).0).unwrap(),
            serde_json::to_value(full).unwrap()
        );
    }

    #[test]
    fn battlefield_grouping_uses_current_controller_for_temporary_control_effects() {
        let _id_counter_guard = crate::test_id_counter_guard();
        use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
        use ironsmith::effect::Until;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let protected_ids = std::collections::HashSet::new();
        let bears_card = test_bears_card();
        let bear = game.create_object_from_card(&bears_card, bob, Zone::Battlefield);

        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                bear,
                alice,
                EffectTarget::Specific(bear),
                Modification::ChangeController(alice),
            )
            .until(Until::EndOfTurn)
            .with_expires_end_of_turn(game.turn.turn_number),
        );
        game.refresh_continuous_state();

        assert_eq!(
            game.current_controller(bear),
            Some(alice),
            "continuous control effect should make Alice the current controller"
        );

        let (alice_battlefield, alice_total) =
            grouped_battlefield_for_player(&game, alice, &protected_ids);
        let (bob_battlefield, bob_total) =
            grouped_battlefield_for_player(&game, bob, &protected_ids);

        assert_eq!(alice_total, 1, "Alice should see the stolen bear");
        assert!(
            alice_battlefield
                .iter()
                .any(|permanent| permanent.member_ids.contains(&bear.0)),
            "Alice's battlefield snapshot should contain the stolen bear: {alice_battlefield:?}"
        );
        assert_eq!(bob_total, 0, "Bob should no longer see the stolen bear");
        assert!(
            bob_battlefield.is_empty(),
            "Bob's battlefield snapshot should not contain the stolen bear: {bob_battlefield:?}"
        );
    }

    #[test]
    fn type_line_display_compacts_all_creature_types() {
        let mut subtypes = vec![Subtype::Shapeshifter];
        for subtype in Subtype::all_creature_types() {
            if !subtypes.contains(subtype) {
                subtypes.push(*subtype);
            }
        }

        let (display, badges) = format_type_line_display_parts(
            &[],
            &[CardType::Creature],
            &subtypes,
            &[Subtype::Shapeshifter],
        );

        assert_eq!(display, "Creature - Shapeshifter");
        assert_eq!(badges, vec!["All creature types"]);
    }

    #[test]
    fn type_line_display_keeps_normal_subtypes_inline() {
        let (display, badges) = format_type_line_display_parts(
            &[],
            &[CardType::Creature],
            &[Subtype::Angel, Subtype::Advisor],
            &[Subtype::Angel],
        );

        assert_eq!(display, "Creature - Angel Advisor");
        assert!(badges.is_empty());
    }
}
