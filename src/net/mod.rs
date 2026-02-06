use serde::Deserializer;
use serde::Serializer;
use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::game_state::{Phase, Step};
use crate::{ObjectId, PlayerId, Zone};

pub mod adapters;
pub mod codec;
pub mod crypto;
pub mod message;
pub mod runtime;
pub mod state;
pub use codec::{CanonicalDecode, CanonicalEncode, CodecError};

// Network-identity primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash32(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PubKey(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sig64(pub [u8; 64]);

impl Serialize for Sig64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

struct Sig64Visitor;

impl<'de> Visitor<'de> for Sig64Visitor {
    type Value = Sig64;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a 64-byte signature")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.len() != 64 {
            return Err(E::invalid_length(v.len(), &self));
        }
        let mut out = [0u8; 64];
        out.copy_from_slice(v);
        Ok(Sig64(out))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_bytes(&v)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut out = [0u8; 64];
        for i in 0..64 {
            out[i] = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(i, &self))?;
        }
        Ok(Sig64(out))
    }
}

impl<'de> Deserialize<'de> for Sig64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(Sig64Visitor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub [u8; 32]);

pub type SeqNum = u64;

// Game-identity wrappers for network encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GamePlayerId(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameObjectId(pub u64);

impl From<PlayerId> for GamePlayerId {
    fn from(value: PlayerId) -> Self {
        Self(value.0)
    }
}

impl From<GamePlayerId> for PlayerId {
    fn from(value: GamePlayerId) -> Self {
        PlayerId(value.0)
    }
}

impl From<ObjectId> for GameObjectId {
    fn from(value: ObjectId) -> Self {
        Self(value.0)
    }
}

impl From<GameObjectId> for ObjectId {
    fn from(value: GameObjectId) -> Self {
        ObjectId(value.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ZoneCode {
    Library = 0,
    Hand = 1,
    Battlefield = 2,
    Graveyard = 3,
    Stack = 4,
    Exile = 5,
    Command = 6,
}

impl From<Zone> for ZoneCode {
    fn from(value: Zone) -> Self {
        match value {
            Zone::Library => ZoneCode::Library,
            Zone::Hand => ZoneCode::Hand,
            Zone::Battlefield => ZoneCode::Battlefield,
            Zone::Graveyard => ZoneCode::Graveyard,
            Zone::Stack => ZoneCode::Stack,
            Zone::Exile => ZoneCode::Exile,
            Zone::Command => ZoneCode::Command,
        }
    }
}

impl From<ZoneCode> for Zone {
    fn from(value: ZoneCode) -> Self {
        match value {
            ZoneCode::Library => Zone::Library,
            ZoneCode::Hand => Zone::Hand,
            ZoneCode::Battlefield => Zone::Battlefield,
            ZoneCode::Graveyard => Zone::Graveyard,
            ZoneCode::Stack => Zone::Stack,
            ZoneCode::Exile => Zone::Exile,
            ZoneCode::Command => Zone::Command,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PhaseCode {
    Beginning = 0,
    FirstMain = 1,
    Combat = 2,
    NextMain = 3,
    Ending = 4,
}

impl From<Phase> for PhaseCode {
    fn from(value: Phase) -> Self {
        match value {
            Phase::Beginning => PhaseCode::Beginning,
            Phase::FirstMain => PhaseCode::FirstMain,
            Phase::Combat => PhaseCode::Combat,
            Phase::NextMain => PhaseCode::NextMain,
            Phase::Ending => PhaseCode::Ending,
        }
    }
}

impl From<PhaseCode> for Phase {
    fn from(value: PhaseCode) -> Self {
        match value {
            PhaseCode::Beginning => Phase::Beginning,
            PhaseCode::FirstMain => Phase::FirstMain,
            PhaseCode::Combat => Phase::Combat,
            PhaseCode::NextMain => Phase::NextMain,
            PhaseCode::Ending => Phase::Ending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum StepCode {
    Untap = 0,
    Upkeep = 1,
    Draw = 2,
    BeginCombat = 3,
    DeclareAttackers = 4,
    DeclareBlockers = 5,
    CombatDamage = 6,
    EndCombat = 7,
    End = 8,
    Cleanup = 9,
}

impl From<Step> for StepCode {
    fn from(value: Step) -> Self {
        match value {
            Step::Untap => StepCode::Untap,
            Step::Upkeep => StepCode::Upkeep,
            Step::Draw => StepCode::Draw,
            Step::BeginCombat => StepCode::BeginCombat,
            Step::DeclareAttackers => StepCode::DeclareAttackers,
            Step::DeclareBlockers => StepCode::DeclareBlockers,
            Step::CombatDamage => StepCode::CombatDamage,
            Step::EndCombat => StepCode::EndCombat,
            Step::End => StepCode::End,
            Step::Cleanup => StepCode::Cleanup,
        }
    }
}

impl From<StepCode> for Step {
    fn from(value: StepCode) -> Self {
        match value {
            StepCode::Untap => Step::Untap,
            StepCode::Upkeep => Step::Upkeep,
            StepCode::Draw => Step::Draw,
            StepCode::BeginCombat => Step::BeginCombat,
            StepCode::DeclareAttackers => Step::DeclareAttackers,
            StepCode::DeclareBlockers => Step::DeclareBlockers,
            StepCode::CombatDamage => Step::CombatDamage,
            StepCode::EndCombat => Step::EndCombat,
            StepCode::End => Step::End,
            StepCode::Cleanup => Step::Cleanup,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ObjectKindCode {
    Card = 0,
    Token = 1,
    SpellCopy = 2,
    Emblem = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ManaPoolSpec {
    pub white: u32,
    pub blue: u32,
    pub black: u32,
    pub red: u32,
    pub green: u32,
    pub colorless: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicTurnState {
    pub active_player: GamePlayerId,
    pub priority_player: Option<GamePlayerId>,
    pub turn_number: u32,
    pub phase: PhaseCode,
    pub step: Option<StepCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicPlayerState {
    pub id: GamePlayerId,
    pub life: i32,
    pub mana_pool: ManaPoolSpec,
    pub poison_counters: u32,
    pub energy_counters: u32,
    pub experience_counters: u32,
    pub lands_played_this_turn: u32,
    pub land_plays_per_turn: u32,
    pub max_hand_size: i32,
    pub has_lost: bool,
    pub has_won: bool,
    pub has_left_game: bool,
    pub hand_size: u32,
    pub library_size: u32,
    pub graveyard: Vec<GameObjectId>,
    pub commanders: Vec<GameObjectId>,
    pub commander_damage: Vec<(GamePlayerId, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicZoneIndex {
    pub battlefield: Vec<GameObjectId>,
    pub exile: Vec<GameObjectId>,
    pub command_zone: Vec<GameObjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicObjectState {
    pub id: GameObjectId,
    pub stable_id: GameObjectId,
    pub kind: ObjectKindCode,
    pub card_ref: Option<u32>,
    pub zone: ZoneCode,
    pub owner: GamePlayerId,
    pub controller: GamePlayerId,
    pub object_hash: Hash32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicStackEntry {
    pub object_id: GameObjectId,
    pub controller: GamePlayerId,
    pub entry_hash: Hash32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicStateSnapshot {
    pub version: u8,
    pub turn: PublicTurnState,
    pub turn_order: Vec<GamePlayerId>,
    pub players: Vec<PublicPlayerState>,
    pub zones: PublicZoneIndex,
    pub objects: Vec<PublicObjectState>,
    pub stack: Vec<PublicStackEntry>,
    pub combat_hash: Option<Hash32>,
    pub trackers_hash: Hash32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum CommitmentScheme {
    ElgamalSecp256k1Sha256MerkleV1 = 0,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneCommitment {
    pub scheme: CommitmentScheme,
    pub root: Hash32,
    pub len: u32,
    pub top: u32,
    pub viewers: Vec<PeerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiddenZoneCommitment {
    pub zone: ZoneCode,
    pub owner: GamePlayerId,
    pub commitment: ZoneCommitment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateCommitment {
    pub version: u8,
    pub public_state_hash: Hash32,
    pub hidden_zones: Vec<HiddenZoneCommitment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhaseStep {
    pub phase: PhaseCode,
    pub step: Option<StepCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ManaSymbolCode {
    White = 0,
    Blue = 1,
    Black = 2,
    Red = 3,
    Green = 4,
    Colorless = 5,
    Generic = 6,
    Snow = 7,
    Life = 8,
    X = 9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ManaSymbolSpec {
    pub symbol: ManaSymbolCode,
    pub value: u8,
}

impl PhaseStep {
    pub fn from_phase_step(phase: Phase, step: Option<Step>) -> Self {
        Self {
            phase: phase.into(),
            step: step.map(Into::into),
        }
    }

    pub fn to_phase_step(self) -> (Phase, Option<Step>) {
        (self.phase.into(), self.step.map(Into::into))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum MsgType {
    ActionPropose = 1,
    ActionAck = 2,
    ActionReject = 3,
    ActionCommit = 4,
    PolicyToken = 5,
    PolicyCancel = 6,
    ContribRequest = 7,
    ContribShare = 8,
    TimeoutClaim = 9,
    ForfeitCommit = 10,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub msg_type: MsgType,
    pub sender: PeerId,
    pub session_id: SessionId,
    pub seq: SeqNum,
    pub payload: Vec<u8>,
    pub sig: Sig64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RejectCode {
    InvalidAction = 0,
    InvalidProof = 1,
    InvalidStateHash = 2,
    NotPriorityHolder = 3,
    Unauthorized = 4,
    Malformed = 5,
    Other = 255,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TimeoutReason {
    MissingAck = 0,
    MissingShare = 1,
    MissingShuffleProof = 2,
    MissingPolicyResponse = 3,
    Other = 255,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ActionKind {
    PassPriority = 0,
    CastSpell = 1,
    ActivateAbility = 2,
    ResolveTop = 3,
    DrawCard = 4,
    ShuffleLibrary = 5,
    SearchLibrary = 6,
    ReorderTopN = 7,
    MoveCard = 8,
    RevealCard = 9,
    Other = 255,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPropose {
    pub action_id: Hash32,
    pub prev_state_hash: Hash32,
    pub action: ActionPayload,
    pub proofs: ProofBundle,
    pub contribs_hash: Option<Hash32>,
    pub proposer_sig: Sig64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionAck {
    pub action_id: Hash32,
    pub ack_sig: Sig64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionReject {
    pub action_id: Hash32,
    pub reason: RejectCode,
    pub reject_sig: Sig64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionCommit {
    pub action_id: Hash32,
    pub ack_sigs: Vec<(PeerId, Sig64)>,
    pub commit_sig: Sig64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContribRequest {
    pub request_id: Hash32,
    pub prev_state_hash: Hash32,
    pub action_kind: ActionKind,
    pub required_from: Vec<PeerId>,
    pub deadline_ms: u64,
    pub request_sig: Sig64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContribShare {
    pub request_id: Hash32,
    pub contributor: PeerId,
    pub share_payload: Vec<u8>,
    pub share_proof: Vec<u8>,
    pub share_sig: Sig64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyToken {
    pub policy_id: Hash32,
    pub owner: PeerId,
    pub active_from_state_hash: Hash32,
    pub expires_at: PhaseStep,
    pub conditions: PolicyConditions,
    pub owner_sig: Sig64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyCancel {
    pub policy_id: Hash32,
    pub cancel_sig: Sig64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyConditions {
    pub stop_on_stack_event: bool,
    pub stop_if_targets_me: bool,
    pub stop_if_attackers_declared: bool,
    pub stop_if_blockers_declared: bool,
    pub until_phase: PhaseStep,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutClaim {
    pub action_id: Hash32,
    pub missing_peer: PeerId,
    pub reason: TimeoutReason,
    pub claimer_sig: Sig64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForfeitCommit {
    pub missing_peer: PeerId,
    pub reason: TimeoutReason,
    pub claim_sigs: Vec<(PeerId, Sig64)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetSpec {
    Object(GameObjectId),
    Player(GamePlayerId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CostPayment {
    Tap {
        objects: Vec<GameObjectId>,
    },
    Untap {
        objects: Vec<GameObjectId>,
    },
    Sacrifice {
        objects: Vec<GameObjectId>,
    },
    Discard {
        objects: Vec<GameObjectId>,
    },
    Exile {
        objects: Vec<GameObjectId>,
        from_zone: ZoneCode,
    },
    Reveal {
        objects: Vec<GameObjectId>,
    },
    ReturnToHand {
        objects: Vec<GameObjectId>,
    },
    ActivateManaAbility {
        source: GameObjectId,
        ability_index: u32,
    },
    Life {
        amount: u32,
    },
    Energy {
        amount: u32,
    },
    Mill {
        count: u32,
    },
    Other {
        tag: u8,
        data: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CostStep {
    Mana(ManaSymbolSpec),
    Payment(CostPayment),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CostSpec {
    /// Ordered sequence of payment steps as performed by the payer.
    /// This must preserve the exact payment order to keep deterministic replay.
    pub payment_trace: Vec<CostStep>,
    /// Optional costs paid, aligned to the card's optional cost list by index.
    /// Each entry is the number of times the optional cost at that index was paid.
    pub optional_costs: Vec<u32>,
    pub x_value: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPayload {
    PassPriority {
        policy_id: Option<Hash32>,
    },
    CastSpell {
        card_ref: GameObjectId,
        targets: Vec<TargetSpec>,
        costs: CostSpec,
    },
    ActivateAbility {
        source_ref: GameObjectId,
        costs: CostSpec,
        targets: Vec<TargetSpec>,
    },
    ResolveTop,
    DrawCard {
        count: u8,
    },
    ShuffleLibrary,
    SearchLibrary {
        predicate_root: Hash32,
        reveal: bool,
    },
    ReorderTopN {
        n: u8,
        new_commitment: Hash32,
    },
    MoveCard {
        from_zone: ZoneCode,
        to_zone: ZoneCode,
        card_ref: GameObjectId,
    },
    RevealCard {
        card_ref: GameObjectId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProofBundle {
    pub shuffle_proof: Option<Vec<u8>>,
    pub decrypt_shares: Vec<Vec<u8>>,
    pub decrypt_share_proofs: Vec<Vec<u8>>,
    pub plaintext_commitment: Option<Hash32>,
    pub plaintext_eq_proof: Option<Vec<u8>>,
    pub membership_stark: Option<Vec<u8>>,
    pub permutation_stark: Option<Vec<u8>>,
}
