import assert from 'node:assert/strict';
import { realpath } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

// Runs actual browser WASM, excluding ceremony creation from reveal timings.
// --package-dir permits comparing a newly built verifier without replacing
// the application's current WASM package.
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const packageArg = process.argv.indexOf('--package-dir');
if (packageArg >= 0 && !process.argv[packageArg + 1]) {
  throw new Error('--package-dir requires a directory');
}
const packageDir = await realpath(packageArg >= 0
  ? path.resolve(process.argv[packageArg + 1])
  : path.resolve(root, '../wasm_demo/pkg'));
const server = await createServer({
  root,
  configFile: path.join(root, 'vite.config.js'),
  cacheDir: path.join(root, 'node_modules', `.vite-ziffle-bench-${process.pid}`),
  logLevel: 'error',
  server: { host: '127.0.0.1', port: 0, hmr: false, watch: null,
    fs: { allow: [path.resolve(root, '../..'), packageDir] } },
});
let browser;
try {
  await server.listen();
  browser = await chromium.launch();
  const page = await browser.newPage();
  await page.route('**/ziffle-benchmark', route => route.fulfill({
    contentType: 'text/html', body: '<title>Ziffle reveal benchmark</title>',
  }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/ziffle-benchmark`);
  const result = await page.evaluate(async moduleUrl => {
    const api = await import(moduleUrl);
    await api.default();
    const deckCount = 60;
    const context = 'ironsmith-fetch-reveal-benchmark';
    const identities = ['11', '22'].map(byte => api.ziffleKeygen({
      deckCount, context, entropyHex: byte.repeat(32),
    }));
    const keys = identities.map((key, player) => ({ player,
      publicKeyHex: key.publicKeyHex, ownershipProofHex: key.ownershipProofHex }));
    const steps = [];
    for (let shuffler = 0; shuffler < keys.length; shuffler++) {
      const step = api.ziffleBuildShuffleStep({ deckCount, context, keys, steps,
        shuffler, entropyHex: ['33', '44'][shuffler].repeat(32) });
      steps.push({ shuffler, deckHex: step.deckHex, proofHex: step.proofHex });
    }
    const ceremony = { deckCount, context, keys, steps };
    const samples = [];
    const measure = (label, operation) => {
      const start = performance.now();
      const value = operation();
      samples.push({ label, ms: Math.round((performance.now() - start) * 10) / 10 });
      return value;
    };
    measure('verify ceremony, first call', () => api.ziffleVerifyShuffle(ceremony));
    for (let repeat = 0; repeat < 3; repeat++) {
      measure('verify ceremony, repeat', () => api.ziffleVerifyShuffle(ceremony));
    }
    const positions = Array.from({ length: 50 }, (_, position) => position);
    let tokens = [];
    for (let player = 0; player < keys.length; player++) {
      const input = { ...ceremony, ...identities[player], entropyHex: ['55', '66'][player].repeat(32) };
      measure('build tokens, 1 position', () => api.ziffleBuildRevealTokens({ ...input, cardPositions: [0] }));
      tokens.push(...measure('build tokens, 50 positions', () => api.ziffleBuildRevealTokens({ ...input, cardPositions: positions })));
    }
    const onePosition = { ...ceremony, cardPositions: [0], tokens: tokens.filter(token => token.cardPosition === 0) };
    let one;
    for (let repeat = 0; repeat < 3; repeat++) {
      one = measure('reveal, 1 position', () => api.ziffleRevealCards(onePosition));
    }
    const many = measure('reveal, 50 positions', () => api.ziffleRevealCards({ ...ceremony, cardPositions: positions, tokens }));
    let rejectedTamperedToken = false;
    const badTokens = onePosition.tokens.map((token, index) => index === 0 ? { ...token, proofHex: '00' } : token);
    try { api.ziffleRevealCards({ ...onePosition, tokens: badTokens }); }
    catch { rejectedTamperedToken = true; }
    return { samples, one, many, rejectedTamperedToken };
  }, `/@fs/${path.join(packageDir, 'verifier.js')}`);
  assert.equal(result.many.length, 50);
  assert.deepEqual(result.one[0], result.many[0]);
  assert.equal(new Set(result.many.map(card => card.originalSlot)).size, 50);
  assert.equal(result.rejectedTamperedToken, true, 'cached ceremonies must still reject invalid card proofs');
  console.log(JSON.stringify({ deckCount: 60, players: 2, packageDir, samples: result.samples }, null, 2));
} finally {
  await browser?.close();
  await server.close();
}
