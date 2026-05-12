use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};
use ark_std::rand::{SeedableRng as ArkSeedableRng, rngs::StdRng as ArkStdRng};
use ziffle::{
    AggregatePublicKey, AggregateRevealToken, MaskedDeck, OwnershipProof, PublicKey, RevealToken,
    RevealTokenProof, SecretKey, Shuffle, ShuffleProof, Verified,
};

fn ziffle_to_hex<T: CanonicalSerialize>(value: &T) -> Result<String, JsValue> {
    let mut bytes = Vec::new();
    value
        .serialize_compressed(&mut bytes)
        .map_err(|e| JsValue::from_str(&format!("failed to serialize ziffle artifact: {e}")))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn ziffle_from_hex<T: CanonicalDeserialize>(hex: &str, label: &str) -> Result<T, JsValue> {
    let bytes = hex_to_vec(hex).map_err(|e| JsValue::from_str(&format!("invalid {label}: {e}")))?;
    T::deserialize_with_mode(bytes.as_slice(), Compress::Yes, Validate::Yes)
        .map_err(|e| JsValue::from_str(&format!("failed to decode {label}: {e}")))
}

fn hex_to_vec(hex: &str) -> Result<Vec<u8>, String> {
    let normalized = hex.trim();
    if normalized.len() % 2 != 0 {
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

fn rng_from_entropy_hex(hex: &str) -> Result<ArkStdRng, JsValue> {
    let bytes = hex_to_vec(hex).map_err(|e| JsValue::from_str(&format!("invalid entropy: {e}")))?;
    if bytes.is_empty() {
        return Err(JsValue::from_str("ziffle entropy cannot be empty"));
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

fn ziffle_deck_hash(deck_hex: &str) -> Result<String, JsValue> {
    let bytes = hex_to_vec(deck_hex)
        .map_err(|e| JsValue::from_str(&format!("invalid ziffle deck hex: {e}")))?;
    Ok(sha256_hex(&bytes))
}

fn verified_public_keys(
    keys: &[ZifflePublicKeyInput],
    context: &[u8],
) -> Result<Vec<(u8, PublicKey, Verified<PublicKey>)>, JsValue> {
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        let public_key: PublicKey = ziffle_from_hex(&key.public_key_hex, "ziffle public key")?;
        let proof: OwnershipProof =
            ziffle_from_hex(&key.ownership_proof_hex, "ziffle ownership proof")?;
        let verified = proof.verify(public_key, context).ok_or_else(|| {
            JsValue::from_str(&format!(
                "ziffle ownership proof failed for player {}",
                key.player
            ))
        })?;
        out.push((key.player, public_key, verified));
    }
    out.sort_by_key(|(player, _, _)| *player);
    Ok(out)
}

fn aggregate_public_key(
    keys: &[ZifflePublicKeyInput],
    context: &[u8],
) -> Result<
    (
        Vec<(u8, PublicKey, Verified<PublicKey>)>,
        AggregatePublicKey,
    ),
    JsValue,
> {
    let verified = verified_public_keys(keys, context)?;
    if verified.is_empty() {
        return Err(JsValue::from_str("ziffle requires at least one public key"));
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
) -> Result<Option<Verified<MaskedDeck<N>>>, JsValue> {
    let shuffle = Shuffle::<N>::default();
    let (verified_keys, aggregate) = aggregate_public_key(keys, key_context)?;
    let mut verified_deck = None;
    for (index, step) in steps.iter().enumerate() {
        let expected_shuffler = verified_keys
            .get(index)
            .map(|(player, _, _)| *player)
            .ok_or_else(|| JsValue::from_str("ziffle shuffle has more steps than player keys"))?;
        if step.shuffler != expected_shuffler {
            return Err(JsValue::from_str(&format!(
                "ziffle shuffle step {index} was attributed to player {}, expected player {expected_shuffler}",
                step.shuffler
            )));
        }
        let deck: MaskedDeck<N> = ziffle_from_hex(&step.deck_hex, "ziffle masked deck")?;
        let proof: ShuffleProof<N> = ziffle_from_hex(&step.proof_hex, "ziffle shuffle proof")?;
        verified_deck = Some(if index == 0 {
            shuffle
                .verify_initial_shuffle(aggregate, deck, proof, context)
                .ok_or_else(|| JsValue::from_str("ziffle initial shuffle proof failed"))?
        } else {
            let previous = verified_deck
                .as_ref()
                .ok_or_else(|| JsValue::from_str("ziffle previous deck is missing"))?;
            shuffle
                .verify_shuffle(aggregate, previous, deck, proof, context)
                .ok_or_else(|| JsValue::from_str(&format!("ziffle shuffle proof {index} failed")))?
        });
    }
    Ok(verified_deck)
}

fn build_ziffle_shuffle_step<const N: usize>(
    input: ZiffleBuildShuffleStepInput,
) -> Result<ZiffleShuffleStepOutput, JsValue> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let shuffle = Shuffle::<N>::default();
    let (verified_keys, aggregate) = aggregate_public_key(&input.keys, key_context)?;
    let expected_shuffler = verified_keys
        .get(input.steps.len())
        .map(|(player, _, _)| *player)
        .ok_or_else(|| JsValue::from_str("ziffle shuffle already has one step for every player"))?;
    if input.shuffler != expected_shuffler {
        return Err(JsValue::from_str(&format!(
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
) -> Result<ZiffleVerifyShuffleOutput, JsValue> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    if input.steps.len() != input.keys.len() {
        return Err(JsValue::from_str("ziffle final shuffle must include one step per player"));
    }
    verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| JsValue::from_str("ziffle ceremony has no shuffle steps"))?;
    let deck_hex = input
        .steps
        .last()
        .map(|step| step.deck_hex.clone())
        .ok_or_else(|| JsValue::from_str("ziffle ceremony has no shuffle steps"))?;
    Ok(ZiffleVerifyShuffleOutput {
        deck_count: N,
        deck_hash: ziffle_deck_hash(&deck_hex)?,
        deck_hex,
    })
}

fn build_ziffle_reveal_token<const N: usize>(
    input: ZiffleBuildRevealTokenInput,
) -> Result<ZiffleRevealTokenOutput, JsValue> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let verified_deck = verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| JsValue::from_str("ziffle ceremony has no shuffle steps"))?;
    let card = verified_deck
        .get(input.card_position)
        .ok_or_else(|| JsValue::from_str("ziffle card position is out of range"))?;
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
) -> Result<Vec<ZiffleRevealTokenBatchOutput>, JsValue> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let verified_deck = verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| JsValue::from_str("ziffle ceremony has no shuffle steps"))?;
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
            .ok_or_else(|| JsValue::from_str("ziffle card position is out of range"))?;
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
) -> Result<ZiffleRevealCardOutput, JsValue> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let (keys, _) = aggregate_public_key(&input.keys, key_context)?;
    let verified_deck = verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| JsValue::from_str("ziffle ceremony has no shuffle steps"))?;
    let card = verified_deck
        .get(input.card_position)
        .ok_or_else(|| JsValue::from_str("ziffle card position is out of range"))?;
    let mut verified_tokens = Vec::with_capacity(keys.len());
    for (player, _, verified_public_key) in keys {
        let token_input = input
            .tokens
            .iter()
            .find(|token| token.player == player)
            .ok_or_else(|| {
                JsValue::from_str(&format!("missing ziffle reveal token for player {player}"))
            })?;
        let token: RevealToken = ziffle_from_hex(&token_input.token_hex, "ziffle reveal token")?;
        let proof: RevealTokenProof =
            ziffle_from_hex(&token_input.proof_hex, "ziffle reveal-token proof")?;
        verified_tokens.push(
            proof
                .verify(verified_public_key, token, card, context)
                .ok_or_else(|| {
                    JsValue::from_str(&format!(
                        "ziffle reveal-token proof failed for player {player}"
                    ))
                })?,
        );
    }
    let aggregate = AggregateRevealToken::new(&verified_tokens);
    let shuffle = Shuffle::<N>::default();
    let original_slot = shuffle
        .reveal_card(aggregate, card)
        .ok_or_else(|| JsValue::from_str("ziffle aggregate reveal failed"))?;
    Ok(ZiffleRevealCardOutput {
        card_position: input.card_position,
        original_slot,
    })
}

fn reveal_ziffle_cards<const N: usize>(
    input: ZiffleRevealCardsInput,
) -> Result<Vec<ZiffleRevealCardOutput>, JsValue> {
    let context = input.context.as_bytes();
    let key_context = ziffle_key_context(&input.key_context, &input.context).as_bytes();
    let (keys, _) = aggregate_public_key(&input.keys, key_context)?;
    let verified_deck = verify_ziffle_steps::<N>(context, key_context, &input.keys, &input.steps)?
        .ok_or_else(|| JsValue::from_str("ziffle ceremony has no shuffle steps"))?;
    let shuffle = Shuffle::<N>::default();
    let mut out = Vec::with_capacity(input.card_positions.len());
    for card_position in input.card_positions {
        let card = verified_deck
            .get(card_position)
            .ok_or_else(|| JsValue::from_str("ziffle card position is out of range"))?;
        let mut verified_tokens = Vec::with_capacity(keys.len());
        for (player, _, verified_public_key) in &keys {
            let token_input = input
                .tokens
                .iter()
                .find(|token| token.player == *player && token.card_position == card_position)
                .ok_or_else(|| {
                    JsValue::from_str(&format!(
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
                        JsValue::from_str(&format!(
                            "ziffle reveal-token proof failed for player {player}"
                        ))
                    })?,
            );
        }
        let aggregate = AggregateRevealToken::new(&verified_tokens);
        let original_slot = shuffle
            .reveal_card(aggregate, card)
            .ok_or_else(|| JsValue::from_str("ziffle aggregate reveal failed"))?;
        out.push(ZiffleRevealCardOutput {
            card_position,
            original_slot,
        });
    }
    Ok(out)
}

macro_rules! ziffle_dispatch {
    ($deck_count:expr, $func:ident, $input:expr) => {
        match $deck_count {
            2 => $func::<2>($input),
            3 => $func::<3>($input),
            4 => $func::<4>($input),
            5 => $func::<5>($input),
            6 => $func::<6>($input),
            7 => $func::<7>($input),
            8 => $func::<8>($input),
            9 => $func::<9>($input),
            10 => $func::<10>($input),
            11 => $func::<11>($input),
            12 => $func::<12>($input),
            13 => $func::<13>($input),
            14 => $func::<14>($input),
            15 => $func::<15>($input),
            16 => $func::<16>($input),
            17 => $func::<17>($input),
            18 => $func::<18>($input),
            19 => $func::<19>($input),
            20 => $func::<20>($input),
            21 => $func::<21>($input),
            22 => $func::<22>($input),
            23 => $func::<23>($input),
            24 => $func::<24>($input),
            25 => $func::<25>($input),
            26 => $func::<26>($input),
            27 => $func::<27>($input),
            28 => $func::<28>($input),
            29 => $func::<29>($input),
            30 => $func::<30>($input),
            31 => $func::<31>($input),
            32 => $func::<32>($input),
            33 => $func::<33>($input),
            34 => $func::<34>($input),
            35 => $func::<35>($input),
            36 => $func::<36>($input),
            37 => $func::<37>($input),
            38 => $func::<38>($input),
            39 => $func::<39>($input),
            40 => $func::<40>($input),
            41 => $func::<41>($input),
            42 => $func::<42>($input),
            43 => $func::<43>($input),
            44 => $func::<44>($input),
            45 => $func::<45>($input),
            46 => $func::<46>($input),
            47 => $func::<47>($input),
            48 => $func::<48>($input),
            49 => $func::<49>($input),
            50 => $func::<50>($input),
            51 => $func::<51>($input),
            52 => $func::<52>($input),
            53 => $func::<53>($input),
            54 => $func::<54>($input),
            55 => $func::<55>($input),
            56 => $func::<56>($input),
            57 => $func::<57>($input),
            58 => $func::<58>($input),
            59 => $func::<59>($input),
            60 => $func::<60>($input),
            61 => $func::<61>($input),
            62 => $func::<62>($input),
            63 => $func::<63>($input),
            64 => $func::<64>($input),
            65 => $func::<65>($input),
            66 => $func::<66>($input),
            67 => $func::<67>($input),
            68 => $func::<68>($input),
            69 => $func::<69>($input),
            70 => $func::<70>($input),
            71 => $func::<71>($input),
            72 => $func::<72>($input),
            73 => $func::<73>($input),
            74 => $func::<74>($input),
            75 => $func::<75>($input),
            76 => $func::<76>($input),
            77 => $func::<77>($input),
            78 => $func::<78>($input),
            79 => $func::<79>($input),
            80 => $func::<80>($input),
            81 => $func::<81>($input),
            82 => $func::<82>($input),
            83 => $func::<83>($input),
            84 => $func::<84>($input),
            85 => $func::<85>($input),
            86 => $func::<86>($input),
            87 => $func::<87>($input),
            88 => $func::<88>($input),
            89 => $func::<89>($input),
            90 => $func::<90>($input),
            91 => $func::<91>($input),
            92 => $func::<92>($input),
            93 => $func::<93>($input),
            94 => $func::<94>($input),
            95 => $func::<95>($input),
            96 => $func::<96>($input),
            97 => $func::<97>($input),
            98 => $func::<98>($input),
            99 => $func::<99>($input),
            100 => $func::<100>($input),
            other => Err(JsValue::from_str(&format!(
                "unsupported ziffle deck size {other}; supported sizes are 2 through 100"
            ))),
        }
    };
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(js_name = ziffleKeygen)]
    pub fn ziffle_keygen(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: ZiffleEntropyInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid ziffle keygen input: {e}")))?;
        let mut rng = rng_from_entropy_hex(&input.entropy_hex)?;
        let context = input.context.as_bytes();
        let shuffle = Shuffle::<60>::default();
        let (secret_key, public_key, ownership_proof) = shuffle.keygen(&mut rng, context);
        serde_wasm_bindgen::to_value(&ZiffleKeygenOutput {
            deck_count: input.deck_count,
            public_key_hex: ziffle_to_hex(&public_key)?,
            secret_key_hex: ziffle_to_hex(&secret_key)?,
            ownership_proof_hex: ziffle_to_hex(&ownership_proof)?,
        })
        .map_err(|e| JsValue::from_str(&format!("failed to encode ziffle keygen output: {e}")))
    }

    #[wasm_bindgen(js_name = ziffleBuildShuffleStep)]
    pub fn ziffle_build_shuffle_step(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: ZiffleBuildShuffleStepInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid ziffle shuffle input: {e}")))?;
        let output = ziffle_dispatch!(input.deck_count, build_ziffle_shuffle_step, input)?;
        serde_wasm_bindgen::to_value(&output)
            .map_err(|e| JsValue::from_str(&format!("failed to encode ziffle shuffle step: {e}")))
    }

    #[wasm_bindgen(js_name = ziffleVerifyShuffle)]
    pub fn ziffle_verify_shuffle(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: ZiffleVerifyShuffleInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid ziffle verify input: {e}")))?;
        let output = ziffle_dispatch!(input.deck_count, verify_ziffle_shuffle, input)?;
        serde_wasm_bindgen::to_value(&output)
            .map_err(|e| JsValue::from_str(&format!("failed to encode ziffle verify output: {e}")))
    }

    #[wasm_bindgen(js_name = ziffleBuildRevealToken)]
    pub fn ziffle_build_reveal_token(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: ZiffleBuildRevealTokenInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid ziffle reveal token input: {e}")))?;
        let output = ziffle_dispatch!(input.deck_count, build_ziffle_reveal_token, input)?;
        serde_wasm_bindgen::to_value(&output).map_err(|e| {
            JsValue::from_str(&format!("failed to encode ziffle reveal token output: {e}"))
        })
    }

    #[wasm_bindgen(js_name = ziffleBuildRevealTokens)]
    pub fn ziffle_build_reveal_tokens(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: ZiffleBuildRevealTokensInput =
            serde_wasm_bindgen::from_value(input).map_err(|e| {
                JsValue::from_str(&format!("invalid ziffle reveal tokens input: {e}"))
            })?;
        let output = ziffle_dispatch!(input.deck_count, build_ziffle_reveal_tokens, input)?;
        serde_wasm_bindgen::to_value(&output).map_err(|e| {
            JsValue::from_str(&format!("failed to encode ziffle reveal tokens output: {e}"))
        })
    }

    #[wasm_bindgen(js_name = ziffleRevealCard)]
    pub fn ziffle_reveal_card(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: ZiffleRevealCardInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid ziffle reveal input: {e}")))?;
        let output = ziffle_dispatch!(input.deck_count, reveal_ziffle_card, input)?;
        serde_wasm_bindgen::to_value(&output)
            .map_err(|e| JsValue::from_str(&format!("failed to encode ziffle reveal output: {e}")))
    }

    #[wasm_bindgen(js_name = ziffleRevealCards)]
    pub fn ziffle_reveal_cards(&self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: ZiffleRevealCardsInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid ziffle reveals input: {e}")))?;
        let output = ziffle_dispatch!(input.deck_count, reveal_ziffle_cards, input)?;
        serde_wasm_bindgen::to_value(&output).map_err(|e| {
            JsValue::from_str(&format!("failed to encode ziffle reveals output: {e}"))
        })
    }
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
