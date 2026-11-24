// Registers the browser transports (PeerJS, relay WebSocket, LAN) with the
// session code. Imported for its side effect by the web app's hook.
import Peer from "peerjs";
import { NativeLanPeer } from "../../lib/lan/native-peer.js";
import { WebSocketPeer } from "../../lib/relay/websocket-peer.js";
import { registerPeerFactory } from "../../lib/verified-session/peer-factory.js";

registerPeerFactory((requestedPeerId, options) => {
  if (options?.transport === "websocket") return new WebSocketPeer(requestedPeerId, options);
  if (options?.transport === "lan") return new NativeLanPeer(requestedPeerId);
  // Reclaiming a saved ID can legitimately report unavailable-id while an
  // old socket closes or another host is elected. The error event is handled
  // by recovery and surfaced in status; avoid PeerJS's duplicate console errors.
  const peer = requestedPeerId ? new Peer(requestedPeerId, { ...options, debug: 0 }) : new Peer(options);
  const release = () => peer.destroy();
  window.addEventListener('pagehide', release, { once: true });
  peer.on('close', () => window.removeEventListener('pagehide', release));
  return peer;
});
