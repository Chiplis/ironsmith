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

<<<<<<< HEAD
## Public WebSocket lobbies

Set `VITE_LOBBY_RELAY_URL` to enable the alternative WebSocket transport and public lobby search. Public rooms require a format and enforce its deck restrictions and game setup. The backend runs on Cloudflare Workers Free with SQLite Durable Objects. See [relay setup, deployment, format data, and limits](../relay/README.md).
=======
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
>>>>>>> 8ed139bb9225234cec5289bf32ecf29ef7c24c7c
