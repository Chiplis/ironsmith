// Same-origin LAN discovery and WebRTC signaling. Game data never passes here.
export const LAN_PATH = "/__ironsmith_lan";

export function createLanService({ leaseMs = 15_000 } = {}) {
  const peers = new Map();
  const validId = (value) => typeof value === "string" && /^[\w-]{1,80}$/.test(value);
  const send = (peer, message) => {
    if (!peer?.stream || peer.stream.destroyed) return false;
    // A slow signaling consumer must not retain an unbounded response buffer.
    if (peer.stream.writableLength > 1024 * 1024) {
      peer.stream.destroy();
      return false;
    }
    peer.stream.write(`data: ${JSON.stringify(message)}\n\n`);
    return true;
  };
  const timer = setInterval(() => {
    for (const [id, peer] of peers) {
      if (!peer.stream && Date.now() - peer.lastSeen > leaseMs) peers.delete(id);
      else if (peer.stream) peer.stream.write(": heartbeat\n\n");
    }
  }, Math.min(5000, leaseMs));
  timer.unref();

  async function middleware(req, res, next) {
    const url = new URL(req.url, "http://localhost");
    if (!url.pathname.startsWith(`${LAN_PATH}/`)) return next();
    const reply = (status, body) => {
      res.writeHead(status, { "Content-Type": "application/json", "Cache-Control": "no-store" });
      res.end(JSON.stringify(body));
    };
    // No cross-origin access to the local directory or signaling service.
    if ((req.headers.origin && new URL(req.headers.origin).host !== req.headers.host)
      || req.headers["sec-fetch-site"] === "cross-site") return reply(403, { error: "Same-origin requests required" });
    if (url.pathname === `${LAN_PATH}/lobbies` && req.method === "GET") {
      return reply(200, { lobbies: [...peers.values()]
        .filter((peer) => peer.stream && peer.lobby)
        .map((peer) => ({ id: peer.id, ...peer.lobby })) });
    }
    const id = url.searchParams.get("id");
    const token = url.searchParams.get("token");
    if (!validId(id) || !validId(token) || token.length < 32) return reply(400, { error: "Invalid identity" });
    let peer = peers.get(id);
    if (url.pathname === `${LAN_PATH}/events` && req.method === "GET") {
      if (peer && peer.token !== token) return reply(409, { error: "Peer ID is in use" });
      if (!peer && peers.size >= 128) return reply(503, { error: "LAN service is full" });
      if (!peer) {
        peer = { id, token, stream: null, lobby: null, lastSeen: Date.now() };
        peers.set(id, peer);
      }
      const oldStream = peer.stream;
      peer.stream = res;
      oldStream?.end();
      res.writeHead(200, {
        "Content-Type": "text/event-stream", "Cache-Control": "no-store",
        Connection: "keep-alive", "X-Accel-Buffering": "no",
      });
      send(peer, { type: "open", id });
      res.on("close", () => {
        if (peer.stream === res) {
          peer.stream = null;
          peer.lastSeen = Date.now();
        }
      });
      return;
    }
    if (!peer || peer.token !== token) return reply(403, { error: "Unknown peer" });
    if (req.method === "DELETE" && url.pathname === `${LAN_PATH}/peer`) {
      peers.delete(id);
      peer.stream?.end();
      return reply(200, {});
    }
    if (req.method !== "POST") return reply(405, { error: "Method not allowed" });
    try {
      let body = "";
      for await (const chunk of req) {
        body += chunk;
        if (body.length > 128 * 1024) return reply(413, { error: "Message too large" });
      }
      const message = JSON.parse(body);
      if (url.pathname === `${LAN_PATH}/lobby`) {
        peer.lobby = message && message.available === true ? {
          name: String(message.name || "Host").slice(0, 60),
          format: ["normal", "commander", "planechase"].includes(message.format) ? message.format : "normal",
          securityMode: message.securityMode === "verified" ? "verified" : "trusted",
          playerCount: Math.max(1, Math.min(4, Number(message.playerCount) || 1)),
          desiredPlayers: Math.max(2, Math.min(4, Number(message.desiredPlayers) || 2)),
        } : null;
        return reply(200, {});
      }
      if (url.pathname === `${LAN_PATH}/signal`) {
        if (!["offer", "answer", "candidate", "close"].includes(message.type)
          || !validId(message.connectionId) || !validId(message.to)) return reply(400, { error: "Invalid signal" });
        const delivered = send(peers.get(message.to), { ...message, from: id });
        return reply(delivered ? 200 : 404, delivered ? {} : { error: "Peer is unavailable" });
      }
      return reply(404, { error: "Unknown endpoint" });
    } catch {
      if (!res.headersSent) reply(400, { error: "Invalid request" });
    }
  }
  return {
    middleware,
    close() {
      clearInterval(timer);
      for (const peer of peers.values()) peer.stream?.end();
      peers.clear();
    },
  };
}

export function lanLobbyPlugin() {
  const install = (server) => {
    const service = createLanService();
    server.middlewares.use((req, res, next) => {
      service.middleware(req, res, next).catch(() => {
        if (!res.headersSent) res.writeHead(500);
        res.end();
      });
    });
    server.httpServer?.once("close", () => service.close());
  };
  return { name: "ironsmith-lan-lobbies", configureServer: install, configurePreviewServer: install };
}
