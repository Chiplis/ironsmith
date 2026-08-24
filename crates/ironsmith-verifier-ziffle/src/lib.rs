//! Byte-oriented Ziffle proof and verification service.

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};
use ark_std::rand::{SeedableRng as ArkSeedableRng, rngs::StdRng as ArkStdRng};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use ziffle::{
    AggregatePublicKey, AggregateRevealToken, MaskedDeck, OwnershipProof, PublicKey, RevealToken,
    RevealTokenProof, SecretKey, Shuffle, ShuffleProof, Verified,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierError(String);

impl VerifierError {
    fn new(message: impl ToString) -> Self {
        Self(message.to_string())
    }
}

impl std::fmt::Display for VerifierError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for VerifierError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Keygen,
    BuildShuffleStep,
    VerifyShuffle,
    BuildRevealToken,
    BuildRevealTokens,
    RevealCard,
    RevealCards,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleEntropyInput {
    deck_count: usize,
    context: String,
    entropy_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleKeygenOutput {
    deck_count: usize,
    public_key_hex: String,
    secret_key_hex: String,
    ownership_proof_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZifflePublicKeyInput {
    player: u8,
    public_key_hex: String,
    ownership_proof_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleShuffleStepInput {
    shuffler: u8,
    deck_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleShuffleStepOutput {
    shuffler: u8,
    deck_hex: String,
    proof_hex: String,
    deck_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleBuildShuffleStepInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    shuffler: u8,
    entropy_hex: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleVerifyShuffleInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleVerifyShuffleOutput {
    deck_count: usize,
    deck_hex: String,
    deck_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleBuildRevealTokenInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    card_position: usize,
    public_key_hex: String,
    secret_key_hex: String,
    entropy_hex: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleBuildRevealTokensInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    card_positions: Vec<usize>,
    public_key_hex: String,
    secret_key_hex: String,
    entropy_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealTokenInput {
    player: u8,
    public_key_hex: String,
    token_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealTokenBatchInput {
    card_position: usize,
    player: u8,
    public_key_hex: String,
    token_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealTokenOutput {
    player: u8,
    public_key_hex: String,
    token_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealTokenBatchOutput {
    card_position: usize,
    player: u8,
    public_key_hex: String,
    token_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealCardInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    card_position: usize,
    tokens: Vec<ZiffleRevealTokenInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealCardsInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    card_positions: Vec<usize>,
    tokens: Vec<ZiffleRevealTokenBatchInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealCardOutput {
    card_position: usize,
    original_slot: usize,
}

fn decode<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T, VerifierError> {
    serde_json::from_slice(bytes)
        .map_err(|error| VerifierError::new(format!("invalid {label} input: {error}")))
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, VerifierError> {
    serde_json::to_vec(value)
        .map_err(|error| VerifierError::new(format!("failed to encode verifier output: {error}")))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeckCountInput {
    deck_count: usize,
}

/// Read only the routing field so the facade can choose a monomorphization shard.
pub fn input_deck_count(input: &[u8]) -> Result<usize, VerifierError> {
    decode::<DeckCountInput>(input, "ziffle operation").map(|input| input.deck_count)
}

/// Keep key generation in the common crate because its key types are independent
/// of the requested deck size.
pub fn execute_keygen(input: &[u8]) -> Result<Vec<u8>, VerifierError> {
    let input: ZiffleEntropyInput = decode(input, "ziffle keygen")?;
    let mut rng = rng_from_entropy_hex(&input.entropy_hex)?;
    let shuffle = Shuffle::<60>::default();
    let (secret_key, public_key, ownership_proof) =
        shuffle.keygen(&mut rng, input.context.as_bytes());
    encode(&ZiffleKeygenOutput {
        deck_count: input.deck_count,
        public_key_hex: ziffle_to_hex(&public_key)?,
        secret_key_hex: ziffle_to_hex(&secret_key)?,
        ownership_proof_hex: ziffle_to_hex(&ownership_proof)?,
    })
}

/// Execute one non-keygen operation for one concrete deck size. Concrete
/// instantiations live in sibling shard crates so rustc can codegen them in
/// parallel instead of placing all 594 instantiations in one unit.
pub fn execute_for<const N: usize>(
    operation: Operation,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match operation {
        Operation::Keygen => execute_keygen(input),
        Operation::BuildShuffleStep => {
            let input: ZiffleBuildShuffleStepInput = decode(input, "ziffle shuffle")?;
            ensure_deck_count::<N>(input.deck_count)?;
            let output = build_ziffle_shuffle_step::<N>(input)?;
            encode(&output)
        }
        Operation::VerifyShuffle => {
            let input: ZiffleVerifyShuffleInput = decode(input, "ziffle verify")?;
            ensure_deck_count::<N>(input.deck_count)?;
            let output = verify_ziffle_shuffle::<N>(input)?;
            encode(&output)
        }
        Operation::BuildRevealToken => {
            let input: ZiffleBuildRevealTokenInput = decode(input, "ziffle reveal token")?;
            ensure_deck_count::<N>(input.deck_count)?;
            let output = build_ziffle_reveal_token::<N>(input)?;
            encode(&output)
        }
        Operation::BuildRevealTokens => {
            let input: ZiffleBuildRevealTokensInput = decode(input, "ziffle reveal tokens")?;
            ensure_deck_count::<N>(input.deck_count)?;
            let output = build_ziffle_reveal_tokens::<N>(input)?;
            encode(&output)
        }
        Operation::RevealCard => {
            let input: ZiffleRevealCardInput = decode(input, "ziffle reveal")?;
            ensure_deck_count::<N>(input.deck_count)?;
            let output = reveal_ziffle_card::<N>(input)?;
            encode(&output)
        }
        Operation::RevealCards => {
            let input: ZiffleRevealCardsInput = decode(input, "ziffle reveals")?;
            ensure_deck_count::<N>(input.deck_count)?;
            let output = reveal_ziffle_cards::<N>(input)?;
            encode(&output)
        }
    }
}

fn ensure_deck_count<const N: usize>(deck_count: usize) -> Result<(), VerifierError> {
    if deck_count == N {
        Ok(())
    } else {
        Err(VerifierError::new(format!(
            "verifier shard mismatch: routed deck size {deck_count} to {N}"
        )))
    }
}

fn ziffle_to_hex<T: CanonicalSerialize>(value: &T) -> Result<String, VerifierError> {
    let mut bytes = Vec::new();
    value
        .serialize_compressed(&mut bytes)
        .map_err(|e| VerifierError::new(format!("failed to serialize ziffle artifact: {e}")))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn ziffle_from_hex<T: CanonicalDeserialize>(hex: &str, label: &str) -> Result<T, VerifierError> {
    let bytes = hex_to_vec(hex).map_err(|e| VerifierError::new(format!("invalid {label}: {e}")))?;
    T::deserialize_with_mode(bytes.as_slice(), Compress::Yes, Validate::Yes)
        .map_err(|e| VerifierError::new(format!("failed to decode {label}: {e}")))
}

fn hex_to_vec(hex: &str) -> Result<Vec<u8>, String> {
    let normalized = hex.trim();
    if !normalized.len().is_multiple_of(2) {
        return Err("hex string has odd length".to_string());
    }
    let mut out = Vec::with_capacity(normalized.len() / 2);
    let bytes = normalized.as_bytes();
    for index in (0..bytes.len()).step_by(2) {
        let pair = std::str::from_utf8(&bytes[index..index + 2])
            .map_err(|_| "hex string is not utf-8".to_string())?;
        let byte = u8::from_str_radix(pair, 16)
            .map_err(|_| format!("invalid hex byte at offset {index}"))?;
        out.push(byte);
    }
    Ok(out)
}

fn rng_from_entropy_hex(hex: &str) -> Result<ArkStdRng, VerifierError> {
    let bytes = hex_to_vec(hex).map_err(|e| VerifierError::new(format!("invalid entropy: {e}")))?;
    if bytes.is_empty() {
        return Err(VerifierError::new("ziffle entropy cannot be empty"));
    }
    let digest = Sha256::digest(&bytes);
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&digest);
    Ok(ArkStdRng::from_seed(seed))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn ziffle_deck_hash(deck_hex: &str) -> Result<String, VerifierError> {
    let bytes = hex_to_vec(deck_hex)
        .map_err(|e| VerifierError::new(format!("invalid ziffle deck hex: {e}")))?;
    Ok(sha256_hex(&bytes))
}

fn verified_public_keys(
    keys: &[ZifflePublicKeyInput],
    context: &[u8],
) -> Result<Vec<(u8, PublicKey, Verified<PublicKey>)>, VerifierError> {
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        let public_key: PublicKey = ziffle_from_hex(&key.public_key_hex, "ziffle public key")?;
        let proof: OwnershipProof =
            ziffle_from_hex(&key.ownership_proof_hex, "ziffle ownership proof")?;
        let verified = proof.verify(public_key, context).ok_or_else(|| {
            VerifierError::new(format!(
                "ziffle ownership proof failed for player {}",
                key.player
            ))
        })?;
        out.push((key.player, public_key, verified));
    }
    out.sort_by_key(|(player, _, _)| *player);
    Ok(out)
}

type VerifiedPublicKeys = Vec<(u8, PublicKey, Verified<PublicKey>)>;

fn aggregate_public_key(
    keys: &[ZifflePublicKeyInput],
    context: &[u8],
) -> Result<(VerifiedPublicKeys, AggregatePublicKey), VerifierError> {
    let verified = verified_public_keys(keys, context)?;
    if verified.is_empty() {
        return Err(VerifierError::new(
            "ziffle requires at least one public key",
        ));
    }
    let public_keys = verified
        .iter()
        .map(|(_, _, verified)| *verified)
        .collect::<Vec<_>>();
    Ok((verified, AggregatePublicKey::new(&public_keys)))
}

fn ziffle_key_context<'a>(key_context: &'a str, context: &'a str) -> &'a str {
    let key_context = key_context.trim();
    if key_context.is_empty() {
        context
    } else {
        key_context
    }
}

fn verify_ziffle_steps<const N: usize>(
    context: &[u8],
    key_context: &[u8],
    keys: &[ZifflePublicKeyInput],
    steps: &[ZiffleShuffleStepInput],
) -> Result<Option<Verified<MaskedDeck<N>>>, VerifierError> {
    let shuffle = Shuffle::<N>::default();
    let (verified_keys, aggregate) = aggregate_public_key(keys, key_context)?;
    let mut verified_deck = None;
    for (index, step) in steps.iter().enumerate() {
        let expected_shuffler = verified_keys
            .get(index)
            .map(|(player, _, _)| *player)
            .ok_or_else(|| VerifierError::new("ziffle shuffle has more steps than player keys"))?;
        if step.shuffler != expected_shuffler {
            return Err(VerifierError::new(format!(
                "ziffle shuffle step {index} was attributed to player {}, expected player {expected_shuffler}",
                step.shuffler
            )));
        }
        let deck: MaskedDeck<N> = ziffle_from_hex(&step.deck_hex, "ziffle masked deck")?;
        let proof: ShuffleProof<N> = ziffle_from_hex(&step.proof_hex, "ziffle shuffle proof")?;
        verified_deck = Some(if index == 0 {
            shuffle
                .verify_initial_shuffle(aggregate, deck, proof, context)
                .ok_or_else(|| VerifierError::new("ziffle initial shuffle proof failed"))?
        } else {
            let previous = verified_deck
                .as_ref()
                .ok_or_else(|| VerifierError::new("ziffle previous deck is missing"))?;
            shuffle
                .verify_shuffle(aggregate, previous, deck, proof, context)
                .ok_or_else(|| VerifierError::new(format!("ziffle shuffle proof {index} failed")))?
        });
    }
    Ok(verified_deck)
}

fn build_ziffle_shuffle_step<const N: usize>(
    input: ZiffleBuildShuffleStepInput,
) -> Result<ZiffleShuffleStepOutput, VerifierError> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let shuffle = Shuffle::<N>::default();
    let (verified_keys, aggregate) = aggregate_public_key(&input.keys, key_context)?;
    let expected_shuffler = verified_keys
        .get(input.steps.len())
        .map(|(player, _, _)| *player)
        .ok_or_else(|| {
            VerifierError::new("ziffle shuffle already has one step for every player")
        })?;
    if input.shuffler != expected_shuffler {
        return Err(VerifierError::new(format!(
            "ziffle shuffle step must be built by player {expected_shuffler}"
        )));
    }
    let previous = verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?;
    let mut rng = rng_from_entropy_hex(&input.entropy_hex)?;
    let (deck, proof) = if let Some(previous) = previous.as_ref() {
        shuffle.shuffle_deck(&mut rng, aggregate, previous, context)
    } else {
        shuffle.shuffle_initial_deck(&mut rng, aggregate, context)
    };
    let deck_hex = ziffle_to_hex(&deck)?;
    Ok(ZiffleShuffleStepOutput {
        shuffler: input.shuffler,
        proof_hex: ziffle_to_hex(&proof)?,
        deck_hash: ziffle_deck_hash(&deck_hex)?,
        deck_hex,
    })
}

fn verify_ziffle_shuffle<const N: usize>(
    input: ZiffleVerifyShuffleInput,
) -> Result<ZiffleVerifyShuffleOutput, VerifierError> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    if input.steps.len() != input.keys.len() {
        return Err(VerifierError::new(
            "ziffle final shuffle must include one step per player",
        ));
    }
    verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| VerifierError::new("ziffle ceremony has no shuffle steps"))?;
    let deck_hex = input
        .steps
        .last()
        .map(|step| step.deck_hex.clone())
        .ok_or_else(|| VerifierError::new("ziffle ceremony has no shuffle steps"))?;
    Ok(ZiffleVerifyShuffleOutput {
        deck_count: N,
        deck_hash: ziffle_deck_hash(&deck_hex)?,
        deck_hex,
    })
}

fn build_ziffle_reveal_token<const N: usize>(
    input: ZiffleBuildRevealTokenInput,
) -> Result<ZiffleRevealTokenOutput, VerifierError> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let verified_deck = verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| VerifierError::new("ziffle ceremony has no shuffle steps"))?;
    let card = verified_deck
        .get(input.card_position)
        .ok_or_else(|| VerifierError::new("ziffle card position is out of range"))?;
    let secret_key: SecretKey = ziffle_from_hex(&input.secret_key_hex, "ziffle secret key")?;
    let public_key: PublicKey = ziffle_from_hex(&input.public_key_hex, "ziffle public key")?;
    let mut rng = rng_from_entropy_hex(&input.entropy_hex)?;
    let (token, proof) = card.reveal_token(&mut rng, &secret_key, public_key, context);
    Ok(ZiffleRevealTokenOutput {
        player: input
            .keys
            .iter()
            .find(|key| key.public_key_hex == input.public_key_hex)
            .map(|key| key.player)
            .unwrap_or(0),
        public_key_hex: input.public_key_hex,
        token_hex: ziffle_to_hex(&token)?,
        proof_hex: ziffle_to_hex(&proof)?,
    })
}

fn build_ziffle_reveal_tokens<const N: usize>(
    input: ZiffleBuildRevealTokensInput,
) -> Result<Vec<ZiffleRevealTokenBatchOutput>, VerifierError> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let verified_deck = verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| VerifierError::new("ziffle ceremony has no shuffle steps"))?;
    let secret_key: SecretKey = ziffle_from_hex(&input.secret_key_hex, "ziffle secret key")?;
    let public_key: PublicKey = ziffle_from_hex(&input.public_key_hex, "ziffle public key")?;
    let player = input
        .keys
        .iter()
        .find(|key| key.public_key_hex == input.public_key_hex)
        .map(|key| key.player)
        .unwrap_or(0);
    let mut rng = rng_from_entropy_hex(&input.entropy_hex)?;
    let mut out = Vec::with_capacity(input.card_positions.len());
    for card_position in input.card_positions {
        let card = verified_deck
            .get(card_position)
            .ok_or_else(|| VerifierError::new("ziffle card position is out of range"))?;
        let (token, proof) = card.reveal_token(&mut rng, &secret_key, public_key, context);
        out.push(ZiffleRevealTokenBatchOutput {
            card_position,
            player,
            public_key_hex: input.public_key_hex.clone(),
            token_hex: ziffle_to_hex(&token)?,
            proof_hex: ziffle_to_hex(&proof)?,
        });
    }
    Ok(out)
}

fn reveal_ziffle_card<const N: usize>(
    input: ZiffleRevealCardInput,
) -> Result<ZiffleRevealCardOutput, VerifierError> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let (keys, _) = aggregate_public_key(&input.keys, key_context)?;
    let verified_deck = verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| VerifierError::new("ziffle ceremony has no shuffle steps"))?;
    let card = verified_deck
        .get(input.card_position)
        .ok_or_else(|| VerifierError::new("ziffle card position is out of range"))?;
    let mut verified_tokens = Vec::with_capacity(keys.len());
    for (player, _, verified_public_key) in keys {
        let token_input = input
            .tokens
            .iter()
            .find(|token| token.player == player)
            .ok_or_else(|| {
                VerifierError::new(format!("missing ziffle reveal token for player {player}"))
            })?;
        let token: RevealToken = ziffle_from_hex(&token_input.token_hex, "ziffle reveal token")?;
        let proof: RevealTokenProof =
            ziffle_from_hex(&token_input.proof_hex, "ziffle reveal-token proof")?;
        verified_tokens.push(
            proof
                .verify(verified_public_key, token, card, context)
                .ok_or_else(|| {
                    VerifierError::new(format!(
                        "ziffle reveal-token proof failed for player {player}"
                    ))
                })?,
        );
    }
    let aggregate = AggregateRevealToken::new(&verified_tokens);
    let shuffle = Shuffle::<N>::default();
    let original_slot = shuffle
        .reveal_card(aggregate, card)
        .ok_or_else(|| VerifierError::new("ziffle aggregate reveal failed"))?;
    Ok(ZiffleRevealCardOutput {
        card_position: input.card_position,
        original_slot,
    })
}

fn reveal_ziffle_cards<const N: usize>(
    input: ZiffleRevealCardsInput,
) -> Result<Vec<ZiffleRevealCardOutput>, VerifierError> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let (keys, _) = aggregate_public_key(&input.keys, key_context)?;
    let verified_deck = verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| VerifierError::new("ziffle ceremony has no shuffle steps"))?;
    let shuffle = Shuffle::<N>::default();
    let mut out = Vec::with_capacity(input.card_positions.len());
    for card_position in input.card_positions {
        let card = verified_deck
            .get(card_position)
            .ok_or_else(|| VerifierError::new("ziffle card position is out of range"))?;
        let mut verified_tokens = Vec::with_capacity(keys.len());
        for (player, _, verified_public_key) in &keys {
            let token_input = input
                .tokens
                .iter()
                .find(|token| token.player == *player && token.card_position == card_position)
                .ok_or_else(|| {
                    VerifierError::new(format!(
                        "missing ziffle reveal token for player {player} at position {card_position}"
                    ))
                })?;
            let token: RevealToken =
                ziffle_from_hex(&token_input.token_hex, "ziffle reveal token")?;
            let proof: RevealTokenProof =
                ziffle_from_hex(&token_input.proof_hex, "ziffle reveal-token proof")?;
            verified_tokens.push(
                proof
                    .verify(*verified_public_key, token, card, context)
                    .ok_or_else(|| {
                        VerifierError::new(format!(
                            "ziffle reveal-token proof failed for player {player}"
                        ))
                    })?,
            );
        }
        let aggregate = AggregateRevealToken::new(&verified_tokens);
        let original_slot = shuffle
            .reveal_card(aggregate, card)
            .ok_or_else(|| VerifierError::new("ziffle aggregate reveal failed"))?;
        out.push(ZiffleRevealCardOutput {
            card_position,
            original_slot,
        });
    }
    Ok(out)
}

pub fn unsupported_deck_count(deck_count: usize) -> VerifierError {
    VerifierError::new(format!(
        "unsupported ziffle deck size {deck_count}; supported sizes are 2 through 100"
    ))
}

#[cfg(test)]
mod ziffle_backend_tests {
    use super::*;

    #[test]
    fn ziffle_helpers_shuffle_and_reveal_with_four_players() {
        let context = "ironsmith-wasm-ziffle-test".to_string();
        let shuffle = Shuffle::<10>::default();
        let mut key_rng =
            rng_from_entropy_hex("00112233445566778899aabbccddeeff").expect("key rng should build");
        let mut keys = Vec::new();
        let mut secrets = Vec::new();
        for player in 0..4u8 {
            let (secret_key, public_key, ownership_proof) =
                shuffle.keygen(&mut key_rng, context.as_bytes());
            keys.push(ZifflePublicKeyInput {
                player,
                public_key_hex: ziffle_to_hex(&public_key).expect("public key hex"),
                ownership_proof_hex: ziffle_to_hex(&ownership_proof).expect("proof hex"),
            });
            secrets.push((
                player,
                ziffle_to_hex(&secret_key).expect("secret key hex"),
                ziffle_to_hex(&public_key).expect("public key hex"),
            ));
        }

        let mut steps = Vec::new();
        for player in 0..4u8 {
            let step = build_ziffle_shuffle_step::<10>(ZiffleBuildShuffleStepInput {
                deck_count: 10,
                context: context.clone(),
                key_context: String::new(),
                keys: keys.clone(),
                steps: steps.clone(),
                shuffler: player,
                entropy_hex: format!("feedfacecafebeef{player:02x}"),
            })
            .expect("shuffle step should build");
            steps.push(ZiffleShuffleStepInput {
                shuffler: player,
                deck_hex: step.deck_hex,
                proof_hex: step.proof_hex,
            });
        }
        let verified = verify_ziffle_shuffle::<10>(ZiffleVerifyShuffleInput {
            deck_count: 10,
            context: context.clone(),
            key_context: String::new(),
            keys: keys.clone(),
            steps: steps.clone(),
        })
        .expect("shuffle should verify");
        assert!(!verified.deck_hash.is_empty());

        let mut tokens = Vec::new();
        for (player, secret_key_hex, public_key_hex) in secrets {
            let token = build_ziffle_reveal_token::<10>(ZiffleBuildRevealTokenInput {
                deck_count: 10,
                context: context.clone(),
                key_context: String::new(),
                keys: keys.clone(),
                steps: steps.clone(),
                card_position: 0,
                public_key_hex,
                secret_key_hex,
                entropy_hex: format!("decafbadbeadfeed{player:02x}"),
            })
            .expect("reveal token should build");
            tokens.push(ZiffleRevealTokenInput {
                player,
                public_key_hex: token.public_key_hex,
                token_hex: token.token_hex,
                proof_hex: token.proof_hex,
            });
        }
        let reveal = reveal_ziffle_card::<10>(ZiffleRevealCardInput {
            deck_count: 10,
            context,
            key_context: String::new(),
            keys,
            steps,
            card_position: 0,
            tokens,
        })
        .expect("card should reveal");
        assert!(reveal.original_slot < 10);
    }

    #[test]
    fn ziffle_helpers_allow_action_shuffle_context_with_match_keys() {
        let key_context = "ironsmith-wasm-ziffle-match".to_string();
        let shuffle_context = "ironsmith-wasm-ziffle-match:action:7:shuffle:p0".to_string();
        let shuffle = Shuffle::<10>::default();
        let mut key_rng =
            rng_from_entropy_hex("abcdef00112233445566778899").expect("key rng should build");
        let mut keys = Vec::new();
        for player in 0..2u8 {
            let (_, public_key, ownership_proof) =
                shuffle.keygen(&mut key_rng, key_context.as_bytes());
            keys.push(ZifflePublicKeyInput {
                player,
                public_key_hex: ziffle_to_hex(&public_key).expect("public key hex"),
                ownership_proof_hex: ziffle_to_hex(&ownership_proof).expect("proof hex"),
            });
        }

        let mut steps = Vec::new();
        for shuffler in 0..2u8 {
            let step = build_ziffle_shuffle_step::<10>(ZiffleBuildShuffleStepInput {
                deck_count: 10,
                context: shuffle_context.clone(),
                key_context: key_context.clone(),
                keys: keys.clone(),
                steps: steps.clone(),
                shuffler,
                entropy_hex: format!("feedface0002{shuffler:02x}"),
            })
            .expect("shuffle step should verify ownership under key context");
            steps.push(ZiffleShuffleStepInput {
                shuffler: step.shuffler,
                deck_hex: step.deck_hex,
                proof_hex: step.proof_hex,
            });
        }
        let verified = verify_ziffle_shuffle::<10>(ZiffleVerifyShuffleInput {
            deck_count: 10,
            context: shuffle_context,
            key_context,
            keys,
            steps,
        })
        .expect("shuffle should verify with distinct key and shuffle contexts");
        assert!(!verified.deck_hash.is_empty());
    }
}
