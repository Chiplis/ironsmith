//! Mental poker shuffle protocol using zero-knowledge proofs.
//!
//! `ziffle` implements a mental poker protocol where multiple players can
//! collaboratively shuffle a deck of cards without any player learning the order,
//! and then selectively reveal individual cards.
//!
//! The protocol uses [Bayer-Groth 2012](http://www0.cs.ucl.ac.uk/staff/J.Groth/MinimalShuffle.pdf)
//! shuffle proofs to ensure that each shuffle is performed correctly without revealing
//! the permutation.
//!
//! `ziffle` is a `#[no_std]` crate.
//!
//! # ⚠️ Security Warning
//!
//! **This code has not been independently audited.** It is experimental software provided
//! as-is without any warranties. While the implementation follows the Bayer-Groth 2012
//! specification, it may contain bugs or vulnerabilities.
//!
//! **DO NOT use this library to play for non-trivial amounts of money or in any
//! high-stakes scenario.** Use at your own risk.
//!
//! # Main Entry Point
//!
//! The [`Shuffle`] struct is the primary interface for using this library. Create an
//! instance with `Shuffle::<N>::default()` where `N` is the number of cards in your deck.
//!
//! # Example: Three-Player Poker Game
//!
//! ```
//! use ziffle::{Shuffle, AggregatePublicKey, AggregateRevealToken};
//!
//! // Create a standard 52-card deck
//! let shuffle = Shuffle::<52>::default();
//! let mut rng = ark_std::test_rng(); // DO NOT USE IN PRODUCTION
//! let ctx = b"poker_game_session_123";
//!
//! // Three players generate their keys and prove ownership
//! let (alice_sk, alice_pk, alice_proof) = shuffle.keygen(&mut rng, ctx);
//! let (bob_sk, bob_pk, bob_proof) = shuffle.keygen(&mut rng, ctx);
//! let (carol_sk, carol_pk, carol_proof) = shuffle.keygen(&mut rng, ctx);
//!
//! // Each player verifies others' key ownership proofs
//! let alice_vpk = alice_proof.verify(alice_pk, ctx).unwrap();
//! let bob_vpk = bob_proof.verify(bob_pk, ctx).unwrap();
//! let carol_vpk = carol_proof.verify(carol_pk, ctx).unwrap();
//!
//! // Create aggregate public key from all verified keys
//! let apk = AggregatePublicKey::new(&[alice_vpk, bob_vpk, carol_vpk]);
//!
//! // Alice performs the initial shuffle
//! let (alice_deck, alice_proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
//! let alice_vdeck = shuffle
//!     .verify_initial_shuffle(apk, alice_deck, alice_proof, ctx)
//!     .expect("Alice's shuffle should be valid");
//!
//! // Bob shuffles Alice's deck
//! let (bob_deck, bob_proof) = shuffle.shuffle_deck(&mut rng, apk, &alice_vdeck, ctx);
//! let bob_vdeck = shuffle
//!     .verify_shuffle(apk, &alice_vdeck, bob_deck, bob_proof, ctx)
//!     .expect("Bob's shuffle should be valid");
//!
//! // Carol shuffles Bob's deck
//! let (final_deck, carol_proof) = shuffle.shuffle_deck(&mut rng, apk, &bob_vdeck, ctx);
//! let final_vdeck = shuffle
//!     .verify_shuffle(apk, &bob_vdeck, final_deck, carol_proof, ctx)
//!     .expect("Carol's shuffle should be valid");
//!
//! // Now the deck is fully shuffled and encrypted. Let's reveal the first card.
//! let first_card = final_vdeck.get(0).unwrap();
//!
//! // Each player creates a reveal token for the first card
//! let (alice_token, alice_token_proof) =
//!     first_card.reveal_token(&mut rng, &alice_sk, alice_pk, ctx);
//! let (bob_token, bob_token_proof) =
//!     first_card.reveal_token(&mut rng, &bob_sk, bob_pk, ctx);
//! let (carol_token, carol_token_proof) =
//!     first_card.reveal_token(&mut rng, &carol_sk, carol_pk, ctx);
//!
//! // All players verify each other's reveal tokens
//! let alice_vtoken = alice_token_proof
//!     .verify(alice_vpk, alice_token, first_card, ctx)
//!     .expect("Alice's token should be valid");
//! let bob_vtoken = bob_token_proof
//!     .verify(bob_vpk, bob_token, first_card, ctx)
//!     .expect("Bob's token should be valid");
//! let carol_vtoken = carol_token_proof
//!     .verify(carol_vpk, carol_token, first_card, ctx)
//!     .expect("Carol's token should be valid");
//!
//! // Aggregate the verified tokens to decrypt the card
//! let aggregate_token = AggregateRevealToken::new(&[alice_vtoken, bob_vtoken, carol_vtoken]);
//!
//! // Reveal the card's index in the original deck (0-51)
//! let card_index = shuffle
//!     .reveal_card(aggregate_token, first_card)
//!     .expect("Card should be revealed successfully");
//!
//! println!("First card is at index: {}", card_index);
//! assert!(card_index < 52);
//! ```
//!
//! # Serialized Sizes
//!
//! The following table shows the serialized sizes of types that need to be transmitted
//! over the network in a typical mental poker protocol. All sizes use compressed canonical
//! serialization from [arkworks](https://docs.rs/ark-serialize/0.5.0/ark_serialize/index.html).
//!
//! | Type | Size | Notes |
//! |------|------|-------|
//! | `PublicKey` | 33 bytes | One per player at setup |
//! | `OwnershipProof` | 65 bytes | One per player at setup |
//! | `MaskedDeck<52>` | 3,432 bytes | 66 bytes per card (33 × 2) |
//! | `ShuffleProof<52>` | 5,547 bytes | One per shuffle operation |
//! | `RevealToken` | 33 bytes | One per player per revealed card |
//! | `RevealTokenProof` | 98 bytes | One per player per revealed card |
#![no_std]
#![forbid(clippy::all)]

use core::array;

use ark_ec::{AffineRepr, CurveConfig, CurveGroup, short_weierstrass::SWCurveConfig};
use ark_ff::{
    Field, UniformRand, Zero,
    field_hashers::{DefaultFieldHasher, HashToField},
};
use ark_serialize::CanonicalSerialize;
use ark_std::rand::{RngCore as Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

type Curve = ark_secp256k1::Config;
type CurveAffine = ark_secp256k1::Affine;
type CurveProj = <CurveAffine as AffineRepr>::Group;
type Scalar = <Curve as CurveConfig>::ScalarField;
type Ciphertext = (CurveAffine, CurveAffine);

const GENERATOR: CurveAffine = <Curve as SWCurveConfig>::GENERATOR;

// Deterministic PRNG Seeds
const OPEN_CARD_PRNG_SEED: &[u8] = b"CARDS-V1";
const PEDERSON_H_PRNG_SEED: &[u8] = b"PEDERSON-H-V1";
const PEDERSON_VECTOR_G_PRNG_SEED: &[u8] = b"PEDERSON-VECTOR-G-V1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PedersonCommitment(CurveAffine);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PedersonWitness(Scalar);

#[derive(Debug, Clone, Copy)]
struct PedersonCommitKey<const N: usize> {
    h: CurveProj,
    gs: [CurveProj; N],
}

impl<const N: usize> Default for PedersonCommitKey<N> {
    fn default() -> Self {
        let mut h_drng = StdRng::from_seed(Sha256::digest(PEDERSON_H_PRNG_SEED).into());
        let h = CurveProj::rand(&mut h_drng);

        let mut gs_drng = StdRng::from_seed(Sha256::digest(PEDERSON_VECTOR_G_PRNG_SEED).into());
        let gs = array::from_fn(|_| CurveProj::rand(&mut gs_drng));

        Self { h, gs }
    }
}

impl<const N: usize> PedersonCommitKey<N> {
    fn commit_with_r(&self, m: Scalar, r: Scalar) -> CurveProj {
        (GENERATOR * m) + (self.h * r)
    }

    fn commit<R: Rng>(&self, rng: &mut R, m: Scalar) -> (PedersonCommitment, PedersonWitness) {
        let r = Scalar::rand(rng);
        (
            PedersonCommitment(self.commit_with_r(m, r).into_affine()),
            PedersonWitness(r),
        )
    }

    fn vector_commit_with_r(&self, ms: &[Scalar; N], r: Scalar) -> CurveProj {
        (0..N).map(|i| self.gs[i] * ms[i]).sum::<CurveProj>() + (self.h * r)
    }

    fn vector_commit<R: Rng>(
        &self,
        rng: &mut R,
        ms: &[Scalar; N],
    ) -> (PedersonCommitment, PedersonWitness) {
        let r = Scalar::rand(rng);
        (
            PedersonCommitment(self.vector_commit_with_r(ms, r).into_affine()),
            PedersonWitness(r),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Transcript([u8; 32]);

impl Transcript {
    const SERIALIZE_BUFFER_SIZE: usize = 256;

    fn init(user_ctx: &[u8]) -> Self {
        Self(Sha256::digest(user_ctx).into())
    }

    fn update_with_serialized<T: CanonicalSerialize>(h: &mut Sha256, label: &str, t: &T) {
        let mut serialize_buffer = [0u8; Self::SERIALIZE_BUFFER_SIZE];
        let serialized_size = t.compressed_size();
        assert!(
            serialized_size <= Self::SERIALIZE_BUFFER_SIZE,
            "serialize buffer too small to serialize {label}: {serialized_size} < {}",
            Self::SERIALIZE_BUFFER_SIZE
        );
        t.serialize_compressed(&mut serialize_buffer[..serialized_size])
            .expect("infallible serialization");
        h.update(serialized_size.to_be_bytes());
        h.update(&serialize_buffer[..serialized_size]);
    }

    fn append<T: CanonicalSerialize>(self, label: &str, t: &T) -> Self {
        let mut h = Sha256::new();
        h.update(self.0);
        h.update(label.len().to_be_bytes());
        h.update(label.as_bytes());
        Self::update_with_serialized(&mut h, label, t);
        Self(h.finalize().into())
    }

    fn append_vec<T: CanonicalSerialize>(self, label: &str, v: &[T]) -> Self {
        let mut h = Sha256::new();
        h.update(self.0);
        h.update(label.len().to_be_bytes());
        h.update(label.as_bytes());

        for (i, t) in v.iter().enumerate() {
            h.update(i.to_be_bytes());
            Self::update_with_serialized(&mut h, label, t);
        }

        Self(h.finalize().into())
    }

    fn derive_challenge_scalars<const N: usize>(&self, dst: &[u8]) -> [Scalar; N] {
        <DefaultFieldHasher<Sha256> as HashToField<Scalar>>::new(dst).hash_to_field(&self.0)
    }
}

/// Secret key for a player. Memory is automatically wiped on drop for security.
///
/// The secret key is used to decrypt cards and create reveal tokens.
/// It must be kept private and never shared with other players.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
#[cfg_attr(test, derive(Debug))]
pub struct SecretKey(Scalar);

/// Wrapper type indicating that a value has been cryptographically verified.
///
/// This type provides compile-time guarantees that proofs have been checked
/// before sensitive operations. Values can only be constructed through
/// successful verification.
///
/// # Examples
///
/// ```
/// use ziffle::Shuffle;
/// # let mut rng = ark_std::test_rng();
/// # let shuffle = Shuffle::<10>::default();
/// # let ctx = b"game";
///
/// let (_, pk, proof) = shuffle.keygen(&mut rng, ctx);
///
/// // This creates a Verified<PublicKey>
/// let verified_pk = proof.verify(pk, ctx).unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verified<T>(T);

/// Public key for a player, derived from their secret key.
///
/// Public keys are shared with all players and used to encrypt cards.
/// The relationship `pk = sk · G` ensures that only the secret key holder
/// can decrypt cards encrypted with this public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicKey(CurveAffine);

/// Zero-knowledge proof of secret key ownership.
///
/// Demonstrates knowledge of the discrete logarithm (secret key) corresponding
/// to a public key without revealing the secret key itself. Uses a Schnorr-style
/// sigma protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OwnershipProof {
    a: CurveAffine, // commitment
    z: Scalar,      // response
}

impl OwnershipProof {
    const DLOG_DST: &[u8] = b"ziffle/DLOG/v1";

    fn challenge(pk: &PublicKey, a: &CurveAffine, ctx: &[u8]) -> Scalar {
        let [e] = Transcript::init(ctx)
            .append("pk", pk)
            .append("a", a)
            .derive_challenge_scalars(Self::DLOG_DST);
        e
    }

    fn new<R: Rng>(rng: &mut R, sk: &SecretKey, pk: PublicKey, ctx: &[u8]) -> Self {
        let w = Scalar::rand(rng);
        let a = (GENERATOR * w).into_affine();
        let e = Self::challenge(&pk, &a, ctx);
        let z = w + e * sk.0;
        OwnershipProof { a, z }
    }

    /// Verifies the ownership proof for a given public key.
    ///
    /// Returns `Some(Verified<PublicKey>)` if the proof is valid, demonstrating
    /// that the prover knows the secret key corresponding to the public key.
    ///
    /// # Arguments
    ///
    /// * `pk` - The public key to verify ownership of
    /// * `ctx` - Context string that was used when creating the proof
    ///
    /// # Returns
    ///
    /// `Some(Verified<PublicKey>)` if valid, `None` if the proof is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::Shuffle;
    /// # let mut rng = ark_std::test_rng();
    /// # let shuffle = Shuffle::<10>::default();
    /// # let ctx = b"game";
    ///
    /// let (_, pk, proof) = shuffle.keygen(&mut rng, ctx);
    ///
    /// // Verify the ownership proof
    /// match proof.verify(pk, ctx) {
    ///     Some(verified_pk) => println!("Valid ownership proof"),
    ///     None => println!("Invalid proof"),
    /// }
    /// ```
    #[must_use]
    pub fn verify(&self, pk: PublicKey, ctx: &[u8]) -> Option<Verified<PublicKey>> {
        let e = Self::challenge(&pk, &self.a, ctx);
        // check: z·G == a + e·pk
        let lhs = (GENERATOR.into_group() * self.z).into_affine();
        let rhs = (self.a.into_group() + (pk.0 * e)).into_affine();
        (lhs == rhs).then_some(Verified(pk))
    }
}

/// Aggregate public key combining all players' public keys.
///
/// The aggregate key is computed as the sum of all verified individual public keys.
/// Cards encrypted with this key require cooperation from all players to decrypt.
///
/// # Examples
///
/// ```
/// use ziffle::{Shuffle, AggregatePublicKey};
/// # let mut rng = ark_std::test_rng();
/// # let shuffle = Shuffle::<10>::default();
/// # let ctx = b"game";
///
/// let (_, pk1, proof1) = shuffle.keygen(&mut rng, ctx);
/// let vpk1 = proof1.verify(pk1, ctx).unwrap();
///
/// let (_, pk2, proof2) = shuffle.keygen(&mut rng, ctx);
/// let vpk2 = proof2.verify(pk2, ctx).unwrap();
///
/// // Create aggregate key from verified keys
/// let apk = AggregatePublicKey::new(&[vpk1, vpk2]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AggregatePublicKey(CurveAffine);

impl AggregatePublicKey {
    /// Creates an aggregate public key from verified individual public keys.
    ///
    /// All public keys must be verified before aggregation to ensure they
    /// come from players who proved ownership of their secret keys.
    ///
    /// # Arguments
    ///
    /// * `pks` - Slice of verified public keys from all players
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::{Shuffle, AggregatePublicKey};
    /// # let mut rng = ark_std::test_rng();
    /// # let shuffle = Shuffle::<10>::default();
    /// # let ctx = b"game";
    /// # let (_, pk1, proof1) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk1 = proof1.verify(pk1, ctx).unwrap();
    /// # let (_, pk2, proof2) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk2 = proof2.verify(pk2, ctx).unwrap();
    ///
    /// let apk = AggregatePublicKey::new(&[vpk1, vpk2]);
    /// ```
    #[must_use]
    pub fn new(pks: &[Verified<PublicKey>]) -> Self {
        let apk: CurveProj = pks.iter().map(|pk| pk.0.0).sum();
        Self(apk.into_affine())
    }
}

/// Zero-knowledge proof for a reveal token using a DLEQ (Discrete Log Equality) proof.
///
/// Proves that a reveal token was correctly computed as `sk · c1` without revealing
/// the secret key. Uses a Chaum-Pedersen style protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealTokenProof {
    t_g: CurveAffine,
    t_c1: CurveAffine,
    z: Scalar,
}

impl RevealTokenProof {
    const DLEQ_DST: &[u8] = b"ziffle/DLEQ/v1";

    fn challenge(
        pk: PublicKey,
        share: CurveAffine,
        c1: CurveAffine,
        t_g: CurveAffine,
        t_c1: CurveAffine,
        ctx: &[u8],
    ) -> Scalar {
        let [e]: [_; 1] = Transcript::init(ctx)
            .append("pk", &pk)
            .append("share", &share)
            .append("c1", &c1)
            .append("t_g", &t_g)
            .append("t_c1", &t_c1)
            .derive_challenge_scalars(Self::DLEQ_DST);
        e
    }

    fn new<R: Rng>(
        rng: &mut R,
        sk: Scalar,
        pk: PublicKey,
        share: CurveAffine,
        c1: CurveProj,
        ctx: &[u8],
    ) -> Self {
        let w = Scalar::rand(rng);
        let t_g = (GENERATOR * w).into_affine();
        let t_c1 = (c1 * w).into_affine();
        let e = Self::challenge(pk, share, c1.into_affine(), t_g, t_c1, ctx);
        let z = w - (e * sk);
        Self { t_g, t_c1, z }
    }

    /// Verifies that a reveal token was correctly computed for a specific card.
    ///
    /// # Arguments
    ///
    /// * `pk` - Verified public key of the player who created the token
    /// * `token` - The reveal token to verify
    /// * `card` - The masked card this token is for
    /// * `ctx` - Context string used when creating the token
    ///
    /// # Returns
    ///
    /// `Some(Verified<RevealToken>)` if valid, `None` otherwise.
    #[must_use]
    pub fn verify(
        &self,
        pk: Verified<PublicKey>,
        token: RevealToken,
        card: MaskedCard,
        ctx: &[u8],
    ) -> Option<Verified<RevealToken>> {
        let Verified(pk) = pk;
        let RevealToken(share) = token;
        let MaskedCard((c1, _)) = card;

        // Step 1: reproducde challenge scalar
        let e = Self::challenge(pk, share, c1, self.t_g, self.t_c1, ctx);

        // Step 2: chec t_g == g·z + pk·e
        if self.t_g != ((GENERATOR * self.z) + (pk.0.into_group() * e)).into_affine() {
            return None;
        }

        // Step 3: check t_c1 == c1·z + share·e
        if self.t_c1 != ((c1.into_group() * self.z) + (share.into_group() * e)).into_affine() {
            return None;
        }

        Some(Verified(token))
    }
}

/// Decryption share for a specific card from one player.
///
/// A reveal token is computed as `sk · c1` where `sk` is the player's secret key
/// and `c1` is part of the encrypted card. All players must provide valid reveal
/// tokens to decrypt a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealToken(CurveAffine);

/// Aggregate reveal token combining all players' reveal tokens.
///
/// Computed as the sum of all verified individual reveal tokens. Used to
/// decrypt a card by combining all players' decryption shares.
///
/// # Examples
///
/// ```
/// use ziffle::{Shuffle, AggregatePublicKey, AggregateRevealToken};
/// # let mut rng = ark_std::test_rng();
/// # let shuffle = Shuffle::<10>::default();
/// # let ctx = b"game";
/// # let (sk1, pk1, proof1) = shuffle.keygen(&mut rng, ctx);
/// # let vpk1 = proof1.verify(pk1, ctx).unwrap();
/// # let (sk2, pk2, proof2) = shuffle.keygen(&mut rng, ctx);
/// # let vpk2 = proof2.verify(pk2, ctx).unwrap();
/// # let apk = AggregatePublicKey::new(&[vpk1, vpk2]);
/// # let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
/// # let vdeck = shuffle.verify_initial_shuffle(apk, deck, proof, ctx).unwrap();
/// # let card = vdeck.get(0).unwrap();
/// # let (rt1, rt_proof1) = card.reveal_token(&mut rng, &sk1, pk1, ctx);
/// # let (rt2, rt_proof2) = card.reveal_token(&mut rng, &sk2, pk2, ctx);
///
/// // Aggregate verified reveal tokens
/// let art = AggregateRevealToken::new(&[
///     rt_proof1.verify(vpk1, rt1, card, ctx).unwrap(),
///     rt_proof2.verify(vpk2, rt2, card, ctx).unwrap(),
/// ]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateRevealToken(CurveAffine);

impl AggregateRevealToken {
    /// Creates an aggregate reveal token from verified individual tokens.
    ///
    /// All reveal tokens must be verified before aggregation.
    ///
    /// # Arguments
    ///
    /// * `pks` - Slice of verified reveal tokens from all players
    #[must_use]
    pub fn new(pks: &[Verified<RevealToken>]) -> Self {
        let art: CurveProj = pks.iter().map(|t| t.0.0.into_group()).sum();
        Self(art.into_affine())
    }
}

/// An encrypted card from the deck.
///
/// Cards are encrypted using ElGamal encryption with the aggregate public key.
/// To reveal a card, all players must provide reveal tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskedCard(Ciphertext);

impl MaskedCard {
    /// Creates a reveal token and proof for this card.
    ///
    /// Each player uses their secret key to create a partial decryption of the card.
    /// The proof demonstrates that the token was computed correctly without revealing
    /// the secret key.
    ///
    /// # Arguments
    ///
    /// * `rng` - Cryptographically secure random number generator
    /// * `sk` - Player's secret key
    /// * `pk` - Player's public key
    /// * `ctx` - Context string to bind the proof
    ///
    /// # Returns
    ///
    /// A tuple of `(RevealToken, RevealTokenProof)` that other players can verify.
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::{Shuffle, AggregatePublicKey};
    /// # let mut rng = ark_std::test_rng();
    /// # let shuffle = Shuffle::<10>::default();
    /// # let ctx = b"game";
    /// # let (sk, pk, proof) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk = proof.verify(pk, ctx).unwrap();
    /// # let apk = AggregatePublicKey::new(&[vpk]);
    /// # let (deck, shuf_proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    /// # let vdeck = shuffle.verify_initial_shuffle(apk, deck, shuf_proof, ctx).unwrap();
    ///
    /// let card = vdeck.get(0).unwrap();
    /// let (token, proof) = card.reveal_token(&mut rng, &sk, pk, ctx);
    ///
    /// // Other players verify the token
    /// let verified_token = proof.verify(vpk, token, card, ctx).unwrap();
    /// ```
    pub fn reveal_token<R: Rng>(
        &self,
        rng: &mut R,
        sk: &SecretKey,
        pk: PublicKey,
        ctx: &[u8],
    ) -> (RevealToken, RevealTokenProof) {
        let SecretKey(sk) = sk;
        let c1 = self.0.0.into_group();
        let share = (c1 * sk).into_affine();
        let proof = RevealTokenProof::new(rng, *sk, pk, share, c1, ctx);
        (RevealToken(share), proof)
    }
}

/// A deck of `N` encrypted cards.
///
/// The deck is represented as an array of ElGamal ciphertexts. Cards can only
/// be accessed from verified decks (obtained after successful shuffle verification).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskedDeck<const N: usize>([Ciphertext; N]);

impl<const N: usize> Verified<MaskedDeck<N>> {
    /// Extract an authenticated ciphertext without granting a way to deserialize
    /// or construct the verification marker. Its manifest identity stays hidden.
    pub fn verified_card(&self, idx: usize) -> Option<Verified<MaskedCard>> {
        self.get(idx).map(Verified)
    }

    /// Compose a differently sized deck exclusively from authenticated cards.
    /// The caller must additionally enforce its game-specific membership and
    /// single-use rules, as ciphertexts from different reshuffle epochs cannot
    /// be compared for plaintext equality without revealing their identities.
    pub fn from_verified_cards(cards: &[Verified<MaskedCard>]) -> Option<Self> {
        if cards.len() != N {
            return None;
        }
        if cards.iter().enumerate().any(|(index, card)| cards[..index].contains(card)) {
            return None;
        }
        Some(Verified(MaskedDeck(array::from_fn(|index| cards[index].0.0))))
    }

    /// Gets a card from the verified deck by index.
    ///
    /// # Arguments
    ///
    /// * `idx` - Zero-based index of the card (0 to N-1)
    ///
    /// # Returns
    ///
    /// `Some(MaskedCard)` if the index is valid, `None` if out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::{Shuffle, AggregatePublicKey};
    /// # let mut rng = ark_std::test_rng();
    /// # let shuffle = Shuffle::<10>::default();
    /// # let ctx = b"game";
    /// # let (_, pk, proof) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk = proof.verify(pk, ctx).unwrap();
    /// # let apk = AggregatePublicKey::new(&[vpk]);
    /// # let (deck, shuf_proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    /// # let vdeck = shuffle.verify_initial_shuffle(apk, deck, shuf_proof, ctx).unwrap();
    ///
    /// // Access cards from the verified deck
    /// let first_card = vdeck.get(0).unwrap();
    /// let last_card = vdeck.get(9).unwrap();
    ///
    /// // Out of bounds returns None
    /// assert!(vdeck.get(10).is_none());
    /// ```
    pub fn get(&self, idx: usize) -> Option<MaskedCard> {
        self.0.0.get(idx).copied().map(MaskedCard)
    }
}

macro_rules! usize_to_u64 {
    ($i:expr) => {
        u64::try_from($i).expect("usize <= 64 bits")
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MultiExpArg<const N: usize> {
    // commitments
    c_alpha: PedersonCommitment,
    c_beta: PedersonCommitment,
    // pre-committed ciphertexts
    ct_mxp0: (CurveAffine, CurveAffine),
    ct_mxp1: (CurveAffine, CurveAffine),
    // response
    o_alpha: [Scalar; N],
    o_r: Scalar,
    beta: Scalar,
    o_beta: Scalar,
    tau: Scalar,
}

// Multi-scalar “product of powers” over a vector of ElGamal ciphertexts, i.e. 𝒞 = (𝒞_1, 𝒞_2), and matching scalars:
// ∏[𝒞(i)·𝓈(i)] ==> (Σ 𝒞_1[i]·𝓈[i], Σ 𝒞_2[i]·𝓈[i]) for all i in [0; N-1]
macro_rules! ct_mspp {
    ($cts:expr, $ss:expr) => {
        $cts.iter()
            .zip($ss)
            .map(|((c1, c2), s)| (c1.into_group() * s, c2.into_group() * s))
            .reduce(|(c1_acc, c2_acc), (c1, c2)| (c1_acc + c1, c2_acc + c2))
            .expect("N > 0")
    };
}

struct ProveMultiExpArgInputs<'a, const N: usize> {
    ck: &'a PedersonCommitKey<N>,
    apk: AggregatePublicKey,
    xpi: &'a [Scalar; N],
    w_xpi: PedersonWitness,
    next: &'a [Ciphertext; N],
    rho: &'a [Scalar; N],
    ts: Transcript,
}

struct VerifyMultiExpArgInputs<'a, const N: usize> {
    ck: &'a PedersonCommitKey<N>,
    apk: AggregatePublicKey,
    prev: &'a [Ciphertext; N],
    next: &'a [Ciphertext; N],
    x_base: Scalar,
    c_xpi: PedersonCommitment,
    ts: Transcript,
}

impl<const N: usize> MultiExpArg<N> {
    const X_DST: &[u8] = b"ziffle/BG12MultiExpArgX/v1";

    fn challenge_x(
        ts: Transcript,
        c_alpha: PedersonCommitment,
        c_beta: PedersonCommitment,
        ct_mxp0: Ciphertext,
        ct_mxp1: Ciphertext,
    ) -> Scalar {
        let [x] = ts
            .append("c_alpha", &c_alpha)
            .append("c_beta", &c_beta)
            .append("ct_mxp0", &ct_mxp0)
            .append("ct_mxp1", &ct_mxp1)
            .derive_challenge_scalars(Self::X_DST);
        x
    }

    fn new<R: Rng>(
        rng: &mut R,
        ProveMultiExpArgInputs {
            ck,
            apk: AggregatePublicKey(pk),
            xpi,
            w_xpi: PedersonWitness(w_xpi),
            next,
            rho,
            ts,
        }: ProveMultiExpArgInputs<N>,
    ) -> Self {
        // Step 1: Then we sample a random scalar vector ɑ and scalar β
        // (`alpha` and `beta` resp.) and commit to them
        let alpha: [Scalar; N] = array::from_fn(|_| Scalar::rand(rng));
        let beta = Scalar::rand(rng);
        let (c_alpha, PedersonWitness(w_alpha)) = ck.vector_commit(rng, &alpha);
        let (c_beta, PedersonWitness(w_beta)) = ck.commit(rng, beta);

        // Step 2: we create two ciphertexts, blinding 𝒞mxp0 and anchoring 𝒞mxp1 (`ct_mxp{0,1}`).
        // 𝒞mxp0 = ℰ(G·β; τ0)·∏[𝒞'(i)·ɑ(i)] for all i in [0; N-1] where τ0 (`tau0`) is a random scalar.
        let tau0 = Scalar::rand(rng);
        let ct_mxp0 = {
            let (c1, c2) = ct_mspp!(next, alpha);
            (
                ((GENERATOR * tau0) + c1).into_affine(),
                ((GENERATOR * beta) + (pk * tau0) + c2).into_affine(),
            )
        };
        // 𝒞mxp1 = ℰ(1; ρ_agg)·∏[𝒞'(i)·x^π(i)] for all i in [0; N-1]
        // where ρ_agg = -𝛴[ρ(i)·x^π(i)] for all i in [0; N-1]
        let rho_agg: Scalar = -(0..N).map(|i| rho[i] * xpi[i]).sum::<Scalar>();
        let ct_mxp1 = {
            let (c1, c2) = ct_mspp!(next, xpi);
            (
                ((GENERATOR * rho_agg) + c1).into_affine(),
                ((pk * rho_agg) + c2).into_affine(),
            )
        };

        // Step 3: add the commitments and ciphertexts to the transcript and derive a challenge scalar x
        let x = Self::challenge_x(ts, c_alpha, c_beta, ct_mxp0, ct_mxp1);

        // Step 4: compute the openings
        // use the challenge to compute the following witness openings (𝒪) to reveal:
        // 𝒪ɑ = [ɑ(i) + x·x^π(i)] for all i in [0; N-1]
        let o_alpha: [Scalar; N] = array::from_fn(|i| alpha[i] + (x * xpi[i]));
        // 𝒪r = 𝒲 ɑ + x_mxp·𝒲 x^π where 𝒲 ɑ and 𝒲 x^π are the witnesses to commitments to ɑ and x^π
        let o_r: Scalar = w_alpha + (x * w_xpi);
        // τ (`tau`) = τ0 + x·ρ_agg
        let tau = tau0 + (x * rho_agg);

        Self {
            c_alpha,
            c_beta,
            ct_mxp0,
            ct_mxp1,
            o_alpha,
            o_r,
            beta,
            o_beta: w_beta,
            tau,
        }
    }

    #[must_use]
    fn verify(
        &self,
        VerifyMultiExpArgInputs {
            ck,
            apk: AggregatePublicKey(pk),
            prev,
            next,
            x_base,
            c_xpi: PedersonCommitment(c_xpi),
            ts,
        }: VerifyMultiExpArgInputs<N>,
    ) -> bool {
        // Step 1: derive the challenge scalar x_mxp
        let x = Self::challenge_x(ts, self.c_alpha, self.c_beta, self.ct_mxp0, self.ct_mxp1);

        // Step 2: check that ∏[𝒞(i)·x^(i + 1)] for all i in [0; N-1] == 𝒞mxp1
        let check1 = || {
            let xs: [Scalar; N] = array::from_fn(|i| x_base.pow([usize_to_u64!(i + 1)]));
            let (prod_c1, prod_c2) = ct_mspp!(prev, xs);
            let (ct_mxp1_c1, ct_mxp1_c2) = self.ct_mxp1;
            prod_c1.into_affine() == ct_mxp1_c1 && prod_c2.into_affine() == ct_mxp1_c2
        };

        // Step 3: check x·comm(x^π) + comm(ɑ) == comm(𝒪ɑ; 𝒪r)
        let check2 = || {
            let lhs = (c_xpi.into_group() * x) + self.c_alpha.0.into_group();
            let rhs = ck.vector_commit_with_r(&self.o_alpha, self.o_r);
            lhs.into_affine() == rhs.into_affine()
        };

        // Step 4: check that comm(β) == comm(β; 𝒪β)
        let check3 = || self.c_beta.0 == ck.commit_with_r(self.beta, self.o_beta);

        // Step 5: check that 𝒞mxp0·𝒞mxp1^x == ℰ(G·β; 𝒪τ)·∏[𝒞'(i)·ɑ(i)] for all i in [0; N-1]
        let check4 = || {
            let (ct_mxp0_c1, ct_mxp0_c2) = self.ct_mxp0;
            let (ct_mxp1_c1, ct_mxp1_c2) = self.ct_mxp1;
            let lhs_c1 = ct_mxp0_c1.into_group() + (ct_mxp1_c1.into_group() * x);
            let lhs_c2 = ct_mxp0_c2.into_group() + (ct_mxp1_c2.into_group() * x);
            let (prod_c1, prod_c2) = ct_mspp!(next, self.o_alpha);
            let rhs_c1 = (GENERATOR * self.tau) + prod_c1;
            let rhs_c2 = (GENERATOR * self.beta) + (pk * self.tau) + prod_c2;
            lhs_c1.into_affine() == rhs_c1.into_affine()
                && lhs_c2.into_affine() == rhs_c2.into_affine()
        };

        check1() && check2() && check3() && check4()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SingleValueProductArg<const N: usize> {
    // commitments
    c_d: PedersonCommitment,
    c_sdelta: PedersonCommitment,
    c_cdelta: PedersonCommitment,
    // response
    a_tilde: [Scalar; N],
    b_tilde: [Scalar; N],
    r_tilde: Scalar,
    s_tilde: Scalar,
}

struct ProveSvpArgInputs<'a, const N: usize> {
    ck: &'a PedersonCommitKey<N>,
    y: Scalar,
    z: Scalar,
    pi: &'a [Scalar; N],
    xpi: &'a [Scalar; N],
    w_pi: PedersonWitness,
    w_xpi: PedersonWitness,
    ts: Transcript,
}

struct VerifySvpArgInputs<'a, const N: usize> {
    ck: &'a PedersonCommitKey<N>,
    x_base: Scalar,
    y: Scalar,
    z: Scalar,
    c_pi: PedersonCommitment,
    c_xpi: PedersonCommitment,
    ts: Transcript,
}

impl<const N: usize> SingleValueProductArg<N> {
    const X_DST: &[u8] = b"ziffle/BG12ProductArgX/v1";

    fn challenge_x(
        ts: Transcript,
        c_d: PedersonCommitment,
        c_sdelta: PedersonCommitment,
        c_cdelta: PedersonCommitment,
    ) -> Scalar {
        let [x] = ts
            .append("c_d", &c_d)
            .append("c_sdelta", &c_sdelta)
            .append("c_cdelta", &c_cdelta)
            .derive_challenge_scalars(Self::X_DST);
        x
    }

    fn new<R: Rng>(
        rng: &mut R,
        ProveSvpArgInputs {
            ck,
            y,
            z,
            pi,
            xpi,
            w_pi: PedersonWitness(w_pi),
            w_xpi: PedersonWitness(w_xpi),
            ts,
        }: ProveSvpArgInputs<N>,
    ) -> Self {
        // Step 1: sample a random scalar vector d and commit to it
        let d: [Scalar; N] = array::from_fn(|_| Scalar::rand(rng));
        let (c_d, PedersonWitness(w_d)) = ck.vector_commit(rng, &d);

        // Step 2: create semi-random scalar vector δ (small delta => `sdelta`)
        // NOTE: semi-random because δ[0] == d[0] & δ[N-1] == 0
        let mut sdelta: [Scalar; N] = [Scalar::zero(); N];
        sdelta[0] = d[0];
        (1..N - 1).for_each(|i| sdelta[i] = Scalar::rand(rng));
        // Compute [-δ(i)·d(i + 1)] for all i in [0; N-2] and commit to it
        let (c_sdelta, PedersonWitness(w_sdelta)) = {
            // NOTE: we have to use a vec of length N because const generic expressions require nightly
            // Inititalizing the vector with 0 means the last element has no effect when committing.
            let mut v = [Scalar::zero(); N];
            (0..N - 1).for_each(|i| v[i] = -sdelta[i] * d[i + 1]);
            ck.vector_commit(rng, &v)
        };

        // Step 3: Compute & commit to 𝛥 (capital delta => `cdelta`) over the vector of:
        // [δ(i + 1) − a(i + 1)·δ(i) − b(i)·d(i + 1)] for all i in [0; N-2] where:
        // a = [y·π(i) + x^π(i) - z]
        // b = [a0, b0·a1, ..., bN-2·aN-1] for all i in [0; N-1], i.e. product progression of a
        let a: [Scalar; N] = array::from_fn(|i| (y * pi[i]) + xpi[i] - z);
        let mut b = [Scalar::ONE; N];
        b[0] = a[0];
        (1..N).for_each(|i| b[i] = b[i - 1] * a[i]);
        let (c_cdelta, PedersonWitness(w_cdelta)) = {
            let mut v = [Scalar::zero(); N];
            (0..N - 1)
                .for_each(|i| v[i] = sdelta[i + 1] - (a[i + 1] * sdelta[i]) - (b[i] * d[i + 1]));
            ck.vector_commit(rng, &v)
        };

        // Step 4: add the commitments to the transcript and derive a challenge scalar x
        let x = Self::challenge_x(ts, c_d, c_sdelta, c_cdelta);

        // Step 5: compute the responses
        // a~ = [x·a(i) + d(i)] for all i in [0; N-1]
        let a_tilde: [Scalar; N] = array::from_fn(|i| (x * a[i]) + d[i]);
        // b~ = [x·b(i) + δ(i)] for all i in [0; N-1]
        let b_tilde: [Scalar; N] = array::from_fn(|i| (x * b[i]) + sdelta[i]);
        // r~ = x·𝒲 a + 𝒲 d where the witness 𝒲 a can be computed from y·𝒲 π + 𝒲 x^π
        let w_a = (y * w_pi) + w_xpi;
        let r_tilde: Scalar = (x * w_a) + w_d;
        // s~ = x·𝒲 𝛥 + 𝒲 δ
        let s_tilde: Scalar = (x * w_cdelta) + w_sdelta;

        Self {
            c_d,
            c_sdelta,
            c_cdelta,
            a_tilde,
            b_tilde,
            r_tilde,
            s_tilde,
        }
    }

    #[must_use]
    fn verify(
        &self,
        VerifySvpArgInputs {
            ck,
            x_base,
            y,
            z,
            c_pi: PedersonCommitment(c_pi),
            c_xpi: PedersonCommitment(c_xpi),
            ts,
        }: VerifySvpArgInputs<N>,
    ) -> bool {
        // Step 1: derive the challenge scalar x
        let x = Self::challenge_x(ts, self.c_d, self.c_sdelta, self.c_cdelta);

        // Step 2: compute the constant vector commit comm([-z; N], 0)
        let c_mz = ck.vector_commit_with_r(&[-z; N], Scalar::zero());

        // Step 3: homomorphically compute comm(a) = comm(y·π(i) + x^π(i)) for all i in [1; N]
        let c_a = (c_pi.into_group() * y) + c_xpi.into_group();

        // Step 4: check comm(d) + (comm(a) + comm(-z))·x == comm(a~, r~)
        let check1 = || {
            let c_d_dmz = self.c_d.0.into_group() + ((c_a + c_mz) * x);
            let c_a_tilde_r_tilde = ck.vector_commit_with_r(&self.a_tilde, self.r_tilde);
            c_d_dmz.into_affine() == c_a_tilde_r_tilde.into_affine()
        };

        // Step 5: check comm(δ) + comm(𝛥)·x == comm([x·b~(i + 1) - b~(i)·a~(i + 2)] for all i in [0; N - 2]; s~)
        let check2 = || {
            let c_sdelta_cdelta = self.c_sdelta.0.into_group() + (self.c_cdelta.0.into_group() * x);
            // we have to use a vec of length N because const generic expressions require nightly
            // inititalizing the vector with 0 means the last element has no effect when committing
            let mut v = [Scalar::zero(); N];
            (0..N - 1).for_each(|i| {
                v[i] = (x * self.b_tilde[i + 1]) - (self.b_tilde[i] * self.a_tilde[i + 1]);
            });
            let c_a_tilde_b_tilde = ck.vector_commit_with_r(&v, self.s_tilde);
            c_sdelta_cdelta.into_affine() == c_a_tilde_b_tilde.into_affine()
        };

        // Step 6: check b~[0] == a~[0]
        let check3 = || self.b_tilde[0] == self.a_tilde[0];

        // Step 7: check b~[N - 1] == x · ∏[y·i + x^i - z] for i in [1; N]
        let check4 = || {
            let public_prod: Scalar = (1..=N)
                .map(|i| {
                    (y * Scalar::new(usize_to_u64!(i).into())) + (x_base.pow([usize_to_u64!(i)]))
                        - z
                })
                .product();
            self.b_tilde[N - 1] == (x * public_prod)
        };

        check1() && check2() && check3() && check4()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// # BayerGroth 2012 (BG12) Efficient Zero-Knowledge Argument for Correctness of a Shuffle
///
/// <http://www0.cs.ucl.ac.uk/staff/J.Groth/MinimalShuffle.pdf>
///
/// The prover must convince the verifier that they know a hidden permutation 𝜋
/// and a random witness ρ such that the output ciphertext 𝒞' is the
/// corresponding input ciphertext 𝒞 but re-ordered and re-masked by adding ElGamal
/// encryption function ℰ over message 1 with randomness ρ(i).
///
/// i.e. 𝒞'\[i\] = 𝒞\[𝜋(i)\]·ℰ(1; ρ(i)) for all i in \[0; N-1\]
///
/// # Verification
///
/// To obtain a `Verified<MaskedDeck<N>>` from a shuffle proof, use:
/// - [`Shuffle::verify_initial_shuffle`] for the first shuffle
/// - [`Shuffle::verify_shuffle`] for subsequent shuffles
pub struct ShuffleProof<const N: usize> {
    // commitment to permutations
    c_pi: PedersonCommitment,
    // commitment to vector[x^p[i]] where p are permutations and x is a fiat-shamir challenge
    c_xpi: PedersonCommitment,
    // multi-exponentiation argument of knowledge
    mexp_arg: MultiExpArg<N>,
    // product argument of knowledge
    prod_arg: SingleValueProductArg<N>,
}

struct ShuffleProofInputs<'a, const N: usize> {
    ck: &'a PedersonCommitKey<N>,
    apk: AggregatePublicKey,
    perm: &'a [usize; N],
    prev: &'a [Ciphertext; N],
    next: &'a [Ciphertext; N],
    rho: &'a [Scalar; N],
    ctx: &'a [u8],
}

// NOTE: in the paper, N is split into m rows for proof-size optimization, however for simplicity
// we construct the proof over 1 row of N ciphertexts, i.e. we set m = 1.
//
// Also the variable/field naming attempts to follow the paper as closely as possible to make
// cross-referencing the implementation to the paper easier.
// 𝜋 = `pi`
// ρ = `rho`
// 𝒞 = `prev`
// 𝒞' = `next`
impl<const N: usize> ShuffleProof<N> {
    const X_DST: &[u8] = b"ziffle/BG12X/v1";
    const YZ_DST: &[u8] = b"ziffle/BG12YZ/v1";

    fn challenge_x(
        apk: AggregatePublicKey,
        prev: &[Ciphertext; N],
        next: &[Ciphertext; N],
        c_pi: PedersonCommitment,
        ctx: &[u8],
    ) -> (Transcript, Scalar) {
        let ts = Transcript::init(ctx)
            .append("apk", &apk)
            .append_vec("prev", prev)
            .append_vec("next", next)
            .append("c_pi", &c_pi);
        let [x] = ts.derive_challenge_scalars(Self::X_DST);
        (ts, x)
    }

    fn challenge_yz(ts: Transcript, c_xpi: PedersonCommitment) -> (Transcript, Scalar, Scalar) {
        let ts = ts.append("c_xpi", &c_xpi);
        let [y, z] = ts.derive_challenge_scalars(Self::YZ_DST);
        (ts, y, z)
    }

    fn new<R: Rng>(
        rng: &mut R,
        ShuffleProofInputs {
            ck,
            apk,
            perm,
            prev,
            next,
            rho,
            ctx,
        }: ShuffleProofInputs<N>,
    ) -> Self {
        // Step 1: convert permutation to 1-based scalars and commit to it
        let pi: [_; N] = array::from_fn(|i| Scalar::new(usize_to_u64!(perm[i] + 1).into()));
        let (c_pi, w_pi) = ck.vector_commit(rng, &pi);

        // Step 2: setup the initial transcript and derive challenge x
        let (ts, x) = Self::challenge_x(apk, prev, next, c_pi, ctx);

        // Step 3: compute and commit to [x^{π(i) + 1}] for all i in [0; N-1]
        // NOTE: the paper uses 1-based indices but we use 0-based, to avoid x⁰ = 1 we add 1 to π(i)
        let xpi: [Scalar; N] = array::from_fn(|i| x.pow([usize_to_u64!(perm[i]) + 1]));
        let (c_xpi, w_xpi) = ck.vector_commit(rng, &xpi);

        // Step 4: update the transcipt with the commitment to c_xpi and derive a challenge scalars y & z
        let (ts, y, z) = Self::challenge_yz(ts, c_xpi);

        // Step 5: compute the multi-exponentiation argument of knowledge
        // This proves that every element of 𝒞' is an element of 𝒞 re-masked with ℰ(1; ρ(i)).
        // NOTE: The transcript is forked here.
        let mexp_arg = MultiExpArg::new(
            rng,
            ProveMultiExpArgInputs {
                ck,
                apk,
                xpi: &xpi,
                w_xpi,
                next,
                rho,
                ts: ts.clone(),
            },
        );

        // Step 6: compute the product argument of knowledge
        // This proves that the permutation is valid, i.e. all indexes 0..N-1 are present
        // NOTE: As we set m = 1, only the Single Value Product Argument protocol (Section 5.3) is required.
        let prod_arg = SingleValueProductArg::new(
            rng,
            ProveSvpArgInputs {
                ck,
                y,
                z,
                pi: &pi,
                xpi: &xpi,
                w_pi,
                w_xpi,
                ts,
            },
        );

        Self {
            c_pi,
            c_xpi,
            mexp_arg,
            prod_arg,
        }
    }

    #[must_use]
    fn verify(
        &self,
        ck: &PedersonCommitKey<N>,
        apk: AggregatePublicKey,
        prev: &[Ciphertext; N],
        next: &[Ciphertext; N],
        ctx: &[u8],
    ) -> Option<Verified<MaskedDeck<N>>> {
        let (ts, x) = Self::challenge_x(apk, prev, next, self.c_pi, ctx);
        let (ts, y, z) = Self::challenge_yz(ts, self.c_xpi);

        if !self.mexp_arg.verify(VerifyMultiExpArgInputs {
            ck,
            apk,
            prev,
            next,
            x_base: x,
            c_xpi: self.c_xpi,
            ts: ts.clone(),
        }) {
            return None;
        }

        if !self.prod_arg.verify(VerifySvpArgInputs {
            ck,
            x_base: x,
            y,
            z,
            c_pi: self.c_pi,
            c_xpi: self.c_xpi,
            ts,
        }) {
            return None;
        }

        Some(Verified(MaskedDeck(*next)))
    }
}

/// Build a deterministic "open" deck of `N` plaintext cards using a PRNG seeded with a SHA256 hash.
/// Each card is `s_i · G` where `s_i` is derived deterministically.
fn open_deck<const N: usize>() -> [CurveAffine; N] {
    let mut drng = StdRng::from_seed(Sha256::digest(OPEN_CARD_PRNG_SEED).into());
    array::from_fn(|_| (GENERATOR * Scalar::rand(&mut drng)).into_affine())
}

/// applies the differential update
/// (c1, c2) <- (c1 + r·G, c2 + r·pk) with fresh r.
fn remask_card<R: Rng>(
    rng: &mut R,
    AggregatePublicKey(pk): AggregatePublicKey,
    (c1, c2): Ciphertext,
) -> (Ciphertext, Scalar) {
    let r = Scalar::rand(rng);
    let c1 = c1 + GENERATOR * r;
    let c2 = c2 + (pk * r);
    ((c1.into_affine(), c2.into_affine()), r)
}

fn shuffle_remask_prove<const N: usize, R: Rng>(
    rng: &mut R,
    ck: &PedersonCommitKey<N>,
    apk: AggregatePublicKey,
    prev: &[Ciphertext; N],
    ctx: &[u8],
) -> (MaskedDeck<N>, ShuffleProof<N>) {
    let mut perm: [usize; N] = array::from_fn(|idx| idx);
    perm.as_mut_slice().shuffle(rng);

    let next = &mut [(CurveAffine::identity(), CurveAffine::identity()); N];
    // remasked randomness witness vector
    let rho = &mut [Scalar::zero(); N];
    (0..N).for_each(|i| {
        let (c, r) = remask_card(rng, apk, prev[perm[i]]);
        next[i] = c;
        rho[i] = r;
    });

    let proof = ShuffleProof::new(
        rng,
        ShuffleProofInputs {
            ck,
            apk,
            perm: &perm,
            prev,
            next,
            rho,
            ctx,
        },
    );

    let Verified(next) = proof
        .verify(ck, apk, prev, next, ctx)
        .expect("invalid shuffle proof");

    (next, proof)
}

/// Mental poker shuffle protocol for `N` cards using Bayer-Groth 2012 shuffle proofs.
///
/// This struct provides a complete implementation of a mental poker protocol where
/// multiple players can collaboratively shuffle a deck of cards without any player
/// knowing the order, then reveal individual cards with cryptographic proofs.
///
/// # Type Parameters
///
/// * `N` - The number of cards in the deck (must be > 1)
///
/// # Examples
///
/// ```
/// use ziffle::Shuffle;
/// # use ziffle::AggregatePublicKey;
/// # let mut rng = ark_std::test_rng();
///
/// // Create a 52-card deck
/// let shuffle = Shuffle::<52>::default();
///
/// let ctx = b"my_poker_game";
///
/// // Player 1 generates keys
/// let (sk1, pk1, proof1) = shuffle.keygen(&mut rng, ctx);
/// let vpk1 = proof1.verify(pk1, ctx).unwrap();
///
/// // Player 2 generates keys
/// let (sk2, pk2, proof2) = shuffle.keygen(&mut rng, ctx);
/// let vpk2 = proof2.verify(pk2, ctx).unwrap();
///
/// // Create aggregate public key
/// let apk = AggregatePublicKey::new(&[vpk1, vpk2]);
///
/// // Player 1 shuffles the initial deck
/// let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
/// let vdeck = shuffle.verify_initial_shuffle(apk, deck, proof, ctx).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Shuffle<const N: usize> {
    commit_key: PedersonCommitKey<N>,
    open_deck: [CurveAffine; N],
}

impl<const N: usize> Default for Shuffle<N> {
    fn default() -> Self {
        Self {
            commit_key: PedersonCommitKey::default(),
            open_deck: open_deck(),
        }
    }
}

impl<const N: usize> Shuffle<N> {
    const _N_GREATER_THAN_1: () = assert!(N > 1);

    fn initial_deck(&self) -> [Ciphertext; N] {
        array::from_fn(|i| (CurveAffine::identity(), self.open_deck[i]))
    }

    /// Generates a new keypair and ownership proof for a player.
    ///
    /// Each player must generate their own secret key, public key, and proof of ownership.
    /// The ownership proof demonstrates knowledge of the secret key without revealing it.
    ///
    /// # Arguments
    ///
    /// * `rng` - Cryptographically secure random number generator
    /// * `ctx` - Context string to bind the proof (e.g., game session ID)
    ///
    /// # Returns
    ///
    /// A tuple of `(SecretKey, PublicKey, OwnershipProof)` where the proof can be
    /// verified by other players to obtain a `Verified<PublicKey>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::Shuffle;
    /// # let mut rng = ark_std::test_rng();
    ///
    /// let shuffle = Shuffle::<52>::default();
    /// let ctx = b"game_session_123";
    ///
    /// let (secret_key, public_key, ownership_proof) = shuffle.keygen(&mut rng, ctx);
    ///
    /// // Verify the ownership proof
    /// let verified_pk = ownership_proof.verify(public_key, ctx).unwrap();
    /// ```
    #[must_use]
    pub fn keygen<R: Rng>(
        &self,
        rng: &mut R,
        ctx: &[u8],
    ) -> (SecretKey, PublicKey, OwnershipProof) {
        let sk = Scalar::rand(rng);
        let pk = GENERATOR * sk;
        let (sk, pk) = (SecretKey(sk), PublicKey(pk.into_affine()));
        let proof = OwnershipProof::new(rng, &sk, pk, ctx);
        (sk, pk, proof)
    }

    /// Performs the first shuffle of the deck with a zero-knowledge proof.
    ///
    /// The first player shuffles the initial (unencrypted) deck and encrypts it with
    /// the aggregate public key. A zero-knowledge proof is generated to demonstrate
    /// that the shuffle was performed correctly without revealing the permutation.
    ///
    /// # Arguments
    ///
    /// * `rng` - Cryptographically secure random number generator
    /// * `apk` - Aggregate public key from all players
    /// * `ctx` - Context string to bind the proof
    ///
    /// # Returns
    ///
    /// A tuple of `(MaskedDeck, ShuffleProof)` that must be verified by all players
    /// using [`verify_initial_shuffle`](Self::verify_initial_shuffle).
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::{Shuffle, AggregatePublicKey};
    /// # let mut rng = ark_std::test_rng();
    ///
    /// let shuffle = Shuffle::<10>::default();
    /// let ctx = b"game_session";
    ///
    /// // Two players generate keys
    /// let (_, pk1, proof1) = shuffle.keygen(&mut rng, ctx);
    /// let vpk1 = proof1.verify(pk1, ctx).unwrap();
    ///
    /// let (_, pk2, proof2) = shuffle.keygen(&mut rng, ctx);
    /// let vpk2 = proof2.verify(pk2, ctx).unwrap();
    ///
    /// let apk = AggregatePublicKey::new(&[vpk1, vpk2]);
    ///
    /// // First player shuffles
    /// let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    ///
    /// // All players verify
    /// let verified_deck = shuffle.verify_initial_shuffle(apk, deck, proof, ctx).unwrap();
    /// ```
    #[must_use]
    pub fn shuffle_initial_deck<R: Rng>(
        &self,
        rng: &mut R,
        apk: AggregatePublicKey,
        ctx: &[u8],
    ) -> (MaskedDeck<N>, ShuffleProof<N>) {
        shuffle_remask_prove(rng, &self.commit_key, apk, &self.initial_deck(), ctx)
    }

    /// Shuffles an already-shuffled deck with a zero-knowledge proof.
    ///
    /// Subsequent players can shuffle the deck again to add their own randomness.
    /// Each shuffle operation re-encrypts and permutes the deck while generating
    /// a proof that the shuffle was performed correctly.
    ///
    /// # Arguments
    ///
    /// * `rng` - Cryptographically secure random number generator
    /// * `apk` - Aggregate public key from all players
    /// * `prev` - Previously verified deck state
    /// * `ctx` - Context string to bind the proof
    ///
    /// # Returns
    ///
    /// A tuple of `(MaskedDeck, ShuffleProof)` that must be verified by all players
    /// using [`verify_shuffle`](Self::verify_shuffle).
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::{Shuffle, AggregatePublicKey};
    /// # let mut rng = ark_std::test_rng();
    /// # let shuffle = Shuffle::<10>::default();
    /// # let ctx = b"game";
    /// # let (_, pk1, proof1) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk1 = proof1.verify(pk1, ctx).unwrap();
    /// # let (_, pk2, proof2) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk2 = proof2.verify(pk2, ctx).unwrap();
    /// # let apk = AggregatePublicKey::new(&[vpk1, vpk2]);
    /// # let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    /// # let vdeck = shuffle.verify_initial_shuffle(apk, deck, proof, ctx).unwrap();
    ///
    /// // Player 2 shuffles the already-shuffled deck
    /// let (deck2, proof2) = shuffle.shuffle_deck(&mut rng, apk, &vdeck, ctx);
    /// let vdeck2 = shuffle.verify_shuffle(apk, &vdeck, deck2, proof2, ctx).unwrap();
    /// ```
    #[must_use]
    pub fn shuffle_deck<R: Rng>(
        &self,
        rng: &mut R,
        apk: AggregatePublicKey,
        prev: &Verified<MaskedDeck<N>>,
        ctx: &[u8],
    ) -> (MaskedDeck<N>, ShuffleProof<N>) {
        shuffle_remask_prove(rng, &self.commit_key, apk, &prev.0.0, ctx)
    }

    /// Verifies the initial shuffle proof and returns a verified deck.
    ///
    /// All players must verify the initial shuffle before proceeding. Verification
    /// ensures that the deck was shuffled correctly and all cards are present.
    ///
    /// # Arguments
    ///
    /// * `apk` - Aggregate public key from all players
    /// * `next` - The shuffled deck to verify
    /// * `proof` - Zero-knowledge shuffle proof
    /// * `ctx` - Context string used during shuffle
    ///
    /// # Returns
    ///
    /// `Some(Verified<MaskedDeck>)` if the proof is valid, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::{Shuffle, AggregatePublicKey};
    /// # let mut rng = ark_std::test_rng();
    /// # let shuffle = Shuffle::<10>::default();
    /// # let ctx = b"game";
    /// # let (_, pk1, proof1) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk1 = proof1.verify(pk1, ctx).unwrap();
    /// # let (_, pk2, proof2) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk2 = proof2.verify(pk2, ctx).unwrap();
    /// # let apk = AggregatePublicKey::new(&[vpk1, vpk2]);
    /// # let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    ///
    /// // Verify returns None if proof is invalid
    /// match shuffle.verify_initial_shuffle(apk, deck, proof, ctx) {
    ///     Some(verified_deck) => println!("Shuffle verified!"),
    ///     None => println!("Invalid shuffle proof"),
    /// }
    /// ```
    #[must_use]
    pub fn verify_initial_shuffle(
        &self,
        apk: AggregatePublicKey,
        next: MaskedDeck<N>,
        proof: ShuffleProof<N>,
        ctx: &[u8],
    ) -> Option<Verified<MaskedDeck<N>>> {
        proof.verify(&self.commit_key, apk, &self.initial_deck(), &next.0, ctx)
    }

    /// Verifies a subsequent shuffle proof and returns a verified deck.
    ///
    /// All players must verify each shuffle operation before proceeding. Verification
    /// ensures that the re-shuffle was performed correctly.
    ///
    /// # Arguments
    ///
    /// * `apk` - Aggregate public key from all players
    /// * `prev` - Previously verified deck state
    /// * `next` - The newly shuffled deck to verify
    /// * `proof` - Zero-knowledge shuffle proof
    /// * `ctx` - Context string used during shuffle
    ///
    /// # Returns
    ///
    /// `Some(Verified<MaskedDeck>)` if the proof is valid, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::{Shuffle, AggregatePublicKey};
    /// # let mut rng = ark_std::test_rng();
    /// # let shuffle = Shuffle::<10>::default();
    /// # let ctx = b"game";
    /// # let (_, pk1, proof1) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk1 = proof1.verify(pk1, ctx).unwrap();
    /// # let (_, pk2, proof2) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk2 = proof2.verify(pk2, ctx).unwrap();
    /// # let apk = AggregatePublicKey::new(&[vpk1, vpk2]);
    /// # let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    /// # let vdeck = shuffle.verify_initial_shuffle(apk, deck, proof, ctx).unwrap();
    /// # let (deck2, proof2) = shuffle.shuffle_deck(&mut rng, apk, &vdeck, ctx);
    ///
    /// // Verify the second shuffle
    /// match shuffle.verify_shuffle(apk, &vdeck, deck2, proof2, ctx) {
    ///     Some(verified_deck) => println!("Re-shuffle verified!"),
    ///     None => println!("Invalid shuffle proof"),
    /// }
    /// ```
    #[must_use]
    pub fn verify_shuffle(
        &self,
        apk: AggregatePublicKey,
        prev: &Verified<MaskedDeck<N>>,
        next: MaskedDeck<N>,
        proof: ShuffleProof<N>,
        ctx: &[u8],
    ) -> Option<Verified<MaskedDeck<N>>> {
        proof.verify(&self.commit_key, apk, &prev.0.0, &next.0, ctx)
    }

    /// Reveals a card using an aggregate reveal token from all players.
    ///
    /// After all players have created and verified reveal tokens for a specific card,
    /// the tokens are aggregated and used to decrypt the card. The card index in the
    /// original deck is returned.
    ///
    /// # Arguments
    ///
    /// * `art` - Aggregate reveal token from all players
    /// * `card` - The masked card to reveal
    ///
    /// # Returns
    ///
    /// `Some(usize)` with the card's index in the original deck (0-based),
    /// or `None` if the card cannot be revealed (wrong tokens or corrupted card).
    ///
    /// # Examples
    ///
    /// ```
    /// use ziffle::{Shuffle, AggregatePublicKey, AggregateRevealToken};
    /// # let mut rng = ark_std::test_rng();
    /// # let shuffle = Shuffle::<10>::default();
    /// # let ctx = b"game";
    /// # let (sk1, pk1, proof1) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk1 = proof1.verify(pk1, ctx).unwrap();
    /// # let (sk2, pk2, proof2) = shuffle.keygen(&mut rng, ctx);
    /// # let vpk2 = proof2.verify(pk2, ctx).unwrap();
    /// # let apk = AggregatePublicKey::new(&[vpk1, vpk2]);
    /// # let (deck, proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
    /// # let vdeck = shuffle.verify_initial_shuffle(apk, deck, proof, ctx).unwrap();
    ///
    /// let card = vdeck.get(0).unwrap();
    ///
    /// // Each player creates a reveal token
    /// let (rt1, rt_proof1) = card.reveal_token(&mut rng, &sk1, pk1, ctx);
    /// let (rt2, rt_proof2) = card.reveal_token(&mut rng, &sk2, pk2, ctx);
    ///
    /// // Verify and aggregate tokens
    /// let art = AggregateRevealToken::new(&[
    ///     rt_proof1.verify(vpk1, rt1, card, ctx).unwrap(),
    ///     rt_proof2.verify(vpk2, rt2, card, ctx).unwrap(),
    /// ]);
    ///
    /// // Reveal the card
    /// if let Some(card_index) = shuffle.reveal_card(art, card) {
    ///     println!("Card is at index {}", card_index);
    /// }
    /// ```
    #[must_use]
    pub fn reveal_card(&self, art: AggregateRevealToken, card: MaskedCard) -> Option<usize> {
        let AggregateRevealToken(art) = art;
        let MaskedCard((_, c2)) = card;
        // m = c2 − Σ[sk(i)·c1] for all i in [0; n - 1]
        let pt = (c2.into_group() - art.into_group()).into_affine();
        self.open_deck.iter().position(|&oc| oc == pt)
    }



}

macro_rules! impl_valid_and_serde_unit {
    (@impl_check) => {
        fn check(&self) -> Result<(), ark_serialize::SerializationError> {
            self.0.check()
        }
    };

    (@impl_ser) => {
        fn serialize_with_mode<W: ark_serialize::Write>(
            &self,
            writer: W,
            compress: ark_serialize::Compress,
        ) -> Result<(), ark_serialize::SerializationError> {
            self.0.serialize_with_mode(writer, compress)
        }

        fn serialized_size(&self, compress: ark_serialize::Compress) -> usize {
            self.0.serialized_size(compress)
        }
    };

    (@impl_deser) => {
        fn deserialize_with_mode<R: ark_serialize::Read>(
            reader: R,
            compress: ark_serialize::Compress,
            validate: ark_serialize::Validate,
        ) -> Result<Self, ark_serialize::SerializationError> {
            ark_serialize::CanonicalDeserialize::deserialize_with_mode(reader, compress, validate).map(Self)
        }
    };

    ($t:tt< $N:ident >) => {
        impl<const $N: usize> ark_serialize::Valid for $t<$N> {
            impl_valid_and_serde_unit!(@impl_check );
        }

        impl<const $N: usize> ark_serialize::CanonicalSerialize for $t<$N> {
            impl_valid_and_serde_unit!(@impl_ser );
        }

        impl<const $N: usize> ark_serialize::CanonicalDeserialize for $t<$N> {
            impl_valid_and_serde_unit!(@impl_deser);
        }
    };

    ($t:ty) => {
        impl ark_serialize::Valid for $t {
            impl_valid_and_serde_unit!(@impl_check );
        }

        impl ark_serialize::CanonicalSerialize for $t {
            impl_valid_and_serde_unit!(@impl_ser );
        }

        impl ark_serialize::CanonicalDeserialize for $t {
            impl_valid_and_serde_unit!(@impl_deser);
        }
    };
}

impl_valid_and_serde_unit!(PublicKey);
impl_valid_and_serde_unit!(SecretKey);
impl_valid_and_serde_unit!(PedersonCommitment);
impl_valid_and_serde_unit!(RevealToken);
impl_valid_and_serde_unit!(MaskedDeck<N>);

// only implement serialize so it can only be constructed from verified public keys
impl ark_serialize::CanonicalSerialize for AggregatePublicKey {
    impl_valid_and_serde_unit!(@impl_ser );
}

macro_rules! impl_valid_and_deser {
    (@impl_check $($field:ident),+) => {
        fn check(&self) -> Result<(), ark_serialize::SerializationError> {
            $( ark_serialize::Valid::check(&self.$field)?; )*
            Ok(())
        }
    };

    (@impl_ser $($field:ident),+) => {
        fn serialize_with_mode<W: ark_serialize::Write>(
            &self,
            mut writer: W,
            compress: ark_serialize::Compress,
        ) -> Result<(), ark_serialize::SerializationError> {
            $( self.$field.serialize_with_mode(&mut writer, compress)?; )*
            Ok(())
        }

        fn serialized_size(&self, compress: ark_serialize::Compress) -> usize {
            [
                $( self.$field.serialized_size(compress), )*
            ].iter().sum()
        }
    };

    (@impl_deser $($field:ident),+) => {
        fn deserialize_with_mode<R: ark_serialize::Read>(
            mut reader: R,
            compress: ark_serialize::Compress,
            validate: ark_serialize::Validate,
        ) -> Result<Self, ark_serialize::SerializationError> {
            Ok(Self {
                $( $field: ark_serialize::CanonicalDeserialize::deserialize_with_mode(
                    &mut reader, compress, validate
                )?, )*
            })
        }
    };

    ($t:tt< $N:ident > { $($field:ident),+ }) => {
        impl<const $N: usize> ark_serialize::Valid for $t<$N> {
            impl_valid_and_deser!(@impl_check $($field),*);
        }

        impl<const $N: usize> ark_serialize::CanonicalSerialize for $t<$N> {
            impl_valid_and_deser!(@impl_ser $($field),*);
        }

        impl<const $N: usize> ark_serialize::CanonicalDeserialize for $t<$N> {
            impl_valid_and_deser!(@impl_deser $($field),*);
        }
    };

    ($t:ty { $($field:ident),+ }) => {
        impl ark_serialize::Valid for $t {
            impl_valid_and_deser!(@impl_check $($field),*);
        }

        impl ark_serialize::CanonicalSerialize for $t {
            impl_valid_and_deser!(@impl_ser $($field),*);
        }

        impl ark_serialize::CanonicalDeserialize for $t {
            impl_valid_and_deser!(@impl_deser $($field),*);
        }
    };
}

impl_valid_and_deser!(MultiExpArg<N> {
    c_alpha, c_beta, ct_mxp0, ct_mxp1, o_alpha, o_r, beta, o_beta, tau
});
impl_valid_and_deser!(SingleValueProductArg<N> {
    c_d, c_sdelta, c_cdelta, a_tilde, b_tilde, r_tilde, s_tilde
});
impl_valid_and_deser!(ShuffleProof<N> {
    c_pi, c_xpi, mexp_arg, prod_arg
});
impl_valid_and_deser!(RevealTokenProof { t_g, t_c1, z });
impl_valid_and_deser!(OwnershipProof { a, z });

#[cfg(test)]
mod test;
