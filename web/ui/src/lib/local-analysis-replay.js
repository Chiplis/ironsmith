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

// Native branch handles still live after these operations.
function liveRuntimeSavepoints(operations) {
  const handles = new Set();
  for (const operation of operations) {
    if (operation.failed) continue;
    if (operation.method === 'createRuntimeSavepoint') handles.add(operation.handle);
    if (/^(restore|release)RuntimeSavepoint$/.test(operation.method)) handles.delete(operation.args[0]);
  }
  return handles;
}

// An instance image retains Rust's branch map, but its old JS owners expired.
// Release those branches and journal their lifetimes so auxiliary replicas do
// not retain orphan handles or eventually exhaust the native branch limit.
export function releaseRestoredRuntimeSavepoints(journal) {
  for (const handle of liveRuntimeSavepoints(journal.capture().operations)) journal.game.releaseRuntimeSavepoint(handle);
}

// A replica this far behind restores an instance image instead of replaying.
export const ANALYSIS_SEED_MIN_OPERATIONS = 128;

// Seed a replica from an exact image of the runtime taken at this journal
// position, so its replay cost no longer grows with the session's length.
// The image already contains every earlier operation, so none are sent.
export function seededLocalReplay(replay, image) {
  return { epoch: replay.epoch, identityOrigin: replay.identityOrigin,
    base: replay.operations.length, operations: [],
    seed: { image, handles: [...liveRuntimeSavepoints(replay.operations)] } };
}

export const localReplayEnd = replay => (replay?.base ?? 0) + (replay?.operations?.length ?? 0);

// How far a replica has caught up. Positions from another epoch mean nothing.
export const localReplayMark = replay => ({ epoch: replay?.epoch, position: localReplayEnd(replay) });
export const localReplayCaughtUp = (replay, mark) => mark?.epoch === replay.epoch ? mark.position : 0;

// Seed memory moves to the analysis worker instead of being copied again.
export const localReplayTransfer = replay => replay?.seed ? [replay.seed.image.memory.buffer] : [];

export function createLocalAnalysisReplica(createGame, { restoreSeed = null } = {}) {
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
      const base = journal.base ?? 0, end = base + journal.operations.length;
      const stale = !game || rebuild || epoch !== journal.epoch || identityOriginKey !== incomingOriginKey || position > end;
      if (journal.seed && restoreSeed && (stale || position < base)) {
        rebuild = true;
        const previous = game;
        game = null;
        // The image replaces this runtime, including any canonical branch.
        // Constructing a placeholder game would cost more than the restore.
        game = await restoreSeed(journal.seed.image, previous ?? null);
        epoch = journal.epoch;
        identityOriginKey = incomingOriginKey;
        position = base;
        canonical = null;
        rebuild = false;
        handles.clear();
        // Branches created before the image keep their numbers inside it.
        for (const handle of journal.seed.handles) handles.set(handle, handle);
      } else if (stale) {
        if (base > 0) throw new Error('Local analysis replay starts after an unseeded replica');
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
      if (position < base) throw new Error('Local analysis replay starts after this replica');
      while (position < end) {
        const operation = journal.operations[position - base];
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
      try {
        canonical = game.createRuntimeSavepoint();
        // The native branch now owns the reset state, so release the large
        // seed image. If every branch slot belongs to the session, keep the
        // image instead: its truncated journal cannot rebuild without it.
        if (journal.seed) lastJournal = { ...journal, seed: undefined };
      }
      catch (error) {
        rebuild = true;
        if (!errorMessage(error).includes('too many live runtime savepoints')) throw error;
        // All branch slots belong to the actual session. Rebuild from the
        // retained seed and log on the next request without discarding a branch.
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
