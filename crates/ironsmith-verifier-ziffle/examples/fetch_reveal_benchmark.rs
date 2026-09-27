//! Run with `cargo run -p ironsmith-verifier-ziffle --release --example fetch_reveal_benchmark`.
//! Measures the public byte API for a two-player, 60-card library search.

use ironsmith_verifier_ziffle::{Operation, execute_for, execute_keygen};
use serde_json::{Value, json};
use std::time::Instant;

fn timed(label: &str, operation: Operation, input: &Value) -> Value {
    let bytes = serde_json::to_vec(input).unwrap();
    let start = Instant::now();
    let result = if operation == Operation::Keygen {
        execute_keygen(&bytes)
    } else {
        execute_for::<60>(operation, &bytes)
    }
    .unwrap();
    println!("{label}: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    serde_json::from_slice(&result).unwrap()
}

fn main() {
    let context = "fetch-reveal-benchmark";
    let identities: Vec<Value> = (0..2)
        .map(|player| {
            timed(
                &format!("keygen_{player}"),
                Operation::Keygen,
                &json!({"deckCount":60,"context":context,"entropyHex":format!("00{player:02x}")}),
            )
        })
        .collect();
    let keys: Vec<Value> = identities.iter().enumerate().map(|(player, key)| json!({
        "player":player, "publicKeyHex":key["publicKeyHex"], "ownershipProofHex":key["ownershipProofHex"]
    })).collect();
    let mut ceremony = json!({"deckCount":60,"context":context,"keys":keys,"steps":[]});
    for player in 0..2 {
        let mut input = ceremony.clone();
        input["shuffler"] = json!(player);
        input["entropyHex"] = json!(format!("aabb{player:02x}"));
        let step = timed(
            &format!("shuffle_step_{player}"),
            Operation::BuildShuffleStep,
            &input,
        );
        ceremony["steps"].as_array_mut().unwrap().push(step);
    }
    for run in 0..3 {
        timed(
            &format!("verify_shuffle_{run}"),
            Operation::VerifyShuffle,
            &ceremony,
        );
    }
    let positions: Vec<usize> = (0..50).collect();
    let mut tokens = Vec::new();
    for (player, identity) in identities.iter().enumerate() {
        let mut input = ceremony.clone();
        input["cardPositions"] = json!(positions);
        input["publicKeyHex"] = identity["publicKeyHex"].clone();
        input["secretKeyHex"] = identity["secretKeyHex"].clone();
        input["entropyHex"] = json!(format!("ccdd{player:02x}"));
        if player == 0 {
            let mut single = input.clone();
            single["cardPositions"] = json!([0]);
            timed("build_tokens_1", Operation::BuildRevealTokens, &single);
        }
        let result = timed(
            &format!("build_tokens_50_player{player}"),
            Operation::BuildRevealTokens,
            &input,
        );
        tokens.extend(result.as_array().unwrap().iter().cloned());
    }
    let mut input = ceremony;
    input["tokens"] = json!(tokens);
    input["cardPositions"] = json!([0]);
    for run in 0..3 {
        timed(&format!("reveal_1_{run}"), Operation::RevealCards, &input);
    }
    input["cardPositions"] = json!(positions);
    let revealed = timed("reveal_50", Operation::RevealCards, &input);
    assert_eq!(revealed.as_array().unwrap().len(), 50);
}
