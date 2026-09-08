import { isRelayId, relayBaseUrl } from './formats.js';
const id = () => Array.from(crypto.getRandomValues(new Uint8Array(16)), n => n.toString(16).padStart(2, '0')).join('');
const MAX_MESSAGE = 64 * 1024 * 1024;
const CHUNK = 16000;
class Events {
  listeners = new Map();
  on(type, fn) { this.listeners.set(type, [...(this.listeners.get(type) || []), fn]); return this; }
  emit(type, ...args) { for (const fn of this.listeners.get(type) || []) fn(...args); }
}
class RelayConnection extends Events {
  open = false;
  closed = false;
  incoming = '';
  constructor(owner, peer, connectionId, metadata = {}) {
    super(); Object.assign(this, { owner, peer, connectionId, metadata });
    this.timeout = setTimeout(() => this.close(), 15000);
  }
  signal(type, extra = {}) { this.owner.send({ type, to: this.peer, connectionId: this.connectionId, ...extra }); }
  opened() { if (this.closed || this.open) return; clearTimeout(this.timeout); this.open = true; this.emit('open'); }
  receive(message) {
    if (this.closed) return;
    if (message.type === 'answer') this.opened();
    else if (message.type === 'close') this.close(false);
    else if (message.type === 'data') {
      const frame = message.data;
      if (typeof frame !== 'string' || !['+', '.'].includes(frame[0]) || this.incoming.length + frame.length > MAX_MESSAGE) throw new Error('Invalid relay data');
      this.incoming += frame.slice(1);
      if (frame[0] === '.') {
        const payload = JSON.parse(this.incoming); this.incoming = '';
        if (['lobby_state', 'match_start'].includes(payload.type)) {
          const config = this.owner.config;
          if (payload.format !== config.format || payload.securityMode !== 'trusted') throw new Error('Lobby rules do not match the relay room');
        }
        this.emit('data', payload);
      }
    }
  }
  send(payload) {
    if (!this.open) throw new Error('Relay connection closed');
    const data = JSON.stringify(payload);
    if (data.length > MAX_MESSAGE) throw new Error('Relay message too large');
    const frames = [];
    for (let offset = 0; offset < data.length;) {
      let end = Math.min(data.length, offset + CHUNK);
      if (end < data.length && data.charCodeAt(end - 1) >= 0xd800 && data.charCodeAt(end - 1) <= 0xdbff) end--;
      frames.push({ type: 'data', to: this.peer, connectionId: this.connectionId, data: (end === data.length ? '.' : '+') + data.slice(offset, end) });
      offset = end;
    }
    this.owner.sendBatch(frames);
  }
  close(notify = true) {
    if (this.closed) return;
    this.closed = true; this.open = false; clearTimeout(this.timeout); this.incoming = '';
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
  queue = [];
  queuedSize = 0;
  constructor(peerId, options = {}) {
    super();
    this.options = { ...options, transport: 'websocket' };
    const room = options.room || (isRelayId(peerId) ? peerId.split('-')[1] : id());
    this.id = peerId || `ws-${room}-${id()}`;
    this.room = room;
    this.token = id();
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
    ws.onopen = () => ws.send(JSON.stringify({ type: 'auth', token: this.token,
      format: this.options.format, desiredPlayers: this.options.desiredPlayers }));
    ws.onmessage = ({ data }) => {
      if (this.socket !== ws || this.destroyed) return;
      try {
        if (data === 'pong') { this.lastPong = Date.now(); return; }
        const msg = JSON.parse(data);
        if (msg.type === 'open') {
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
          conn.signal('answer'); conn.opened();
        } else if (conn?.peer === msg.from) conn.receive(msg);
      } catch (error) { this.emit('error', error); ws.close(1008, 'Invalid frame'); }
    };
    ws.onclose = () => {
      if (this.socket !== ws) return;
      this.open = false; this.disconnected = true;
      clearInterval(this.pingTimer); clearTimeout(this.flushTimer); clearInterval(this.advertiseTimer);
      this.queue = []; this.queuedSize = 0;
      for (const conn of [...this.connections.values()]) conn.close(false);
      if (!this.destroyed) this.emit('disconnected');
    };
    ws.onerror = () => { const error = new Error('Could not reach WebSocket lobby service'); error.type = 'network'; this.emit('error', error); };
  }
  send(msg) { this.sendBatch([msg]); }
  sendBatch(messages) {
    if (!this.open) throw new Error('Relay is disconnected');
    const serialized = messages.map(msg => JSON.stringify(msg));
    const size = serialized.reduce((sum, s) => sum + s.length, 0);
    if (size + this.queuedSize > MAX_MESSAGE * 2) throw new Error('Relay send queue full');
    this.queue.push(...serialized); this.queuedSize += size; this.flush();
  }
  flush() {
    clearTimeout(this.flushTimer);
    // Pacing avoids monopolizing the DO and bounds buffers on slow networks.
    let count = 0;
    while (this.open && this.queue.length && this.socket.bufferedAmount < 512 * 1024 && count++ < 8) {
      const frame = this.queue.shift(); this.queuedSize -= frame.length; this.socket.send(frame);
    }
    if (this.open && this.queue.length) this.flushTimer = setTimeout(() => this.flush(), 50);
  }
  connect(peer, options = {}) {
    const conn = new RelayConnection(this, peer, id(), options.metadata);
    this.connections.set(conn.connectionId, conn);
    queueMicrotask(() => {
      try { conn.signal('offer', { metadata: conn.metadata }); }
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
    this.open = false; this.disconnected = true; this.queue = []; this.queuedSize = 0;
    this.socket?.close(1000, 'Leaving lobby'); this.emit('close');
  }
}
