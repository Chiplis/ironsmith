# React + Vite

This template provides a minimal setup to get React working in Vite with HMR and some ESLint rules.

Currently, two official plugins are available:

- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react) uses [Babel](https://babeljs.io/) (or [oxc](https://oxc.rs) when used in [rolldown-vite](https://vite.dev/guide/rolldown)) for Fast Refresh
- [@vitejs/plugin-react-swc](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react-swc) uses [SWC](https://swc.rs/) for Fast Refresh

## React Compiler

The React Compiler is not enabled on this template because of its impact on dev & build performances. To add it, see [this documentation](https://react.dev/learn/react-compiler/installation).

## Expanding the ESLint configuration

If you are developing a production application, we recommend using TypeScript with type-aware lint rules enabled. Check out the [TS template](https://github.com/vitejs/vite/tree/main/packages/create-vite/template-react-ts) for information on how to integrate TypeScript and [`typescript-eslint`](https://typescript-eslint.io) in your project.

## Multiplayer Signaling

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
