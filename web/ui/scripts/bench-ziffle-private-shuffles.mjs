import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, realpath, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

// Real browser WASM only. Creation and each cold verification use distinct pages
// (distinct WASM instances), so the ceremony cache cannot make a cold result warm.
// Seeds are deterministic benchmark fixtures, never gameplay randomness.
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const arg = (name, fallback) => {
  const at = process.argv.indexOf(name);
  if (at < 0) return fallback;
  assert.ok(process.argv[at + 1], `${name} requires a value`);
  return process.argv[at + 1];
};
const packageDir = await realpath(path.resolve(arg('--package-dir', path.resolve(root, '../wasm_demo/pkg'))));
const mode = arg('--mode', 'baseline');
assert.ok(['baseline', 'private'].includes(mode), 'mode must be baseline or private');
const repeats = Number(arg('--repeats', '3'));
assert.ok(Number.isInteger(repeats) && repeats >= 1);
const output = arg('--output', null);
const fixtureOutput = arg('--fixture-output', null);
const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'),
  cacheDir: path.join(os.tmpdir(), `ironsmith-ziffle-private-bench-${process.pid}`), logLevel: 'error',
  server: { host: '127.0.0.1', port: 0, hmr: false, watch: null,
    fs: { allow: [path.resolve(root, '../..'), packageDir] } } });
let browser;
const loadAverageStart = os.loadavg();
try {
  await server.listen();
  browser = await chromium.launch();
  const moduleUrl = `/@fs/${path.join(packageDir, 'verifier.js')}`;
  const newRealm = async () => {
    const page = await browser.newPage();
    await page.route('**/private-shuffle-benchmark', route => route.fulfill({
      contentType: 'text/html', body: '<title>Private shuffle verification benchmark</title>' }));
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/private-shuffle-benchmark`);
    await page.evaluate(async url => { globalThis.benchmarkApi = await import(url); await globalThis.benchmarkApi.default(); }, moduleUrl);
    return page;
  };
  const builder = await newRealm();
  const fixtures = await builder.evaluate(async ({ mode }) => {
    const api = globalThis.benchmarkApi;
    const bytes = value => new TextEncoder().encode(JSON.stringify(value)).length;
    const seed = n => Number(n).toString(16).padStart(2, '0').repeat(32);
    const elapsed = operation => { const start = performance.now(); const value = operation(); return { value, ms: performance.now() - start }; };
    const keyContext = 'ironsmith-private-shuffle-benchmark';
    const identities = [1, 2].map(n => api.ziffleKeygen({ deckCount: 60, context: keyContext, entropyHex: seed(n) }));
    const keys = identities.map((identity, player) => ({ player, publicKeyHex: identity.publicKeyHex, ownershipProofHex: identity.ownershipProofHex }));
    const cases = [];
    for (const [caseIndex, spec] of [
      { name: 'initial-60', deckCount: 60 }, { name: 'initial-61', deckCount: 61 },
      { name: 'reshuffle-52', deckCount: 52 }, { name: 'second-reshuffle-52', deckCount: 52 },
    ].entries()) {
      const ceremony = { deckCount: spec.deckCount, context: `${keyContext}:${spec.name}`, keyContext, keys, steps: [] };
      if (mode === 'private' && caseIndex >= 2) {
        const genesis = cases[0].ceremony;
        const epochs = [{ deckCount: genesis.deckCount, context: genesis.context, steps: genesis.steps }];
        if (caseIndex === 3) {
          const previous = cases[2].ceremony;
          epochs.push({ deckCount: previous.deckCount, context: previous.context,
            sources: previous.inputDeck.sources, steps: previous.steps });
        }
        ceremony.inputDeck = { universeCount: genesis.deckCount, epochs,
          sources: Array.from({ length: spec.deckCount }, (_, position) => ({ epoch: epochs.length - 1, position })) };
      }
      const creationSamples = [];
      for (let shuffler = 0; shuffler < 2; shuffler++) {
        const result = elapsed(() => api.ziffleBuildShuffleStep({ ...ceremony, shuffler, entropyHex: seed(20 + caseIndex * 2 + shuffler) }));
        ceremony.steps.push({ shuffler, deckHex: result.value.deckHex, proofHex: result.value.proofHex });
        creationSamples.push({ operation: `build-step-${shuffler}`, ms: result.ms });
      }
      const tokens = [];
      const cardPositions = Array.from({ length: 50 }, (_, i) => i);
      for (let player = 0; player < 2; player++) {
        const common = { ...ceremony, ...identities[player], deckCount: spec.deckCount, entropyHex: seed(40 + caseIndex * 2 + player) };
        const one = elapsed(() => api.ziffleBuildRevealTokens({ ...common, cardPositions: [0] }));
        const fifty = elapsed(() => api.ziffleBuildRevealTokens({ ...common, cardPositions }));
        creationSamples.push({ operation: `build-reveal-1-player-${player}`, ms: one.ms }, { operation: `build-reveal-50-player-${player}`, ms: fifty.ms });
        tokens.push(...fifty.value);
      }
      const oneInput = { ...ceremony, cardPositions: [0], tokens: tokens.filter(token => token.cardPosition === 0) };
      const fiftyInput = { ...ceremony, cardPositions, tokens };
      const expected = api.ziffleRevealCards(fiftyInput);
      // Untimed fixture integrity check: every output is a distinct original
      // manifest label, and private descendants preserve the selected set.
      const tailPositions = Array.from({ length: spec.deckCount - 50 }, (_, i) => i + 50);
      const tailTokens = identities.flatMap((identity, player) => api.ziffleBuildRevealTokens({
        ...ceremony, ...identity, deckCount: spec.deckCount, cardPositions: tailPositions,
        entropyHex: seed(80 + caseIndex * 2 + player),
      }));
      const allExpected = api.ziffleRevealCards({ ...ceremony,
        cardPositions: Array.from({ length: spec.deckCount }, (_, i) => i), tokens: [...tokens, ...tailTokens] });
      if (new Set(allExpected.map(card => card.originalSlot)).size !== spec.deckCount) throw new Error('Benchmark deck duplicates a manifest identity');
      if (mode === 'private' && caseIndex >= 2) {
        const ancestors = caseIndex === 2 ? [cases[0]] : [cases[0], cases[2]];
        const inputs = ceremony.inputDeck.sources.map(source => ancestors[source.epoch].allExpected[source.position].originalSlot).sort((a, b) => a - b);
        const outputs = allExpected.map(card => card.originalSlot).sort((a, b) => a - b);
        if (JSON.stringify(inputs) !== JSON.stringify(outputs)) throw new Error('Authenticated reshuffle changed the selected manifest identities');
      }
      cases.push({ name: spec.name, ceremony, oneInput, fiftyInput, expected, allExpected, creationSamples,
        parents: caseIndex >= 2 ? [cases[0].ceremony, ...(caseIndex === 3 ? [cases[2].ceremony] : [])] : [],
        bytes: { ceremonyJson: bytes(ceremony), shuffleProofsBinary: ceremony.steps.reduce((sum, step) => sum + step.proofHex.length / 2, 0),
          encryptedDecksBinary: ceremony.steps.reduce((sum, step) => sum + step.deckHex.length / 2, 0),
          inputGraphJson: ceremony.inputDeck ? bytes(ceremony.inputDeck) : 0,
          parentProofsBinary: (ceremony.inputDeck?.epochs || []).flatMap(epoch => epoch.steps).reduce((sum, step) => sum + step.proofHex.length / 2, 0),
          reveal1Json: bytes(oneInput), reveal50Json: bytes(fiftyInput),
          reveal1TokensJson: bytes(oneInput.tokens), reveal50TokensJson: bytes(tokens) } });
    }
    return cases;
  }, { mode });
  await builder.close();
  if (fixtureOutput) await writeFile(fixtureOutput, JSON.stringify(fixtures) + '\n');
  const cases = [];
  for (const fixture of fixtures) {
    const samples = [];
    const operations = ['verify', ...(fixture.parents.length ? ['verify-after-parents'] : []), 'reveal-1', 'reveal-50'];
    for (const operation of operations) {
      for (let repeat = 0; repeat < repeats; repeat++) {
        const page = await newRealm();
        const measured = await page.evaluate(({ fixture, operation }) => {
          const api = globalThis.benchmarkApi;
          const isVerify = operation.startsWith('verify');
          const input = isVerify ? fixture.ceremony : operation === 'reveal-1' ? fixture.oneInput : fixture.fiftyInput;
          if (operation === 'verify-after-parents') for (const parent of fixture.parents) api.ziffleVerifyShuffle(parent);
          const call = () => isVerify ? api.ziffleVerifyShuffle(input) : api.ziffleRevealCards(input);
          const measure = () => { const start = performance.now(); const value = call(); return { ms: performance.now() - start, value }; };
          const cold = measure();
          const warm = measure();
          const invalid = structuredClone(input);
          if (isVerify) invalid.steps[0].proofHex = '00';
          else invalid.tokens[0].proofHex = '00';
          let tamperRejected = false;
          try { isVerify ? api.ziffleVerifyShuffle(invalid) : api.ziffleRevealCards(invalid); } catch { tamperRejected = true; }
          return { cold, warm, tamperRejected };
        }, { fixture, operation });
        assert.ok(measured.tamperRejected, `${fixture.name} ${operation} must reject tampering after caching`);
        if (!operation.startsWith('verify')) {
          const expected = operation === 'reveal-1' ? fixture.expected.slice(0, 1) : fixture.expected;
          assert.deepEqual(measured.cold.value, expected);
          assert.deepEqual(measured.warm.value, expected);
        } else assert.equal(measured.cold.value.deckCount, fixture.ceremony.deckCount);
        samples.push({ operation, repeat, cache: operation === 'verify-after-parents' ? 'parents-warm' : 'cold', ms: measured.cold.ms }, { operation, repeat, cache: 'warm', ms: measured.warm.ms });
        await page.close();
      }
    }
    const summary = {};
    for (const operation of operations) for (const cache of [operation === 'verify-after-parents' ? 'parents-warm' : 'cold', 'warm']) {
      const values = samples.filter(sample => sample.operation === operation && sample.cache === cache).map(sample => sample.ms).sort((a, b) => a - b);
      summary[`${operation}-${cache}`] = { medianMs: values[Math.floor(values.length / 2)], minMs: values[0], maxMs: values.at(-1) };
    }
    cases.push({ name: fixture.name, deckCount: fixture.ceremony.deckCount, bytes: fixture.bytes, creationSamples: fixture.creationSamples, samples, summary });
  }
  const wasm = await readFile(path.join(packageDir, 'verifier_bg.wasm'));
  const result = { capturedAt: new Date().toISOString(), mode, repeats, node: process.version, chromium: browser.version(),
    os: `${os.type()} ${os.release()} ${os.arch()}`, cpus: os.cpus()[0]?.model, cpuCount: os.cpus().length, loadAverageStart, loadAverageEnd: os.loadavg(), packageDir,
    wasm: { bytes: wasm.length, sha256: createHash('sha256').update(wasm).digest('hex') },
    methodology: { unit: 'milliseconds', cold: 'Fresh browser page and fresh WASM instance, no ceremony operation before timed verification.',
      warm: 'Immediate identical operation in the same WASM instance after the cold call.',
      parentsWarm: 'Fresh instance verifies accepted parent ceremonies first (outside timing), then times the first verification of the NEW current proof.',
      excluded: 'WASM module load, browser startup, network transmission and ceremony generation are outside verification timing.',
      wire: 'Uncompressed JSON verifier call input (including hex encoding), excluding signed gameplay envelopes.',
      baselineReshuffle: 'Fresh canonical 52-card ceremony, as old protocol; does not authenticate subset continuity.' }, cases };
  if (output) await writeFile(output, JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify(result, null, 2));
} finally { await browser?.close(); await server.close(); }
