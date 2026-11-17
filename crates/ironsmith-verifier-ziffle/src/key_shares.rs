//! Verifiable sharing of a seat's Ziffle secret key among the other seats.
//!
//! Every hidden-card reveal needs one reveal token per roster key, so a seat
//! that leaves for good would stop the remaining seats from drawing. Each seat
//! therefore deals a Feldman-verifiable Shamir sharing of its secret key at
//! match start. Once a seat is forfeited, the holders publish their shares of
//! that seat's key and anyone can rebuild it and produce its reveal tokens.
//!
//! A share travels encrypted to its holder's Ziffle public key (hashed
//! ElGamal), so a deal is one public record: the holder opens its share with
//! its own secret key, and whoever later rebuilds a forfeited seat's key can
//! open the shares that seat was holding. The polynomial commitments let every
//! opened share be checked against the dealer's public key without trust.

use ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
use ark_ff::{Field, PrimeField, UniformRand, Zero};
use ark_secp256k1::{Affine, Fr, Projective};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{VerifierError, decode, encode, rng_from_entropy_hex, ziffle_from_hex, ziffle_to_hex};

const SHARE_KEY_DOMAIN: &[u8] = b"ironsmith-ziffle-key-share-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyShareOperation {
    Deal,
    Open,
    Verify,
    Recover,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HolderInput {
    player: u8,
    public_key_hex: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DealInput {
    context: String,
    dealer: u8,
    public_key_hex: String,
    secret_key_hex: String,
    holders: Vec<HolderInput>,
    threshold: usize,
    entropy_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SealedShare {
    holder: u8,
    ephemeral_hex: String,
    cipher_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DealOutput {
    dealer: u8,
    threshold: usize,
    commitments_hex: Vec<String>,
    shares: Vec<SealedShare>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenInput {
    context: String,
    dealer: u8,
    commitments_hex: Vec<String>,
    share: SealedShare,
    /// Secret key of the share's holder: its own, or one rebuilt from shares.
    holder_secret_key_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenShare {
    holder: u8,
    share_hex: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyInput {
    commitments_hex: Vec<String>,
    holder: u8,
    share_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifyOutput {
    valid: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoverInput {
    public_key_hex: String,
    commitments_hex: Vec<String>,
    shares: Vec<OpenShare>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecoverOutput {
    secret_key_hex: String,
}

pub fn execute_key_share(
    operation: KeyShareOperation,
    input: &[u8],
) -> Result<Vec<u8>, VerifierError> {
    match operation {
        KeyShareOperation::Deal => encode(&deal(decode(input, "ziffle key-share deal")?)?),
        KeyShareOperation::Open => encode(&open(decode(input, "ziffle key-share opening")?)?),
        KeyShareOperation::Verify => {
            let input: VerifyInput = decode(input, "ziffle key-share check")?;
            let commitments = commitments_from_hex(&input.commitments_hex)?;
            let valid = ziffle_from_hex::<Fr>(&input.share_hex, "ziffle key share")
                .map(|share| share_matches(&commitments, input.holder, share))
                .unwrap_or(false);
            encode(&VerifyOutput { valid })
        }
        KeyShareOperation::Recover => {
            encode(&recover(decode(input, "ziffle key-share recovery")?)?)
        }
    }
}

/// A share is the dealer's polynomial evaluated at the holder's seat plus one,
/// so seat 0 never receives the secret itself.
fn share_point(holder: u8) -> Fr {
    Fr::from(u64::from(holder) + 1)
}

fn commitments_from_hex(commitments_hex: &[String]) -> Result<Vec<Affine>, VerifierError> {
    if commitments_hex.is_empty() {
        return Err(VerifierError::new(
            "ziffle key-share deal has no commitments",
        ));
    }
    commitments_hex
        .iter()
        .map(|hex| ziffle_from_hex::<Affine>(hex, "ziffle key-share commitment"))
        .collect()
}

/// Feldman check: `share * G` equals the committed polynomial at the holder.
fn share_matches(commitments: &[Affine], holder: u8, share: Fr) -> bool {
    let x = share_point(holder);
    let mut expected = Projective::zero();
    for commitment in commitments.iter().rev() {
        expected = expected * x + commitment.into_group();
    }
    Projective::generator() * share == expected
}

fn share_mask(context: &str, dealer: u8, holder: u8, ephemeral: &Affine, shared: &Affine) -> Fr {
    let mut hasher = Sha256::new();
    hasher.update(SHARE_KEY_DOMAIN);
    hasher.update((context.len() as u64).to_le_bytes());
    hasher.update(context.as_bytes());
    hasher.update([dealer, holder]);
    for point in [ephemeral, shared] {
        let mut bytes = Vec::new();
        ark_serialize::CanonicalSerialize::serialize_compressed(point, &mut bytes)
            .expect("curve point serializes to memory");
        hasher.update(bytes);
    }
    Fr::from_le_bytes_mod_order(&hasher.finalize())
}

fn deal(input: DealInput) -> Result<DealOutput, VerifierError> {
    let secret: Fr = ziffle_from_hex(&input.secret_key_hex, "ziffle secret key")?;
    let public: Affine = ziffle_from_hex(&input.public_key_hex, "ziffle public key")?;
    if (Projective::generator() * secret).into_affine() != public {
        return Err(VerifierError::new(
            "ziffle key-share deal does not match the dealer's public key",
        ));
    }
    if input.threshold == 0 || input.threshold > input.holders.len() {
        return Err(VerifierError::new(
            "ziffle key-share threshold must be between one and the number of holders",
        ));
    }
    let mut seen = Vec::with_capacity(input.holders.len());
    for holder in &input.holders {
        if holder.player == input.dealer || seen.contains(&holder.player) {
            return Err(VerifierError::new(
                "ziffle key-share holders must be distinct seats other than the dealer",
            ));
        }
        seen.push(holder.player);
    }
    let mut rng = rng_from_entropy_hex(&input.entropy_hex)?;
    let mut coefficients = vec![secret];
    while coefficients.len() < input.threshold {
        coefficients.push(Fr::rand(&mut rng));
    }
    let commitments_hex = coefficients
        .iter()
        .map(|coefficient| ziffle_to_hex(&(Projective::generator() * coefficient).into_affine()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut shares = Vec::with_capacity(input.holders.len());
    for holder in &input.holders {
        let holder_key: Affine =
            ziffle_from_hex(&holder.public_key_hex, "ziffle holder public key")?;
        if holder_key.is_zero() {
            return Err(VerifierError::new("ziffle holder public key is invalid"));
        }
        let x = share_point(holder.player);
        let mut share = Fr::zero();
        for coefficient in coefficients.iter().rev() {
            share = share * x + coefficient;
        }
        let nonce = Fr::rand(&mut rng);
        let ephemeral = (Projective::generator() * nonce).into_affine();
        let shared = (holder_key.into_group() * nonce).into_affine();
        let mask = share_mask(
            &input.context,
            input.dealer,
            holder.player,
            &ephemeral,
            &shared,
        );
        shares.push(SealedShare {
            holder: holder.player,
            ephemeral_hex: ziffle_to_hex(&ephemeral)?,
            cipher_hex: ziffle_to_hex(&(share + mask))?,
        });
    }
    Ok(DealOutput {
        dealer: input.dealer,
        threshold: input.threshold,
        commitments_hex,
        shares,
    })
}

fn open(input: OpenInput) -> Result<OpenShare, VerifierError> {
    let commitments = commitments_from_hex(&input.commitments_hex)?;
    let secret: Fr = ziffle_from_hex(&input.holder_secret_key_hex, "ziffle holder secret key")?;
    let ephemeral: Affine =
        ziffle_from_hex(&input.share.ephemeral_hex, "ziffle key-share ephemeral key")?;
    let cipher: Fr = ziffle_from_hex(&input.share.cipher_hex, "ziffle sealed key share")?;
    let shared = (ephemeral.into_group() * secret).into_affine();
    let mask = share_mask(
        &input.context,
        input.dealer,
        input.share.holder,
        &ephemeral,
        &shared,
    );
    let share = cipher - mask;
    if !share_matches(&commitments, input.share.holder, share) {
        return Err(VerifierError::new(format!(
            "ziffle key share dealt by player {} to player {} does not match its commitments",
            input.dealer, input.share.holder
        )));
    }
    Ok(OpenShare {
        holder: input.share.holder,
        share_hex: ziffle_to_hex(&share)?,
    })
}

fn recover(input: RecoverInput) -> Result<RecoverOutput, VerifierError> {
    let commitments = commitments_from_hex(&input.commitments_hex)?;
    let public: Affine = ziffle_from_hex(&input.public_key_hex, "ziffle public key")?;
    if commitments[0] != public {
        return Err(VerifierError::new(
            "ziffle key-share commitments belong to a different public key",
        ));
    }
    let mut points: Vec<(u8, Fr)> = Vec::with_capacity(commitments.len());
    for share in &input.shares {
        if points.iter().any(|(holder, _)| *holder == share.holder) {
            continue;
        }
        let value: Fr = ziffle_from_hex(&share.share_hex, "ziffle key share")?;
        if !share_matches(&commitments, share.holder, value) {
            return Err(VerifierError::new(format!(
                "ziffle key share from player {} does not match its commitments",
                share.holder
            )));
        }
        points.push((share.holder, value));
        if points.len() == commitments.len() {
            break;
        }
    }
    if points.len() < commitments.len() {
        return Err(VerifierError::new(format!(
            "ziffle key recovery needs {} shares, got {}",
            commitments.len(),
            points.len()
        )));
    }
    // Lagrange interpolation at zero.
    let mut secret = Fr::zero();
    for (index, (holder, value)) in points.iter().enumerate() {
        let x = share_point(*holder);
        let mut numerator = Fr::ONE;
        let mut denominator = Fr::ONE;
        for (other_index, (other, _)) in points.iter().enumerate() {
            if other_index == index {
                continue;
            }
            let other_x = share_point(*other);
            numerator *= other_x;
            denominator *= other_x - x;
        }
        let inverse = denominator
            .inverse()
            .ok_or_else(|| VerifierError::new("ziffle key shares repeat a holder"))?;
        secret += *value * numerator * inverse;
    }
    if (Projective::generator() * secret).into_affine() != public {
        return Err(VerifierError::new(
            "recovered ziffle key does not match the public key",
        ));
    }
    Ok(RecoverOutput {
        secret_key_hex: ziffle_to_hex(&secret)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Shuffle, ZiffleBuildRevealTokenInput, ZiffleBuildShuffleStepInput, ZifflePublicKeyInput,
        ZiffleRevealCardInput, ZiffleRevealTokenInput, ZiffleShuffleStepInput,
        build_ziffle_reveal_token, build_ziffle_shuffle_step, reveal_ziffle_card,
    };
    use serde_json::{Value, json};

    struct Seat {
        player: u8,
        secret_key_hex: String,
        public_key_hex: String,
        ownership_proof_hex: String,
    }

    fn seats(count: u8, context: &str) -> Vec<Seat> {
        let shuffle = Shuffle::new(10);
        let mut rng = rng_from_entropy_hex("5ea7c0de00112233").expect("rng");
        (0..count)
            .map(|player| {
                let (secret, public, proof) = shuffle.keygen(&mut rng, context.as_bytes());
                Seat {
                    player,
                    secret_key_hex: ziffle_to_hex(&secret).expect("secret hex"),
                    public_key_hex: ziffle_to_hex(&public).expect("public hex"),
                    ownership_proof_hex: ziffle_to_hex(&proof).expect("proof hex"),
                }
            })
            .collect()
    }

    fn call(operation: KeyShareOperation, input: Value) -> Result<Value, VerifierError> {
        let output = execute_key_share(operation, input.to_string().as_bytes())?;
        Ok(serde_json::from_slice(&output).expect("output json"))
    }

    fn deal_for(seats: &[Seat], dealer: u8, context: &str) -> Value {
        let holders = seats
            .iter()
            .filter(|seat| seat.player != dealer)
            .map(|seat| json!({ "player": seat.player, "publicKeyHex": seat.public_key_hex }))
            .collect::<Vec<_>>();
        let seat = &seats[dealer as usize];
        call(
            KeyShareOperation::Deal,
            json!({
                "context": context,
                "dealer": dealer,
                "publicKeyHex": seat.public_key_hex,
                "secretKeyHex": seat.secret_key_hex,
                "threshold": holders.len(),
                "holders": holders,
                "entropyHex": format!("d0d0cafe{dealer:02x}"),
            }),
        )
        .expect("deal")
    }

    fn open_for(
        deal: &Value,
        holder: &Seat,
        secret_key_hex: &str,
        context: &str,
    ) -> Result<Value, VerifierError> {
        let share = deal["shares"]
            .as_array()
            .expect("shares")
            .iter()
            .find(|share| share["holder"] == json!(holder.player))
            .expect("holder share")
            .clone();
        call(
            KeyShareOperation::Open,
            json!({
                "context": context,
                "dealer": deal["dealer"],
                "commitmentsHex": deal["commitmentsHex"],
                "share": share,
                "holderSecretKeyHex": secret_key_hex,
            }),
        )
    }

    #[test]
    fn every_other_seat_together_rebuilds_a_dealt_key() {
        for count in [3u8, 4] {
            let context = "key-share-match";
            let seats = seats(count, context);
            let deal = deal_for(&seats, 0, context);
            assert_eq!(deal["commitmentsHex"][0], json!(seats[0].public_key_hex));
            let opened = seats[1..]
                .iter()
                .map(|seat| open_for(&deal, seat, &seat.secret_key_hex, context).expect("open"))
                .collect::<Vec<_>>();
            for share in &opened {
                let check = call(
                    KeyShareOperation::Verify,
                    json!({
                        "commitmentsHex": deal["commitmentsHex"],
                        "holder": share["holder"],
                        "shareHex": share["shareHex"],
                    }),
                )
                .expect("verify");
                assert_eq!(check["valid"], json!(true));
            }
            let recovered = call(
                KeyShareOperation::Recover,
                json!({
                    "publicKeyHex": seats[0].public_key_hex,
                    "commitmentsHex": deal["commitmentsHex"],
                    "shares": opened,
                }),
            )
            .expect("recover");
            assert_eq!(recovered["secretKeyHex"], json!(seats[0].secret_key_hex));

            // One share short reveals nothing usable.
            let short = call(
                KeyShareOperation::Recover,
                json!({
                    "publicKeyHex": seats[0].public_key_hex,
                    "commitmentsHex": deal["commitmentsHex"],
                    "shares": opened[1..].to_vec(),
                }),
            );
            assert!(short.is_err());
        }
    }

    #[test]
    fn wrong_or_foreign_shares_are_rejected() {
        let context = "key-share-match";
        let seats = seats(3, context);
        let deal = deal_for(&seats, 0, context);
        // A seat cannot open a share sealed to another seat.
        assert!(open_for(&deal, &seats[1], &seats[2].secret_key_hex, context).is_err());
        // A share is bound to its match.
        assert!(open_for(&deal, &seats[1], &seats[1].secret_key_hex, "another-match").is_err());
        let good = open_for(&deal, &seats[1], &seats[1].secret_key_hex, context).expect("open");
        let other = open_for(&deal, &seats[2], &seats[2].secret_key_hex, context).expect("open");
        // A share presented under another holder's seat fails the public check.
        let swapped = call(
            KeyShareOperation::Verify,
            json!({
                "commitmentsHex": deal["commitmentsHex"],
                "holder": 2,
                "shareHex": good["shareHex"],
            }),
        )
        .expect("verify");
        assert_eq!(swapped["valid"], json!(false));
        let forged = call(
            KeyShareOperation::Recover,
            json!({
                "publicKeyHex": seats[0].public_key_hex,
                "commitmentsHex": deal["commitmentsHex"],
                "shares": [good, { "holder": 2, "shareHex": other["shareHex"].as_str().map(|hex| {
                    let mut bytes = hex.to_string();
                    let flipped = if bytes.starts_with('0') { "1" } else { "0" };
                    bytes.replace_range(0..1, flipped);
                    bytes
                }) }],
            }),
        );
        assert!(forged.is_err());
    }

    #[test]
    fn a_rebuilt_key_opens_the_shares_its_seat_was_holding() {
        let context = "key-share-match";
        let seats = seats(4, context);
        // Seat 0 leaves: seats 1..3 rebuild its key.
        let deal_zero = deal_for(&seats, 0, context);
        let opened = seats[1..]
            .iter()
            .map(|seat| open_for(&deal_zero, seat, &seat.secret_key_hex, context).expect("open"))
            .collect::<Vec<_>>();
        let rebuilt_zero = call(
            KeyShareOperation::Recover,
            json!({
                "publicKeyHex": seats[0].public_key_hex,
                "commitmentsHex": deal_zero["commitmentsHex"],
                "shares": opened,
            }),
        )
        .expect("recover")["secretKeyHex"]
            .as_str()
            .expect("secret")
            .to_string();
        // Seat 1 leaves next: seat 0's share of seat 1's key is readable with
        // the rebuilt key, so seats 2 and 3 can finish the job.
        let deal_one = deal_for(&seats, 1, context);
        let shares = vec![
            open_for(&deal_one, &seats[0], &rebuilt_zero, context).expect("open with rebuilt key"),
            open_for(&deal_one, &seats[2], &seats[2].secret_key_hex, context).expect("open"),
            open_for(&deal_one, &seats[3], &seats[3].secret_key_hex, context).expect("open"),
        ];
        let rebuilt_one = call(
            KeyShareOperation::Recover,
            json!({
                "publicKeyHex": seats[1].public_key_hex,
                "commitmentsHex": deal_one["commitmentsHex"],
                "shares": shares,
            }),
        )
        .expect("recover");
        assert_eq!(rebuilt_one["secretKeyHex"], json!(seats[1].secret_key_hex));
    }

    #[test]
    fn a_rebuilt_key_produces_reveal_tokens_the_table_accepts() {
        let context = "key-share-reveal".to_string();
        let seats = seats(3, &context);
        let keys = seats
            .iter()
            .map(|seat| ZifflePublicKeyInput {
                player: seat.player,
                public_key_hex: seat.public_key_hex.clone(),
                ownership_proof_hex: seat.ownership_proof_hex.clone(),
            })
            .collect::<Vec<_>>();
        let mut steps = Vec::new();
        for seat in &seats {
            let step = build_ziffle_shuffle_step(
                10,
                ZiffleBuildShuffleStepInput {
                    deck_count: 10,
                    context: context.clone(),
                    key_context: String::new(),
                    input_deck: None,
                    keys: keys.clone(),
                    steps: steps.clone(),
                    shuffler: seat.player,
                    entropy_hex: format!("feedface{:02x}", seat.player),
                },
            )
            .expect("step");
            steps.push(ZiffleShuffleStepInput {
                shuffler: seat.player,
                deck_hex: step.deck_hex,
                proof_hex: step.proof_hex,
            });
        }
        // Seat 2 is gone; seats 0 and 1 rebuild its key from their shares.
        let deal = deal_for(&seats, 2, &context);
        let opened = seats[..2]
            .iter()
            .map(|seat| open_for(&deal, seat, &seat.secret_key_hex, &context).expect("open"))
            .collect::<Vec<_>>();
        let rebuilt = call(
            KeyShareOperation::Recover,
            json!({
                "publicKeyHex": seats[2].public_key_hex,
                "commitmentsHex": deal["commitmentsHex"],
                "shares": opened,
            }),
        )
        .expect("recover")["secretKeyHex"]
            .as_str()
            .expect("secret")
            .to_string();
        let mut tokens = Vec::new();
        for seat in &seats {
            let secret_key_hex = if seat.player == 2 {
                rebuilt.clone()
            } else {
                seat.secret_key_hex.clone()
            };
            let token = build_ziffle_reveal_token(
                10,
                ZiffleBuildRevealTokenInput {
                    deck_count: 10,
                    deck_hash: String::new(),
                    context: context.clone(),
                    key_context: String::new(),
                    input_deck: None,
                    keys: keys.clone(),
                    steps: steps.clone(),
                    card_position: 3,
                    public_key_hex: seat.public_key_hex.clone(),
                    secret_key_hex,
                    entropy_hex: format!("decafbad{:02x}", seat.player),
                },
            )
            .expect("token");
            assert_eq!(token.player, seat.player);
            tokens.push(ZiffleRevealTokenInput {
                player: token.player,
                public_key_hex: token.public_key_hex,
                token_hex: token.token_hex,
                proof_hex: token.proof_hex,
            });
        }
        let reveal = reveal_ziffle_card(
            10,
            ZiffleRevealCardInput {
                deck_count: 10,
                deck_hash: String::new(),
                context,
                key_context: String::new(),
                input_deck: None,
                keys,
                steps,
                card_position: 3,
                tokens,
            },
        )
        .expect("reveal with a rebuilt key");
        assert!(reveal.original_slot < 10);
    }
}
