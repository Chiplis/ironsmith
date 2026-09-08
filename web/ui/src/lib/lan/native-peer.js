const BASE = "/__ironsmith_lan";
const MAX_MESSAGE = 64 * 1024 * 1024;
const CHUNK_SIZE = 8000;
const id = () => Array.from(crypto.getRandomValues(new Uint8Array(24)), (byte) => byte.toString(16).padStart(2, "0")).join("");

class Events {
  listeners = new Map();
  on(name, callback) {
    const callbacks = this.listeners.get(name) || [];
    callbacks.push(callback);
    this.listeners.set(name, callbacks);
    return this;
  }
  emit(name, ...args) {
    for (const callback of this.listeners.get(name) || []) callback(...args);
  }
}

// Implements the small connection interface used by the lobby protocol.
class LanConnection extends Events {
  open = false;
  closed = false;
  queue = [];
  queuedSize = 0;
  incoming = "";
  candidates = [];
  localCandidates = [];
  descriptionSent = false;

  constructor(owner, peer, connectionId, metadata = {}) {
    super();
    this.owner = owner;
    this.peer = peer;
    this.connectionId = connectionId;
    this.metadata = metadata;
    this.peerConnection = new RTCPeerConnection({ iceServers: [] });
    this.peerConnection.onicecandidate = ({ candidate }) => {
      if (candidate) {
        const message = { type: "candidate", candidate: candidate.toJSON() };
        if (this.descriptionSent) this.signal(message);
        else this.localCandidates.push(message);
      }
    };
    this.peerConnection.onconnectionstatechange = () => {
      if (["failed", "closed"].includes(this.peerConnection.connectionState)) this.close();
    };
    this.peerConnection.ondatachannel = ({ channel }) => this.attach(channel);
    this.timeout = setTimeout(() => this.fail(new Error("Local WebRTC connection timed out")), 20_000);
  }

  signal(message) {
    return this.owner.signal({ ...message, to: this.peer, connectionId: this.connectionId })
      .catch((error) => { if (!this.closed) this.fail(error); });
  }

  attach(channel) {
    this.dataChannel = channel;
    channel.bufferedAmountLowThreshold = 128 * 1024;
    channel.onbufferedamountlow = () => this.flush();
    channel.onopen = () => {
      if (this.closed) return;
      clearTimeout(this.timeout);
      this.open = true;
      this.emit("open");
    };
    channel.onclose = () => this.close();
    channel.onerror = () => this.fail(new Error("Local data channel failed"));
    channel.onmessage = ({ data }) => {
      try {
        // Ordered reliable channels permit one contiguous chunked JSON message.
        if (typeof data !== "string" || !["+", "."].includes(data[0])) throw new Error("Invalid LAN frame");
        if (this.incoming.length + data.length > MAX_MESSAGE) throw new Error("LAN message too large");
        this.incoming += data.slice(1);
        if (data[0] === ".") {
          const payload = JSON.parse(this.incoming);
          this.incoming = "";
          this.emit("data", payload);
        }
      } catch (error) { this.fail(error); }
    };
  }

  async offer() {
    this.attach(this.peerConnection.createDataChannel("ironsmith", { ordered: true }));
    await this.peerConnection.setLocalDescription(await this.peerConnection.createOffer());
    await this.sendDescription("offer");
  }

  async sendDescription(type) {
    const result = this.signal({ type, description: this.peerConnection.localDescription.toJSON(), metadata: this.metadata });
    this.descriptionSent = true;
    for (const candidate of this.localCandidates.splice(0)) this.signal(candidate);
    await result;
  }

  async receive(message) {
    if (this.closed) return;
    if (message.type === "close") return this.close(false);
    if (message.type === "candidate") {
      if (this.peerConnection.remoteDescription) await this.peerConnection.addIceCandidate(message.candidate);
      else this.candidates.push(message.candidate);
      return;
    }
    await this.peerConnection.setRemoteDescription(message.description);
    for (const candidate of this.candidates.splice(0)) await this.peerConnection.addIceCandidate(candidate);
    if (message.type === "offer") {
      await this.peerConnection.setLocalDescription(await this.peerConnection.createAnswer());
      await this.sendDescription("answer");
    }
  }

  send(payload) {
    if (!this.open) throw new Error("LAN connection is closed");
    const serialized = JSON.stringify(payload);
    if (serialized.length > MAX_MESSAGE || this.queuedSize + serialized.length > MAX_MESSAGE * 2) {
      this.fail(new Error("LAN send queue is full"));
      throw new Error("LAN send queue is full");
    }
    for (let offset = 0; offset < serialized.length;) {
      let end = Math.min(serialized.length, offset + CHUNK_SIZE);
      const last = serialized.charCodeAt(end - 1);
      if (end < serialized.length && last >= 0xd800 && last <= 0xdbff) end -= 1;
      this.queue.push((end === serialized.length ? "." : "+") + serialized.slice(offset, end));
      offset = end;
    }
    this.queuedSize += serialized.length;
    this.flush();
  }

  flush() {
    try {
      while (this.open && this.queue.length && this.dataChannel.bufferedAmount < 512 * 1024) {
        const chunk = this.queue.shift();
        this.queuedSize -= chunk.length - 1;
        this.dataChannel.send(chunk);
      }
    } catch (error) { this.fail(error); }
  }

  fail(error) {
    if (this.closed) return;
    this.emit("error", error);
    this.close();
  }

  close(notify = true) {
    if (this.closed) return;
    this.closed = true;
    this.open = false;
    clearTimeout(this.timeout);
    this.queue = [];
    this.incoming = "";
    this.peerConnection.close();
    this.owner.connections.delete(this.connectionId);
    if (notify && this.owner.open) this.signal({ type: "close" });
    this.emit("close");
  }
}

export class NativeLanPeer extends Events {
  open = false;
  disconnected = true;
  destroyed = false;
  connections = new Map();
  inbound = Promise.resolve();
  outbound = Promise.resolve();

  constructor(peerId) {
    super();
    this.id = peerId || id();
    this.token = id();
    queueMicrotask(() => this.reconnect());
  }

  url(endpoint) {
    return `${BASE}/${endpoint}?id=${encodeURIComponent(this.id)}&token=${this.token}`;
  }

  reconnect() {
    if (this.destroyed || this.open) return;
    this.events?.close();
    const events = new EventSource(this.url("events"));
    this.events = events;
    events.onmessage = ({ data }) => {
      this.inbound = this.inbound.then(async () => {
        if (this.destroyed || this.events !== events) return;
        const message = JSON.parse(data);
        if (message.type === "open") {
          this.open = true;
          this.disconnected = false;
          this.emit("open", this.id);
          if (this.lobby) this.advertise(this.lobby, true);
          return;
        }
        let connection = this.connections.get(message.connectionId);
        if (!connection && message.type === "offer") {
          connection = new LanConnection(this, message.from, message.connectionId, message.metadata);
          this.connections.set(message.connectionId, connection);
          this.emit("connection", connection);
        }
        if (connection && connection.peer === message.from) {
          try { await connection.receive(message); } catch (error) { connection.fail(error); }
        }
      }).catch((error) => this.emit("error", error));
    };
    events.onerror = () => {
      if (this.destroyed || this.events !== events) return;
      events.close();
      this.open = false;
      this.disconnected = true;
      this.emit("disconnected");
    };
  }

  async request(endpoint, message, method = "POST") {
    const response = await fetch(this.url(endpoint), {
      method, headers: { "Content-Type": "application/json" },
      ...(method === "POST" ? { body: JSON.stringify(message) } : {}),
      keepalive: method === "DELETE",
      signal: AbortSignal.timeout(10_000),
    });
    if (!response.ok) {
      const error = new Error((await response.json()).error || "LAN signaling failed");
      error.type = response.status === 404 ? "peer-unavailable" : "server-error";
      throw error;
    }
  }

  signal(message) {
    // Offers must precede their ICE candidates on the signaling channel.
    const result = this.outbound.then(() => this.request("signal", message));
    this.outbound = result.catch(() => {});
    return result;
  }

  connect(peerId, options = {}) {
    const connection = new LanConnection(this, peerId, id(), options.metadata);
    this.connections.set(connection.connectionId, connection);
    queueMicrotask(() => connection.offer().catch((error) => connection.fail(error)));
    return connection;
  }

  advertise(lobby, force = false) {
    if (!force && JSON.stringify(lobby) === JSON.stringify(this.lobby)) return;
    this.lobby = lobby;
    if (this.open) {
      // Share the ordered request queue so late lobby updates cannot re-list a match.
      this.outbound = this.outbound.then(() => this.request("lobby", lobby)).catch((error) => {
        if (!this.destroyed) this.emit("error", error);
      });
    }
  }

  destroy() {
    if (this.destroyed) return;
    this.destroyed = true;
    for (const connection of [...this.connections.values()]) connection.close(false);
    this.events?.close();
    this.open = false;
    this.disconnected = true;
    this.request("peer", null, "DELETE").catch(() => {});
    this.emit("close");
  }
}
