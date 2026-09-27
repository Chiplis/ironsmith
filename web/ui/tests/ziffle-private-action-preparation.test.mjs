import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../src/hooks/usePeerLobby.js', import.meta.url), 'utf8');
const start = source.indexOf('        updateLocalActionProgress({\n          kind: "local_payload",\n          operation: "Building random reveals",');
const end = source.indexOf('        let remoteCryptoMaterial =', start);
assert.ok(start >= 0 && end > start);

test('live preparation agrees random outcome before proving its shuffle and queues its seed once', async () => {
  const random = { id: 'coin', type: 'fair_random' };
  const provisional = { id: 'wrong-branch', type: 'verifiable_shuffle' };
  const final = { id: 'actual-branch', type: 'verifiable_shuffle' };
  const reveal = { requirementId: 'coin', combinedSeedHex: '1234' };
  const queued = [], events = [];
  let prepared = false;
  const context = {
    cryptoRequirements: [random, provisional], requestRemoteCryptoPreview: false,
    updateLocalActionProgress() {}, recordPeerSyncPerf() {},
    summarizeCryptoRequirementsForPerf: requirements => requirements,
    payloadSizeBytes: () => 0, submitPerf: {}, command: { type: 'priority_action' },
    nextSequence: 8, session: { localPlayerIndex: 0 }, preActionStateHash: 'previous',
    preActionPublicCheckpointHash: 'checkpoint', preSubmitState: {}, signedActionIntent: {},
    runSubmissionPhase: async (_name, _details, work) => work(),
    buildLocalRngRevealsForRequirements: async () => [reveal],
    injectCryptoMaterialForRequirements: async (_requirements, audit, options) => {
      if (!options.skipRandomness) queued.push(...(audit.rngReveals || []));
      if (!options.skipRandomness && audit.rngReveals?.length) events.push('agreed-random');
      if (audit.shuffleProofs?.length) prepared = true;
    },
    previewRequirementsForCommand: async () => {
      assert.equal(queued.length, 1, 'the agreed seed is queued exactly once across previews');
      events.push(prepared ? 'preview-epoch' : 'preview-random');
      return [random, final];
    },
    filterCryptoRequirementsForCommand: (_command, _state, requirements) => requirements,
    freshCryptoRequirementsForSequence: (_sequence, requirements) => requirements,
    rememberActionCryptoRequirements() {}, shouldRequestRemoteCryptoPreview: () => true,
    buildLocalShuffleProofsForRequirements: async requirements => {
      assert.deepEqual(requirements, [random, final]);
      events.push('prove-actual-shuffle');
      return [{ requirementId: final.id, inputDeck: {} }];
    },
    alignShuffleProofsWithRequirements: proofs => proofs,
  };
  const run = new Function(...Object.keys(context), `return (async () => {${source.slice(start, end)}
    return { shuffleProofs, actionCryptoOptions }; })();`);
  const result = await run(...Object.values(context));
  assert.deepEqual(events.slice(0, 3), ['agreed-random', 'preview-random', 'prove-actual-shuffle']);
  // Later identity injection skips randomness; it must not enqueue again.
  assert.equal(queued.length, 1);
  assert.equal(result.shuffleProofs[0].requirementId, final.id);
  assert.deepEqual(result.actionCryptoOptions.rngReveals, [reveal]);
});
