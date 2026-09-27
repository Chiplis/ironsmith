use super::*;

const N: usize = 4;

struct Fixture {
    input: ZiffleVerifyShuffleInput,
    secrets: Vec<String>,
}

fn fixture() -> Fixture {
    VERIFIED_CEREMONIES.with(|cache| cache.borrow_mut().clear());
    let mut input = ZiffleVerifyShuffleInput {
        deck_count: N,
        context: "cache-test-shuffle".into(),
        key_context: "cache-test-keys".into(),
        input_deck: None,
        keys: Vec::new(),
        steps: Vec::new(),
    };
    let shuffle = Shuffle::<N>::default();
    let mut rng = rng_from_entropy_hex("abcdef123456").unwrap();
    let mut secrets = Vec::new();
    for player in 0..2 {
        let (secret, public, proof) = shuffle.keygen(&mut rng, input.key_context.as_bytes());
        secrets.push(ziffle_to_hex(&secret).unwrap());
        input.keys.push(ZifflePublicKeyInput {
            player,
            public_key_hex: ziffle_to_hex(&public).unwrap(),
            ownership_proof_hex: ziffle_to_hex(&proof).unwrap(),
        });
    }
    for shuffler in 0..2 {
        let step = build_ziffle_shuffle_step::<N>(ZiffleBuildShuffleStepInput {
            deck_count: N,
            context: input.context.clone(),
            key_context: input.key_context.clone(),
            input_deck: None,
            keys: input.keys.clone(),
            steps: input.steps.clone(),
            shuffler,
            entropy_hex: format!("feedface{shuffler:02x}"),
        })
        .unwrap();
        input.steps.push(ZiffleShuffleStepInput {
            shuffler,
            deck_hex: step.deck_hex,
            proof_hex: step.proof_hex,
        });
    }
    Fixture { input, secrets }
}

fn verify(input: &ZiffleVerifyShuffleInput) -> Result<Rc<VerifiedCeremony<N>>, VerifierError> {
    verify_ziffle_steps::<N>(
        input.context.as_bytes(),
        ziffle_key_context(&input.key_context, &input.context).as_bytes(),
        &input.keys,
        &input.steps,
    )
}

fn key<const M: usize>(input: &ZiffleVerifyShuffleInput) -> [u8; 32] {
    ceremony_cache_key::<M>(
        input.context.as_bytes(),
        ziffle_key_context(&input.key_context, &input.context).as_bytes(),
        &input.keys,
        &input.steps,
    )
    .unwrap()
}

fn corrupt(hex: &mut String) {
    // Change coordinate/scalar bytes, not an unused compressed-point flag.
    let replacement = if hex.starts_with('0') { "1" } else { "0" };
    hex.replace_range(0..1, replacement);
}

fn reveal_input(fixture: &Fixture) -> ZiffleRevealCardInput {
    let input = &fixture.input;
    let tokens = input
        .keys
        .iter()
        .enumerate()
        .map(|(seat, key)| {
            let token = build_ziffle_reveal_token::<N>(ZiffleBuildRevealTokenInput {
                deck_count: N,
                deck_hash: String::new(),
                context: input.context.clone(),
                key_context: input.key_context.clone(),
                input_deck: None,
                keys: input.keys.clone(),
                steps: input.steps.clone(),
                card_position: 0,
                public_key_hex: key.public_key_hex.clone(),
                secret_key_hex: fixture.secrets[seat].clone(),
                entropy_hex: format!("aabbcc{seat:02x}"),
            })
            .unwrap();
            ZiffleRevealTokenInput {
                player: key.player,
                public_key_hex: token.public_key_hex,
                token_hex: token.token_hex,
                proof_hex: token.proof_hex,
            }
        })
        .collect();
    ZiffleRevealCardInput {
        deck_count: N,
        deck_hash: String::new(),
        context: input.context.clone(),
        key_context: input.key_context.clone(),
        input_deck: None,
        keys: input.keys.clone(),
        steps: input.steps.clone(),
        card_position: 0,
        tokens,
    }
}

#[test]
fn cache_reuses_only_the_exact_verified_public_transcript() {
    let fixture = fixture();
    let input = fixture.input;
    let first = verify(&input).unwrap();
    assert!(Rc::ptr_eq(&first, &verify(&input).unwrap()));
    let mut mutations = Vec::new();
    let mut add = |label, change: fn(&mut ZiffleVerifyShuffleInput)| {
        let mut changed = input.clone();
        change(&mut changed);
        mutations.push((label, changed));
    };
    add("shuffle context", |i| i.context.push('!'));
    add("key context", |i| i.key_context.push('!'));
    add("public key", |i| {
        i.keys[0].public_key_hex = i.keys[1].public_key_hex.clone()
    });
    add("ownership proof", |i| {
        i.keys[0].ownership_proof_hex = i.keys[1].ownership_proof_hex.clone()
    });
    add("key player", |i| i.keys[0].player = 7);
    add("key list", |i| {
        i.keys.pop();
    });
    add("step shuffler", |i| i.steps[0].shuffler = 1);
    add("step deck", |i| corrupt(&mut i.steps[0].deck_hex));
    add("step proof", |i| corrupt(&mut i.steps[0].proof_hex));
    add("step order", |i| i.steps.reverse());
    for (label, changed) in mutations {
        assert_ne!(key::<N>(&input), key::<N>(&changed), "{label}");
        let before = VERIFIED_CEREMONIES.with(|cache| cache.borrow().len());
        assert!(
            verify(&changed).is_err(),
            "{label} reused or passed cached verification"
        );
        assert_eq!(
            before,
            VERIFIED_CEREMONIES.with(|cache| cache.borrow().len()),
            "failed {label} was cached"
        );
        assert!(Rc::ptr_eq(&first, &verify(&input).unwrap()));
    }
    let mut reordered = input.clone();
    reordered.keys.reverse();
    assert_ne!(key::<N>(&input), key::<N>(&reordered));
    let other = verify(&reordered).unwrap();
    assert!(
        !Rc::ptr_eq(&first, &other),
        "reordered input must verify independently"
    );
    assert_ne!(key::<N>(&input), key::<5>(&input));
    assert!(
        verify_ziffle_steps::<5>(
            input.context.as_bytes(),
            input.key_context.as_bytes(),
            &input.keys,
            &input.steps
        )
        .is_err()
    );
    let request = serde_json::json!({"deckCount":5,"context":input.context,"keyContext":input.key_context,"keys":input.keys,"steps":input.steps});
    assert!(
        execute_for::<N>(
            Operation::VerifyShuffle,
            &serde_json::to_vec(&request).unwrap()
        )
        .unwrap_err()
        .to_string()
        .contains("shard mismatch")
    );
}

#[test]
fn cached_prefixes_still_require_all_final_steps_and_the_expected_shuffler() {
    let fixture = fixture();
    let mut prefix = fixture.input.clone();
    prefix.steps.truncate(1);
    verify(&prefix).unwrap();
    assert!(verify_ziffle_shuffle::<N>(prefix.clone()).is_err());
    prefix.steps.clear();
    assert!(verify(&prefix).unwrap().deck.is_none());
    assert!(verify_ziffle_shuffle::<N>(prefix.clone()).is_err());
    assert!(
        build_ziffle_shuffle_step::<N>(ZiffleBuildShuffleStepInput {
            deck_count: N,
            context: prefix.context,
            key_context: prefix.key_context,
            input_deck: None,
            keys: prefix.keys,
            steps: prefix.steps,
            shuffler: 1,
            entropy_hex: "deadbeef".into(),
        })
        .is_err()
    );
    verify_ziffle_shuffle::<N>(fixture.input).unwrap();
}

#[test]
fn warm_cache_never_bypasses_token_proofs_or_card_positions() {
    let fixture = fixture();
    let good = reveal_input(&fixture);
    let first = reveal_ziffle_card::<N>(good.clone()).unwrap();
    assert_eq!(
        first.original_slot,
        reveal_ziffle_card::<N>(good.clone()).unwrap().original_slot
    );
    let mut invalid = good.clone();
    corrupt(&mut invalid.tokens[0].proof_hex);
    assert!(reveal_ziffle_card::<N>(invalid).is_err());
    let mut invalid = good.clone();
    corrupt(&mut invalid.tokens[0].token_hex);
    assert!(reveal_ziffle_card::<N>(invalid).is_err());
    let mut invalid = good.clone();
    invalid.tokens.pop();
    assert!(reveal_ziffle_card::<N>(invalid).is_err());
    let mut invalid = good.clone();
    invalid.card_position = 1;
    assert!(
        reveal_ziffle_card::<N>(invalid).is_err(),
        "tokens belong to position zero"
    );
    let mut invalid = good.clone();
    invalid.card_position = N;
    assert!(reveal_ziffle_card::<N>(invalid).is_err());
    let batch = ZiffleRevealCardsInput {
        deck_count: N,
        deck_hash: String::new(),
        context: good.context,
        key_context: good.key_context,
        input_deck: None,
        keys: good.keys,
        steps: good.steps,
        card_positions: vec![0],
        tokens: good
            .tokens
            .into_iter()
            .map(|token| ZiffleRevealTokenBatchInput {
                card_position: 0,
                player: token.player,
                public_key_hex: token.public_key_hex,
                token_hex: token.token_hex,
                proof_hex: token.proof_hex,
            })
            .collect(),
    };
    assert_eq!(
        reveal_ziffle_cards::<N>(batch.clone()).unwrap()[0].original_slot,
        first.original_slot
    );
    let mut invalid = batch.clone();
    corrupt(&mut invalid.tokens[0].proof_hex);
    assert!(reveal_ziffle_cards::<N>(invalid).is_err());
    let mut invalid = batch;
    invalid.card_positions = vec![N];
    assert!(reveal_ziffle_cards::<N>(invalid).is_err());
    let input = fixture.input;
    assert!(
        build_ziffle_reveal_tokens::<N>(ZiffleBuildRevealTokensInput {
            deck_count: N,
            deck_hash: String::new(),
            context: input.context,
            key_context: input.key_context,
            input_deck: None,
            public_key_hex: input.keys[0].public_key_hex.clone(),
            secret_key_hex: fixture.secrets[0].clone(),
            keys: input.keys,
            steps: input.steps,
            card_positions: vec![N],
            entropy_hex: "aabb".into(),
        })
        .is_err()
    );
}

#[test]
fn cache_has_one_lru_bound_across_deck_sizes() {
    let fixture = fixture();
    let mut prefix = fixture.input.clone();
    prefix.steps.clear();
    VERIFIED_CEREMONIES.with(|cache| cache.borrow_mut().clear());
    let oldest = verify(&fixture.input).unwrap();
    for index in 0..VERIFIED_CEREMONY_CACHE_CAPACITY {
        // Empty prefixes still verify the fixed key context. Changing shuffle
        // context produces distinct valid entries without generating proofs.
        prefix.context = format!("eviction-{index}");
        if index % 2 == 0 {
            verify(&prefix).unwrap();
        } else {
            verify_ziffle_steps::<5>(
                prefix.context.as_bytes(),
                prefix.key_context.as_bytes(),
                &prefix.keys,
                &prefix.steps,
            )
            .unwrap();
        }
    }
    assert_eq!(
        VERIFIED_CEREMONIES.with(|cache| cache.borrow().len()),
        VERIFIED_CEREMONY_CACHE_CAPACITY
    );
    assert!(VERIFIED_CEREMONIES.with(|cache| {
        cache
            .borrow()
            .iter()
            .all(|(cached, _)| *cached != key::<N>(&fixture.input))
    }));
    let reverified = verify(&fixture.input).unwrap();
    assert!(!Rc::ptr_eq(&oldest, &reverified));
    assert!(Rc::ptr_eq(&reverified, &verify(&fixture.input).unwrap()));
}
