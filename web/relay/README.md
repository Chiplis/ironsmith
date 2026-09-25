# WebSocket lobbies on Cloudflare Workers Free

This alternative to PeerJS uses one SQLite-backed Durable Object per room and a separate public directory. The existing lobby, action sequencing, resync, and game engine stay in browsers. Use **Connection → WebSocket lobby**, choose a format, and optionally advertise the table. Join by the generated code or the public searcher. Both clients must use a UI build configured for the same relay.

## Local development

```sh
cd web/relay
npm ci
npm run dev
```

In a second terminal:

```sh
cd web/ui
VITE_LOBBY_RELAY_URL=http://localhost:8787 pnpm dev
```

The checked-in origin allowlist accepts `http://localhost:5173` and `http://127.0.0.1:5173`. If Vite chooses a different port, change `ALLOWED_ORIGINS` in `wrangler.jsonc` to match. Origins must be exact, comma-separated values, without paths or trailing slashes.

## Deploy

1. Use a Cloudflare account on Workers **Free**. No paid-plan features, KV namespace, R2 bucket, or external database are required.
2. Set `ALLOWED_ORIGINS` in `wrangler.jsonc` to the production game origin (and any deliberately supported local origins).
3. From this directory run `npx wrangler login`, then `npm run deploy`. The migration uses `new_sqlite_classes`, which is required for Durable Objects on Free.
4. Build the UI with `VITE_LOBBY_RELAY_URL=https://ironsmith-lobby-relay.<account-subdomain>.workers.dev`. This variable is a public service URL, not a secret. Rebuild and deploy the UI after changing it.

The deployed relay is `https://ironsmith-lobby-relay.nicolas-siplis.workers.dev`.
The UI's `.env.production` points production builds at this service. Allowed browser origins are `https://chiplis.com`, `http://localhost:5173`, and `http://127.0.0.1:5173`.

Without that variable, the WebSocket creation option is visibly disabled and public search is hidden. Existing P2P/LAN behavior remains available.

## Formats and deck validation

Supported public formats: Standard, Pioneer, Modern, Legacy, Vintage, Pauper, Commander. Constructed formats require two players, 20 life, at least 60 main-deck cards and at most 15 sideboard cards. Copy limits include the main deck and sideboard; Vintage restricted cards allow one total. Basic lands and Oracle-text copy-limit exceptions are honored. Commander uses 40 life, two to four players, 100 total cards, singleton restrictions, color identity, and a commander or compatible pair. Commander engine setup still checks runtime commander rules and commander damage.

Public rooms use Trusted mode (open decklists). The host checks submitted card names against the local catalog rather than trusting a guest's ready flag. Every participant checks the match's decks before engine startup; rematches pass the same check. Constructed format names stay in the lobby protocol but map to the engine's normal gameplay variant. Commander maps to its existing engine variant. Companion designation is not added by this feature; sideboard cards do not automatically become designated companions.

Regenerate the legality catalog whenever the repository's Scryfall data changes:

```sh
cd web/ui
pnpm formats:build
```

The generated asset records the Scryfall snapshot date and SHA-256 of `cards.json`; the lobby displays the date. It is fetched only when creating/joining a relay lobby and is content-hashed in production. Legality is as current as that snapshot, not a live check of today's bans or rotations. Unknown cards fail closed, including cards absent from this repository's filtered dataset. Engine support is checked separately by the existing match validation.

## Free-plan usage and lifecycle

The implementation uses `acceptWebSocket`, WebSocket attachments, and automatic ping/pong responses, so idle rooms can hibernate. There are no server intervals. Logical end-to-end heartbeats run every 30 seconds on this transport; socket ping/pong uses automatic responses. Directory refresh runs every 30 seconds while visible; advertising hosts renew once per minute. Listings expire after 150 seconds, and are removed when the host disconnects, stops advertising, starts a match, or fills the table. Empty rooms and their identity credentials are eventually deleted by an alarm.

The directory is bounded to 200 listings. Rooms have at most eight sockets (including reconnect overlap), sixteen reserved peer identities, 128 KiB frames, and a per-socket frame rate limit. Large messages are chunked with bounded client queues. Sender identities are assigned by authenticated sockets, and routing is scoped to one room. Besides `offer`/`answer`/`data`/`close`, the relay forwards `rtc` frames (`{type:'rtc', to, connectionId, signal}` → `{type:'rtc', from, connectionId, signal}`, `signal` a plain object) for WebRTC signaling. Reconnect tokens travel in the first WebSocket frame, not the URL. Origin checks are a browser boundary, not an account-based abuse prevention service.

Cloudflare currently includes 100,000 Durable Object requests/day, 13,000 GB-s/day, 5 million rows read/day, 100,000 rows written/day, and 5 GB storage on Free. The front Worker has separate request limits. Free quotas are finite: operations fail after limits are exceeded, until the applicable reset. Hibernation and throttled directory polling reduce use but cannot guarantee that arbitrary traffic stays within the free allowance. Monitor usage in the Cloudflare dashboard; this configuration does not upgrade the account or enable paid overages.

Sources: [Durable Object pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/), [limits](https://developers.cloudflare.com/durable-objects/platform/limits/), [hibernation API](https://developers.cloudflare.com/durable-objects/best-practices/websockets/), [official Magic formats](https://magic.wizards.com/en/formats), [banned/restricted lists](https://magic.wizards.com/en/banned-restricted-list).

## Direct-first transport

A relay connection opens over the room WebSocket immediately, then both browsers try a WebRTC data channel, signaled with `rtc` frames (STUN from `VITE_PEER_ICE_SERVERS`, else Google's public STUN). Once each side's channel is open it queues a `switch` marker behind its relay traffic and sends everything after it directly; the receiver holds direct messages until the marker arrives, so order is preserved. From then on the room socket only carries its 30-second auto-answered ping, so a direct game costs a handful of Durable Object requests for signaling instead of roughly one per 20 game messages. If the channel never opens (strict NATs, no TURN), the connection simply stays on the relay. If a channel that already carried traffic drops, the connection closes and the lobby's reconnect/resync replaces it with a relay-only connection. A direct connection survives a relay socket drop.

Direct channels show each peer the other's IP addresses. "Hide my IP address" in the lobby sheet (stored per browser as `ironsmith-relay-only-v1`) keeps every byte on the relay and never creates an `RTCPeerConnection`; either side opting out keeps that pair on the relay.

## Tournament witness

Tournament rooms (`securityMode: 'verified'` plus a 64-hex `tournamentId` in the host's `auth` frame) get a small witness that never sees game state. It only signs payloads defined in `web/ui/src/lib/tournament/witness-protocol.js`:

- `POST /witness/redeem` (JSON body sent as `text/plain`, so no CORS preflight) checks an organizer-signed invite plus a proof-of-possession by the player's audit key, binds invite ↔ key in the `TournamentRegistry` Durable Object (one per tournament), and returns a signed player certificate. `GET /witness/key` returns the witness public key.
- Over the room WebSocket, `{type:'witness', id, op, body}` frames (`genesis`, `challenge`, `answer`, `status`) are answered with `{type:'witness_result', id, ok, result|error}`. The host gets a signed genesis attestation binding certified keys to seats. A seat can open a challenge against another; the `WitnessDisputes` Durable Object (one per room+match) pushes it to the accused as `{type:'witness_event', event:'challenge'}`, forwards an in-time answer to the claimant (`event:'answer'`), and otherwise signs a forfeit (`event:'forfeit'`) when a storage alarm fires at the deadline. `status` lets a reconnecting peer catch up and also decides overdue challenges lazily.

Tournament rooms are never listed in the public directory. Trusted rooms are unchanged; old clients that send no `securityMode` get the same config as before. The `v2` migration adds both classes with `new_sqlite_classes` (required on Free).

Key setup is automatic: `pnpm build` in `web/ui` first runs `scripts/ensure-witness.mjs`, which asks the deployed relay for `/witness/key` and then:

- **404** (the deployed relay predates the witness): runs `wrangler deploy` once.
- **503** (no key yet): generates a P-256 key, backs it up at `~/.config/ironsmith/witness-signing-key.json` (mode 600; override with `IRONSMITH_WITNESS_KEY_FILE`), and uploads it with `wrangler secret put WITNESS_SIGNING_KEY`. Worker secrets cannot be read back, so that file is the only copy: keep it private and backed up, and never commit it.
- **200**: pins the served public key as `VITE_WITNESS_PUBLIC_KEYS` in `web/ui/.env.production.local` (git-ignored). If a local backup exists and disagrees with the relay's key, the build stops.

After the first run, builds need only network access to the relay, not a wrangler login or the private key. `IRONSMITH_SKIP_WITNESS=1` skips the step; `IRONSMITH_WITNESS_DRY_RUN=1` prints the wrangler commands instead of running them. Pinned builds reject any other witness; unpinned builds (development) trust the key the relay advertises, and the transcript verifier reports `witnessKeyPinned: false`. Rotating the key invalidates certificates and forfeits already issued. For `wrangler dev`, put `WITNESS_SIGNING_KEY=<json>` in `.dev.vars`. `WITNESS_ANSWER_WINDOW_MS` optionally overrides the 120 s answer window (tests use it).

Quota impact: redeeming costs 1 Worker request + 1 DO request per player per tournament. Genesis and challenge traffic rides the existing room socket (billed like other frames); opening, answering or checking a challenge adds one `WitnessDisputes` request, and an unanswered challenge adds one alarm plus one internal delivery request. Disputes are expected to be rare. Nothing polls.

## Recovery and limits

Socket reconnect preserves the peer identity and resumes through the existing lobby resync protocol. To recover after a refresh or closing a tab, reopen the lobby link and join from the same browser profile and site origin. Each browser stores a room-specific identity/token in localStorage; the public link never contains those credentials. The host also saves its match checkpoint and action transcript to IndexedDB after match start and each committed action. Rejoining as that host replays the saved accepted actions before accepting connections. Replay preserves pending decisions such as surveil and scry, which bare WASM checkpoint import does not preserve. Guest rejoining restores the original seat and replays the current accepted transcript from the host. If the host rejects a speculative trusted action, it sends its accepted state to repair clients that already applied that action. A second tab taking the same seat stops the old tab's reconnect loop.

The host must return before play can continue; host migration remains disabled. Clearing site data, switching devices/profiles/origins, or unavailable browser storage prevents seat recovery. A saved identity cannot claim another player's seat, and the public link alone cannot reclaim a disconnected seat. Existing game timeout/forfeit rules still apply. Empty rooms retain identities for at least 24 hours before cleanup; expired identities fail explicitly. The relay forwards messages without storing game snapshots or becoming an authoritative game server. Trusted mode is not cryptographic anticheat.


## Tests

```sh
# From repository root, after installing web/relay and web/ui dependencies:
node --test web/ui/tests/relay-format.test.mjs
node --test --test-concurrency=1 web/relay/tests/relay.test.mjs web/relay/tests/witness.test.mjs web/ui/tests/relay-browser.test.mjs web/ui/tests/relay-direct.test.mjs web/ui/tests/tournament-witness.test.mjs
```

The integration tests use the real local Cloudflare runtime and Chromium. Lobby/game protocol tests use the existing fake-engine harness; they are not a complete rules-engine conformance suite.
