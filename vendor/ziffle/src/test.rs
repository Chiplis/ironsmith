use super::*;

#[test]
fn three_way_shuffle() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::three_way_shuffle";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let (bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let (jim_sk, jim_pk, jim_id_proof) = shuffle.keygen(&mut rng, ctx);

    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();
    let jim_vpk = jim_id_proof.verify(jim_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk, jim_vpk]);

    let (hal_deck, hal_shfl_proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let hal_vdeck = shuffle
        .verify_initial_shuffle(apk, hal_deck, hal_shfl_proof, ctx)
        .unwrap();

    let (bob_deck, bob_shfl_proof) = shuffle.shuffle_deck(&mut rng, apk, &hal_vdeck, ctx);
    let bob_vdeck = shuffle
        .verify_shuffle(apk, &hal_vdeck, bob_deck, bob_shfl_proof, ctx)
        .unwrap();

    let (jim_deck, jim_shfl_proof) = shuffle.shuffle_deck(&mut rng, apk, &bob_vdeck, ctx);
    let final_vdeck = shuffle
        .verify_shuffle(apk, &bob_vdeck, jim_deck, jim_shfl_proof, ctx)
        .unwrap();

    let first_card = final_vdeck.get(0).unwrap();

    let (hal_rt, hal_rt_proof) = first_card.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);
    let (bob_rt, bob_rt_proof) = first_card.reveal_token(&mut rng, &bob_sk, bob_pk, ctx);
    let (jim_rt, jim_rt_proof) = first_card.reveal_token(&mut rng, &jim_sk, jim_pk, ctx);

    let art = AggregateRevealToken::new(&[
        hal_rt_proof
            .verify(hal_vpk, hal_rt, first_card, ctx)
            .unwrap(),
        bob_rt_proof
            .verify(bob_vpk, bob_rt, first_card, ctx)
            .unwrap(),
        jim_rt_proof
            .verify(jim_vpk, jim_rt, first_card, ctx)
            .unwrap(),
    ]);

    assert!(shuffle.reveal_card(art, first_card).is_some());
}

#[test]
fn serde_roundtrips() {
    use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::serde_roundtrips";

    let (sk, pk, id_proof) = shuffle.keygen(&mut rng, ctx);
    let vpk = id_proof.verify(pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[vpk]);

    let (deck, shfl_proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, shfl_proof, ctx)
        .unwrap();

    let card = vdeck.get(0).unwrap();

    let (reveal_token, reveal_proof) = card.reveal_token(&mut rng, &sk, pk, ctx);

    let mut sk_bytes = [0u8; 1024];
    sk.serialize_compressed(&mut sk_bytes[..]).unwrap();
    let sk_der = SecretKey::deserialize_compressed(&sk_bytes[..]).unwrap();
    assert_eq!(sk, sk_der);

    let mut pk_bytes = [0u8; 1024];
    pk.serialize_compressed(&mut pk_bytes[..]).unwrap();
    let pk_der = PublicKey::deserialize_compressed(&pk_bytes[..]).unwrap();
    assert_eq!(pk, pk_der);

    let mut id_proof_bytes = [0u8; 1024];
    id_proof
        .serialize_compressed(&mut id_proof_bytes[..])
        .unwrap();
    let id_proof_der = OwnershipProof::deserialize_compressed(&id_proof_bytes[..]).unwrap();
    assert_eq!(id_proof, id_proof_der);

    let mut deck_bytes = [0u8; 1024];
    deck.serialize_compressed(&mut deck_bytes[..]).unwrap();
    let deck_der = MaskedDeck::<10>::deserialize_compressed(&deck_bytes[..]).unwrap();
    assert_eq!(deck, deck_der);

    let mut shfl_proof_bytes = [0u8; 2048];
    shfl_proof
        .serialize_compressed(&mut shfl_proof_bytes[..])
        .unwrap();
    let shfl_proof_der = ShuffleProof::<10>::deserialize_compressed(&shfl_proof_bytes[..]).unwrap();
    assert_eq!(shfl_proof, shfl_proof_der);

    let mut reveal_token_bytes = [0u8; 1024];
    reveal_token
        .serialize_compressed(&mut reveal_token_bytes[..])
        .unwrap();
    let reveal_token_der = RevealToken::deserialize_compressed(&reveal_token_bytes[..]).unwrap();
    assert_eq!(reveal_token, reveal_token_der);

    let mut reveal_proof_bytes = [0u8; 1024];
    reveal_proof
        .serialize_compressed(&mut reveal_proof_bytes[..])
        .unwrap();
    let reveal_proof_der =
        RevealTokenProof::deserialize_compressed(&reveal_proof_bytes[..]).unwrap();
    assert_eq!(reveal_proof, reveal_proof_der);
}

#[test]
fn verify_incorrect_public_key_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_incorrect_public_key_fails";

    let (_, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let (_, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);

    assert!(hal_id_proof.verify(bob_pk, ctx).is_none());
    assert!(bob_id_proof.verify(hal_pk, ctx).is_none());

    let t_hal_pk = PublicKey((hal_pk.0.into_group() + GENERATOR).into_affine());
    assert!(hal_id_proof.verify(t_hal_pk, ctx).is_none());
}

#[test]
fn verify_tampered_reveal_token_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_tampered_reveal_token_fails";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let (_bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    let card = vdeck.get(0).unwrap();

    let (hal_rt, hal_rt_proof) = card.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);

    // Valid token should verify
    assert!(hal_rt_proof.verify(hal_vpk, hal_rt, card, ctx).is_some());

    // Tampered reveal token should fail
    let tampered_token = RevealToken((hal_rt.0.into_group() + GENERATOR).into_affine());
    assert!(
        hal_rt_proof
            .verify(hal_vpk, tampered_token, card, ctx)
            .is_none()
    );
}

#[test]
fn verify_tampered_reveal_token_proof_t_g_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_tampered_reveal_token_proof_t_g_fails";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let (_bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    let card = vdeck.get(0).unwrap();

    let (hal_rt, hal_rt_proof) = card.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);

    // Valid proof should verify
    assert!(hal_rt_proof.verify(hal_vpk, hal_rt, card, ctx).is_some());

    // Tampered t_g should fail
    let mut tampered_proof = hal_rt_proof;
    tampered_proof.t_g = (tampered_proof.t_g.into_group() + GENERATOR).into_affine();
    assert!(tampered_proof.verify(hal_vpk, hal_rt, card, ctx).is_none());
}

#[test]
fn verify_tampered_reveal_token_proof_t_c1_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_tampered_reveal_token_proof_t_c1_fails";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let (_bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    let card = vdeck.get(0).unwrap();

    let (hal_rt, hal_rt_proof) = card.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);

    // Valid proof should verify
    assert!(hal_rt_proof.verify(hal_vpk, hal_rt, card, ctx).is_some());

    // Tampered t_c1 should fail
    let mut tampered_proof = hal_rt_proof;
    tampered_proof.t_c1 = (tampered_proof.t_c1.into_group() + GENERATOR).into_affine();
    assert!(tampered_proof.verify(hal_vpk, hal_rt, card, ctx).is_none());
}

#[test]
fn verify_tampered_reveal_token_proof_z_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_tampered_reveal_token_proof_z_fails";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let (_bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    let card = vdeck.get(0).unwrap();

    let (hal_rt, hal_rt_proof) = card.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);

    // Valid proof should verify
    assert!(hal_rt_proof.verify(hal_vpk, hal_rt, card, ctx).is_some());

    // Tampered z should fail
    let mut tampered_proof = hal_rt_proof;
    tampered_proof.z += Scalar::ONE;
    assert!(tampered_proof.verify(hal_vpk, hal_rt, card, ctx).is_none());
}

#[test]
fn verify_reveal_token_wrong_public_key_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_reveal_token_wrong_public_key_fails";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let (_bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    let card = vdeck.get(0).unwrap();

    let (hal_rt, hal_rt_proof) = card.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);

    // Valid proof with correct public key should verify
    assert!(hal_rt_proof.verify(hal_vpk, hal_rt, card, ctx).is_some());

    // Same proof with wrong public key should fail
    assert!(hal_rt_proof.verify(bob_vpk, hal_rt, card, ctx).is_none());
}

#[test]
fn verify_reveal_token_wrong_card_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_reveal_token_wrong_card_fails";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let (_bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    let card0 = vdeck.get(0).unwrap();
    let card1 = vdeck.get(1).unwrap();

    let (hal_rt, hal_rt_proof) = card0.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);

    // Valid proof with correct card should verify
    assert!(hal_rt_proof.verify(hal_vpk, hal_rt, card0, ctx).is_some());

    // Same proof with wrong card should fail
    assert!(hal_rt_proof.verify(hal_vpk, hal_rt, card1, ctx).is_none());
}

#[test]
fn verify_reveal_token_wrong_context_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_reveal_token_wrong_context_fails";
    let wrong_ctx = b"wrong_context";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let (_bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    let card = vdeck.get(0).unwrap();

    let (hal_rt, hal_rt_proof) = card.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);

    // Valid proof with correct context should verify
    assert!(hal_rt_proof.verify(hal_vpk, hal_rt, card, ctx).is_some());

    // Same proof with wrong context should fail
    assert!(
        hal_rt_proof
            .verify(hal_vpk, hal_rt, card, wrong_ctx)
            .is_none()
    );
}

#[test]
fn deck_get_out_of_bounds_returns_none() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::deck_get_out_of_bounds_returns_none";

    let (_, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    // Valid indices should return Some
    assert!(vdeck.get(0).is_some());
    assert!(vdeck.get(9).is_some());

    // Out of bounds should return None
    assert!(vdeck.get(10).is_none());
    assert!(vdeck.get(100).is_none());
}

#[test]
fn reveal_card_with_wrong_aggregate_token_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::reveal_card_with_wrong_aggregate_token_fails";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let (bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    let card0 = vdeck.get(0).unwrap();
    let card1 = vdeck.get(1).unwrap();

    // Create reveal tokens for card0
    let (hal_rt0, hal_rt_proof0) = card0.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);
    let (bob_rt0, bob_rt_proof0) = card0.reveal_token(&mut rng, &bob_sk, bob_pk, ctx);

    // Create reveal tokens for card1
    let (hal_rt1, hal_rt_proof1) = card1.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);
    let (bob_rt1, bob_rt_proof1) = card1.reveal_token(&mut rng, &bob_sk, bob_pk, ctx);

    // Correct aggregate token for card0 should reveal successfully
    let art0 = AggregateRevealToken::new(&[
        hal_rt_proof0.verify(hal_vpk, hal_rt0, card0, ctx).unwrap(),
        bob_rt_proof0.verify(bob_vpk, bob_rt0, card0, ctx).unwrap(),
    ]);
    assert!(shuffle.reveal_card(art0, card0).is_some());

    // Wrong aggregate token (using tokens from card1) should fail for card0
    let art1 = AggregateRevealToken::new(&[
        hal_rt_proof1.verify(hal_vpk, hal_rt1, card1, ctx).unwrap(),
        bob_rt_proof1.verify(bob_vpk, bob_rt1, card1, ctx).unwrap(),
    ]);
    assert!(shuffle.reveal_card(art1, card0).is_none());
}

#[test]
fn verify_shuffle_with_wrong_context_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_shuffle_with_wrong_context_fails";
    let wrong_ctx = b"wrong_context";

    let (_, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);

    // Correct context should verify
    assert!(
        shuffle
            .verify_initial_shuffle(apk, deck, proof, ctx)
            .is_some()
    );

    // Wrong context should fail
    assert!(
        shuffle
            .verify_initial_shuffle(apk, deck, proof, wrong_ctx)
            .is_none()
    );
}

#[test]
fn reveal_all_cards_in_deck() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::reveal_all_cards_in_deck";

    let (hal_sk, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let hal_vpk = hal_id_proof.verify(hal_pk, ctx).unwrap();

    let (bob_sk, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);
    let bob_vpk = bob_id_proof.verify(bob_pk, ctx).unwrap();

    let apk = AggregatePublicKey::new(&[hal_vpk, bob_vpk]);

    let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let vdeck = shuffle
        .verify_initial_shuffle(apk, deck, proof, ctx)
        .unwrap();

    let mut revealed_indices = [None; 10];

    // Reveal all 10 cards
    for i in 0..10 {
        let card = vdeck.get(i).unwrap();

        let (hal_rt, hal_rt_proof) = card.reveal_token(&mut rng, &hal_sk, hal_pk, ctx);
        let (bob_rt, bob_rt_proof) = card.reveal_token(&mut rng, &bob_sk, bob_pk, ctx);

        let art = AggregateRevealToken::new(&[
            hal_rt_proof.verify(hal_vpk, hal_rt, card, ctx).unwrap(),
            bob_rt_proof.verify(bob_vpk, bob_rt, card, ctx).unwrap(),
        ]);

        let revealed_idx = shuffle.reveal_card(art, card);
        assert!(revealed_idx.is_some(), "Failed to reveal card {i}");
        revealed_indices[revealed_idx.unwrap()] = Some(i);
    }

    assert!(revealed_indices.iter().all(Option::is_some));
}

type TamperFn<const N: usize> =
    fn(MaskedDeck<N>, ShuffleProof<N>) -> (MaskedDeck<N>, ShuffleProof<N>);

macro_rules! tamper_fns {
    ($($name:ident: $body:expr),* $(,)?) => {
        const TAMPER_FUNCTIONS: &[(&str, TamperFn<10>)] = &[
            $(
                (stringify!($name), $body),
            )*
        ];
    };
}

tamper_fns! {
    deck_flip_c1_c2: |mut deck, proof| {
        deck.0[5] = (deck.0[5].1, deck.0[5].0);
        (deck, proof)
    },
    deck_tweak_c1: |mut deck, proof| {
        deck.0[5] = ((deck.0[5].0 + GENERATOR).into_affine(), deck.0[5].1);
        (deck, proof)
    },
    deck_tweak_c2: |mut deck, proof| {
        deck.0[5] = (deck.0[5].0, (deck.0[5].1 + GENERATOR).into_affine());
        (deck, proof)
    },
    deck_swap_first_and_last: |mut deck, proof| {
        deck.0.swap(0, 9);
        (deck, proof)
    },
    deck_swap_middle: |mut deck, proof| {
        deck.0.swap(4, 5);
        (deck, proof)
    },
    deck_duplicate_card: |mut deck, proof| {
        deck.0[5] = deck.0[0];
        (deck, proof)
    },
    deck_zero_c1: |mut deck, proof| {
        deck.0[5] = (CurveAffine::identity(), deck.0[5].1);
        (deck, proof)
    },
    deck_zero_c2: |mut deck, proof| {
        deck.0[5] = (deck.0[5].0, CurveAffine::identity());
        (deck, proof)
    },
    deck_reverse: |mut deck, proof| {
        deck.0.reverse();
        (deck, proof)
    },
    proof_tweak_c_pi: |deck, mut proof| {
        proof.c_pi = PedersonCommitment((proof.c_pi.0.into_group() + GENERATOR).into_affine());
        (deck, proof)
    },
    proof_tweak_c_xpi: |deck, mut proof| {
        proof.c_xpi = PedersonCommitment((proof.c_xpi.0.into_group() + GENERATOR).into_affine());
        (deck, proof)
    },
    proof_swap_c_pi_c_xpi: |deck, mut proof| {
        core::mem::swap(&mut proof.c_pi, &mut proof.c_xpi);
        (deck, proof)
    },
    proof_mexp_arg_beta: |deck, mut proof| {
        proof.mexp_arg.beta += Scalar::ONE;
        (deck, proof)
    },
    proof_prod_arg_a_tilde: |deck, mut proof| {
        proof.prod_arg.a_tilde[0] += Scalar::ONE;
        (deck, proof)
    },
    proof_mexp_arg_c_alpha: |deck, mut proof| {
        proof.mexp_arg.c_alpha = PedersonCommitment((proof.mexp_arg.c_alpha.0.into_group() + GENERATOR).into_affine());
        (deck, proof)
    },
    proof_mexp_arg_c_beta: |deck, mut proof| {
        proof.mexp_arg.c_beta = PedersonCommitment((proof.mexp_arg.c_beta.0.into_group() + GENERATOR).into_affine());
        (deck, proof)
    },
    proof_mexp_arg_ct_mxp0_c1: |deck, mut proof| {
        proof.mexp_arg.ct_mxp0.0 = (proof.mexp_arg.ct_mxp0.0.into_group() + GENERATOR).into_affine();
        (deck, proof)
    },
    proof_mexp_arg_ct_mxp1_c2: |deck, mut proof| {
        proof.mexp_arg.ct_mxp1.1 = (proof.mexp_arg.ct_mxp1.1.into_group() + GENERATOR).into_affine();
        (deck, proof)
    },
    proof_mexp_arg_o_alpha: |deck, mut proof| {
        proof.mexp_arg.o_alpha[0] += Scalar::ONE;
        (deck, proof)
    },
    proof_mexp_arg_o_r: |deck, mut proof| {
        proof.mexp_arg.o_r += Scalar::ONE;
        (deck, proof)
    },
    proof_mexp_arg_o_beta: |deck, mut proof| {
        proof.mexp_arg.o_beta += Scalar::ONE;
        (deck, proof)
    },
    proof_mexp_arg_tau: |deck, mut proof| {
        proof.mexp_arg.tau += Scalar::ONE;
        (deck, proof)
    },
    proof_prod_arg_c_d: |deck, mut proof| {
        proof.prod_arg.c_d = PedersonCommitment((proof.prod_arg.c_d.0.into_group() + GENERATOR).into_affine());
        (deck, proof)
    },
    proof_prod_arg_c_sdelta: |deck, mut proof| {
        proof.prod_arg.c_sdelta = PedersonCommitment((proof.prod_arg.c_sdelta.0.into_group() + GENERATOR).into_affine());
        (deck, proof)
    },
    proof_prod_arg_c_cdelta: |deck, mut proof| {
        proof.prod_arg.c_cdelta = PedersonCommitment((proof.prod_arg.c_cdelta.0.into_group() + GENERATOR).into_affine());
        (deck, proof)
    },
    proof_prod_arg_b_tilde: |deck, mut proof| {
        proof.prod_arg.b_tilde[0] += Scalar::ONE;
        (deck, proof)
    },
    proof_prod_arg_r_tilde: |deck, mut proof| {
        proof.prod_arg.r_tilde += Scalar::ONE;
        (deck, proof)
    },
    proof_prod_arg_s_tilde: |deck, mut proof| {
        proof.prod_arg.s_tilde += Scalar::ONE;
        (deck, proof)
    },
}

#[test]
fn verify_tampered_initial_shuffle_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_intitial_shuffle_tampered_fails";

    let (_, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let (_, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);

    let apk = AggregatePublicKey::new(&[
        hal_id_proof.verify(hal_pk, ctx).unwrap(),
        bob_id_proof.verify(bob_pk, ctx).unwrap(),
    ]);

    let (valid_deck, valid_proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);

    assert!(
        shuffle
            .verify_initial_shuffle(apk, valid_deck, valid_proof, ctx)
            .is_some()
    );

    for (name, tamper_fn) in TAMPER_FUNCTIONS {
        let (tampered_deck, tampered_proof) = tamper_fn(valid_deck, valid_proof);

        assert!(
            shuffle
                .verify_initial_shuffle(apk, tampered_deck, tampered_proof, ctx)
                .is_none(),
            "tamper function '{name}' was not detected",
        );
    }
}

#[test]
fn verify_tampered_shuffle_fails() {
    let mut rng = ark_std::test_rng();

    let shuffle = Shuffle::<10>::default();

    let ctx = b"test::verify_intitial_shuffle_tampered_fails";

    let (_, hal_pk, hal_id_proof) = shuffle.keygen(&mut rng, ctx);
    let (_, bob_pk, bob_id_proof) = shuffle.keygen(&mut rng, ctx);

    let apk = AggregatePublicKey::new(&[
        hal_id_proof.verify(hal_pk, ctx).unwrap(),
        bob_id_proof.verify(bob_pk, ctx).unwrap(),
    ]);

    let (initial_deck, initial_proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    let initial_vdeck = shuffle
        .verify_initial_shuffle(apk, initial_deck, initial_proof, ctx)
        .unwrap();

    let (deck, proof) = shuffle.shuffle_deck(&mut rng, apk, &initial_vdeck, ctx);

    assert!(
        shuffle
            .verify_shuffle(apk, &initial_vdeck, deck, proof, ctx)
            .is_some()
    );

    for (name, tamper_fn) in TAMPER_FUNCTIONS {
        let (tampered_deck, tampered_proof) = tamper_fn(deck, proof);

        assert!(
            shuffle
                .verify_initial_shuffle(apk, tampered_deck, tampered_proof, ctx)
                .is_none(),
            "tamper function '{name}' was not detected",
        );
    }
}
