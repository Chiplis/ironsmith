import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

// Full production signature/proof verification and engine replay of unchanged
// saved exports, using a fresh real browser worker for each transcript.
const args = process.argv.slice(2);
const outputAt = args.indexOf('--output');
const output = outputAt >= 0 ? args.splice(outputAt, 2)[1] : null;
const rootAt = args.indexOf('--ui-root');
const root = path.resolve(rootAt >= 0 ? args.splice(rootAt, 2)[1]
  : path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..'));
assert.ok(args.length > 0, 'Usage: node scripts/verify-saved-audit-browser.mjs [--output results.json] transcript.json ...');
const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'),
  cacheDir: path.join(os.tmpdir(), `ironsmith-saved-audit-${process.pid}`), logLevel: 'error',
  server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
const results = [];
const runtimeAssets = Object.fromEntries(await Promise.all(['engine_bg.wasm', 'verifier_bg.wasm'].map(async name => {
  const data = await readFile(path.resolve(root, '../wasm_demo/pkg', name));
  return [name, { bytes: data.length, sha256: createHash('sha256').update(data).digest('hex') }];
})));
let browser;
try {
  await server.listen();
  browser = await chromium.launch();
  for (const file of args) {
    const data = await readFile(file);
    const transcript = JSON.parse(data);
    const page = await browser.newPage();
    await page.route('**/saved-audit-regression', route => route.fulfill({
      contentType: 'text/html', body: '<title>Saved audit compatibility regression</title>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/saved-audit-regression`);
    const result = await page.evaluate(async ({ transcript, verifierUrl }) => {
      const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
      const { verifyLiveAuditTranscript } = await import('/src/lib/multiplayer-audit.js');
      const { replayAuditTranscriptWithGame } = await import('/src/lib/audit-replay.js');
      const ziffleInputDeckFields = value => value?.inputDeck ? { inputDeck: value.inputDeck } : {};
      const verifier = await import(verifierUrl);
      await verifier.default();
      const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
      const decoder = createSnapshotDecoder(), pending = new Map();
      let id = 0;
      const calls = [];
      const ready = new Promise((resolve, reject) => {
        worker.onerror = event => reject(new Error(event.message));
        worker.onmessage = ({ data }) => {
          if (data.type === 'error') return reject(new Error(data.error.message));
          if (data.type === 'ready') return resolve();
          if (data.type !== 'result') return;
          const request = pending.get(data.id); if (!request) return;
          pending.delete(data.id);
          if (!data.ok) request.reject(new Error(`${request.method}: ${data.error.message}`));
          else request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
        };
      });
      const call = (method, ...args) => new Promise((resolve, reject) => {
        calls.push(method); if (calls.length > 12) calls.shift();
        pending.set(++id, { resolve, reject, method });
        worker.postMessage({ type: 'call', id, method, args });
      });
      const methods = ['uiState', 'startMatch', 'setPerspective', 'dispatch', 'previewCryptoRequirements',
        'revealHiddenPosition', 'revealHiddenSlot', 'revealHiddenObject', 'exportSyncCheckpoint',
        'exportPublicAuditCheckpoint', 'importSyncCheckpoint', 'injectTranscriptRandomSeeds',
        'applyVerifiedHiddenLibraryShuffle', 'queueVerifiedHiddenLibraryEpoch', 'queueVerifiedHiddenLibraryOpening',
        'cancelDecision', 'forfeitPlayer', 'endOfMatchDisclosureRequirements', 'verifyEndOfMatchDisclosure',
        'createRuntimeSavepoint', 'restoreRuntimeSavepoint'];
      const game = Object.fromEntries(methods.map(method => [method, (...args) => call(method, ...args)]));
      worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
      const start = performance.now();
      let engineReplay = null;
      try {
        await ready;
        const report = await verifyLiveAuditTranscript(transcript, globalThis.crypto, {
          requireEngineReplay: true,
          replayTranscript: async ({ transcript: value }) => {
            engineReplay = await replayAuditTranscriptWithGame({ game, transcript: value });
            return engineReplay;
          },
          verifyShuffleProof: async proof => {
            const verified = verifier.ziffleVerifyShuffle({ deckCount: Number(proof.deckCount),
              context: proof.context, keyContext: proof.keyContext || proof.context,
              keys: proof.keys || [], steps: proof.steps || [], ...ziffleInputDeckFields(proof) });
            if (verified.deckHash !== proof.deckHash) throw new Error('Saved transcript shuffle hash differs');
            return verified;
          },
          verifyZiffleOpening: async ({ proof, ceremony }) => verifier.ziffleRevealCard({
            deckCount: ceremony.deckCount, context: ceremony.context, keyContext: ceremony.keyContext || ceremony.context,
            keys: ceremony.keys || [], steps: ceremony.steps || [], ...ziffleInputDeckFields(ceremony),
            cardPosition: Number(proof.position), tokens: proof.tokens || [] }),
        });
        return { ok: true, elapsedMs: performance.now() - start, report };
      } catch (error) { return { ok: false, elapsedMs: performance.now() - start, error: error.message, calls, engineReplay }; }
      finally { worker.terminate(); }
    }, { transcript, verifierUrl: `/@fs${path.resolve(root, '../wasm_demo/pkg/verifier.js')}` });
    await page.close();
    const evidence = { file: path.resolve(file), sha256: createHash('sha256').update(data).digest('hex'),
      uiRoot: root, runtimeAssets, protocolVersion: transcript.protocolVersion, actions: transcript.actions?.length, ...result };
    results.push(evidence);
    console.log(JSON.stringify({ ...evidence, engineReplay: undefined, report: evidence.report ? {
      verifiedActions: evidence.report.verifiedActions, engineReplay: evidence.report.engineReplay?.verified,
      replayedActions: evidence.report.engineReplay?.replayedActions,
      endOfMatchDisclosuresVerified: evidence.report.engineReplay?.endOfMatchDisclosuresVerified,
      outcome: evidence.report.outcome } : undefined }));
  }
} finally {
  await browser?.close();
  await server.close();
  if (output) await writeFile(output, JSON.stringify(results, null, 2));
}
if (results.some(result => !result.ok)) process.exitCode = 1;
