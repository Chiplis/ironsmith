//! Hidden cards a player has legitimately seen stay known to that player
//! after the view that showed them ends (a Thoughtseize look at a hand, a
//! scry, a face-down card's identity), for as long as the card is still the
//! same object in the same hidden place.
//!
//! The memory is presentation only and per local engine: it never feeds the
//! rules, a public checkpoint or a peer. Each entry re-validates against the
//! live game on every snapshot, so a card that changed zones (a new object,
//! CR 400.7), turned face up, or sat in a library that was shuffled since is
//! forgotten rather than tracked.
use super::*;
use ui_snapshot::{library_position, GameSnapshot, ViewedCardSnapshot};

#[derive(Clone, Debug)]
struct KnownHiddenCard {
    id: ObjectId,
    stable_id: StableId,
    zone: Zone,
    /// Hidden-info audit log length when the card was seen. A library card is
    /// only known while no shuffle of its owner's library happened since.
    audit_checkpoint: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct HiddenCardMemory {
    known: HashMap<PlayerId, Vec<KnownHiddenCard>>,
}

fn tracked_zone(game: &GameState, object: &ironsmith::object::Object) -> bool {
    match object.zone {
        Zone::Hand | Zone::Library => true,
        Zone::Exile | Zone::Battlefield => game.is_face_down(object.id),
        _ => false,
    }
}

fn still_known(game: &GameState, card: &KnownHiddenCard) -> bool {
    let Some(object) = game.object(card.id) else {
        return false;
    };
    if object.stable_id != card.stable_id
        || object.zone != card.zone
        || game.is_hidden_card_placeholder(card.id)
        || !tracked_zone(game, object)
    {
        return false;
    }
    if card.zone != Zone::Library {
        return true;
    }
    // A rollback can shorten the log; nothing after that point is trusted.
    card.audit_checkpoint <= game.crypto_audit_checkpoint()
        && !game
            .crypto_audit_operations_since(card.audit_checkpoint)
            .iter()
            .any(|operation| matches!(operation,
                HiddenInfoOperation::LibraryShuffle { player, .. } if *player == object.owner))
}

impl HiddenCardMemory {
    fn remember(&mut self, game: &GameState, viewer: PlayerId, id: ObjectId) {
        let Some(object) = game.object(id) else {
            return;
        };
        // A player's own hand is always open to them.
        if !tracked_zone(game, object)
            || game.is_hidden_card_placeholder(id)
            || (object.zone == Zone::Hand && game.controlling_player_for(object.owner) == viewer)
        {
            return;
        }
        let entry = KnownHiddenCard {
            id,
            stable_id: object.stable_id,
            zone: object.zone,
            audit_checkpoint: game.crypto_audit_checkpoint(),
        };
        let known = self.known.entry(viewer).or_default();
        // Seeing a card again (after a shuffle) restarts its library window.
        known.retain(|card| card.id != id);
        known.push(entry);
    }

    fn remember_view(&mut self, game: &GameState, view: &ActiveViewedCards) {
        let viewers: Vec<PlayerId> = if view.public {
            game.players.iter().map(|player| player.id).collect()
        } else {
            vec![view.viewer]
        };
        for (index, card) in view.cards.iter().enumerate() {
            let id = view.resolved_object_id(game, index, *card);
            for viewer in &viewers {
                self.remember(game, *viewer, id);
            }
        }
    }

    /// Records everything the views and this snapshot show, then adds the
    /// perspective's still-valid memories to each owner's look cards, with
    /// zone and library position so the UI can place them.
    pub(crate) fn apply(
        &mut self,
        game: &GameState,
        views: &[&ActiveViewedCards],
        snapshot: &mut GameSnapshot,
    ) {
        let perspective = PlayerId::from_index(snapshot.perspective);
        for view in views {
            self.remember_view(game, view);
        }
        for player in &snapshot.players {
            for card in player.hand_cards.iter() {
                self.remember(game, perspective, ObjectId::from_raw(card.id));
            }
            for card in player.persistent_look_cards.iter() {
                self.remember(game, perspective, ObjectId::from_raw(card.id));
            }
        }
        if let Some(view) = &snapshot.viewed_cards {
            for card in &view.cards {
                self.remember(game, perspective, ObjectId::from_raw(card.id));
            }
        }
        for known in self.known.values_mut() {
            known.retain(|card| still_known(game, card));
        }
        self.known.retain(|_, known| !known.is_empty());

        // The controller may look at its own face-down permanents (CR 708.5).
        let own_face_down = game.object_store.battlefield.iter().copied().filter(|id| {
            game.is_face_down(*id)
                && !game.is_hidden_card_placeholder(*id)
                && game
                    .current_controller(*id)
                    .is_some_and(|controller| game.controlling_player_for(controller) == perspective)
        });
        let known_ids: Vec<ObjectId> = self
            .known
            .get(&perspective)
            .into_iter()
            .flatten()
            .map(|card| card.id)
            .chain(own_face_down)
            .collect();

        for player in &mut snapshot.players {
            let owner = PlayerId::from_index(player.id);
            let needs_position = player
                .persistent_look_cards
                .iter()
                .any(|card| card.zone.as_deref() == Some("library") && card.position.is_none());
            let additions: Vec<&ironsmith::object::Object> = known_ids
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| object.owner == owner)
                .filter(|object| !player.persistent_look_cards.iter().any(|card| card.id == object.id.0))
                .collect();
            if additions.is_empty() && !needs_position {
                continue;
            }
            let mut cards: Vec<Arc<ViewedCardSnapshot>> = player
                .persistent_look_cards
                .iter()
                .map(|card| {
                    let position = game
                        .object(ObjectId::from_raw(card.id))
                        .and_then(|object| library_position(game, object));
                    if card.position == position {
                        card.clone()
                    } else {
                        Arc::new(ViewedCardSnapshot { position, ..card.as_ref().clone() })
                    }
                })
                .collect();
            for object in additions {
                // A face-down permanent's own characteristics hide its face.
                let mut identity = object.clone();
                identity.end_face_down_cast_overlay();
                let mut card = ui_snapshot::viewed_card_snapshot(&identity);
                card.zone = Some(decision_zone_name(object.zone));
                card.position = library_position(game, object);
                cards.push(Arc::new(card));
            }
            player.persistent_look_cards = Arc::new(cards);
        }
    }
}

impl WasmGame {
    pub(super) fn apply_hidden_card_memory(&mut self, snapshot: &mut GameSnapshot) {
        let game = self.pending_decision_game.as_deref().unwrap_or(&self.game);
        let views: Vec<&ActiveViewedCards> = self
            .active_viewed_cards
            .iter()
            .chain(self.active_audit_viewed_cards.iter())
            .collect();
        self.hidden_card_memory.apply(game, &views, snapshot);
    }
}
