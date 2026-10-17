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
use std::sync::Arc;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_with_memory(wasm: &mut WasmGame, viewer: PlayerId) -> GameSnapshot {
        let mut snapshot = GameSnapshot::from_game(
            &wasm.game, viewer, None, None, None, None, None, Vec::new(),
            wasm.active_viewed_cards.as_ref(), false, None, 0,
        );
        let views: Vec<&ActiveViewedCards> = wasm.active_viewed_cards.iter().collect();
        wasm.hidden_card_memory.apply(&wasm.game, &views, &mut snapshot);
        snapshot
    }

    fn private_view(wasm: &WasmGame, viewer: PlayerId, subject: PlayerId, zone: Zone, cards: Vec<ObjectId>) -> ActiveViewedCards {
        ActiveViewedCards {
            acknowledged_by: Vec::new(),
            viewer,
            subject,
            zone,
            card_stable_ids: stable_ids_for_viewed_cards(&wasm.game, &cards),
            cards,
            public: false,
            source: None,
            description: "Look".to_string(),
        }
    }

    fn known(snapshot: &GameSnapshot, owner: PlayerId) -> Vec<(String, Option<String>, Option<usize>)> {
        snapshot.players[owner.index()].persistent_look_cards.iter()
            .map(|card| (card.name.clone(), card.zone.clone(), card.position)).collect()
    }

    fn card(name: &str, id: u32) -> ironsmith::card::Card {
        ironsmith::card::CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Instant])
            .build()
    }

    #[test]
    fn a_hand_card_seen_once_stays_known_until_it_leaves_the_hand() {
        let mut wasm = WasmGame::new();
        let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
        let secret = wasm.game.create_object_from_card(&card("Bob's Secret", 91_001), bob, Zone::Hand);
        wasm.game.create_object_from_card(&card("Bob's Other Card", 91_002), bob, Zone::Hand);
        wasm.active_viewed_cards = Some(private_view(&wasm, alice, bob, Zone::Hand, vec![secret]));
        snapshot_with_memory(&mut wasm, alice);

        wasm.active_viewed_cards = None;
        let after = snapshot_with_memory(&mut wasm, alice);
        assert_eq!(known(&after, bob), vec![("Bob's Secret".to_string(), Some("hand".to_string()), None)]);
        // Bob never looked at his own hand through Alice's eyes.
        assert!(known(&snapshot_with_memory(&mut wasm, bob), bob).is_empty());

        wasm.game.remove_object(secret);
        assert!(known(&snapshot_with_memory(&mut wasm, alice), bob).is_empty());
    }

    #[test]
    fn library_cards_keep_their_order_until_the_library_is_shuffled() {
        let mut wasm = WasmGame::new();
        let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
        for (index, name) in ["Bottom", "Third", "Second", "Top"].into_iter().enumerate() {
            wasm.game.create_object_from_card(&card(name, 92_000 + index as u32), bob, Zone::Library);
        }
        let library = wasm.game.player(bob).unwrap().library.to_vec();
        let top_two = vec![library[library.len() - 1], library[library.len() - 2]];
        wasm.active_viewed_cards = Some(private_view(&wasm, alice, bob, Zone::Library, top_two));
        snapshot_with_memory(&mut wasm, alice);

        wasm.active_viewed_cards = None;
        let mut remembered = known(&snapshot_with_memory(&mut wasm, alice), bob);
        remembered.sort_by_key(|(_, _, position)| *position);
        assert_eq!(remembered, vec![
            ("Top".to_string(), Some("library".to_string()), Some(0)),
            ("Second".to_string(), Some("library".to_string()), Some(1)),
        ]);

        wasm.game.shuffle_player_library(bob);
        assert!(known(&snapshot_with_memory(&mut wasm, alice), bob).is_empty());
    }
}
