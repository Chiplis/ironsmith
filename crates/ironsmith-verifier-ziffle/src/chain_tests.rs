use super::*;
use serde_json::{Value, json};

fn dispatch(operation: Operation, input: &[u8]) -> Result<Vec<u8>, VerifierError> {
    execute(operation, input_deck_count(input)?, input)
}

fn call(operation: Operation, input: &Value) -> Result<Value, VerifierError> {
    let bytes = execute_with_input_chain(operation, &encode(input)?, dispatch)?;
    serde_json::from_slice(&bytes).map_err(VerifierError::new)
}

struct Fixture {
    initial: Value,
    secrets: Vec<Value>,
}

fn fixture() -> Fixture {
    let context = "private-chain-match";
    let mut keys = Vec::new();
    let mut secrets = Vec::new();
    for player in 0..2 {
        let generated: Value = serde_json::from_slice(
            &execute_keygen(
                &encode(&json!({
                    "deckCount": 6, "context": context, "entropyHex":format!("abcdef00{player:02x}")
                }))
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        keys.push(json!({"player":player,"publicKeyHex":generated["publicKeyHex"],"ownershipProofHex":generated["ownershipProofHex"]}));
        secrets.push(generated);
    }
    let initial = finish(json!({
        "deckCount":6,"context":"private-chain-match:owner0:initial","keyContext":context,"keys":keys,"steps":[]
    }));
    Fixture { initial, secrets }
}

fn finish(mut input: Value) -> Value {
    for shuffler in 0..2 {
        input["shuffler"] = json!(shuffler);
        input["entropyHex"] = json!(format!(
            "{}{shuffler:02x}",
            sha256_hex(input["context"].as_str().unwrap().as_bytes())
        ));
        let step = call(Operation::BuildShuffleStep, &input).unwrap();
        input["steps"].as_array_mut().unwrap().push(step);
    }
    call(Operation::VerifyShuffle, &input).unwrap();
    input.as_object_mut().unwrap().remove("shuffler");
    input.as_object_mut().unwrap().remove("entropyHex");
    input
}

fn next_input(previous: &Value, sources: Vec<Value>, context: &str) -> Value {
    let mut epochs = previous["inputDeck"]["epochs"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut parent = json!({"deckCount":previous["deckCount"],"context":previous["context"],"steps":previous["steps"]});
    if previous.get("inputDeck").is_some() {
        parent["sources"] = previous["inputDeck"]["sources"].clone();
    }
    epochs.push(parent);
    json!({
        "deckCount":sources.len(),"context":context,"keyContext":previous["keyContext"],"keys":previous["keys"],"steps":[],
        "inputDeck":{"universeCount":6,"epochs":epochs,"sources":sources}
    })
}

fn sources(epoch: usize, positions: &[usize]) -> Vec<Value> {
    positions
        .iter()
        .map(|position| json!({"epoch":epoch,"position":position}))
        .collect()
}

fn reveal_all(input: &Value, fixture: &Fixture) -> Vec<usize> {
    let count = input["deckCount"].as_u64().unwrap() as usize;
    let positions: Vec<_> = (0..count).collect();
    let mut tokens = Vec::new();
    for player in 0..2 {
        let mut request = input.clone();
        request["cardPositions"] = json!(positions);
        request["publicKeyHex"] = fixture.secrets[player]["publicKeyHex"].clone();
        request["secretKeyHex"] = fixture.secrets[player]["secretKeyHex"].clone();
        request["entropyHex"] = json!(format!("deadbeef{player:02x}"));
        tokens.extend(
            call(Operation::BuildRevealTokens, &request)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .cloned(),
        );
    }
    let mut request = input.clone();
    request["cardPositions"] = json!(positions);
    request["tokens"] = json!(tokens);
    call(Operation::RevealCards, &request)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|card| card["originalSlot"].as_u64().unwrap() as usize)
        .collect()
}

#[test]
fn authenticated_reshuffles_shrink_grow_and_reveal_original_manifest_slots() {
    let fixture = fixture();
    let initial = reveal_all(&fixture.initial, &fixture);
    let first = finish(next_input(
        &fixture.initial,
        sources(0, &[2, 3, 4, 5]),
        "private-chain-match:first",
    ));
    let first_slots = reveal_all(&first, &fixture);
    let second = finish(next_input(
        &first,
        sources(1, &[1, 2, 3]),
        "private-chain-match:second",
    ));
    let second_slots = reveal_all(&second, &fixture);
    let mut selected = sources(2, &[0, 1, 2]);
    selected.extend(sources(0, &[0]));
    selected.extend(sources(1, &[0]));
    let grown = finish(next_input(
        &second,
        selected,
        "private-chain-match:return-two",
    ));
    let mut actual = reveal_all(&grown, &fixture);
    let mut expected = second_slots;
    expected.extend([initial[0], first_slots[0]]);
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(
        actual, expected,
        "returns preserve physical manifest labels through multiple ciphertext epochs"
    );
    assert_eq!(actual.len(), 5);
    let mut expected_first = initial[2..].to_vec();
    let mut actual_first = first_slots;
    expected_first.sort_unstable();
    actual_first.sort_unstable();
    assert_eq!(actual_first, expected_first);
    assert!(
        actual_first.iter().any(|slot| *slot >= 4),
        "shrunk deck must reveal original slots outside its current size"
    );
    let genesis = call(Operation::VerifyShuffle, &fixture.initial).unwrap();
    let verification = call(Operation::VerifyShuffle, &grown).unwrap();
    assert_eq!(verification["universeCount"], 6);
    assert_eq!(verification["rootDeckHash"], genesis["deckHash"]);
    assert_eq!(verification["rootContext"], fixture.initial["context"]);
    assert_ne!(verification["stateHash"], genesis["stateHash"]);
    for input in [&first, &second, &grown] {
        let output = call(Operation::VerifyShuffle, input).unwrap();
        let text = serde_json::to_string(&output).unwrap();
        for forbidden in ["beforeOrder", "afterOrder", "permutation", "originalSlot"] {
            assert!(
                !text.contains(forbidden),
                "verification must not return input/output identity mapping"
            );
        }
    }
}

#[test]
fn authenticated_reshuffle_rejects_missing_substituted_duplicate_and_tampered_parents() {
    let fixture = fixture();
    let first = finish(next_input(
        &fixture.initial,
        sources(0, &[2, 3, 4, 5]),
        "private-chain-match:first",
    ));
    let second = finish(next_input(
        &first,
        sources(1, &[1, 2, 3]),
        "private-chain-match:second",
    ));
    // Warm caches with the valid graph. Every adversarial variant still needs
    // to authenticate its exact transcript and ciphertext consumption ledger.
    call(Operation::VerifyShuffle, &second).unwrap();
    let mut cases = Vec::new();
    let mut add = |label, mutate: fn(&mut Value)| {
        let mut bad = second.clone();
        mutate(&mut bad);
        cases.push((label, bad));
    };
    add("missing parent", |bad| {
        bad["inputDeck"]["epochs"].as_array_mut().unwrap().pop();
    });
    add("duplicate source", |bad| {
        bad["inputDeck"]["sources"][1] = bad["inputDeck"]["sources"][0].clone();
    });
    add("substituted source", |bad| {
        bad["inputDeck"]["sources"][0] = json!({"epoch":0,"position":0});
    });
    add("consumed source", |bad| {
        bad["inputDeck"]["sources"][0] = json!({"epoch":0,"position":2});
    });
    add("out of range source", |bad| {
        bad["inputDeck"]["sources"][0]["position"] = json!(99);
    });
    add("wrong universe", |bad| {
        bad["inputDeck"]["universeCount"] = json!(5);
    });
    add("wrong current context", |bad| {
        bad["context"] = json!("different-match:second");
    });
    add("wrong match key context", |bad| {
        bad["keyContext"] = json!("different-match");
    });
    add("wrong root context", |bad| {
        bad["inputDeck"]["epochs"][0]["context"] = json!("different-root");
    });
    add("parent missing last player", |bad| {
        bad["inputDeck"]["epochs"][0]["steps"]
            .as_array_mut()
            .unwrap()
            .pop();
    });
    add("parent forward reference", |bad| {
        bad["inputDeck"]["epochs"][1]["sources"][0]["epoch"] = json!(1);
    });
    add("duplicate key seat", |bad| {
        bad["keys"][1]["player"] = json!(0);
    });
    add("duplicate public key", |bad| {
        bad["keys"][1]["publicKeyHex"] = bad["keys"][0]["publicKeyHex"].clone();
    });
    add("untrusted raw deck", |bad| {
        bad["inputDeck"]["deckHex"] = json!("deadbeef");
    });
    add("tampered parent proof", |bad| {
        let value = &mut bad["inputDeck"]["epochs"][0]["steps"][0]["proofHex"];
        let mut hex = value.as_str().unwrap().to_string();
        hex.replace_range(..2, if hex.starts_with("00") { "01" } else { "00" });
        *value = json!(hex);
    });
    add("tampered parent ciphertext", |bad| {
        let value = &mut bad["inputDeck"]["epochs"][0]["steps"][1]["deckHex"];
        let mut hex = value.as_str().unwrap().to_string();
        hex.replace_range(..2, if hex.starts_with("00") { "01" } else { "00" });
        *value = json!(hex);
    });
    add("trailing ciphertext bytes", |bad| {
        let value = &mut bad["inputDeck"]["epochs"][0]["steps"][1]["deckHex"];
        *value = json!(format!("{}00", value.as_str().unwrap()));
    });
    for (label, bad) in cases {
        assert!(
            call(Operation::VerifyShuffle, &bad).is_err(),
            "accepted {label}"
        );
    }
    call(Operation::VerifyShuffle, &second).unwrap();
    assert!(
        execute(Operation::VerifyShuffle, 3, &encode(&second).unwrap()).is_err(),
        "bypassing the graph dispatcher cannot mint trusted source handles"
    );
}

#[test]
fn linked_reveal_tokens_and_valid_parent_proofs_cannot_be_transplanted() {
    let fixture = fixture();
    let first = finish(next_input(
        &fixture.initial,
        sources(0, &[2, 3, 4, 5]),
        "private-chain-match:single",
    ));
    let expected = reveal_all(&first, &fixture)[0];
    let mut reveal = first.clone();
    reveal["cardPosition"] = json!(0);
    let mut tokens = Vec::new();
    for player in 0..2 {
        let mut request = reveal.clone();
        request["publicKeyHex"] = fixture.secrets[player]["publicKeyHex"].clone();
        request["secretKeyHex"] = fixture.secrets[player]["secretKeyHex"].clone();
        request["entropyHex"] = json!(format!("ffff{player:02x}"));
        tokens.push(call(Operation::BuildRevealToken, &request).unwrap());
    }
    reveal["tokens"] = json!(tokens);
    let opened = call(Operation::RevealCard, &reveal).unwrap();
    assert_eq!(opened["originalSlot"], expected);
    let mut changed_position = reveal.clone();
    changed_position["cardPosition"] = json!(1);
    assert!(call(Operation::RevealCard, &changed_position).is_err());
    let mut changed_context = reveal.clone();
    changed_context["context"] = json!("private-chain-match:other-action");
    assert!(call(Operation::RevealCard, &changed_context).is_err());
    let mut substituted = reveal.clone();
    substituted["inputDeck"]["sources"][0] = json!({"epoch":0,"position":0});
    assert!(call(Operation::RevealCard, &substituted).is_err());

    let mut alternative_root = fixture.initial.clone();
    alternative_root["context"] = json!("private-chain-match:alternate-root");
    alternative_root["steps"] = json!([]);
    let alternative_root = finish(alternative_root);
    let mut transplanted = first.clone();
    transplanted["inputDeck"]["epochs"][0] = json!({
        "deckCount":6,"context":alternative_root["context"],"steps":alternative_root["steps"]
    });
    assert!(
        call(Operation::VerifyShuffle, &transplanted).is_err(),
        "individually valid parent proofs cannot substitute the graph under an existing shuffle"
    );
    let legitimate_other_history = finish(next_input(
        &alternative_root,
        sources(0, &[2, 3, 4, 5]),
        "private-chain-match:single",
    ));
    let original_verification = call(Operation::VerifyShuffle, &first).unwrap();
    let alternative_verification =
        call(Operation::VerifyShuffle, &legitimate_other_history).unwrap();
    assert_ne!(
        original_verification["rootDeckHash"], alternative_verification["rootDeckHash"],
        "the application receives the genesis identity needed to reject a re-proven unauthorized history"
    );
    assert_ne!(
        original_verification["stateHash"],
        alternative_verification["stateHash"]
    );
}

#[test]
fn appended_graph_reuses_only_its_exact_authenticated_prefix() {
    let fixture = fixture();
    let first = finish(next_input(
        &fixture.initial,
        sources(0, &[2, 3, 4, 5]),
        "private-chain-match:prefix-one",
    ));
    let mut next = next_input(
        &first,
        sources(1, &[1, 2, 3]),
        "private-chain-match:prefix-two",
    );
    next["shuffler"] = json!(0);
    next["entropyHex"] = json!("1234abcd");
    let calls = RefCell::new(Vec::new());
    execute_with_input_chain(
        Operation::BuildShuffleStep,
        &encode(&next).unwrap(),
        |operation, bytes| {
            calls
                .borrow_mut()
                .push((operation, input_deck_count(bytes)?));
            dispatch(operation, bytes)
        },
    )
    .unwrap();
    assert_eq!(
        *calls.borrow(),
        vec![
            (Operation::VerifyShuffle, 4),
            (Operation::BuildShuffleStep, 3)
        ],
        "only the newly appended completed epoch needs proof verification"
    );
    next["inputDeck"]["epochs"][0]["context"] = json!("tampered-cached-prefix");
    calls.borrow_mut().clear();
    assert!(
        execute_with_input_chain(
            Operation::BuildShuffleStep,
            &encode(&next).unwrap(),
            |operation, bytes| {
                calls
                    .borrow_mut()
                    .push((operation, input_deck_count(bytes)?));
                dispatch(operation, bytes)
            }
        )
        .is_err()
    );
    assert_eq!(
        *calls.borrow(),
        vec![(Operation::VerifyShuffle, 6)],
        "a changed root loses all prefix trust and is independently rejected"
    );
}
