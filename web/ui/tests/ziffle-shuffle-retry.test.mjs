import test from 'node:test';
import assert from 'node:assert/strict';
import { authorizationHarness } from './ziffle-reveal-authorization-harness.mjs';

function fixture({ repeated = false } = {}) {
  const genesis = { owner: 0, deckCount: 3, context: 'match', deckHash: 'root',
    steps: [{ shuffler: 0, deckHex: 'root', proofHex: 'valid', signature: 'signed' }] };
  const requirement = { id: 'search', type: 'verifiable_shuffle', owner: 0, zone: 'library',
    count: 2, randomCountBefore: 10, inputCommitments: ['ziffle:root:0', 'ziffle:root:2'] };
  const requirements = [requirement, ...(repeated ? [{ ...requirement, id: 'search-again',
    randomCountBefore: 11, inputCommitments: ['ziffle:shuffle-1:0', 'ziffle:shuffle-1:1'] }] : [])];
  const locks = { current: new Map() }, verified = { current: new Set() };
  let ceremonies = 0;
  const h = authorizationHarness({ requirements, materialRequirements: requirements,
    match: { protocolVersion: 15, players: [{ index: 0 }], ziffleCeremonies: [genesis] },
    overrides: {
      reindexPlayers: players => players, zifflePublicKeysForPlayers: () => ['signed-roster'],
      ziffleActionRevealLocksRef: locks, verifiedShuffleProofsRef: verified,
      runBatchedZiffleShuffleCeremonies: async requests => {
        ceremonies++;
        return requests.map(request => ({ ...request, deckHash: `shuffle-${ceremonies}`,
          steps: [{ shuffler: 0, deckHex: `shuffle-${ceremonies}`, proofHex: 'valid', signature: 'signed' }],
          verification: { deckHash: `shuffle-${ceremonies}`, deckCount: request.deckCount,
            rootDeckHash: 'root', rootContext: 'match', universeCount: 3 },
        }));
      },
    },
  });
  return { h, requirement, requirements, locks, verified, ceremonies: () => ceremonies };
}

test('a failed submission reuses the exact authorized shuffle after verification-cache rollback', async () => {
  const f = fixture();
  const first = await f.h.build([f.requirement], 8);
  await f.h.verify([f.requirement], first, { seq: 8 });
  const expected = structuredClone(first);
  // Local runtime/crypto rollback drops transient verification, not authority.
  f.verified.current.clear();
  first[0].steps[0].deckHex = 'caller-mutated-proof';
  const retry = await f.h.build([f.requirement], 8);
  assert.deepEqual(retry, expected);
  assert.equal(f.ceremonies(), 1, 'retry must not sample a new encrypted library');
  await f.h.verify([f.requirement], retry, { seq: 8 });
  assert.equal(f.locks.current.size, 1);
});

test('retry cannot reuse the authorized shuffle for a different library selection', async () => {
  const f = fixture();
  const first = await f.h.build([f.requirement], 8);
  await f.h.verify([f.requirement], first, { seq: 8 });
  f.verified.current.clear();
  await assert.rejects(f.h.build([{ ...f.requirement,
    inputCommitments: ['ziffle:root:0', 'ziffle:root:1'] }], 8), /inputs do not match/);
  assert.equal(f.ceremonies(), 1);
  assert.deepEqual(await f.h.build([f.requirement], 8), first, 'original action remains retryable');
});

// Proof generation can precede remote openings and final action verification.
test('failure before final action verification also retains completed shuffle material', async () => {
  const f = fixture();
  const first = await f.h.build([f.requirement], 8);
  f.verified.current.clear();
  assert.deepEqual(await f.h.build([f.requirement], 8), first);
  assert.equal(f.ceremonies(), 1);
});

test('retry of repeated shuffles preserves the preceding authenticated epoch', async () => {
  const f = fixture({ repeated: true });
  const first = await f.h.build(f.requirements, 8, { command: f.h.message.actionAuthorization.command });
  await f.h.verify(f.requirements, first, { seq: 8 });
  f.verified.current.clear();
  assert.deepEqual(await f.h.build(f.requirements, 8, { command: f.h.message.actionAuthorization.command }), first);
  assert.equal(f.ceremonies(), 2, 'each original shuffle is sampled exactly once');
});
