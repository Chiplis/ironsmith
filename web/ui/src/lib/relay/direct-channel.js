// A WebRTC data channel negotiated through relay 'rtc' frames. The owning
// RelayConnection decides when traffic moves onto it; this class only
// negotiates, frames messages and reports failure.
const MAX_MESSAGE = 64 * 1024 * 1024;
const CHUNK = 16000;
const HIGH_WATER = 512 * 1024;
export const DIRECT_TIMEOUT = 15_000;
export const DEFAULT_ICE_SERVERS = Object.freeze([{ urls: ['stun:stun.l.google.com:19302', 'stun:stun1.l.google.com:19302'] }]);

export class DirectChannel {
  channel = null;
  open = false;
  closed = false;
  incoming = '';
  queue = [];
  queuedSize = 0;
  signals = Promise.resolve();

  constructor({ initiator, iceServers, timeoutMs = DIRECT_TIMEOUT, sendSignal, onOpen, onMessage, onFailed }) {
    Object.assign(this, { sendSignal, onOpen, onMessage, onFailed });
    this.pc = new RTCPeerConnection({ iceServers: iceServers?.length ? iceServers : DEFAULT_ICE_SERVERS });
    this.pc.onicecandidate = ({ candidate }) => { if (candidate) this.sendSignal({ kind: 'candidate', candidate: candidate.toJSON() }); };
    this.pc.onconnectionstatechange = () => { if (this.pc.connectionState === 'failed') this.fail('Direct connection failed'); };
    this.timer = setTimeout(() => { if (!this.open) this.fail('Direct connection timed out'); }, timeoutMs);
    if (initiator) {
      this.attach(this.pc.createDataChannel('ironsmith', { ordered: true }));
      this.signals = this.signals
        .then(async () => {
          await this.pc.setLocalDescription(await this.pc.createOffer());
          this.sendSignal({ kind: 'description', description: this.pc.localDescription.toJSON() });
        })
        .catch(error => this.fail(error));
    } else {
      this.pc.ondatachannel = ({ channel }) => this.attach(channel);
    }
  }

  attach(channel) {
    this.channel = channel;
    channel.bufferedAmountLowThreshold = 128 * 1024;
    channel.onbufferedamountlow = () => this.flush();
    channel.onopen = () => {
      if (this.closed) return;
      clearTimeout(this.timer);
      this.open = true;
      this.onOpen();
    };
    channel.onclose = () => this.fail('Direct channel closed');
    channel.onerror = (event) => this.fail(`Direct channel failed: ${event?.error?.message || 'unknown error'}`);
    channel.onmessage = ({ data }) => {
      if (this.closed) return;
      // One ordered reliable channel carries contiguous '+'/'.' chunk runs.
      if (typeof data !== 'string' || !['+', '.'].includes(data[0])) return this.fail('Invalid direct frame');
      if (this.incoming.length + data.length > MAX_MESSAGE) return this.fail('Direct message too large');
      this.incoming += data.slice(1);
      if (data[0] !== '.') return;
      const text = this.incoming;
      this.incoming = '';
      let payload;
      try { payload = JSON.parse(text); } catch { return this.fail('Invalid direct message'); }
      this.onMessage(payload, text.length);
    };
  }

  // Signals are applied strictly in arrival order; ICE candidates that beat
  // their description are held by the chain rather than a side buffer.
  handleSignal(signal) {
    this.signals = this.signals.then(async () => {
      if (this.closed) return;
      if (signal.kind === 'candidate') {
        await this.pc.addIceCandidate(signal.candidate);
      } else if (signal.kind === 'description') {
        await this.pc.setRemoteDescription(signal.description);
        if (signal.description?.type === 'offer') {
          await this.pc.setLocalDescription(await this.pc.createAnswer());
          this.sendSignal({ kind: 'description', description: this.pc.localDescription.toJSON() });
        }
      }
    }).catch(error => this.fail(error));
  }

  send(serialized) {
    if (!this.open) throw new Error('Direct channel is closed');
    if (serialized.length > MAX_MESSAGE || this.queuedSize + serialized.length > MAX_MESSAGE * 2) {
      const error = new Error('Direct send queue full'); error.code = 'RELAY_BACKPRESSURE'; throw error;
    }
    for (let offset = 0; offset < serialized.length || offset === 0;) {
      let end = Math.min(serialized.length, offset + CHUNK);
      const last = serialized.charCodeAt(end - 1);
      if (end < serialized.length && last >= 0xd800 && last <= 0xdbff) end -= 1;
      this.queue.push((end === serialized.length ? '.' : '+') + serialized.slice(offset, end));
      offset = end;
      if (end === serialized.length) break;
    }
    this.queuedSize += serialized.length;
    this.flush();
  }

  flush() {
    try {
      while (this.open && this.queue.length && this.channel.bufferedAmount < HIGH_WATER) {
        const chunk = this.queue.shift();
        this.queuedSize -= chunk.length - 1;
        this.channel.send(chunk);
      }
    } catch (error) { this.fail(error); }
  }

  fail(error) {
    if (this.closed) return;
    this.close();
    this.onFailed(String(error?.message || error));
  }

  close() {
    if (this.closed) return;
    this.closed = true;
    this.open = false;
    clearTimeout(this.timer);
    if (this.channel) this.channel.onclose = this.channel.onerror = this.channel.onmessage = null;
    this.pc.onicecandidate = this.pc.onconnectionstatechange = this.pc.ondatachannel = null;
    this.queue = [];
    try { this.pc.close(); } catch { /* already closed */ }
  }
}
