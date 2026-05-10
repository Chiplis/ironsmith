use std::collections::{BTreeMap, BTreeSet};

use crate::{ActionEnvelope, AuditCommand, AuditFailure, CardOpening, RngReveal, Visibility};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuditState {
    player_seats: BTreeSet<u8>,
    deck_slots: BTreeMap<(u8, u16), HiddenSlotState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HiddenSlotState {
    commitment: String,
    opened: bool,
}

impl AuditState {
    pub(crate) fn new(
        player_seats: BTreeSet<u8>,
        deck_commitments: &BTreeMap<(u8, u16), String>,
    ) -> Self {
        Self {
            player_seats,
            deck_slots: deck_commitments
                .iter()
                .map(|(key, commitment)| {
                    (
                        *key,
                        HiddenSlotState {
                            commitment: commitment.clone(),
                            opened: false,
                        },
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn apply_action(&mut self, action: &ActionEnvelope) -> Result<(), AuditFailure> {
        match &action.command {
            AuditCommand::PassPriority => {
                self.expect_no_openings(action)?;
                self.expect_no_rng_reveals(action)?;
            }
            AuditCommand::DrawCards { player, count } => {
                self.expect_actor(action, *player)?;
                self.expect_opening_count(action, usize::from(*count))?;
                self.expect_no_rng_reveals(action)?;
                for opening in &action.openings {
                    self.expect_private_opening(action, opening, *player, *player)?;
                    self.mark_opened(action, opening)?;
                }
            }
            AuditCommand::PublicReveal { owner, slot, .. } => {
                self.expect_opening_count(action, 1)?;
                self.expect_no_rng_reveals(action)?;
                let opening = &action.openings[0];
                if opening.owner != *owner || opening.slot != *slot {
                    return Err(AuditFailure {
                        seq: Some(action.seq),
                        player: Some(action.actor),
                        reason: "public reveal opening does not match command target".to_string(),
                    });
                }
                if opening.visibility != Visibility::Public {
                    return Err(AuditFailure {
                        seq: Some(action.seq),
                        player: Some(action.actor),
                        reason: "public reveal must use public visibility".to_string(),
                    });
                }
                self.mark_opened(action, opening)?;
            }
            AuditCommand::SearchLibrary {
                searcher,
                library_owner,
                selected_slot,
                ..
            } => {
                self.expect_actor(action, *searcher)?;
                self.expect_all_player_rng_reveals(action)?;
                match selected_slot {
                    Some(slot) => {
                        self.expect_opening_count(action, 1)?;
                        let opening = &action.openings[0];
                        if opening.owner != *library_owner || opening.slot != *slot {
                            return Err(AuditFailure {
                                seq: Some(action.seq),
                                player: Some(action.actor),
                                reason: "search opening does not match selected slot".to_string(),
                            });
                        }
                        if opening.visibility != (Visibility::Viewer { viewer: *searcher })
                            && opening.visibility != Visibility::Public
                        {
                            return Err(AuditFailure {
                                seq: Some(action.seq),
                                player: Some(action.actor),
                                reason: "search opening must be visible to searcher or public"
                                    .to_string(),
                            });
                        }
                        self.mark_opened(action, opening)?;
                    }
                    None => self.expect_no_openings(action)?,
                }
            }
            AuditCommand::ShuffleLibrary { player } => {
                self.expect_actor(action, *player)?;
                self.expect_no_openings(action)?;
                self.expect_all_player_rng_reveals(action)?;
            }
        }
        Ok(())
    }

    fn expect_actor(&self, action: &ActionEnvelope, expected: u8) -> Result<(), AuditFailure> {
        if action.actor != expected {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(action.actor),
                reason: format!("command actor must be player {expected}"),
            });
        }
        Ok(())
    }

    fn expect_opening_count(
        &self,
        action: &ActionEnvelope,
        expected: usize,
    ) -> Result<(), AuditFailure> {
        if action.openings.len() != expected {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(action.actor),
                reason: format!(
                    "expected {expected} card opening(s), found {}",
                    action.openings.len()
                ),
            });
        }
        Ok(())
    }

    fn expect_no_openings(&self, action: &ActionEnvelope) -> Result<(), AuditFailure> {
        self.expect_opening_count(action, 0)
    }

    fn expect_no_rng_reveals(&self, action: &ActionEnvelope) -> Result<(), AuditFailure> {
        if !action.rng_reveals.is_empty() {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(action.actor),
                reason: "command must not include rng reveals".to_string(),
            });
        }
        Ok(())
    }

    fn expect_all_player_rng_reveals(&self, action: &ActionEnvelope) -> Result<(), AuditFailure> {
        let reveal_players = action
            .rng_reveals
            .iter()
            .map(|reveal| reveal.player)
            .collect::<BTreeSet<_>>();
        if reveal_players != self.player_seats {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(action.actor),
                reason: "random operation must include one rng reveal from every player"
                    .to_string(),
            });
        }
        let mut seen_events = BTreeSet::new();
        for reveal in &action.rng_reveals {
            self.validate_rng_reveal_shape(action, reveal, &mut seen_events)?;
        }
        Ok(())
    }

    fn validate_rng_reveal_shape(
        &self,
        action: &ActionEnvelope,
        reveal: &RngReveal,
        seen_events: &mut BTreeSet<(String, u8)>,
    ) -> Result<(), AuditFailure> {
        if !seen_events.insert((reveal.event_id.clone(), reveal.player)) {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(reveal.player),
                reason: format!("duplicate rng reveal for event {}", reveal.event_id),
            });
        }
        if !reveal.event_id.contains(&format!("seq{}", action.seq)) {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(reveal.player),
                reason: "rng event id must bind the action sequence".to_string(),
            });
        }
        Ok(())
    }

    fn expect_private_opening(
        &self,
        action: &ActionEnvelope,
        opening: &CardOpening,
        owner: u8,
        viewer: u8,
    ) -> Result<(), AuditFailure> {
        if opening.owner != owner {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(action.actor),
                reason: format!("opening owner must be player {owner}"),
            });
        }
        if opening.visibility != Visibility::OwnerOnly
            && opening.visibility != (Visibility::Viewer { viewer })
        {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(action.actor),
                reason: "private opening visibility does not match command".to_string(),
            });
        }
        Ok(())
    }

    fn mark_opened(
        &mut self,
        action: &ActionEnvelope,
        opening: &CardOpening,
    ) -> Result<(), AuditFailure> {
        let Some(slot) = self.deck_slots.get_mut(&(opening.owner, opening.slot)) else {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(opening.owner),
                reason: format!("opened unknown deck slot {}", opening.slot),
            });
        };
        if slot.commitment != opening.commitment {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(opening.owner),
                reason: format!("opening commitment mismatch for slot {}", opening.slot),
            });
        }
        if slot.opened {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(opening.owner),
                reason: format!("deck slot {} was opened more than once", opening.slot),
            });
        }
        slot.opened = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionEnvelope, AuditCommand};

    fn state() -> AuditState {
        let mut commitments = BTreeMap::new();
        commitments.insert((1, 0), "c10".to_string());
        AuditState::new(BTreeSet::from([0, 1, 2, 3]), &commitments)
    }

    fn draw(openings: Vec<CardOpening>) -> ActionEnvelope {
        ActionEnvelope {
            seq: 1,
            actor: 1,
            prev_state_hash: "prev".to_string(),
            command: AuditCommand::DrawCards {
                player: 1,
                count: 1,
            },
            openings,
            rng_reveals: Vec::new(),
            next_state_hash: "next".to_string(),
            signature: String::new(),
        }
    }

    fn opening() -> CardOpening {
        CardOpening {
            owner: 1,
            slot: 0,
            card: "Island".to_string(),
            salt: "salt".to_string(),
            commitment: "c10".to_string(),
            visibility: Visibility::OwnerOnly,
        }
    }

    #[test]
    fn draw_marks_hidden_slot_opened() {
        let mut state = state();
        state.apply_action(&draw(vec![opening()])).unwrap();
        let err = state.apply_action(&draw(vec![opening()])).unwrap_err();
        assert!(err.reason.contains("opened more than once"));
    }

    #[test]
    fn draw_rejects_public_opening() {
        let mut state = state();
        let mut opening = opening();
        opening.visibility = Visibility::Public;
        let err = state.apply_action(&draw(vec![opening])).unwrap_err();
        assert!(err.reason.contains("private opening visibility"));
    }
}
