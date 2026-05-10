use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::state::AuditState;

pub const TRANSCRIPT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditTranscript {
    pub version: u32,
    pub match_id: String,
    pub players: Vec<PlayerInfo>,
    pub initial_state_hash: String,
    pub deck_ceremonies: Vec<DeckCeremony>,
    pub actions: Vec<ActionEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerInfo {
    pub seat: u8,
    pub name: String,
    pub verifying_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeckCeremony {
    pub owner: u8,
    pub deck_id: String,
    pub declared_decklist_hash: String,
    pub initial_encrypted_deck_hash: String,
    pub required_shufflers: Vec<u8>,
    pub steps: Vec<ShuffleStep>,
    pub final_encrypted_deck_hash: String,
    pub slot_commitments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShuffleStep {
    pub shuffler: u8,
    pub input_deck_hash: String,
    pub output_deck_hash: String,
    pub entropy_commitment: String,
    pub entropy_opening: String,
    pub shuffle_proof: ShuffleProof,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "scheme", rename_all = "snake_case")]
pub enum ShuffleProof {
    /// Selected production target: a mental-poker encrypted deck shuffle proof.
    ///
    /// The concrete backend is a Bayer-Groth 2012 proof over an
    /// ElGamal-style encrypted deck using the ziffle 0.1 implementation.
    BayerGrothMentalPokerV1 {
        proof_transcript_hash: String,
        backend: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionEnvelope {
    pub seq: u64,
    pub actor: u8,
    pub prev_state_hash: String,
    pub command: AuditCommand,
    pub openings: Vec<CardOpening>,
    pub rng_reveals: Vec<RngReveal>,
    pub next_state_hash: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditCommand {
    PassPriority,
    DrawCards {
        player: u8,
        count: u8,
    },
    PublicReveal {
        owner: u8,
        slot: u16,
        reason: String,
    },
    SearchLibrary {
        searcher: u8,
        library_owner: u8,
        filter: String,
        selected_slot: Option<u16>,
    },
    ShuffleLibrary {
        player: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardOpening {
    pub owner: u8,
    pub slot: u16,
    pub card: String,
    pub salt: String,
    pub commitment: String,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    OwnerOnly,
    Public,
    Viewer { viewer: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RngReveal {
    pub event_id: String,
    pub player: u8,
    pub commitment: String,
    pub opening: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionSigningPayload {
    pub match_id: String,
    pub seq: u64,
    pub actor: u8,
    pub prev_state_hash: String,
    pub command: AuditCommand,
    pub openings: Vec<CardOpening>,
    pub rng_reveals: Vec<RngReveal>,
    pub next_state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub valid: bool,
    pub verified_actions: usize,
    pub final_state_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditFailure {
    pub seq: Option<u64>,
    pub player: Option<u8>,
    pub reason: String,
}

impl fmt::Display for AuditFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.seq, self.player) {
            (Some(seq), Some(player)) => {
                write!(f, "invalid at seq {seq}, player {player}: {}", self.reason)
            }
            (Some(seq), None) => write!(f, "invalid at seq {seq}: {}", self.reason),
            (None, Some(player)) => write!(f, "invalid for player {player}: {}", self.reason),
            (None, None) => write!(f, "invalid: {}", self.reason),
        }
    }
}

impl std::error::Error for AuditFailure {}

pub fn verify_transcript(transcript: &AuditTranscript) -> Result<VerificationReport, AuditFailure> {
    if transcript.version != TRANSCRIPT_VERSION {
        return Err(AuditFailure {
            seq: None,
            player: None,
            reason: format!(
                "unsupported transcript version {}, expected {}",
                transcript.version, TRANSCRIPT_VERSION
            ),
        });
    }

    let players = player_map(&transcript.players)?;
    if players.len() != 4 {
        return Err(AuditFailure {
            seq: None,
            player: None,
            reason: format!("expected exactly 4 players, found {}", players.len()),
        });
    }

    let deck_commitments = verify_deck_ceremonies(transcript, &players)?;
    let player_seats = players.keys().copied().collect::<BTreeSet<_>>();
    let mut audit_state = AuditState::new(player_seats, &deck_commitments);
    let mut state_hash = transcript.initial_state_hash.clone();
    let mut expected_seq = 1;
    let mut verified_actions = 0;

    for action in &transcript.actions {
        if action.seq != expected_seq {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(action.actor),
                reason: format!("expected seq {expected_seq}, found {}", action.seq),
            });
        }
        if action.prev_state_hash != state_hash {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(action.actor),
                reason: "previous state hash does not match verifier state".to_string(),
            });
        }

        verify_action_signature(transcript, action, &players)?;
        verify_rng_reveals(transcript, action)?;
        verify_card_openings(transcript, action, &deck_commitments)?;
        audit_state.apply_action(action)?;

        let computed_next = action_state_hash(
            &transcript.match_id,
            action.seq,
            &action.prev_state_hash,
            &action.command,
            &action.openings,
            &action.rng_reveals,
        )
        .map_err(|reason| AuditFailure {
            seq: Some(action.seq),
            player: Some(action.actor),
            reason,
        })?;
        if action.next_state_hash != computed_next {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(action.actor),
                reason: "next state hash does not match action payload".to_string(),
            });
        }

        state_hash = action.next_state_hash.clone();
        expected_seq += 1;
        verified_actions += 1;
    }

    Ok(VerificationReport {
        valid: true,
        verified_actions,
        final_state_hash: Some(state_hash),
    })
}

fn player_map(players: &[PlayerInfo]) -> Result<BTreeMap<u8, VerifyingKey>, AuditFailure> {
    let mut out = BTreeMap::new();
    for player in players {
        if out.contains_key(&player.seat) {
            return Err(AuditFailure {
                seq: None,
                player: Some(player.seat),
                reason: "duplicate player seat".to_string(),
            });
        }
        let key_bytes = decode_fixed_32(&player.verifying_key).map_err(|reason| AuditFailure {
            seq: None,
            player: Some(player.seat),
            reason,
        })?;
        let key = VerifyingKey::from_bytes(&key_bytes).map_err(|err| AuditFailure {
            seq: None,
            player: Some(player.seat),
            reason: format!("invalid verifying key: {err}"),
        })?;
        out.insert(player.seat, key);
    }
    Ok(out)
}

fn verify_deck_ceremonies(
    transcript: &AuditTranscript,
    players: &BTreeMap<u8, VerifyingKey>,
) -> Result<BTreeMap<(u8, u16), String>, AuditFailure> {
    let required_players = players.keys().copied().collect::<BTreeSet<_>>();
    let mut commitments = BTreeMap::new();

    for ceremony in &transcript.deck_ceremonies {
        if !players.contains_key(&ceremony.owner) {
            return Err(AuditFailure {
                seq: None,
                player: Some(ceremony.owner),
                reason: "deck ceremony references unknown owner".to_string(),
            });
        }
        let required = ceremony
            .required_shufflers
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if required != required_players {
            return Err(AuditFailure {
                seq: None,
                player: Some(ceremony.owner),
                reason: "deck ceremony does not require every player to shuffle".to_string(),
            });
        }
        if ceremony.steps.len() != required_players.len() {
            return Err(AuditFailure {
                seq: None,
                player: Some(ceremony.owner),
                reason: "deck ceremony has wrong number of shuffle steps".to_string(),
            });
        }

        let mut input = ceremony.initial_encrypted_deck_hash.clone();
        let mut seen = BTreeSet::new();
        for step in &ceremony.steps {
            if step.input_deck_hash != input {
                return Err(AuditFailure {
                    seq: None,
                    player: Some(step.shuffler),
                    reason: format!(
                        "shuffle step input does not match deck {}",
                        ceremony.deck_id
                    ),
                });
            }
            if !required_players.contains(&step.shuffler) || !seen.insert(step.shuffler) {
                return Err(AuditFailure {
                    seq: None,
                    player: Some(step.shuffler),
                    reason: "invalid or duplicate shuffle participant".to_string(),
                });
            }
            let expected_commitment = entropy_commitment(
                &transcript.match_id,
                &ceremony.deck_id,
                step.shuffler,
                &step.entropy_opening,
            );
            if step.entropy_commitment != expected_commitment {
                return Err(AuditFailure {
                    seq: None,
                    player: Some(step.shuffler),
                    reason: "shuffle entropy opening does not match commitment".to_string(),
                });
            }
            let expected_proof_hash = shuffle_proof_transcript_hash(
                &transcript.match_id,
                &ceremony.deck_id,
                step.shuffler,
                &step.input_deck_hash,
                &step.output_deck_hash,
                &step.entropy_commitment,
            );
            verify_shuffle_proof(step, &expected_proof_hash)?;
            input = step.output_deck_hash.clone();
        }
        if input != ceremony.final_encrypted_deck_hash {
            return Err(AuditFailure {
                seq: None,
                player: Some(ceremony.owner),
                reason: "final encrypted deck hash does not match last shuffle step".to_string(),
            });
        }
        for (slot, commitment) in ceremony.slot_commitments.iter().enumerate() {
            commitments.insert((ceremony.owner, slot as u16), commitment.clone());
        }
    }

    Ok(commitments)
}

fn verify_shuffle_proof(step: &ShuffleStep, expected_hash: &str) -> Result<(), AuditFailure> {
    match &step.shuffle_proof {
        ShuffleProof::BayerGrothMentalPokerV1 {
            proof_transcript_hash,
            backend,
        } => {
            if !crate::backend::backend_is_supported(backend) {
                return Err(AuditFailure {
                    seq: None,
                    player: Some(step.shuffler),
                    reason: format!("unsupported shuffle proof backend {backend}"),
                });
            }
            if proof_transcript_hash != expected_hash {
                return Err(AuditFailure {
                    seq: None,
                    player: Some(step.shuffler),
                    reason: "shuffle proof does not bind the transcripted step".to_string(),
                });
            }
            Ok(())
        }
    }
}

fn verify_action_signature(
    transcript: &AuditTranscript,
    action: &ActionEnvelope,
    players: &BTreeMap<u8, VerifyingKey>,
) -> Result<(), AuditFailure> {
    let Some(key) = players.get(&action.actor) else {
        return Err(AuditFailure {
            seq: Some(action.seq),
            player: Some(action.actor),
            reason: "unknown actor".to_string(),
        });
    };
    let signature_bytes = decode_vec(&action.signature).map_err(|reason| AuditFailure {
        seq: Some(action.seq),
        player: Some(action.actor),
        reason,
    })?;
    let signature = Signature::from_slice(&signature_bytes).map_err(|err| AuditFailure {
        seq: Some(action.seq),
        player: Some(action.actor),
        reason: format!("invalid signature bytes: {err}"),
    })?;
    let payload = ActionSigningPayload {
        match_id: transcript.match_id.clone(),
        seq: action.seq,
        actor: action.actor,
        prev_state_hash: action.prev_state_hash.clone(),
        command: action.command.clone(),
        openings: action.openings.clone(),
        rng_reveals: action.rng_reveals.clone(),
        next_state_hash: action.next_state_hash.clone(),
    };
    let bytes = canonical_bytes(&payload).map_err(|reason| AuditFailure {
        seq: Some(action.seq),
        player: Some(action.actor),
        reason,
    })?;
    key.verify(&bytes, &signature).map_err(|err| AuditFailure {
        seq: Some(action.seq),
        player: Some(action.actor),
        reason: format!("signature verification failed: {err}"),
    })
}

fn verify_rng_reveals(
    transcript: &AuditTranscript,
    action: &ActionEnvelope,
) -> Result<(), AuditFailure> {
    for reveal in &action.rng_reveals {
        let expected = rng_commitment(
            &transcript.match_id,
            &reveal.event_id,
            reveal.player,
            &reveal.opening,
        );
        if reveal.commitment != expected {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(reveal.player),
                reason: format!("rng opening does not match commitment {}", reveal.event_id),
            });
        }
    }
    Ok(())
}

fn verify_card_openings(
    transcript: &AuditTranscript,
    action: &ActionEnvelope,
    deck_commitments: &BTreeMap<(u8, u16), String>,
) -> Result<(), AuditFailure> {
    for opening in &action.openings {
        let expected = card_commitment(
            &transcript.match_id,
            opening.owner,
            opening.slot,
            &opening.card,
            &opening.salt,
        );
        if opening.commitment != expected {
            return Err(AuditFailure {
                seq: Some(action.seq),
                player: Some(opening.owner),
                reason: format!(
                    "card opening for slot {} does not match commitment",
                    opening.slot
                ),
            });
        }
        match deck_commitments.get(&(opening.owner, opening.slot)) {
            Some(committed) if committed == &opening.commitment => {}
            Some(_) => {
                return Err(AuditFailure {
                    seq: Some(action.seq),
                    player: Some(opening.owner),
                    reason: format!(
                        "opened card does not match committed deck slot {}",
                        opening.slot
                    ),
                });
            }
            None => {
                return Err(AuditFailure {
                    seq: Some(action.seq),
                    player: Some(opening.owner),
                    reason: format!("opened unknown deck slot {}", opening.slot),
                });
            }
        }
    }
    Ok(())
}

pub fn action_state_hash(
    match_id: &str,
    seq: u64,
    prev_state_hash: &str,
    command: &AuditCommand,
    openings: &[CardOpening],
    rng_reveals: &[RngReveal],
) -> Result<String, String> {
    #[derive(Serialize)]
    struct StateInput<'a> {
        domain: &'static str,
        match_id: &'a str,
        seq: u64,
        prev_state_hash: &'a str,
        command: &'a AuditCommand,
        openings: &'a [CardOpening],
        rng_reveals: &'a [RngReveal],
    }

    hash_json(&StateInput {
        domain: "ironsmith-audit-state-v1",
        match_id,
        seq,
        prev_state_hash,
        command,
        openings,
        rng_reveals,
    })
}

pub fn initial_state_hash(match_id: &str, players: &[u8]) -> String {
    hash_bytes(
        b"ironsmith-audit-initial-state-v1",
        &[match_id.as_bytes(), &players.to_vec()],
    )
}

pub fn card_commitment(match_id: &str, owner: u8, slot: u16, card: &str, salt: &str) -> String {
    hash_bytes(
        b"ironsmith-audit-card-commitment-v1",
        &[
            match_id.as_bytes(),
            &[owner],
            &slot.to_le_bytes(),
            card.as_bytes(),
            salt.as_bytes(),
        ],
    )
}

pub fn entropy_commitment(match_id: &str, deck_id: &str, shuffler: u8, opening: &str) -> String {
    hash_bytes(
        b"ironsmith-audit-shuffle-entropy-v1",
        &[
            match_id.as_bytes(),
            deck_id.as_bytes(),
            &[shuffler],
            opening.as_bytes(),
        ],
    )
}

pub fn shuffle_proof_transcript_hash(
    match_id: &str,
    deck_id: &str,
    shuffler: u8,
    input_hash: &str,
    output_hash: &str,
    entropy_commitment: &str,
) -> String {
    hash_bytes(
        b"ironsmith-audit-shuffle-proof-v1",
        &[
            match_id.as_bytes(),
            deck_id.as_bytes(),
            &[shuffler],
            input_hash.as_bytes(),
            output_hash.as_bytes(),
            entropy_commitment.as_bytes(),
        ],
    )
}

pub fn rng_commitment(match_id: &str, event_id: &str, player: u8, opening: &str) -> String {
    hash_bytes(
        b"ironsmith-audit-rng-commitment-v1",
        &[
            match_id.as_bytes(),
            event_id.as_bytes(),
            &[player],
            opening.as_bytes(),
        ],
    )
}

pub fn encrypted_deck_hash(match_id: &str, deck_id: &str, label: &str, previous: &str) -> String {
    hash_bytes(
        b"ironsmith-audit-encrypted-deck-v1",
        &[
            match_id.as_bytes(),
            deck_id.as_bytes(),
            label.as_bytes(),
            previous.as_bytes(),
        ],
    )
}

pub fn decklist_hash(match_id: &str, owner: u8, cards: &[String]) -> Result<String, String> {
    #[derive(Serialize)]
    struct Decklist<'a> {
        domain: &'static str,
        match_id: &'a str,
        owner: u8,
        cards: &'a [String],
    }
    hash_json(&Decklist {
        domain: "ironsmith-audit-decklist-v1",
        match_id,
        owner,
        cards,
    })
}

pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|err| format!("canonical serialization failed: {err}"))
}

fn hash_json<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = canonical_bytes(value)?;
    Ok(hash_bytes(b"ironsmith-audit-json-v1", &[&bytes]))
}

fn hash_bytes(domain: &[u8], parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    encode_hex(&hasher.finalize())
}

pub fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn decode_vec(hex: &str) -> Result<Vec<u8>, String> {
    let trimmed = hex.trim();
    if !trimmed.len().is_multiple_of(2) {
        return Err("hex string has odd length".to_string());
    }
    let mut out = Vec::with_capacity(trimmed.len() / 2);
    for idx in (0..trimmed.len()).step_by(2) {
        let high = decode_nibble(trimmed.as_bytes()[idx])?;
        let low = decode_nibble(trimmed.as_bytes()[idx + 1])?;
        out.push((high << 4) | low);
    }
    Ok(out)
}

fn decode_fixed_32(hex: &str) -> Result<[u8; 32], String> {
    let bytes = decode_vec(hex)?;
    bytes
        .try_into()
        .map_err(|_| "expected 32-byte hex value".to_string())
}

fn decode_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("invalid hex character".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{cheating_transcript, fair_transcript};

    #[test]
    fn fair_fixture_verifies() {
        let transcript = fair_transcript().expect("fixture should build");
        let report = verify_transcript(&transcript).expect("fair transcript should verify");
        assert!(report.valid);
        assert_eq!(report.verified_actions, 5);
    }

    #[test]
    fn cheating_fixture_is_detected() {
        let transcript = cheating_transcript().expect("fixture should build");
        let err = verify_transcript(&transcript).expect_err("cheating transcript must fail");
        assert_eq!(err.seq, Some(3));
        assert_eq!(err.player, Some(2));
        assert!(
            err.reason.contains("does not match committed deck slot"),
            "{}",
            err.reason
        );
    }

    #[test]
    fn ziffle_backend_supports_four_player_shuffle_and_reveal() {
        let deck = real_engine_deck();
        assert_eq!(deck.len(), 10);

        let mut rng = ark_std::test_rng();
        let ctx = b"ironsmith-audit::four-player-ziffle-smoke";
        let revealed = crate::backend::ziffle_four_player_round_trip::<10, _>(&mut rng, ctx)
            .expect("ziffle backend should reveal");
        assert!(revealed < 10);
        let revealed_card_name = &deck[revealed];
        assert!(
            !revealed_card_name.trim().is_empty(),
            "revealed encrypted index should map to a real engine card"
        );
    }

    fn real_engine_deck() -> Vec<String> {
        let requested = [
            "Forest",
            "Island",
            "Mountain",
            "Swamp",
            "Llanowar Elves",
            "Counterspell",
            "Lightning Bolt",
            "Sol Ring",
            "Giant Growth",
            "Grizzly Bears",
        ];
        let registry = ironsmith_registry::CardRegistry::with_builtin_cards();
        requested
            .iter()
            .map(|name| {
                registry
                    .get(name)
                    .unwrap_or_else(|| panic!("registry should load real MTG card {name}"))
                    .card
                    .name
                    .clone()
            })
            .collect()
    }
}

pub mod backend;
pub mod fixtures;
pub mod protocol;
pub mod runtime_bridge;
mod state;
