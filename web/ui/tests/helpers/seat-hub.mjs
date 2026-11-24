// Main-thread side: spawns seat workers and routes their in-memory network.
import { Worker } from 'node:worker_threads';

export class SeatHub {
  seats = new Map();      // name -> handle
  peerOwner = new Map();  // peer id -> handle
  connections = new Map(); // connId -> { a: handle, b: handle }
  log = [];

  async spawn(name, { deckText = '' } = {}) {
    const worker = new Worker(new URL('./seat-worker.mjs', import.meta.url), { workerData: { name } });
    const handle = { name, worker, pending: new Map(), nextId: 1, errors: [] };
    this.seats.set(name, handle);
    await new Promise((resolve, reject) => {
      worker.once('error', reject);
      worker.on('message', message => {
        if (message.t === 'ready') resolve();
        this.onWorker(handle, message);
      });
      worker.on('error', error => { handle.errors.push(String(error)); });
    });
    await this.rpc(handle, { op: 'create', deckText });
    const seat = {
      name,
      handle,
      call: (path, method, ...args) => this.rpc(handle, { op: 'call', path, method, args }),
      lobby: (method, ...args) => this.rpc(handle, { op: 'call', path: 'lobby', method, args }),
      snapshot: () => this.rpc(handle, { op: 'snapshot' }),
      async until(label, predicate, timeoutMs = 60_000) {
        const started = Date.now();
        let last = null;
        for (;;) {
          last = await seat.snapshot();
          const value = predicate(last);
          if (value) return value;
          if (Date.now() - started > timeoutMs) {
            throw new Error(`${name}: timed out waiting for ${label}: ${JSON.stringify({ ...last, state: { decision: last?.state?.decision?.kind, turn: last?.state?.turn } })}`);
          }
          await new Promise(resolve => setTimeout(resolve, 25));
        }
      },
    };
    return seat;
  }

  rpc(handle, message) {
    return new Promise((resolve, reject) => {
      const id = handle.nextId++;
      handle.pending.set(id, { resolve, reject });
      handle.worker.postMessage({ t: 'rpc', id, ...message });
    });
  }

  onWorker(handle, message) {
    if (message.t === 'rpc-result') {
      const pending = handle.pending.get(message.id);
      handle.pending.delete(message.id);
      if (message.ok) pending.resolve(message.result); else pending.reject(new Error(`${handle.name}: ${message.error}`));
    } else if (message.t === 'peer-open') {
      this.peerOwner.set(message.id, handle);
    } else if (message.t === 'peer-destroy') {
      this.peerOwner.delete(message.id);
    } else if (message.t === 'connect') {
      const target = this.peerOwner.get(message.to);
      this.connections.set(message.connId, { a: handle, b: target });
      if (!target) { handle.worker.postMessage({ t: 'refuse', connId: message.connId }); return; }
      target.worker.postMessage({ t: 'incoming', connId: message.connId, from: message.from, to: message.to, metadata: message.metadata });
    } else if (message.t === 'accepted' || message.t === 'refuse') {
      const link = this.connections.get(message.connId);
      link?.a.worker.postMessage(message);
    } else if (message.t === 'send') {
      const link = this.connections.get(message.connId);
      if (!link) return;
      const other = link.a === handle ? link.b : link.a;
      this.log.push({ from: handle.name, type: message.payload?.type });
      other?.worker.postMessage({ t: 'data', connId: message.connId, payload: message.payload });
    } else if (message.t === 'close') {
      const link = this.connections.get(message.connId);
      if (!link) return;
      const other = link.a === handle ? link.b : link.a;
      other?.worker.postMessage({ t: 'closed', connId: message.connId });
      this.connections.delete(message.connId);
    }
  }

  async dispose() {
    for (const handle of this.seats.values()) {
      try { await this.rpc(handle, { op: 'dispose' }); } catch { /* ignore */ }
      await handle.worker.terminate();
    }
  }
}
