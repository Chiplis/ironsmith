# Serverless, cheaterless Magic: The Gathering multiplayer

Ironsmith multiplayer is deliberately not built around an authoritative game server. The browser that hosts the lobby helps people find each other, and PeerJS gives us signaling and data channels, but the host is not trusted to sequence the game, see hidden information, roll random numbers, or decide which action is canonical. Every participating browser runs the same WebAssembly rules engine, validates every cryptographic artifact it receives, and keeps a locally verifiable transcript of the match.

The short version is:

- every action is signed by the player who is allowed to take it;
- every action extends a hash chain over the previous action, the exact command, hidden-information openings, randomness proofs, shuffle proofs, private-view proofs, the match clock audit, and the resulting public checkpoint;
- every hidden card identity is committed before the game starts and opened only with the salt that matches that commitment;
- every library order is produced by a multi-party mental-poker shuffle, not by one player's RNG;
- every non-library random outcome is derived from a signed commit/reveal transcript contributed by every player;
- for three and four player games, actions also need a peer quorum certificate before they are accepted as canonical;
- a complete transcript can be verified after the match, including optional engine replay against the WebAssembly rules engine.

That is what "serverless" means here: no server-side referee is required. It does not mean there is no network infrastructure at all. A PeerJS signaling service can still introduce peers, and peers can still relay messages through each other. It means the authority lives in deterministic state transition, cryptographic binding, and cross-peer verification, not in a private process controlled by us.

It is also what "cheaterless" means in practice. A malicious client can still modify its local JavaScript, disconnect, refuse to answer a proof request, or try to sign garbage. The protocol is designed so that honest peers either reject the message before mutating state, or retain enough signed evidence to prove equivocation afterwards.

## Threat model

The adversary is a match participant running an arbitrary browser client. They can inspect and patch local code, send malformed PeerJS messages, replay old messages, alter commands in transit, try to act for another seat, skip transcript sequence numbers, fork the action log, reveal the wrong hidden card, bias random outcomes, shuffle their own library favorably, claim a timeout early, or ask another player for hidden-card material they are not entitled to see.

The adversary is not assumed to be able to break standard browser cryptography, forge ECDSA signatures for another player's key, invert SHA-256 commitments, decrypt ECDH/AES-GCM private-view material without the recipient key, or make an honest browser accept a transcript that fails local replay. Denial of service is explicitly out of scope: a player can always close their laptop. The protocol can turn some stalls into signed timeout or disconnect forfeits, but it cannot force a malicious peer to remain online.

Engine correctness is also a dependency. If the deterministic rules engine has a consensus bug, every honest peer can faithfully agree on the wrong result. The audit layer prevents tampering with the agreed transition; it does not make the rules implementation magically correct.

## Protocol surfaces

The browser implementation is concentrated in a few places:

- `web/ui/src/lib/multiplayer-audit.js`: canonical JSON, hashes, signatures, deck commitments, match genesis, action envelopes, action quorums, fair-random verification, ziffle proof validation wrappers, private-view verification, resync envelopes, transcript verification.
- `web/ui/src/lib/audit-replay.js`: deterministic transcript replay through a game instance.
- `web/ui/src/hooks/usePeerLobby.js`: PeerJS integration, match start, cryptographic-material collection, action submission, action reception, quorum collection, resync, timeouts, disconnect policy.
- `crates/ironsmith-wasm/src/wasm_game_impl/ziffle_backend.rs`: the WebAssembly bridge to the ziffle mental-poker primitives.
- `crates/ironsmith-wasm/src/lib.rs` and `crates/ironsmith-wasm/src/wasm_game_impl/dispatch.rs`: hidden-information tracking, crypto requirement generation, command preview, and public checkpoint export.

The current browser audit protocol is version `14`, supports two to four players, and uses browser-native WebCrypto for the audit signatures and private-view encryption.

## Canonical bytes first

Cryptographic protocols fail if two peers serialize the same logical object differently. Ironsmith signs and hashes canonical JSON, not raw JavaScript objects. The canonicalizer recursively sorts object keys, drops `undefined`-like non-data values, normalizes `-0` to `0`, rejects non-finite numbers, and serializes BigInts as decimal strings. That canonical JSON is then domain-separated before hashing or signing.

For example, an action state hash is:

```text
SHA256(canonicalJson({
  domain: "ironsmith-ui-audit-state-v1",
  matchId,
  seq,
  prevStateHash,
  command,
  clock,
  openings,
  rngReveals,
  shuffleProofs,
  privateViewProofs,
  publicCheckpointHash
}))
```

The signature layer uses ECDSA P-256 with SHA-256. WebCrypto returns raw 64-byte `r || s` signatures. We canonicalize them to low-S form and reject non-canonical signatures on verification. That removes ECDSA malleability as a cheap way to create multiple bytewise-distinct signatures for the same payload.

The result is that every hash and signature commits to the same stable bytes in every browser.

## Identities and match genesis

Before the match starts, each player has three independent identities:

1. An ECDSA P-256 audit signing key. This signs player genesis records, action envelopes, quorum votes, random commitments, random reveals, timeout votes, disconnect votes, protocol-timeout votes, and resync envelopes.
2. An ECDH P-256 private-view encryption key. This lets one player reveal hidden-card material only to the viewer who was legally allowed to see it.
3. A ziffle mental-poker key, with an ownership proof bound to the match context. This key participates in encrypted deck shuffles and later card-position openings.

Each player signs a player genesis payload containing their peer id, name, seat, audit public key, encryption public key, deck commitment manifest, ziffle key, and deck counts. The host then signs a match genesis payload containing the protocol version, match id, lobby id, host peer id, format, opening hand size, seed, match clock policy, all public player genesis records, all deck manifests, all ziffle public keys, and all initial ziffle shuffle ceremonies.

This is the first anti-tamper boundary. If a reconnecting peer tries to claim someone else's seat, or the host rewrites a deck manifest, key, player list, clock policy, or ziffle ceremony, genesis verification fails before gameplay begins.

The host signs genesis, but that does not make the host authoritative after match start. It means the host assembles the initial roster and settings. Once genesis is signed, every later action must extend the signed match id and the live audit chain.

## Deck commitments

A deck manifest is the bridge between Magic's hidden information and peer-verifiable play.

For each player, the client builds:

- a decklist hash over the normalized deck, sideboard, commanders, match id, and owner;
- a salted decklist commitment;
- one salted commitment per main-deck slot;
- a commitment root over the decklist commitment and all slot commitments.

The per-slot commitment is domain-separated:

```text
SHA256(canonicalJson({
  domain: "ironsmith-ui-audit-card-commitment-v1",
  matchId,
  owner,
  slot,
  card,
  salt
}))
```

The public manifest exposes the owner, card counts, decklist commitment, commitment root, and `(slot, commitment)` pairs. The private manifest retains `(slot, card, salt, commitment)` so the owner can later open a slot.

Current browser-hosted matches publish open decklist fields in the match payload and verify those slot openings at genesis. Even then, the shuffled position of each card is hidden: the WebAssembly engine starts from runtime hidden deck manifests rather than raw card arrays. The commitment machinery is also capable of closed-list openings, because a revealed card is always validated by recomputing the slot commitment from `(matchId, owner, slot, card, salt)`.

If a peer tries to reveal an Island for a slot that was committed as Lightning Bolt, every honest browser recomputes the hash and rejects the action.

## Ziffle and mental-poker libraries

The hardest part of serverless Magic is the library. A player must not be able to stack their deck, but no opponent should see its order before cards are revealed.

Ironsmith uses the ziffle mental-poker backend from WebAssembly. Conceptually, each library starts as an encrypted deck of original slots. The deck owner does not get unilateral control over the shuffle. Instead, every player contributes one verifiable shuffle step, in sorted player order.

For each owner's deck:

1. Every player publishes a ziffle public key and ownership proof bound to the match key context.
2. The first shuffler calls `shuffle_initial_deck`, producing a masked deck and shuffle proof.
3. Each later shuffler verifies the previous steps, then calls `shuffle_deck`, producing the next masked deck and proof.
4. Every peer verifies the full chain: the first step with `verify_initial_shuffle`, later steps with `verify_shuffle`.
5. The final masked deck bytes are hashed to a `deckHash`.

The match genesis stores the ceremony:

```text
{
  owner,
  deckCount,
  context,
  keyContext,
  keys,
  steps: [{ shuffler, deckHex, proofHex }, ...],
  deckHash
}
```

The runtime hidden manifest then represents shuffled positions with commitments like:

```text
ziffle:<deckHash>:<position>
```

Opening a card requires reveal tokens from the ziffle key holders. The opening proof includes the owner, shuffled position, original slot, position commitment, card commitment, deck hash, key roster, shuffle steps or a compact reference to them, and reveal tokens. The verifier:

1. re-verifies the ziffle keys against the signed roster;
2. re-verifies the shuffle ceremony and deck hash;
3. checks that the position commitment is exactly `ziffle:<deckHash>:<position>`;
4. verifies every reveal-token proof against the masked card at that position;
5. aggregates the reveal tokens;
6. recovers the original committed slot;
7. verifies that the card opening `(slot, card, salt)` matches the deck manifest.

This links three facts without trusting the owner:

- this physical game object came from a particular encrypted shuffled position;
- that encrypted position opens to a particular original deck slot;
- that original slot was committed to a particular card before the game started.

No single player can choose the final order after seeing another player's entropy, because every player contributes a shuffle step. No single player can reveal arbitrary card identities, because card identity still has to match the committed slot.

## In-game shuffles

Initial deck order is not the only shuffle in Magic. Fetch lands, tutors, cascade-like effects, and many graveyard/library effects produce new random library orders during the game.

The WebAssembly engine journals hidden-information operations. When an action is previewed or applied, it compares the hidden/random state before and after the command and emits crypto requirements. A library shuffle produces a `verifiable_shuffle` requirement with:

- owner;
- zone, currently `library`;
- object order before the shuffle;
- object order after the shuffle;
- random counter before and after;
- count of randomized cards.

The action must then carry a `ziffle_shuffle` proof bound to the match id and action sequence. The verifier checks that the proof:

- references a known owner and library zone;
- has a valid deck count;
- is bound to the signed ziffle key roster;
- uses the current match id as key context;
- uses an action-specific context whose prefix is the match id;
- is bound to the action epoch;
- includes a valid ziffle proof chain;
- preserves the authenticated object order expected by the engine requirement.

This matters for search effects. If a player searches for a card, shuffles the rest, and puts the found card on top, the randomized subset is not the whole library. The engine emits a `verifiable_shuffle` for the randomized subset and a separate `hidden_order_update` for the deterministic placement. The proof therefore authenticates the actual random part instead of pretending that the tutored card was shuffled.

## Fair random outside libraries

Not every random choice is a library shuffle. Some effects choose targets at random, order hidden piles randomly, or otherwise consume irreversible randomness. For these, the engine emits a `fair_random` requirement.

The browser satisfies that requirement with a commit/reveal ceremony:

1. Each player chooses a nonce.
2. Each player signs a commitment response:

   ```text
   commitmentHex = SHA256(canonicalJson({
     domain: "ironsmith-rng-commit-v1",
     nonceHex
   }))
   ```

3. After all commitments are collected, each player signs a reveal response containing the nonce and commitment echo.
4. Every verifier checks all signatures, checks that every revealed nonce hashes to the earlier commitment, and requires one sorted entry per player.
5. The combined seed is:

   ```text
   SHA256(canonicalJson({
     domain: "ironsmith-combined-rng-v2",
     matchId,
     seq,
     requirementId,
     commits: sortedByPlayer(...),
     reveals: sortedByPlayer(...)
   }))
   ```

6. That seed is injected into the local deterministic engine before applying or replaying the command.

The last revealer can refuse to reveal, but cannot wait to see everyone else's nonces and then choose a favorable nonce. Their commitment fixes the nonce first. Refusal becomes a liveness problem, handled by protocol-response timeout policy rather than by letting one peer invent the random seed.

## Action envelopes

A Magic action is only accepted if it is a signed transition from the current transcript head.

An action envelope contains:

```text
{
  matchId,
  seq,
  actor,
  signer,
  prevStateHash,
  command,
  clock,
  openings,
  rngReveals,
  shuffleProofs,
  privateViewProofs,
  publicCheckpointHash,
  nextStateHash,
  signatureAlgorithm: "ecdsa-p256-sha256",
  signature
}
```

`nextStateHash` is the audit hash over all of the transition material except the signature. The signature signs the envelope payload including `nextStateHash`.

On the submitting peer, the flow is:

1. Confirm the local player is the current decision player.
2. Compute the next sequence number.
3. Capture the previous audit state hash.
4. Build a match-clock audit entry.
5. Ask the engine to preview crypto requirements for the command.
6. Build local shuffle proofs, RNG reveals, public openings, private-view proofs, and requested remote crypto material.
7. Inject that material into the deterministic engine.
8. Apply the command locally.
9. Handle post-apply openings and shuffle requirements.
10. Export a public audit checkpoint and hash it.
11. Build and sign the action envelope.
12. Ask peers for an action quorum certificate when the player count requires one.
13. Append the action locally and relay it through the peer mesh.

On a receiving peer, the flow is intentionally symmetric:

1. Reject duplicate actions that are bytewise equivalent to an already-applied sequence.
2. Reject gaps and request/resync missing history.
3. Verify `seq == lastAppliedSequence + 1`.
4. Verify `matchId`, `prevStateHash`, command equality, and `nextStateHash`.
5. Verify `signer == actor`.
6. Verify the actor's ECDSA signature.
7. Verify any pending signed action intent.
8. Verify the quorum certificate when required.
9. Verify that the actor is the current decision player, except for explicitly supported timeout/disconnect/protocol-forfeit commands.
10. Verify the match-clock audit.
11. Reveal pre-action hidden openings.
12. Decrypt private material addressed to the local viewer.
13. Preview local crypto requirements for the command.
14. Verify shuffle proofs, RNG reveals, public openings, and private-view proofs satisfy those requirements.
15. Inject transcript seeds/proofs into the engine.
16. Apply the command locally.
17. Reveal post-action openings.
18. Apply verified shuffle proofs.
19. Reveal the local ziffle hand if needed.
20. Export the local public checkpoint and require its hash to equal the signed `publicCheckpointHash`.
21. Commit the clock audit, append the action, and relay it onward.

An action that passes signature verification but fails engine replay is rejected. An action that is legal in the sender's modified client but not legal in an honest engine is rejected. An action that produces a different public checkpoint is rejected.

## Public checkpoints

The public checkpoint is the consensus digest of what every player is allowed to know after an action. It is exported by the WebAssembly engine with hidden information redacted. Before hashing, the browser normalizes it:

- transient metadata keys are stripped;
- runtime object ids are remapped to stable public ids where possible;
- public object lists are sorted where order is not semantically meaningful;
- stack entries, attachments, battlefield, graveyard, command zone, and public exile references are normalized.

The hash is:

```text
SHA256(canonicalJson({
  domain: "ironsmith-public-audit-checkpoint-v1",
  checkpoint: normalizedPublicCheckpoint
}))
```

This gives us a cheap consensus check. Peers do not have to expose hands and libraries to prove they got the same public result. They only need to agree on the redacted public state hash after applying the signed command and cryptographic material.

## Action quorum

Two-player games are tamper-evident but cannot have an honest third voter. In protocol v14, the action quorum threshold is:

- two players: `0`;
- three players: `2`;
- four players: `3`.

For three and four player matches, the acting player collects action quorum votes. A vote signs:

```text
{
  domain: "ironsmith-action-quorum-vote-v1",
  matchId,
  seq,
  actor,
  voter,
  prevStateHash,
  nextStateHash,
  publicCheckpointHash,
  actionSignature
}
```

The resulting certificate is attached to the action as `ironsmith-action-quorum-v1`. Verifiers require enough unique eligible voters and reject duplicate voters, mismatched action signatures, mismatched public checkpoint hashes, and invalid vote signatures.

Peers persist their own quorum votes and refuse to sign conflicting votes for the same sequence. If a fork appears anyway, the transcript can include `action_fork_v1` dispute evidence. That evidence contains two signed actions with the same sequence and previous hash but different signed payloads. The verifier can identify the equivocating actor, and in quorum games can also identify voters who signed both sides.

This is how we remove host sequencing authority. The host can relay quickly, slowly, or not at all, but it cannot make an invalid action canonical. Any peer can relay the same signed action to the mesh, and the quorum certificate is about the action payload, not about who delivered it.

## Hidden-card openings and private views

The WebAssembly engine emits crypto requirements when hidden information crosses a visibility boundary:

- `public_open`: a hidden card identity became public;
- `private_open`: a hidden card identity became known to one viewer;
- `public_view_window`: a public batch view happened;
- `private_view_window`: a private batch view happened;
- `hidden_move`: a hidden object moved zones while remaining hidden;
- `hidden_order_update`: a hidden zone order changed deterministically;
- `verifiable_shuffle`: hidden order changed randomly and must be proven by ziffle;
- `fair_random`: non-library randomness was consumed.

Public openings carry enough material to recompute the original slot commitment. Private openings are encrypted to the legal viewer. The encryption scheme is `ecdh-p256-aes-gcm-sha256`: the sender uses an ephemeral ECDH key with the viewer's public encryption key, encrypts canonical JSON under AES-GCM, and records a plaintext hash in the signed action proof.

The signed proof therefore commits to the private opening without revealing it to everyone:

```text
{
  type: "encrypted_private_opening",
  owner,
  viewer,
  zone,
  objectId,
  commitment,
  encryptedOpening: {
    scheme: "ecdh-p256-aes-gcm-sha256",
    recipientPublicKey,
    ephemeralPublicKey,
    ivHex,
    ciphertextHex,
    plaintextHash
  }
}
```

The recipient can decrypt immediately. Everyone else can still verify that the action committed to a particular encrypted disclosure. Postgame disclosure can reveal the plaintext and prove that the encrypted private opening matched the original deck commitment.

There is an additional request-authorization layer. A peer only answers cryptographic-material requests for local owner requirements that were previewed and authorized by the action. If someone asks for an unrelated hidden card, the client raises "Cryptographic material request asks for unauthorized hidden-card material" and refuses.

## Match clock, disconnects, and stalls

Serverless games need a way to progress when a peer disappears. Ironsmith uses a signed, hash-chained match clock audit.

Each action can include a `match_clock_v1` entry with:

- match id;
- action sequence;
- actor;
- reason, such as normal action, timeout claim, disconnect timeout claim, or protocol response timeout claim;
- clock policy;
- active player;
- elapsed milliseconds;
- remaining time by player;
- previous clock hash;
- basis sequence;
- clock hash.

The clock hash is domain-separated with `ironsmith-match-clock-audit-v1`, so the clock has its own chain parallel to the action hash chain. Receivers verify that the clock active player matches the current decision, the elapsed time is plausible under local observation bounds, the remaining times are exactly the debited values, and timeout forfeits exhaust the active player's clock.

Disconnect forfeits and protocol-response timeouts are separate from normal clock expiration:

- Disconnect auto-forfeit uses `disconnect_timeout_policy` and currently waits 60 seconds.
- Protocol-response timeout uses `protocol_response_timeout_policy` and currently waits 60 seconds for required proof material or votes.

Those certificates are also signed. Non-target players vote over the match id, basis sequence, forfeited player, peer id, timeout duration, observation timestamps, request type, request id, and request payload hash. A player cannot sign their own timeout or disconnect forfeit as an eligible voter. Early claims fail because the vote eligibility timestamp must equal the observed timestamp plus the timeout.

The protocol cannot prevent a malicious player from going offline. It can prevent another player from fabricating an early timeout, and it can produce a verifiable reason why the stalled player was forfeited.

## Resync and reconnect

Peer-to-peer delivery is unreliable by design. A peer can miss messages, reconnect with a new PeerJS id, or receive action `N + 1` before action `N`.

Resync uses a signed envelope:

```text
{
  domain: "ironsmith-resync-envelope-v1",
  matchId,
  signer,
  lastSequence,
  finalStateHash,
  checkpointHash,
  actionsHash,
  signature
}
```

The checkpoint hash is domain-separated with `ironsmith-resync-checkpoint-v1`. The action log hash is domain-separated with `ironsmith-resync-actions-v1`. A recipient refuses resync data that is older than its local transcript or does not contain its local prefix exactly. That prevents a peer from "resyncing" someone onto a fork that erases already-applied actions.

When closed-list redaction is active, the payload sent to a peer can redact other players' decklists while preserving the signed audit material needed for verification. Current open-decklist matches simply send the full payload.

## Full transcript verification

At the end of a match, the audit transcript is just data:

```text
{
  kind: "ironsmith-live-browser-audit-v1",
  match,
  matchId,
  protocolVersion,
  signatureAlgorithm,
  genesis,
  initialStateHash,
  initialPublicCheckpointHash,
  actions,
  privateViewDisclosures,
  finalStateHash,
  finalPublicCheckpointHash,
  finalPublicCheckpoint,
  disputes,
  outcome
}
```

`verifyLiveAuditTranscript` replays the audit semantics without trusting the original sender:

1. Verify transcript kind and protocol version.
2. Verify signed match genesis.
3. Verify the player roster, deck manifests, ziffle keys, ziffle ceremonies, and initial public checkpoint.
4. Walk actions from sequence `1`.
5. Require each `prevStateHash` to equal the running hash.
6. Verify the actor signature and low-S canonical form.
7. Verify action quorum certificates.
8. Verify timeout/disconnect/protocol-timeout certificates.
9. Verify ziffle shuffle proofs and ziffle opening proofs.
10. Verify public openings against deck manifests.
11. Verify private-view encrypted proofs and disclosures.
12. Verify all fair-random commitments, reveals, and combined seeds.
13. Recompute each `nextStateHash`.
14. Verify the final public checkpoint hash and claimed outcome.
15. Optionally replay the transcript through the WebAssembly engine and require every replayed public checkpoint hash to match the signed action.
16. Verify dispute evidence and derive disputed outcomes when forks are present.

The verifier does not need the original host. It needs only the transcript, the public keys and manifests bound in genesis, and the same deterministic engine for replay.

## Why this catches common cheats

Acting for another player fails because `signer` must equal `actor`, and the signature must verify against the actor's genesis-bound audit key.

Changing a command in transit fails because the broadcast command must canonicalize to the command inside the signed audit envelope, and the signature covers that command.

Skipping sequence numbers fails because receivers require `seq == lastAppliedSequence + 1`; otherwise they request/resync missing history.

Replaying a duplicate action is idempotent only if it is equivalent to the already-applied action. A different action for the same sequence becomes fork evidence.

Adding a card from nowhere fails at multiple layers: the action command is unauthorized, hidden-card openings must match precommitted slots, and the resulting public checkpoint hash must match every honest engine.

Revealing the wrong card fails because the opening salt and card name must recompute to the committed slot hash.

Stacking a deck fails because library order is the result of all players' ziffle shuffle steps, and later shuffles must carry verifiable shuffle proofs bound to the action.

Biasing non-library randomness fails because the combined seed is derived from all players' signed commit/reveal nonces.

Lying about a timeout fails because clock entries are hash-chained, elapsed time is locally checked, and timeout certificates require eligible non-target signatures.

Withholding proof material does not create a fake action. It creates a stall that can become a signed protocol-response timeout.

## What remains hard

This protocol does not eliminate all trust. It moves trust to narrower and more inspectable assumptions.

Peers still trust browser crypto, the ziffle implementation, the WebAssembly engine, and the specific code that maps Magic rules to deterministic engine transitions. A sufficiently bad engine bug can become a consensus bug. Colluding players can refuse to play, can sign their own fork among themselves, and can withhold liveness from honest peers. Two-player games cannot have a third-party quorum, so they are tamper-evident but not quorum-final in the same way as three and four player games.

The practical win is that no participant gets unilateral authority. The host does not get to be the judge. The deck owner does not get to be the shuffler. The active player does not get to be the random oracle. The viewer of private information gets encrypted access, not a public leak. Every accepted transition leaves a signed, replayable, hash-chained audit trail.

For a game as stateful and hidden-information-heavy as Magic, that is the important boundary: cheating stops being a claim about someone's screen and becomes a concrete verification failure in a transcript.
