const MAX_MESSAGE_BYTES = 256 * 1024;
const MAX_ACTION_BYTES = 128 * 1024;
const MAX_REPLAY = 500;

function json(data) {
  return JSON.stringify(data);
}

function response(status, body) {
  return new Response(json(body), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === "/health") return response(200, { ok: true });
    if (url.pathname !== "/lobby") return response(404, { error: "not_found" });
    const lobbyId = url.searchParams.get("id")?.trim();
    if (!lobbyId || lobbyId.length > 128) return response(400, { error: "invalid_lobby" });
    const id = env.LOBBIES.idFromName(lobbyId);
    return env.LOBBIES.get(id).fetch(request);
  },
};

export class LobbyRoom {
  constructor(state) {
    this.state = state;
    this.sessions = new Map();
  }

  async fetch(request) {
    if (request.headers.get("Upgrade")?.toLowerCase() !== "websocket") {
      return response(426, { error: "websocket_required" });
    }
    const pair = new WebSocketPair();
    const [client, server] = Object.values(pair);
    server.accept();
    const session = { socket: server, playerId: "", lastSequence: 0 };
    this.sessions.set(server, session);
    server.addEventListener("message", (event) => {
      this.handleMessage(session, event.data).catch((error) => {
        this.send(session, { type: "error", code: "server_error", message: String(error?.message || error) });
      });
    });
    server.addEventListener("close", () => this.sessions.delete(server));
    server.addEventListener("error", () => this.sessions.delete(server));
    return new Response(null, { status: 101, webSocket: client });
  }

  async handleMessage(session, raw) {
    if (typeof raw !== "string" || new TextEncoder().encode(raw).byteLength > MAX_MESSAGE_BYTES) {
      this.send(session, { type: "error", code: "message_too_large" });
      return;
    }
    let message;
    try { message = JSON.parse(raw); } catch { this.send(session, { type: "error", code: "invalid_json" }); return; }
    if (message?.type === "join") return this.join(session, message);
    if (message?.type === "ping") return this.send(session, { type: "pong", at: message.at ?? Date.now() });
    if (message?.type === "action") return this.action(session, message);
    if (message?.type === "resume") return this.resume(session, message);
    if (message?.type === "snapshot") return this.snapshot(session, message);
    this.send(session, { type: "error", code: "unknown_message" });
  }

  async join(session, message) {
    const playerId = String(message.playerId || "").trim();
    if (!playerId || playerId.length > 128) return this.send(session, { type: "error", code: "invalid_player" });
    session.playerId = playerId;
    const sequence = await this.sequence();
    this.send(session, { type: "joined", playerId, sequence });
    await this.replay(session, Number(message.lastSequence ?? 0));
  }

  async resume(session, message) {
    if (!session.playerId) return this.send(session, { type: "error", code: "join_required" });
    await this.replay(session, Number(message.lastSequence ?? 0));
  }

  async snapshot(session, message) {
    if (!session.playerId) return this.send(session, { type: "error", code: "join_required" });
    const snapshot = message.state;
    if (snapshot === undefined) return this.send(session, { type: "error", code: "invalid_snapshot" });
    const encoded = json(snapshot);
    if (new TextEncoder().encode(encoded).byteLength > MAX_ACTION_BYTES) {
      return this.send(session, { type: "error", code: "snapshot_too_large" });
    }
    const sequence = await this.sequence();
    await this.state.storage.put("snapshot", { sequence, state: snapshot, at: Date.now() });
    this.send(session, { type: "snapshot_ack", sequence });
  }

  async action(session, message) {
    if (!session.playerId) return this.send(session, { type: "error", code: "join_required" });
    const actionId = String(message.actionId || "").trim();
    if (!actionId || actionId.length > 256) return this.send(session, { type: "error", code: "invalid_action_id" });
    const payload = message.payload;
    const encoded = json(payload);
    if (new TextEncoder().encode(encoded).byteLength > MAX_ACTION_BYTES) return this.send(session, { type: "error", code: "action_too_large" });
    const key = `action:${actionId}`;
    const existing = await this.state.storage.get(key);
    if (existing) return this.send(session, { type: "action_ack", actionId, sequence: existing.sequence, duplicate: true });
    const sequence = (await this.sequence()) + 1;
    const record = { actionId, playerId: session.playerId, payload, sequence, at: Date.now() };
    await this.state.storage.put(key, record);
    await this.state.storage.put(`sequence:${sequence}`, record);
    await this.state.storage.put("meta:sequence", sequence);
    this.broadcast({ type: "action", ...record });
    this.send(session, { type: "action_ack", actionId, sequence, duplicate: false });
  }

  async sequence() { return Number((await this.state.storage.get("meta:sequence")) || 0); }

  async replay(session, lastSequence) {
    const current = await this.sequence();
    const from = Math.max(0, Number.isFinite(lastSequence) ? lastSequence : 0);
    const snapshot = await this.state.storage.get("snapshot");
    if (snapshot && from < snapshot.sequence && current - from > MAX_REPLAY) {
      this.send(session, { type: "snapshot_required", sequence: snapshot.sequence, snapshot: snapshot.state });
      session.lastSequence = snapshot.sequence;
      return;
    }
    const records = [];
    for (let sequence = from + 1; sequence <= current && records.length < MAX_REPLAY; sequence += 1) {
      const record = await this.state.storage.get(`sequence:${sequence}`);
      if (record) records.push(record);
    }
    const complete = records.length === current - from;
    this.send(session, { type: "replay", from: from + 1, to: current, actions: records, complete });
    session.lastSequence = complete ? current : from + records.length;
  }

  send(session, message) { try { session.socket.send(json(message)); } catch { this.sessions.delete(session.socket); } }
  broadcast(message) { for (const session of this.sessions.values()) this.send(session, message); }
}
