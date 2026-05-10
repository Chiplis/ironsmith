use serde::{Deserialize, Serialize};

pub const ZIFFLE_0_1_BACKEND: &str = "ziffle-0.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentalPokerBackend {
    pub id: String,
    pub protocol: MentalPokerProtocol,
    pub enabled: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MentalPokerProtocol {
    BayerGroth2012ElGamal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentalPokerKeyArtifact {
    pub player: u8,
    pub public_key_hex: String,
    pub ownership_proof_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentalPokerDeckArtifact {
    pub owner: u8,
    pub deck_id: String,
    pub encrypted_deck_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentalPokerShuffleArtifact {
    pub shuffler: u8,
    pub input_deck_hash: String,
    pub output_deck_hash: String,
    pub proof_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentalPokerRevealArtifact {
    pub player: u8,
    pub encrypted_card_hash: String,
    pub reveal_token_hex: String,
    pub reveal_token_proof_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentalPokerArtifactBundle {
    pub backend: MentalPokerBackend,
    pub keys: Vec<MentalPokerKeyArtifact>,
    pub decks: Vec<MentalPokerDeckArtifact>,
    pub shuffles: Vec<MentalPokerShuffleArtifact>,
    pub reveals: Vec<MentalPokerRevealArtifact>,
}

pub fn selected_backend() -> MentalPokerBackend {
    MentalPokerBackend {
        id: ZIFFLE_0_1_BACKEND.to_string(),
        protocol: MentalPokerProtocol::BayerGroth2012ElGamal,
        enabled: true,
        notes: "ziffle 0.1 Bayer-Groth mental-poker backend".to_string(),
    }
}

pub fn backend_is_supported(id: &str) -> bool {
    id == ZIFFLE_0_1_BACKEND
}

pub fn ziffle_four_player_round_trip<const N: usize, R: ark_std::rand::Rng>(
    rng: &mut R,
    ctx: &[u8],
) -> Result<usize, String> {
    use ziffle::{AggregatePublicKey, AggregateRevealToken, Shuffle};

    let shuffle = Shuffle::<N>::default();

    let (p0_sk, p0_pk, p0_proof) = shuffle.keygen(rng, ctx);
    let (p1_sk, p1_pk, p1_proof) = shuffle.keygen(rng, ctx);
    let (p2_sk, p2_pk, p2_proof) = shuffle.keygen(rng, ctx);
    let (p3_sk, p3_pk, p3_proof) = shuffle.keygen(rng, ctx);

    let p0_vpk = p0_proof
        .verify(p0_pk, ctx)
        .ok_or_else(|| "player 0 key proof failed".to_string())?;
    let p1_vpk = p1_proof
        .verify(p1_pk, ctx)
        .ok_or_else(|| "player 1 key proof failed".to_string())?;
    let p2_vpk = p2_proof
        .verify(p2_pk, ctx)
        .ok_or_else(|| "player 2 key proof failed".to_string())?;
    let p3_vpk = p3_proof
        .verify(p3_pk, ctx)
        .ok_or_else(|| "player 3 key proof failed".to_string())?;
    let apk = AggregatePublicKey::new(&[p0_vpk, p1_vpk, p2_vpk, p3_vpk]);

    let (deck0, proof0) = shuffle.shuffle_initial_deck(rng, apk, ctx);
    let vdeck0 = shuffle
        .verify_initial_shuffle(apk, deck0, proof0, ctx)
        .ok_or_else(|| "initial shuffle proof failed".to_string())?;
    let (deck1, proof1) = shuffle.shuffle_deck(rng, apk, &vdeck0, ctx);
    let vdeck1 = shuffle
        .verify_shuffle(apk, &vdeck0, deck1, proof1, ctx)
        .ok_or_else(|| "player 1 shuffle proof failed".to_string())?;
    let (deck2, proof2) = shuffle.shuffle_deck(rng, apk, &vdeck1, ctx);
    let vdeck2 = shuffle
        .verify_shuffle(apk, &vdeck1, deck2, proof2, ctx)
        .ok_or_else(|| "player 2 shuffle proof failed".to_string())?;
    let (deck3, proof3) = shuffle.shuffle_deck(rng, apk, &vdeck2, ctx);
    let vdeck3 = shuffle
        .verify_shuffle(apk, &vdeck2, deck3, proof3, ctx)
        .ok_or_else(|| "player 3 shuffle proof failed".to_string())?;

    let first_card = vdeck3
        .get(0)
        .ok_or_else(|| "shuffled deck has no first card".to_string())?;
    let (p0_token, p0_token_proof) = first_card.reveal_token(rng, &p0_sk, p0_pk, ctx);
    let (p1_token, p1_token_proof) = first_card.reveal_token(rng, &p1_sk, p1_pk, ctx);
    let (p2_token, p2_token_proof) = first_card.reveal_token(rng, &p2_sk, p2_pk, ctx);
    let (p3_token, p3_token_proof) = first_card.reveal_token(rng, &p3_sk, p3_pk, ctx);

    let aggregate = AggregateRevealToken::new(&[
        p0_token_proof
            .verify(p0_vpk, p0_token, first_card, ctx)
            .ok_or_else(|| "player 0 reveal-token proof failed".to_string())?,
        p1_token_proof
            .verify(p1_vpk, p1_token, first_card, ctx)
            .ok_or_else(|| "player 1 reveal-token proof failed".to_string())?,
        p2_token_proof
            .verify(p2_vpk, p2_token, first_card, ctx)
            .ok_or_else(|| "player 2 reveal-token proof failed".to_string())?,
        p3_token_proof
            .verify(p3_vpk, p3_token, first_card, ctx)
            .ok_or_else(|| "player 3 reveal-token proof failed".to_string())?,
    ]);

    shuffle
        .reveal_card(aggregate, first_card)
        .ok_or_else(|| "aggregate reveal failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_backend_is_enabled() {
        let backend = selected_backend();
        assert_eq!(backend.id, ZIFFLE_0_1_BACKEND);
        assert_eq!(backend.protocol, MentalPokerProtocol::BayerGroth2012ElGamal);
        assert!(backend.enabled);
    }

    #[test]
    fn ziffle_backend_round_trip_reveals_a_card_index() {
        let mut rng = ark_std::test_rng();
        let revealed = ziffle_four_player_round_trip::<10, _>(
            &mut rng,
            b"ironsmith-audit::backend-round-trip",
        )
        .expect("ziffle backend should complete four-player round trip");
        assert!(revealed < 10);
    }
}
