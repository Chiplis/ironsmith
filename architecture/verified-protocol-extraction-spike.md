# Spike: extracting the Verified protocol so Manabrew can host it

Date: 2026-11-23. Read-only survey; nothing built or run. Counts come from grep
over `web/ui/src`, so treat them as approximate.

## Goal

Let Manabrew rooms run Ironsmith **Verified** matches (signed actions, ziffle
shuffles, committed decks, quorum, signed resync) by sharing one protocol
implementation with the Ironsmith web app, not by re-implementing it.

## What the protocol is made of today

| Layer | Files | Lines | Coupling |
| --- | --- | --- | --- |
| Pure crypto/audit | `lib/multiplayer-audit.js` (+ `ziffle-*.js`, `accepted-actions.js`, `sync-commands.js`, `sync-object-identity.js`, `hidden-card-metadata.js`, `sha256.js`) | ~4.6k + ~1.7k | None. No React, no DOM, no `crypto.subtle` outside the file's own helpers. Already unit-tested under node. |
| Protocol "hooks" | `hooks/peer-lobby/{validation,crypto-resync,audit-material,connections,messaging,optimistic-state,trusted-sequencer,end-of-match-disclosure,tournament-witness,genesis-binding,shared}.js` + `usePeerLobby.js` | ~31k | React-shaped, but mechanically so (below). |
| Transport | `lib/relay/*` (relay websocket peer, direct channel, resume journal), PeerJS in `shared.js`/`messaging.js` | ~1k + embedded | `localStorage` journal, `window` timers, PeerJS objects held in refs. |
| Engine | wasm handle (`game.*`) | n/a | ~20 distinct methods touched by the protocol. |
| Ziffle verifier | `ironsmith-verifier-wasm` | in npm package already | Already exported by `split-facade.js`. |

## Key finding: the "hooks" are mixins over one shared closure

Each `usePeerLobbyX(base, servicesRef)` returns a bag of functions. `base` is one
object of ~80 `useRef` boxes plus `game`, `state`, `setState`, `setStatus`,
`multiplayer`. `servicesRef` is a late-bound registry the modules fill with
`Object.assign(servicesRef.current, ...)` so they can call each other. In effect
this is one large class split across files.

- `useCallback` x ~390 and `useEffect` x ~15 across the five big modules. Almost
  all `useCallback(fn, deps)` calls can become plain `fn`. The `useEffect` bodies
  are lifecycle (visibility/online/pageshow recovery, teardown).
- ~1,900 `xRef.current` uses. `useRef` boxes are `{current}`, so they work
  unchanged as plain objects outside React.
- `setState`-style calls: ~140 in `messaging.js` (lobby UI), ~40 in
  `usePeerLobby.js`, ~10-19 each in validation/crypto-resync/connections. These
  mirror protocol state into React for rendering. They are the one real seam.
- Browser APIs: `window.setTimeout/clearTimeout` (~60 uses) is trivial to swap
  for `globalThis`. `document`/`window` event listeners appear only in the
  foreground-recovery effect (`messaging.js` ~290-320). `localStorage` lives in
  `lib/relay/session.js`, `shared.js` (stored player index, seat records) and
  `value-store.js`.
- `messaging.js` (5.3k lines) is mostly **lobby and host-election**
  (create/join/rematch, PeerJS reconnect, takeover). Manabrew has its own
  lobby, so most of it is not needed for extraction.
- `validation.js` (5.5k), `crypto-resync.js` (4.9k), `audit-material.js` (4.2k)
  are the protocol core: sequenced-action validation, quorum collection, RNG
  commit/reveal, shuffle verification, material requests, openings, resync.

## Engine surface the protocol uses

`uiState`(18), `exportPublicAuditCheckpoint`, `previewCryptoRequirements`(+WithMaterial),
`injectTranscriptRandomSeeds`, `revealHiddenObject`, `hiddenCardOpenState`,
`queueVerifiedHiddenLibraryEpoch`, `queueVerifiedHiddenLibraryOpening`,
`createRuntimeSavepoint` / `restoreRuntimeSavepoint` / `releaseRuntimeSavepoint` /
`copyRuntimeSavepoint`, `retainPaymentDisclosure`, `getPaymentDisclosureForCommand`,
`endOfMatchDisclosureObligations`, `captureExactBuildSnapshot`, `ziffleRevealCard`.

That is a small, explicit interface, which is good news. The protocol is written
against `UiCommand`/`uiState`, **not** against Manabrew's `Prompt`/`PromptOutput`.
Manabrew's shim already sits on `dispatch_typed_command`, so the adapter needs a
deterministic `PromptOutput -> UiCommand` mapping that every seat computes the
same way. `manabrew_response_action` is that function today.

## Wire inventory (~75 message types)

Protocol messages (must be carried by the Manabrew relay as opaque, directed or
broadcast envelopes): `apply_action`, `pending_action_intent`,
`action_intent_progress/cancel`, `provisional_action/cancel`,
`action_quorum_vote_{request,response}`, `crypto_material_{request,response}`,
`ziffle_shuffle_step_{request,response}`, `ziffle_reveal_token_{request,response}`,
`rng_commit_{request,response}`, `rng_reveal_{request,response}`,
`match_seed_reveal_*`, `timeout_vote_*`, `protocol_timeout_vote_*`,
`disconnect_forfeit_vote_*`, `forfeit_player`, `protocol_wait_notice/answer`,
`private_view_opening(_disclosure)`, `end_of_match_disclosure`,
`state_resync`/`resync_request`/`resync_ack`, `signed_action_recovery_response`,
`peer_heartbeat`.

Lobby messages (Manabrew replaces these with its own): `lobby_*`, `join_request`,
`deck_update`, `match_start`, `rematch_*`, `reconnect_challenge`, `peer_ready`,
`player_presence`.

Trusted-mode only (not needed): `trusted_command`, `trusted_action_ack`,
`trusted_recovery_needed`, `trusted_command_error`.

## Proposed package shape

`@ironsmith/verified-session` (framework-free, ESM, runs in a worker or the main
thread):

```ts
createVerifiedSession({
  engine:    VerifiedEngine,    // the ~20 methods above, async
  transport: VerifiedTransport, // send(seat|"all", msg), onMessage, onSeatPresence
  store:     KeyValueStore,     // journal, seat keys, manifests (replaces localStorage)
  clock:     { now, setTimeout, clearTimeout },
  seats:     SeatRoster,        // relay identity <-> audit key binding
  events:    SessionEvents,     // state/status/notice/dispute callbacks (replaces setState)
}): VerifiedSession
// commands in:  submit(command), resync(), leave(), concede()
// events out:   state, prompt-needed, waiting-on(seat, reason), disputed, ended, notice
```

Extraction recipe:

1. Keep `lib/multiplayer-audit.js` and `lib/ziffle-*.js` as is; publish them.
2. Turn each `usePeerLobbyX(base, servicesRef)` into `createX(ctx)` where `ctx`
   is the old `base` with refs as plain `{current}` boxes. Replace
   `useCallback(fn, deps)` with `fn` and `useEffect` with explicit
   `start()`/`stop()`. No logic changes.
3. Replace `setState/setStatus/setMultiplayer` with `events.emit`; the React hook
   in the Ironsmith web app becomes a thin subscriber that mirrors events into
   state.
4. Move `localStorage` use behind `store`; move PeerJS/relay sockets behind
   `transport`. Keep the existing PeerJS and relay implementations as two
   `VerifiedTransport` adapters.
5. Split lobby/host-election out of `messaging.js`; it stays in the web app.

## Risks and unknowns

1. **Verified needs placeholder engines; the Manabrew shim does not have them.**
   Every shim view function (`manabrewView`, `manabrew_card_is_visible`,
   library-top logic, hand views, `manabrewPublicState`) assumes the engine
   knows all cards. That is the largest single piece of new Rust-side work, and
   this spike did not size it. Needs its own look at how `uiState` redaction
   differs from `manabrewView`.
2. **Prompt IDs.** The shim allocates prompt ids on first view and the host
   transcript replays them. Verified commits to `UiCommand`s and checkpoint
   hashes, so prompt ids must stay local presentation state.
3. **Menu determinism.** Bounded priority menus, proof budgets, and
   per-bot "proven offers only" mean menus can differ by seat/time. Fine for
   presentation, fatal if they gate command acceptance. Acceptance must use the
   engine's own legality check, not the offered menu.
4. **Seat identity.** Manabrew trusted mode binds a seat to a relay username
   only. Verified needs the audit key bound at genesis. The relay must preserve
   per-seat identity across reloads and host rotation.
5. **Bots.** No seat key. Disallow in Verified rooms for v1.
6. **Worker boundary.** Ironsmith runs the engine/verifier in workers
   (`wasmGameWorker`, `ziffleWorker`). Manabrew's engine currently runs
   wherever `ironsmithRuntime.ts` loads it; shuffle proofs will block the UI
   thread if left there.
7. **Tests we can reuse.** `tests/multiplayer-audit.test.js` (has ~9 known stale
   failures), `peer-crypto-message-order`, `peer-sequenced-action-recovery`,
   `relay-resync`, `relay-host-migration`, `audit-replay-*`,
   `verified-forfeit-draw.e2e`. They exercise the hook modules through
   harnesses, so the extraction needs the harness moved onto
   `createVerifiedSession` first.

## Suggested spike follow-ups (in order)

1. **Mechanical proof:** convert `trusted-sequencer.js` (146 lines) and
   `end-of-match-disclosure.js` (452) into context-factory form with no React,
   keep tests green. Cheapest way to validate the recipe.
2. **Pure-JS dry run:** build a node harness that drives two
   `createVerifiedSession`s over an in-memory transport with the real engine
   wasm, no browser, no React. Reuses `replay-trusted-match.js` ideas.
3. **Shim redaction study:** diff `uiState` vs `manabrewView` hidden handling;
   list every shim function that reads a hidden zone.
4. **Relay contract:** confirm `manabrew-server` can carry opaque directed
   envelopes of the sizes ziffle steps/reveal batches produce, and preserves
   seat binding across `migrate_host`.

## Rough sizing (unverified)

- Step 1-2 above: the extraction is large but low-risk and mechanical; it is the
  critical path.
- Shim redaction (risk 1) is the unknown that could dominate.
- Manabrew adapter + lobby wiring + UI: moderate once the two above exist.

## Result of follow-up 1 (2026-11-23): trusted-sequencer + end-of-match-disclosure

Converted both modules to the context-factory form; recipe confirmed mechanical.

- New React-free modules under `web/ui/src/lib/verified-session/`:
  `trusted-sequencer.js` (`createTrustedSequencer(base, servicesRef, host)`),
  `end-of-match-disclosure.js` (`createEndOfMatchDisclosure(...)`, plus the pure
  constants/payload/verdict helpers) and `payload.js` (clone/canonical payload,
  `sessionSecurityMode`, `toErrorMessage`, `playerNameForIndex`, `enqueueAsync`).
- `hooks/peer-lobby/{trusted-sequencer,end-of-match-disclosure}.js` are now ~20-line
  React adapters (`useRef` for the instance, `useEffect` for `start/stop/dispose`).
  `hooks/peer-lobby/browser-host.js` is the browser `host` object (send, diagnostics,
  perf, notice, `mergeAuditOpenings`). `shared.js` re-exports the moved helpers, so
  no other importer changed.
- Recipe as applied: keep the `(base, servicesRef)` signature; `useRef(x)` ->
  `{ current: x }`; `useCallback(fn, deps)` -> `fn`; `useEffect` -> explicit
  `start/stop`; `window.*` timers -> injectable `host.clock`; every platform call
  (`safeSend`, `recordPeerSyncPerf`, `emitSyncFailureNotice`, `mergeAuditOpenings`)
  -> `host.*`. Capturing `base` once is behaviour-equivalent because every ref in
  it is a stable `useRef`.
- `mergeAuditOpenings` stays injected through `host`: it drags in the ziffle opening
  link-key helpers in `shared.js`, which are the next pure-extraction target.
- New `tests/verified-session-modules.test.mjs` (8 tests, plain node, manual clock,
  real ECDSA keys) covers host accept/dedupe/stale/reuse, redelivery until ack, guest
  wait/resolve/reject/stop, disclosure send-once, verification, forged signature,
  withheld vs missing, and timer disposal.
- Trap found: some existing tests slice function source out of `shared.js`
  (`peer-crypto-message-order.test.mjs` sliced `enqueueAsync`; now imports it).
  Other `validation.js`-slicing harnesses are already broken on HEAD
  (`assertPaymentDisclosureIntent is not defined`, 44x). Prefer real imports.
- Verification: new tests 8/8; esbuild bundle of `usePeerLobby.js` resolves; the same 12
  related suites fail identically on pristine HEAD and now (81 failures each, same
  reasons: stale protocol-31/checkpoint-11 expectations, sliced-harness
  ReferenceErrors); `relay-browser`/`relay-host-migration` real-browser tests incl.
  "lost accepted action is retried and command IDs prevent duplicate application"
  pass, with 2 pre-existing failures ("State resync requires audit protocol 33")
  identical on HEAD.

## Outcome (2026-11-24): Verified mode runs inside Manabrew

The plan above (rewrite every hook module as a context factory) was replaced by a
cheaper, safer one: run the **unchanged** hook stack headless.

- `web/ui/src/lib/verified-session/hook-runtime.js` implements the four hooks the stack
  uses (`useState`, `useRef`, `useCallback`, `useEffect`) with React-compatible rules
  (stable setters, dependency-gated callbacks and effects, cleanup first). Inside a
  headless render they use slot tables; otherwise they call the React the web app
  registers (`registerReactHooks`, in `hooks/peer-lobby/browser-peers.js`). No logic
  of `usePeerLobby` and its modules changed, so closures behave exactly as before.
- Transport is injected: `createPeer` input of `usePeerLobby` (falls back to the
  registered factory). `window` timers became `globalThis`; `@/` aliases became relative.
- `createVerifiedSession({ engine, createPeer, ensureCards })`
  (`lib/verified-session/index.js`) composes it with an engine (`call(method, ...args)`),
  a state store and `applySyncedCommand`. `scripts/build-verified-session.mjs` bundles
  it into the npm package as `ironsmith-wasm/verified-session` (no React import).
- The Manabrew adapter methods in the engine: `manabrewAttach` (adopt a match started
  with committed decks), `manabrewSetVerified` (no engine-side auto-answers; a forced
  answer is returned as `forcedCommand`), `manabrewTranslate` (prompt answer ->
  `UiCommand`, not applied), and unopened hidden cards render as hidden.
- Tests: `tests/verified-headless-match.test.mjs` (two seats in worker threads, real
  engine, ~80 signed actions), `tests/verified-manabrew-shim.test.mjs` (Manabrew
  prompts -> translate -> signed transcript, privacy of hands),
  `tests/verified-session-hook-runtime.test.mjs`, `tests/verified-session-modules.test.mjs`.
  `IRONSMITH_PKG=<built npm package dir>` points the harness at a different engine.
- Manabrew side (branch `feat/ironsmith-verified-mode`): `IronsmithVerified` engine kind,
  `src/game/ironsmithVerified/{roomPeer,ironsmithVerifiedRuntime}.ts`, worker
  `verifiedOpen/verifiedCall/verifiedClose`, lobby option, and
  `tests/e2e-ironsmith/ironsmith-verified-pair.mjs` (two real browsers, real relay).

Gotchas found: the web app turns off the engine's cleanup auto-discard (a random pick no
other seat can replay); a room peer must emit `open` after the lobby attaches listeners;
the host must not invite guests before its lobby id exists; seats are joined in table
order so engine seats equal room seats.

Not done / limits: two players only exercised (the join-order logic is written for 2-4);
no lobby bots in verified rooms; no spectators; Manabrew's room still shows decklists to
the host before the match (hidden-deck privacy is the game's, not the lobby's); host
migration, reconnect and forfeit votes are the web app's, untested through Manabrew;
opponents' cards are registered from Scryfall data when opened (needs network);
`ironsmith-wasm` must be released with `verified-session` before Manabrew can depend on it.
