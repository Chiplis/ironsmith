import test from 'node:test';
import assert from 'node:assert/strict';
import { createOperationRevealBatch } from '../src/lib/ziffle-operation-reveal-batch.js';
test('batch is lazy, shared by concurrent consumers and scoped to one operation', async () => {
  let calls = 0;
  const reveal = createOperationRevealBatch([2, 9], 60, async () => {
    calls++;
    return [{ cardPosition: 9, originalSlot: 3 }, { cardPosition: 2, originalSlot: 51 }];
  });
  assert.equal(calls, 0);
  assert.deepEqual((await Promise.all([reveal(2), reveal(9), reveal(2)])).map(x => x.originalSlot), [51, 3, 51]);
  assert.equal(calls, 1);
  await assert.rejects(reveal(10), /outside/);
  const next = createOperationRevealBatch([2], 60, async () => [{ cardPosition: 2, originalSlot: 8 }]);
  assert.equal((await next(2)).originalSlot, 8);
});
test('missing, duplicate, foreign and invalid verifier outputs fail closed', async () => {
  for (const values of [[], null,
    [{ cardPosition: 2, originalSlot: 4 }, { cardPosition: 2, originalSlot: 5 }],
    [{ cardPosition: 3, originalSlot: 4 }],
    [{ cardPosition: 2, originalSlot: -1 }],
    [{ cardPosition: 2, originalSlot: null }],
    [{ cardPosition: null, originalSlot: 4 }],
    [{ cardPosition: 2, originalSlot: 60 }]]) {
    await assert.rejects(createOperationRevealBatch([2], 60, async () => values)(2));
  }
  let calls = 0;
  const failed = createOperationRevealBatch([2], 60, async () => { calls++; throw new Error('proof rejected'); });
  await assert.rejects(failed(2), /proof rejected/);
  await assert.rejects(failed(2), /proof rejected/);
  assert.equal(calls, 1);
});

test('production hand batch keeps pending-action and visible-state authorization separate', async () => {
  const { readFileSync } = await import('node:fs');
  const source = readFileSync(new URL('../src/hooks/peer-lobby/validation.js', import.meta.url), 'utf8');
  const start = source.indexOf('          const positions = [...new Set(entries.map(entry => Number(entry.position)))];');
  const end = source.indexOf('for (const entry of entries)', start);
  assert.ok(start >= 0 && end > start);
  const requests = [], checks = [], workerCalls = [];
  const context = {
    createOperationRevealBatch,
    entries: [{ position: 2 }, { position: 9 }],
    ceremony: { deckCount: 60, context: 'current', keys: [], steps: [] },
    options: { command: { type: 'priority_action' } }, localIndex: 0,
    ziffleRevealTokenOptionsForLocalHandReveal: async ({ positions }) => {
      assert.equal(positions.length, 1);
      checks.push(positions[0]);
      return { route: positions[0] === 9 ? 'pending-action' : 'visible-state' };
    },
    collectZiffleRevealTokensBatch: async (_ceremony, positions, options) => {
      requests.push({ positions, options });
      return [{ cardPosition: positions[0], proof: options.route }];
    },
    ziffleKeyContextForCeremony: () => 'key', cloneMultiplayerPayload: structuredClone,
    ziffleInputDeckFields: () => ({}),
    currentGame: { ziffleRevealCards: async request => {
      workerCalls.push(request);
      return request.cardPositions.map(cardPosition => ({ cardPosition, originalSlot: cardPosition + 1 }));
    } },
  };
  const reveal = new Function(...Object.keys(context), `${source.slice(start, end)}\nreturn revealPosition;`)(...Object.values(context));
  assert.equal(checks.length, 0);
  assert.equal((await reveal(9)).originalSlot, 10);
  assert.equal((await reveal(2)).originalSlot, 3);
  assert.deepEqual(requests.map(x => x.options.route), ['visible-state', 'pending-action']);
  assert.equal(workerCalls.length, 1);
  assert.deepEqual(workerCalls[0].tokens.map(x => x.proof), ['visible-state', 'pending-action']);
});
