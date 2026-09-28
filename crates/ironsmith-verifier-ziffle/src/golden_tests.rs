//! Byte-for-byte regression over every verifier operation. Proofs are
//! deterministic for fixed entropy, so any change to the shuffle proof system,
//! its serialization, or the JSON envelope changes the transcript digest and
//! would split peers running different builds.

use super::*;
use serde_json::{Value, json};

const GOLDEN_DECK_SIZES: [usize; 7] = [2, 3, 6, 10, 52, 60, 100];

fn dispatch(operation: Operation, input: &[u8]) -> Result<Vec<u8>, VerifierError> {
    let deck_count = input_deck_count(input)?;
    if !GOLDEN_DECK_SIZES.contains(&deck_count) {
        return Err(unsupported_deck_count(deck_count));
    }
    execute(operation, deck_count, input)
}

struct Transcript {
    hasher: Sha256,
}

impl Transcript {
    fn call(&mut self, operation: Operation, input: &Value) -> Value {
        let bytes = execute_with_input_chain(operation, &encode(input).unwrap(), dispatch)
            .unwrap_or_else(|error| panic!("{operation:?} failed: {error}"));
        self.record(&bytes);
        serde_json::from_slice(&bytes).unwrap()
    }

    fn keygen(&mut self, input: &Value) -> Value {
        let bytes = execute_keygen(&encode(input).unwrap()).unwrap();
        self.record(&bytes);
        serde_json::from_slice(&bytes).unwrap()
    }

    fn record(&mut self, bytes: &[u8]) {
        self.hasher.update((bytes.len() as u64).to_be_bytes());
        self.hasher.update(bytes);
    }
}

fn shuffle_all(transcript: &mut Transcript, mut input: Value, players: u8) -> Value {
    for shuffler in 0..players {
        input["shuffler"] = json!(shuffler);
        input["entropyHex"] = json!(format!(
            "{}{shuffler:02x}",
            sha256_hex(input["context"].as_str().unwrap().as_bytes())
        ));
        let step = transcript.call(Operation::BuildShuffleStep, &input);
        input["steps"].as_array_mut().unwrap().push(json!({
            "shuffler": step["shuffler"],
            "deckHex": step["deckHex"],
            "proofHex": step["proofHex"],
        }));
    }
    let object = input.as_object_mut().unwrap();
    object.remove("shuffler");
    object.remove("entropyHex");
    transcript.call(Operation::VerifyShuffle, &input);
    input
}

fn reveal(transcript: &mut Transcript, input: &Value, secrets: &[Value], positions: &[usize]) {
    let mut batch_tokens = Vec::new();
    let mut single_tokens = Vec::new();
    for (player, secret) in secrets.iter().enumerate() {
        let mut request = input.clone();
        request["publicKeyHex"] = secret["publicKeyHex"].clone();
        request["secretKeyHex"] = secret["secretKeyHex"].clone();
        request["entropyHex"] = json!(format!("deadbeef{player:02x}"));
        request["cardPositions"] = json!(positions);
        let tokens = transcript.call(Operation::BuildRevealTokens, &request);
        batch_tokens.extend(tokens.as_array().unwrap().iter().cloned());
        request["cardPosition"] = json!(positions[0]);
        single_tokens.push(transcript.call(Operation::BuildRevealToken, &request));
    }
    let mut request = input.clone();
    request["cardPositions"] = json!(positions);
    request["tokens"] = json!(batch_tokens);
    transcript.call(Operation::RevealCards, &request);
    let mut request = input.clone();
    request["cardPosition"] = json!(positions[0]);
    request["tokens"] = json!(single_tokens);
    transcript.call(Operation::RevealCard, &request);
}

#[test]
fn every_operation_reproduces_the_golden_transcript() {
    let mut transcript = Transcript {
        hasher: Sha256::new(),
    };
    for (index, deck_count) in GOLDEN_DECK_SIZES.into_iter().enumerate() {
        let players = 2 + (index % 3) as u8;
        let context = format!("golden-{deck_count}");
        let mut keys = Vec::new();
        let mut secrets = Vec::new();
        for player in 0..players {
            let generated = transcript.keygen(&json!({
                "deckCount": deck_count,
                "context": context,
                "entropyHex": format!("{deck_count:04x}c0ffee{player:02x}"),
            }));
            keys.push(json!({
                "player": player,
                "publicKeyHex": generated["publicKeyHex"],
                "ownershipProofHex": generated["ownershipProofHex"],
            }));
            secrets.push(generated);
        }
        let initial = shuffle_all(
            &mut transcript,
            json!({
                "deckCount": deck_count,
                "context": format!("{context}:initial"),
                "keyContext": context,
                "keys": keys,
                "steps": [],
            }),
            players,
        );
        let positions: Vec<usize> = (0..deck_count).step_by(deck_count.div_ceil(4)).collect();
        reveal(&mut transcript, &initial, &secrets, &positions);

        // A linked reshuffle epoch drawn from the verified initial deck
        // exercises the authenticated input-deck path at a second size.
        let shrunk = GOLDEN_DECK_SIZES[index.saturating_sub(1)].min(deck_count);
        let sources: Vec<Value> = (0..shrunk)
            .map(|offset| json!({"epoch": 0, "position": deck_count - 1 - offset}))
            .collect();
        let linked = shuffle_all(
            &mut transcript,
            json!({
                "deckCount": shrunk,
                "context": format!("{context}:linked"),
                "keyContext": context,
                "keys": initial["keys"],
                "steps": [],
                "inputDeck": {
                    "universeCount": deck_count,
                    "epochs": [{
                        "deckCount": deck_count,
                        "context": initial["context"],
                        "steps": initial["steps"],
                    }],
                    "sources": sources,
                },
            }),
            players,
        );
        reveal(&mut transcript, &linked, &secrets, &[0, shrunk - 1]);
    }
    let digest: String = transcript
        .hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        digest,
        "c959015410d1c6647f4cb86d70196cc408cc6aa60056ec1016757627aba87702"
    );
}
