# Ironsmith Audit Prototype

This crate defines the first concrete audit surface for cheat-resistant multiplayer:

- four-player transcript validation
- Ed25519-signed action envelopes
- chained public state hashes
- card commitment openings for hidden-zone cards
- RNG commitment openings
- deck shuffle ceremonies that require every player to contribute entropy
- fair and cheating fixture generation

The verifier intentionally validates generic engine operations, not card names. Runtime
integration should emit these records from reusable effect, decision, zone-change, and RNG
surfaces.

## Runtime Integration Added

- `ironsmith-wasm` now supports committed hidden-card placeholders, redacted sync
  checkpoints, and reveal-by-slot/object APIs. Peers can import hidden libraries without
  card definitions, then hydrate a real card only after a verified opening.
- `web/ui/src/lib/multiplayer-audit.js` provides canonical JSON, WebCrypto signing,
  audit state hashing, private deck manifests, signed action requests, signed sequenced
  action envelopes, card-opening helpers, and a verifier for exported browser live-audit
  transcripts.
- `usePeerLobby` protocol v7 advertises audit public keys, requires remote players to sign
  their action requests, has the host sign the final sequenced action hash chain, sends
  only public deck manifests/counts for remote libraries, exports redacted resyncs by
  peer perspective, reveals committed slots before applying live actions, and can export
  a browser live-audit transcript.

The live browser path now removes the host plaintext decklist dependency for remote
players. A local peer keeps its own deck salts privately, sends only public commitments to
the lobby, and attaches card openings to actions when a committed hidden object becomes
public. Full mental-poker encrypted deck custody remains the production target for
preventing any player from learning library order; the current live integration is the
engine/audit surface needed to plug that backend into live browser play.

## Mental-Poker Backend

The selected hidden-zone architecture is mental-poker encrypted deck custody with
Bayer-Groth-style verifiable shuffle proofs. This is represented as
`bayer_groth_mental_poker_v1` in transcript `ShuffleProof` records.

The concrete backend is `ziffle-0.1`. The audit crate depends on it directly and exposes
a four-player backend round trip through `backend::ziffle_four_player_round_trip`.
That backend verifies:

- four players generate and verify keys
- all four players perform/verify encrypted deck shuffle steps
- all four players produce verified reveal tokens for one card
- the aggregate reveal opens a card index without exposing deck order first

The checked-in transcript fixtures still store compact transcript-binding hashes for
shuffle artifacts, not full serialized ziffle proof blobs. Live browser play also still
needs a WebAssembly-facing ziffle ceremony before players are fully protected from
knowing or biasing library order.

## CLI

```sh
cargo run -p ironsmith-audit -- verify fixtures/audit/four_player_fair.json
cargo run -p ironsmith-audit -- verify fixtures/audit/four_player_cheat_detected.json
cargo run -p ironsmith-audit -- explain fixtures/audit/four_player_fair.json
```

The fair fixture exits successfully. The cheating fixture exits nonzero and reports the
first invalid sequence.
