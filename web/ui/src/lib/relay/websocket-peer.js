import { readRelaySession, saveRelayIdentity } from './session.js';
import { isRelayId, relayBaseUrl } from './formats.js';
const id = () => Array.from(crypto.getRandomValues(new Uint8Array(16)), n => n.toString(16).padStart(2, '0')).join('');
const MAX_MESSAGE = 64 * 1024 * 1024;
const CHUNK = 16000;
const encoder = new TextEncoder();
const bytes = value => encoder.encode(value).byteLength;
const CONTROL = new Set(['peer_heartbeat', 'peer_heartbeat_ack', 'resync_ack', 'action_ack', 'action_error', 'apply_action', 'trusted_command', 'trusted_command_ack', 'trusted_action_ack', 'trusted_command_error', 'trusted_recovery_needed']);
const FRAME_RATE = 160, FRAME_BURST = 8;
const ASSEMBLY_TIMEOUT = 120_000;
class Queue {
  entries = []; head = 0;
  get length() { return this.entries.length - this.head; }
  push(value) { this.entries.push(value); }
  first() { return this.entries[this.head]; }
  remove() {
    this.entries[this.head++] = null;
    if (this.head >= 1024 && this.head * 2 >= this.entries.length) {
      this.entries = this.entries.slice(this.head); this.head = 0;
    }
  }
}

class Events {
  listeners = new Map();
  on(type, fn) { this.listeners.set(type, [...(this.listeners.get(type) || []), fn]); return this; }
  emit(type, ...args) { for (const fn of this.listeners.get(type) || []) fn(...args); }
}
class RelayConnection extends Events {
  open = false;
  closed = false;
  incoming = new Map();
  incomingBytes = 0;
  nextMessage = 0;
  wireVersion = 1;
  constructor(owner, peer, connectionId, metadata = {}) {
    super(); Object.assign(this, { owner, peer, connectionId, metadata });
    this.timeout = setTimeout(() => this.close(), 15000);
  }
  signal(type, extra = {}) { this.owner.send({ type, to: this.peer, connectionId: this.connectionId, ...extra }); }
  opened() { if (this.closed || this.open) return; clearTimeout(this.timeout); this.open = true; this.emit('open'); }
  receive(message) {
    if (this.closed) return;
    if (message.type === 'answer') { this.wireVersion = message.metadata?.relayWireVersion === 2 ? 2 : 1; this.opened(); }
    else if (message.type === 'close') this.close(false);
    else if (message.type === 'data') {
      const frame = message.data;
      // Accept legacy peers, but new peers identify chunks so control traffic
      // can pass an unfinished bulk message without corrupting its assembly.
      const legacy = typeof frame === 'string';
      const messageId = legacy ? 'legacy' : frame?.id;
      const text = legacy ? frame.slice(1) : frame?.text;
      const last = legacy ? frame[0] === '.' : frame?.last;
      if ((legacy && !['+', '.'].includes(frame[0])) || (!legacy && (frame?.v !== 2
          || !Number.isSafeInteger(messageId) || messageId < 1 || typeof last !== 'boolean'))
          || typeof text !== 'string') throw new Error('Invalid relay data');
      let assembly = this.incoming.get(messageId);
      if (!assembly) {
        if (this.incoming.size >= 8) throw new Error('Too many partial relay messages');
        assembly = { chunks: [], bytes: 0, index: 0, timer: setTimeout(() => {
          this.emit('error', new Error('Relay message assembly timed out')); this.close();
        }, ASSEMBLY_TIMEOUT) };
        this.incoming.set(messageId, assembly);
      }
      if (!legacy && frame.index !== assembly.index) throw new Error('Out-of-order relay chunk');
      const size = bytes(text);
      if (assembly.bytes + size > MAX_MESSAGE || this.owner.incomingBytes + size > MAX_MESSAGE * 2) throw new Error('Relay message too large');
      assembly.chunks.push(text); assembly.index++; assembly.bytes += size;
      this.incomingBytes += size; this.owner.incomingBytes += size;
      if (last) {
        clearTimeout(assembly.timer); this.incoming.delete(messageId);
        this.incomingBytes -= assembly.bytes; this.owner.incomingBytes -= assembly.bytes;
        const payload = JSON.parse(assembly.chunks.join(''));
        if (['lobby_state', 'match_start'].includes(payload.type)) {
          const config = this.owner.config;
          if (payload.format !== config.format || payload.securityMode !== 'trusted') throw new Error('Lobby rules do not match the relay room');
        }
        this.emit('data', payload, { bytes: assembly.bytes });
      }
    }
  }
  send(payload) {
    if (!this.open) throw new Error('Relay connection closed');
    const data = JSON.stringify(payload), size = bytes(data);
    if (size > MAX_MESSAGE) throw new Error('Relay message too large');
    let offset = 0, index = 0;
    const messageId = ++this.nextMessage;
    this.owner.enqueueMessage({ bytes: size, connection: this, next: () => {
      let end = Math.min(data.length, offset + CHUNK);
      if (end < data.length && data.charCodeAt(end - 1) >= 0xd800 && data.charCodeAt(end - 1) <= 0xdbff) end--;
      const text = data.slice(offset, end); offset = end;
      const last = end === data.length;
      const frame = JSON.stringify({ type: 'data', to: this.peer, connectionId: this.connectionId,
        data: this.wireVersion === 2 ? { v: 2, id: messageId, index: index++, last, text } : `${last ? '.' : '+'}${text}` });
      return { frame, bytes: bytes(text), done: last };
    } }, this.wireVersion === 2 && CONTROL.has(payload?.type));
    return { bytes: size };
  }

  close(notify = true) {
    if (this.closed) return;
    this.closed = true; this.open = false; clearTimeout(this.timeout);
    for (const assembly of this.incoming.values()) clearTimeout(assembly.timer);
    this.incoming.clear(); this.owner.incomingBytes -= this.incomingBytes; this.incomingBytes = 0;
    this.owner.connections.delete(this.connectionId);
    if (notify && this.owner.open) this.signal('close');
    this.emit('close');
  }
}

// PeerJS-compatible logical connections carried by one room WebSocket.
export class WebSocketPeer extends Events {
  open = false;
  destroyed = false;
  disconnected = true;
  connections = new Map();
  queue = new Queue();
  controlQueue = new Queue();
  queuedSize = 0;
  incomingBytes = 0;
  tokens = FRAME_BURST;
  tokenTime = performance.now();
  constructor(peerId, options = {}) {
    super();
    this.options = { ...options, transport: 'websocket' };
    const room = options.room || (isRelayId(peerId) ? peerId.split('-')[1] : id());
    const saved = options.room ? readRelaySession(room, options.url) : null;
    this.id = peerId || saved?.peerId || `ws-${room}-${id()}`;
    this.room = room;
    this.token = saved?.token || id();
    this.resuming = Boolean(saved);
    queueMicrotask(() => this.reconnect());
  }
  reconnect() {
    if (this.destroyed || this.open || this.socket?.readyState === 0) return;
    const base = this.options.url || relayBaseUrl();
    if (!base) { this.emit('error', new Error('WebSocket lobby service is not configured')); return; }
    const url = new URL(`${base}/rooms/${this.room}/socket`);
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    url.searchParams.set('peer', this.id);
    const ws = new WebSocket(url);
    this.socket = ws;
    ws.onopen = () => ws.send(JSON.stringify({ type: 'auth', token: this.token, resume: this.resuming,
      format: this.options.format, desiredPlayers: this.options.desiredPlayers }));
    ws.onmessage = ({ data }) => {
      if (this.socket !== ws || this.destroyed) return;
      try {
        if (data === 'pong') { this.lastPong = Date.now(); return; }
        const msg = JSON.parse(data);
        if (msg.type === 'open') {
          saveRelayIdentity(this.room, this.options.url, { peerId: this.id, token: this.token, advertise: this.options.advertise });
          this.resuming = true;
          this.open = true; this.disconnected = false; this.config = msg.config;
          this.lastPong = Date.now(); this.lastPing = Date.now();
          this.pingTimer = setInterval(() => {
            if (Date.now() - this.lastPing > 90_000) this.lastPong = Date.now();
            this.lastPing = Date.now();
            if (Date.now() - this.lastPong > 90_000) { ws.close(); return; }
            ws.send('ping');
          }, 30_000);
          this.emit('open', this.id);
          if (this.lobby) this.advertise(this.lobby, true);
          return;
        }
        if (msg.type === 'error') { this.emit('error', new Error(msg.message)); return; }
        if (msg.type === 'offline') {
          for (const conn of [...this.connections.values()]) if (conn.peer === msg.peer) conn.close(false);
          return;
        }
        let conn = this.connections.get(msg.connectionId);
        if (msg.type === 'unavailable') { conn?.close(false); return; }
        if (!conn && msg.type === 'offer') {
          if (!isRelayId(msg.from) || msg.from.split('-')[1] !== this.room || this.connections.size >= 24) return;
          conn = new RelayConnection(this, msg.from, msg.connectionId, msg.metadata);
          this.connections.set(conn.connectionId, conn);
          this.emit('connection', conn);
          conn.wireVersion = msg.metadata?.relayWireVersion === 2 ? 2 : 1;
          conn.signal('answer', { metadata: { relayWireVersion: 2 } }); conn.opened();
        } else if (conn?.peer === msg.from) conn.receive(msg);
      } catch (error) { this.emit('error', error); ws.close(1008, 'Invalid frame'); }
    };
    ws.onclose = (event) => {
      if (this.socket !== ws) return;
      this.open = false; this.disconnected = true;
      clearInterval(this.pingTimer); clearTimeout(this.flushTimer); clearInterval(this.advertiseTimer);
      this.queue = new Queue(); this.controlQueue = new Queue(); this.queuedSize = 0;
      for (const conn of [...this.connections.values()]) conn.close(false);
      if (event.code === 4001) {
        this.destroyed = true;
        this.emit('error', new Error('This seat was reopened in another tab. Continue there.'));
      } else if (!this.destroyed) this.emit('disconnected');
    };
    ws.onerror = () => { const error = new Error('Could not reach WebSocket lobby service'); error.type = 'network'; this.emit('error', error); };
  }
  send(msg) { this.sendBatch([msg]); }
  sendBatch(messages) {
    if (!messages.length) return;
    if (!this.open) throw new Error('Relay is disconnected');
    const serialized = messages.map(msg => JSON.stringify(msg));
    const size = serialized.reduce((sum, frame) => sum + bytes(frame), 0);
    this.enqueueMessage({ bytes: size, next: (() => {
      let index = 0;
      return () => { const frame = serialized[index++]; return { frame, bytes: bytes(frame), done: index === serialized.length }; };
    })() }, true);
  }
  enqueueMessage(message, control) {
    if (!this.open) throw new Error('Relay is disconnected');
    const limit = MAX_MESSAGE * 2 + (control ? 256 * 1024 : 0);
    if (this.queuedSize + message.bytes > limit) {
      const error = new Error('Relay send queue full'); error.code = 'RELAY_BACKPRESSURE'; throw error;
    }
    message.remainingBytes = message.bytes;
    (control ? this.controlQueue : this.queue).push(message);
    this.queuedSize += message.bytes;
    this.flush();
  }
  flush() {
    clearTimeout(this.flushTimer); this.flushTimer = null;
    const now = performance.now();
    this.tokens = Math.min(FRAME_BURST, this.tokens + (now - this.tokenTime) * FRAME_RATE / 1000);
    this.tokenTime = now;
    while (this.open && (this.controlQueue.length || this.queue.length)
        && this.socket.bufferedAmount < 512 * 1024 && this.tokens >= 1) {
      const queue = this.controlQueue.length ? this.controlQueue : this.queue;
      const message = queue.first();
      if (message.connection?.closed) { this.queuedSize -= message.remainingBytes; queue.remove(); continue; }
      const chunk = message.pendingChunk ||= message.next();
      try { this.socket.send(chunk.frame); }
      catch (error) {
        this.emit('error', error);
        this.socket.close();
        return;
      }
      message.pendingChunk = null;
      this.tokens--;
      this.queuedSize -= chunk.bytes; message.remainingBytes -= chunk.bytes;
      if (chunk.done) queue.remove();
    }
    if (this.open && (this.controlQueue.length || this.queue.length)) {
      this.flushTimer = setTimeout(() => this.flush(), this.socket.bufferedAmount >= 512 * 1024 ? 25 : Math.max(1, Math.ceil((1 - this.tokens) * 1000 / FRAME_RATE)));
    }
  }
  connect(peer, options = {}) {
    const conn = new RelayConnection(this, peer, id(), options.metadata);
    this.connections.set(conn.connectionId, conn);
    queueMicrotask(() => {
      try { conn.signal('offer', { metadata: { ...conn.metadata, relayWireVersion: 2 } }); }
      catch (error) { conn.emit('error', error); conn.close(false); }
    });
    return conn;
  }
  advertise(lobby, force = false) {
    if (!force && JSON.stringify(lobby) === JSON.stringify(this.lobby)) return;
    this.lobby = lobby;
    clearInterval(this.advertiseTimer);
    if (!this.open || this.config?.host !== this.id) return;
    const publish = () => { if (this.open) this.send({ type: 'advertise', lobby: { ...this.lobby, available: this.options.advertise !== false && this.lobby.available } }); };
    publish();
    if (lobby.available && this.options.advertise !== false) this.advertiseTimer = setInterval(publish, 60_000);
  }
  destroy() {
    if (this.destroyed) return;
    this.destroyed = true;
    clearInterval(this.pingTimer); clearInterval(this.advertiseTimer); clearTimeout(this.flushTimer);
    for (const conn of [...this.connections.values()]) conn.close(false);
    this.open = false; this.disconnected = true; this.queue = new Queue(); this.controlQueue = new Queue(); this.queuedSize = 0;
    this.socket?.close(1000, 'Leaving lobby'); this.emit('close');
  }
}
