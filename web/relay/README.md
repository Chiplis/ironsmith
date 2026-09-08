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

The directory is bounded to 200 listings. Rooms have at most eight sockets (including reconnect overlap), sixteen reserved peer identities, 128 KiB frames, and a per-socket frame rate limit. Large messages are chunked with bounded client queues. Sender identities are assigned by authenticated sockets, and routing is scoped to one room. Reconnect tokens travel in the first WebSocket frame, not the URL. Origin checks are a browser boundary, not an account-based abuse prevention service.

Cloudflare currently includes 100,000 Durable Object requests/day, 13,000 GB-s/day, 5 million rows read/day, 100,000 rows written/day, and 5 GB storage on Free. The front Worker has separate request limits. Free quotas are finite: operations fail after limits are exceeded, until the applicable reset. Hibernation and throttled directory polling reduce use but cannot guarantee that arbitrary traffic stays within the free allowance. Monitor usage in the Cloudflare dashboard; this configuration does not upgrade the account or enable paid overages.

Sources: [Durable Object pricing](https://developers.cloudflare.com/durable-objects/platform/pricing/), [limits](https://developers.cloudflare.com/durable-objects/platform/limits/), [hibernation API](https://developers.cloudflare.com/durable-objects/best-practices/websockets/), [official Magic formats](https://magic.wizards.com/en/formats), [banned/restricted lists](https://magic.wizards.com/en/banned-restricted-list).

## Recovery and limits

Socket reconnect preserves the peer identity and resumes through the existing lobby resync protocol. To recover after a refresh or closing a tab, reopen the lobby link and join from the same browser profile and site origin. Each browser stores a room-specific identity/token in localStorage; the public link never contains those credentials. The host also saves its match checkpoint and action transcript to IndexedDB after match start and each committed action. Rejoining as that host replays the saved accepted actions before accepting connections. Replay preserves pending decisions such as surveil and scry, which bare WASM checkpoint import does not preserve. Guest rejoining restores the original seat and replays the current accepted transcript from the host. If the host rejects a speculative trusted action, it sends its accepted state to repair clients that already applied that action. A second tab taking the same seat stops the old tab's reconnect loop.

The host must return before play can continue; host migration remains disabled. Clearing site data, switching devices/profiles/origins, or unavailable browser storage prevents seat recovery. A saved identity cannot claim another player's seat, and the public link alone cannot reclaim a disconnected seat. Existing game timeout/forfeit rules still apply. Empty rooms retain identities for at least 24 hours before cleanup; expired identities fail explicitly. The relay forwards messages without storing game snapshots or becoming an authoritative game server. Trusted mode is not cryptographic anticheat.


## Tests

```sh
# From repository root, after installing web/relay and web/ui dependencies:
node --test web/ui/tests/relay-format.test.mjs
node --test web/relay/tests/relay.test.mjs web/ui/tests/relay-browser.test.mjs
```

The integration tests use the real local Cloudflare runtime and Chromium. Lobby/game protocol tests use the existing fake-engine harness; they are not a complete rules-engine conformance suite.
