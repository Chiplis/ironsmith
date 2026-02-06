use sha2::{Digest, Sha256};

use super::{
    ActionAck, ActionCommit, ActionPropose, ActionReject, CanonicalEncode, ContribRequest,
    ContribShare, Envelope, Hash32, PolicyCancel, PolicyToken, PubKey, Sig64, StateCommitment,
    TimeoutClaim,
};

use secp256k1::Keypair;
use secp256k1::SecretKey;
use secp256k1::schnorr::Signature;
use secp256k1::{Secp256k1, XOnlyPublicKey};

pub const DOMAIN_SESSION: &[u8] = b"mtg/session";
pub const DOMAIN_ACTION: &[u8] = b"mtg/action";
pub const DOMAIN_POLICY: &[u8] = b"mtg/policy";
pub const DOMAIN_CONTRIB: &[u8] = b"mtg/contrib";
pub const DOMAIN_PROOF: &[u8] = b"mtg/proof";
pub const DOMAIN_ENVELOPE: &[u8] = b"mtg/envelope";
pub const DOMAIN_STATE_ROOT: &[u8] = b"mtg/state_root/v1";
pub const DOMAIN_PUBLIC_STATE: &[u8] = b"mtg/public_state/v1";
pub const DOMAIN_ZONE_LEAF: &[u8] = b"mtg/zone_leaf/v1";
pub const DOMAIN_ZONE_NODE: &[u8] = b"mtg/zone_node/v1";
pub const DOMAIN_ZONE_EMPTY: &[u8] = b"mtg/zone_empty/v1";
pub const DOMAIN_PLAINTEXT_COMMIT: &[u8] = b"mtg/plaintext/v1";
pub const DOMAIN_PUBLIC_OBJECT: &[u8] = b"mtg/object_public/v1";
pub const DOMAIN_STACK_STATE: &[u8] = b"mtg/stack/v1";
pub const DOMAIN_COMBAT_STATE: &[u8] = b"mtg/combat/v1";
pub const DOMAIN_TRACKERS_STATE: &[u8] = b"mtg/trackers/v1";

pub trait Signer {
    fn sign(&self, msg: &[u8]) -> Sig64;
}

pub trait Verifier {
    fn verify(&self, msg: &[u8], sig: Sig64) -> bool;
}

pub struct Secp256k1Signer {
    keypair: Keypair,
}

impl Secp256k1Signer {
    pub fn from_secret_bytes(secret: [u8; 32]) -> Result<Self, secp256k1::Error> {
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&secret)?;
        let keypair = Keypair::from_secret_key(&secp, &sk);
        Ok(Self { keypair })
    }

    pub fn public_key(&self) -> PubKey {
        let (xonly, _) = self.keypair.x_only_public_key();
        PubKey(xonly.serialize())
    }

    fn sign_message(&self, msg: &[u8]) -> Result<Signature, secp256k1::Error> {
        let secp = Secp256k1::new();
        Ok(secp.sign_schnorr_no_aux_rand(msg, &self.keypair))
    }
}

impl Signer for Secp256k1Signer {
    fn sign(&self, msg: &[u8]) -> Sig64 {
        let signature = self.sign_message(msg).expect("schnorr signing failed");
        Sig64(*signature.as_ref())
    }
}

pub struct Secp256k1Verifier {
    pubkey: XOnlyPublicKey,
}

impl Secp256k1Verifier {
    pub fn from_pubkey_bytes(pubkey: PubKey) -> Result<Self, secp256k1::Error> {
        let xonly = XOnlyPublicKey::from_slice(&pubkey.0)?;
        Ok(Self { pubkey: xonly })
    }

    fn verify_message(&self, msg: &[u8], sig: Sig64) -> Result<(), secp256k1::Error> {
        let secp = Secp256k1::verification_only();
        let signature = Signature::from_slice(&sig.0)?;
        secp.verify_schnorr(&signature, msg, &self.pubkey)
    }
}

impl Verifier for Secp256k1Verifier {
    fn verify(&self, msg: &[u8], sig: Sig64) -> bool {
        self.verify_message(msg, sig).is_ok()
    }
}

pub fn verify_signature(pubkey: PubKey, msg: &[u8], sig: Sig64) -> bool {
    Secp256k1Verifier::from_pubkey_bytes(pubkey)
        .map(|verifier| verifier.verify(msg, sig))
        .unwrap_or(false)
}

pub fn hash_with_domain(domain: &[u8], payload: &[u8]) -> Hash32 {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(payload);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest[..]);
    Hash32(out)
}

pub fn hash_action_payload(propose: &ActionPropose) -> Hash32 {
    hash_with_domain(DOMAIN_ACTION, &propose.encode_for_hash())
}

pub fn hash_action_reject(reject: &ActionReject) -> Hash32 {
    hash_with_domain(DOMAIN_ACTION, &reject.encode_for_hash())
}

pub fn hash_action_commit(commit: &ActionCommit) -> Hash32 {
    hash_with_domain(DOMAIN_ACTION, &commit.encode_for_hash())
}

pub fn hash_contrib_payload(request: &ContribRequest) -> Hash32 {
    hash_with_domain(DOMAIN_CONTRIB, &request.encode_for_hash())
}

pub fn hash_contrib_share(share: &ContribShare) -> Hash32 {
    hash_with_domain(DOMAIN_CONTRIB, &share.encode_for_hash())
}

pub fn hash_policy_payload(policy: &PolicyToken) -> Hash32 {
    hash_with_domain(DOMAIN_POLICY, &policy.encode_for_hash())
}

pub fn hash_policy_cancel(cancel: &PolicyCancel) -> Hash32 {
    hash_with_domain(DOMAIN_POLICY, &cancel.encode_for_hash())
}

pub fn hash_envelope(envelope: &Envelope) -> Hash32 {
    hash_with_domain(DOMAIN_ENVELOPE, &envelope.encode_for_hash())
}

pub fn hash_timeout_claim(claim: &TimeoutClaim) -> Hash32 {
    hash_with_domain(DOMAIN_ACTION, &claim.encode_for_hash())
}

pub fn set_action_id(propose: &mut ActionPropose) -> Hash32 {
    let action_id = hash_action_payload(propose);
    propose.action_id = action_id;
    action_id
}

pub fn sign_action_propose(signer: &impl Signer, propose: &mut ActionPropose) -> Hash32 {
    let action_id = set_action_id(propose);
    propose.proposer_sig = signer.sign(&action_id.0);
    action_id
}

pub fn sign_action_ack(signer: &impl Signer, ack: &mut ActionAck) {
    ack.ack_sig = signer.sign(&ack.action_id.0);
}

pub fn sign_action_reject(signer: &impl Signer, reject: &mut ActionReject) -> Hash32 {
    let digest = hash_action_reject(reject);
    reject.reject_sig = signer.sign(&digest.0);
    digest
}

pub fn sign_action_commit(signer: &impl Signer, commit: &mut ActionCommit) -> Hash32 {
    let digest = hash_action_commit(commit);
    commit.commit_sig = signer.sign(&digest.0);
    digest
}

pub fn set_contrib_request_id(request: &mut ContribRequest) -> Hash32 {
    let request_id = hash_contrib_payload(request);
    request.request_id = request_id;
    request_id
}

pub fn sign_contrib_request(signer: &impl Signer, request: &mut ContribRequest) -> Hash32 {
    let request_id = set_contrib_request_id(request);
    request.request_sig = signer.sign(&request_id.0);
    request_id
}

pub fn sign_contrib_share(signer: &impl Signer, share: &mut ContribShare) -> Hash32 {
    let digest = hash_contrib_share(share);
    share.share_sig = signer.sign(&digest.0);
    digest
}

pub fn set_policy_id(policy: &mut PolicyToken) -> Hash32 {
    let policy_id = hash_policy_payload(policy);
    policy.policy_id = policy_id;
    policy_id
}

pub fn sign_policy_token(signer: &impl Signer, policy: &mut PolicyToken) -> Hash32 {
    let policy_id = set_policy_id(policy);
    policy.owner_sig = signer.sign(&policy_id.0);
    policy_id
}

pub fn sign_policy_cancel(signer: &impl Signer, cancel: &mut PolicyCancel) -> Hash32 {
    let digest = hash_policy_cancel(cancel);
    cancel.cancel_sig = signer.sign(&digest.0);
    digest
}

pub fn sign_envelope(signer: &impl Signer, envelope: &mut Envelope) -> Hash32 {
    let digest = hash_envelope(envelope);
    envelope.sig = signer.sign(&digest.0);
    digest
}

pub fn sign_timeout_claim(signer: &impl Signer, claim: &mut TimeoutClaim) -> Hash32 {
    let digest = hash_timeout_claim(claim);
    claim.claimer_sig = signer.sign(&digest.0);
    digest
}

pub fn hash_bytes(domain: &[u8], value: &impl CanonicalEncode) -> Hash32 {
    hash_with_domain(domain, &value.to_bytes())
}

pub fn hash_state_commitment(commitment: &StateCommitment) -> Hash32 {
    hash_bytes(DOMAIN_STATE_ROOT, commitment)
}

pub fn verify_action_propose(pubkey: PubKey, propose: &ActionPropose) -> bool {
    let expected = hash_action_payload(propose);
    if expected != propose.action_id {
        return false;
    }
    verify_signature(pubkey, &propose.action_id.0, propose.proposer_sig)
}

pub fn verify_action_ack(pubkey: PubKey, ack: &ActionAck) -> bool {
    verify_signature(pubkey, &ack.action_id.0, ack.ack_sig)
}

pub fn verify_action_reject(pubkey: PubKey, reject: &ActionReject) -> bool {
    let digest = hash_action_reject(reject);
    verify_signature(pubkey, &digest.0, reject.reject_sig)
}

pub fn verify_action_commit(pubkey: PubKey, commit: &ActionCommit) -> bool {
    let digest = hash_action_commit(commit);
    verify_signature(pubkey, &digest.0, commit.commit_sig)
}

pub fn verify_contrib_request(pubkey: PubKey, request: &ContribRequest) -> bool {
    let expected = hash_contrib_payload(request);
    if expected != request.request_id {
        return false;
    }
    verify_signature(pubkey, &request.request_id.0, request.request_sig)
}

pub fn verify_contrib_share(pubkey: PubKey, share: &ContribShare) -> bool {
    let digest = hash_contrib_share(share);
    verify_signature(pubkey, &digest.0, share.share_sig)
}

pub fn verify_policy_token(pubkey: PubKey, policy: &PolicyToken) -> bool {
    let expected = hash_policy_payload(policy);
    if expected != policy.policy_id {
        return false;
    }
    verify_signature(pubkey, &policy.policy_id.0, policy.owner_sig)
}

pub fn verify_policy_cancel(pubkey: PubKey, cancel: &PolicyCancel) -> bool {
    let digest = hash_policy_cancel(cancel);
    verify_signature(pubkey, &digest.0, cancel.cancel_sig)
}

pub fn verify_timeout_claim(pubkey: PubKey, claim: &TimeoutClaim) -> bool {
    let digest = hash_timeout_claim(claim);
    verify_signature(pubkey, &digest.0, claim.claimer_sig)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schnorr_sign_and_verify() {
        let signer = Secp256k1Signer::from_secret_bytes([42u8; 32]).expect("signer");
        let msg = [7u8; 32];
        let sig = signer.sign(&msg);
        let pubkey = signer.public_key();
        assert!(verify_signature(pubkey, &msg, sig));
    }
}
