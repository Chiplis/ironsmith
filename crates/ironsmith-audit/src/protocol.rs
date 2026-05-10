use serde::{Deserialize, Serialize};

/// Chosen architecture for cheat-resistant hidden zones.
///
/// We use mental-poker encrypted deck custody for library order, not whole-engine
/// ZK or whole-engine MPC. The game engine emits generic visibility and zone
/// requirements; this layer decides which cryptographic primitive must satisfy
/// each requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HiddenZoneProtocol {
    /// Mental-poker encrypted deck with a publicly verifiable shuffle proof.
    ///
    /// Target backend: Bayer-Groth 2012 proof over an ElGamal-style encrypted
    /// deck, currently represented as `ziffle-0.1` in transcripts.
    MentalPokerBayerGrothV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HiddenZoneKind {
    Library,
    Hand,
    FaceDownExile,
    FaceDownPermanent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CryptoRequirement {
    /// A hidden object identity is opened only to one player.
    PrivateOpen {
        viewer: u8,
        owner: u8,
        zone: HiddenZoneKind,
        slot: u16,
    },
    /// A hidden object identity is opened to every player and the audit log.
    PublicOpen {
        owner: u8,
        zone: HiddenZoneKind,
        slot: u16,
    },
    /// A player is allowed to inspect a hidden set for a bounded operation such
    /// as "look at top N", scry, surveil, or search library.
    PrivateViewWindow {
        viewer: u8,
        owner: u8,
        zone: HiddenZoneKind,
        count: u16,
        reason: String,
    },
    /// A hidden zone order must be randomized by all players before future
    /// order-dependent operations continue.
    VerifiableShuffle {
        owner: u8,
        zone: HiddenZoneKind,
        protocol: HiddenZoneProtocol,
    },
    /// A hidden object moved without becoming known. Public state carries only
    /// the encrypted payload id / commitment id.
    HiddenMove {
        owner: u8,
        from: HiddenZoneKind,
        to: HiddenZoneKind,
        slot: u16,
    },
    /// A random selection over hidden slots must use transcripted fair RNG.
    FairRandomSlot {
        owner: u8,
        zone: HiddenZoneKind,
        count: u16,
    },
}

pub fn requirements_for_command(command: &crate::AuditCommand) -> Vec<CryptoRequirement> {
    use crate::AuditCommand;

    match command {
        AuditCommand::PassPriority => Vec::new(),
        AuditCommand::DrawCards { player, count } => (0..*count)
            .map(|slot| CryptoRequirement::PrivateOpen {
                viewer: *player,
                owner: *player,
                zone: HiddenZoneKind::Library,
                slot: u16::from(slot),
            })
            .collect(),
        AuditCommand::PublicReveal { owner, slot, .. } => vec![CryptoRequirement::PublicOpen {
            owner: *owner,
            zone: HiddenZoneKind::Library,
            slot: *slot,
        }],
        AuditCommand::SearchLibrary {
            searcher,
            library_owner,
            ..
        } => vec![
            CryptoRequirement::PrivateViewWindow {
                viewer: *searcher,
                owner: *library_owner,
                zone: HiddenZoneKind::Library,
                count: u16::MAX,
                reason: "search_library".to_string(),
            },
            CryptoRequirement::VerifiableShuffle {
                owner: *library_owner,
                zone: HiddenZoneKind::Library,
                protocol: HiddenZoneProtocol::MentalPokerBayerGrothV1,
            },
        ],
        AuditCommand::ShuffleLibrary { player } => {
            vec![CryptoRequirement::VerifiableShuffle {
                owner: *player,
                zone: HiddenZoneKind::Library,
                protocol: HiddenZoneProtocol::MentalPokerBayerGrothV1,
            }]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuditCommand;

    #[test]
    fn draw_requires_private_open_only_to_owner() {
        let requirements = requirements_for_command(&AuditCommand::DrawCards {
            player: 2,
            count: 2,
        });
        assert_eq!(
            requirements,
            vec![
                CryptoRequirement::PrivateOpen {
                    viewer: 2,
                    owner: 2,
                    zone: HiddenZoneKind::Library,
                    slot: 0,
                },
                CryptoRequirement::PrivateOpen {
                    viewer: 2,
                    owner: 2,
                    zone: HiddenZoneKind::Library,
                    slot: 1,
                }
            ]
        );
    }

    #[test]
    fn search_requires_private_view_and_post_search_shuffle() {
        let requirements = requirements_for_command(&AuditCommand::SearchLibrary {
            searcher: 1,
            library_owner: 3,
            filter: "basic land".to_string(),
            selected_slot: Some(4),
        });
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
