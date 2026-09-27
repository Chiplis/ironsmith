# Local Ziffle extension

Based on crates.io ziffle 0.1.0 (MIT OR Apache-2.0). The Bayer–Groth proof,
ElGamal ciphertexts, and original initial-ceremony serialization are unchanged.

The extension extracts non-deserializable `Verified<MaskedCard>` values from
verified decks and composes differently sized decks from those values. The
application retains the original verified ceremony for manifest-label reveal
lookup independently of the current deck size. The application verifier owns
same-roster authentication and the append-only ciphertext consumption ledger;
this API cannot establish cross-epoch plaintext equality from ciphertexts alone.
