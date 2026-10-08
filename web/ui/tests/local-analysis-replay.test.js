import test from 'node:test';
import assert from 'node:assert/strict';
import { createLocalAnalysisJournal, createLocalAnalysisReplica, releaseRestoredRuntimeSavepoints, seededLocalReplay, localReplayEnd } from '../src/lib/local-analysis-replay.js';

class Game {
  state = { objects: [], choices: {}, history: [], manaProvenance: [], temporaryPermissions: [] };
  handles = new Map();
  nextHandle = 0;
  getRuntimeIdentityOrigin() { return { object: 1 }; }
  initializeRuntimeIdentityOrigin(origin) { assert.deepEqual(origin, { object: 1 }); }
  edit(field, value) { this.state[field] = structuredClone(value); }
  createRuntimeSavepoint() { const h = ++this.nextHandle; this.handles.set(h, structuredClone(this.state)); return h; }
  exchangeRuntimeSavepoint(h) { const s = this.handles.get(h); if (!s) throw Error('expired'); this.handles.set(h, this.state); this.state = s; }
  copyRuntimeSavepoint(h) { this.state = structuredClone(this.handles.get(h)); }
  restoreRuntimeSavepoint(h) { this.copyRuntimeSavepoint(h); this.releaseRuntimeSavepoint(h); }
  releaseRuntimeSavepoint(h) { return this.handles.delete(h); }
  failureWithSideEffects() { this.state.history.push('failed action'); throw Error('rejected'); }
  readWithSideEffects() { this.state.choices.readCount = (this.state.choices.readCount || 0) + 1; return 0; }
  free() {}
}

test('local analysis retains all engine state omitted by the public checkpoint', async () => {
  const original = new Game(), journal = createLocalAnalysisJournal(original, 1);
  for (const [field, value] of Object.entries({
    objects: [{ id: 1, copiedAbilities: ['mana'], grantedAbilities: ['flash'] }],
    choices: { creature: 'Dwarf', color: 'red', namedCard: 'Opt' },
    history: ['cast spell', 'exhaust used', 'land played'],
    manaProvenance: [{ snow: true, creatureOnly: 'Dwarf', onSpend: ['uncounterable'] }],
    temporaryPermissions: ['cast from exile', 'cost reduction'],
  })) journal.game.edit(field, value);
  const frozen = journal.capture();
  journal.game.edit('choices', { creature: 'Elf' });
  const replica = createLocalAnalysisReplica(() => new Game());
  const restored = await replica.hydrate(frozen);
  assert.equal(restored.state.choices.creature, 'Dwarf');
  assert.deepEqual(restored.state.history, original.state.history);
  assert.deepEqual(restored.state.manaProvenance, original.state.manaProvenance);
  assert.deepEqual(restored.state.temporaryPermissions, original.state.temporaryPermissions);
  assert.deepEqual(restored.state.objects, original.state.objects);
  assert.equal((await replica.hydrate(journal.capture())).state.choices.creature, 'Elf');
});

test('incremental replay discards speculative writes and translates native branch handles', async () => {
  const original = new Game(), journal = createLocalAnalysisJournal(original, 2);
  journal.game.edit('history', ['original']);
  const replica = createLocalAnalysisReplica(() => new Game());
  const restored = await replica.hydrate(journal.capture());
  restored.edit('history', ['speculation']);
  // The analysis savepoint has consumed a handle unknown to the original.
  const branch = journal.game.createRuntimeSavepoint();
  journal.game.edit('history', ['verified']);
  journal.game.exchangeRuntimeSavepoint(branch);
  journal.game.edit('choices', { verification: true });
  journal.game.exchangeRuntimeSavepoint(branch);
  journal.game.copyRuntimeSavepoint(branch);
  journal.game.releaseRuntimeSavepoint(branch);
  const next = await replica.hydrate(journal.capture());
  assert.deepEqual(next.state, original.state);
  next.edit('choices', { preview: true });
  await replica.resetWorkingState();
  assert.deepEqual(next.state, original.state);
});

test('reads and rejected actions retain side effects; divergent replay fails closed', async () => {
  const original = new Game(), journal = createLocalAnalysisJournal(original, 3);
  journal.game.readWithSideEffects();
  assert.throws(() => journal.game.failureWithSideEffects(), /rejected/);
  const replica = createLocalAnalysisReplica(() => new Game());
  assert.deepEqual((await replica.hydrate(journal.capture())).state, original.state);
  const divergent = createLocalAnalysisReplica(() => {
    const game = new Game(); game.failureWithSideEffects = () => {};
    return game;
  });
  await assert.rejects(divergent.hydrate(journal.capture()), /replay diverged/);
});

test('a new session and a shorter prefix rebuild rather than alias old state', async () => {
  const replica = createLocalAnalysisReplica(() => new Game());
  const a = createLocalAnalysisJournal(new Game(), 4);
  const short = a.capture();
  a.game.edit('choices', { old: true });
  await replica.hydrate(a.capture());
  assert.deepEqual((await replica.hydrate(short)).state.choices, {});
  const b = createLocalAnalysisJournal(new Game(), 5);
  b.game.edit('choices', { current: true });
  assert.deepEqual((await replica.hydrate(b.capture())).state.choices, { current: true });
});

test('exhausted native branch capacity rebuilds complete snapshots without dropping session branches', async () => {
  class LimitedGame extends Game {
    createRuntimeSavepoint() {
      if (this.handles.size === 2) throw Error('too many live runtime savepoints');
      return super.createRuntimeSavepoint();
    }
  }
  const journal = createLocalAnalysisJournal(new LimitedGame(), 6);
  journal.game.createRuntimeSavepoint();
  journal.game.createRuntimeSavepoint();
  journal.game.edit('history', ['used ability']);
  const replica = createLocalAnalysisReplica(() => new LimitedGame());
  const first = await replica.hydrate(journal.capture());
  first.edit('history', ['preview']);
  const restored = await replica.resetWorkingState();
  assert.deepEqual(restored.state.history, ['used ability']);
  assert.equal(restored.handles.size, 2);
});

test('expired session handles cannot accidentally address an analysis savepoint', async () => {
  const journal = createLocalAnalysisJournal(new Game(), 7);
  const branch = journal.game.createRuntimeSavepoint();
  journal.game.releaseRuntimeSavepoint(branch);
  journal.game.releaseRuntimeSavepoint(branch);
  assert.throws(() => journal.game.exchangeRuntimeSavepoint(branch), /expired/);
  const replica = createLocalAnalysisReplica(() => new Game());
  assert.deepEqual((await replica.hydrate(journal.capture())).state, journal.game.state);
});

test('different failures cannot silently drop side effects and a failed replica is rebuilt', async () => {
  const journal = createLocalAnalysisJournal(new Game(), 8);
  assert.throws(() => journal.game.failureWithSideEffects(), /rejected/);
  let attempt = 0;
  const replica = createLocalAnalysisReplica(() => {
    const game = new Game();
    if (++attempt === 1) game.failureWithSideEffects = () => { throw Error('different failure'); };
    return game;
  });
  await assert.rejects(replica.hydrate(journal.capture()), /replay diverged.*different failure/);
  assert.deepEqual((await replica.hydrate(journal.capture())).state, journal.game.state);
});


test('allocator bootstrap is applied once and a changed origin rebuilds an existing replica', async () => {
  class BootstrapGame extends Game {
    origin = { object: 1 };
    initialized = false;
    getRuntimeIdentityOrigin() { return this.origin; }
    initializeRuntimeIdentityOrigin(origin) {
      if (this.initialized) throw Error('origin already initialized');
      this.initialized = true;
      this.origin = structuredClone(origin);
    }
  }
  const original = new BootstrapGame();
  const journal = createLocalAnalysisJournal(original, 4);
  const replica = createLocalAnalysisReplica(() => new BootstrapGame());
  assert.deepEqual((await replica.hydrate(journal.capture())).origin, { object: 1 });
  journal.game.initializeRuntimeIdentityOrigin({ object: 41 });
  journal.game.edit('history', ['action after bootstrap']);
  assert.throws(() => journal.game.initializeRuntimeIdentityOrigin({ object: 99 }), /already initialized/);
  const captured = journal.capture();
  assert.ok(!captured.operations.some(operation => operation.method === 'initializeRuntimeIdentityOrigin'));
  const restored = await replica.hydrate(captured);
  assert.deepEqual(restored.origin, { object: 41 });
  assert.deepEqual(restored.state, original.state);
});

test('cold instance cleanup releases only live journaled savepoints and records their retirement', () => {
  const released=[];
  const operations=[
    {method:'createRuntimeSavepoint',handle:1,args:[]},
    {method:'createRuntimeSavepoint',handle:2,args:[]},
    {method:'restoreRuntimeSavepoint',args:[1]},
    {method:'createRuntimeSavepoint',handle:3,args:[],failed:true},
    {method:'createRuntimeSavepoint',handle:4,args:[]},
    {method:'releaseRuntimeSavepoint',args:[4]},
    {method:'copyRuntimeSavepoint',args:[2]},
    {method:'exchangeRuntimeSavepoint',args:[2]},
  ];
  const journal=createLocalAnalysisJournal({releaseRuntimeSavepoint:handle=>released.push(handle)},'restored',{identityOrigin:{object:1},operations});
  releaseRestoredRuntimeSavepoints(journal);
  assert.deepEqual(released,[2]);
  assert.equal(journal.capture().operations.at(-1).method,'releaseRuntimeSavepoint');
  releaseRestoredRuntimeSavepoints(journal);
  assert.deepEqual(released,[2],'retired handles are not released twice');
});

test('a seeded replica restores the image and replays only later operations', async () => {
  const original = new Game(), journal = createLocalAnalysisJournal(original, 9);
  journal.game.edit('history', ['before seed']);
  const branch = journal.game.createRuntimeSavepoint();
  // The image is the exact runtime, including the live native branch.
  const image = { state: structuredClone(original.state), handles: new Map(original.handles), nextHandle: original.nextHandle };
  const seeded = seededLocalReplay(journal.capture(), image);
  assert.equal(seeded.operations.length, 0);
  assert.equal(localReplayEnd(seeded), journal.capture().operations.length);
  assert.deepEqual(seeded.seed.handles, [branch]);
  journal.game.edit('choices', { after: true });
  journal.game.exchangeRuntimeSavepoint(branch);
  journal.game.exchangeRuntimeSavepoint(branch);
  const full = journal.capture();
  const later = { ...seeded, operations: full.operations.slice(seeded.base) };
  let restores = 0, replays = 0;
  const replica = createLocalAnalysisReplica(() => { replays++; return new Game(); }, {
    restoreSeed: async (seed, old) => {
      restores++;
      assert.equal(old, null, 'a fresh replica has no runtime to detach');
      return Object.assign(new Game(), { state: structuredClone(seed.state), handles: new Map(seed.handles), nextHandle: seed.nextHandle });
    },
  });
  const restored = await replica.hydrate(later);
  assert.equal(restores, 1);
  assert.deepEqual(restored.state, original.state);
  // A caught-up replica ignores a seed it does not need.
  journal.game.edit('history', ['after']);
  const next = await replica.hydrate({ ...later, operations: journal.capture().operations.slice(seeded.base) });
  assert.equal(restores, 1);
  assert.deepEqual(next.state, original.state);
  assert.equal(replays, 0, 'no engine is constructed only to be replaced');
});

test('a replay that starts after an unseeded replica fails closed', async () => {
  const replica = createLocalAnalysisReplica(() => new Game());
  await assert.rejects(replica.hydrate({ epoch: 1, identityOrigin: { object: 1 }, base: 3, operations: [] }), /unseeded replica/);
});

// UNRUN: retain a trusted exact image when the session owns every native slot.
test('a seeded replica with full native branch capacity keeps its exact reset image', async () => {
  class LimitedGame extends Game {
    createRuntimeSavepoint() {
      if (this.handles.size === 2) throw Error('too many live runtime savepoints');
      return super.createRuntimeSavepoint();
    }
  }
  const original = new LimitedGame(), journal = createLocalAnalysisJournal(original, 10);
  journal.game.createRuntimeSavepoint();
  journal.game.createRuntimeSavepoint();
  journal.game.edit('history', ['before seed']);
  const image = { state: structuredClone(original.state), handles: structuredClone(original.handles), nextHandle: original.nextHandle };
  const seeded = seededLocalReplay(journal.capture(), image);
  journal.game.edit('choices', { after: true });
  const replay = { ...seeded, operations: journal.capture().operations.slice(seeded.base) };
  let restores = 0;
  const replica = createLocalAnalysisReplica(() => { throw Error('seed must rebuild the truncated journal'); }, {
    restoreSeed: async seed => {
      restores++;
      return Object.assign(new LimitedGame(), structuredClone(seed));
    },
  });
  let restored = await replica.hydrate(replay);
  for (let preview = 0; preview < 2; preview++) {
    restored.edit('history', ['speculation']);
    restored = await replica.resetWorkingState();
    assert.deepEqual(restored.state, original.state);
    assert.deepEqual(restored.handles, original.handles);
  }
  assert.equal(restores, 3);
});
