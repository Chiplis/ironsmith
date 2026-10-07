//! Verified library shuffles replace the engine's identities with anonymous
//! ciphertext positions. Cryptography proves the input/output relation; the
//! engine must never receive or retain the secret permutation.

use super::{GameState, HiddenCardInfo, ObjectId, PlayerId, Zone};
use crate::cards::CardDefinition;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone)]
pub(super) struct VerifiedHiddenLibraryEpoch {
    player: PlayerId,
    deck_hash: String,
    count: usize,
    expected_inputs: Option<Vec<String>>,
    public_openings: BTreeMap<u16, VerifiedEpochPublicOpening>,
}

#[derive(Debug, Clone)]
struct VerifiedEpochPublicOpening {
    definition: CardDefinition,
    original_slot: Option<u16>,
    commitment: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct VerifiedHiddenReplayOpening {
    definition: CardDefinition,
    info: HiddenCardInfo,
    viewer: PlayerId,
    zone: Zone,
}

impl GameState {
    /// Schedule an already verified ciphertext deck at an exact shuffle
    /// boundary. The schedule survives speculative replay: it is indexed by
    /// the public irreversible-random counter, rather than consumed by FIFO.
    /// `expected_inputs` binds the proof to the actual participating cards,
    /// including cards returned before the shuffle and excluding tutor picks.
    pub fn queue_verified_hidden_library_epoch(
        &self,
        player: PlayerId,
        deck_hash: String,
        count: usize,
        random_count_before: u64,
        mut expected_inputs: Option<Vec<String>>,
    ) -> Result<(), String> {
        if self.player(player).is_none() {
            return Err("verified hidden shuffle owner is not present".into());
        }
        if deck_hash.is_empty() || deck_hash.contains(':') || count > u16::MAX as usize + 1 {
            return Err("invalid verified hidden shuffle epoch".into());
        }
        if let Some(inputs) = expected_inputs.as_mut() {
            inputs.sort();
            if inputs.len() != count || inputs.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(
                    "verified hidden shuffle inputs must be unique and match its count".into(),
                );
            }
        }
        let epoch = VerifiedHiddenLibraryEpoch {
            player,
            deck_hash,
            count,
            expected_inputs,
            public_openings: BTreeMap::new(),
        };
        let mut schedule = self
            .runtime_cache
            .verified_hidden_library_epochs
            .borrow_mut();
        if let Some(existing) = schedule.get(&random_count_before) {
            if existing.player != epoch.player
                || existing.deck_hash != epoch.deck_hash
                || existing.count != epoch.count
                || existing.expected_inputs != epoch.expected_inputs
            {
                return Err(
                    "conflicting verified hidden shuffle at the same random counter".into(),
                );
            }
        } else {
            schedule.insert(random_count_before, epoch);
        }
        Ok(())
    }

    /// Whether an authenticated epoch position will be created by a future
    /// shuffle in this action. Used to prepare a public opening before a move
    /// whose source object does not exist until that shuffle runs.
    pub fn pending_verified_hidden_library_position(
        &self,
        player: PlayerId,
        commitment: &str,
    ) -> Option<(String, u16)> {
        self.runtime_cache
            .verified_hidden_library_epochs
            .borrow()
            .iter()
            .filter(|(counter, epoch)| {
                **counter >= self.irreversible_random_count() && epoch.player == player
            })
            .find_map(|(_, epoch)| {
                let prefix = format!("ziffle:{}:", epoch.deck_hash);
                let position = commitment.strip_prefix(&prefix)?.parse::<u16>().ok()?;
                (usize::from(position) < epoch.count && commitment == format!("{prefix}{position}"))
                    .then(|| (epoch.deck_hash.clone(), position))
            })
    }

    /// Store a cryptographically authenticated PUBLIC opening for a future
    /// epoch. Never use this for private hand openings: only positions that
    /// are already public may influence both peers' execution at the boundary.
    pub fn queue_verified_hidden_library_public_opening(
        &self,
        player: PlayerId,
        deck_hash: &str,
        position: u16,
        definition: &CardDefinition,
        original_slot: Option<u16>,
        commitment: Option<&str>,
    ) -> Result<(), String> {
        if original_slot.is_some() != commitment.is_some() {
            return Err(
                "verified public opening must supply both manifest slot and commitment".into(),
            );
        }
        let mut schedule = self
            .runtime_cache
            .verified_hidden_library_epochs
            .borrow_mut();
        let epoch = schedule
            .iter_mut()
            .find_map(|(counter, epoch)| {
                (*counter >= self.irreversible_random_count()
                    && epoch.player == player
                    && epoch.deck_hash == deck_hash)
                    .then_some(epoch)
            })
            .ok_or("public opening does not reference a pending verified library epoch")?;
        if usize::from(position) >= epoch.count {
            return Err("public opening position is outside the verified library epoch".into());
        }
        if let Some(existing) = epoch.public_openings.get(&position) {
            if existing.definition.card.id != definition.card.id
                || existing.original_slot != original_slot
                || existing.commitment.as_deref() != commitment
            {
                return Err(
                    "conflicting public opening for a verified library epoch position".into(),
                );
            }
        } else {
            epoch.public_openings.insert(
                position,
                VerifiedEpochPublicOpening {
                    definition: definition.clone(),
                    original_slot,
                    commitment: commitment.map(str::to_string),
                },
            );
        }
        Ok(())
    }

    /// Preserve an already authenticated local opening across a continuation
    /// whose checkpoint predates creation of this anonymous position. Private
    /// identities are restored at the same zone/view boundary, never by an
    /// old-object correspondence and never while the fresh deck is shuffled.
    pub fn queue_verified_hidden_library_replay_opening(
        &self,
        info: &HiddenCardInfo,
        definition: &CardDefinition,
        viewer: PlayerId,
    ) -> Result<bool, String> {
        let commitment = info
            .public_commitment
            .as_deref()
            .unwrap_or(&info.commitment);
        if self
            .pending_verified_hidden_library_position(info.owner, commitment)
            .is_none()
        {
            return Ok(false);
        }
        let key = (info.owner, commitment.to_string());
        let mut openings = self
            .runtime_cache
            .verified_hidden_replay_openings
            .borrow_mut();
        if let Some(existing) = openings.get(&key) {
            if existing.definition.card.id != definition.card.id {
                return Err("conflicting replay identity for a verified library position".into());
            }
        }
        openings.insert(
            key,
            VerifiedHiddenReplayOpening {
                definition: definition.clone(),
                info: info.clone(),
                viewer,
                zone: info.zone,
            },
        );
        Ok(true)
    }

    pub(crate) fn hydrate_verified_library_replay_zone(&mut self, id: ObjectId, zone: Zone) {
        if zone == Zone::Library {
            return;
        }
        let opening = self.verified_hidden_replay_opening_for_object(id);
        if let Some(opening) = opening.filter(|opening| opening.zone == zone) {
            self.apply_verified_hidden_replay_opening(id, opening);
        }
    }

    pub(crate) fn hydrate_verified_library_replay_view(
        &mut self,
        ids: &[ObjectId],
        viewers: &[PlayerId],
        public: bool,
    ) {
        for id in ids {
            let opening = self.verified_hidden_replay_opening_for_object(*id);
            if let Some(opening) = opening.filter(|opening| {
                opening.zone == Zone::Library && (public || viewers.contains(&opening.viewer))
            }) {
                self.apply_verified_hidden_replay_opening(*id, opening);
            }
        }
    }

    fn verified_hidden_replay_opening_for_object(
        &self,
        id: ObjectId,
    ) -> Option<VerifiedHiddenReplayOpening> {
        let info = self.hidden_card_info(id)?;
        let commitment = info
            .public_commitment
            .as_deref()
            .unwrap_or(&info.commitment);
        self.runtime_cache
            .verified_hidden_replay_openings
            .borrow()
            .get(&(info.owner, commitment.to_string()))
            .cloned()
    }

    fn apply_verified_hidden_replay_opening(
        &mut self,
        id: ObjectId,
        opening: VerifiedHiddenReplayOpening,
    ) {
        let Some(zone) = self.object(id).map(|object| object.zone) else {
            return;
        };
        if self
            .reveal_hidden_card_with_definition(id, &opening.definition)
            .is_some()
        {
            let mut info = opening.info;
            info.zone = zone;
            self.set_hidden_card_info(id, info);
        } else {
            *self
                .runtime_cache
                .verified_hidden_library_epoch_error
                .borrow_mut() =
                Some("verified replay opening conflicts with the current card identity".into());
        }
    }

    /// Protocol adapters must reject the action if a verified shuffle could
    /// not be applied. An invalid epoch never falls back to a local shuffle.
    pub fn verified_hidden_library_epoch_error(&self) -> Option<String> {
        self.runtime_cache
            .verified_hidden_library_epoch_error
            .borrow()
            .clone()
    }

    /// Returns true whenever this boundary has a verified epoch, including
    /// validation failure, to forbid falling back to a linked permutation.
    pub(super) fn apply_verified_hidden_library_epoch(
        &mut self,
        player: PlayerId,
        random_count_before: u64,
    ) -> bool {
        let epoch = self
            .runtime_cache
            .verified_hidden_library_epochs
            .borrow()
            .get(&random_count_before)
            .cloned();
        let Some(epoch) = epoch else {
            return false;
        };
        let result = self.install_verified_hidden_library_epoch(player, &epoch);
        if let Err(error) = result {
            *self
                .runtime_cache
                .verified_hidden_library_epoch_error
                .borrow_mut() = Some(error);
        }
        true
    }

    fn install_verified_hidden_library_epoch(
        &mut self,
        player: PlayerId,
        epoch: &VerifiedHiddenLibraryEpoch,
    ) -> Result<(), String> {
        let old = self
            .player(player)
            .ok_or("verified hidden shuffle owner is not present")?
            .library
            .to_vec();
        if epoch.player != player || epoch.count != old.len() {
            return Err("verified hidden shuffle does not match the participating library".into());
        }
        let mut inputs = Vec::with_capacity(old.len());
        for id in &old {
            let object = self
                .object(*id)
                .ok_or("verified hidden shuffle object is missing")?;
            let info = self
                .hidden_card_info(*id)
                .ok_or("verified hidden shuffle input is not authenticated")?;
            if object.owner != player || object.zone != Zone::Library || info.owner != player {
                return Err("verified hidden shuffle input ownership or zone mismatch".into());
            }
            inputs.push(
                info.public_commitment
                    .clone()
                    .unwrap_or_else(|| info.commitment.clone()),
            );
        }
        if let Some(expected) = &epoch.expected_inputs {
            inputs.sort();
            if &inputs != expected {
                return Err("verified hidden shuffle ciphertext inputs do not match the participating library".into());
            }
        }

        // Even an identical ciphertext deck hash at a later accepted shuffle
        // must not revive a stale origin witness. This public high-water mark
        // needs no secret permutation and is part of native rollback state.
        self.reserve_hidden_incarnation_epoch()?;

        // Detach any public deferred claim before removing its object. The
        // old ciphertext anchor is disclosed at match end, independently of
        // all new positions. It never provides a link to this epoch.
        for id in &old {
            let info = self
                .hidden_card_info(*id)
                .cloned()
                .expect("validated hidden input");
            self.anchor_hidden_card_entering_library(*id, &info);
        }
        let retired: HashSet<_> = old.iter().copied().collect();
        // Tutor helpers temporarily take their chosen cards out of the index
        // while leaving the objects in Zone::Library. Keep those objects
        // indexed during object creation (whose invariants inspect all zones),
        // then restore the helper's transient state before it reinserts them.
        let mut excluded = self
            .objects
            .iter()
            .filter_map(|(id, object)| {
                (object.owner == player && object.zone == Zone::Library && !retired.contains(id))
                    .then_some(*id)
            })
            .collect::<Vec<_>>();
        excluded.sort_unstable();
        if !excluded.is_empty() {
            self.player_mut(player)
                .expect("validated owner")
                .library
                .extend(excluded.iter().copied());
        }
        for id in old {
            self.remove_object(id);
            let tracking = self.auxiliary_tracking_mut();
            tracking.hidden_cards.remove(&id);
            tracking.hidden_face_down_cast_claims.remove(&id);
            tracking.blind_face_down_declarations.remove(&id);
            tracking.publicly_revealed_hidden_cards.remove(&id);
        }
        // Prior zone changes may still name a retired object. They cannot
        // resolve to a new position; discard those dead aliases altogether.
        self.zone_change_result_objects
            .retain(|source, destinations| {
                !retired.contains(source) && !destinations.iter().any(|id| retired.contains(id))
            });
        for position in 0..epoch.count {
            let slot = position as u16;
            let commitment = format!("ziffle:{}:{position}", epoch.deck_hash);
            let id = self.create_hidden_card_placeholder(
                player,
                Zone::Library,
                slot,
                commitment.clone(),
            );
            self.set_hidden_card_info(
                id,
                HiddenCardInfo {
                incarnation: Some(0),
                    owner: player,
                    zone: Zone::Library,
                    slot,
                    commitment: commitment.clone(),
                    origin_slot: Some(slot),
                    origin_commitment: Some(commitment.clone()),
                    public_slot: Some(slot),
                    public_commitment: Some(commitment),
                },
            );
            if let Some(opening) = epoch.public_openings.get(&slot) {
                if let (Some(original_slot), Some(commitment)) =
                    (opening.original_slot, opening.commitment.as_ref())
                {
                    let mut info = self
                        .hidden_card_info(id)
                        .expect("new epoch hidden metadata")
                        .clone();
                    info.slot = original_slot;
                    info.commitment = commitment.clone();
                    self.set_hidden_card_info(id, info);
                }
                self.reveal_hidden_card_with_definition(id, &opening.definition)
                    .expect("new epoch placeholder must accept its verified identity");
            }
        }
        if !excluded.is_empty() {
            self.player_mut(player)
                .expect("validated owner")
                .library
                .retain(|id| !excluded.contains(id));
        }
        Ok(())
    }
}
