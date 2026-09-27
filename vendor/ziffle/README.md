# ziffle

Mental poker shuffle protocol using zero-knowledge proofs.

`ziffle` implements a mental poker protocol where multiple players can
collaboratively shuffle a deck of cards without any player learning the order,
and then selectively reveal individual cards.

The protocol uses [Bayer-Groth 2012](http://www0.cs.ucl.ac.uk/staff/J.Groth/MinimalShuffle.pdf)
shuffle proofs to ensure that each shuffle is performed correctly without revealing
the permutation.

`ziffle` is a `#[no_std]` crate.

## ⚠️ Security Warning

**This code has not been independently audited.** It is experimental software provided
as-is without any warranties. While the implementation follows the Bayer-Groth 2012
specification, it may contain bugs or vulnerabilities.

**DO NOT use this library to play for non-trivial amounts of money or in any
high-stakes scenario.** Use at your own risk.

## Main Entry Point

The `Shuffle` struct is the primary interface for using this library. Create an
instance with `Shuffle::<N>::default()` where `N` is the number of cards in your deck.

## Example: Three-Player Poker Game

```rust
use ziffle::{Shuffle, AggregatePublicKey, AggregateRevealToken};

// Create a standard 52-card deck
let shuffle = Shuffle::<52>::default();
let mut rng = ark_std::test_rng(); // DO NOT USE IN PRODUCTION
let ctx = b"poker_game_session_123";

// Three players generate their keys and prove ownership
let (alice_sk, alice_pk, alice_proof) = shuffle.keygen(&mut rng, ctx);
let (bob_sk, bob_pk, bob_proof) = shuffle.keygen(&mut rng, ctx);
let (carol_sk, carol_pk, carol_proof) = shuffle.keygen(&mut rng, ctx);

// Each player verifies others' key ownership proofs
let alice_vpk = alice_proof.verify(alice_pk, ctx).unwrap();
let bob_vpk = bob_proof.verify(bob_pk, ctx).unwrap();
let carol_vpk = carol_proof.verify(carol_pk, ctx).unwrap();

// Create aggregate public key from all verified keys
let apk = AggregatePublicKey::new(&[alice_vpk, bob_vpk, carol_vpk]);

// Alice performs the initial shuffle
let (alice_deck, alice_proof) = shuffle.shuffle_initial_deck(&mut rng, apk, ctx);
let alice_vdeck = shuffle
    .verify_initial_shuffle(apk, alice_deck, alice_proof, ctx)
    .expect("Alice's shuffle should be valid");

// Bob shuffles Alice's deck
let (bob_deck, bob_proof) = shuffle.shuffle_deck(&mut rng, apk, &alice_vdeck, ctx);
let bob_vdeck = shuffle
    .verify_shuffle(apk, &alice_vdeck, bob_deck, bob_proof, ctx)
    .expect("Bob's shuffle should be valid");

// Carol shuffles Bob's deck
let (final_deck, carol_proof) = shuffle.shuffle_deck(&mut rng, apk, &bob_vdeck, ctx);
let final_vdeck = shuffle
    .verify_shuffle(apk, &bob_vdeck, final_deck, carol_proof, ctx)
    .expect("Carol's shuffle should be valid");

// Now the deck is fully shuffled and encrypted. Let's reveal the first card.
let first_card = final_vdeck.get(0).unwrap();

// Each player creates a reveal token for the first card
let (alice_token, alice_token_proof) =
    first_card.reveal_token(&mut rng, alice_sk, alice_pk, ctx);
let (bob_token, bob_token_proof) =
    first_card.reveal_token(&mut rng, bob_sk, bob_pk, ctx);
let (carol_token, carol_token_proof) =
    first_card.reveal_token(&mut rng, carol_sk, carol_pk, ctx);

// All players verify each other's reveal tokens
let alice_vtoken = alice_token_proof
    .verify(alice_vpk, alice_token, first_card, ctx)
    .expect("Alice's token should be valid");
let bob_vtoken = bob_token_proof
    .verify(bob_vpk, bob_token, first_card, ctx)
    .expect("Bob's token should be valid");
let carol_vtoken = carol_token_proof
    .verify(carol_vpk, carol_token, first_card, ctx)
    .expect("Carol's token should be valid");

// Aggregate the verified tokens to decrypt the card
let aggregate_token = AggregateRevealToken::new(&[alice_vtoken, bob_vtoken, carol_vtoken]);

// Reveal the card's index in the original deck (0-51)
let card_index = shuffle
    .reveal_card(aggregate_token, first_card)
    .expect("Card should be revealed successfully");

println!("First card is at index: {}", card_index);
assert!(card_index < 52);
```

## Serialized Sizes

The following table shows the serialized sizes of types that need to be transmitted
over the network in a typical mental poker protocol. All sizes use compressed canonical
serialization from [arkworks](https://docs.rs/ark-serialize/0.5.0/ark_serialize/index.html).

| Type | Size | Notes |
|------|------|-------|
| `PublicKey` | 33 bytes | One per player at setup |
| `OwnershipProof` | 65 bytes | One per player at setup |
| `MaskedDeck<52>` | 3,432 bytes | 66 bytes per card (33 × 2) |
| `ShuffleProof<52>` | 5,547 bytes | One per shuffle operation |
| `RevealToken` | 33 bytes | One per player per revealed card |
| `RevealTokenProof` | 98 bytes | One per player per revealed card |

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
