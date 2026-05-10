use serde::{Deserialize, Serialize};

use crate::protocol::{
    CryptoRequirement, HiddenZoneKind, HiddenZoneProtocol, requirements_for_command,
};
use crate::{AuditCommand, Visibility};

/// Runtime-facing audit events emitted by generic engine surfaces.
///
/// These events are intentionally card-agnostic. The runtime should emit them
/// from reusable operations such as draw, search, reveal, shuffle, zone move,
/// private view, and fair-random selection. The multiplayer transcript layer can
/// then turn them into signed actions, openings, and proof requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuntimeAuditEvent {
    ActionAccepted {
        seq: u64,
        actor: u8,
        command: AuditCommand,
    },
    HiddenCardOpened {
        owner: u8,
        zone: HiddenZoneKind,
        slot: u16,
        visibility: Visibility,
    },
    HiddenCardsViewed {
        viewer: u8,
        owner: u8,
        zone: HiddenZoneKind,
        count: u16,
        reason: String,
    },
    HiddenCardMoved {
        owner: u8,
        from: HiddenZoneKind,
        to: HiddenZoneKind,
        slot: u16,
    },
    FairRandomRequested {
        owner: u8,
        zone: HiddenZoneKind,
        count: u16,
        reason: String,
    },
    VerifiableShuffleRequired {
        owner: u8,
        zone: HiddenZoneKind,
        reason: String,
    },
}

impl RuntimeAuditEvent {
    pub fn crypto_requirements(&self) -> Vec<CryptoRequirement> {
        match self {
            RuntimeAuditEvent::ActionAccepted { command, .. } => requirements_for_command(command),
            RuntimeAuditEvent::HiddenCardOpened {
                owner,
                zone,
                slot,
                visibility,
            } => match visibility {
                Visibility::OwnerOnly => vec![CryptoRequirement::PrivateOpen {
                    viewer: *owner,
                    owner: *owner,
                    zone: *zone,
                    slot: *slot,
                }],
                Visibility::Viewer { viewer } => vec![CryptoRequirement::PrivateOpen {
                    viewer: *viewer,
                    owner: *owner,
                    zone: *zone,
                    slot: *slot,
                }],
                Visibility::Public => vec![CryptoRequirement::PublicOpen {
                    owner: *owner,
                    zone: *zone,
                    slot: *slot,
                }],
            },
            RuntimeAuditEvent::HiddenCardsViewed {
                viewer,
                owner,
                zone,
                count,
                reason,
            } => vec![CryptoRequirement::PrivateViewWindow {
                viewer: *viewer,
                owner: *owner,
                zone: *zone,
                count: *count,
                reason: reason.clone(),
            }],
            RuntimeAuditEvent::HiddenCardMoved {
                owner,
                from,
                to,
                slot,
            } => vec![CryptoRequirement::HiddenMove {
                owner: *owner,
                from: *from,
                to: *to,
                slot: *slot,
            }],
            RuntimeAuditEvent::FairRandomRequested {
                owner, zone, count, ..
            } => vec![CryptoRequirement::FairRandomSlot {
                owner: *owner,
                zone: *zone,
                count: *count,
            }],
            RuntimeAuditEvent::VerifiableShuffleRequired { owner, zone, .. } => {
                vec![CryptoRequirement::VerifiableShuffle {
                    owner: *owner,
                    zone: *zone,
                    protocol: HiddenZoneProtocol::MentalPokerBayerGrothV1,
                }]
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeAuditRecorder {
    events: Vec<RuntimeAuditEvent>,
}

impl RuntimeAuditRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, event: RuntimeAuditEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[RuntimeAuditEvent] {
        &self.events
    }

    pub fn drain(&mut self) -> Vec<RuntimeAuditEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn crypto_requirements(&self) -> Vec<CryptoRequirement> {
        self.events
            .iter()
            .flat_map(RuntimeAuditEvent::crypto_requirements)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_draw_action_maps_to_private_open_requirement() {
        let event = RuntimeAuditEvent::ActionAccepted {
            seq: 7,
            actor: 2,
            command: AuditCommand::DrawCards {
                player: 2,
                count: 1,
            },
        };
        assert_eq!(
            event.crypto_requirements(),
            vec![CryptoRequirement::PrivateOpen {
                viewer: 2,
                owner: 2,
                zone: HiddenZoneKind::Library,
                slot: 0,
            }]
        );
    }

    #[test]
    fn runtime_search_view_and_shuffle_requirements_are_card_agnostic() {
        let mut recorder = RuntimeAuditRecorder::new();
        recorder.record(RuntimeAuditEvent::HiddenCardsViewed {
            viewer: 1,
            owner: 3,
            zone: HiddenZoneKind::Library,
            count: u16::MAX,
            reason: "search_library".to_string(),
        });
        recorder.record(RuntimeAuditEvent::VerifiableShuffleRequired {
            owner: 3,
            zone: HiddenZoneKind::Library,
            reason: "search_library_complete".to_string(),
        });
        let requirements = recorder.crypto_requirements();
        assert!(matches!(
            requirements.as_slice(),
            [
                CryptoRequirement::PrivateViewWindow {
                    viewer: 1,
                    owner: 3,
                    zone: HiddenZoneKind::Library,
                    ..
                },
                CryptoRequirement::VerifiableShuffle {
                    owner: 3,
                    zone: HiddenZoneKind::Library,
                    protocol: HiddenZoneProtocol::MentalPokerBayerGrothV1,
                }
            ]
        ));
    }
}
