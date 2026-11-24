// The lobby opens peers (a PeerJS peer, the relay WebSocket peer, a LAN peer)
// through this seam so the session code never imports a transport. A host
// registers its factory once; the web app registers the browser one in
// hooks/peer-lobby/browser-peers.js and Manabrew registers its room transport.
let peerFactory = null;

export function registerPeerFactory(factory) {
  peerFactory = factory;
}

export function createPeer(peerId, options) {
  if (!peerFactory) throw new Error("No peer transport registered for this session host");
  return peerFactory(String(peerId || "").trim(), options);
}
