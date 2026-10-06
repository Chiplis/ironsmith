// Replay exact engine calls in a private local worker; native branches retain
// live programs and continuations. Never send this journal to another seat.
const errorMessage = error => String(error?.message ?? error);

export function createLocalAnalysisJournal(runtime, epoch, restored = null) {
  let identityOrigin = structuredClone(restored?.identityOrigin ?? runtime.getRuntimeIdentityOrigin());
  const operations = restored ? structuredClone(restored.operations) : [];
  const wrappers = new Map();
  const game = new Proxy(runtime, {
    get(target, method) {
      const value = Reflect.get(target, method, target);
      if (typeof value !== 'function') return value;
      if (!wrappers.has(method)) wrappers.set(method, (...args) => {
        // Allocator initialization is fresh-runtime bootstrap, already applied
        // by the replica. Record its resulting origin, not a second execution.
        if (method === 'initializeRuntimeIdentityOrigin') {
          const updateOrigin = result => {
            identityOrigin = structuredClone(runtime.getRuntimeIdentityOrigin());
            return result;
          };
          const result = value.apply(target, args);
          return result?.then ? result.then(updateOrigin) : updateOrigin(result);
        }
        // Record even reads and failures: either can have engine side effects.
        // Calling on the original receiver avoids recording facade calls twice.
        const operation = { method, args: structuredClone(args), failed: false };
        const finish = result => {
          if (method === 'createRuntimeSavepoint') operation.handle = result;
          operations.push(operation);
          return result;
        };
        const fail = error => {
          operation.failed = true;
          operation.error = errorMessage(error);
          operations.push(operation);
          throw error;
        };
        try {
          const result = value.apply(target, args);
          return result?.then ? result.then(finish, fail) : finish(result);
        } catch (error) { return fail(error); }
      });
      return wrappers.get(method);
    },
  });
  return { game, capture: () => ({ epoch, identityOrigin, operations: operations.slice() }) };
}

// An instance image retains Rust's branch map, but its old JS owners expired.
// Release those branches and journal their lifetimes so auxiliary replicas do
// not retain orphan handles or eventually exhaust the native branch limit.
export function releaseRestoredRuntimeSavepoints(journal) {
  const handles = new Set();
  for (const operation of journal.capture().operations) {
    if (operation.failed) continue;
    if (operation.method === 'createRuntimeSavepoint') handles.add(operation.handle);
    if (/^(restore|release)RuntimeSavepoint$/.test(operation.method)) handles.delete(operation.args[0]);
  }
  for (const handle of handles) journal.game.releaseRuntimeSavepoint(handle);
}

export function createLocalAnalysisReplica(createGame) {
  let game, epoch, identityOriginKey, position = 0, canonical = null, rebuild = false, lastJournal;
  const handles = new Map();
  const restore = () => {
    if (canonical == null) return;
    // Exchange, rather than restore+snapshot: no presentation or discovery
    // operation may alter the canonical runtime before the next replay.
    game.exchangeRuntimeSavepoint(canonical);
    game.releaseRuntimeSavepoint(canonical);
    canonical = null;
  };
  return {
    async hydrate(journal, yieldControl = async () => {}) {
      if (!journal || !Array.isArray(journal.operations)) throw new Error('Missing local analysis journal');
      const incomingOriginKey = JSON.stringify(journal.identityOrigin);
      if (!game || rebuild || epoch !== journal.epoch || identityOriginKey !== incomingOriginKey || position > journal.operations.length) {
        rebuild = true;
        game?.free();
        game = createGame();
        game.initializeRuntimeIdentityOrigin(journal.identityOrigin);
        epoch = journal.epoch;
        identityOriginKey = incomingOriginKey;
        position = 0;
        canonical = null;
        rebuild = false;
        handles.clear();
      } else restore();
      let sliceStarted = performance.now();
      while (position < journal.operations.length) {
        const operation = journal.operations[position];
        const args = structuredClone(operation.args);
        if (/^(exchange|copy|restore|release)RuntimeSavepoint$/.test(operation.method)) {
          if (handles.has(args[0])) args[0] = handles.get(args[0]);
          else args[0] = 0; // Native expired-handle behavior, never our analysis branch.
        }
        let result, error, failed = false;
        try { result = await game[operation.method](...args); }
        catch (caught) { error = caught; failed = true; }
        if (failed !== operation.failed || (failed && errorMessage(error) !== operation.error)) {
          rebuild = true;
          // Do not publish speculative actions from a divergent replica.
          throw new Error(`Local analysis replay diverged at ${position}: ${operation.method}: ${failed ? (error?.message || String(error)) : "expected rejection"}`, { cause: error });
        }
        if (!failed && operation.method === 'createRuntimeSavepoint') handles.set(operation.handle, result);
        if (!failed && /^(restore|release)RuntimeSavepoint$/.test(operation.method)) handles.delete(operation.args[0]);
        position++;
        if (performance.now() - sliceStarted >= 8) {
          await yieldControl();
          sliceStarted = performance.now();
        }
      }
      // Analysis is allowed to mutate this working branch. The next hydrate
      // resumes the exact canonical state, including continuations and RNG.
      lastJournal = journal;
      try { canonical = game.createRuntimeSavepoint(); }
      catch (error) {
        rebuild = true;
        if (!errorMessage(error).includes('too many live runtime savepoints')) throw error;
        // All branch slots belong to the actual session. Rebuild from its log
        // on the next request rather than discard a branch or partial state.
      }
      return game;
    },
    async resetWorkingState() {
      if (rebuild) return this.hydrate(lastJournal);
      if (canonical == null) throw new Error('No local analysis snapshot');
      game.copyRuntimeSavepoint(canonical);
      return game;
    },
  };
}
