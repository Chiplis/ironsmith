use super::crypto::{
    Signer, sign_action_ack, sign_action_commit, sign_action_propose, sign_action_reject,
    sign_contrib_request, sign_contrib_share, sign_envelope, sign_policy_cancel, sign_policy_token,
    sign_timeout_claim,
};
use super::state;
use super::*;
use crate::game_state::GameState;

fn zero_hash() -> Hash32 {
    Hash32([0u8; 32])
}

fn zero_sig() -> Sig64 {
    Sig64([0u8; 64])
}

pub struct EnvelopeBuilder<'a, S: Signer> {
    signer: &'a S,
    sender: PeerId,
    session_id: SessionId,
    seq: SeqNum,
}

impl<'a, S: Signer> EnvelopeBuilder<'a, S> {
    pub fn new(signer: &'a S, sender: PeerId, session_id: SessionId, starting_seq: SeqNum) -> Self {
        Self {
            signer,
            sender,
            session_id,
            seq: starting_seq,
        }
    }

    pub fn next_seq(&mut self) -> SeqNum {
        let current = self.seq;
        self.seq = self.seq.wrapping_add(1);
        current
    }

    pub fn wrap<T: CanonicalEncode>(&mut self, msg_type: MsgType, payload: &T) -> Envelope {
        let seq = self.next_seq();
        let mut envelope = Envelope {
            msg_type,
            sender: self.sender,
            session_id: self.session_id,
            seq,
            payload: payload.to_bytes(),
            sig: zero_sig(),
        };
        sign_envelope(self.signer, &mut envelope);
        envelope
    }
}

pub fn build_action_propose(
    signer: &impl Signer,
    prev_state_hash: Hash32,
    action: ActionPayload,
    proofs: ProofBundle,
    contribs_hash: Option<Hash32>,
) -> ActionPropose {
    let mut propose = ActionPropose {
        action_id: zero_hash(),
        prev_state_hash,
        action,
        proofs,
        contribs_hash,
        proposer_sig: zero_sig(),
    };
    sign_action_propose(signer, &mut propose);
    propose
}

pub fn build_action_propose_for_game(
    signer: &impl Signer,
    game: &GameState,
    action: ActionPayload,
    proofs: ProofBundle,
    contribs_hash: Option<Hash32>,
) -> ActionPropose {
    let prev_state_hash = state::hash_public_state(game);
    build_action_propose(signer, prev_state_hash, action, proofs, contribs_hash)
}

pub fn build_action_ack(signer: &impl Signer, action_id: Hash32) -> ActionAck {
    let mut ack = ActionAck {
        action_id,
        ack_sig: zero_sig(),
    };
    sign_action_ack(signer, &mut ack);
    ack
}

pub fn build_action_reject(
    signer: &impl Signer,
    action_id: Hash32,
    reason: RejectCode,
) -> ActionReject {
    let mut reject = ActionReject {
        action_id,
        reason,
        reject_sig: zero_sig(),
    };
    sign_action_reject(signer, &mut reject);
    reject
}

pub fn build_action_commit(
    signer: &impl Signer,
    action_id: Hash32,
    mut ack_sigs: Vec<(PeerId, Sig64)>,
) -> ActionCommit {
    ack_sigs.sort_by_key(|(peer, _)| peer.0);
    let mut commit = ActionCommit {
        action_id,
        ack_sigs,
        commit_sig: zero_sig(),
    };
    sign_action_commit(signer, &mut commit);
    commit
}

pub fn build_contrib_request(
    signer: &impl Signer,
    prev_state_hash: Hash32,
    action_kind: ActionKind,
    required_from: Vec<PeerId>,
    deadline_ms: u64,
) -> ContribRequest {
    let mut request = ContribRequest {
        request_id: zero_hash(),
        prev_state_hash,
        action_kind,
        required_from,
        deadline_ms,
        request_sig: zero_sig(),
    };
    sign_contrib_request(signer, &mut request);
    request
}

pub fn build_contrib_request_for_game(
    signer: &impl Signer,
    game: &GameState,
    action_kind: ActionKind,
    required_from: Vec<PeerId>,
    deadline_ms: u64,
) -> ContribRequest {
    let prev_state_hash = state::hash_public_state(game);
    build_contrib_request(
        signer,
        prev_state_hash,
        action_kind,
        required_from,
        deadline_ms,
    )
}

pub fn build_contrib_share(
    signer: &impl Signer,
    request_id: Hash32,
    contributor: PeerId,
    share_payload: Vec<u8>,
    share_proof: Vec<u8>,
) -> ContribShare {
    let mut share = ContribShare {
        request_id,
        contributor,
        share_payload,
        share_proof,
        share_sig: zero_sig(),
    };
    sign_contrib_share(signer, &mut share);
    share
}

pub fn build_policy_token(
    signer: &impl Signer,
    owner: PeerId,
    active_from_state_hash: Hash32,
    expires_at: PhaseStep,
    conditions: PolicyConditions,
) -> PolicyToken {
    let mut token = PolicyToken {
        policy_id: zero_hash(),
        owner,
        active_from_state_hash,
        expires_at,
        conditions,
        owner_sig: zero_sig(),
    };
    sign_policy_token(signer, &mut token);
    token
}

pub fn build_policy_cancel(signer: &impl Signer, policy_id: Hash32) -> PolicyCancel {
    let mut cancel = PolicyCancel {
        policy_id,
        cancel_sig: zero_sig(),
    };
    sign_policy_cancel(signer, &mut cancel);
    cancel
}

pub fn build_timeout_claim(
    signer: &impl Signer,
    action_id: Hash32,
    missing_peer: PeerId,
    reason: TimeoutReason,
) -> TimeoutClaim {
    let mut claim = TimeoutClaim {
        action_id,
        missing_peer,
        reason,
        claimer_sig: zero_sig(),
    };
    sign_timeout_claim(signer, &mut claim);
    claim
}

pub fn build_forfeit_commit(
    missing_peer: PeerId,
    reason: TimeoutReason,
    mut claim_sigs: Vec<(PeerId, Sig64)>,
) -> ForfeitCommit {
    claim_sigs.sort_by_key(|(peer, _)| peer.0);
    ForfeitCommit {
        missing_peer,
        reason,
        claim_sigs,
    }
}
