# React + Vite

This template provides a minimal setup to get React working in Vite with HMR and some ESLint rules.

Currently, two official plugins are available:

- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react) uses [Babel](https://babeljs.io/) (or [oxc](https://oxc.rs) when used in [rolldown-vite](https://vite.dev/guide/rolldown)) for Fast Refresh
- [@vitejs/plugin-react-swc](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react-swc) uses [SWC](https://swc.rs/) for Fast Refresh

## React Compiler

The React Compiler is not enabled on this template because of its impact on dev & build performances. To add it, see [this documentation](https://react.dev/learn/react-compiler/installation).

## Expanding the ESLint configuration

If you are developing a production application, we recommend using TypeScript with type-aware lint rules enabled. Check out the [TS template](https://github.com/vitejs/vite/tree/main/packages/create-vite/template-react-ts) for information on how to integrate TypeScript and [`typescript-eslint`](https://typescript-eslint.io) in your project.

## Local network multiplayer

Run this on one computer connected to the players' LAN:

```bash
cd /Users/chiplis/ironsmith/web/ui
pnpm lan
```

Open the **same Network URL printed by Vite** (for example, `http://192.168.1.50:5173`) on every device. Create a lobby on one device. On the other devices, open **Join Lobby**, find the host under **Local network lobbies**, select it, and join. Listings refresh every three seconds. Full lobbies, started matches, and disconnected hosts disappear from the directory. Lobby codes still work for reconnecting.

This mode replaces PeerJS with the browser's native WebRTC data channels. The local process serves the game, advertises lobbies, and exchanges WebRTC offers/answers and ICE candidates. Game traffic travels directly between browsers. No PeerJS Cloud, STUN, TURN, internet signaling, or additional package is needed. The directory contains only lobbies using this server; browsers do not scan the LAN. Wi-Fi client isolation or a firewall blocking the server/WebRTC can prevent connections.

For a built UI instead of the development server:

```bash
pnpm build:lan
pnpm preview:lan
```

Keep the serving process running while playing so new joins and reconnects can discover peers. A static file host alone cannot provide the LAN directory or signaling endpoints. LAN builds must be paired with LAN preview; the normal build continues to use the online PeerJS mode.

### Verified mode over HTTPS

Trusted mode works at the HTTP LAN address. Verified mode needs WebCrypto, which requires a secure browser context. Use a certificate valid for the host's LAN address and trusted by **every participating device**, then run:

```bash
LAN_TLS_CERT=/absolute/path/lan-cert.pem LAN_TLS_KEY=/absolute/path/lan-key.pem pnpm lan
```

The same variables work with `pnpm preview:lan`. Open the printed HTTPS Network URL on every device. A certificate warning bypass is not a substitute for installing a trusted certificate. `localhost` is secure for development but points to each device itself.

Run the service and real Chromium transport/lobby tests with `pnpm test:lan`.

## Online multiplayer signaling

The multiplayer lobby uses PeerJS for signaling. By default it connects to PeerJS Cloud (`0.peerjs.com:443`). If one of your networks drops that websocket, run the bundled PeerServer instead:

```bash
cd /Users/chiplis/ironsmith/web/ui
pnpm signal
```

Then point both UI clients at the same signaling server with a local `.env.local`:

```bash
VITE_PEER_HOST=192.168.1.50
VITE_PEER_PORT=9000
VITE_PEER_PATH=/peerjs
VITE_PEER_KEY=peerjs
VITE_PEER_SECURE=false
```

Use the host machine's LAN IP for `VITE_PEER_HOST`, not `0.0.0.0`. If you are serving the Vite dev app across machines, start it with `pnpm dev --host 0.0.0.0`.

## Public WebSocket lobbies

Set `VITE_LOBBY_RELAY_URL` to enable the alternative WebSocket transport and public lobby search. Public rooms require a format and enforce its deck restrictions and game setup. The backend runs on Cloudflare Workers Free with SQLite Durable Objects. See [relay setup, deployment, format data, and limits](../relay/README.md).

### ICE/TURN for restrictive networks

The client accepts an optional `VITE_PEER_ICE_SERVERS` JSON array and passes it
to WebRTC. Public STUN servers can improve address discovery, but they do not
relay traffic. For peers that cannot connect directly, configure a TURN relay
you control (for example coturn) in both clients' `.env.local` files:

```bash
VITE_PEER_ICE_SERVERS=[{"urls":["stun:stun.l.google.com:19302"]},{"urls":"turn:turn.example.com:3478","username":"user","credential":"pass"}]
```

Keep TURN credentials out of git and inject them at deployment time. No
browser-only P2P setup can guarantee connectivity when a device is offline or
the browser is suspended; TURN removes the common NAT traversal failure mode.

### Mana symbols and deployment

[Andrew Gioia’s Mana](https://github.com/andrewgioia/Mana) is pinned as the
`vendor/mana` Git submodule. `pnpm build` (also `pnpm build:lan`, `pnpm dev`,
and `pnpm lan`) initializes that submodule at its recorded commit and runs
`scripts/sync-mana-assets.mjs`. Git and access to GitHub are required on the
first build of a checkout. Subsequent builds reuse the pinned checkout.

The script copies upstream SVGs into `public/mana/svg`, builds colored costs
and hybrid/Phyrexian combinations in `public/mana/symbols`, and copies the
upstream attribution/license notice to `public/mana/NOTICE.md`. Vite includes
all of these in `dist/mana`; deploy the entire `dist` directory using the
existing `~/home/ironsmith` deployment workflow. Symbols need no runtime CDN
access. Asset URLs respect Vite’s base path. Unknown counter kinds retain
their text labels; unsupported mana codes retain a numeric/text fallback.

To intentionally upgrade Mana, update the submodule checkout, run
`pnpm assets:mana`, and commit the submodule pointer and generated manifest
(`src/lib/mana-assets.generated.js`). Generated public assets are ignored.

### Adaptive moonlit board

The game workspace mounts `ForgeBoard`, a lazy-loaded Three.js sanctuary beneath
existing DOM cards. Original moonlit terrain artwork provides the backdrop;
locally bundled CC0 Poly Haven rock and stone-ring glTF models provide the 3D
scenery. Stone rings contain animated moonwell ripples, with silver motes and cool
lighting. See `public/theme/forge-arena/CREDITS.md` for sources and the artwork prompt.

Measured zone type and player ownership select the environmental treatment:
library lecterns, recessed graveyard slabs, narrow exile fissures, command plinths, and hand
ledges. Battlefield and exile zones have no enclosing ring or corner outlines. These follow the actual player layout
rather than assuming two fixed seats. The engine's card packing remains authoritative.
Public turn/combat changes and clicking free scenery provide subtle light pulses.

`measure-forge.js` reads the actual battlefield, hand, and zone rectangles,
including attachments and scroll clipping. Existing battlefield packing remains
authoritative. Platforms grow with occupied space and wait at least 1.6 seconds
before shrinking. Dragging, target selection, and active zone-effect overlays hold
outgoing space. Resize and tab resume discard stale bounds. Imported scenery yields
to cards and controls. Empty mobile support lanes do not allocate scenery.

Rendering is capped at 30 FPS and 1.5 device pixel ratio. Reduced motion renders
only while layout changes; hidden tabs and lost contexts pause rendering. WebGL
failure uses the CSS moonlit background, leaving the game playable. Unmount releases
observers, animation frames, geometry, materials, and the WebGL context.

Run `pnpm test:forge-board` for layout tests, browser scenario fixtures, WebGL
fallback/recovery, reduced-motion idle checks, and a real WASM puzzle smoke test.
The latter requires the normal generated WASM/catalog assets used by Vite.
The visual fixture is available at `/tests/forge-board.html` while Vite runs.
Scenarios cover empty/first-card boards, 100 permanents, attachments, wipes and
blink, uneven multiplayer boards, control-change footprints, large hands, opened
zone viewers, interaction locks, resizing, and remounting. Fixtures exercise UI
geometry; the existing engine tests remain responsible for rules semantics.

The floor uses a rough `MeshStandardMaterial` under cool directional moonlight and
low hemisphere fill. Imported scenery, stone zone structures, and shadow-only
card geometry cast PCF soft shadows onto the floor. The painted albedo still has
baked artistic shading; it is not a fully modeled terrain or ray-traced scene.

Settings → Appearance exposes **Battlefield cards** (compact artwork/full cards)
and **Noncreature lands** (small artwork tiles/match battlefield cards). Both
compact options default on and persist locally. Live creature types take priority
over land types, so animated lands retain their creature presentation. Full hover
inspection and hand cards keep their existing presentation.

Each visible player seat receives a separate offscreen spotlight in its HUD accent
color (including player color overrides). The perspective player's light enters
from the bottom; opponents have separate sources across the upper edge. Spectators
use the nearest top/bottom edge. Sources follow visible seat rectangles, remain
steady on empty battlefields, and disappear when a mobile opponent is hidden.
The spotlights illuminate the floor and meshes with inverse-square falloff, soft
cone edges, and shadow maps; card artwork is not tinted. Shadows update only when
geometry or light placement changes. Player-color changes also render in reduced
motion mode.

Compact artwork tiles include a color-matched top name strip. Small lands use a
1.2:1 outline, with consecutive lands packed at six pixels between hit rectangles;
manual empty placement slots remain intact. Mixed rows retain full-sized nonland
objects, and live creature lands do not use the small-land treatment.
Hover highlights follow rectangular artwork frames. Floating inspection sits eight
pixels beside the source edge, flips left at the viewport boundary, and measures
untransformed frame dimensions so the opening animation does not skew placement.

Opponent row groups are reversed in screen space: resources at the far edge,
creatures toward the center. This applies to desktop, dense boards, and mobile.
Token artwork uses an arched crown with an inset name strip; ordinary cards keep
rectangular outlines.
